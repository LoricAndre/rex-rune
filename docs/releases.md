# Publishing a binary release

`.github/workflows/release.yml` builds natively on Ubuntu 24.04 for x86-64 and ARM64. Both architectures must pass formatting, Clippy, application tests, SDK tests, packaging and install/uninstall checks before publication. The toolchains come from `mise.toml`. No client auto-update code is included in the signed-release pipeline.

## Release sequence

Complete the [owner setup](#owner-setup-before-the-first-signed-release) before the first signed tag. Key custody (U6) is still an owner decision; the implemented default is signing in `publish`, inside the reviewed `release` environment. The isolated signing step can be replaced with [manual signing](#manual-or-offline-signing).

1. Update the version in `Cargo.toml`, the `rex-linux` entry in `Cargo.lock`, and `nix/package.nix`; update `CHANGELOG.md` and installation examples as needed.
2. Commit and push to `master`. Run **Binary release** manually in GitHub Actions to test without publishing. The `release-tools` job tests the manifest and signs/verifies fixture metadata using encrypted throwaway keys, deleted at test exit. The `publish` job gathers real build artifacts and generates metadata, but **never creates or publishes a release on `workflow_dispatch`, even when run on a tag**. Approve the environment if prompted. With either signing secret absent it explicitly skips real signing; with both present it signs and verifies, failing on a bad key/password or missing public keys. Manual runs do not create attestations.
3. After checking the results, create and push an annotated tag matching the package version, for example `git tag -a v0.5.0 -m 'Rex Linux 0.5.0'` and `git push origin v0.5.0`. Only stable `vX.Y.Z` versions are supported by the manifest generator.
4. The tag workflow rebuilds and attests all four packages using `actions/attest-build-provenance`. Inspect the triggering commit and workflow, then approve the `release` environment as its required reviewer. Builds have no signing secrets; `publish` alone receives them.
5. `publish` gathers both architectures, writes `SHA256SUMS`, generates `latest.json`, signs both metadata files, and verifies each signature against the repository's public keys. **A tag run fails closed** if either secret, either public-key file, or a valid signature is missing. Nothing is published on failure.
6. Only after verification does the job create a draft, upload **all eight assets** (two tarballs, two `.deb`s, `SHA256SUMS`, `SHA256SUMS.minisig`, `latest.json`, `latest.json.minisig`), then publish and mark latest. Check the assets and attestations. Never move published tags, replace assets, or retrofit signatures onto old releases.

The manifest is a release asset at:

- `https://github.com/shuv1337/rex-rune/releases/latest/download/latest.json`
- `https://github.com/shuv1337/rex-rune/releases/latest/download/latest.json.minisig`

Its schema is `1`, product is `rex-linux`, and both `x86_64-linux-gnu` and `aarch64-linux-gnu` targets are mandatory. Each target has `min_glibc`, a version-specific tarball URL, and a Debian filename, with sizes in bytes and SHA-256 digests for both packages. The generator checks all four package hashes against `SHA256SUMS`; missing, extra, duplicate, malformed or mismatched entries fail. Output is stable for fixed inputs. `published` records metadata generation time (before publication), using an RFC 3339 Pacific Time offset (PST/PDT); `--published` allows a fixed timestamp for reproducibility. There is **no expiry**.

`update_from` is currently `0.4.2`, the current compatible tarball layout, not a claim that that version already has an updater. Raise the workflow's explicit floor only when an installation-layout change requires a manual reinstall. `min_glibc` defaults to `2.39`; change `--min-glibc` with any future build baseline change. Trusted signature comments name the tag and file, for example `rex-linux v0.5.0 latest.json`.

## Owner setup before the first signed release

**No production keys or secrets are supplied by this change. Only the owner creates them.** Do not paste a private key or password into chat, a PR, the repository, or a command-line argument. The backup private key must never reach GitHub. U1 (the APT address), OpenPGP keys, Pages, and `APT_SIGNING_KEY` belong to the later APT PR and are not needed here.

Exactly what to create/configure:

1. **Two password-protected minisign key pairs.** On a trusted machine, outside any checkout, run the following yourself. Minisign prompts for passwords. Create the backup pair on your offline machine instead; bring back only its public half. Keep the backup secret and its password offline in two secure places.

   ```sh
   umask 077
   # On the trusted signing machine, in a private directory outside the repo:
   minisign -G -p release.pub -s rex-linux-release.key
   # On the offline backup machine:
   minisign -G -p release-backup.pub -s rex-linux-release-backup.key
   ```

2. **Commit only the two public files**, as `packaging/keys/release.pub` and `packaging/keys/release-backup.pub`, in a reviewed change. Print both public-key values in the README in that change, replacing the pending-setup wording. Never commit a throwaway test key as a trust root. The signing helper requires both public files and accepts signatures from either; the later client PR will embed both.

   ```sh
   # From the checkout, after copying only public files from the machines above:
   install -m 644 /path/to/public/release.pub packaging/keys/release.pub
   install -m 644 /path/to/public/release-backup.pub packaging/keys/release-backup.pub
   git add packaging/keys/release.pub packaging/keys/release-backup.pub
   ```

3. **Create the GitHub environment `release`** at <https://github.com/shuv1337/rex-rune/settings/environments>. Add **shuv1337 as required reviewer**, disable administrator bypass of protection rules, and allow only the `master` branch and `v*` tags under **Selected branches and tags**. Leave “Prevent self-review” off for the single-maintainer workflow, or designate a second human reviewer if enabling it. Verify the protections before supplying secrets; a workflow's `environment:` declaration alone does not configure them.

4. **Create these two environment secrets**, not repository-wide secrets. The key secret is the complete password-encrypted minisign secret-key file, not base64 wrapped. Supply the password interactively when `gh` prompts; never put it into shell history. Do not upload the offline key.

   ```sh
   gh secret set RELEASE_MINISIGN_KEY --repo shuv1337/rex-rune --env release < /secure/path/rex-linux-release.key
   gh secret set RELEASE_MINISIGN_PASSWORD --repo shuv1337/rex-rune --env release
   # Lists names, not secret values:
   gh secret list --repo shuv1337/rex-rune --env release
   ```

5. **Create an active tag ruleset for `v*`** at <https://github.com/shuv1337/rex-rune/settings/rules>. Restrict tag creation to the maintainer (repository-admin bypass for creation), and restrict updates and deletion. Never use that bypass to move/delete a release tag. This controls who can start a release; it does not replace reviewing the commit before granting the signing environment access.

6. **Enable immutable releases** in the repository's Settings → General → Releases before the first signed release. Drafts remain editable; publication locks their assets and tag. The latest-release pointer remains editable. Existing releases are not retroactively signed or made immutable by this workflow.

7. Run the local tests below and, when ready, a manual **Binary release** run. Inspect its `release-metadata` artifact, verify it against your retained public keys, and confirm environment review was enforced before pushing any release tag. Attestation permissions (`id-token: write`, `attestations: write`) are already scoped to the build job; no extra signing secret is needed for provenance.

## Local checks and packaging

```sh
# Python 3.11+ and minisign required; no production credentials or GitHub access.
python3 scripts/test-release.py
# If installed (also runs ShellCheck when available):
actionlint .github/workflows/release.yml
```

To package locally after a native `cargo build --locked --release --bin rex-linux`, run `python3 scripts/package-release.py` (tarball) or add `--deb` (also requires `dpkg-dev`). Outputs go into ignored `dist/`. Packages include source commit/toolchain information, desktop integration, dependency license declarations, available license texts, font licenses, and icon provenance. Icon redistribution is authorized by the project owner; it is not covered by the project's MIT license.

The compatibility baseline is the build distribution, not just CPU architecture: current official binaries require glibc 2.39+. Build on an older supported distribution if changing this baseline; do not describe newer-host builds as compatible with older glibc. Runtime-loaded graphics/windowing dependencies are listed explicitly in the Debian package in addition to linked dependencies resolved by `dpkg-shlibdeps`.

## Manual or offline signing

For U6's alternative, keep building and generating metadata in CI but replace only **Sign and verify metadata** with importing the manually signed files and running `--verify-only`; keep that gate before draft publication. This PR does not automate an offline handoff. Never publish unsigned files while waiting for signatures. On the signing machine, with the exact `latest.json` and `SHA256SUMS` from the same build run in `dist/`:

```sh
# Set this to the exact candidate tag; minisign prompts for the key password.
tag=v0.5.0
for file in latest.json SHA256SUMS; do
  minisign -S -s /secure/path/rex-linux-release.key -m "dist/$file" \
    -t "rex-linux $tag $file"
done
# Same verification gate as CI; uses no private key or environment secrets.
python3 scripts/sign-release.py --tag "$tag" --verify-only
```

Transfer only the two `.minisig` files back. Do not regenerate/reformat the signed metadata. Verify the signatures, package hashes and candidate tag/commit before uploading. [README verification instructions](../README.md#verify-a-download) cover manual downloads and `gh attestation verify`.

## Recovery and rotation

If a publish job fails after creating a draft, inspect the draft before retrying; the workflow intentionally refuses to overwrite an existing release and never uses `--clobber`. Recover by downloading packages and the `release-metadata` artifact from **that exact workflow run**, checking all four hashes (`sha256sum -c SHA256SUMS`) and both signatures (`scripts/sign-release.py --tag vX.Y.Z --verify-only`), and uploading only missing assets to the draft. Publish only after all eight match. Do not rebuild and mix artifacts from unrelated commits or re-sign a different manifest under the same published tag.

**Pulling a bad release:** publish a fixed patch version. Meanwhile mark the bad release as a prerelease so it is not offered as latest, and confirm the latest pointer serves the previous good manifest. Do not delete its assets or move the tag. No automatic downgrade is implied. Once the APT repository exists, rebuild it without the bad release as well.

**Planned minisign rotation:** once the updater embeds keys, ship a release signed by the old key that trusts the new key too. Keep signing with the old key for a long overlap (a year), then switch the CI secret and committed active public key. Installs older than the bridging release must reinstall manually. Coordinate future client trust-root changes with the CI verification keys; do not simply replace a key embedded in installed clients.

**Suspected compromise:** delete the leaked GitHub environment secret. Sign the next release manually with the offline backup key (never upload that key to CI); that release must remove the compromised client trust root and introduce a newly generated backup. Create a replacement CI key and password through the reviewed setup above. Keeping the backup public key in the app from its first updater release makes recovery possible. Losing both private keys requires manual reinstalls; there is no unsigned-update escape hatch.

APT signing and OpenPGP rotation are intentionally deferred to the APT PR.
