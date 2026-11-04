#!/usr/bin/env bash
# Install a user launcher pointing to this checkout's release build (or a supplied binary).
#   install-desktop.sh [--copy] [path/to/rex-linux]
#   install-desktop.sh --uninstall
# --copy installs the binary into $XDG_BIN_HOME (default ~/.local/bin) and points the
# launcher there, so it keeps working if the checkout moves.
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
applications="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"
desktop="$applications/dev.shuv.rex-linux.desktop"

usage() {
    echo "Usage: $0 [--copy] [path/to/rex-linux] | --uninstall" >&2
    exit 2
}

refresh_caches() {
    if command -v gtk-update-icon-cache >/dev/null && [[ -d "$icons" ]]; then
        gtk-update-icon-cache -f -t "$icons" >/dev/null 2>&1 || true
    fi
    if command -v update-desktop-database >/dev/null && [[ -d "$applications" ]]; then
        update-desktop-database "$applications"
    fi
}

# Quote a value as one argument of a desktop entry's Exec key.
desktop_exec_quote() {
    local value="$1"
    if [[ "$value" == *$'\n'* ]]; then
        echo "Desktop launcher paths cannot contain newlines" >&2
        return 1
    fi
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//\`/\\\`}"
    value="${value//\$/\\\$}"
    printf '"%s"' "$value"
}

copy=false
case "${1:-}" in
    --uninstall)
        [[ $# -eq 1 ]] || usage
        rm -f -- "$bin_dir/rex-linux" "$desktop"
        for size in 32 128 256; do
            rm -f -- "$icons/${size}x${size}/apps/dev.shuv.rex-linux.png"
        done
        refresh_caches
        echo "Removed Rex Linux's launcher, icons and $bin_dir/rex-linux"
        exit 0
        ;;
    --copy)
        copy=true
        shift
        ;;
    -*) usage ;;
esac
[[ $# -le 1 ]] || usage
source_binary="$(realpath -m -- "${1:-$root/target/release/rex-linux}")"

if [[ ! -x "$source_binary" ]]; then
    echo "Build Rex Linux first: cargo build --locked --release --bin rex-linux" >&2
    exit 1
fi

binary="$source_binary"
if $copy; then
    binary="$bin_dir/rex-linux"
    if [[ ! "$source_binary" -ef "$binary" ]]; then
        install -Dm755 "$source_binary" "$binary"
    fi
fi
exec_value="$(desktop_exec_quote "$binary")"

for size in 32 128 256; do
    install -Dm644 "$root/resources/icons/${size}x${size}/apps/dev.shuv.rex-linux.png" \
        "$icons/${size}x${size}/apps/dev.shuv.rex-linux.png"
done

desktop-file-install --dir="$applications" \
    --set-key=Exec --set-value="$exec_value" \
    --set-key=TryExec --set-value="$binary" \
    "$root/resources/dev.shuv.rex-linux.desktop"
desktop-file-validate "$desktop"
refresh_caches
echo "Installed $desktop"
