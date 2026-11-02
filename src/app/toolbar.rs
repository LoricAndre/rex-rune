//! The 52pt bar on the window ground: sidebar toggle, identity chip, tab track, drag space,
//! Commands, and the window's own controls.
use super::{
    panel::Intent,
    theme::*,
    widgets::{app_logo, chrome_button, glyph},
    workspace::Workspace,
};
use rune::{Cursor, Cx, El, ScrollPos, Tx, col, el, row, text};
use serde_json::json;

/// A chip's press: consume it (chips never drag the window) and keep the terminal focused.
fn chip_press(e: El) -> El {
    e.cursor(Cursor::Pointer)
        .preserve_focus_on_press(true)
        .tab_index(-1)
}

fn identity(s: &Workspace) -> El {
    let model = s.model();
    let host = &s.settings.hosts[s.active].label;
    let session = model
        .view()
        .and_then(|v| v.label.clone())
        .unwrap_or_else(|| "Choose session".into());
    let state = if !model.connected {
        "Reconnecting"
    } else {
        s.host_state(s.active)
    };
    let dot = rgb(if model.connected { LIVE } else { WARNING });
    let tile = app_logo(s, 22.);
    chip_press(chip(
        row()
            .key("identity")
            .role("button")
            .label(format!("{session} · {host} · Switch session"))
            .w(210.)
            .h(34.)
            .pl(6.)
            .pr(8.)
            .gap(9.)
            .center_y()
            .hover(white(0.08))
            .help(format!("{session} · {host} · Switch session · Ctrl K"))
            .on_press_tx(|s: &mut Workspace, tx: &mut Tx| {
                s.picker(true);
                tx.blur();
            })
            .child(tile)
            .child(
                col()
                    .grow(1.)
                    .min_w(0.)
                    .gap(1.)
                    .child(text(session).font(strong(12.5)).color(rgb(INK)).truncate())
                    .child(
                        row()
                            .gap(5.)
                            .center_y()
                            .child(el().size(6., 6.).radius(3.).fill(dot).box_shadow(shadow(
                                0.,
                                0.,
                                8.,
                                dot.alpha(0.7),
                            )))
                            .child(
                                text(format!("{host} · {state}"))
                                    .font(ui(10.5))
                                    .color(rgb(MUTED))
                                    .truncate()
                                    .shrink(),
                            ),
                    ),
            )
            .child(glyph(&CHEVRON, 14., rgb(MUTED))),
    ))
}

fn tab_track(s: &Workspace) -> El {
    let model = s.model();
    let session = model.selected.clone().unwrap_or_default();
    let mut names = row()
        .key("tab-names")
        .h(28.)
        .gap(2.)
        .on_layout(|s: &mut Workspace, r| s.tab_strip.content = r[2]);
    if let Some(view) = model.view() {
        for tab in &view.windows {
            let Some(id) = tab.window_id.clone() else {
                continue;
            };
            let active = view.active_window_id.as_ref() == Some(&id);
            let label = tab.label.clone().unwrap_or_else(|| "Terminal".into());
            let (press_id, rename_id, close_id) = (id.clone(), id.clone(), id.clone());
            let mut tab_el = row()
                .key(format!("tab:{id}"))
                .role("tab")
                .label(label.clone())
                .h_full()
                .min_w(64.)
                .max_w(210.)
                .px(12.)
                .gap(7.)
                .center_y()
                .cursor(Cursor::Pointer)
                .preserve_focus_on_press(true)
                .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                    s.request("session.focus_window", json!({"window_id":press_id}));
                    tx.blur();
                })
                .on_double_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                    s.perform(Intent::RenameWindow(rename_id.clone()), tx);
                });
            if active {
                tab_el = tab_el
                    .child(
                        el().pin(Some(0.), Some(0.), Some(0.), Some(0.))
                            .radius(8.)
                            .continuous_corners()
                            .fill(white(0.11))
                            .box_shadow(shadow(0., 1., 3., black(0.35)))
                            .box_shadow(inset(1., white(0.09)))
                            .matched(format!("tab-pill:{session}"))
                            .transition(no_presence())
                            .motion(SNAPPY),
                    )
                    .child(el().size(5., 5.).radius(2.5).fill(rgb(ACCENT)));
            } else {
                tab_el = tab_el.radius(8.).hover(white(0.05));
            }
            tab_el = tab_el.child(
                text(label.clone())
                    .font(if active { medium(12.5) } else { ui(12.5) })
                    .color(rgb(if active { INK } else { MUTED }))
                    .truncate()
                    .help_when_truncated(label)
                    .shrink(),
            );
            if active {
                tab_el = tab_el.child(
                    row()
                        .key(format!("close-tab:{close_id}"))
                        .role("button")
                        .label("Close tab and its programs")
                        .size(18., 18.)
                        .center()
                        .radius(6.)
                        .hover(white(0.1))
                        .cursor(Cursor::Pointer)
                        .preserve_focus_on_press(true)
                        .help("Close tab and its programs")
                        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
                            s.perform(Intent::CloseWindow(close_id.clone()), tx)
                        })
                        .child(glyph(&CLOSE, 12., rgb(INK).alpha(0.8))),
                );
            }
            names = names.child(tab_el);
        }
    }
    let overflow = s.tab_strip.overflows();
    let (fade, pos) = (s.tab_fade.clone(), s.tab_pos.clone());
    let strip = row()
        .key("tabs")
        .min_w(0.)
        .shrink()
        .h(28.)
        .scroll_x()
        .scroll_edge_fade(16.)
        .on_layout(|s: &mut Workspace, r| s.tab_strip.viewport = r[2])
        .on_scroll_offset(move |p: ScrollPos| {
            pos.set([p.offset[0], p.max[0]]);
            fade[0].set(if p.offset[0] > 0.5 { 1. } else { 0.35 });
            fade[1].set(if p.offset[0] < p.max[0] - 0.5 {
                1.
            } else {
                0.35
            });
        })
        .child(names);
    let mut track = row()
        .key("tab-track")
        .h(34.)
        .px(3.)
        .gap(2.)
        .center_y()
        .min_w(0.)
        .shrink();
    if overflow {
        track = track.child(
            chrome_button(
                "previous-tabs",
                &LEFT,
                "Scroll previous tabs",
                26.,
                |s, tx| s.scroll_tabs(-1, tx),
            )
            .live_opacity(&s.tab_fade[0]),
        );
    }
    track = track.child(strip);
    if overflow {
        track = track.child(
            chrome_button("next-tabs", &RIGHT, "Scroll next tabs", 26., |s, tx| {
                s.scroll_tabs(1, tx)
            })
            .live_opacity(&s.tab_fade[1]),
        );
    }
    track = track
        .child(el().w(1.).h(16.).mx(3.).fill(white(0.08)))
        .child(chrome_button(
            "new-tab",
            &PLUS,
            "New tab · Ctrl Shift T",
            28.,
            |s, tx| s.perform(Intent::NewWindow, tx),
        ));
    chip_press(chip(track).on_press(|_: &mut Workspace| {}))
}

