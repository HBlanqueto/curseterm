# AGENTS.md — CurseTerm

CurseTerm sigue las convenciones de
[`barrierkit/AGENTS.md`](https://github.com/HBlanqueto/barrierkit/blob/main/AGENTS.md):
rol, «Documentación (solo inglés)», «Código mínimo» (reglas 1–10) y
formato de commits. Aquí solo lo propio.

## Reglas del proyecto

1. **No se modifica Malaysia.** Lo que le falta a BarrierKit se completa en
   barrierkit con sus reglas; CurseTerm sube el `rev` y lo anota en
   `docs/barrierkit-gaps.md` en el mismo commit (ADR 0006).
2. **Diseño con tokens.** La interfaz usa `Palette`, `vow_design` y, para las
   medidas propias de CurseTerm, `crates/curseterm/src/tokens.rs`. Los colores
   del terminal son datos de esquema (`crates/curseterm/schemes/*.toml`).
3. **Texto visible por Fluent:** `crates/curseterm/i18n/{en,es}/curseterm.ftl` y `tr!`.
4. **Atajos con Super** (ADR 0003).
5. **Docs solo en inglés (`nombre.md`)**, sin copias en español; código,
   comentarios y rustdoc también en inglés.
6. **Nada se da por probado sin ejecutarlo**; lo no probado se dice en
   `docs/untested.md`.

## Compilar

`nix develop`, luego `./dev.sh check` (fmt, clippy `-D warnings`, tests).

## Ramas

Todo va directo a `main`. `git pull --ff-only` antes de cada push; nunca `--force`.
