# Publishing a binary release

`.github/workflows/release.yml` builds natively on Ubuntu 24.04 for x86-64 and ARM64. Both architectures must pass formatting, Clippy, application tests, SDK tests, packaging and install/uninstall checks before publication. The toolchains come from `mise.toml`.

1. Update the version in `Cargo.toml`, the `rex-linux` entry in `Cargo.lock`, and `nix/package.nix`; update `CHANGELOG.md` and installation examples as needed.
2. Commit and push to `master`. Run **Binary release** manually in GitHub Actions to test without publishing.
3. After checking the results, create and push an annotated tag matching the package version, for example `git tag -a v0.3.2 -m 'Rex Linux 0.3.2'` and `git push origin v0.3.2`.
4. The tag workflow rebuilds, gathers both architectures, writes `SHA256SUMS`, uploads all four packages to a draft, and publishes it only after uploads succeed. Check the release and its assets. Do not move published tags or silently replace binaries.

To package locally after a native `cargo build --locked --release --bin rex-linux`, run `python3 scripts/package-release.py` (tarball) or add `--deb` (also requires `dpkg-dev`). Outputs go into ignored `dist/`. Packages include source commit/toolchain information, desktop integration, dependency license declarations, available license texts, font licenses, and icon provenance. Icon redistribution is authorized by the project owner; it is not covered by the project's MIT license.

The compatibility baseline is the build distribution, not just CPU architecture: current official binaries require glibc 2.39+. Build on an older supported distribution if changing this baseline; do not describe newer-host builds as compatible with older glibc. Runtime-loaded graphics/windowing dependencies are listed explicitly in the Debian package in addition to linked dependencies resolved by `dpkg-shlibdeps`.

If a publish job fails after creating a draft, inspect the draft before retrying; the workflow intentionally refuses to overwrite an existing release. Recover by uploading the missing artifacts from that exact workflow run, checking their checksums, and publishing the draft. Do not rebuild and mix artifacts from unrelated commits.
