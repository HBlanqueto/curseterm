# 0005 — Own I/O loop to see iTerm2's OSCs

Status: **accepted** (2026-11-05). It supersedes point 2 of ADR 0002; point 4
(a parallel `vte::Parser`) is dropped.

## Context

Phase 4 needs the iTerm2 OSCs that `alacritty_terminal` ignores: 133
(shell marks), 1337 (current directory, marks, inline images) and 7
(current directory). Also, an inline image must take its place in
the grid in the same order as the text around it.

`alacritty_terminal::event_loop::EventLoop` reads the PTY on its own thread and passes the
bytes to the `Term` without offering any hook. A second parallel parser does not
see the order between text and image.

## Decision

1. CurseTerm reads the PTY on its own thread (`crates/curseterm/src/io.rs`): a
   duplicate of the `tty::Pty` reader, `rustix::event::poll` and a
   `vte::ansi::Processor` that feeds the `Term` just like Alacritty's loop,
   including the cutoff of synchronized updates
   (`sync_timeout`).
2. Before the `Processor`, a pure splitter (`osc.rs`) pulls OSC
   7, 133 and 1337 out of the stream and keeps the rest in order. Each OSC is handled with the `Term`
   locked, between the text chunks around it.
3. Writing goes through a thread with a channel; resizing, through
   the `Pty`'s `OnResize`, which lives in the session.

## Consequences

- `EventLoop` is no longer used; the rest of `alacritty_terminal` stays the same.
- The splitter is pure logic with tests (OSCs split across reads, BEL and
  ST, OSCs that are not iTerm2's).

## References

- `alacritty_terminal-0.26.0/src/event_loop.rs` (original loop and its handling
  of `sync_timeout`).
- iTerm2 documentation on its own sequences and inline images:
  https://iterm2.com/documentation-escape-codes.html and
  https://iterm2.com/documentation-images.html
