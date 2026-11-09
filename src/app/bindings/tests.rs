use super::*;
use rex_linux::keymap::{Overrides, resolve};
use rune::Mods;
use serde_json::json;

#[test]
fn layout_matrix() {
    for (binding, text, code, base, shift, logo, expected) in [
        (
            "ctrl+shift+t",
            "Е",
            Code::KeyT,
            Some('е'),
            true,
            false,
            true,
        ),
        ("ctrl+,", "б", Code::Comma, Some('б'), false, false, true),
        (
            "ctrl++",
            "+",
            Code::BracketRight,
            Some('+'),
            false,
            false,
            true,
        ),
        ("ctrl+0", "à", Code::Digit0, Some('à'), false, false, true),
        (
            "ctrl+shift+t",
            "T",
            Code::KeyK,
            Some('t'),
            true,
            false,
            true,
        ),
        (
            "ctrl+shift+k",
            "T",
            Code::KeyK,
            Some('t'),
            true,
            false,
            false,
        ),
        (
            "ctrl+shift+5",
            "%",
            Code::Digit5,
            Some('5'),
            true,
            false,
            true,
        ),
        (
            "ctrl+[KeyQ]",
            "a",
            Code::KeyQ,
            Some('a'),
            false,
            false,
            true,
        ),
        ("ctrl+shift+t", "T", Code::KeyK, None, true, false, true),
        ("ctrl+shift+t", "Е", Code::KeyT, None, true, false, true),
        ("ctrl+,", "б", Code::Comma, None, false, false, true),
        ("ctrl+0", "à", Code::Digit0, None, false, false, true),
        ("ctrl+shift+5", "%", Code::Digit5, None, true, false, true),
        ("cmd+k", "k", Code::KeyK, Some('k'), false, false, false),
    ] {
        let ev = KeyEvent {
            key: Key::Text(text.into()),
            mods: Mods {
                ctrl: true,
                shift,
                logo,
                ..Mods::default()
            },
        };
        assert_eq!(
            matches(
                &Chord::parse(binding).unwrap(),
                Input {
                    event: &ev,
                    code: Some(code),
                    base,
                    repeat: false
                }
            ),
            expected,
            "{binding}, {text}, {base:?}"
        );
    }
    let ev = KeyEvent {
        key: Key::Command('k'),
        mods: Mods {
            logo: true,
            ..Mods::default()
        },
    };
    assert!(matches(
        &Chord::parse("cmd+k").unwrap(),
        Input {
            event: &ev,
            code: None,
            base: None,
            repeat: false
        }
    ));
    assert!(!matches(
        &Chord::parse("ctrl+k").unwrap(),
        Input {
            event: &ev,
            code: None,
            base: None,
            repeat: false
        }
    ));
}

#[test]
fn sequences_complete_cancel_and_do_not_retry_failed_tail() {
    let map = resolve(
        &[],
        &Overrides::parse(&json!([
            {"keys":"ctrl+b>c>s", "action":"session.switch"},
            {"keys":"ctrl+b>z", "action":"pane.zoom"},
            {"keys":"x", "action":"client.settings"}
        ])),
        &[],
    );
    let mut seq = Sequence::default();
    let mut step = |key: Key, ctrl| {
        let event = KeyEvent {
            key,
            mods: Mods {
                ctrl,
                ..Mods::default()
            },
        };
        seq.advance(
            &map,
            Input {
                event: &event,
                code: None,
                base: None,
                repeat: false,
            },
            |_| true,
        )
    };
    assert_eq!(step(Key::Command('b'), true), Step::Pending);
    assert_eq!(step(Key::Text("c".into()), false), Step::Pending);
    assert!(matches!(
        step(Key::Text("s".into()), false),
        Step::Run { sequence: true, .. }
    ));
    assert_eq!(step(Key::Command('b'), true), Step::Pending);
    assert_eq!(step(Key::Escape, false), Step::Pending);
    assert_eq!(step(Key::Text("z".into()), false), Step::None);
    assert_eq!(step(Key::Command('b'), true), Step::Pending);
    assert_eq!(step(Key::Text("x".into()), false), Step::None);
    assert!(matches!(
        step(Key::Text("x".into()), false),
        Step::Run {
            sequence: false,
            ..
        }
    ));
}

#[test]
fn newest_available_binding_decides_sequence_or_single() {
    for (entries, expected) in [
        (
            json!([{"keys":"ctrl+b", "action":"pane.zoom"},{"keys":"ctrl+b>c", "action":"client.tab.new"}]),
            true,
        ),
        (
            json!([{"keys":"ctrl+b>c", "action":"client.tab.new"},{"keys":"ctrl+b", "action":"pane.zoom"}]),
            false,
        ),
    ] {
        let map = resolve(&[], &Overrides::parse(&entries), &[]);
        let event = KeyEvent {
            key: Key::Command('b'),
            mods: Mods {
                ctrl: true,
                ..Mods::default()
            },
        };
        let input = Input {
            event: &event,
            code: None,
            base: None,
            repeat: false,
        };
        let step = Sequence::default().advance(&map, input, |_| true);
        assert_eq!(step == Step::Pending, expected);
        let step = Sequence::default().advance(&map, input, |i| i != 0);
        assert_eq!(step == Step::Pending, !expected);
    }
}

#[test]
fn modifier_only_passes_at_depth_zero_but_does_not_cancel_a_prefix() {
    let map = resolve(
        &[],
        &Overrides::parse(&json!([{"keys":"ctrl+b>c","action":"pane.zoom"}])),
        &[],
    );
    let mut seq = Sequence::default();
    let modifier = KeyEvent {
        key: Key::Modifier {
            modifier: rune::Modifier::ShiftLeft,
            pressed: true,
        },
        mods: Mods::default(),
    };
    let input = Input {
        event: &modifier,
        code: None,
        base: None,
        repeat: false,
    };
    assert_eq!(seq.advance(&map, input, |_| true), Step::None);
    let prefix = KeyEvent {
        key: Key::Command('b'),
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    };
    assert_eq!(
        seq.advance(
            &map,
            Input {
                event: &prefix,
                ..input
            },
            |_| true
        ),
        Step::Pending
    );
    assert_eq!(seq.advance(&map, input, |_| true), Step::Pending);
    assert_eq!(seq.depth, 1);
}

#[test]
fn send_key_uses_payload_builder_for_both_edges_and_rejects_f13() {
    let [press, release] = crate::app::keys::synthesize("ctrl+b").unwrap();
    assert_eq!(press["key"], "key_b");
    assert_eq!(press["mods"], "ctrl");
    assert_eq!(press["action"], "press");
    assert_eq!(release["action"], "release");
    assert_eq!(release["text"], "");
    assert!(crate::app::keys::synthesize("f13").is_none());
    assert!(crate::app::keys::synthesize("ctrl+b>c").is_none());
    let [press, _] = crate::app::keys::synthesize("ctrl+[KeyT]").unwrap();
    assert_eq!(press["key"], "key_t");
    assert_eq!(press["unshifted_codepoint"], 0);
}
