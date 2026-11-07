# 0006 — What the kit lacks is completed in barrierkit

Status: **accepted** (2026-11-07). Supersedes
[ADR 0001](0001-bloques-y-parches-del-kit.en.md).

## Context

The twelve patches in `kit-parches/` and the generic blocks in
`crates/barrier-kit-next` were tested in CurseTerm. Keeping them outside the
kit meant building in two modes and duplicating branches behind the
`kit-parcheado` feature. The user asked to move them into barrierkit.

## Decision

1. Patches 0001–0012 are commits on barrierkit's `main`, from `ba76e23` to
   `13313e6`.
2. Barrierkit commit `dda9e12` adds `canvas`, the `app_menu` hooks
   (`use_menu_enabled`, `use_menu_label`, `use_menu_checked`,
   `use_submenu_items` and `MenuItem::with_id`), `notifications`, `secrets`,
   `views::split_panes`, `views::floating_pane`, `views::text_view` and the
   `MONO`, `SPLIT_PANES` and `FLOATING_PANE` tokens. Commit `4e23c81` adds
   `Resolved::contrast` (BK-046).
3. What is iTerm2's own stays in CurseTerm: `tab_bar.rs`, with the
   `PSMTabBarControl` styles, and `tokens.rs`, with the sizes of its views.
   For generic tabs the kit already has `WindowTabs`.
4. CurseTerm pins barrierkit at `4e23c81` and deletes `crates/barrier-kit-next`,
   `kit-parches/`, the `kit-parcheado` feature and the `kit-parches` and
   `kit-tal-cual` modes of `dev.sh`.
5. A new gap is solved with a commit in barrierkit, under its rules.
   CurseTerm bumps the `rev` and records it in `barrierkit-gaps.en.md` in the
   same commit.

## Consequences

- There is a single build mode.
- The kit has `SplitPanes` next to `SplitView` and `TextView` next to
  `TextEditor`. Barrierkit still lacks the Tengen App Show page and the
  render test of each new view (barrierkit ADR 0047 §2).
- The workarounds the new kit covers are removed: the portal file chooser
  (BK-023), `MenuButton` for «Presets…» (BK-042), `use_alive` (BK-032) and
  `Resolved::contrast` (BK-046).

## References

- [`barrierkit-gaps.en.md`](../barrierkit-gaps.en.md).
- Barrierkit ADR 0048: git dependency.
