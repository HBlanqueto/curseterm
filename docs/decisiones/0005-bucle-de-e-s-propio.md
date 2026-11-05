# 0005 — Bucle de E/S propio para ver los OSC de iTerm2

Estado: **aceptado** (2026-11-05). Sustituye el punto 2 del ADR 0002; el
punto 4 (un `vte::Parser` en paralelo) se descarta.

## Contexto

La fase 4 necesita los OSC de iTerm2 que `alacritty_terminal` ignora: 133
(marcas de la shell), 1337 (directorio actual, marcas, imágenes en línea) y 7
(directorio actual). Además, una imagen en línea tiene que ocupar su sitio en
la rejilla en el mismo orden que el texto que la rodea.

`alacritty_terminal::event_loop::EventLoop` lee el PTY en su hilo y pasa los
bytes al `Term` sin ofrecer ningún gancho. Un segundo parser en paralelo no
ve el orden entre texto e imagen.

## Decisión

1. CurseTerm lee el PTY en un hilo propio (`crates/curseterm/src/io.rs`): un
   duplicado del lector de `tty::Pty`, `rustix::event::poll` y un
   `vte::ansi::Processor` que alimenta el `Term` igual que el bucle de
   Alacritty, incluido el corte de las actualizaciones sincronizadas
   (`sync_timeout`).
2. Antes del `Processor`, un separador puro (`osc.rs`) saca del flujo los OSC
   7, 133 y 1337 y deja el resto en orden. Cada OSC se atiende con el `Term`
   bloqueado, entre los trozos de texto que lo rodean.
3. La escritura va por un hilo con canal; el redimensionado, por
   `OnResize` del `Pty`, que vive en la sesión.

## Consecuencias

- Se deja de usar `EventLoop`; lo demás de `alacritty_terminal` sigue igual.
- El separador es lógica pura con tests (OSC partidos entre lecturas, BEL y
  ST, OSC que no son de iTerm2).

## Referencias

- `alacritty_terminal-0.26.0/src/event_loop.rs` (bucle original y su manejo
  de `sync_timeout`).
- Documentación de iTerm2 sobre secuencias propias e imágenes en línea:
  https://iterm2.com/documentation-escape-codes.html y
  https://iterm2.com/documentation-images.html
