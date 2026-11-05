# 0001 — What the kit lacks is built as movable blocks and patches

Status: **accepted** (2026-11-05).

## Context

CurseTerm measures BarrierKit as it is and does not modify it. The prior study
found gaps of two kinds ([`barrierkit-gaps.en.md`](../barrierkit-gaps.en.md)):

- **Composable:** they can be built on the public API of the kit and of Freya
  (document tabs, split panes, canvas, monospace token, dynamic
  menus).
- **Platform:** they need the runner, which is `pub(crate)` (multiple windows,
  live title, consumed shortcuts, IME, primary selection).

The user wants whatever is missing to move into the kit without a rewrite.

## Decision

### 1. Composable blocks: `crates/barrier-kit-next`

- A crate in the CurseTerm workspace with the **same module path** it
  would have in `barrier-kit` (`views::tab_bar`, `views::split_panes`, `canvas`…).
  Moving it to the kit means moving the file.
- It depends only on `barrier-kit`, `vow-design`, `freya-*` and `torin`. Never on
  `curseterm`.
- It follows the rules for every kit component (ADR 0047 §2): Tengen
  values, variants, controlled, `use_interaction`, accessible label in `new`.
- New tokens live in `barrier-kit-next/src/tokens.rs`, with the `vow_design`
  type and the name they would have in `tengen/tokens.json` or
  `components.json`. It is the only place in CurseTerm with design values.
- Tests follow rule 8 of the kit. The Tengen App Show page and the
  light and dark render test are written when the block moves.

### 2. Platform gaps: `kit-parches/` tested locally

1. Each platform gap has a patch `kit-parches/NNNN-<tema>.patch`
   against the `rev` pinned in `Cargo.toml`, written with the rules of
   `barrierkit/AGENTS.md`.
2. `./dev.sh kit-parches` clones barrierkit at that `rev` into
   `target/barrierkit-parcheado`, applies the patches and writes an
   **unversioned** `.cargo/config.toml` with
   `[patch."https://github.com/HBlanqueto/barrierkit"]`. `./dev.sh kit-tal-cual`
   deletes it. It is the same mechanism as malaysia's `dev.sh kit-local`.
3. CurseTerm builds and works in both modes. Code that uses a patched
   API sits behind the `kit-parcheado` feature; without it, it uses the workaround.
4. `barrierkit-gaps.en.md` states, for each gap, whether a block or a
   patch solved it and in which mode it was tested.

### 3. Numbering

CurseTerm ADRs have their own numbering. A block or patch that moves to the kit
gets its own ADR with the next free number of the series shared by
barrierkit and malaysia (ADR 0048 §4).

## Consequences

- Tests in `kit-parcheado` mode do not measure the published kit; each gap
  says so.
- The `kit-parcheado` feature duplicates some code branches until the
  patches reach the kit; then the workaround and the feature are deleted.
- `Cargo.lock` is always committed as generated without the `[patch]`.

## References

- [`estudio-previo.en.md`](../estudio-previo.en.md), §3.
- barrierkit ADR 0047: layers and component rules.
- barrierkit ADR 0048: git dependency and `kit-local`.
