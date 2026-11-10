# Rex Linux SDK patch

Vendored from `https://github.com/a10y/rexrs` at
`780be3f9f0455c758273e57199af30bdaa145cf1`. The upstream manifest declares
`MIT OR Apache-2.0`; that revision ships no license text files, so the standard
`LICENSE-MIT` and `LICENSE-APACHE` texts were added here.

Local changes:

- `codegen/help.json` and `version.json` capture Rex `0.1.0-preview.14`,
  commit `7e460d02f51573990bfcdc6c44a5606d11c7d47a`, API revision 12 (`rex api list --json` and `server.version` from a disposable
  server). `python3 codegen/generate.py` regenerates the committed types and
  example tests. The API additions since upstream are
  `session.bootstrap.data_tickets` (8), layer `state` and
  `session.set_layer_state` (9), the Tailscale `control_url` (10) and
  `terminal.upload_begin` (11), and optional `listeners` in `server.status`
  and `session.bootstrap.server_status` (12). No methods, events, keymap or
  action contracts changed between the previous revision-11 capture and 12.
  See `docs/server-preview14.md` in the client repository for the comparison.
- `ClientBuilder::api_revisions(min..=max)` passes an explicit supported range
  to HELLO. The SDK default still advertises only its generated revision.
  Rex Linux advertises 10–12, validates WELCOME and gates uploads on 11.
- A focused negotiation test accompanies the builder change.
- Hand-written `tests/listeners.rs` covers the new optional listener metadata in
  status and bootstrap, including omitted/null lists and unknown enum-like strings.
- The generator preserves `attach_session_ids` as `Option<Vec<String>>`:
  omitting it selects a default session, while an explicit `[]` attaches none.
  A serialization test covers this distinction.

Tests: `cargo test --manifest-path vendor/rexrs/Cargo.toml --all-features`.

Remove this patch once upstream has equivalent generated types and negotiation.
