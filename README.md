# Rex Linux

An unofficial, independent native Linux client for [Rex](https://www.superlogical.com/rex), the terminal multiplexer from Superlogical. It is not affiliated with, endorsed by or supported by Superlogical or the Rex authors.

![Rex Linux with the sessions sidebar open and a three-pane split showing a file tree, source code and test results](docs/screenshot.png)

Rex Linux attaches to an existing Rex server. Saved hosts contain sessions, each session's Rex windows become tabs, and the server defines the terminal splits. Sessions and their programs stay on the server when the app closes, so any other Rex client can attach to the same sessions.

It is written in Rust on [Rune](https://github.com/anomalyco/rune) (wgpu/winit, `rune-kit` components) with libghostty-vt for terminal emulation, and talks to the server through the [`rexrs`](https://github.com/a10y/rexrs) control SDK.

> **Status:** early. The protocol layer, workspace, terminal rendering and input are implemented and tested against real servers, but some areas (see [Current scope](#current-scope)) still need hands-on validation, and there are no packaged releases yet.

## Features

- Multiple saved hosts, each with its sessions; session windows are tabs and splits follow the server's layout.
- Unix-socket and HTTPS/WSS endpoints (for example a Tailscale-exposed Rex server).
- A single toolbar with session identity, tabs, commands and the window's own controls, over rounded pane cards. Host-grouped session navigation opens on demand, with an optional saved sidebar.
- Searchable session switcher (`Ctrl K`) and command palette (`Ctrl Shift P`).
- Snapshot restore, live output, input, bracketed paste, text selection and copy, local scrollback, and mouse/wheel reporting for terminal programs.
- Reconnect handling: a transport failure pauses input on affected panes (input is never replayed) while parsed output survives reconnects and session changes.
- Interface zoom (70–150%) and a configurable terminal font and size.
- Borderless window with its own minimize/maximize/close, edge resizing and drag region.
- Keys are sent by physical position (Ghostty key names), so non-US layouts work.

**Attachment only.** Rex Linux never starts, stops, replaces or reconfigures a Rex server and does not need a `rex` executable on `PATH`. An unavailable server is shown as disconnected and retried. Closing the client leaves server programs running.

## Requirements

- Rust 1.96 or newer (edition 2024; see `rust-version` in `Cargo.toml`).
- [Zig](https://ziglang.org/) 0.17.x. The `ghostty-vt` crate builds Ghostty's VT library from a checksum-pinned source archive with Zig.
- Linux with Wayland or X11, plus the usual graphics/input libraries: Vulkan, xkbcommon, fontconfig and the Wayland/X11 client libraries.
- A running Rex server that supports API revision 7 or 8. The client negotiates revision 8 when available and falls back to 7, including on newer servers that offer compatibility.

## Build

```sh
cargo build --locked --release --bin rex-linux
```

`Cargo.lock` pins the dependency graph. TLS uses Rustls with the system trust roots. The workspace also builds two protocol probes (`rex-probe`, `rex-workspace-probe`) that do not need the UI; use `--no-default-features` to build only those.

## Run

```sh
# Create a persistent session on the local Rex server.
target/release/rex-linux --new work

# Reattach to an existing session by exact label or ID.
target/release/rex-linux --session work

# Attach to a remote server (for example one exposed over Tailscale).
target/release/rex-linux --endpoint https://my-host.example.ts.net --session work

# Choose a terminal font and size.
target/release/rex-linux --session work --font 'JetBrainsMono Nerd Font' --font-size 15
```

Options:

| Option | Meaning |
| --- | --- |
| `--endpoint <URL>` | `unix:///absolute/path.sock`, `https://…`, `http://…`, `wss://…` or `ws://…`. Defaults to `$REX_SERVER`, then `unix://$XDG_RUNTIME_DIR/rex/server.sock` (or `unix:///run/user/1000/rex/server.sock` if `XDG_RUNTIME_DIR` is unset). |
| `--session <LABEL\|ID>` | Select a session by exact label or ID. Missing or ambiguous names never pick an arbitrary session. |
| `--new <LABEL>` | Explicitly create a persistent session (conflicts with `--session`). Ordinary attachment never creates or destroys sessions. |
| `--block <ID>` | Initially focus this terminal block. |
| `--font <FAMILY>`, `--font-size <PX>` | Terminal font (default `JetBrainsMono Nerd Font`, 15) resolved through fontconfig. If the family is missing, the bundled IBM Plex Mono is used and a line is printed on stderr. |
| `--config <PATH>` | Use a separate workspace preferences file. |
| `--window-size <WxH>` | Requested startup size (default `1440x900`); tiling compositors may override it. |

Without `--session`, the workspace restores the saved host and session; on a new host it selects the first listed session. Rune's own developer flags go after `--` (for example `-- --snap out.png`). `REX_FRAME_STATS=1` prints per-frame CPU percentiles.

### Desktop install

```sh
bash scripts/install-desktop.sh [path/to/rex-linux]
```

This installs `dev.shuv.rex-linux.desktop` under `$XDG_DATA_HOME/applications` (normally `~/.local/share/applications`) pointing at the release binary of this checkout (or the binary you pass), and the app icon at 32, 128 and 256 px into the user's `hicolor` theme. Re-run it if you move the checkout. It needs `desktop-file-install`, `desktop-file-validate` and `update-desktop-database`. See [icon provenance](resources/icons/README.md).

## Configuration

Preferences are stored as JSON in `$XDG_CONFIG_HOME/rex-linux/workspace.json` (default `~/.config/rex-linux/workspace.json`): saved hosts (label, endpoint, collapsed state, optional default shell), the last host and session per host, sidebar visibility and width, terminal font size and interface zoom (`ui_scale`). Use `--config <path>` to keep a separate workspace.

A host's default shell (empty means the server user's login shell) is used for every new session, tab and split this client creates on that host. Host endpoints must be `unix://` with an absolute path, or `https://`, `http://`, `wss://` or `ws://` URLs.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl K | Search sessions across saved hosts |
| Ctrl Shift P | Search commands, sessions and current-session tabs |
| Ctrl Shift N / T | New session on the active host / new tab |
| Ctrl Shift D / Ctrl Alt D | Split right / down |
| Ctrl PageUp / PageDown | Previous / next tab |
| Alt arrows | Focus an adjacent pane |
| Ctrl Shift Enter | Zoom / restore the focused pane |
| Ctrl Shift Space | Focused-pane actions |
| Ctrl Shift B | Collapse / restore the sidebar |
| Ctrl Shift C / V | Copy selection / paste (bracketed-paste aware) |
| Ctrl Shift R | Resume the focused terminal's data stream |
| Shift PageUp / PageDown, wheel | Scroll local history |
| Ctrl + / Ctrl − / Ctrl 0 | Enlarge / shrink / reset the whole interface |

These chords are reserved by the client and do not reach terminal programs; every other key goes to the focused pane. Interface zoom changes in 10% steps (70–150%), is saved as `ui_scale`, and is independent of the terminal font size. `Ctrl =` and the numpad `+`/`−` also work.

Other interactions: click the session/host identity for grouped navigation; right-click a pane for actions targeting that pane; drag dividers to resize splits and the sidebar's edge to resize it (double-click resets); double-click a tab to rename it. Closing a pane or tab asks for confirmation, with **Keep running** as the default. The active window takes size ownership of a terminal automatically. On the borderless window, the toolbar's empty space moves it (double-click maximizes; right-click requests the window manager's menu on Wayland).

## Development

```sh
cargo test --locked --all-features
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings

# Protocol probes against a Rex server (endpoint optional; they create and remove their own sessions).
cargo run --locked --no-default-features --bin rex-probe
cargo run --locked --no-default-features --bin rex-workspace-probe
```

The `scripts/` directory holds integration checks; each script's docstring lists its requirements (Xvfb, xdotool, a `rex` binary, `wayland-info`, …). Some use the configured Rex server rather than starting their own: point them at a disposable server you control. Wayland checks use the running compositor; X11 checks use an isolated display.

- `revision8-check.py`: API revision negotiation and bootstrap tickets against a disposable server, optionally running the probes.
- `connection-only-smoke.py`: a missing or stale socket must never execute the Rex CLI.
- `x11-smoke.py`: real X11 events (forms, clipboard, pointer selection, key isolation).
- `rune-acceptance-x11.py`: window-corner, terminal-key and frame-CPU acceptance checks on X11.
- `chrome-spike-x11.py` (with the `chrome_spike` example): borderless window chrome on X11.
- `wayland-seat-smoke.py`: Wayland seat/resource selection on a multi-seat compositor.
- `mouse-check.py`: terminal mouse reporting with real pointer input on a private headless Wayland display.
- `readme-screenshot.py`: regenerates `docs/screenshot.png` from a private Rex server and headless compositor, with a throwaway home and prompt so no local details appear.
- `demo-record.py`: records a demo video of the app on a private headless compositor (uses the `tools/vptr` virtual-pointer helper).

See [AGENTS.md](AGENTS.md) for the code layout and contributor conventions, and [DESIGN.md](DESIGN.md) / [PRODUCT.md](PRODUCT.md) for the design system and product intent.

## Vendored dependencies

Two dependencies are vendored with small, documented patches and wired in through `[patch]` in `Cargo.toml`:

- [`vendor/rune/`](vendor/rune/README.md): the `rune`, `rune-motion`, `rune-staging` and `procwatch` crates from `anomalyco/rune` at `100b7d9` (MIT), with window-chrome additions for a borderless Linux window (maximize/restore, minimize, edge resize, window menu, native window icon).
- [`vendor/rexrs/`](vendor/rexrs/REX-LINUX.md): `a10y/rexrs` at `780be3f` (MIT OR Apache-2.0), plus revision-8 generated types, explicit API-revision range negotiation and preservation of an empty `attach_session_ids`.

Each patch is meant to be dropped once the corresponding upstream release has equivalent APIs.

## Current scope

Still needing hands-on validation: physical Wayland pointer gestures (move, edge resize, double-click, window menu), minimize on Wayland, multi-seat Wayland desktops, actual IME input, and sustained daily use. On X11, winit has no window-menu request, so right-clicking the drag region does nothing there.

Not implemented: terminal-content search, complete Kitty graphics, host editing/removal, accessibility (AT-SPI) and distribution packages.

## Third-party software and assets

| Component | License |
| --- | --- |
| [anomalyco/rune](https://github.com/anomalyco/rune) (UI toolkit; vendored crates plus `rune-kit` and `rune-theme-opencode` from git) | MIT |
| [a10y/rexrs](https://github.com/a10y/rexrs) (Rex control SDK; vendored) | MIT OR Apache-2.0 |
| [Ghostty](https://github.com/ghostty-org/ghostty) (terminal core and snapshot format, built by `ghostty-vt-sys`) | MIT |
| [`ghostty-vt`](https://crates.io/crates/ghostty-vt) Rust bindings, pinned `=0.8.0` | Apache-2.0 |
| Inter and IBM Plex Mono (bundled through `rune-theme-opencode`) | SIL Open Font License 1.1; license texts also appear in `vendor/rune/crates/rune/assets/` |
| Rex app icon (`resources/icons/`) | Superlogical artwork, not covered by this repository's license; see [resources/icons/README.md](resources/icons/README.md) |

Other crates are used under their own licenses; see `Cargo.lock`.

## Acknowledgements

- [Superlogical Rex](https://www.superlogical.com/rex), the server and protocol this client speaks. The implementation follows the server's `/llms.txt` API revision 8 contract, with a tested revision-7 path.
- [anomalyco/rune](https://github.com/anomalyco/rune) and [a10y/rexrs](https://github.com/a10y/rexrs).
- [Ghostty](https://github.com/ghostty-org/ghostty) for the terminal core.
- [m4nik1/rex-linux-client](https://github.com/m4nik1/rex-linux-client) and [noahdunnagan/rmx](https://github.com/noahdunnagan/rmx), whose protocol work served as references.

## License

MIT. See [LICENSE](LICENSE). Vendored code keeps its own licenses in `vendor/`.
