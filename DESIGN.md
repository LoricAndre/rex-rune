# Design system: Midnight

Rex Linux draws calm, dark chrome around terminals that the server owns. The tokens below mirror `src/app/theme.rs`, which is the source of truth; update both together.

## Principles

- **The terminal is the content.** Chrome frames it and stays out of the way. Terminal text, colours, cursor and split geometry come from the server and are never recoloured. The one exception: the server's default terminal black is replaced by the pane-card colour so panes sit in the surface; explicit colours are preserved.
- **Chrome moves, content doesn't.** Popups spring open from their anchor and highlights and the sidebar slide; terminal content, pane geometry and cell rendering never animate.
- **One working row.** Session identity, tabs, commands and the window's own controls share a single toolbar. Navigation (hosts → sessions) opens on demand, with an optional saved sidebar.
- **Quiet when healthy.** A connected host, a live pane and the active tab carry no status dots or "Connected" labels; names and host icons are enough. Words appear only for trouble ("Offline", "Reconnecting", "input paused"), in `WARNING`, next to the action that fixes it.
- **Distinct states.** The current session, the keyboard-selected row, a captured pane target and live terminal focus each have a different cue. Keyboard selection is never fill alone.
- **Name the target.** Creation actions name their host; destructive confirmations name the pane, tab and host and default to **Keep running**; closing the app leaves server programs running.

## Colour

| Token | Value | Use |
| --- | --- | --- |
| `GROUND` | `#060b14` | Window ground and page background |
| `CARD` | `#05080e` | Pane cards; the terminal's default background |
| `POPOVER` | `#0d1524` | Popovers, dialogs, raised surfaces (drawn at 95% over a backdrop blur) |
| `INK` | `#e8eef6` | Primary text and icons |
| `MUTED` | `#93a3b8` | Secondary text, labels, shortcuts and icons |
| `ACCENT` | `#7cc4ff` | Focus, selection outline, current-session emphasis, caret |
| `ACCENT2` | `#a78bfa` | Secondary accent |
| `LIVE` | `#5eead4` | Kit success colour |
| `WARNING` | `#f5d48a` | Reconnecting, notices, ownership strips |
| `DANGER` | `#ff8a8a` | Validation errors and destructive text |
| `TERMINAL_SELECTION` | `#2f4a6e` | Local terminal text-selection overlay |

Interaction fills are translucent white over the surface: hover 7%, pressed 11%; borders 6–20% white (focus uses `ACCENT`). Trouble states always carry a word, never colour alone; healthy states show nothing. No light theme is implemented.

## Typography

Chrome uses Inter (bundled with the kit) in 10–13 pt steps, weights about 450 (body), 560 (medium), 620 (strong); group labels are 10.5 pt small capitals with 0.6 tracking. The terminal font is configurable (`--font`, `--font-size`, saved size 8–48 px; default JetBrainsMono Nerd Font 15) and falls back to bundled IBM Plex Mono. Cell width follows the font's advance; ligatures and kerning are off so the grid stays exact. Names keep their case and ellipsize when space is bounded, with tooltips for the full text. Server windows are called **tabs**.

## Layout and shape

- **Toolbar:** 52 pt chip bar. Controls sit on glassy chips (10 pt radius, 5.5% white fill, gradient hairline border). The empty space moves the window; edges and corners resize it; minimize/maximize/close are at the right end.
- **Pane cards:** server-defined placements as rounded cards (12 pt radius) with a 30 pt header (pane name and, when needed, its exception strip), 8 pt gutters and 10 pt outer margin on the ground. Terminal text has 8 px padding inside its card. Divider drags preview immediately and send a ratio clamped to 0.1–0.9.
- **Sidebar:** optional, hidden on a new install, default 236 pt wide (200–360 pt, drag the edge to resize, double-click to reset). Host rows are 28 pt, session rows 32 pt, groups separated by 12 pt.
- **Popovers:** session/host menus anchor to the identity; pane menus to the pointer (right-click) or the toolbar; the command menu is centered near the top. Popovers clamp to the viewport; long lists scroll with directional hints that count hidden rows.
- **Dialogs** (forms, preferences, confirmation) are centered over a dark scrim and sized to their content within the viewport.
- **Radii:** kit small 8, medium 10; chips 10; cards and popovers 12.
- **Icons:** authored strokes on a 20×20 view box, 1.5 stroke, round caps and joins, `currentColor`. Hosts show their kind (a monitor for this machine's Unix socket, a server for a remote endpoint) in the identity chip, sidebar headers and switcher groups; tabs carry a `>_` glyph; the Commands button is ⌘. Magnifying glasses mark search fields only.
- **Elevation:** the working canvas is flat. Popovers and dialogs get a frosted fill, a lit gradient rim and a deep soft shadow.
- **Interface zoom:** 70–150% in 10% steps. Zoom above 100% is limited to preserve a 640×480 layout floor where the viewport allows; terminal font size is independent.

## Interaction

- Workspace shortcuts are listed in the README; they are reserved by the client and never reach the terminal.
- Opening a form or search focuses its first field; close confirmation focuses **Keep running**. Tab and Shift-Tab cycle fields and actions in panels; Enter and Space activate; Escape backs out and restores the previously focused pane. Tab, Space, Return, Escape and Menu never move focus away from the terminal unless a panel is open.
- Right-click captures the specific pane under the pointer; its menu names tab and pane and keeps a target outline through rename and close. A target that disappears dismisses the menu with a notice instead of retargeting.
- Size ownership is automatic: the active window owns terminal size and typing claims it. A non-owner or reconnecting terminal shows an exception strip with **Take control** or **Retry**, and never sends a resize.
- A transport failure marks affected panes input-paused, keeps their output, and never queues or replays input. Notices are toasts: errors are sticky, information leaves after about 6 seconds, and at most three notices are retained.
- Focus cues: a filled block cursor in the focused terminal, a hollow outline cursor in others, and an accent outline on the focused pane when no panel is open.

## Do and don't

- Do keep the toolbar single-row and the terminal canvas dominant.
- Do keep chrome tokens in `theme.rs` rather than hard-coding colours in views.
- Do derive overflow cues and hidden-row counts from measured content.
- Don't recolour terminal output to match the chrome, or animate terminal content.
- Don't let selecting, copying or navigating claim size ownership.
- Don't add status dots, "Connected" labels, terminal size readouts or keyboard-hint footers.
- Don't show illustrative sessions, layouts or captures as shipped runtime state.
