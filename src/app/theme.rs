//! Midnight: Rex's tokens, the kit pinned to them, shared surfaces (chips,
//! glass, cards) and the app's icons.
use rune::{Color, El, Icon, Shadow, Spring, SpringPair, TextStyle, Transition};
use rune_kit::{Kit, PaletteFocus, PaletteSeed};
use std::sync::OnceLock;

/// sRGB `0xRRGGBB` → linear `Color`.
pub fn rgb(value: u32) -> Color {
    let channel = |shift: u32| rune::color::srgb_decode(((value >> shift) & 0xff) as f32 / 255.);
    Color {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: 1.,
    }
}

/// `a` over `b` at opacity `t`, mixed in sRGB.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        ((x * t + y * (1. - t)).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

pub fn white(a: f32) -> Color {
    Color::WHITE.alpha(a)
}
pub fn black(a: f32) -> Color {
    Color::BLACK.alpha(a)
}
pub fn shadow(x: f32, y: f32, blur: f32, color: Color) -> Shadow {
    Shadow {
        x,
        y,
        blur,
        spread: 0.,
        color,
        inset: false,
    }
}
pub fn inset(y: f32, color: Color) -> Shadow {
    Shadow {
        x: 0.,
        y,
        blur: 0.,
        spread: 0.,
        color,
        inset: true,
    }
}

pub const GROUND: u32 = 0x060b14;
/// The card, and the terminal's default background (the server's default black is replaced).
pub const CARD: u32 = 0x05080e;
pub const POPOVER: u32 = 0x0d1524;
pub const INK: u32 = 0xe8eef6;
pub const MUTED: u32 = 0x93a3b8;
pub const ACCENT: u32 = 0x7cc4ff;
pub const ACCENT2: u32 = 0xa78bfa;
pub const LIVE: u32 = 0x5eead4;
pub const WARNING: u32 = 0xf5d48a;
pub const DANGER: u32 = 0xff8a8a;
pub const TERMINAL_SELECTION: u32 = 0x2f4a6e;

pub const TOOLBAR: f32 = 52.;
pub const GUTTER: f32 = 8.;
pub const MARGIN: f32 = 10.;
pub const CARD_RADIUS: f32 = 12.;
pub const CARD_HEADER: f32 = 30.;

/// Springs for chrome: quick and calm.
pub const SNAPPY: Spring = Spring {
    duration: 0.28,
    bounce: 0.0,
};
pub const POP: Spring = Spring {
    duration: 0.32,
    bounce: 0.12,
};
/// For things that must not move (terminal content).
pub const STILL: Spring = Spring {
    duration: 0.001,
    bounce: 0.,
};

/// A sliding highlight's own entrance and exit: none, so one highlight shows even when its
/// matched partner isn't mounted (Reduce Motion, a row scrolled in).
pub fn no_presence() -> Transition {
    Transition::identity().springs(SpringPair::new(STILL, STILL))
}

/// A popover's entrance from its anchor: a little scale and a fade on a gentle spring.
pub fn pop_in() -> Transition {
    Transition::scale(0.96).springs(SpringPair::new(POP, SNAPPY))
}

pub fn kit() -> &'static Kit {
    static KIT: OnceLock<Kit> = OnceLock::new();
    KIT.get_or_init(|| {
        let mut k = Kit::from_palette_focus(
            PaletteSeed {
                bg: rgb(GROUND),
                fg: rgb(INK),
                accent: rgb(ACCENT),
            },
            PaletteFocus::Custom(rgb(ACCENT)),
        );
        let c = &mut k.c;
        (c.page, c.bg_base, c.surface_base) = (rgb(GROUND), rgb(GROUND), rgb(GROUND));
        c.bg_deep = rgb(CARD);
        for slot in [
            &mut c.layer_01,
            &mut c.layer_02,
            &mut c.surface_raised,
            &mut c.surface_raised_base,
            &mut c.surface_float,
        ] {
            *slot = rgb(POPOVER);
        }
        (c.hover, c.surface_hover, c.surface_raised_hover) =
            (white(0.07), white(0.07), white(0.07));
        c.pressed = white(0.11);
        (c.border_base, c.border_muted, c.border_weak) = (white(0.12), white(0.08), white(0.06));
        (c.border_strong, c.border_hover) = (white(0.2), white(0.2));
        c.border_focus = rgb(ACCENT);
        (c.text_base, c.text_strong, c.icon_base) = (rgb(INK), rgb(INK), rgb(INK));
        for slot in [
            &mut c.text_muted,
            &mut c.text_faint,
            &mut c.text_weak,
            &mut c.icon_muted,
            &mut c.icon_faint,
            &mut c.icon_weaker,
        ] {
            *slot = rgb(MUTED);
        }
        (c.text_accent, c.icon_accent) = (rgb(ACCENT), rgb(ACCENT));
        c.success.fg = rgb(LIVE);
        c.warning.fg = rgb(WARNING);
        c.danger.fg = rgb(DANGER);
        k.radius.sm = 8.;
        k.radius.md = 10.;
        k
    })
}

