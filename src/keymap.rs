//! Rex key spelling and layered workspace shortcuts.
//!
//! Adapted from Andrea Zito (nivox)'s `8037d78` modifier aliases, sequence syntax
//! and tests. Rex's recorded `client.keymap` reply is the spelling authority.
use anyhow::{Result, bail, ensure};

/// `cmd` in Rex means the logo/Super key, not Rune's platform shortcut modifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Character(char),
    Named(&'static str),
    Physical(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub modifiers: Modifiers,
    pub key: Key,
}

const NAMED: &[&str] = &[
    "enter",
    "escape",
    "tab",
    "space",
    "backspace",
    "delete",
    "insert",
    "left",
    "right",
    "up",
    "down",
    "home",
    "end",
    "pageup",
    "pagedown",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
    "f7",
    "f8",
    "f9",
    "f10",
    "f11",
    "f12",
];

// W3C KeyboardEvent.code spellings, independent of Rune so protocol-only builds
// can parse overrides too. Unknown codes must not silently become dead shortcuts.
const PHYSICAL: &[&str] = &[
    "Backquote",
    "Backslash",
    "BracketLeft",
    "BracketRight",
    "Comma",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "Equal",
    "IntlBackslash",
    "IntlRo",
    "IntlYen",
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Minus",
    "Period",
    "Quote",
    "Semicolon",
    "Slash",
    "AltLeft",
    "AltRight",
    "Backspace",
    "CapsLock",
    "ContextMenu",
    "ControlLeft",
    "ControlRight",
    "Enter",
    "MetaLeft",
    "MetaRight",
    "ShiftLeft",
    "ShiftRight",
    "Space",
    "Tab",
    "Convert",
    "KanaMode",
    "Lang1",
    "Lang2",
    "Lang3",
    "Lang4",
    "Lang5",
    "NonConvert",
    "Delete",
    "End",
    "Help",
    "Home",
    "Insert",
    "PageDown",
    "PageUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "NumLock",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "NumpadAdd",
    "NumpadBackspace",
    "NumpadClear",
    "NumpadClearEntry",
    "NumpadComma",
    "NumpadDecimal",
    "NumpadDivide",
    "NumpadEnter",
    "NumpadEqual",
    "NumpadHash",
    "NumpadMemoryAdd",
    "NumpadMemoryClear",
    "NumpadMemoryRecall",
    "NumpadMemoryStore",
    "NumpadMemorySubtract",
    "NumpadMultiply",
    "NumpadParenLeft",
    "NumpadParenRight",
    "NumpadStar",
    "NumpadSubtract",
    "Escape",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
    "Fn",
    "FnLock",
    "PrintScreen",
    "ScrollLock",
    "Pause",
    "BrowserBack",
    "BrowserFavorites",
    "BrowserForward",
    "BrowserHome",
    "BrowserRefresh",
    "BrowserSearch",
    "BrowserStop",
    "Eject",
    "LaunchApp1",
    "LaunchApp2",
    "LaunchMail",
    "MediaPlayPause",
    "MediaSelect",
    "MediaStop",
    "MediaTrackNext",
    "MediaTrackPrevious",
    "Power",
    "Sleep",
    "AudioVolumeDown",
    "AudioVolumeMute",
    "AudioVolumeUp",
    "WakeUp",
    "Again",
    "Copy",
    "Cut",
    "Find",
    "Open",
    "Paste",
    "Props",
    "Select",
    "Undo",
];

impl Chord {
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        // split('+') alone loses the literal plus key (German layouts).
        let (mods, key) = if input == "+" {
            ("", "+")
        } else if let Some(mods) = input.strip_suffix("++") {
            (mods, "+")
        } else {
            input.rsplit_once('+').unwrap_or(("", input))
        };
        let mut modifiers = Modifiers::default();
        if !mods.is_empty() {
            for modifier in mods.split('+') {
                let modifier = modifier.trim().to_ascii_lowercase();
                let enabled = match modifier.as_str() {
                    "ctrl" | "control" => &mut modifiers.ctrl,
                    "alt" | "opt" | "option" => &mut modifiers.alt,
                    "shift" => &mut modifiers.shift,
                    "cmd" | "command" | "super" | "meta" | "win" => &mut modifiers.logo,
                    _ => bail!("unknown modifier {modifier:?}"),
                };
                ensure!(!*enabled, "duplicate modifier {modifier:?}");
                *enabled = true;
            }
        }
        // A leading separator is not an empty modifier list.
        ensure!(!input.starts_with('+') || input == "+", "empty modifier");
        let key = key.trim();
        let key = if let Some(code) = key.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            Key::Physical(
                PHYSICAL
                    .iter()
                    .copied()
                    .find(|name| name.eq_ignore_ascii_case(code))
                    .ok_or_else(|| anyhow::anyhow!("unknown physical key {key:?}"))?,
            )
        } else if let Some(name) = NAMED.iter().copied().find(|n| n.eq_ignore_ascii_case(key)) {
            Key::Named(name)
        } else {
            let lower = key.to_lowercase();
            let mut chars = lower.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if !c.is_control() && !c.is_whitespace() && c != '>' => {
                    Key::Character(c)
                }
                _ => bail!("unknown key {key:?}"),
            }
        };
        Ok(Self { modifiers, key })
    }

    pub fn spelling(&self) -> String {
        let mut parts = Vec::new();
        for (enabled, name) in [
            (self.modifiers.ctrl, "ctrl"),
            (self.modifiers.alt, "alt"),
            (self.modifiers.shift, "shift"),
            (self.modifiers.logo, "cmd"),
        ] {
            if enabled {
                parts.push(name.to_owned());
            }
        }
        parts.push(match self.key {
            Key::Character(c) => c.to_string(),
            Key::Named(name) => name.into(),
            Key::Physical(code) => format!("[{code}]"),
        });
        parts.join("+")
    }

    pub fn keycaps(&self) -> Vec<String> {
        let mut caps = Vec::new();
        for (enabled, label) in [
            (self.modifiers.ctrl, "Ctrl"),
            (self.modifiers.alt, "Alt"),
            (self.modifiers.shift, "Shift"),
            (self.modifiers.logo, "Super"),
        ] {
            if enabled {
                caps.push(label.into());
            }
        }
        caps.push(match self.key {
            Key::Character(c) => c.to_uppercase().to_string(),
            Key::Physical(code) => code.into(),
            Key::Named("pageup") => "PageUp".into(),
            Key::Named("pagedown") => "PageDown".into(),
            Key::Named(name) => {
                let mut label = name.to_owned();
                label[..1].make_ascii_uppercase();
                label
            }
        });
        caps
    }
}

