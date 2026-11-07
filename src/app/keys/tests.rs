use super::*;
use PhysicalKeyCode as C;

#[test]
fn physical_codes_use_ghostty_names() {
    for (code, name) in [
        (C::KeyA, "key_a"),
        (C::Digit7, "digit_7"),
        (C::ArrowUp, "arrow_up"),
        (C::PageDown, "page_down"),
        (C::BracketLeft, "bracket_left"),
        (C::Backquote, "backquote"),
        (C::F12, "f12"),
        (C::Numpad0, "numpad_0"),
        (C::NumpadAdd, "numpad_add"),
        (C::IntlBackslash, "intl_backslash"),
        (C::Enter, "enter"),
    ] {
        assert_eq!(code_name(code), name);
    }
}

#[test]
fn key_payloads() {
    let shift = Mods {
        shift: true,
        ..Default::default()
    };
    let ctrl = Mods {
        ctrl: true,
        cmd: true,
        ..Default::default()
    };
    let typed = KeyEvent {
        key: Key::Text("A".into()),
        mods: shift,
    };
    let p = payload(&typed, Some(C::KeyA), Some('a'), Action::Press).unwrap();
    assert_eq!(p["key"], "key_a");
    assert_eq!(p["mods"], "shift");
    assert_eq!(p["consumed_mods"], "shift");
    assert_eq!(p["text"], "A");
    assert_eq!(p["unshifted_codepoint"], 'a' as u32);

    let interrupt = KeyEvent {
        key: Key::Command('c'),
        mods: ctrl,
    };
    let p = payload(&interrupt, Some(C::KeyC), None, Action::Press).unwrap();
    assert_eq!(
        (p["key"].as_str(), p["mods"].as_str(), p["text"].as_str()),
        (Some("key_c"), Some("ctrl"), Some(""))
    );

    let enter = KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    };
    let p = payload(&enter, Some(C::Enter), None, Action::Repeat).unwrap();
    assert_eq!(
        (p["text"].as_str(), p["action"].as_str()),
        (Some("\n"), Some("repeat"))
    );
    let p = payload(&enter, None, None, Action::Release).unwrap();
    assert_eq!(
        (p["key"].as_str(), p["text"].as_str()),
        (Some("enter"), Some(""))
    );
}

#[test]
fn shift_enter_keeps_shift_for_the_encoder() {
    // With shift marked consumed, Ghostty's Kitty encoder sends a bare `\r` and apps
    // cannot tell Shift+Enter from Enter.
    let shift_enter = KeyEvent {
        key: Key::Enter,
        mods: Mods {
            shift: true,
            ..Default::default()
        },
    };
    let p = payload(&shift_enter, Some(C::Enter), None, Action::Press).unwrap();
    assert_eq!(
        (
            p["mods"].as_str(),
            p["consumed_mods"].as_str(),
            p["text"].as_str()
        ),
        (Some("shift"), Some(""), Some("\n"))
    );
}

#[test]
fn dead_key_waits_for_the_composed_text() {
    let dead = KeyEvent {
        key: Key::Dead(Some('~')),
        mods: Mods::default(),
    };
    let first = payload(&dead, Some(C::Backquote), None, Action::Press).unwrap();
    assert_eq!(
        (
            first["key"].as_str(),
            first["text"].as_str(),
            first["composing"].as_bool()
        ),
        (Some("backquote"), Some(""), Some(true))
    );

    let completed = KeyEvent {
        key: Key::Text("~".into()),
        mods: Mods::default(),
    };
    let last = payload(&completed, Some(C::Space), Some(' '), Action::Press).unwrap();
    assert_eq!(
        (
            last["key"].as_str(),
            last["text"].as_str(),
            last["composing"].as_bool()
        ),
        (Some("space"), Some("~"), Some(false))
    );
}

const NONE: Mods = Mods {
    shift: false,
    cmd: false,
    alt: false,
    ctrl: false,
    logo: false,
};
const SHIFT: Mods = Mods {
    shift: true,
    ..NONE
};
/// Rune sets both off macOS.
const CTRL: Mods = Mods {
    ctrl: true,
    cmd: true,
    ..NONE
};
const ALT: Mods = Mods { alt: true, ..NONE };

fn with(a: Mods, b: Mods) -> Mods {
    Mods {
        shift: a.shift || b.shift,
        cmd: a.cmd || b.cmd,
        alt: a.alt || b.alt,
        ctrl: a.ctrl || b.ctrl,
        logo: a.logo || b.logo,
    }
}