fn search_chip(wide: bool) -> El {
    let mut e = row()
        .key("commands")
        .role("button")
        .label("Commands")
        .h(34.)
        .px(10.)
        .gap(8.)
        .center_y()
        .hover(white(0.08))
        .help("Commands, sessions and tabs · Ctrl Shift P")
        .on_press_tx(|s: &mut Workspace, tx: &mut Tx| {
            s.picker(false);
            tx.blur();
        })
        .child(glyph(&SEARCH, 15., rgb(MUTED)));
    if wide {
        e = e
            .w(240.)
            .child(
                text("Commands, sessions, tabs")
                    .font(ui(12.))
                    .color(rgb(MUTED))
                    .grow(1.)
                    .truncate(),
            )
            .child(text("⌃⇧P").font(medium(10.5)).color(rgb(MUTED)));
    } else {
        e = e.w(36.).px(0.).center();
    }
    chip_press(chip(e))
}

/// A window control: the round hover shows inside, but the hit area takes the bar's whole
/// height (and the last one its right margin), so the corner above Close closes.
fn window_control(
    key: &str,
    i: &'static rune::Icon,
    help: &str,
    width: f32,
    danger: bool,
    f: impl Fn(&mut Workspace, &mut Tx) + 'static,
) -> El {
    let hover = if danger {
        rgb(DANGER).alpha(0.22)
    } else {
        white(0.09)
    };
    row()
        .key(key.to_owned())
        .role("button")
        .label(help.to_owned())
        .w(width)
        .h(TOOLBAR)
        .pr(width - 34.)
        .center_y()
        .justify_start()
        .cursor(Cursor::Pointer)
        .hover_group()
        .preserve_focus_on_press(true)
        .tab_index(-1)
        .help(help.to_owned())
        .on_press_tx(move |s: &mut Workspace, tx: &mut Tx| {
            f(s, tx);
            tx.blur();
        })
        .child(
            el().size(30., 30.)
                .ml(2.)
                .radius(15.)
                .center()
                .child(
                    el().pin(Some(0.), Some(0.), Some(0.), Some(0.))
                        .radius(15.)
                        .fill(hover)
                        .group_opacity(0., 1.),
                )
                .child(glyph(i, 15., rgb(MUTED))),
        )
}

pub fn toolbar(s: &Workspace, cx: &mut Cx<Workspace>) -> El {
    let width = cx.layout_size("root").map_or(1440., |[w, _]| w);
    let maximized = cx.window_maximized();
    row()
        .key("toolbar")
        .w_full()
        .h(TOOLBAR)
        .pl(MARGIN)
        .gap(8.)
        .center_y()
        .window_drag()
        .on_secondary_press(|_: &mut Workspace, p, tx| tx.show_window_menu(p.x, p.y))
        .child(chrome_button(
            "toggle-sidebar",
            &SIDEBAR,
            "Toggle sidebar · Ctrl Shift B",
            30.,
            |s, tx| s.perform(Intent::Sidebar, tx),
        ))
        .child(identity(s))
        .child(tab_track(s))
        // Always at least 48pt of window to drag, outside the tabs.
        .child(el().key("title-drag").grow(1.).min_w(48.).h_full())
        .child(search_chip(width >= 1280.))
        .child(
            row()
                .child(window_control(
                    "minimize",
                    &MINIMIZE,
                    "Minimize",
                    36.,
                    false,
                    |_, tx| tx.minimize(),
                ))
                .child(window_control(
                    "maximize",
                    if maximized { &RESTORE } else { &MAXIMIZE },
                    if maximized { "Restore" } else { "Maximize" },
                    36.,
                    false,
                    |_, tx| tx.toggle_maximize(),
                ))
                .child(window_control(
                    "close-app",
                    &CLOSE,
                    "Close app · server programs keep running",
                    36. + MARGIN,
                    true,
                    |_, tx| tx.close_window(),
                )),
        )
}

impl Workspace {
    pub fn scroll_tabs(&mut self, direction: i32, tx: &mut Tx) {
        let step = (self.tab_strip.viewport * 0.6).max(80.);
        let [offset, max] = self.tab_pos.get();
        tx.scroll_to_x("tabs", (offset + step * direction as f32).clamp(0., max));
    }
}
