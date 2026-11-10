use super::*;

async fn boot_remote(listener: &TcpListener) -> WebSocketStream<TcpStream> {
    let mut ws = accept(listener, "rex.control.v1").await;
    let hello = welcome(&mut ws, 12).await;
    assert_eq!(hello["client"]["label"], "remote-test");
    let request = recv(&mut ws).await;
    send(
        &mut ws,
        json!({"type":"response","id":request["id"],"seq":10,"payload":boot()}),
    )
    .await;
    ws
}
async fn event(ws: &mut WebSocketStream<TcpStream>, id: &str) {
    send(ws, json!({"type":"event","seq":1,"payload":{"type":"client_perform_requested","payload":{
        "call_id":id,"action":"client.sidebar.toggle","session_id":"unattached",
        "requester":{"client_id":"cli", "info":{"name":"rex"}, "principal":{"display":"uid 1000"}}
    }}})).await;
}
fn host(endpoint: String) -> HostConnection {
    HostConnection::spawn_labeled(
        endpoint,
        None,
        None,
        None,
        false,
        None,
        "remote-test".into(),
    )
}
async fn queued(host: &HostConnection) -> perform::Pending {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(p) = host.model.lock().unwrap().performs.first().cloned() {
                return p;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn unclaimed_remote_request_expires_and_cannot_run_later() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (done, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = boot_remote(&listener).await;
        event(&mut ws, "1").await;
        let reply = recv(&mut ws).await;
        assert_eq!(reply["payload"]["error"]["code"], "internal");
        assert_eq!(reply["payload"]["call_id"], "1");
        send(
            &mut ws,
            json!({"type":"response","id":reply["id"],"payload":{}}),
        )
        .await;
        done.send(()).unwrap();
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = host(endpoint);
    let pending = queued(&host).await;
    assert_eq!(pending.request.session_id.as_deref(), Some("unattached"));
    assert_eq!(pending.request.requester.to_string(), "rex (uid 1000)");
    tokio::time::timeout(Duration::from_secs(7), wait)
        .await
        .unwrap()
        .unwrap();
    assert!(host.model.lock().unwrap().performs.is_empty());
    assert!(host.model.lock().unwrap().connected);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn busy_host_drops_already_readable_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (done, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = boot_remote(&listener).await;
        let slow = recv(&mut ws).await;
        assert_eq!(slow["method"], "test.slow");
        event(&mut ws, "1").await;
        tokio::time::sleep(Duration::from_millis(5200)).await;
        send(
            &mut ws,
            json!({"type":"response","id":slow["id"],"payload":{}}),
        )
        .await;
        let reply = recv(&mut ws).await;
        assert_eq!(reply["payload"]["error"]["code"], "internal");
        assert!(
            reply["payload"]["error"]["message"]
                .as_str()
                .unwrap()
                .contains("busy")
        );
        send(
            &mut ws,
            json!({"type":"response","id":reply["id"],"payload":{}}),
        )
        .await;
        done.send(()).unwrap();
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = host(endpoint);
    connected(&host).await;
    host.commands
        .send(HostCommand::Request {
            method: "test.slow".into(),
            payload: json!({}),
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(8), wait)
        .await
        .unwrap()
        .unwrap();
    assert!(host.model.lock().unwrap().performs.is_empty());
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn request_buffered_during_a_short_busy_step_keeps_its_original_budget() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (sent, arrival) = tokio::sync::oneshot::channel();
    let (done, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = boot_remote(&listener).await;
        let slow = recv(&mut ws).await;
        assert_eq!(slow["method"], "test.slow");
        let arrived = Instant::now();
        event(&mut ws, "short-busy").await;
        sent.send(arrived).unwrap();
        tokio::time::sleep(Duration::from_secs(4)).await;
        send(
            &mut ws,
            json!({"type":"response","id":slow["id"],"payload":{}}),
        )
        .await;
        let reply = recv(&mut ws).await;
        assert_eq!(reply["payload"]["call_id"], "short-busy");
        assert_eq!(reply["payload"]["error"]["code"], "internal");
        assert!(
            arrived.elapsed() < Duration::from_secs(7),
            "the busy step must not be followed by another full five-second budget"
        );
        send(
            &mut ws,
            json!({"type":"response","id":reply["id"],"payload":{}}),
        )
        .await;
        done.send(()).unwrap();
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = host(endpoint);
    connected(&host).await;
    host.commands
        .send(HostCommand::Request {
            method: "test.slow".into(),
            payload: json!({}),
        })
        .unwrap();
    let arrived = arrival.await.unwrap();
    let pending = queued(&host).await;
    assert!(
        pending.received <= arrived,
        "timestamp starts before the busy step, not at dequeue"
    );
    tokio::time::timeout(Duration::from_secs(3), wait)
        .await
        .unwrap()
        .unwrap();
    assert!(host.model.lock().unwrap().performs.is_empty());
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn reconnect_discards_old_requests_and_old_replies_even_with_reused_call_id() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (close, close_wait) = tokio::sync::oneshot::channel();
    let (done, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = boot_remote(&listener).await;
        event(&mut ws, "1").await;
        close_wait.await.unwrap();
        drop(ws);
        let mut ws = boot_remote(&listener).await;
        event(&mut ws, "1").await;
        let reply = recv(&mut ws).await;
        assert_eq!(reply["method"], "client.reply");
        assert_eq!(reply["payload"], json!({"call_id":"1", "result":null}));
        send(
            &mut ws,
            json!({"type":"response","id":reply["id"],"payload":{}}),
        )
        .await;
        done.send(()).unwrap();
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = host(endpoint);
    let old = queued(&host).await;
    close.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if host.model.lock().unwrap().connection_epoch > old.epoch {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let current = queued(&host).await;
    assert_ne!(current.epoch, old.epoch);
    assert_eq!(host.model.lock().unwrap().performs.len(), 1);
    host.model.lock().unwrap().performs.clear();
    host.commands
        .send(HostCommand::PerformReply {
            epoch: old.epoch,
            call_id: old.request.call_id,
            outcome: perform::Outcome::error("refused", "stale"),
        })
        .unwrap();
    host.commands
        .send(HostCommand::PerformReply {
            epoch: current.epoch,
            call_id: current.request.call_id,
            outcome: perform::Outcome::Performed,
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), wait)
        .await
        .unwrap()
        .unwrap();
    drop(host);
    server.await.unwrap();
}
