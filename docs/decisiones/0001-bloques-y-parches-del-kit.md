# 0001 — Lo que le falta al kit se construye como bloques y parches trasladables

Estado: **aceptado** (2026-11-05).

## Contexto

CurseTerm mide BarrierKit tal como está y no lo modifica. El estudio previo
encontró gaps de dos clases ([`barrierkit-gaps.md`](../barrierkit-gaps.md)):

- **Componibles:** se pueden construir sobre la API pública del kit y de Freya
  (pestañas de documento, paneles divididos, lienzo, token monoespaciado, menús
  dinámicos).
- **De plataforma:** necesitan el runner, que es `pub(crate)` (varias ventanas,
  título en vivo, atajos consumidos, IME, selección primaria).

El usuario quiere que lo que falte se pueda pasar al kit sin reescribirlo.

## Decisión

### 1. Bloques componibles: `crates/barrier-kit-next`

- Un crate del workspace de CurseTerm con la **misma ruta de módulo** que
  tendría en `barrier-kit` (`views::tab_bar`, `views::split_panes`, `canvas`…).
  Pasarlo al kit es mover el archivo.
- Solo depende de `barrier-kit`, `vow-design`, `freya-*` y `torin`. Nunca de
  `curseterm`.
- Cumple las reglas de todo componente del kit (ADR 0047 §2): valores de
  Tengen, variantes, controlado, `use_interaction`, etiqueta accesible en `new`.
- Los tokens nuevos viven en `barrier-kit-next/src/tokens.rs`, con el tipo de
  `vow_design` y el nombre que tendrían en `tengen/tokens.json` o
  `components.json`. Es el único sitio de CurseTerm con valores de diseño.
- Los tests siguen la regla 8 del kit. La página en Tengen App Show y el test
  de render en claro y oscuro se escriben al trasladarlo.

### 2. Gaps de plataforma: `kit-parches/` probados en local

1. Cada gap de plataforma tiene un parche `kit-parches/NNNN-<tema>.patch`
   contra el `rev` fijado en `Cargo.toml`, escrito con las reglas de
   `barrierkit/AGENTS.md`.
2. `./dev.sh kit-parches` clona barrierkit en ese `rev` dentro de
   `target/barrierkit-parcheado`, aplica los parches y escribe un
   `.cargo/config.toml` **no versionado** con
   `[patch."https://github.com/HBlanqueto/barrierkit"]`. `./dev.sh kit-tal-cual`
   lo borra. Es el mismo mecanismo que `dev.sh kit-local` de malaysia.
3. CurseTerm compila y funciona en los dos modos. El código que usa una API
   parcheada va tras la feature `kit-parcheado`; sin ella usa el workaround.
4. `barrierkit-gaps.md` dice, para cada gap, si se resolvió con un bloque o un
   parche y en qué modo se probó.

### 3. Numeración

Los ADR de CurseTerm se numeran aparte. Un bloque o parche que pase al kit
lleva su propio ADR con el siguiente número libre de la serie común de
barrierkit y malaysia (ADR 0048 §4).

## Consecuencias

- Las pruebas en modo `kit-parcheado` no miden el kit publicado; cada gap lo
  dice.
- La feature `kit-parcheado` duplica algunas ramas de código hasta que los
  parches lleguen al kit; entonces se borran el workaround y la feature.
- `Cargo.lock` se sube siempre generado sin el `[patch]`.

## Referencias

- [`estudio-previo.md`](../estudio-previo.md), §3.
- ADR 0047 de barrierkit: capas y reglas de componente.
- ADR 0048 de barrierkit: dependencia git y `kit-local`.
