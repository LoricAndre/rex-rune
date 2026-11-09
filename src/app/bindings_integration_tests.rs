use super::*;
use crate::app::{actions, bindings::Input};
use rex_linux::{connection::Command, keymap::Overrides};
use rune::{PhysicalInput, PhysicalKeyCode as Code};
use serde_json::{Value, json};

fn overrides(ws: &mut Workspace, value: Value) {
    ws.settings.shortcut_file.overrides = Overrides::parse(&value);
    ws.local_keymap = actions::resolve(&ws.settings.shortcut_file.overrides);
    ws.keymap_generation += 1;
}
fn key(rt: &mut Runtime<Workspace>, code: Code, event: KeyEvent, pressed: bool, repeat: bool) {
    rt.physical_key_input(
        PhysicalInput::Key {
            key: code,
            mods: event.mods,
            pressed,
            repeat,
        },
        event,
        None,
    );
    rt.frame(1. / 60.);
}
fn ctrl(key: Key) -> KeyEvent {
    KeyEvent {
        key,
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    }
}

#[test]
fn effective_map_swaps_by_host_and_generation_and_preserves_problems() {
    let mut ws = workspace_hosts(2);
    let overrides = Overrides::parse(&json!([{"keys":"ctrl+b>","action":"client.settings"}]));
    for (index, action) in [(0, "client.sidebar.toggle"), (1, "client.search")] {
        let reply = rexrs::api::client::KeymapResponse {
            bindings: vec![rexrs::api::client::Binding::new(["ctrl+j"], action)],
            ..Default::default()
        };
        ws.hosts[index].model.connected = true;
        ws.hosts[index].model.keymap = Some(std::sync::Arc::new(
            rex_linux::connection::keymap::Snapshot {
                generation: 0,
                resolved: rex_linux::keymap::Resolved::from_server(&reply, &overrides),
                reply,
            },
        ));
    }
    assert_eq!(
        ws.effective_keymap().bindings[0].binding.action,
        "client.sidebar.toggle"
    );
    assert_eq!(ws.effective_keymap().diagnostics.len(), 1);
    ws.active = 1;
    assert_eq!(
        ws.effective_keymap().bindings[0].binding.action,
        "client.search"
    );
    ws.keymap_generation = 1;
    assert_eq!(ws.effective_keymap(), &ws.local_keymap);
    ws.keymap_generation = 0;
    ws.hosts[1].model.connected = false;
    assert_eq!(ws.effective_keymap(), &ws.local_keymap);
}

#[test]
fn settings_diagnostic_sets_toast_once_per_host_never_file_or_linux() {
    let mut ws = workspace();
    capture_host(&mut ws, 0);
    let problem = |source: &str| rexrs::api::client::Diagnostic {
        source: Some(source.into()),
        message: "test diagnostic".into(),
        severity: "warning".into(),
        ..Default::default()
    };
    for (diagnostics, expected) in [
        (vec![problem("linux"), problem("init.lua:1")], 0),
        (vec![problem("settings")], 1),
        (vec![], 1),
        (vec![problem("settings")], 1),
    ] {
        let reply = rexrs::api::client::KeymapResponse {
            diagnostics,
            ..Default::default()
        };
        {
            let mut model = ws.hosts[0].connection.model.lock().unwrap();
            model.connected = true;
            model.keymap = Some(std::sync::Arc::new(
                rex_linux::connection::keymap::Snapshot {
                    generation: 0,
                    resolved: rex_linux::keymap::Resolved::from_server(
                        &reply,
                        &Overrides::default(),
                    ),
                    reply,
                },
            ));
        }
        ws.sync(0, &mut rune::Tx::default());
        assert_eq!(ws.notices.len(), expected);
    }
}

#[test]
fn unbound_ctrl_tab_press_and_release_are_forwarded() {
    let (mut ws, _) = tabbed_workspace("single");
    overrides(&mut ws, json!([{"keys":"ctrl+tab","action":null}]));
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    key(&mut rt, Code::Tab, ctrl(Key::Tab), true, false);
    assert!(
        rt.state.forwarded.contains(
            &Input {
                event: &ctrl(Key::Tab),
                code: Some(Code::Tab),
                base: None,
                repeat: false
            }
            .identity()
        )
    );
    key(&mut rt, Code::Tab, ctrl(Key::Tab), false, false);
    let keys = sent_keys(&sent);
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0]["action"], "press");
    assert_eq!(keys[1]["action"], "release");
    assert!(rt.state.forwarded.is_empty());
}

