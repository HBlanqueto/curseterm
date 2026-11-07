# CurseTerm

CurseTerm es un emulador de terminal para Linux y Wayland que reproduce el
comportamiento, la interfaz y los menús de iTerm2, construido con
[BarrierKit](https://github.com/HBlanqueto/barrierkit), el toolkit de
[Malaysia](https://github.com/HBlanqueto/malaysia).

English: [README.en.md](README.en.md)

## Licencia y relación con iTerm2

CurseTerm es MIT. **Reimplementa el comportamiento de iTerm2 a partir de
leerlo y usarlo; no porta ni traduce código de iTerm2** (GPL-2.0). No usa su
nombre ni su icono. Si algún día se porta código, CurseTerm pasará a
GPL-2.0-o-posterior y constará en `NOTICE`.

La emulación y el PTY son de
[`alacritty_terminal`](https://docs.rs/alacritty_terminal) (Apache-2.0), como
dependencia ([ADR 0002](docs/decisiones/0002-emulacion-con-alacritty-terminal.md)).

## Compilar y probar

```sh
nix develop
./dev.sh check
./dev.sh run
```

Lo que le falta a BarrierKit se completa en barrierkit
([ADR 0006](docs/decisiones/0006-el-kit-se-completa-en-barrierkit.md)).

## Documentación

- [`docs/estudio-previo.md`](docs/estudio-previo.md): lo aprendido antes de empezar.
- [`docs/progreso.md`](docs/progreso.md): plan y resultado de cada fase.
- [`docs/barrierkit-gaps.md`](docs/barrierkit-gaps.md): lo que le falta a BarrierKit.
- [`docs/decisiones/`](docs/decisiones/): ADR.
