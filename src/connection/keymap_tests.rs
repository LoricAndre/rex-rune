use super::*;
use crate::keymap::Overrides;
use rexrs::api::client::{Binding, Keymap, KeymapResponse};

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