#[test]
fn prefixes_repeats_and_failed_tails_do_not_leak_or_retry() {
    let (mut ws, requests_rx) = tabbed_workspace("single");
    overrides(
        &mut ws,
        json!([
            {"keys":"ctrl+b>c","action":"client.tab.new"},
            {"keys":"x","action":"client.settings"}
        ]),
    );
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    assert_eq!(rt.state.bindings.sequence.depth, 1);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, true);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), false, false);
    assert!(sent_keys(&sent).is_empty());
    key(
        &mut rt,
        Code::KeyX,
        KeyEvent {
            key: Key::Text("x".into()),
            mods: Mods::default(),
        },
        true,
        false,
    );
    assert!(
        rt.state.panel.is_none(),
        "failed tail is not retried as Settings"
    );
    assert_eq!(sent_keys(&sent).len(), 1);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    key(
        &mut rt,
        Code::KeyC,
        KeyEvent {
            key: Key::Text("c".into()),
            mods: Mods::default(),
        },
        true,
        false,
    );
    key(
        &mut rt,
        Code::KeyC,
        KeyEvent {
            key: Key::Text("c".into()),
            mods: Mods::default(),
        },
        true,
        true,
    );
    assert_eq!(
        requests(&requests_rx)
            .iter()
            .filter(|r| r.0 == "session.new_window")
            .count(),
        1
    );
    assert!(sent_keys(&sent).is_empty());
}

#[test]
fn nonrepeating_logical_keys_and_consumed_release_survive_dialog_close() {
    let (mut ws, _) = tabbed_workspace("single");
    overrides(
        &mut ws,
        json!([{"keys":"ctrl+j","action":"client.sidebar.toggle"}]),
    );
    let mut rt = runtime(ws);
    let ev = ctrl(Key::Command('j'));
    rt.key(ev.clone());
    rt.frame(0.);
    let open = rt.state.sidebar.is_open();
    rt.repeat_key(ev);
    rt.frame(0.);
    assert_eq!(
        rt.state.sidebar.is_open(),
        open,
        "logical-only repeats are suppressed too"
    );
    key(&mut rt, Code::Comma, ctrl(Key::Command(',')), true, false);
    assert!(rt.state.panel.is_some());
    rt.update(|s, tx| s.close_panel(tx));
    rt.frame(0.);
    key(&mut rt, Code::Comma, ctrl(Key::Command(',')), false, false);
    assert!(
        rt.state
            .bindings
            .consumed
            .keys()
            .all(|id| *id != crate::app::bindings::Identity::Physical(Code::Comma))
    );
}

#[test]
fn repeating_pane_focus_and_context_changes() {
    let (mut ws, rx) = tabbed_workspace("single");
    overrides(
        &mut ws,
        json!([{"keys":"ctrl+b>c","action":"client.tab.new"}]),
    );
    let mut rt = runtime(ws);
    let focus = KeyEvent {
        key: Key::Left,
        mods: Mods {
            ctrl: true,
            alt: true,
            ..Mods::default()
        },
    };
    key(&mut rt, Code::ArrowLeft, focus.clone(), true, false);
    key(&mut rt, Code::ArrowLeft, focus, true, true);
    assert_eq!(requests(&rx).len(), 2);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    rt.update(|s, tx| s.perform(Intent::AddHostManually, tx));
    rt.frame(0.);
    assert_eq!(rt.state.bindings.sequence.depth, 0);
    rt.update(|s, tx| s.close_panel(tx));
    rt.frame(0.);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), false, false);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    rt.state.keymap_generation += 1;
    press(&mut rt, Key::Text("c".into()));
    assert!(requests(&rx).is_empty());
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), false, false);
    key(&mut rt, Code::KeyB, ctrl(Key::Command('b')), true, false);
    rt.state.window_focus(false);
    assert_eq!(rt.state.bindings.sequence.depth, 0);
}

#[test]
fn custom_zoom_reaches_focused_query_and_form_without_editing_them() {
    for form in [false, true] {
        let mut ws = workspace();
        overrides(
            &mut ws,
            json!([{"keys":"ctrl+j","action":"client.zoom.in"}]),
        );
        if form {
            ws.perform(Intent::AddHostManually, &mut rune::Tx::default());
        } else {
            ws.picker(true);
        }
        ws.panel.as_mut().unwrap().inputs[0] = rune::TextEdit::new("keep");
        let mut rt = runtime(ws);
        rt.key(ctrl(Key::Command('j')));
        rt.frame(0.);
        assert_eq!(rt.state.settings.ui_scale, 1.1);
        assert_eq!(rt.state.panel.as_ref().unwrap().inputs[0].text, "keep");
    }
}

#[test]
fn send_key_uses_explicit_command_and_new_close_actions_ask() {
    let (mut ws, _) = tabbed_workspace("single");
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:a", &sender);
    let mut rt = runtime(ws);
    rt.update(|s, tx| {
        assert!(actions::run(
            s,
            "pane.send_key",
            &json!({"key":"ctrl+b"}),
            tx
        ));
    });
    rt.frame(0.);
    let keys: Vec<_> = sent
        .try_iter()
        .filter_map(|c| match c {
            HostCommand::Terminal {
                command: Command::SendKey(k),
                ..
            } => Some(k),
            _ => None,
        })
        .collect();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0]["action"], "press");
    assert_eq!(keys[1]["action"], "release");
    rt.update(|s, tx| {
        assert!(actions::run(s, "client.tab.close", &Value::Null, tx));
    });
    rt.frame(0.);
    assert!(rt.state.panel.as_ref().unwrap().title.contains("Close"));
}
