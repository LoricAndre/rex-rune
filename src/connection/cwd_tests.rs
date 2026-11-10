//! Server-side CWD inheritance and failure/timeout fallback; adapted from Lily's fork.
use super::*;
#[test]
fn the_shells_directory_comes_from_the_child_only() {
    use activity::shell_dir;
    assert_eq!(
        shell_dir(&json!({"child":{"cwd":"/srv/app"},"foreground":{"cwd":"/tmp"}})).as_deref(),
        Some("/srv/app")
    );
    assert_eq!(shell_dir(&json!({"foreground":{"cwd":"/tmp"}})), None);
    assert_eq!(shell_dir(&json!({"child":null})), None);
    assert_eq!(shell_dir(&json!({"child":{"cwd":""}})), None);
    assert_eq!(shell_dir(&json!({"child":{"cwd":"rel/dir"}})), None);
    // macOS: login wraps the shell.
    let login = json!({"name":"login","cwd":"/"});
    let zsh = json!({"name":"zsh","cwd":"/Users/me/app"});
    let vim = json!({"name":"vim","cwd":"/tmp"});
    assert_eq!(
        shell_dir(&json!({"child":login,"foreground":zsh,"ancestors":[login]})).as_deref(),
        Some("/Users/me/app")
    );
    assert_eq!(
        shell_dir(&json!({"child":login,"foreground":vim,"ancestors":[zsh,login]})).as_deref(),
        Some("/Users/me/app")
    );
    assert_eq!(shell_dir(&json!({"child":login})), None);
    assert_eq!(shell_dir(&json!({"child":login,"foreground":vim})), None);
    assert_eq!(
        shell_dir(&json!({"child":login,"foreground":login,"ancestors":[login]})),
        None
    );
}

/// Ask the host for a split of `block:a` in the default shell zsh.
fn split_from_a(host: &HostConnection) {
    host.commands
        .send(HostCommand::NewTerminal {
            method: "session.new_split".into(),
            payload: json!({"session_id":SESSION,"anchor_id":"block:a","direction":"horizontal",
                "side":"after","focus":true,"layout":crate::layout::shell_layout(Some("zsh"), None)}),
            cwd_from: "block:a".into(),
        })
        .unwrap();
}

/// The directory lookup for `block:a`, which must be the next message.
async fn directory_lookup(ws: &mut WebSocketStream<TcpStream>) -> Value {
    let lookup = next(ws).await;
    assert_eq!(lookup["method"], format!("{}.process", activity::TERMINAL));
    assert_eq!(
        lookup["payload"],
        json!({"session_id":SESSION,"block_id":"block:a","args":{}})
    );
    lookup
}

/// The split, which must come within `wait`, with these block options; answer it and the view
/// refresh that follows.
async fn split_with(ws: &mut WebSocketStream<TcpStream>, wait: Duration, options: Value) {
    let split = tokio::time::timeout(wait, recv(ws))
        .await
        .expect("the split in time");
    assert_eq!(split["method"], "session.new_split");
    assert_eq!(split["payload"]["anchor_id"], "block:a");
    assert_eq!(split["payload"]["layout"]["block"]["options"], options);
    reply(ws, &split, 3, json!({"block_ids":["block:b"]})).await;
    let refresh = next(ws).await;
    assert_eq!(refresh["method"], "session.view");
    reply(ws, &refresh, 4, json!({"session_id":SESSION,"revision":2})).await;
}

/// A host connected to a server that, after bootstrap, follows `script`, which hands the socket
/// back for the client's close; the host has just been asked for a split of `block:a`.
async fn split_server<F>(
    script: impl FnOnce(WebSocketStream<TcpStream>) -> F + Send + 'static,
) -> (HostConnection, tokio::task::JoinHandle<()>)
where
    F: std::future::Future<Output = WebSocketStream<TcpStream>> + Send,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 12).await;
        let boot = recv(&mut ws).await;
        reply(&mut ws, &boot, 1, boot_with(&[])).await;
        let mut ws = script(ws).await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    split_from_a(&host);
    (host, server)
}

#[tokio::test]
async fn new_terminals_start_in_the_source_shells_directory() {
    let (host, server) = split_server(|mut socket| async move {
        let ws = &mut socket;
        let lookup = directory_lookup(ws).await;
        let process = json!({"child":{"name":"zsh","cwd":"/srv/app"},"foreground":{"name":"vim","cwd":"/tmp"}});
        reply(ws, &lookup, 2, process).await;
        split_with(ws, Duration::from_secs(2), json!({"shell":"zsh","cwd":"/srv/app"})).await;
        socket
    })
    .await;
    until(&host, |m| m.views[SESSION].revision == 2).await;
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn a_session_ending_after_terminal_creation_is_removed() {
    let (host, server) = split_server(|mut socket| async move {
        let ws = &mut socket;
        let lookup = directory_lookup(ws).await;
        reply(ws, &lookup, 2, json!({})).await;
        let split = next(ws).await;
        assert_eq!(split["method"], "session.new_split");
        reply(ws, &split, 3, json!({"block_ids":["block:b"]})).await;
        let refresh = next(ws).await;
        assert_eq!(refresh["method"], "session.view");
        send(
            ws,
            json!({"type":"response","id":refresh["id"],
                "error":{"code":"session_not_found","message":"session ended","status":404}}),
        )
        .await;
        socket
    })
    .await;
    until(&host, |m| !m.views.contains_key(SESSION)).await;
    {
        let model = host.model.lock().unwrap();
        assert!(model.sessions.iter().all(|s| s.session_id != SESSION));
        assert!(model.selected.is_none());
        assert!(model.notice.is_none());
        assert!(model.connected);
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn a_failed_directory_lookup_still_opens_the_terminal() {
    let (host, server) = split_server(|mut socket| async move {
        let ws = &mut socket;
        let lookup = directory_lookup(ws).await;
        send(
            ws,
            json!({"type":"response","id":lookup["id"],
            "error":{"code":"not_found","message":"no such block","status":404}}),
        )
        .await;
        split_with(ws, Duration::from_secs(2), json!({"shell":"zsh"})).await;
        socket
    })
    .await;
    until(&host, |m| m.views[SESSION].revision == 2).await;
    assert!(host.model.lock().unwrap().connected);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn an_unavailable_shell_directory_keeps_the_server_default() {
    let (host, server) = split_server(|mut socket| async move {
        let ws = &mut socket;
        let lookup = directory_lookup(ws).await;
        reply(
            ws,
            &lookup,
            2,
            json!({"foreground":{"cwd":"/not-the-shell"}}),
        )
        .await;
        split_with(ws, Duration::from_secs(2), json!({"shell":"zsh"})).await;
        socket
    })
    .await;
    until(&host, |m| m.views[SESSION].revision == 2).await;
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn an_unanswered_directory_lookup_opens_the_terminal_once_without_one() {
    let (host, server) = split_server(|mut socket| async move {
        let ws = &mut socket;
        let lookup = directory_lookup(ws).await;
        // Not next(): its own limit would race the lookup's timeout.
        split_with(ws, Duration::from_secs(4), json!({"shell":"zsh"})).await;
        // A late answer changes nothing: the next message is the client closing.
        reply(ws, &lookup, 5, json!({"child":{"cwd":"/srv/app"}})).await;
        socket
    })
    .await;
    until(&host, |m| m.views[SESSION].revision == 2).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(host.model.lock().unwrap().connected);
    drop(host);
    server.await.unwrap();
}
