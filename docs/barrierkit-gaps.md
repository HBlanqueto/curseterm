# Gaps between CurseTerm and BarrierKit

**Goal:** list what CurseTerm still works around because BarrierKit lacks it,
so each workaround is removed when the kit gains the feature.
**Reader:** whoever bumps the `rev` of barrierkit in `Cargo.toml`.
**Input:** the `BK-nnn` comments in `crates/curseterm/src`, and BarrierKit at
`1bf778c` (CurseTerm still pins `36e5958`).
**Output:** one row per workaround, with its state at `1bf778c`.

The 48 gaps found while building CurseTerm (ADR 0006) are in the last copy of
the old document: `git show bc27a76:docs/barrierkit-gaps.md`.

## Steps

1. After bumping the `rev`, re-check every row against the new kit.
2. If the kit now has the feature, delete the workaround and its row in the
   same commit.
3. If a new workaround needs a `BK-nnn` comment, add its row here with the
   next free number (BK-049).

## Gaps

States: **open** (no API in the kit), **fixed** (the kit has it; remove the
workaround once the `rev` is past the commit that fixed it) and **not
re-checked**.

| ID | Gap | Workaround in CurseTerm | State at `1bf778c` |
|---|---|---|---|
| BK-012 | The saved window size is one per `app_id` and cannot be overridden | `windows::forget_saved_size` | open: `platform/window.rs` still saves one size per window state |
| BK-013 | The compositor's close request cannot be intercepted | none | open: no close hook in `platform/` |
| BK-016 | Shortcuts with Shift compare the produced character | `⇧]` is declared as `}` (`menu.rs`) | not re-checked |
| BK-017 | Menu handlers run outside Freya's context | handles taken in `render` (`terminal.rs`) | not re-checked |
| BK-019 | An app cannot ask for an xdg-activation token | none; «Window» lists no windows | open: `platform/activation.rs` serves only the kit |
| BK-020 | Labels of panes, rows and pop-ups are `&'static str` | `preferences::intern` leaks each name once | open: `PopUpButton::new` and `item` take `&'static str` |
| BK-021 | `RadioGroup` is always horizontal | one single-option group per row (`preferences.rs`) | fixed in `afe570e`, after the pin: `RadioGroup` takes `Axis::Vertical` |
| BK-024 | No bell or trash symbol | `SpeakerWave` for the bell; a drawn trash (`note_view.rs`) | bell fixed in `7a3464b`, after the pin (`Symbol::Bell`); trash open |
| BK-025 | No global shortcuts or way to show and hide the window | `curseterm --hotkey` (`windows.rs`) | open: no GlobalShortcuts portal |
| BK-027 | A view with global keys cannot tell whether a text field or an `Alert` keeps the key | the sheet closes one loop later (`terminal.rs`) | not re-checked |
| BK-030 | Mouse events carry no modifiers | modifiers tracked from the keyboard (`terminal.rs`) | open: Freya 0.5.0 `MouseEventData` has only locations and button |
| BK-032 | `Timers` callbacks outlive their window | `Handles::after` checks `alive` | not re-checked; the kit has `use_alive` |
| BK-037 | `Alert` has two buttons and cannot be silenced | own three-button sheet (`terminal.rs`) | partly fixed: `Alert::suppression` exists at the pin; still two buttons |
| BK-040 | `PopUpButton` neither scrolls nor filters | groups and a filter field (`key_bindings.rs`) | open: no such method in `popup_button.rs` |

Gaps of the old document with no `BK-nnn` comment in the code (BK-014, 015,
028, 038, 039, 043, 047) were not re-checked.