fn ev(key: Key, mods: Mods) -> KeyEvent {
    KeyEvent { key, mods }
}

/// (key, mods, consumed_mods, text, unshifted_codepoint)
fn fields(p: &Value) -> (&str, &str, &str, &str, u64) {
    (
        p["key"].as_str().unwrap(),
        p["mods"].as_str().unwrap(),
        p["consumed_mods"].as_str().unwrap(),
        p["text"].as_str().unwrap(),
        p["unshifted_codepoint"].as_u64().unwrap(),
    )
}

fn text(t: &str) -> Key {
    Key::Text(t.into())
}

#[test]
fn modified_keys_keep_their_modifiers_in_the_payload() {
    type Row<'a> = (
        &'a str,
        KeyEvent,
        Option<C>,
        Option<char>,
        (&'a str, &'a str, &'a str, &'a str, u64),
    );
    let rows: Vec<Row> = vec![
        (
            "Tab",
            ev(Key::Tab, NONE),
            Some(C::Tab),
            None,
            ("tab", "", "", "\t", 0),
        ),
        (
            "Shift+Tab",
            ev(Key::Tab, SHIFT),
            Some(C::Tab),
            None,
            ("tab", "shift", "", "\t", 0),
        ),
        (
            "Enter",
            ev(Key::Enter, NONE),
            Some(C::Enter),
            None,
            ("enter", "", "", "\n", 0),
        ),
        (
            "Shift+Enter",
            ev(Key::Enter, SHIFT),
            Some(C::Enter),
            None,
            ("enter", "shift", "", "\n", 0),
        ),
        (
            "Ctrl+Enter",
            ev(Key::Enter, CTRL),
            Some(C::Enter),
            None,
            ("enter", "ctrl", "", "", 0),
        ),
        (
            "Alt+Enter",
            ev(Key::Enter, ALT),
            Some(C::Enter),
            None,
            ("enter", "alt", "", "", 0),
        ),
        (
            "Ctrl+Shift+Enter",
            ev(Key::Enter, with(SHIFT, CTRL)),
            Some(C::Enter),
            None,
            ("enter", "shift+ctrl", "", "", 0),
        ),
        (
            "Space",
            ev(text(" "), NONE),
            Some(C::Space),
            Some(' '),
            ("space", "", "", " ", 32),
        ),
        (
            "Shift+Space",
            ev(text(" "), SHIFT),
            Some(C::Space),
            Some(' '),
            ("space", "shift", "", " ", 32),
        ),
        (
            "Shift+Up",
            ev(Key::Up, SHIFT),
            Some(C::ArrowUp),
            None,
            ("arrow_up", "shift", "", "", 0),
        ),
        (
            "Ctrl+Left",
            ev(Key::Left, CTRL),
            Some(C::ArrowLeft),
            None,
            ("arrow_left", "ctrl", "", "", 0),
        ),
        (
            "Alt+Right",
            ev(Key::Right, ALT),
            Some(C::ArrowRight),
            None,
            ("arrow_right", "alt", "", "", 0),
        ),
        (
            "Shift+Home",
            ev(Key::Home, SHIFT),
            Some(C::Home),
            None,
            ("home", "shift", "", "", 0),
        ),
        (
            "Ctrl+End",
            ev(Key::End, CTRL),
            Some(C::End),
            None,
            ("end", "ctrl", "", "", 0),
        ),
        (
            "Shift+PageUp",
            ev(Key::PageUp, SHIFT),
            Some(C::PageUp),
            None,
            ("page_up", "shift", "", "", 0),
        ),
        (
            "Ctrl+PageDown",
            ev(Key::PageDown, CTRL),
            Some(C::PageDown),
            None,
            ("page_down", "ctrl", "", "", 0),
        ),
        (
            "Shift+Backspace",
            ev(Key::Backspace, SHIFT),
            Some(C::Backspace),
            None,
            ("backspace", "shift", "", "", 0),
        ),
        (
            "Ctrl+Backspace",
            ev(Key::Backspace, CTRL),
            Some(C::Backspace),
            None,
            ("backspace", "ctrl", "", "", 0),
        ),
        (
            "Alt+Backspace",
            ev(Key::Backspace, ALT),
            Some(C::Backspace),
            None,
            ("backspace", "alt", "", "", 0),
        ),
        (
            "Shift+Delete",
            ev(Key::Delete, SHIFT),
            Some(C::Delete),
            None,
            ("delete", "shift", "", "", 0),
        ),
        (
            "Ctrl+Delete",
            ev(Key::Delete, CTRL),
            Some(C::Delete),
            None,
            ("delete", "ctrl", "", "", 0),
        ),
        (
            "Shift+a",
            ev(text("A"), SHIFT),
            Some(C::KeyA),
            Some('a'),
            ("key_a", "shift", "shift", "A", 97),
        ),
        (
            "Shift+1",
            ev(text("!"), SHIFT),
            Some(C::Digit1),
            Some('1'),
            ("digit_1", "shift", "shift", "!", 49),
        ),
        (
            "Russian Shift+ф",
            ev(text("Ф"), SHIFT),
            Some(C::KeyA),
            Some('ф'),
            ("key_a", "shift", "shift", "Ф", 'ф' as u64),
        ),
        // xkb counts Shift as consumed on a cased key even when Caps Lock cancels it.
        (
            "Caps+Shift+a",
            ev(text("a"), SHIFT),
            Some(C::KeyA),
            Some('a'),
            ("key_a", "shift", "shift", "a", 97),
        ),
        // AltGr is a layout level, not a Rune modifier.
        (
            "AltGr+2 (sv)",
            ev(text("@"), NONE),
            Some(C::Digit2),
            Some('2'),
            ("digit_2", "", "", "@", 50),
        ),
        (
            "Alt+Shift+a",
            ev(text("A"), with(ALT, SHIFT)),
            Some(C::KeyA),
            Some('a'),
            ("key_a", "shift+alt", "shift", "A", 97),
        ),
        (
            "logical Shift+Space",
            ev(text(" "), SHIFT),
            None,
            None,
            ("space", "shift", "", " ", 0),
        ),
        (
            "logical Shift+A",
            ev(text("A"), SHIFT),
            None,
            None,
            ("key_a", "shift", "shift", "A", 0),
        ),
    ];
    for (label, event, code, base, expected) in rows {
        let p = payload(&event, code, base, Action::Press).unwrap();
        assert_eq!(fields(&p), expected, "{label}");
        assert_eq!(p["action"], "press", "{label}");
    }
}

