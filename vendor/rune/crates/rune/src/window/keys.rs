//! winit keys and modifiers as Rune's.
use winit::keyboard::{Key as WKey, ModifiersState, NamedKey};

use crate::edit::{Key, KeyEvent, Mods};

pub(crate) fn map_mods(m: ModifiersState) -> Mods {
    Mods { shift: m.shift_key(), cmd: if cfg!(target_os = "macos") { m.super_key() } else { m.control_key() },
        alt: m.alt_key(), ctrl: m.control_key(), logo: m.super_key() }
}

pub(crate) fn map_key(key: &WKey, m: ModifiersState) -> Option<KeyEvent> {
    let mods = map_mods(m);
    let key = match key {
        WKey::Named(n) => match n {
            NamedKey::Enter => Key::Enter,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::ArrowLeft => Key::Left,
            NamedKey::ArrowRight => Key::Right,
            NamedKey::ArrowUp => Key::Up,
            NamedKey::ArrowDown => Key::Down,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::Escape => Key::Escape,
            NamedKey::Tab => Key::Tab,
            NamedKey::Space => Key::Text(" ".into()),
            NamedKey::F1 => Key::Function(1),
            NamedKey::F2 => Key::Function(2),
            NamedKey::F3 => Key::Function(3),
            NamedKey::F4 => Key::Function(4),
            NamedKey::F5 => Key::Function(5),
            NamedKey::F6 => Key::Function(6),
            NamedKey::F7 => Key::Function(7),
            NamedKey::F8 => Key::Function(8),
            NamedKey::F9 => Key::Function(9),
            NamedKey::F10 => Key::Function(10),
            NamedKey::F11 => Key::Function(11),
            NamedKey::F12 => Key::Function(12),
            NamedKey::F13 => Key::Function(13),
            NamedKey::F14 => Key::Function(14),
            NamedKey::F15 => Key::Function(15),
            NamedKey::F16 => Key::Function(16),
            NamedKey::F17 => Key::Function(17),
            NamedKey::F18 => Key::Function(18),
            NamedKey::F19 => Key::Function(19),
            NamedKey::F20 => Key::Function(20),
            NamedKey::F21 => Key::Function(21),
            NamedKey::F22 => Key::Function(22),
            NamedKey::F23 => Key::Function(23),
            NamedKey::F24 => Key::Function(24),
            NamedKey::ContextMenu => Key::ContextMenu,
            _ => return None,
        },
        WKey::Character(c) => {
            if mods.cmd {
                Key::Command(c.chars().next()?.to_ascii_lowercase())
            } else {
                Key::Text(c.to_string())
            }
        }
        WKey::Dead(accent) => Key::Dead(*accent),
        _ => return None,
    };
    Some(KeyEvent { key, mods })
}

/// `event`'s logical key, with the text a dead key composed on it. winit keeps a named key's
/// name (Space after a dead tilde) and carries the composed `~` only in `text`.
pub(crate) fn event_key(event: &winit::event::KeyEvent, m: ModifiersState) -> Option<KeyEvent> {
    map_key(&event.logical_key, m).map(|ev| with_composed_text(ev, event.text.as_deref()))
}

fn with_composed_text(mut ev: KeyEvent, text: Option<&str>) -> KeyEvent {
    if let (Key::Text(key), Some(text)) = (&mut ev.key, text) && !text.is_empty() && key != text {
        *key = text.to_owned();
    }
    ev
}

/// The character `event`'s key types under the current layout with no modifiers, for
/// `Tx::key_base`.
pub(crate) fn key_base(event: &winit::event::KeyEvent) -> Option<char> {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
        match event.key_without_modifiers() {
            WKey::Character(s) => { let mut c = s.chars(); c.next().filter(|_| c.next().is_none()) }
            WKey::Named(NamedKey::Space) => Some(' '),
            _ => None,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    { let _ = event; None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_keys_reach_logical_key_handlers() {
        let event = map_key(&WKey::Dead(Some('~')), ModifiersState::empty()).unwrap();
        assert_eq!(event.key, Key::Dead(Some('~')));
    }

    #[test]
    fn a_dead_key_composed_on_space_types_its_accent() {
        let space = map_key(&WKey::Named(NamedKey::Space), ModifiersState::empty()).unwrap();
        assert_eq!(with_composed_text(space.clone(), Some("~")).key, Key::Text("~".into()));
        // A release carries no composed text, and a plain space types itself.
        assert_eq!(with_composed_text(space.clone(), None).key, Key::Text(" ".into()));
        assert_eq!(with_composed_text(space, Some(" ")).key, Key::Text(" ".into()));
    }
}
