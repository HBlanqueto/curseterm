#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
cd "$script_dir"

fail() {
    _code=$1
    shift
    printf 'dev.sh: %s\n' "$*" >&2
    exit "$_code"
}

require_devshell() {
    [ "${CURSETERM_DEV_SHELL:-}" = "1" ] ||
        fail 2 "not inside the dev shell; run 'nix develop' first"
}

help() {
    cat <<'EOF'
Usage: ./dev.sh <command>

Commands:
  run [args]      run CurseTerm against the current WAYLAND_DISPLAY, with
                  MALAYSIA_CONFIG_DIR and MALAYSIA_DATA_DIR under $XDG_RUNTIME_DIR/curseterm
  check           run fmt, clippy -D warnings and tests
  help            show this message

All commands except help require 'nix develop'.
EOF
}

scratch_dirs() {
    _scratch=${XDG_RUNTIME_DIR:-$script_dir/target}/curseterm
    export MALAYSIA_CONFIG_DIR="${MALAYSIA_CONFIG_DIR:-$_scratch/config}"
    export MALAYSIA_DATA_DIR="${MALAYSIA_DATA_DIR:-$_scratch/data}"
    mkdir -p "$MALAYSIA_CONFIG_DIR" "$MALAYSIA_DATA_DIR"
}

run() {
    require_devshell
    scratch_dirs
    RUST_LOG=${RUST_LOG:-info} cargo run -p curseterm -- "$@"
}

check() {
    require_devshell
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
}

case "${1:-help}" in
    run)
        shift
        run "$@"
        ;;
    check) check ;;
    help | -h | --help) help ;;
    *) fail 1 "unknown command: $1 (see ./dev.sh help)" ;;
esac
