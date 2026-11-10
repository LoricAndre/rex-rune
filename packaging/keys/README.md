# Release public keys

The owner must supply these two minisign public-key files before the first signed release:

- `release.pub`: the CI release key.
- `release-backup.pub`: the offline recovery key, never supplied to CI.

Neither key exists in this change. Do not use a generated test key or a placeholder
as a release trust root. Tag publication fails closed until both public keys and
the environment secrets are configured. See [release setup](../../docs/releases.md#owner-setup-before-the-first-signed-release)
for the commands and review requirements.

Only public keys belong here. Keep secret keys and passwords outside the checkout.
The future updater will embed both public keys; no client verification is added
by this change.
