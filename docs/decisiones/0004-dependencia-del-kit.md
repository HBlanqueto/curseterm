# 0004 — BarrierKit como input del flake y dependencia git

Estado: **aceptado** (2026-11-05).

## Contexto

El encargo pide consumir BarrierKit como input del flake, o como dependencia
git en `Cargo.toml` si el flake no lo permite. El flake de barrierkit exporta
`packages.skia-binaries` y `devShells.default`, pero no las crates como
librería de Nix.

## Decisión

1. `flake.nix` declara `barrierkit` como input con el mismo `rev` que
   `Cargo.toml` y `nixpkgs.follows = "barrierkit/nixpkgs"`.
2. El dev shell de CurseTerm es el de barrierkit (`devShells.default`) más
   `CURSETERM_DEV_SHELL=1`: hereda Skia precompilado, Vulkan, fuentes y la
   ruta de librerías sin copiarlos.
3. Las crates (`barrier-kit`, `vow-design`, `vow-appearance`) son dependencias
   git en `[workspace.dependencies]` con ese mismo `rev`, como en malaysia
   (ADR 0048 §2). Cargo no puede tomar crates de un input del flake.
4. Subir el kit es un commit que cambia el `rev` en los dos sitios:
   `chore(deps): barrierkit <hash>`.

## Consecuencias

- Hay una sola guardia de versión de Skia, la de barrierkit.
- El input usa `git+https://` y no `github:`, porque así se descarga por git.

## Referencias

- ADR 0048 de barrierkit, §2.