#[test]
fn shifted_dead_key_composition_consumes_shift_for_the_composed_text() {
    let dead = payload(
        &ev(Key::Dead(Some('¨')), NONE),
        Some(C::BracketRight),
        None,
        Action::Press,
    )
    .unwrap();
    assert_eq!(
        (
            fields(&dead).2,
            fields(&dead).3,
            dead["composing"].as_bool()
        ),
        ("", "", Some(true))
    );
    let composed = payload(
        &ev(text("Ä"), SHIFT),
        Some(C::KeyA),
        Some('a'),
        Action::Press,
    )
    .unwrap();
    assert_eq!(fields(&composed), ("key_a", "shift", "shift", "Ä", 97));
    assert_eq!(composed["composing"], false);
}

#[test]
fn press_repeat_and_release_carry_the_same_identity() {
    for (label, event, code, base) in [
        ("Shift+Tab", ev(Key::Tab, SHIFT), C::Tab, None),
        ("Shift+Space", ev(text(" "), SHIFT), C::Space, Some(' ')),
        ("Shift+a", ev(text("A"), SHIFT), C::KeyA, Some('a')),
        ("Ctrl+Left", ev(Key::Left, CTRL), C::ArrowLeft, None),
    ] {
        let [press, repeat, release] = [Action::Press, Action::Repeat, Action::Release]
            .map(|action| payload(&event, Some(code), base, action).unwrap());
        let identity = |p: &Value| {
            let (key, mods, consumed, _, unshifted) = fields(p);
            (
                key.to_owned(),
                mods.to_owned(),
                consumed.to_owned(),
                unshifted,
            )
        };
        assert_eq!(identity(&repeat), identity(&press), "{label} repeat");
        assert_eq!(identity(&release), identity(&press), "{label} release");
        assert_eq!(
            [&press, &repeat, &release].map(|p| p["action"].as_str().unwrap()),
            ["press", "repeat", "release"],
            "{label}"
        );
        assert_eq!(release["text"], "", "{label} release types nothing");
    }
}
