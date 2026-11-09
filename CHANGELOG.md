# Changelog

## Unreleased

Several of these come from [Lily](https://github.com/kitten-lily)'s fork ([kitten-lily/rex-rune](https://github.com/kitten-lily/rex-rune)); thank you.

- Terminal fidelity: the cursor takes the shape programs ask for (DECSCUSR block, bar or underline); curly, dotted, dashed and double underlines in their own colours and overlines (SGR 4:x, 58, 53); box-drawing and block characters are drawn on whole pixels, so borders meet and meters have no seams. (@kitten-lily)
- A server that dropped this client while the network was gone (a laptop asleep) is noticed within about 30 seconds through TCP keepalives, and the host and its terminals reconnect. (@kitten-lily)
- Progress that programs report with OSC 9;4 draws as a thin bar under their tab's label and their pane's name, "Paused" and "Error" as words; the bar goes when the program clears it or leaves the foreground. A terminal bell that rings out of sight marks its tab "Bell" until you see it. In a split, each pane's header carries its own activity word, and an unrenamed pane shows its program's terminal title. (@kitten-lily)
- Ctrl Shift F searches the focused terminal's screen and scrollback with Ghostty's search: matches are highlighted, Enter and Shift+Enter step through them, Escape gives the keyboard back to the terminal, and nothing typed in the field reaches the program. (@kitten-lily)
- A pane this client owns waits up to half a second for a frame at its own size before drawing, so the previous client's size no longer flashes on attach. (@kitten-lily)
- Add host lists the machines on your tailnet (`tailscale status --json`), searchable by name, by Tailscale IP when the tailnet has no MagicDNS. Each online machine is checked for a Rex server at the addresses the official clients try for a bare host (HTTPS 443, HTTPS 7297, HTTP 7297); one that answers is added and connected with one press. Any other machine takes an SSH user name and an optional Rex socket path and connects as `ssh://user@host[/socket]`; the form says whether the check found nothing listening or couldn't tell. A machine already added stays listed, so another login to it can be added. Entering an endpoint by hand remains.
- `ssh://[user@]host[:port][/socket]` endpoints: the Rex server's Unix socket on that machine is forwarded over `ssh -L`, so a machine with Tailscale SSH, or any SSH login you already have, works with nothing configured on its Rex. Without a `/socket`, the machine is asked where its server listens (`rex server status`, the user's runtime directory, `/run/rex`, `/var/run/rex`). Each instance keeps its forwards in a directory of its own, so two clients or a probe reaching the same machine don't disturb each other. The forward's end is a disconnect like any other, and `ssh`'s reason is shown next to Retry.
- Settings > Terminal now says when a saved font family waits for the client to be reopened, and when the saved family isn't installed. Saving a font change shows a notice to reopen the client. Applying fonts without a restart is tracked in [#6](https://github.com/kitten-lily/rex-rune/issues/6).
- Shift+Space, Shift+Tab and Shift+Enter now reach terminal programs that use the Kitty keyboard protocol, such as Claude Code, as distinct keys instead of plain Space, Tab and Enter. Shift is reported unless the keyboard layout used it to type the character. ([#4](https://github.com/kitten-lily/rex-rune/issues/4))

## 0.3.2 — 2026-11-06

- First prebuilt Linux releases for x86-64 and ARM64: user-installable tarballs and Debian/Ubuntu packages, with SHA-256 checksums.
- Automated native builds and tests on Ubuntu 24.04. No application behavior changes since 0.3.1.

## 0.3.1 — 2026-11-06

- Only Rex API revisions 10 and 11 are supported; servers older than revision 10 are refused with a message to update Rex. The revision-7 and revision-8 compatibility paths and `scripts/revision8-check.py` are gone.

## 0.3.0 — 2026-11-06

Changes since the 0.2.0 public release on 2026-11-01. Many of them come from two forks; thank you to [Joe Goldin](https://github.com/joegoldin) ([joegoldin/rex-rune](https://github.com/joegoldin/rex-rune)) and [Lily](https://github.com/kitten-lily) ([kitten-lily/rex-rune](https://github.com/kitten-lily/rex-rune)), credited on each item below and as authors or co-authors of the commits.

### Terminal

- Drop files on a pane, or paste copied files or a screenshot with Ctrl+Shift+V, and Rex uploads them to the machine that terminal runs on and types their paths, local or remote, with no SSH setup. Needs a Rex server with API revision 11; older servers say so. ([#2](https://github.com/shuv1337/rex-rune/issues/2), requested by @joegoldin)
- The client speaks Rex API revisions 7 to 11, and doesn't draw floating layers another client minimized.
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
