# Rex Linux: product

## Purpose

A native Linux client for [Rex](https://www.superlogical.com/rex) servers, aimed at people who do daily coding work across several machines. It attaches to server-owned sessions that other Rex clients (for example the official Mac and iOS apps) can use at the same time. Closing the client leaves programs running on the server.

## Platform and stack

Native Linux desktop (Wayland and X11). Rust, [Rune](https://github.com/anomalyco/rune) (wgpu/winit, rune-kit) and libghostty-vt.

## Principles

- **Attachment only.** The client never starts, stops or reconfigures a server, and never creates or destroys a session unless asked explicitly.
- **The server owns the terminal.** Terminal content, colours and split geometry come from the server; the client draws chrome around them and never recolours output.
- **Hosts → sessions → tabs.** Saved hosts contain sessions; a session's Rex windows are tabs and its splits are the server's. Navigation must stay useful with a dozen or more sessions across several hosts.
- **Compact and calm.** One toolbar, rounded pane cards on a flat dark ground. Popups and highlights animate; terminal content never does.
- **Safe by default.** Closing a pane or tab names what will end and defaults to **Keep running**. A transport failure pauses input rather than queueing it.
- **Keyboard first.** Workspace shortcuts are reserved by the client; every other key reaches the focused terminal.

## Capabilities

Saved hosts, sessions, tabs, authoritative splits, snapshot restoration, live output, input, mouse reporting, reconnect and resume. API revision 8 is used when the server offers it, with a revision-7 compatibility path. On revision 8, bootstrap bundles the initial terminal tickets; live streams and data resume take priority over new snapshots. Program activity (OSC 7501) marks tabs and, for tabs out of view, sends desktop notifications. Settings can set up Rex's skill and hooks for coding agents on this machine through the local `rex skill` command, previewing every change.

## Out of scope for now

Terminal-content search, complete Kitty graphics, accessibility (AT-SPI) and distribution packages. See the README for the current list of unvalidated areas.
