# 0004 — BarrierKit as a flake input and git dependency

Status: **accepted** (2026-11-05).

## Context

The brief asks to consume BarrierKit as a flake input, or as a git
dependency in `Cargo.toml` if the flake does not allow it. The barrierkit flake exports
`packages.skia-binaries` and `devShells.default`, but not the crates as a
Nix library.

## Decision

1. `flake.nix` declares `barrierkit` as an input with the same `rev` as
   `Cargo.toml` and `nixpkgs.follows = "barrierkit/nixpkgs"`.
2. CurseTerm's dev shell is barrierkit's (`devShells.default`) plus
   `CURSETERM_DEV_SHELL=1`: it inherits prebuilt Skia, Vulkan, fonts and the
   library path without copying them.
3. The crates (`barrier-kit`, `vow-design`, `vow-appearance`) are git
   dependencies in `[workspace.dependencies]` with that same `rev`, as in malaysia
   (ADR 0048 §2). Cargo cannot take crates from a flake input.
4. Bumping the kit is one commit that changes the `rev` in both places:
   `chore(deps): barrierkit <hash>`.

## Consequences

- There is a single Skia version guard, barrierkit's.
- The input uses `git+https://` and not `github:`, because that way it is fetched through git.

## References

- barrierkit ADR 0048, §2.
