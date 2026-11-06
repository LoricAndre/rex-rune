# Rex Linux: product

## Purpose

A native Linux client for [Rex](https://www.superlogical.com/rex) servers, aimed at people who do daily coding work across several machines. It attaches to server-owned sessions that other Rex clients (for example the official Mac and iOS apps) can use at the same time. Closing the client leaves programs running on the server.

## Platform and stack

Native Linux desktop (Wayland and X11). Rust, [Rune](https://github.com/anomalyco/rune) (wgpu/winit, rune-kit) and libghostty-vt.

## Principles

- **Attachment only.** The client never starts, stops or reconfigures a server, and never creates or destroys a session unless asked explicitly.
- **The server owns the terminal.** Terminal content, colours and split geometry come from the server; the client draws chrome around them and never recolours output. The client may declare a terminal theme to the server, as it declares a size: Rex applies it at once when this client owns the terminal or no client does, and otherwise when this client becomes the owner. Rex keeps each client's last declaration for each terminal, and the terminal shows its owner's; an owner that declared nothing leaves the colours as they were. So while this client owns a terminal (or nobody does), other clients viewing it see the declared theme, just as they see its size, and when another client that declared a theme takes it over, everyone sees that one. Because Rex keeps a declaration across launches, following the server's colours is not silence once this client has declared any theme: from then on it declares Rex's own colours (an empty theme) once per terminal per connection to each terminal it owns or nobody owns, so an old theme never comes back unasked. A client that has never declared a theme declares nothing.
- **Hosts → sessions → tabs.** Saved hosts contain sessions; a session's Rex windows are tabs and its splits are the server's. Navigation must stay useful with a dozen or more sessions across several hosts.
- **Compact and calm.** One toolbar, rounded pane cards on a flat dark ground. Popups and highlights animate; terminal content never does.
- **Safe by default.** Closing a pane or tab names what will end and defaults to **Keep running**. A transport failure pauses input rather than queueing it.
- **Keyboard first.** Workspace shortcuts are reserved by the client; every other key reaches the focused terminal.

## Capabilities

Saved hosts, sessions, tabs, authoritative splits, snapshot restoration, live output, input, mouse reporting, reconnect and resume. API revisions 10 and 11 are supported; older servers are refused rather than carried along while Rex is in alpha. Bootstrap bundles the initial terminal tickets; live streams and data resume take priority over new snapshots. On revision 11, files dropped on a pane or pasted are uploaded to the terminal's machine and their paths typed into it. Program activity (OSC 7501) marks tabs and, for tabs out of view, sends desktop notifications. Settings can set up Rex's skill and hooks for coding agents on this machine through the local `rex skill` command, previewing every change.

## Out of scope for now

Terminal-content search, complete Kitty graphics, accessibility (AT-SPI) and distribution packages. See the README for the current list of unvalidated areas.
