use super::*;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

const SESSION: &str = "session:00000000-0000-0000-0000-000000000001";

fn ticket(session: &str, block: &str, expires_in_ms: u64) -> session::DataTicket {
    session::DataTicket {
        session_id: session.into(),
        block_id: block.into(),
        data_ref_id: "terminal".into(),
        subprotocol: Some(wire::PROTOCOL.into()),
        ticket: "single-use".into(),
        expires_in_ms,
    }
}

#[test]
fn tickets_are_scoped_expiring_and_single_use() {
    let mut state = ControlState::default();
    let mut wrong = ticket(SESSION, "wrong", 60_000);
    wrong.subprotocol = Some("other".into());
    state.add_tickets(
        vec![
            ticket(SESSION, "valid", 60_000),
            ticket(SESSION, "expired", 0),
            wrong,
        ],
        Instant::now(),
    );
    assert!(state.take_ticket("other-session", "valid").is_none());
    assert!(state.take_ticket(SESSION, "wrong").is_none());
    assert!(state.take_ticket(SESSION, "expired").is_none());
    assert!(state.take_ticket(SESSION, "valid").unwrap().bundled);
    assert!(state.take_ticket(SESSION, "valid").is_none());
    state.add_tickets(
        vec![ticket(SESSION, "slow", 100)],
        Instant::now() - Duration::from_secs(1),
    );
    assert!(state.take_ticket(SESSION, "slow").is_none());
    state.add_tickets(vec![ticket(SESSION, "removed", 60_000)], Instant::now());
    state.prune(&HostModel::default());
    assert!(state.tickets.is_empty());
}

#[test]
fn selection_and_bootstrap_preserve_explicit_intent() {
    let sessions = vec![session::Session {
        session_id: SESSION.into(),
        label: "work".into(),
    }];
    assert_eq!(select_session(&sessions, None).as_deref(), Some(SESSION));
    assert_eq!(
        select_session(&sessions, Some("work")).as_deref(),
        Some(SESSION)
    );
    assert_eq!(
        select_session(&sessions, Some(SESSION)).as_deref(),
        Some(SESSION)
    );
    assert!(select_session(&sessions, Some("missing")).is_none());
    let mut duplicate = sessions.clone();
    duplicate.push(session::Session {
        session_id: "other".into(),
        label: "work".into(),
    });
    assert!(select_session(&duplicate, Some("work")).is_none());
    for (wanted, creating, expected) in [
        (None, false, None),
        (Some("work"), false, Some(json!([]))),
        (Some("session:bad-id"), false, Some(json!([]))),
        (None, true, Some(json!([]))),
        (Some(SESSION), false, Some(json!([SESSION]))),
    ] {
        let request =
            serde_json::to_value(bootstrap_request(wanted, creating, std::iter::empty())).unwrap();
        assert_eq!(request.get("attach_session_ids").cloned(), expected);
        assert_eq!(request["data_tickets"][0]["subprotocol"], wire::PROTOCOL);
    }
    let retained = bootstrap_request(
        Some(SESSION),
        false,
        [SESSION.to_owned(), SESSION.to_owned()].into_iter(),
    );
    assert_eq!(retained.attach_session_ids, Some(vec![SESSION.into()]));
}

#[allow(clippy::result_large_err)] // tungstenite's callback fixes the error type.
async fn accept(listener: &TcpListener, protocol: &str) -> WebSocketStream<TcpStream> {
    let (tcp, _) = listener.accept().await.unwrap();
    tokio_tungstenite::accept_hdr_async(
        tcp,
        |_: &_, mut response: tokio_tungstenite::tungstenite::handshake::server::Response| {
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", protocol.parse().unwrap());
            Ok(response)
        },
    )
    .await
    .unwrap()
}

async fn recv(ws: &mut WebSocketStream<TcpStream>) -> Value {
    serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap()
}

async fn send(ws: &mut WebSocketStream<TcpStream>, value: Value) {
    ws.send(Message::Text(value.to_string().into()))
        .await
        .unwrap();
}

async fn welcome(ws: &mut WebSocketStream<TcpStream>, revision: u32) -> Value {
    let hello = recv(ws).await;
    assert_eq!(hello["api_revision"], 8);
    assert_eq!(hello["api_min_revision"], 7);
    send(ws, json!({"type":"welcome", "protocol":"rex.control.v1", "api_revision":revision,
        "limits":{"max_message_bytes":65536}, "client":{"id":"client:test", "reconnect_token":"test-token"}})).await;
    hello
}

fn boot() -> session::BootstrapResponse {
    session::BootstrapResponse {
        client_id: "client:test".into(),
        attach_all: session::AttachAllResponse {
            sessions: vec![session::Session {
                session_id: SESSION.into(),
                label: "snapshot".into(),
            }],
        },
        selected_session: Some(session::SelectedSession {
            session: session::Session {
                session_id: SESSION.into(),
                label: "snapshot".into(),
            },
            view: session::ViewResponse {
                session_id: Some(SESSION.into()),
                label: Some("snapshot".into()),
                revision: 1,
                ..Default::default()
            },
        }),
        ..Default::default()
    }
}

