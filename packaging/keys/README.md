# Release public keys

Only public keys belong here. Keep secret keys and passwords outside the checkout.
Never upload the offline backup secret or its passphrase to CI.

## CI signing key — `release.pub`

- Minisign key ID: `A1FD78C27652816E`
- SHA-256 fingerprint (decoded 42-byte minisign public-key record):
  `9e779605311a46a5287d6c9b25eb2083fb17447454239c7d66f1ecebe336d0c6`

```text
untrusted comment: minisign public key A1FD78C27652816E
RWRugVJ2wnj9oVdIvBhQFK1/BDvL1KkA4aH1HB2Q0uA30OBHXIvTlHwL
```

The encrypted CI private key and random passphrase are environment secrets in
GitHub's protected `release` environment, not repository-wide secrets. Each tag
release requires approval by shuv1337; administrator bypass is disabled.

## Offline backup public key — PENDING OWNER INPUT

- Destination: `packaging/keys/release-backup.pub` (intentionally absent).
- Minisign key ID: **PENDING**.
- SHA-256 fingerprint: **PENDING**.
- Complete public-key text: **PENDING**.

Generate the backup on your own offline machine, keep its private key and
passphrase offline, and send only the complete two-line `release-backup.pub`
content as a chat message or `.pub` attachment. Commit it through a reviewed PR,
then replace these pending fields with its actual public value, ID and fingerprint.
Do not substitute the CI key, a test key, or a placeholder file.

**Tag publication currently fails closed because the backup public key is missing.**
Manual workflow tests use throwaway keys and do not enter the `release` environment;
they cannot prove production-secret availability. The future updater will embed
both public keys; no client verification is added by this change.

See [release setup](../../docs/releases.md) for verification and recovery.
