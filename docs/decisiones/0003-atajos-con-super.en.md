# 0003 — iTerm2's ⌘ is Super in CurseTerm

Status: **accepted** (2026-11-05).

## Context

iTerm2 uses ⌘ for almost everything. Linux has two common schemes: Super
(like macOS) or Ctrl+Shift (GNOME Terminal, Konsole).

- The Malaysia shell already translates ⌘ to Super in its menus (Super+C, Super+V,
  Super+Q…) and its compositor reserves Super+Q, Super+H, Super+M and Super+W
  like macOS (`malaysia/docs/configuracion/opciones.md`).
- A terminal does not send Super to programs. Ctrl+Shift does reach them in
  modern keyboard modes (CSI u, kitty), so a Ctrl+Shift shortcut
  takes a combination away from `vim` or `emacs`.
- GNOME reserves several Super+letter combinations: among them Super+V (notification
  list), Super+A, Super+L and Super+H.

## Decision

1. Every iTerm2 ⌘ is **Super**; ⌥ is Alt, ⌃ is Ctrl and ⇧ is Shift. Examples:
   ⌘T → Super+T, ⇧⌘D → Super+Shift+D, ⌥⌘← → Super+Alt+←.
2. The global menu shows that shortcut and the app handles it through `Action`.
3. The only exception, outside the menu: **Ctrl+Shift+C** and **Ctrl+Shift+V** also
   copy and paste, so that it works in GNOME, where Super+V does not arrive. The
   terminal's keyboard encoder handles them and the menu does not publish them.
4. The keyboard encoder never sends a key with Super to the PTY.

## Consequences

- In Malaysia, Super+W closes the whole window even if it has several tabs,
  because the compositor handles it before the app. `progreso.en.md` records it
  as a difference from iTerm2.
- In GNOME, the Super shortcuts the desktop reserves do not arrive; the menus
  still work with the mouse.

## References

- [`estudio-previo.en.md`](../estudio-previo.en.md), §2 "Malaysia's fixed shortcuts" and
  §6.2.
