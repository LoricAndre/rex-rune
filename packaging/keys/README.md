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

## Offline recovery key — `release-backup.pub`

- Minisign key ID: `368EA1A12E2721F3`
- SHA-256 fingerprint (decoded 42-byte minisign public-key record):
  `d9fe424ca8b20725e11d6039c39445d38a33023a8d366c69e0612a1c3528de76`

```text
untrusted comment: minisign public key 368EA1A12E2721F3
RWTzIScuoaGONtqf+4m48HAGuwNgUxvd/AFZareIgMwCGCKVjbTjcmJe
```

Only this public half was supplied for the setup PR. Its private key and passphrase
remain on the owner's separate backup machine, never in this checkout or GitHub.
Keep recoverable offline copies; never send the backup secret or its passphrase.

**Both required public-key files are present.** A tag release no longer fails
for a missing public key, but still fails closed on missing secrets or invalid
signatures. Manual workflow tests use throwaway keys and do not enter the
`release` environment; they cannot prove production-secret availability. The
future updater will embed both public keys; no client verification is added
by this change.

See [release setup](../../docs/releases.md) for verification and recovery.
