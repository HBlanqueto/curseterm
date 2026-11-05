# AGENTS.md — CurseTerm

CurseTerm sigue las convenciones de
[`barrierkit/AGENTS.md`](https://github.com/HBlanqueto/barrierkit/blob/main/AGENTS.md):
rol, «Documentación (español e inglés)», «Código mínimo» (reglas 1–10) y
formato de commits. Aquí solo lo propio.

## Reglas del proyecto

1. **No se modifican BarrierKit ni Malaysia.** Lo que le falta al kit se hace
   como bloque en `crates/barrier-kit-next` o como parche en `kit-parches/`
   (ADR 0001) y se anota en `docs/barrierkit-gaps.md` en el mismo commit.
2. **`barrier-kit-next` no depende de `curseterm`** y usa la ruta de módulo que
   tendría en `barrier-kit`. Sus tokens viven solo en `src/tokens.rs`.
3. **Diseño con tokens.** La interfaz usa `Palette`, `vow_design` y, para lo
   que Tengen aún no tiene, `barrier_kit_next::tokens`. Los colores
   del terminal son datos de esquema (`crates/curseterm/schemes/*.toml`).
4. **Texto visible por Fluent:** `crates/curseterm/i18n/{en,es}/curseterm.ftl` y `tr!`.
5. **Atajos con Super** (ADR 0003).
6. **Docs en español (`nombre.md`) e inglés (`nombre.en.md`)**, que cambian
   en el mismo commit; código, comentarios y rustdoc solo en inglés.
7. **Nada se da por probado sin ejecutarlo**; lo no probado se dice en
   `docs/progreso.md`.

## Compilar

`nix develop`, luego `./dev.sh check` (fmt, clippy `-D warnings`, tests) en los
dos modos: `./dev.sh kit-tal-cual` y `./dev.sh kit-parches`. `Cargo.lock` se
sube generado sin el `[patch]`.

## Ramas

Todo va directo a `main`. `git pull --ff-only` antes de cada push; nunca `--force`.
