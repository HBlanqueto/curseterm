#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
cd "$script_dir"

kit_url=https://github.com/HBlanqueto/barrierkit
patched_dir=$script_dir/target/barrierkit-parcheado

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
  check           run fmt, clippy -D warnings and tests in the current kit mode
  kit-parches     build against BarrierKit with kit-parches/*.patch applied (ADR 0001)
  kit-tal-cual    build against BarrierKit as pinned in Cargo.toml
  help            show this message

All commands except help require 'nix develop'.
EOF
}

kit_rev() {
    sed -n 's/^barrier-kit = .*rev = "\([0-9a-f]*\)".*/\1/p' Cargo.toml
}

features() {
    if [ -f .cargo/config.toml ]; then
        printf '%s' '--features=curseterm/kit-parcheado'
    fi
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
    # shellcheck disable=SC2046
    RUST_LOG=${RUST_LOG:-info} cargo run -p curseterm $(features) -- "$@"
}

check() {
    require_devshell
    cargo fmt --all --check
    # shellcheck disable=SC2046
    cargo clippy --workspace --all-targets $(features) -- -D warnings
    # shellcheck disable=SC2046
    cargo test --workspace $(features)
}

kit_parches() {
    require_devshell
    _rev=$(kit_rev)
    [ -n "$_rev" ] || fail 1 "no barrier-kit rev in Cargo.toml"
    rm -rf "$patched_dir"
    git init -q "$patched_dir"
    git -C "$patched_dir" fetch -q --depth 1 "$kit_url" "$_rev"
    git -C "$patched_dir" checkout -q FETCH_HEAD
    for _patch in kit-parches/*.patch; do
        [ -e "$_patch" ] || continue
        git -C "$patched_dir" apply "$script_dir/$_patch" ||
            fail 1 "$_patch does not apply on $_rev"
        printf 'dev.sh: applied %s\n' "$_patch"
    done
    mkdir -p .cargo
    cat >.cargo/config.toml <<EOF
[patch."$kit_url"]
barrier-kit = { path = "$patched_dir/crates/barrier-kit" }
vow-design = { path = "$patched_dir/crates/vow-design" }
vow-appearance = { path = "$patched_dir/crates/vow-appearance" }
EOF
    printf 'dev.sh: building against the patched kit at %s\n' "$_rev"
}

kit_tal_cual() {
    rm -f .cargo/config.toml
    git checkout -q -- Cargo.lock 2>/dev/null || true
    printf 'dev.sh: building against the pinned kit\n'
}

case "${1:-help}" in
    run)
        shift
        run "$@"
        ;;
    check) check ;;
    kit-parches) kit_parches ;;
    kit-tal-cual) kit_tal_cual ;;
    help | -h | --help) help ;;
    *) fail 1 "unknown command: $1 (see ./dev.sh help)" ;;
esac
