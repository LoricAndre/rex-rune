//! Rune key events → Rex `client.key` payloads (Ghostty key names, by physical position).
use rune::{Key, KeyEvent, Mods, PhysicalKeyCode};
use serde_json::{Value, json};

/// Ghostty's key name for a W3C code: `KeyA` → `key_a`, `Digit1` → `digit_1`,
/// `ArrowUp` → `arrow_up`, `Numpad0` → `numpad_0`, `F12` → `f12`.
pub fn code_name(code: PhysicalKeyCode) -> String {
    let debug = format!("{code:?}");
    if let Some(n) = debug.strip_prefix('F')
        && !n.is_empty()
        && n.bytes().all(|b| b.is_ascii_digit())
    {
        return format!("f{n}");
    }
    let mut out = String::with_capacity(debug.len() + 4);
    let mut previous: Option<char> = None;
    for ch in debug.chars() {
        let boundary = match previous {
            Some(p) => {
                (ch.is_ascii_uppercase() && (p.is_ascii_lowercase() || p.is_ascii_digit()))
                    || (ch.is_ascii_digit() && p.is_ascii_alphabetic())
            }
            None => false,
        };
        if boundary {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
        previous = Some(ch);
    }
    out
}

/// A logical key without a physical origin (tests, scripts) still needs a Rex name.
fn logical_name(key: &Key) -> Option<String> {
    Some(
        match key {
            Key::Enter => "enter",
            Key::Backspace => "backspace",
            Key::Delete => "delete",
            Key::Left => "arrow_left",
            Key::Right => "arrow_right",
            Key::Up => "arrow_up",
            Key::Down => "arrow_down",
            Key::Home => "home",
            Key::End => "end",
            Key::PageUp => "page_up",
            Key::PageDown => "page_down",
            Key::Escape => "escape",
            Key::Tab => "tab",
            Key::Function(n) => return Some(format!("f{n}")),
            Key::Text(t) if t == " " => "space",
            Key::Command(c) => return char_name(*c),
            Key::Text(t) => {
                return t
                    .chars()
                    .next()
                    .filter(|_| t.chars().count() == 1)
                    .and_then(char_name);
            }
            _ => return None,
        }
        .to_owned(),
    )
}

fn char_name(c: char) -> Option<String> {
    let c = c.to_ascii_lowercase();
    Some(match c {
        'a'..='z' => format!("key_{c}"),
        '0'..='9' => format!("digit_{c}"),
        '-' => "minus".into(),
        '=' => "equal".into(),
        '[' => "bracket_left".into(),
        ']' => "bracket_right".into(),
        '\\' => "backslash".into(),
        ';' => "semicolon".into(),
        '\'' => "quote".into(),
        ',' => "comma".into(),
        '.' => "period".into(),
        '/' => "slash".into(),
        '`' => "backquote".into(),
        ' ' => "space".into(),
        _ => return None,
    })
}

pub fn mods(m: Mods) -> String {
    let ctrl = m.ctrl || (m.cmd && !cfg!(target_os = "macos"));
    [
        (m.shift, "shift"),
        (ctrl, "ctrl"),
        (m.alt, "alt"),
        (m.logo, "super"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect::<Vec<_>>()
    .join("+")
}

/// Text a key types into the terminal: what the server expects (Enter `\n`, Tab `\t`, no text
/// with Ctrl/Alt/Super).
fn text(ev: &KeyEvent) -> String {
    let m = ev.mods;
    let plain = !(m.ctrl || m.cmd || m.logo);
    match &ev.key {
        Key::Text(t) if plain || m.alt => t.clone(),
        Key::Enter if plain && !m.alt => "\n".into(),
        Key::Tab if plain && !m.alt => "\t".into(),
        _ => String::new(),
    }
}

pub enum Action {
    Press,
    Repeat,
    Release,
}

/// Returns `None` for keys Rex has no name for.
pub fn payload(
    ev: &KeyEvent,
    physical: Option<PhysicalKeyCode>,
    base: Option<char>,
    action: Action,
) -> Option<Value> {
    let name = physical.map(code_name).or_else(|| logical_name(&ev.key))?;
    let action = match action {
        Action::Press => "press",
        Action::Repeat => "repeat",
        Action::Release => "release",
    };
    let text = if action == "release" {
        String::new()
    } else {
        text(ev)
    };
    let unshifted = base
        .or_else(|| match &ev.key {
            Key::Command(c) => Some(c.to_ascii_lowercase()),
            Key::Text(t) if t.chars().count() == 1 && !ev.mods.shift => t.chars().next(),
            _ => None,
        })
        .map_or(0, |c| c as u32);
    // Shift is consumed only when the layout used it to pick the text (`A`); Enter and Tab type
    // the same text either way, so their shift must stay visible to the encoder.
    let consumed = ev.mods.shift && !text.is_empty() && matches!(ev.key, Key::Text(_));
    Some(json!({
        "key": name,
        "mods": mods(ev.mods),
        "consumed_mods": if consumed { "shift" } else { "" },
        "text": text,
        "unshifted_codepoint": unshifted,
        "composing": matches!(ev.key, Key::Dead(_)),
        "action": action,
    }))
}

#[cfg(test)]
mod tests {
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
}
