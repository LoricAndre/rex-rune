# Design system

Rex Linux draws calm, dark chrome around terminals that the server owns. Chrome palettes are token sets defined in `src/app/theme.rs`, which is the source of truth, and chosen at runtime (Settings → Interface → Theme): Midnight, the default, Rosé Pine, Rosé Pine Moon and Gruvbox Dark. Their tokens are listed below (update both together). Each palette pairs with a built-in terminal theme whose background equals the chrome's ground, so a single pane reads as one surface with the toolbar. Palettes are dark only and change colours only; fills, borders, motion and layout are shared.

## Principles

- **The terminal is the content.** Chrome frames it and stays out of the way. Terminal text, colours, cursor and split geometry come from the server and are never recoloured; a terminal theme the client declares reaches the terminal through the server (Rex's `set_theme`). The one exception: the server's default terminal background is drawn in the palette's `CARD` colour (or the background of the theme this client declares), the same as the ground, so a single pane and the toolbar read as one surface. It is substituted only when both are dark or both are light, so a light default under another client's light theme, or under a server that ignored `set_theme`, keeps its own colour and the default text stays legible. Explicit background colours and inverse video's block are preserved; the substitution is by colour value, so an explicit background equal to the default is filled too.
- **Chrome moves, content doesn't.** Popups spring open from their anchor and highlights and the sidebar slide; terminal content, pane geometry and cell rendering never animate.
- **One working row.** Session identity, tabs, commands and the window's own controls share a single toolbar. Navigation (hosts → sessions) opens on demand, with an optional sidebar. The Sidebar tab position is an option that moves tabs out of the toolbar into the sidebar while the sidebar is open; closed, the toolbar shows them again.
- **Quiet when healthy.** A connected host, a live pane and the active tab carry no status dots or "Connected" labels; names and host icons are enough. Words appear only for trouble ("Offline", "Reconnecting", "input paused"), in `WARNING`, next to the action that fixes it. The one exception is program activity: a tab whose programs finished while the user was away says "Done" in `ACCENT` until it is seen.
- **Distinct states.** The current session, the keyboard-selected row, a captured pane target and live terminal focus each have a different cue. Keyboard selection is never fill alone.
- **Name the target.** Creation actions name their host; destructive confirmations name the pane, tab and host and default to **Keep running**; closing the app leaves server programs running.

## Colour

| Token | Use |
| --- | --- |
| `GROUND` | Window ground and page background |
| `CARD` | Pane cards; the terminal's default background when it is dark and no theme with its own background is declared (the same colour as the ground, and the paired terminal theme's background) |
| `POPOVER` | Popovers, dialogs, raised surfaces (drawn at 95% over a backdrop blur) |
| `INK` | Primary text and icons |
| `MUTED` | Secondary text, labels, shortcuts and icons |
| `ACCENT` | Focus, selection outline, current-session emphasis, caret |
| `ACCENT2` | Secondary accent |
| `LIVE` | Kit success colour |
| `WARNING` | Reconnecting, notices, ownership strips |
| `DANGER` | Validation errors and destructive text |
| `ON_ACCENT` | Text on an accent fill (primary buttons) |
| `TERMINAL_SELECTION` | Local terminal text-selection overlay, unless the declared theme sets `selection-background` |

| Token | Midnight | Rosé Pine | Rosé Pine Moon | Gruvbox Dark |
| --- | --- | --- | --- | --- |
| `GROUND`, `CARD` | `#001528` | `#191724` base | `#232136` base | `#282828` bg |
| `POPOVER` | `#0d1524` | `#1f1d2e` surface | `#2a273f` surface | `#3c3836` bg1 |
| `INK` | `#e8eef6` | `#e0def4` text | `#e0def4` text | `#ebdbb2` fg |
| `MUTED` | `#93a3b8` | `#908caa` subtle | `#908caa` subtle | `#a89984` gray |
| `ACCENT` | `#7cc4ff` | `#c4a7e7` iris | `#c4a7e7` iris | `#83a598` blue |
| `ACCENT2` | `#a78bfa` | `#9ccfd8` foam | `#9ccfd8` foam | `#d3869b` purple |
| `LIVE` | `#5eead4` | `#31748f` pine | `#3e8fb0` pine | `#b8bb26` green |
| `WARNING` | `#f5d48a` | `#f6c177` gold | `#f6c177` gold | `#fabd2f` yellow |
| `DANGER` | `#ff8a8a` | `#eb6f92` love | `#eb6f92` love | `#fb4934` red |
| `ON_ACCENT` | `#061220` | `#191724` base | `#232136` base | `#282828` bg |
| `TERMINAL_SELECTION` | `#2f4a6e` | `#403d52` highlight med | `#44415a` highlight med | `#504945` bg2 |

