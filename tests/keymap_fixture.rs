//! Protocol contract recorded from a disposable Rex revision-11 server.
use rexrs::api::client::{Keymap, KeymapResponse};

#[test]
fn recorded_keymap_has_layers_sequences_forwarding_and_standard_spelling() {
    let request: Keymap =
        serde_json::from_str(include_str!("fixtures/keymap/request.json")).unwrap();
    let reply: KeymapResponse =
        serde_json::from_str(include_str!("fixtures/keymap/response.json")).unwrap();
    assert_eq!(request.defaults[0].name, "linux");
    assert_eq!(request.overrides[0].name, "settings");
    assert!(reply.config.exists);
    assert_eq!(reply.bindings.len(), 17);
    let keys: Vec<_> = reply.bindings.iter().map(|b| b.keys.clone()).collect();
    assert!(keys.contains(&vec!["ctrl+alt+shift+cmd+k".into()]));
    assert!(keys.contains(&vec!["ctrl++".into()]));
    assert!(keys.contains(&vec!["ctrl+[KeyT]".into()]));
    assert!(keys.contains(&vec!["ctrl+FutureKey".into()]));
    assert!(!keys.contains(&vec!["ctrl+r".into()]));
    assert!(!keys.contains(&vec!["ctrl+shift+t".into()]));
    assert!(
        reply
            .bindings
            .iter()
            .any(|b| { b.keys == ["ctrl+b", "x"] && b.action == "client.key.forward" })
    );
    assert_eq!(reply.bindings[0].source.as_deref(), Some("settings"));
    assert_eq!(reply.bindings[1].action, "client.settings");
    assert_eq!(reply.bindings[2].action, "client.tab.new");
    assert!(reply.diagnostics.iter().any(|d| {
        d.source.as_deref() == Some("settings") && d.key.as_deref() == Some("ctrl+q")
    }));
    assert!(
        reply.diagnostics.iter().any(|d| {
            d.source.as_deref() == Some("linux") && d.key.as_deref() == Some("ctrl+b")
        })
    );
}
