#!/usr/bin/env python3
"""Sign release metadata, then verify against the committed minisign public keys.

Secrets come only from RELEASE_MINISIGN_KEY and RELEASE_MINISIGN_PASSWORD.
--verify-only supports offline/manual signing without changing the upload path.
"""

import argparse
import os
from pathlib import Path
import re
import subprocess
import tempfile


FILES = ("latest.json", "SHA256SUMS")


def verify(dist, keys):
    for name in FILES:
        for key in keys:
            result = subprocess.run(
                ["minisign", "-Vm", str(dist / name), "-p", str(key), "-q"],
                capture_output=True,
            )
            if result.returncode == 0:
                break
        else:
            raise ValueError(f"{name}: signature did not verify against either repository public key")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, default=Path("dist"))
    parser.add_argument("--keys-dir", type=Path, default=Path("packaging/keys"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--allow-unsigned", action="store_true", help="manual test runs only: skip if either secret is absent")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    # Do not pass these environment variables to minisign or any other child.
    secret = os.environ.pop("RELEASE_MINISIGN_KEY", "")
    password = os.environ.pop("RELEASE_MINISIGN_PASSWORD", "")
    try:
        if not re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", args.tag):
            raise ValueError("tag must be a stable vX.Y.Z release")
        if not args.verify_only and (not secret or not password):
            message = "RELEASE_MINISIGN_KEY and RELEASE_MINISIGN_PASSWORD must both be set in the release environment"
            if args.allow_unsigned:
                print(f"Skipping signing for manual test run: {message}")
                return
            raise ValueError(f"refusing unsigned release: {message}")
        keys = [args.keys_dir / name for name in ("release.pub", "release-backup.pub")]
        for key in keys:
            if not key.is_file():
                raise ValueError(f"missing public key {key}; complete docs/releases.md setup before releasing")
        if not args.verify_only:
            # Private directory and file disappear on success or any exception.
            with tempfile.TemporaryDirectory(prefix="rex-release-sign-") as temp:
                key = Path(temp) / "release.key"
                key.touch(mode=0o600)
                key.write_text(secret)
                for name in FILES:
                    result = subprocess.run(
                        ["minisign", "-S", "-s", str(key), "-m", str(args.dist / name),
                         "-t", f"rex-linux {args.tag} {name}", "-q"],
                        input=password + "\n", text=True, capture_output=True,
                    )
                    if result.returncode != 0:
                        raise ValueError(f"minisign failed to sign {name}; check the release key and password")
        verify(args.dist, keys)
        print("Verified latest.json and SHA256SUMS against repository public keys")
    except (OSError, ValueError) as error:
        parser.exit(1, f"release signing: {error}\n")


if __name__ == "__main__":
    main()