The paired terminal themes are Midnight (its navy behind Rex's default text), Rosé Pine and Rosé Pine Moon as their Ghostty themes draw them, and Gruvbox Dark's background, text, cursor and 16-colour palette. In every palette `MUTED` on `CARD` and `INK` on `POPOVER` meet WCAG AA (4.5:1); `WARNING` and `DANGER` are at least 3:1 on both surfaces (Gruvbox Dark's red is the lowest, 4.3:1 on its ground and 3.4:1 on popovers) and always carry a word. Switching palettes is instant: colours change in one frame, with no crossfade.

Interaction fills are translucent white over the surface: hover 7%, pressed 11%; borders 6–20% white (focus uses `ACCENT`). Trouble states always carry a word, never colour alone; healthy states show nothing. There is no light chrome palette.

## Typography

Chrome uses Inter (bundled with the kit) in 10–13 pt steps, weights about 450 (body), 560 (medium), 620 (strong); group labels are 10.5 pt small capitals with 0.6 tracking. The terminal family, ordered fallback families and size (8–48 px) are saved locally, and `--font`, repeated `--font-fallback` and `--font-size` override them for one launch; the default is JetBrainsMono Nerd Font 15. Missing glyphs come from the named fallbacks in order, then bundled IBM Plex Mono, then the system's script and symbol fallbacks; an unavailable primary is replaced by IBM Plex Mono. Family changes apply after reopening the client. Cell width follows the primary font's advance; ligatures and kerning are off so the grid stays exact. Names keep their case and ellipsize when space is bounded, with tooltips for the full text. Server windows are called **tabs**.

## Layout and shape

- **Toolbar:** 52 pt chip bar. Controls sit on glassy chips (10 pt radius, 5.5% white fill, gradient hairline border). Tab pills are 96–240 pt wide with no close button: a tab closes from its right-click menu, a middle-click or `Ctrl Shift W`. Commands (⌘), New tab (+) and Settings (sliders) sit at the right end with Close, all as bare 15 pt icons on a 36 pt pitch with no fill at rest; hovering shows the same 30 pt round fill on each (9% white, `DANGER` at 22% for Close). A 1×16 pt hairline divides the app's buttons from Close. There are no minimize or maximize buttons: tiling window managers often ignore them. A pill shows its focused pane's program icon, its label, and then a word for program activity: "Needs approval", "Question", "Needs sign-in" or "Needs input" in `WARNING`, "Error" in `DANGER`, unseen "Done" in `ACCENT`; idle and working tabs show no word. A tab still called "Shell", the name new tabs get, shows its focused pane's terminal title instead while the program that set it is in the foreground; a tab the user named keeps its name. The label truncates before the word does; the program's message is the word's tooltip. The empty space moves the window; edges and corners resize it.
- **Single pane:** a tab with one pane, not zoomed, draws it edge to edge under the toolbar with no card, header, border or margin. Its exception strip floats at the pane's top-right corner.
- **Pane cards:** splits, and a zoomed pane, are server-defined placements as rounded cards (12 pt radius) with a 30 pt header (pane name and, when needed, its exception strip), 8 pt gutters and 10 pt outer margin on the ground; their border separates them from the same-coloured ground. Terminal text has 8 px padding inside its card; a terminal background other than `CARD` fills the card's body inside its border, below the header, with the card's bottom corners. Divider drags preview immediately and send a ratio clamped to 0.1–0.9.
- **Sidebar:** optional; it opens at launch as it was left, at its saved width. Default 236 pt wide (200–360 pt, drag the edge to resize, double-click to reset). Host rows are 28 pt, session rows 32 pt, groups separated by 12 pt. With the Sidebar tab position, a session's row becomes a 24 pt muted label and its tabs follow as 32 pt rows indented 12 pt, each with the toolbar pill's icon, label and activity word; the active tab gets the pill's fill and hairline.
- **Popovers:** session/host menus anchor to the identity; pane and tab menus to the pointer (right-click), and pane menus also to the toolbar; the command menu is centered near the top. Popovers are compact: the switcher 340 pt wide, Commands 460, pane menus 260; 28 pt rows (30 in Commands), 26 pt group headers, a 34 pt search field, 5 pt padding and a 12 pt radius. Popovers clamp to the viewport; long lists scroll with directional hints that count hidden rows. A popover never opens another popover from its rows: the switcher's highlighted session row trades its detail for pencil and × buttons, and renaming turns the row itself into a field (accent outline) until Enter or Esc.
- **Dialogs** (forms, confirmation) are centered over a dark scrim and sized to their content within the viewport. **Settings** is a larger dialog (760×520, clamped to the viewport) with a 190 pt section list on the left and the section's rows on the right; each row names the setting, explains it in one muted line and puts its control on the right. Forms, confirmations and the chrome and terminal theme lists opened from Settings return to it.
- **Radii:** kit small 8, medium 10; chips 10; cards and popovers 12.
- **Icons:** authored strokes on a 20×20 view box, 1.5 stroke, round caps and joins, `currentColor`. Hosts show their kind (a monitor for this machine's Unix socket, a server for a remote endpoint) in the identity chip, sidebar headers and switcher groups; tabs carry their program's icon at 15 pt (Claude Code, Codex, OpenCode, Pi, Vim, Git, Cargo, SSH), or `>_` for shells and anything else. Pi and OpenCode use solid marks after their own logos, since outlines of them are unrecognisable at that size; the Commands button is ⌘. Magnifying glasses mark search fields only.
- **Elevation:** the working canvas is flat. Popovers and dialogs get a frosted fill, a lit gradient rim and a deep soft shadow.
- **Interface zoom:** 70–150% in 10% steps. Zoom above 100% is limited to preserve a 640×480 layout floor where the viewport allows; terminal font size is independent.

## Interaction

- Workspace shortcuts are listed in the README; they are reserved by the client and never reach the terminal.
- Opening a form or search focuses its first field; close confirmation focuses **Keep running**. Tab and Shift-Tab cycle fields and actions in panels; Enter and Space activate; Escape backs out and restores the previously focused pane. Tab, Space, Return, Escape and Menu never move focus away from the terminal unless a panel is open.
- Right-click captures the specific pane under the pointer; its menu names tab and pane and keeps a target outline through rename and close. A target that disappears dismisses the menu with a notice instead of retargeting.
- Size ownership is automatic: the active window owns terminal size and typing claims it. A non-owner or reconnecting terminal shows an exception strip with **Take control** or **Retry**, and never sends a resize.
- A transport failure marks affected panes input-paused, keeps their output, and never queues or replays input. Notices are toasts: errors are sticky, information leaves after about 6 seconds, and at most three notices are retained.
- Focus cues: the focused terminal draws the cursor shape its program asked for (a filled block unless it chose a bar or underline), other panes a hollow outline block, and the focused pane has an accent outline when no panel is open. Cursor blink requests draw steadily, like the rest of terminal content.
- Links open only on Ctrl+click (press and release on the same link), never on a plain click, so selecting text and clicking into programs stay unchanged. Holding Ctrl over a link underlines it in its own text colour, shows the hand pointer and gives it a tooltip with its target URL (never its displayed text).

## Do and don't

- Do keep the toolbar single-row and the terminal canvas dominant.
- Do keep chrome tokens in `theme.rs` rather than hard-coding colours in views.
- Do derive overflow cues and hidden-row counts from measured content.
- Don't recolour terminal output to match the chrome, or animate terminal content.
- Don't let selecting, copying or navigating claim size ownership.
- Don't add status dots, "Connected" labels, terminal size readouts or keyboard-hint footers.
- Don't show illustrative sessions, layouts or captures as shipped runtime state.
