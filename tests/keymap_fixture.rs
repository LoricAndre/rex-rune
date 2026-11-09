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
    assert_eq!(reply.bindings.len(), 24);
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
    assert_eq!(reply.bindings[0].keys, ["ctrl+x"]);
    assert_eq!(reply.bindings[1].keys, ["ctrl+x", "y"]);
    for spelling in ["cmd+u", "cmd+i", "alt+o", "cmd+h"] {
        assert!(keys.contains(&vec![spelling.into()]));
    }
    assert!(
        reply
            .diagnostics
            .iter()
            .any(|d| d.key.as_deref() == Some("ctrl+b z"))
    );
    assert!(reply.diagnostics.iter().any(|d| d.message == r#"key sequence "ctrl+x y" (settings) cannot start, because "ctrl+x" is bound to "client.settings" above it (settings)"#));
    assert!(reply.diagnostics.iter().any(|d| {
        d.source.as_deref() == Some("settings") && d.key.as_deref() == Some("ctrl+q")
    }));
    assert!(
        reply.diagnostics.iter().any(|d| {
            d.source.as_deref() == Some("linux") && d.key.as_deref() == Some("ctrl+b")
        })
    );
}
