//! Modal forms, the destructive confirmation, Preferences and notices.
use super::widgets::{Pill, glyph, pill};
use super::{
    panel::{Form, Intent, PanelKind},
    popups::target_outline,
    theme::*,
    workspace::Workspace,
};
use rune::{Cursor, El, Tx, col, el, row, text};
use rune_kit::{IconButton, Keybind, Switch, TextField};

fn modal(s: &Workspace, viewport: [f32; 2], body: El) -> El {
    let width = 520f32.min(viewport[0] * 0.9);
    let panel = glass(
        col()
            .key("dialog")
            .role("dialog")
            .w(width)
            .max_h(viewport[1] * 0.85)
            .padding(22.)
            .gap(14.),
        16.,
        true,
    )
    .transition(pop_in())
    .child(body);
    let mut layer = el()
        .key("modal")
        .pin(Some(0.), Some(0.), Some(0.), Some(0.))
        .modal(false, |s: &mut Workspace| s.cancel_panel_quiet())
        .child(
            el().key("scrim")
                .pin(Some(0.), Some(0.), Some(0.), Some(0.))
                // No blur: it would hide the pane a confirmation names, and re-blur every
                // frame while terminals stream behind it.
                .fill(black(0.5))
                .on_press_tx(|s: &mut Workspace, tx: &mut Tx| s.cancel_panel(tx)),
        );
    if let Some(outline) = target_outline(s) {
        layer = layer.child(outline);
    }
    layer.child(
        col()
            .pin(Some(0.), Some(0.), Some(0.), Some(0.))
            .center()
            .child(panel),
    )
}

fn title(t: &str) -> El {
    text(t.to_owned()).font(strong(17.)).color(rgb(INK))
}

fn form(s: &Workspace, form: &Form) -> El {
    let panel = s.panel.as_ref().expect("form panel");
    let k = kit();
    let mut body = col().gap(12.).child(title(&panel.title));
    for (i, edit) in panel.inputs.iter().enumerate() {
        let (label, placeholder) = &panel.fields[i];
        body = body.child(
            TextField::new(label.clone(), edit)
                .placeholder(placeholder.clone())
                .on_edit(move |s: &mut Workspace| s.edit_input(i))
                .build(k)
                .on_key(super::input::zoom_key)
                .key(format!("field:{i}")),
        );
    }
    if let Some(error) = &panel.error {
        body = body.child(
            text(error.clone())
                .font(ui(13.))
                .color(rgb(DANGER))
                .key("form-error"),
        );
    }
    if let Form::HostShell(_) = form {
        body = body.child(
            text("Used for new sessions, tabs and splits on this host. Running terminals are unchanged.")
                .font(ui(12.))
                .color(rgb(MUTED))
                .wrap(),
        );
    }
    let primary = "Save";
    body.child(
        row()
            .gap(8.)
            .justify_end()
            .child(pill(
                "cancel-form",
                "Cancel",
                Pill::Neutral,
                32.,
                |s, tx| s.cancel_panel(tx),
            ))
            .child(
                pill("save-form", primary, Pill::Primary, 32., |s, tx| {
                    s.submit(tx)
                })
                .default_button(),
            ),
    )
}

fn confirm(panel_title: &str, description: &str) -> El {
    col()
        .gap(12.)
        .child(title(panel_title))
        .child(
            text(description.to_owned())
                .font(ui(13.))
                .color(rgb(MUTED))
                .wrap(),
        )
        .child(
            row()
                .gap(8.)
                .justify_end()
                .child(
                    pill(
                        "cancel-close",
                        "Keep running",
                        Pill::Neutral,
                        32.,
                        |s, tx| s.close_panel(tx),
                    )
                    .default_button()
                    .tab_index(1),
                )
                .child(
                    pill(
                        "confirm-close",
                        "Close and end programs",
                        Pill::Danger,
                        32.,
                        |s, tx| s.submit(tx),
                    )
                    .tab_index(2),
                ),
        )
}