/// User-file syntax only. Wire keys are already split, and must not be joined
/// then reparsed: an unreadable server spelling is preserved verbatim.
pub fn normalize(input: &str) -> Result<Vec<String>> {
    input
        .split('>')
        .map(|part| Chord::parse(part).map(|chord| chord.spelling()))
        .collect()
}

/// Parse a server binding once. Failure disables the entire binding, not just
/// its unreadable chord; keep the original keys separately for display/prefix.
pub fn parse_keys(keys: &[String]) -> Result<Vec<Chord>> {
    ensure!(!keys.is_empty(), "empty key sequence");
    keys.iter().map(|key| Chord::parse(key)).collect()
}

/// Unknown server spellings stay visible, but are never usable as match keys.
pub fn display(keys: &[String]) -> String {
    keys.iter()
        .map(|key| {
            Chord::parse(key)
                .map(|chord| chord.keycaps().join(" "))
                .unwrap_or_else(|_| key.clone())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_and_order_follow_the_recorded_server_spelling() {
        for logo in ["cmd", "command", "super", "meta", "win"] {
            assert_eq!(
                normalize(&format!("{logo}+SHIFT+Option+Control+K")).unwrap(),
                ["ctrl+alt+shift+cmd+k"]
            );
        }
        assert_eq!(
            normalize(" opt + control + K > Shift+5 ").unwrap(),
            ["ctrl+alt+k", "shift+5"]
        );
        assert_eq!(
            normalize("Win+SHIFT+D").unwrap(),
            normalize("shift+cmd+d").unwrap()
        );
        let chord = Chord::parse("cmd+k").unwrap();
        assert!(chord.modifiers.logo && !chord.modifiers.ctrl);
    }

    #[test]
    fn punctuation_named_keys_and_physical_codes_round_trip() {
        for input in [
            "ctrl++",
            "+",
            "ctrl+shift++",
            "ctrl+=",
            "ctrl+,",
            "shift+5",
            "ctrl+é",
            "[",
        ] {
            assert_eq!(normalize(input).unwrap(), [input]);
        }
        assert_eq!(normalize("CTRL+[keyt]").unwrap(), ["ctrl+[KeyT]"]);
        for key in NAMED {
            assert_eq!(Chord::parse(key).unwrap().spelling(), *key);
        }
        for code in PHYSICAL {
            let input = format!("ctrl+[{code}]");
            assert_eq!(Chord::parse(&input).unwrap().spelling(), input);
        }
    }

    #[test]
    fn invalid_chords_are_rejected_not_silently_normalized() {
        for input in [
            "",
            "ctrl+b>",
            "ctrl+",
            "ctrl++t",
            "+k",
            "++",
            "ctrl+++",
            "ctrl+k+t",
            "ctrl+control+k",
            "hyper+k",
            "f0",
            "f13",
            "f24",
            "ctrl+[Foo]",
            "ctrl+[KeyT",
            "ctrl+\n",
            "ctrl+ ",
        ] {
            assert!(normalize(input).is_err(), "{input:?}");
        }
        assert!(parse_keys(&[]).is_err());
        assert!(parse_keys(&["ctrl+b>c".into()]).is_err());
    }

    #[test]
    fn display_preserves_unknown_spelling_without_matching_it() {
        for (keys, expected) in [
            (vec!["ctrl+shift+t".into()], "Ctrl Shift T"),
            (vec!["ctrl+b".into(), "c".into()], "Ctrl B, C"),
            (vec!["ctrl+[KeyT]".into()], "Ctrl KeyT"),
            (vec!["shift+pageup".into()], "Shift PageUp"),
            (vec!["ctrl++".into()], "Ctrl +"),
            (vec!["meta+k".into()], "Super K"),
        ] {
            assert_eq!(display(&keys), expected);
        }
        let keys = vec!["ctrl+b".into(), "ctrl+FutureKey".into()];
        assert!(parse_keys(&keys).is_err());
        assert_eq!(display(&keys), "Ctrl B, ctrl+FutureKey");
    }

    #[test]
    fn recorded_readable_keys_use_server_spelling() {
        let reply: rexrs::api::client::KeymapResponse =
            serde_json::from_str(include_str!("../tests/fixtures/keymap/response.json")).unwrap();
        for binding in reply.bindings {
            if binding.keys == ["ctrl+FutureKey"] {
                assert!(parse_keys(&binding.keys).is_err());
            } else {
                let chords = parse_keys(&binding.keys).unwrap();
                assert_eq!(
                    chords.iter().map(Chord::spelling).collect::<Vec<_>>(),
                    binding.keys
                );
            }
        }
    }
}
