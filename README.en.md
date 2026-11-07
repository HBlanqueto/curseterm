# CurseTerm

Español: [README.md](README.md)

CurseTerm is a terminal emulator for Linux and Wayland that reproduces the
behavior, interface and menus of iTerm2. It is built with
[BarrierKit](https://github.com/HBlanqueto/barrierkit), the toolkit of
[Malaysia](https://github.com/HBlanqueto/malaysia).

## License and relationship with iTerm2

CurseTerm is MIT. **It reimplements iTerm2's behavior by reading and using
it; it does not port or translate iTerm2 code** (GPL-2.0). It does not use its
name or its icon. If code is ever ported, CurseTerm moves to
GPL-2.0-or-later and records it in `NOTICE`.

Emulation and the PTY come from
[`alacritty_terminal`](https://docs.rs/alacritty_terminal) (Apache-2.0), as a
dependency ([ADR 0002](docs/decisiones/0002-emulacion-con-alacritty-terminal.en.md)).

## Build and test

```sh
nix develop
./dev.sh check
./dev.sh run
```

What BarrierKit lacks is completed in barrierkit
([ADR 0006](docs/decisiones/0006-el-kit-se-completa-en-barrierkit.en.md)).

## Install

`nix build .#curseterm` leaves the binary and its `.desktop` in `result/`.
Malaysia installs it with its NixOS module and opens it in `./dev.sh demo` and
`./dev.sh lab`. The grid uses Geist Mono, which Malaysia's module installs with
`geist-font`.

## Documentation

- [`docs/estudio-previo.en.md`](docs/estudio-previo.en.md): what was learned before starting.
- [`docs/progreso.en.md`](docs/progreso.en.md): plan and outcome of each phase.
- [`docs/barrierkit-gaps.en.md`](docs/barrierkit-gaps.en.md): what BarrierKit lacks.
- [`docs/decisiones/`](docs/decisiones/): ADRs.
