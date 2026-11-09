# Revision-11 keymap contract

Captured through `src/bin/probe.rs` from Rex
`0.0.1793910724-g2e71f8` (API 11). `response.json` is the full,
unmodified `client.keymap` result, including the throwaway config path and
process ID. No daily socket or tailnet host was used.

Reproduce from the repository root:

```sh
cargo build --locked --no-default-features --bin rex-probe
python3 scripts/keymap-fixture.py
cargo test --locked --test keymap_fixture
```

The script explicitly sets `REX_SERVER`, starts a foreground server with a
private Unix socket, config and state directory, disables restore, captures
the reply, and terminates only that process. Each run changes the config
path and instance ID; the config hash and keymap contents should be stable.

## Observations used by Phase 6

- Standard modifier order is `ctrl+alt+shift+cmd`; `WIN` canonicalizes to
  `cmd`, not `super`. `super`, `meta` and `command` also become `cmd`;
  `opt` becomes `alt` (`option` is an alias too). Named keys are lowercase.
- Physical codes retain W3C case: `ctrl+[KeyT]`.
- `ctrl++` survives verbatim. So does an unknown spelling
  (`ctrl+FutureKey`); preserving a spelling does not imply it is parseable.
- Try order is reverse binding order within each layer, then descending
  layer precedence. Same-key bindings are retained, not overwritten.
- Lua functions become `client.key.forward`, including sequences. Their
  returned `keys` must be used verbatim as `client.key.prefix`.
- File removal and client removal both remove exact keys from lower layers.
- An unavailable Mac-only action remains in the result and has a diagnostic.
  It is not a server action.
- A sequence hiding a default produces a diagnostic about the losing default;
  it does not remove it. A key hiding a sequence reports `cannot start` and
  names the winning action. Diagnostic source is essential to notice policy.
- `nivox-defaults.json` is the complete 29-entry default list from Andrea Zito
  (nivox), `8037d78`, retained as an import regression fixture.

This fixture supports the implementation informed by Andrea Zito (nivox)'s
`8037d78` shortcut work; it records Rex's protocol rather than the fork's
client-local resolution.
