# 0003 — ⌘ de iTerm2 es Super en CurseTerm

Estado: **aceptado** (2026-11-05).

## Contexto

iTerm2 usa ⌘ para casi todo. En Linux hay dos esquemas habituales: Super
(como macOS) o Ctrl+Mayús (GNOME Terminal, Konsole).

- La shell de Malaysia ya traduce ⌘ a Super en sus menús (Super+C, Super+V,
  Super+Q…) y su compositor reserva Super+Q, Super+H, Super+M y Super+W
  como macOS (`malaysia/docs/configuracion/opciones.md`).
- Un terminal no envía Super a los programas. Ctrl+Mayús sí llega a ellos en
  los modos de teclado modernos (CSI u, kitty), así que un atajo Ctrl+Mayús le
  quita una combinación a `vim` o a `emacs`.
- GNOME reserva varias combinaciones Super+letra: entre ellas Super+V (lista de
  notificaciones), Super+A, Super+L y Super+H.

## Decisión

1. Todo ⌘ de iTerm2 es **Super**; ⌥ es Alt, ⌃ es Ctrl y ⇧ es Mayús. Ejemplos:
   ⌘T → Super+T, ⇧⌘D → Super+Mayús+D, ⌥⌘← → Super+Alt+←.
2. El menú global muestra ese atajo y la app lo atiende por `Action`.
3. Única excepción, fuera del menú: **Ctrl+Mayús+C** y **Ctrl+Mayús+V** también
   copian y pegan, para poder hacerlo en GNOME, donde Super+V no llega. Las
   atiende el codificador de teclado del terminal y no se publican en el menú.
4. El codificador de teclado nunca envía al PTY una tecla con Super.

## Consecuencias

- En Malaysia, Super+W cierra la ventana entera aunque tenga varias pestañas,
  porque el compositor la atiende antes que la app. Se anota en `progreso.md`
  como diferencia con iTerm2.
- En GNOME, los atajos Super que reserva el escritorio no llegan; los menús
  siguen funcionando con el ratón.

## Referencias

- [`estudio-previo.md`](../estudio-previo.md), §2 «Atajos fijos de Malaysia» y
  §6.2.
