# Auto-updating installed releases

Plan for the request of 2026-11-09 (PST), written the same day against `master` at `3e731a8` (Rex Linux 0.4.1). This is a plan only: nothing here is implemented, no keys or secrets exist, and the open questions in section 14 need answers before work starts.

## Summary

Rex Linux is installed four ways (a `.deb`, a tarball in `~/.local/bin`, Nix, a source build) and none of them learns about a new release. The plan gives each channel the update path its users already expect, and one notice that works for all of them.

- **Signed releases come first.** Every release gains a small signed manifest, `latest.json`, and a signature over `SHA256SUMS`, both made with [minisign](https://jedisct1.github.io/minisign/). The app carries the public keys. Everything else builds on this.
- **deb:** a signed APT repository on GitHub Pages, rebuilt from the release assets on every release. The `.deb` ships its own `.sources` file with the key inside it, so `sudo apt install ./rex-linux_X.deb` is still the whole setup and later versions arrive with `apt upgrade`.
- **Tarball:** `rex-linux --update` and an **Update** button. It downloads the tarball named in the signed manifest, checks it, swaps the binary with one `rename`, keeps the old one for `rex-linux --rollback`, and offers **Restart now**. Sessions stay on their servers, so a restart costs nothing but a reattach.
- **Nix:** no self-update. The notice names the Nix command; the docs explain which flake reference follows releases.
- **Every channel:** one check a day of `latest.json`, switchable off, sending nothing but an HTTPS request. It shows "0.4.3 available" in Settings → Updates with the action for that channel, and one toast per new version.
- **Security:** a changed download fails its signature in the app and in APT. A stolen repository token cannot sign, because the keys sit in a GitHub environment that needs your approval. An attacker who holds the hosting can at worst hide updates; the app never goes down a version.
- **Size:** six pull requests after this plan. The first two need no client code and can ship in the next release.

## 1. Where things stand

Checked on 2026-11-09 (PST):

| Fact | State |
| --- | --- |
| Release assets (v0.4.1) | two tarballs (about 15.7 MB each), two `.deb`s (about 11.6 MB each), `SHA256SUMS`. No signatures. |
| Release workflow | `release.yml`: native build on `ubuntu-24.04` and `ubuntu-24.04-arm`, then a `publish` job that creates a draft, uploads, and publishes. No secrets, no environment. |
| GitHub Pages | not enabled on `shuv1337/rex-rune`. |
| Environments, tag rulesets | none. |
| Immutable releases | off (`isImmutable: false` on v0.4.1). |
| `…/releases/latest/download/<asset>` | redirects to the latest release's asset; no API token and no API rate limit. |
| HTTP client in the app | none. `ureq` 3.5, `sha2` and `tar` are already in `Cargo.lock` as build dependencies of `ghostty-vt-sys`; `flate2` is a normal dependency. |
| Notices | text only (`Workspace::notify` / `inform`); no buttons. Settings has six sections. |
| The same binary | goes into both the tarball and the `.deb` (CI compares them), so a compile-time flag cannot tell those two apart. |

`SHA256SUMS` protects against a damaged download and nothing else: whoever can replace a package can replace the checksum file next to it.

One thing noticed in passing: the `.deb`'s description still says "API revision 10 or 11" while the README says 10, 11 or 12. It is not part of this plan.

## 2. What comparable apps do

| App | deb / package users | Tarball or user install | Notice |
| --- | --- | --- | --- |
| **VS Code** | The `.deb`'s `postinst` writes `/etc/apt/sources.list.d/vscode.sources` (deb822, `Signed-By: /usr/share/keyrings/microsoft.gpg`) and the key, after a debconf question that defaults to yes. | The Linux tarball does not replace itself; it tells you a new version exists. | In-app, with `update.mode` to turn it off. |
| **Chrome** | The `.deb` adds its APT source and key and installs `/etc/cron.daily/google-chrome` to put them back; `/etc/default/google-chrome` opts out. | none | none in-app on Linux; APT does the work. |
| **Zed** | Distribution packages are built with `ZED_UPDATE_EXPLANATION="…"`, which turns the updater off and shows that text instead. | `install.sh` unpacks to `~/.local/zed.app`; the app downloads the next tarball and replaces that directory. `"auto_update": false` turns it off. | In-app. |
| **Ghostty** | No official Linux binaries; distributions package it. Source tarballs are signed with minisign. | Updates only on macOS (Sparkle, EdDSA-signed feed). `auto-update = off \| check \| download`. | macOS only. Its docs state that checking sends nothing beyond what the protocol needs. |
| **WezTerm** | An APT repository on a hosted service (`apt.fury.io/wez`), set up with a key in `/usr/share/keyrings` and a `signed-by` source line. | none | `check_for_updates = true`, every 86 400 s, against GitHub; "it doesn't automatically download the release". |
| **Tauri apps** | n/a | The updater plugin reads a static `latest.json`, commonly at `…/releases/latest/download/latest.json`, and checks a minisign signature against a public key built into the app. | In-app. |

Patterns this plan takes:

1. A vendor `.deb` that installs its own APT source (VS Code, Chrome), but as a plain packaged file rather than a `postinst` script or a cron job.
2. A build-time marker so packaged builds never update themselves and say how to update instead (Zed).
3. A daily check that only notifies unless the install is user-owned (WezTerm, Ghostty's `check`).
4. A static signed manifest behind the "latest release" redirect, with the key in the app (Tauri). Zed and VS Code rely on HTTPS to their own servers; this project's downloads come from a shared host, so a signature is worth having.

## 3. Release signing and the manifest

### Why minisign

| Option | For | Against |
| --- | --- | --- |
| **minisign** (recommended) | Ed25519. One-line public key. People can check a download with `minisign -Vm`. The `minisign-verify` crate (0.3.0, no dependencies) verifies in the app. Ghostty, Zig and Tauri use it. | A secret key to look after. |
| Sigstore / cosign, keyless | No key to keep. | Verifying in the app needs `sigstore-rs` and its trust-root refresh: a large dependency tree and more network calls. A signature proves "a workflow in this repository ran at a `v*` tag", which anyone who can push a tag can arrange, so it is weaker than a key behind an approval. |
| Raw Ed25519 with `ring` | No new crate. | A home-made file format and no standard tool for people to verify with. |
| OpenPGP (reuse the APT key) | One key. | Verifying in the app needs an OpenPGP library or `gpgv`, which tarball users on non-Debian systems may not have. |

APT only accepts OpenPGP, so there are two keys whatever is chosen: an OpenPGP key for the repository (section 4) and minisign for everything the app verifies.

GitHub's artifact attestations (`actions/attest-build-provenance`) can be added to the build for people who use `gh attestation verify`. They cost one workflow step and are not something the app checks.

### New release assets

Each release gains three small files, uploaded to the draft with the packages and published with them:

| Asset | Purpose |
| --- | --- |
| `latest.json` | The manifest the app reads. |
| `latest.json.minisig` | Its signature. |
| `SHA256SUMS.minisig` | For people verifying a download by hand. |

The app fetches `https://github.com/shuv1337/rex-rune/releases/latest/download/latest.json` and the `.minisig` beside it. Because the manifest is an asset of the release it describes, it appears at the same moment as the packages, is never edited afterwards, and needs no second host. Marking a release as latest is the only "pointer", and GitHub keeps that editable even for immutable releases.

```json
{
  "schema": 1,
  "product": "rex-linux",
  "version": "0.4.3",
  "tag": "v0.4.3",
  "published": "2026-11-20T18:00:00Z",
  "notes_url": "https://github.com/shuv1337/rex-rune/releases/tag/v0.4.3",
  "update_from": "0.4.2",
  "targets": {
    "x86_64-linux-gnu": {
      "min_glibc": "2.39",
      "tarball": {
        "url": "https://github.com/shuv1337/rex-rune/releases/download/v0.4.3/rex-linux-0.4.3-x86_64-linux-gnu.tar.gz",
        "size": 15825007,
        "sha256": "…"
      },
      "deb": { "name": "rex-linux_0.4.3_amd64.deb", "size": 11771002, "sha256": "…" }
    },
    "aarch64-linux-gnu": { "…": "…" }
  }
}
```

- The signature covers the exact bytes of the file. The app verifies first and parses second.
- `product` must be `rex-linux`, so a signature made with the same key for another project is useless here.
- `update_from` is the oldest version that may update itself to this one. An older install is told to reinstall by hand. It exists for the day the install layout changes.
- `min_glibc` lets the app decline before downloading when a future release raises the baseline.
- Unknown fields are ignored. A `schema` the app does not know shows "a newer version is available" with a link and nothing more.
- The minisign trusted comment names the tag and file (`rex-linux v0.4.3 latest.json`).

`scripts/release-manifest.py` writes the manifest from `dist/` and `SHA256SUMS`, and a unit test checks that the two agree.

### Keys

- **Release key:** a minisign key pair whose secret half, with its password, is a secret in a GitHub environment named `release`.
- **Backup key:** a second minisign key pair made at the same time, kept offline and never given to CI. Its public half is in the app from the first release.
- Public keys live in the repository (`packaging/keys/release.pub`, `release-backup.pub`) and are compiled into the app. The README prints both.

The app accepts a manifest signed by either key. That makes the backup key the revocation plan:

| Event | Response |
| --- | --- |
| Planned rotation | Release N, signed with the old key, carries the new public key as well. Keep signing with the old key for a long overlap (a year), then switch. Installs older than N must reinstall by hand once. |
| Release key leaked or suspected | Sign the next release with the backup key. Every install already trusts it. That release drops the leaked key and carries a new backup. Delete the leaked secret from GitHub. |
| Both keys lost | Self-update stops. The notice still cannot be forged; people reinstall by hand. |

A leaked key alone does nothing: the attacker must also control what the app downloads. An install that has not yet updated past the leak still trusts the old key, and no design fixes that without the app phoning somewhere the attacker does not control.

## 4. deb: a signed APT repository

### Hosting

| Option | For | Against |
| --- | --- | --- |
| **GitHub Pages, deployed by Actions** (recommended) | No new account. Site limit 1 GB and a soft 100 GB a month; five versions for two architectures are about 120 MB. The project gets no access logs, which suits a no-telemetry promise. | Cannot redirect, so the `.deb`s are copied into the site. |
| Cloudflare R2 behind a custom domain | No size worries; could serve only the indexes and redirect `pool/` to release assets. | A second account and API token in CI. Access logs with IP addresses exist. |
| Release assets | Nothing to host. | APT needs fixed `dists/` and `pool/` paths; release asset URLs are flat and per-tag. Not workable. |

The address goes into every user's `sources` file and stays there, so it matters more than the host behind it. A custom domain (question 1) lets the host change later without anyone editing a file. The examples below write `APT_URL` for whichever is chosen: `https://shuv1337.github.io/rex-rune/apt` or a domain of yours.

### Tooling

| Tool | Fit |
| --- | --- |
| **`apt-ftparchive`** (recommended) | Stateless: it writes `Packages` and `Release` from a directory of `.deb`s. Ships in `apt-utils` on the runner. |
| `reprepro` | Keeps a database that would have to be stored between runs. Suits a repository with many packages and suites. |
| `aptly` | Snapshots and S3 publishing; much more than one package needs. |

The repository is a **pure function of the published releases and the signing key**. `scripts/build-apt-repo.py`:

1. Lists the newest five non-prerelease releases from the first signed release on.
2. Downloads their `.deb`s, `SHA256SUMS` and `SHA256SUMS.minisig`; verifies the signature with the repository's own public key and then the checksums. A release asset that was tampered with never reaches the APT repository.
3. Lays out `pool/main/r/rex-linux/` and runs `apt-ftparchive` for `dists/stable/main/binary-{amd64,arm64}/Packages{,.gz}` and `Release`.
4. Signs `InRelease` (clearsigned) and `Release.gpg` with SHA-512.
5. Copies `rex-linux.sources` to the site root.

`Release` carries `Origin: rex-linux`, `Label: Rex Linux`, `Suite: stable`, `Codename: stable`, `Components: main`, `Architectures: amd64 arm64`. One suite serves every distribution: the package's own `Depends` (from `dpkg-shlibdeps`) already refuses systems older than glibc 2.39. Keeping five versions lets someone pin or go back with `sudo apt install rex-linux=0.4.2`.

Nothing is stored between runs, so a failed deploy is fixed by running the job again, and the workflow can be started by hand to rebuild after a key rotation.

### The repository key

- An OpenPGP key with an Ed25519 primary key that only certifies and stays offline, and an Ed25519 signing subkey that expires in three years. Only the subkey goes to CI.
- The public certificate is committed as `packaging/apt/rex-linux-archive-key.asc` and embedded in `rex-linux.sources`.
- **Rotation has a trap:** an installed `sources` file holds the old certificate, which does not know a new subkey or a new expiry date. So rotate early. From two years in, the release job fails when the subkey has under 180 days left. To rotate: add the new subkey, publish the certificate (in the `.deb` and on the site), and sign `InRelease` with both subkeys until the old one expires. Anyone who upgrades in that window gets the new certificate through the package.
- **Revocation:** revoke the subkey with the offline primary, add a new one, republish. Installs holding the old certificate see a signature error on `apt update` until they fetch `rex-linux.sources` again. As with minisign, a leaked subkey is only useful to someone who also controls the site.

### Setup for users

The one-line setup for someone who has no `.deb` yet:

```sh
sudo curl -fsSLo /etc/apt/sources.list.d/rex-linux.sources APT_URL/rex-linux.sources
sudo apt update && sudo apt install rex-linux
```

```
Types: deb
URIs: APT_URL
Suites: stable
Components: main
Signed-By:
 -----BEGIN PGP PUBLIC KEY BLOCK-----
 .
 mDMEZ…
 -----END PGP PUBLIC KEY BLOCK-----
```

The key is inside the file (supported since APT 2.4; every system with glibc 2.39 has a newer one), so there is one file to fetch, one to remove, and no keyring that can go missing.

### Should the `.deb` ship the source?

**Yes, as a packaged conffile and nothing more.** The `.deb` installs `/etc/apt/sources.list.d/rex-linux.sources`, byte for byte the file on the site. No `postinst`, no debconf question, no cron job.

- Today's instruction, `sudo apt install ./rex-linux_X_amd64.deb`, becomes the complete setup. This is what people expect from Chrome and VS Code.
- dpkg's conffile rules give the opt-out for free: set `Enabled: no` or delete the file and upgrades leave it alone.
- A new certificate reaches users through a normal package upgrade.
- `apt remove` leaves the file (it still works, since the key is inside it); `apt purge` removes it.
- If someone ran the one-line setup first, the file on disk is identical to the packaged one and dpkg asks nothing. The container test covers this.

The README and release notes say plainly that the package adds an APT source and how to turn it off. A release upgrade (`do-release-upgrade`) disables third-party sources; the in-app notice then explains how to turn it back on rather than the package fighting the system as Chrome's cron job does.

### unattended-upgrades

`apt upgrade`, GNOME Software and Update Manager pick the repository up as soon as it is configured. `unattended-upgrades` only installs from origins it is told about, so the README documents a two-line file:

```
// /etc/apt/apt.conf.d/51unattended-upgrades-rex-linux
Unattended-Upgrade::Origins-Pattern:: "origin=rex-linux,codename=stable";
```

The container test confirms the pattern. The package does not install it (question 3): turning on unattended installs of third-party software is the administrator's decision.

## 5. Knowing the channel

The app must never replace a file that a package manager owns, and must not overwrite someone's own build with an official one. Two forks of this repository exist, and a build from either lands at the same `~/.local/bin/rex-linux` path as a tarball install, so the second case is real. Detection fails closed: anything unclear gets a notice and a link, never a self-update.

| Step | Test | Channel | Updates by |
| --- | --- | --- | --- |
| 1 | Built without `REX_LINUX_CHANNEL` (any `cargo build`, `mise run install`, a fork) | **Source** | git. No automatic check. |
| 2 | Built with `REX_LINUX_CHANNEL=nix` (`nix/package.nix` sets it) | **Nix** | Nix. Notice only. |
| 3 | Built with `REX_LINUX_CHANNEL=official` (`release.yml` sets it) and a marker file exists at `../share/rex-linux/channel` relative to the binary's directory | **Package** (`channel=deb` in the `.deb`) | the package manager named in the marker. Notice only. |
| 4 | Official build, and a valid install receipt matches the running binary | **Tarball** | the self-updater. |
| 5 | Anything else (an official binary copied somewhere by hand) | **Unmanaged** | Notice with a link to the release. |

- **The compile-time value** is read with `option_env!`. Both official packages still contain the same binary, built once.
- **The marker file** is `/usr/share/rex-linux/channel` in the `.deb`: `channel=deb`, and optionally `hint=<text to show>`. Any other packager (an AUR `-bin` package, for example) ships the same file with its own name and hint. This is Zed's `ZED_UPDATE_EXPLANATION` as a file, because the binary is shared.
- **The receipt** is `$XDG_DATA_HOME/rex-linux/install.json`, written by the tarball's `install.sh` after it succeeds: the installed path, version and the binary's SHA-256. The app is on the Tarball channel only if the receipt's path is the running executable (resolved), the file's hash matches, the file belongs to the current user, and its directory is writable. `mise run install` and `install-desktop.sh --uninstall` delete the receipt; the updater rewrites it after each update. So a source build copied over a tarball install stops matching and is left alone.
- With both a `.deb` and a tarball installed, the channel is that of the binary that is running.

`rex-linux --check-update` prints the channel and how it was decided, for bug reports.

## 6. The notice, for every channel

### Checking

- **What is fetched:** `latest.json` and `latest.json.minisig`, two small HTTPS GETs through GitHub's redirect. No token, no GitHub API, so the API's 60-requests-an-hour limit never applies.
- **When:** about 30 seconds after launch if the last check was more than 24 hours ago, then every 24 hours while the app stays open, with a few minutes of jitter. The client often runs for days, so launch alone is not enough.
- **State:** `$XDG_STATE_HOME/rex-linux/update.json` holds the time of the last check, the latest version seen, the highest version ever verified, and the version already announced. A lock on it lets only one of several running instances check. A timestamp in the future counts as due.
- **Turning it off:** `updates.check` in `workspace.json` (a switch in Settings), or `REX_LINUX_NO_UPDATE_CHECK=1` in the environment for administrators. Off means no request, no timer and no thread. `--check-update` and **Check now** still work, because the user asked.
- **Source builds** never check on their own.
- **Offline or failing:** silent. A captive portal that returns a login page fails the signature and is treated the same as no network. The next attempt is at the next launch or an hour later, whichever is first, then daily again. Only **Check now** and the CLI report errors.

### Privacy

No identifier, no query string, no cookie and nothing about the machine or its hosts. The `User-Agent` is `rex-linux/<version>`. GitHub sees the IP address and that header, as for any download, and shows a public total download count for each asset. The project gets no log. The README says exactly this.

### What the user sees

DESIGN.md asks for quiet chrome: no status dots, words only for things that need attention.

- **One information toast per new version**, not per launch: "Rex Linux 0.4.3 is available. Settings → Updates has the details." It leaves after six seconds like other information.
- **Settings → Updates**, a seventh section, in the usual row form:

| Row | Control |
| --- | --- |
| **Version**: "Rex Linux 0.4.1, installed from the .deb package." A muted line gives the last check or "0.4.3 is available". | **Check now** |
| **Check for updates**: "Once a day, read the latest release's version from GitHub. Nothing about you is sent." | switch |
| **Install updates automatically** (Tarball only): "Download and verify new releases; they take effect the next time the app starts." | switch, off by default |
| **Update** (only when one is available) | by channel, below |

| Channel | Action shown |
| --- | --- |
| Package (deb, source enabled) | `sudo apt update && sudo apt install --only-upgrade rex-linux`, selectable, with **Copy**. |
| Package (deb, source missing or `Enabled: no`) | How to turn the repository on, and a link to the `.deb`. |
| Package (other marker) | The marker's `hint`. |
| Tarball | **Update to 0.4.3**, then progress, then "0.4.3 is installed. Restart to use it." with **Restart now** and **Roll back to 0.4.2**. |
| Nix | `nix flake update rex-rune` or `nix profile upgrade rex-linux`, selectable. |
| Source | "Built from source. Update with git." |
| Unmanaged | A link to the release page. |

- **Commands** gains "Check for updates" and, when one is known, "Update Rex Linux…", both opening the section.
- Trouble (a download that failed verification, a directory that is not writable) is a line in `WARNING` in that section with its reason. It is a sticky notice only when the user started the action.
- On every channel, if the binary on disk changed since launch (APT upgraded it, or the updater ran in another window), the section says "Restart to use 0.4.3".

## 7. Tarball: the self-updater

### Commands

| Command | Does |
| --- | --- |
| `rex-linux --check-update` | Prints channel, current and latest version and the action for this channel. Always checks, whatever the setting. |
| `rex-linux --update` | Tarball channel: check, download, verify, install. Any other channel: prints that channel's command and exits non-zero without touching anything. |
| `rex-linux --rollback` | Puts the previous binary back. |

All three run in `main` before the window, the GPU or any Rex connection is set up, so they work over SSH and in CI. The **Update** button runs the same code on a worker thread.

### Steps

1. Take the update lock.
2. Fetch and verify the manifest. Refuse unless the version is higher than the running one and at least the highest ever verified, `product` matches, this target is listed, `update_from` allows it and glibc is new enough.
3. Download the tarball to `$XDG_CACHE_HOME/rex-linux/updates/` with the size from the manifest as a hard limit; check size and SHA-256. HTTPS only, including redirects.
4. Unpack into a staging directory. Entries must be plain files and directories under the one expected top directory.
5. **Try it:** run the staged binary with `--version` under a short timeout. It must print the manifest's version. A binary that cannot start on this system stops here, with the old one untouched.
6. Copy the staged binary into the install directory under a temporary name, `fsync` it, copy the current binary to `$XDG_STATE_HOME/rex-linux/previous/`, then `rename` the new one over the old and `fsync` the directory. The rename is atomic: at every instant the path holds a complete binary.
7. Refresh the launcher and icons with the staged `scripts/install-desktop.sh --copy <installed path>`. The script already skips the copy when source and target are the same file. A failure here is reported and does not undo the update.
8. Rewrite the receipt, record the previous version, clear the cache.

### The running app

Linux lets a running program's file be replaced; the process keeps the old one until it exits. So the update installs at once and takes effect at the next start. **Restart now** starts the installed path with the original arguments and then quits. It is always the user's click, never automatic. Nothing is lost: closing the client leaves programs running on the server, and the new process reattaches. If uploads are in flight, the button says so first.

### Rollback

One previous binary is kept. `--rollback` (or **Roll back**) swaps it in with the same rename and marks the version it left as skipped, so neither the notice nor automatic install offers it again; a later version clears the mark. Rolling back is a local act with a local file, so the "never go down a version" rule for downloads does not apply to it. Preferences are not rolled back. `workspace.json` already ignores fields it does not know and reads unknown values as defaults, and that must stay true. `shortcuts.json` entries reject unknown fields, so an entry written for a newer version shows under Problems after a rollback while the defaults keep working.

### Dependencies

`minisign-verify` is the only crate new to the lockfile. `ureq`, `sha2` and `tar` are already locked as build dependencies and become normal ones; `flate2` is already normal. `ureq` is blocking and runs on a worker thread. All are crates.io packages, so `nix/package.nix` needs no new hash and the license bundle picks them up from `cargo metadata`. The code lives in the library (`src/update/`), with no UI dependency.

## 8. Nix

No self-update and no button: the store is read-only and the user's configuration decides the version. `nix/package.nix` sets `REX_LINUX_CHANNEL = "nix"`, so the app only notifies, and the usual switch turns that off.

`docs/nix.md` gains a section on staying current:

| How it was installed | To update |
| --- | --- |
| A flake input (`github:shuv1337/rex-rune`) in NixOS or Home Manager | `nix flake update rex-rune`, then rebuild. |
| `nix profile install github:shuv1337/rex-rune` | `nix profile upgrade rex-linux` |
| Pinned to a tag (`github:shuv1337/rex-rune/v0.4.3`) | Change the tag. |

The plain flake reference follows `master`, which can be ahead of the latest release. Tags are never moved, so a reference that follows releases needs something else: a `stable` branch that the release workflow fast-forwards to each tag, or FlakeHub's version ranges. Both are small; neither is needed for the rest of this plan (question 9). A nixpkgs package and a binary cache are later work.

## 9. Release pipeline

### `release.yml`

| Job | Change |
| --- | --- |
| `build` | `REX_LINUX_CHANNEL=official` for `cargo build`. Packaging adds the marker file and `rex-linux.sources` to the `.deb`, and the receipt step to the tarball's `install.sh`. Still no secrets. New checks below. |
| `publish` | Runs in the `release` environment. After `SHA256SUMS`: write `latest.json`, sign it and `SHA256SUMS`, **verify both against the public keys in the repository** (a wrong secret stops here), then draft, upload everything, publish, as today. Then build and sign the APT repository and upload it as the Pages artifact. |
| `pages` (new) | Deploys that artifact to GitHub Pages. No secrets. |

A separate `apt-repo.yml` with `workflow_dispatch` runs the last two steps alone, to rebuild the repository without a release.

### Where secrets live

One environment, `release`, with:

- `RELEASE_MINISIGN_KEY`, `RELEASE_MINISIGN_PASSWORD`
- `APT_SIGNING_KEY` (the armored secret subkey only)
- **You as required reviewer**, and deployment limited to `v*` tags and `master`.

The approval is the control that matters. A workflow runs from the ref that triggered it, so without it anyone who can push a tag can run their own workflow with the secrets. With it, each release waits for one click from you. Putting both signatures in the one `publish` job keeps it to one click.

### "Never replace binaries, never move tags"

- Signatures and the manifest are **new assets added to the draft**, before publication. Nothing already published changes.
- Old releases are not given signatures after the fact. The APT repository and the updater start from the first signed release.
- Turning on GitHub's **immutable releases** makes the rule something the platform enforces: assets and tags lock at publication, and the draft-then-publish flow the workflow already uses is the supported way to do it. Which release is "latest" stays editable.
- **Pulling a bad release:** publish a fixed patch release. To stop more installs taking the bad one meanwhile, mark it as a pre-release so "latest" returns to the one before; apps then see nothing newer than what they have. Rebuild the APT repository without it. Nobody is moved down a version automatically.

`docs/releases.md` gains the approval step, the three new assets, how to pull a release, and both rotation procedures.

## 10. Security model

| Attacker | Self-updater (tarball) | APT | Notice | Manual download |
| --- | --- | --- | --- | --- |
| On the network, TLS intact | nothing | nothing | nothing | nothing |
| Breaks TLS, or serves altered files from GitHub's storage or Pages | Signature or hash fails; nothing is installed. Can hide updates. | `apt update` fails on the signature. Can hide updates. | Cannot invent a version. Can hide one. | Caught if the user checks `SHA256SUMS.minisig`; not otherwise. |
| Holds a token that can write to the repository | Can publish a fake "latest" release, which fails verification and hides real updates until removed. Cannot sign: the keys need your approval. With immutable releases, cannot alter a published one. | Cannot sign. Pages is deployed by Actions, not from a branch, so a push cannot change the site. | Cannot invent a version; the fake release hides real ones. | A fake release page with unsigned files could fool someone who does not verify. |
| Takes over the GitHub account | **Can approve the environment and sign. Full compromise of every install that updates.** | **Same.** | Same. | Same. |
| Has a signing key but not the hosting | nothing | nothing | nothing | nothing |
| Has a signing key and the hosting | Full compromise of installs that still trust that key. Rotation to the backup key limits it to installs that have not updated since. | Same, until users fetch the new certificate. | Same. | Same. |
| Controls the DNS of a custom domain | not involved | Can send APT elsewhere, where signatures fail. Can serve a false `rex-linux.sources` to **new** users. | not involved | not involved |

What this means in plain terms:

- **First contact is trust on first use**, as with every vendor repository. The first `.deb` or tarball, the README's key and the `sources` file all come from GitHub. The signatures protect every update after that.
- **Account takeover is not covered.** Only signing on a machine that is not GitHub would cover it, at the cost of a manual step in every release (question 6). Hardware two-factor on the account is the practical defence.
- **Downgrade:** the app installs only a version above the one running and never below the highest it has verified. A replayed old manifest is ignored. APT does not move an installed package down by itself.
- **Version floor:** `update_from` in the manifest, and the "highest verified" number in the state file.
- **Hiding updates** cannot be prevented by anything in this plan. See below.

## 11. Left out on purpose

Each of these adds real machinery for a narrow case. They are listed so the choice is visible.

| Left out | The case it covers | Likelihood | Cost |
| --- | --- | --- | --- |
| Expiring metadata (`Valid-Until` in `Release`, an expiry in the manifest) | Someone who controls the hosting replays old, validly signed metadata so users never hear of a fix. | Needs control of GitHub's hosting or TLS. | A scheduled job that signs again every week. If it ever fails, every user's `apt update` reports an expired repository. |
| Automatic rollback after a crash | A release that starts (`--version` works) but crashes on first use. | Low; CI runs the tests on both architectures. | A "started successfully" marker and launch-time logic. `--rollback` covers it by hand. |
| APT `by-hash` indexes | "Hash Sum mismatch" for someone who runs `apt update` in the minutes after a release while the CDN holds mixed files. | Occasional, and it fixes itself. | Keeping the previous run's indexes in a stateless build. |
| A full TUF repository | Layered keys with thresholds and timestamps. | The threats above are the realistic ones. | A new dependency and four roles of keys for one maintainer. |
| Staged roll-outs, beta channel, delta updates | none needed at this size | n/a | n/a |
| Overrides for the update address or key in the shipped app | Testing. | n/a | A way to point the updater elsewhere. The end-to-end test uses a separate probe binary instead. |

## 12. Phasing

Each pull request stands alone and keeps the README honest about what exists.

| PR | Contents | Tests | Needs from you |
| --- | --- | --- | --- |
| **0** | This plan. | n/a | Answers to section 14. |
| **1. Signed releases** (about 300 lines; scripts and workflow only) | `scripts/release-manifest.py`; minisign signing and self-verification in `publish`; `release` environment; public keys in `packaging/keys/`; README "verify a download"; `docs/releases.md`. | Manifest script unit tests (matches `SHA256SUMS`, both targets, stable output). The manual workflow run signs with a throwaway key made in the job and verifies it. | Both minisign key pairs, the environment and its secrets. |
| **2. APT repository** (about 450 lines; scripts and workflow only) | `scripts/build-apt-repo.py`; `pages` job; `apt-repo.yml`; `packaging/apt/`; README setup; PRODUCT.md's "distribution packages" out-of-scope line. | **`scripts/apt-repo-test.sh`, in containers** (`ubuntu:24.04`, `debian:trixie`): build a repository with a throwaway key from two versions of a small test package; `apt-get update` with no warnings; install the old one; add the new one; `apt-get upgrade` moves to it; `unattended-upgrade --dry-run` with the documented pattern picks it up; a changed `Packages` and an unsigned `Release` are both refused. | The OpenPGP key, Pages turned on (source: GitHub Actions), the address decision. |
| **3. The `.deb` carries its source; channel markers** (about 200 lines) | `package-release.py`: `rex-linux.sources` as a conffile, the marker file, the receipt step in `install.sh`; `install-desktop.sh` removes the receipt on uninstall; `REX_LINUX_CHANNEL` in `release.yml` and `nix/package.nix`. | Container test extended: `apt install ./x.deb` creates the source; an identical file already present causes no prompt; a deleted or disabled file stays so after an upgrade; `apt remove` then `apt update` is clean; `purge` removes it. CI checks the receipt after `install.sh` and its removal on uninstall. | nothing new |
| **4. Update check and notice** (about 900 lines) | `src/update/` (manifest, verification, version rules, state, channel); `updates.check`; Settings → Updates; toast; Commands entries; `--check-update`; README and PRODUCT.md. | Unit: signature good, altered, wrong key, wrong product; version and floor rules; schedule with an injected clock; the channel table over a fake filesystem; no request when off. Headless UI: the section for each channel; one toast per version. Integration: a loopback server. | nothing new |
| **5. Self-updater** (about 800 lines) | Download, unpack, try, swap, receipt, `--update`, `--rollback`, the Update and Restart buttons, automatic install switch, `src/bin/update_probe.rs`. | **End-to-end, `scripts/update-e2e.py`, against a local fake release server:** install the real tarball into a temporary home; build a "99.0.0" tarball with the real packaging script and a stub binary; sign with a throwaway key; serve on loopback; update through the probe, which calls the same library code with the test key and address. Checks: binary replaced, previous kept, receipt rewritten, rollback restores. Refusals: altered tarball, altered manifest, other key, lower version, missing target, short download, stub that fails `--version`, server down, a `.deb`-style marker, a receipt whose hash does not match. Also: the shipped binary's `--update` on the CI's `.deb` install refuses without any network. | nothing new |
| **6. Nix and polish** (about 100 lines) | `docs/nix.md`; the `stable` branch step if wanted; attestations if wanted. | Docs; a workflow dry run. | Answer to question 9. |

**What ships first:** PRs 1 and 2, in one release. They change no client code and give `.deb` users working updates as soon as they add the repository. PR 3 goes in the same release if it is ready, so that release's `.deb` is the last one anyone installs by hand. PRs 4 and 5 are the client work and can follow in the next two releases. The first release with PR 4 is the first whose users hear about later ones; the first with PR 5 is the last a tarball user installs by hand.

## 13. What you need to provide

Nobody but you creates keys or secrets. For reference, the commands:

```sh
# Two minisign key pairs. The first goes to CI; the second stays offline.
minisign -G -p release.pub        -s rex-linux-release.key
minisign -G -p release-backup.pub -s rex-linux-release-backup.key

# The APT key: an offline primary, and a signing subkey for CI.
gpg --quick-generate-key 'Rex Linux APT repository <shuv@shuv.dev>' ed25519 cert never
gpg --quick-add-key <PRIMARY-FINGERPRINT> ed25519 sign 3y
gpg --armor --export <PRIMARY-FINGERPRINT> > rex-linux-archive-key.asc
gpg --armor --export-secret-subkeys '<SUBKEY-FINGERPRINT>!' > apt-signing-subkey.asc
```

| Item | Where it goes |
| --- | --- |
| `release.pub`, `release-backup.pub`, `rex-linux-archive-key.asc` | Committed, in PRs 1 and 2. |
| `rex-linux-release.key` and its password | Secrets `RELEASE_MINISIGN_KEY`, `RELEASE_MINISIGN_PASSWORD` in the `release` environment. |
| `apt-signing-subkey.asc` | Secret `APT_SIGNING_KEY` in the same environment. |
| The backup minisign key and the OpenPGP primary key | Offline, in two places. Never on GitHub. |
| Repository settings | The `release` environment (you as required reviewer; `v*` tags and `master`); Pages with source "GitHub Actions"; a ruleset limiting who creates `v*` tags; immutable releases. |
| If a custom domain is chosen | A DNS record pointing at GitHub Pages (in Cloudflare, DNS only, so GitHub issues the certificate) and the domain in the Pages settings. |

## 14. Open questions

1. **What address does the APT repository get?** It is written into every user's system for good. *Recommendation:* GitHub Pages behind a domain you own, so the host can change later; if you would rather not touch DNS, `https://shuv1337.github.io/rex-rune/apt` works today and moving later means every user edits one line.
2. **Should the `.deb` install its APT source?** *Recommendation:* yes, as a conffile with the key inside, with no `postinst` or cron job, and stated in the README and release notes.
3. **Should the `.deb` also turn on unattended upgrades for itself?** *Recommendation:* no; document the two-line file.
4. **Is the daily check on by default?** *Recommendation:* on for official builds and Nix, off for source builds, described in the README and the release notes of the first release that has it. The alternative is off until asked, which most users would never find.
5. **Do tarball installs update by themselves?** *Recommendation:* no by default: the notice appears and **Update** is one click. **Install updates automatically** is there for those who want it. Say so if you want it on by default, as Zed and Chrome do.
6. **Where do the signing keys live?** *Recommendation:* in the `release` environment with you as required reviewer (one click per release), plus the offline backup minisign key. Signing on your own machine instead would also survive a takeover of the GitHub account, and costs a manual step every release.
7. **Where does the app read the manifest?** *Recommendation:* the release asset behind `releases/latest/download/`: it appears with the release, cannot drift from it, and needs no second host. Pages would add a file to keep in step.
8. **Turn on immutable releases, and add build attestations?** *Recommendation:* yes to both. The first enforces a rule the project already follows; the second is one step.
9. **How should Nix users follow releases?** *Recommendation:* document the three forms in section 8 now and add nothing. Add a `stable` branch only if someone asks.
10. **Expiring metadata, automatic crash rollback, `by-hash`?** *Recommendation:* leave all three out for the reasons in section 11.
11. **How many versions stay in the APT repository, and does 0.4.1 go in?** *Recommendation:* five, starting from the first signed release; 0.4.1 stays on the Releases page only.
12. **Which release starts this?** *Recommendation:* PRs 1 to 3 in 0.5.0, the notice in the release after, the self-updater in the one after that.