fn preferences(s: &Workspace) -> El {
    let k = kit();
    let shortcuts: &[(&[&str], &str)] = &[
        (&["Ctrl", "Shift", "C"], "Copy"),
        (&["Ctrl", "Shift", "V"], "Paste"),
        (&["Ctrl", "K"], "Switch session"),
        (&["Ctrl", "Shift", "P"], "Commands"),
        (&["Ctrl", "PageUp"], "Previous tab"),
        (&["Ctrl", "PageDown"], "Next tab"),
        (&["Alt", "Arrows"], "Move pane focus"),
        (&["Ctrl", "Shift", "D"], "Split right"),
        (&["Ctrl", "Alt", "D"], "Split down"),
        (&["Ctrl", "Shift", "Enter"], "Zoom pane"),
        (&["Ctrl", "Shift", "Space"], "Pane actions"),
        (&["Ctrl", "Shift", "W"], "Close pane or tab"),
        (&["Ctrl", "Shift", "B"], "Toggle sidebar"),
    ];
    let mut table = col().gap(6.);
    for (keys, label) in shortcuts {
        table = table.child(
            row()
                .gap(12.)
                .center_y()
                .child(el().w(200.).child(Keybind::new(keys).build(k)))
                .child(text(*label).font(ui(12.)).color(rgb(MUTED))),
        );
    }
    let font = s.terminal_family.unwrap_or("IBM Plex Mono (bundled)");
    col()
        .gap(14.)
        .child(
            row()
                .justify_between()
                .center_y()
                .child(title("Workspace preferences"))
                .child(
                    IconButton::new(&CLOSE)
                        .label("Close")
                        .on_press_tx(|s: &mut Workspace, tx: &mut Tx| s.close_panel(tx))
                        .build(k)
                        .tab_index(4)
                        .key("close-preferences"),
                ),
        )
        .child(
            row()
                .gap(12.)
                .center_y()
                .child(
                    text("Terminal font size")
                        .font(ui(13.))
                        .color(rgb(INK))
                        .grow(1.),
                )
                .child(
                    IconButton::new(&MINUS)
                        .label("Smaller")
                        .on_press(|s: &mut Workspace| s.change_font(-1.))
                        .build(k)
                        .key("font-smaller")
                        .tab_index(1),
                )
                .child(
                    text(format!("{} pt", s.settings.font_size))
                        .font(ui(13.))
                        .color(rgb(INK)),
                )
                .child(
                    IconButton::new(&PLUS)
                        .label("Larger")
                        .on_press(|s: &mut Workspace| s.change_font(1.))
                        .build(k)
                        .key("font-larger")
                        .tab_index(2),
                ),
        )
        .child(
            Switch::new(s.sidebar.is_open())
                .label("Show session sidebar")
                .on_toggle_tx(|s: &mut Workspace, tx: &mut Tx| {
                    s.perform(Intent::Sidebar, tx);
                })
                .build(k)
                .key("sidebar-switch")
                .tab_index(3),
        )
        .child(
            text(format!(
                "Font: {font}\nPreferences: {}",
                s.settings_path.display()
            ))
            .font(ui(11.))
            .color(rgb(MUTED))
            .wrap()
            .selectable(true),
        )
        .child(table)
        // Return acts on the focused control here, as Space does: these are steppers, and
        // there is no default button to compete with.
        .on_key(|s: &mut Workspace, ev: &rune::KeyEvent, tx: &mut Tx| {
            if ev.key != rune::Key::Enter {
                return false;
            }
            match tx
                .focused()
                .map(|f| f.rsplit('/').next().unwrap_or(f).to_owned())
                .as_deref()
            {
                Some("font-smaller") => s.change_font(-1.),
                Some("font-larger") => s.change_font(1.),
                Some("sidebar-switch") => s.perform(Intent::Sidebar, tx),
                Some("close-preferences") => s.close_panel(tx),
                _ => return false,
            }
            true
        })
}

pub fn dialog(s: &Workspace, viewport: [f32; 2]) -> Option<El> {
    let panel = s.panel.as_ref()?;
    let body = match &panel.kind {
        PanelKind::Form(f) => form(s, f),
        PanelKind::Confirm { description, .. } => confirm(&panel.title, description),
        PanelKind::Settings => preferences(s),
        _ => return None,
    };
    Some(modal(s, viewport, body))
}

fn toast(key: String, message: String, close: impl Fn(&mut Workspace) + 'static) -> El {
    glass(
        row()
            .key(key)
            .role("status")
            .w_full()
            .pl(14.)
            .pr(6.)
            .py(8.)
            .gap(10.)
            .center_y(),
        12.,
        false,
    )
    .fill(rgb(POPOVER).alpha(0.96))
    .transition(pop_in())
    .preserve_focus_on_press(true)
    .child(
        el().size(7., 7.)
            .radius(3.5)
            .fill(rgb(WARNING))
            .box_shadow(shadow(0., 0., 8., rgb(WARNING).alpha(0.6))),
    )
    .child(
        text(message)
            .font(ui(12.5))
            .color(rgb(INK))
            .wrap()
            .grow(1.)
            .shrink(),
    )
    .child(
        row()
            .role("button")
            .label("Dismiss")
            .size(24., 24.)
            .center()
            .radius(8.)
            .hover(white(0.08))
            .cursor(Cursor::Pointer)
            .preserve_focus_on_press(true)
            .on_press(close)
            .child(glyph(&CLOSE, 13., rgb(MUTED))),
    )
}

pub fn toasts(s: &Workspace) -> Option<El> {
    let host = s.model().notice.clone();
    if s.notices.is_empty() && host.is_none() {
        return None;
    }
    let mut region = col()
        .key("toasts")
        .role("status")
        .pin(None, Some(TOOLBAR + 4.), Some(MARGIN + 6.), None)
        .w(340.)
        .gap(8.);
    if let Some(text) = host {
        let active = s.active;
        region = region.child(toast(
            "toast-host".into(),
            text,
            move |s: &mut Workspace| s.dismiss_host_notice(active),
        ));
    }
    let room = if s.model().notice.is_some() { 2 } else { 3 };
    for notice in s.notices.iter().rev().take(room) {
        let id = notice.id;
        region = region.child(toast(
            format!("toast-{id}"),
            notice.text.clone(),
            move |s: &mut Workspace| s.dismiss_notice(id),
        ));
    }
    Some(region)
}
