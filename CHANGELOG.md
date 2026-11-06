# Changelog

## Unreleased

Changes since the 0.2.0 public release on 2026-11-01. Many of them come from two forks; thank you to [Joe Goldin](https://github.com/joegoldin) ([joegoldin/rex-rune](https://github.com/joegoldin/rex-rune)) and [Lily](https://github.com/kitten-lily) ([kitten-lily/rex-rune](https://github.com/kitten-lily/rex-rune)), credited on each item below and as authors or co-authors of the commits.

### Terminal

- Inline images with the Kitty graphics protocol, restored when you reattach. Adds a patched copy of `ghostty-vt` under `vendor/`. (@joegoldin)
- Copies made by terminal programs (OSC 52) reach your clipboard, accepted only from the focused pane while the window is focused. Remote programs still can't read your clipboard. (@joegoldin)
- Double-click selects a word or shell path and triple-click a line; a selection stays attached to its text while output scrolls, and dragging past the edge autoscrolls. (@joegoldin)
- Ctrl+click opens web and mail links, including OSC 8 hyperlinks; the tooltip shows the real target. `file://` links are never opened. (@kitten-lily)
- Full-screen programs no longer flicker while they redraw (synchronized output). (@kitten-lily)
- Tabs with a server-generated name show their program's title, as tabs called "Shell" already did. (@joegoldin)
- Terminal themes: Settings → Terminal → Theme sends a theme to the server with Rex's `set_theme`, so programs that ask for colours get the ones on screen. Other clients viewing a terminal see the theme while this client owns it, as they see its size. Choose Rex's default colours, Midnight, any Ghostty theme you have installed, or follow the chrome palette. The default, Follow server, changes nothing. (@kitten-lily)
- The terminal font family and fallback families are saved in Settings → Terminal; only the families you name are loaded. (@joegoldin, @kitten-lily)

### Keyboard

- Dead-key accents are typed into the terminal, including an accent followed by Space. (@kitten-lily)
- Programs can tell Shift+Enter, Shift+Tab and Shift+Backspace from the unshifted keys. (@kitten-lily)
- Key releases are sent correctly when nothing in the window has focus. (@kitten-lily, @joegoldin)
- Alt+arrows reach terminal programs (word movement in shells and editors); switching panes moved to Ctrl+Alt+arrows. (@joegoldin)
- Ctrl+Shift+V pastes only into the pane that asked, even if focus moves before the clipboard read returns.

### Window and navigation

- Chrome palettes, switchable live in Settings → Interface → Theme: Midnight (the default, unchanged), Rosé Pine, Rosé Pine Moon and Gruvbox Dark. (@joegoldin, @kitten-lily)
- Program activity on tabs (OSC 7501: "Needs approval", "Error", "Done" and so on), program icons, and desktop notifications for tabs you aren't looking at.
- Settings in sections (Interface, Terminal, Notifications, Integrations, Remote Hosts, Shortcuts), including adding, editing and removing hosts.
- A single pane draws edge to edge; tabs and toolbar restyled.
- Tabs in the sidebar: Settings → Interface → Tab position → Sidebar lists each session's tabs under it while the sidebar is open. (@joegoldin)
- The sidebar opens at launch as you left it. (@kitten-lily)
- A Settings button in the toolbar; the toolbar's buttons share one plain hover style, and the minimize and maximize buttons are gone.
- Edit a host (name, endpoint, default shell) from its sidebar menu, from Commands or from an offline session's menu.
- Rename and close sessions in place in the switcher and from the session chip.
- The pane menu opens on right-click even over programs that track the mouse.

### Integrations and packaging

- Settings → Integrations installs, updates or removes Rex's skill and session hooks for Claude Code, Codex, OpenCode and Pi with the local `rex skill`, always previewed first.
- OMP (oh-my-pi) in Settings → Integrations. It shares Pi's skill directory, so it uses Rex's Pi setup until Rex knows OMP. (@kitten-lily)
- `rex` is also found in `~/.local/bin` (or `$XDG_BIN_HOME`), not only on `PATH`. (@kitten-lily)
- A Nix flake with pinned packages and desktop integration; see [docs/nix.md](docs/nix.md). (@joegoldin)
- Optional mise tasks, and `scripts/install-desktop.sh --copy` / `--uninstall`. (@kitten-lily)
