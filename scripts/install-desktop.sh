#!/usr/bin/env bash
# Install a user launcher pointing to this checkout's release build (or a supplied binary).
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$(realpath -- "${1:-$root/target/release/rex-linux}")"
applications="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"

if [[ ! -x "$binary" ]]; then
    echo "Build Rex Linux first: cargo build --locked --release --bin rex-linux" >&2
    exit 1
fi

for size in 32 128 256; do
    install -Dm644 "$root/resources/icons/${size}x${size}/apps/dev.shuv.rex-linux.png" \
        "$icons/${size}x${size}/apps/dev.shuv.rex-linux.png"
done
if command -v gtk-update-icon-cache >/dev/null; then
    gtk-update-icon-cache -f -t "$icons" >/dev/null 2>&1 || true
fi

desktop-file-install --dir="$applications" \
    --set-key=Exec --set-value="\"$binary\"" \
    --set-key=TryExec --set-value="$binary" \
    "$root/resources/dev.shuv.rex-linux.desktop"
desktop-file-validate "$applications/dev.shuv.rex-linux.desktop"
update-desktop-database "$applications"
echo "Installed $applications/dev.shuv.rex-linux.desktop"
