# 0002 — Emulación y PTY con `alacritty_terminal`

Estado: **aceptado** (2026-11-05).

## Contexto

El encargo pide no reinventar la emulación y elegir entre
[`alacritty_terminal`](https://docs.rs/alacritty_terminal/0.26.0) y
[`vte`](https://docs.rs/vte/0.15.0), con el PTY por `rustix`/`nix` o
`portable-pty`.

| | `alacritty_terminal` 0.26.0 (Apache-2.0) | `vte` 0.15.0 (Apache-2.0 o MIT) |
|---|---|---|
| Parser | `vte` | Es el parser |
| Rejilla, scrollback, reflow al redimensionar | Sí (`grid`, `Term::resize`) | No |
| Selección (simple, palabra, línea, bloque) | Sí (`selection`) | No |
| Búsqueda regex en el buffer | Sí (`term::search`) | No |
| PTY | `tty::new` con `rustix-openpty`; `OnResize` hace `TIOCSWINSZ`, que manda `SIGWINCH` | No |
| Bucle de E/S | `event_loop::EventLoop` en su hilo, con `Notifier` para escribir | No |
| OSC propios de iTerm2 (1337, 133) | Los ignora | Se pueden parsear |

## Decisión

1. **`alacritty_terminal` 0.26** para la emulación, el scrollback, la
   selección, la búsqueda y el PTY. `portable-pty` no hace falta.
2. Su `EventLoop` corre en su hilo. El `EventListener` de CurseTerm guarda un
   `barrier_kit::platform::WakeHandle` (de `Timers::wakeup`) y lo despierta en
   cada `Event`; el callback, en el hilo de interfaz, sube una revisión que
   repinta.
3. La rejilla se lee bajo el `FairMutex` del `Term` y se dibuja con el lienzo
   de `barrier-kit-next` (ADR 0001, BK-009).
4. Para OSC 1337 y 133 (fase 4) se pasa el mismo flujo de bytes por un
   `vte::Parser` propio que solo atiende esos OSC.

## Consecuencias

- Las funciones de iTerm2 que dependen del modelo de pantalla (marcas,
  imágenes en línea) se guardan junto a la rejilla, indexadas por línea.
- La API de `alacritty_terminal` no es estable; el `rev` se fija en
  `Cargo.lock` y se sube a mano.

## Referencias

- `alacritty_terminal-0.26.0/src/{event.rs,event_loop.rs,tty/unix.rs,term/mod.rs}`.
