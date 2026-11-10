use super::*;
use crate::keymap::Overrides;
use rexrs::api::client::{Binding, Keymap, KeymapResponse};

async fn take_perform(host: &HostConnection) -> perform::Pending {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(pending) = host.model.lock().unwrap().performs.pop() {
                return pending;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}

fn answer(host: &HostConnection, pending: perform::Pending, outcome: perform::Outcome) {
    host.commands
        .send(HostCommand::PerformReply {
            epoch: pending.epoch,
            call_id: pending.request.call_id,
            outcome,
        })
        .unwrap();
}

fn declaration(generation: u64) -> Box<keymap::Declaration> {
    Box::new(keymap::Declaration {
        generation,
        request: Keymap::new().defaults([crate::keymap::defaults()]),
        overrides: Overrides::default(),
    })
}
async fn updated(host: &HostConnection, revision: u64) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while host.model.lock().unwrap().keymap_revision < revision {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn keymap_after_connected_error_does_not_disconnect_then_event_refreshes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (signal, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let boot_request = recv(&mut ws).await;
        assert_eq!(boot_request["method"], "session.bootstrap");
        send(
            &mut ws,
            json!({"type":"response","id":boot_request["id"],"seq":10,"payload":boot()}),
        )
        .await;
        let request = recv(&mut ws).await;
        assert_eq!(request["method"], "client.keymap");
        send(&mut ws,json!({"type":"response","id":request["id"],"error":{"code":"bad_request","message":"old server","status":400}})).await;
        wait.await.unwrap();
        // Deliberately below bootstrap sequence: config events bypass includes, not its rules.
        send(&mut ws,json!({"type":"event","seq":1,"payload":{"type":"keymap_changed","payload":{"hash":"new"}}})).await;
        let request = recv(&mut ws).await;
        assert_eq!(request["method"], "client.keymap");
        let mut reply = KeymapResponse::default();
        reply.config.hash = "new".into();
        reply.config.instance_id = "instance".into();
        reply
            .bindings
            .push(Binding::new(["ctrl+j"], "client.settings"));
        send(
            &mut ws,
            json!({"type":"response","id":request["id"],"payload":reply}),
        )
        .await;
        // Same hash, different config load counter: no refresh.
        send(&mut ws,json!({"type":"event","seq":2,"payload":{"type":"keymap_changed","payload":{"hash":"new","generation":99}}})).await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    host.commands
        .send(HostCommand::Keymap(declaration(3)))
        .unwrap();
    updated(&host, 1).await;
    {
        let m = host.model.lock().unwrap();
        assert!(m.connected);
        assert!(m.keymap.is_none());
        assert!(m.keymap_error.is_some());
        assert!(m.notice.is_none());
    }
    signal.send(()).unwrap();
    updated(&host, 2).await;
    {
        let m = host.model.lock().unwrap();
        assert!(m.connected);
        assert_eq!(m.keymap.as_ref().unwrap().generation, 3);
        assert!(m.keymap_error.is_none());
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn remote_perform_is_queued_even_without_session_and_after_caller_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (done, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 12).await;
        let bootstrap = recv(&mut ws).await;
        send(
            &mut ws,
            json!({"type":"response","id":bootstrap["id"],"seq":10,"payload":boot()}),
        )
        .await;
        // No call ID: ignore without accidentally executing or answering another request.
        send(
            &mut ws,
            json!({"type":"event","seq":1,"payload":{
            "type":"client_perform_requested","payload":{"action":"client.settings"}}}),
        )
        .await;
        for (call_id, session, expired) in [
            ("expired", Some("unattached"), true),
            ("current", None, false),
        ] {
            send(
                &mut ws,
                json!({"type":"event","seq":1,"session_id":session,"payload":{
                "type":"client_perform_requested","payload":{
                    "call_id":call_id,"action":"client.settings","session_id":session}}}),
            )
            .await;
            let reply = recv(&mut ws).await;
            assert_eq!(reply["method"], "client.reply");
            assert_eq!(reply["payload"]["call_id"], call_id);
            assert_eq!(reply["payload"]["error"]["code"], "not_supported");
            assert!(
                reply["payload"]["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("test refusal")
            );
            assert!(reply["payload"].get("result").is_none());
            let response = if expired {
                json!({"type":"response","id":reply["id"],"error":{"code":"not_found","status":404,"message":"request expired"}})
            } else {
                json!({"type":"response","id":reply["id"],"payload":{}})
            };
            send(&mut ws, response).await;
        }
        done.send(()).unwrap();
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    for _ in 0..2 {
        let pending = take_perform(&host).await;
        answer(
            &host,
            pending,
            perform::Outcome::error("not_supported", "test refusal"),
        );
    }
    tokio::time::timeout(Duration::from_secs(5), wait)
        .await
        .unwrap()
        .unwrap();
    {
        let model = host.model.lock().unwrap();
        assert!(model.connected);
        assert!(model.client_actions.is_empty());
    }
    drop(host);
    server.await.unwrap();
}

async fn refusal_failure_is_nonfatal(error: Option<(&str, u16)>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let error = error.map(|(code, status)| {
        json!({
            "code": code, "status": status, "message": "reply rejected"
        })
    });
    let (ready, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 12).await;
        let bootstrap = recv(&mut ws).await;
        send(
            &mut ws,
            json!({"type":"response","id":bootstrap["id"],"seq":10,"payload":boot()}),
        )
        .await;
        send(
            &mut ws,
            json!({"type":"event","seq":1,"payload":{
                "type":"client_perform_requested","payload":{
                    "call_id":"refusal","action":"client.tab.new"}}}),
        )
        .await;
        let reply = recv(&mut ws).await;
        assert_eq!(reply["method"], "client.reply");
        assert_eq!(reply["payload"]["call_id"], "refusal");
        assert_eq!(reply["payload"]["error"]["code"], "not_supported");
        if let Some(error) = error {
            send(
                &mut ws,
                json!({"type":"response","id":reply["id"],"error":error}),
            )
            .await;
        }
        // With no response, the client must time out and keep using this socket.
        ready.send(()).unwrap();
        let request = recv(&mut ws).await;
        assert_eq!(
            request["method"], "client.keymap",
            "refusal must not be retried"
        );
        send(
            &mut ws,
            json!({"type":"response","id":request["id"],"payload":KeymapResponse::default()}),
        )
        .await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    let pending = take_perform(&host).await;
    answer(
        &host,
        pending,
        perform::Outcome::error("not_supported", "test refusal"),
    );
    tokio::time::timeout(Duration::from_secs(5), wait)
        .await
        .unwrap()
        .unwrap();
    host.commands
        .send(HostCommand::Keymap(declaration(1)))
        .unwrap();
    updated(&host, 1).await;
    {
        let model = host.model.lock().unwrap();
        assert!(model.connected);
        assert!(model.keymap.is_some());
        assert!(model.keymap_error.is_none());
        assert!(model.notice.is_none());
        assert!(model.client_actions.is_empty());
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn remote_perform_reply_api_errors_are_nonfatal() {
    for error in [("not_found", 404), ("bad_request", 400), ("internal", 500)] {
        refusal_failure_is_nonfatal(Some(error)).await;
    }
}

#[tokio::test]
async fn remote_perform_reply_timeout_is_nonfatal() {
    refusal_failure_is_nonfatal(None).await;
}

#[tokio::test]
async fn action_has_35_second_budget_and_queues_client_actions_without_servers() {
    assert_eq!(actions::TIMEOUT, Duration::from_secs(35));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let request = recv(&mut ws).await;
        send(
            &mut ws,
            json!({"type":"response","id":request["id"],"seq":10,"payload":boot()}),
        )
        .await;
        let request = recv(&mut ws).await;
        assert_eq!(request["method"], "client.action");
        assert!(request["payload"].get("args").is_none());
        assert!(request["payload"].get("servers").is_none());
        // A normal 8-second timeout would fail this. Later control work stays queued.
        assert!(
            tokio::time::timeout(Duration::from_secs(9), ws.next())
                .await
                .is_err()
        );
        send(
            &mut ws,
            json!({"type":"response","id":request["id"],"payload":{"client_actions":[
                {"action":"client.sidebar.toggle"},{"action":"client.unknown"}
            ]}}),
        )
        .await;
        let queued = recv(&mut ws).await;
        assert_eq!(queued["method"], "server.status");
        send(
            &mut ws,
            json!({"type":"response","id":queued["id"],"payload":{}}),
        )
        .await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    host.commands
        .send(HostCommand::Action {
            generation: 7,
            request: Box::new(rexrs::api::client::Action::new("clear_screen").origin("key")),
        })
        .unwrap();
    host.commands
        .send(HostCommand::Request {
            method: "server.status".into(),
            payload: json!({}),
        })
        .unwrap();
    tokio::time::timeout(Duration::from_secs(12), async {
        while host.model.lock().unwrap().client_actions.is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    {
        let m = host.model.lock().unwrap();
        assert!(m.connected);
        assert_eq!(m.client_actions.len(), 2);
        assert_eq!(m.client_actions[0].generation, 7);
        assert_eq!(m.client_actions[0].action.action, "client.sidebar.toggle");
        assert_eq!(m.client_actions[1].action.action, "client.unknown");
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn action_json_and_request_size_errors_notify_without_reconnecting() {
    for oversized in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut ws = accept(&listener, "rex.control.v1").await;
            welcome(&mut ws, 11).await;
            let request = recv(&mut ws).await;
            send(
                &mut ws,
                json!({"type":"response","id":request["id"],"seq":10,"payload":boot()}),
            )
            .await;
            if !oversized {
                let request = recv(&mut ws).await;
                assert_eq!(request["method"], "client.action");
                assert_eq!(request["payload"]["action"], "bad_reply");
                send(
                    &mut ws,
                    json!({"type":"response","id":request["id"],
                    "payload":{"client_actions":"not an array"}}),
                )
                .await;
            }
            // Oversize is rejected before sending; malformed replies are consumed.
            // Either way the next action must use this same connection.
            let request = recv(&mut ws).await;
            assert_eq!(request["method"], "client.action");
            assert_eq!(request["payload"]["action"], "after_error");
            send(
                &mut ws,
                json!({"type":"response","id":request["id"],"payload":{
                "client_actions":[{"action":"client.sidebar.toggle"}]}}),
            )
            .await;
            assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
        });
        let host = HostConnection::spawn(endpoint, None, None, None, false, None);
        connected(&host).await;
        let mut bad = rexrs::api::client::Action::new("bad_reply");
        if oversized {
            bad.args = Some(json!({"large": "x".repeat(70_000)}));
        }
        for request in [bad, rexrs::api::client::Action::new("after_error")] {
            host.commands
                .send(HostCommand::Action {
                    generation: 0,
                    request: Box::new(request),
                })
                .unwrap();
        }
        tokio::time::timeout(Duration::from_secs(3), async {
            while host.model.lock().unwrap().client_actions.is_empty() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        {
            let model = host.model.lock().unwrap();
            assert!(model.connected);
            let notice = model.notice.as_ref().expect("action failure notice");
            assert!(notice.contains("Rex client.action failed"));
            assert!(notice.contains(if oversized {
                "larger than"
            } else {
                "JSON error"
            }));
            assert_eq!(model.client_actions.len(), 1);
        }
        drop(host);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn keymap_timeout_is_nonfatal() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 11).await;
        let request = recv(&mut ws).await;
        send(
            &mut ws,
            json!({"type":"response","id":request["id"],"seq":10,"payload":boot()}),
        )
        .await;
        let request = recv(&mut ws).await;
        assert_eq!(request["method"], "client.keymap");
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false, None);
    connected(&host).await;
    host.commands
        .send(HostCommand::Keymap(declaration(0)))
        .unwrap();
    updated(&host, 1).await;
    assert!(host.model.lock().unwrap().connected);
    assert!(host.model.lock().unwrap().keymap_error.is_some());
    drop(host);
    server.await.unwrap();
}