pub fn ui(size: f32) -> TextStyle {
    TextStyle::new(size, 450.)
}
pub fn medium(size: f32) -> TextStyle {
    TextStyle::new(size, 560.)
}
pub fn strong(size: f32) -> TextStyle {
    TextStyle::new(size, 620.)
}
/// Small capitals for group labels.
pub fn label_caps() -> TextStyle {
    TextStyle::new(10.5, 650.).tracking(0.6)
}

/// A glassy pill every control in the bar sits on.
pub fn chip(e: El) -> El {
    e.fill(white(0.055))
        .border_gradient(1., white(0.13), white(0.04))
        .radius(10.)
        .continuous_corners()
}

/// A popover or dialog surface: frosted navy with a lit rim and a deep shadow.
pub fn glass(e: El, radius: f32, blur: bool) -> El {
    let e = e
        .fill(rgb(POPOVER).alpha(0.95))
        .border_gradient(1., white(0.16), white(0.05))
        .radius(radius)
        .continuous_corners()
        .box_shadow(shadow(0., 24., 60., black(0.6)))
        .box_shadow(shadow(0., 2., 6., black(0.35)));
    if blur { e.backdrop_blur(22.) } else { e }
}

macro_rules! icon {
    ($name:ident, $id:literal, $($el:literal),+) => {
        pub static $name: Icon = Icon {
            name: $id,
            view: 20.0,
            body: concat!($("<", $el, r#" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#),+),
        };
    };
}
icon!(
    TERMINAL,
    "rex-terminal",
    r#"path d="m4 6 4 4-4 4""#,
    r#"path d="M10.5 14.5h5.5""#
);
icon!(CHEVRON, "rex-chevron", r#"path d="m7 8.5 3 3 3-3""#);
icon!(RIGHT, "rex-right", r#"path d="m8.5 6.5 3.5 3.5-3.5 3.5""#);
icon!(LEFT, "rex-left", r#"path d="m11.5 6.5-3.5 3.5 3.5 3.5""#);
icon!(CHECK, "rex-check", r#"path d="m5.5 10.5 3 3 6-7""#);
icon!(
    PLUS,
    "rex-plus",
    r#"path d="M10 4.5v11""#,
    r#"path d="M4.5 10h11""#
);
icon!(
    CLOSE,
    "rex-close",
    r#"path d="m6.5 6.5 7 7""#,
    r#"path d="m13.5 6.5-7 7""#
);
icon!(
    SEARCH,
    "rex-search",
    r#"circle cx="9" cy="9" r="5""#,
    r#"path d="m13 13 3.5 3.5""#
);
icon!(
    COMMAND,
    "rex-command",
    r#"path d="M12.5 5v10a2.5 2.5 0 1 0 2.5-2.5H5a2.5 2.5 0 1 0 2.5 2.5V5a2.5 2.5 0 1 0-2.5 2.5h10a2.5 2.5 0 1 0-2.5-2.5""#
);
icon!(
    MONITOR,
    "rex-monitor",
    r#"rect x="3" y="4" width="14" height="9.5" rx="2""#,
    r#"path d="M10 13.5v3""#,
    r#"path d="M7 16.5h6""#
);
icon!(
    SERVER,
    "rex-server",
    r#"rect x="3.5" y="4" width="13" height="5" rx="1.5""#,
    r#"rect x="3.5" y="11" width="13" height="5" rx="1.5""#,
    r#"path d="M6.5 6.5h.01""#,
    r#"path d="M6.5 13.5h.01""#
);
icon!(
    SIDEBAR,
    "rex-sidebar",
    r#"rect x="3" y="4" width="14" height="12" rx="3""#,
    r#"path d="M8 4v12""#
);
icon!(MINIMIZE, "rex-minimize", r#"path d="M6 10h8""#);
icon!(
    MAXIMIZE,
    "rex-maximize",
    r#"rect x="6" y="6" width="8" height="8" rx="2""#
);
icon!(
    RESTORE,
    "rex-restore",
    r#"rect x="5.5" y="7.5" width="7" height="7" rx="1.5""#,
    r#"path d="M8 7.5V5.5h6.5V12h-2""#
);
icon!(MINUS, "rex-minus", r#"path d="M6 10h8""#);
