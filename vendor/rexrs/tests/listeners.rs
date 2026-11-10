//! Revision 12 adds listeners without making older status/bootstrap replies invalid.
use rexrs::api::{server::StatusResponse, session::ServerStatus};
use serde_json::{Value, json};

fn status() -> Value {
    json!({
        "pid": 1, "instance_id": "private", "started_at": "2026-11-09T21:00:41Z",
        "exit_empty": false, "session_count": 0, "config": {"generation": 1, "hash": ""}
    })
}

#[test]
fn older_and_null_listener_lists_remain_compatible() {
    for listeners in [None, Some(Value::Null), Some(json!([]))] {
        let mut value = status();
        if let Some(listeners) = listeners {
            value["listeners"] = listeners;
        }
        assert!(
            serde_json::from_value::<StatusResponse>(value.clone())
                .unwrap()
                .listeners
                .is_empty()
        );
        assert!(
            serde_json::from_value::<ServerStatus>(value)
                .unwrap()
                .listeners
                .is_empty()
        );
    }
}

#[test]
fn listeners_keep_addresses_and_unknown_transport_and_admission_values() {
    let mut value = status();
    value["listeners"] = json!([
        {"endpoint":"unix:///scratch/rex.sock", "addresses":["/scratch/rex.sock"],
         "transport":"unix", "admission":"peer_credentials", "primary":true},
        {"endpoint":"127.0.0.1:0", "addresses":["127.0.0.1:54321"],
         "transport":"future_transport", "admission":"future_admission"},
        {"endpoint":"unknown", "transport":"tcp", "addresses":null}
    ]);
    let status: StatusResponse = serde_json::from_value(value.clone()).unwrap();
    let bootstrap: ServerStatus = serde_json::from_value(value).unwrap();
    assert_eq!(status.listeners.len(), 3);
    assert_eq!(status.listeners[0].primary, Some(true));
    assert_eq!(status.listeners[1].addresses, ["127.0.0.1:54321"]);
    assert_eq!(status.listeners[1].transport, "future_transport");
    assert_eq!(
        status.listeners[1].admission.as_deref(),
        Some("future_admission")
    );
    assert_eq!(status.listeners[1].primary, None);
    assert!(status.listeners[2].addresses.is_empty());
    assert_eq!(status.listeners[2].admission, None);
    assert_eq!(
        serde_json::to_value(status.listeners).unwrap(),
        serde_json::to_value(bootstrap.listeners).unwrap()
    );
}