async fn connected(host: &HostConnection) {
    tokio::time::timeout(Duration::from_secs(8), async {
        while !host.model.lock().unwrap().connected {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn bootstrap_is_first_and_newer_pre_response_events_survive() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut ws = accept(&listener, "rex.control.v1").await;
        welcome(&mut ws, 8).await;
        let request = recv(&mut ws).await;
        assert_eq!(request["method"], "session.bootstrap");
        // An older destruction must not remove a session present in the snapshot.
        send(&mut ws, json!({"type":"event","seq":9,"payload":{"type":"session_destroyed","payload":{"session_id":SESSION}}})).await;
        // A newer event can arrive BEFORE the bootstrap response.
        send(&mut ws, json!({"type":"event","seq":11,"payload":{"type":"session_event","payload":{
            "session_id":SESSION,"event":{"type":"session_label_changed","payload":{"label":"newer"}}}}})).await;
        send(
            &mut ws,
            json!({"type":"response","id":request["id"],"seq":10,"payload":boot()}),
        )
        .await;
        assert!(matches!(ws.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false);
    connected(&host).await;
    {
        let model = host.model.lock().unwrap();
        assert_eq!(model.sessions[0].label, "newer");
        assert_eq!(model.selected.as_deref(), Some(SESSION));
        assert_eq!(model.protocol.bootstrap_requests, 1);
        assert_eq!(model.protocol.attach_requests, 0);
    }
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn unsupported_negotiated_revision_is_rejected() {
    for revision in [6, 9] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut ws = accept(&listener, "rex.control.v1").await;
            welcome(&mut ws, revision).await;
        });
        assert!(transport::control(&endpoint, None).await.is_err());
        server.await.unwrap();
    }
}

#[test]
fn snapshot_cutoffs_are_scoped_to_the_state_they_replace() {
    let state = ControlState {
        bootstrap_seq: 10,
        list_seq: 15,
        view_seq: HashMap::from([(SESSION.into(), 20)]),
        ..Default::default()
    };
    let event = |seq, name: &str, session: Option<&str>| rexrs::Event {
        seq,
        name: name.into(),
        session_id: session.map(str::to_owned),
        payload: json!({}),
    };
    assert!(!state.includes(&event(20, "block_closed", Some(SESSION))));
    assert!(state.includes(&event(21, "block_closed", Some(SESSION))));
    assert!(state.includes(&event(19, "block_closed", Some("other-session"))));
    assert!(!state.includes(&event(15, "session_destroyed", None)));
    assert!(state.includes(&event(16, "session_destroyed", Some(SESSION))));
    assert!(!state.includes(&event(19, "session_label_changed", Some(SESSION))));
    assert!(state.includes(&event(21, "session_label_changed", Some(SESSION))));
}

#[tokio::test]
async fn failed_bootstrap_closes_and_retries_without_old_identity() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut first = accept(&listener, "rex.control.v1").await;
        welcome(&mut first, 8).await;
        let request = recv(&mut first).await;
        assert_eq!(request["method"], "session.bootstrap");
        send(&mut first, json!({"type":"response","id":request["id"],"error":{"code":"session_not_found","message":"closed during setup","status":404}})).await;
        assert!(matches!(first.next().await, Some(Ok(Message::Close(_)))));
        let mut second = accept(&listener, "rex.control.v1").await;
        let hello = welcome(&mut second, 8).await;
        assert!(hello.get("reconnect_token").is_none());
        let request = recv(&mut second).await;
        assert_eq!(request["method"], "session.bootstrap");
        send(
            &mut second,
            json!({"type":"response","id":request["id"],"seq":1,"payload":boot()}),
        )
        .await;
        assert!(matches!(second.next().await, Some(Ok(Message::Close(_)))));
    });
    let host = HostConnection::spawn(endpoint, None, None, None, false);
    connected(&host).await;
    assert_eq!(host.model.lock().unwrap().protocol.bootstrap_requests, 2);
    drop(host);
    server.await.unwrap();
}

#[tokio::test]
async fn rejected_bundle_gets_fresh_ticket_and_new_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for accepted in [false, true] {
            let mut ws = accept(&listener, wire::PROTOCOL).await;
            let message = ws.next().await.unwrap().unwrap().into_data();
            assert_eq!(message.len(), 17);
            assert_eq!(message[0], 1);
            let mut response = if accepted { vec![0; 32] } else { vec![0x82, 1] };
            if accepted {
                response[0] = 0x81;
                response[24] = 1;
            }
            ws.send(Message::Binary(response.into())).await.unwrap();
        }
    });
    let (sender, commands) = flume::unbounded();
    let host = tokio::spawn(async move {
        for bundled in [true, false] {
            let HostCommand::Ticket {
                session,
                block,
                reply,
            } = commands.recv_async().await.unwrap()
            else {
                panic!("expected ticket");
            };
            assert_eq!(session, SESSION);
            assert_eq!(block, "block");
            assert!(
                reply
                    .send(Ok(TerminalTicket {
                        token: "AAAAAAAAAAAAAAAAAAAAAA".into(),
                        bundled
                    }))
                    .is_ok()
            );
        }
    });
    let (_, welcome) = open_data(&endpoint, SESSION, "block", &sender)
        .await
        .unwrap();
    assert_eq!(welcome.nonce, 1);
    host.await.unwrap();
    server.await.unwrap();
}
