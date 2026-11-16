# 0002 — Emulation and PTY with `alacritty_terminal`

Status: **accepted** (2026-11-05).

## Context

The brief asks not to reinvent emulation and to choose between
[`alacritty_terminal`](https://docs.rs/alacritty_terminal/0.26.0) and
[`vte`](https://docs.rs/vte/0.15.0), with the PTY through `rustix`/`nix` or
`portable-pty`.

| | `alacritty_terminal` 0.26.0 (Apache-2.0) | `vte` 0.15.0 (Apache-2.0 or MIT) |
|---|---|---|
| Parser | `vte` | It is the parser |
| Grid, scrollback, reflow on resize | Yes (`grid`, `Term::resize`) | No |
| Selection (simple, word, line, block) | Yes (`selection`) | No |
| Regex search in the buffer | Yes (`term::search`) | No |
| PTY | `tty::new` with `rustix-openpty`; `OnResize` does `TIOCSWINSZ`, which sends `SIGWINCH` | No |
| I/O loop | `event_loop::EventLoop` on its own thread, with `Notifier` for writing | No |
| iTerm2's own OSCs (1337, 133) | Ignores them | Can be parsed |

## Decision

1. **`alacritty_terminal` 0.26** for emulation, scrollback,
   selection, search and the PTY. `portable-pty` is not needed.
2. Its `EventLoop` runs on its own thread. CurseTerm's `EventListener` holds a
   `barrier_kit::platform::WakeHandle` (from `Timers::wakeup`) and wakes it on
   each `Event`; the callback, on the UI thread, bumps a revision that
   repaints.
3. The grid is read under the `Term`'s `FairMutex` and drawn with the
   `barrier-kit-next` canvas (ADR 0001, BK-009).
4. For OSC 1337 and 133 (phase 4), the same byte stream goes through a
   dedicated `vte::Parser` that handles only those OSCs.

## Consequences

- iTerm2 features that depend on the screen model (marks,
  inline images) are stored next to the grid, indexed by line.
- The `alacritty_terminal` API is not stable; the `rev` is pinned in
  `Cargo.lock` and bumped by hand.

## References

- `alacritty_terminal-0.26.0/src/{event.rs,event_loop.rs,tty/unix.rs,term/mod.rs}`.
