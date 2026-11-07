# 0006 — Lo que le falta al kit se completa en barrierkit

Estado: **aceptado** (2026-11-07). Sustituye al
[ADR 0001](0001-bloques-y-parches-del-kit.md).

## Contexto

Los doce parches de `kit-parches/` y los bloques genéricos de
`crates/barrier-kit-next` estaban probados en CurseTerm. Mantenerlos fuera del
kit obligaba a compilar en dos modos y a duplicar ramas con la feature
`kit-parcheado`. El usuario pidió llevarlos a barrierkit.

## Decisión

1. Los parches 0001–0012 son commits de `main` de barrierkit, de `ba76e23` a
   `13313e6`.
2. El commit `dda9e12` de barrierkit añade `canvas`, los hooks de
   `app_menu` (`use_menu_enabled`, `use_menu_label`, `use_menu_checked`,
   `use_submenu_items` y `MenuItem::with_id`), `notifications`, `secrets`,
   `views::split_panes`, `views::floating_pane`, `views::text_view` y los
   tokens `MONO`, `SPLIT_PANES` y `FLOATING_PANE`. El commit `4e23c81` añade
   `Resolved::contrast` (BK-046).
3. Lo propio de iTerm2 se queda en CurseTerm: `tab_bar.rs`, con los estilos
   de `PSMTabBarControl`, y `tokens.rs`, con las medidas de sus vistas. Para
   pestañas genéricas el kit ya tiene `WindowTabs`.
4. CurseTerm fija barrierkit en `4e23c81` y borra `crates/barrier-kit-next`,
   `kit-parches/`, la feature `kit-parcheado` y los modos `kit-parches` y
   `kit-tal-cual` de `dev.sh`.
5. Un gap nuevo se resuelve con un commit en barrierkit, con sus reglas.
   CurseTerm sube el `rev` y lo anota en `barrierkit-gaps.md` en el mismo
   commit.

## Consecuencias

- Hay un solo modo de compilación.
- El kit tiene `SplitPanes` junto a `SplitView` y `TextView` junto a
  `TextEditor`. Falta en barrierkit la página de Tengen App Show y el test de
  render de cada vista nueva (ADR 0047 §2 de barrierkit).
- Se quitan los rodeos que el kit nuevo ya cubre: el selector de archivos del
  portal (BK-023), `MenuButton` para «Presets…» (BK-042), `use_alive`
  (BK-032) y `Resolved::contrast` (BK-046).

## Referencias

- [`barrierkit-gaps.md`](../barrierkit-gaps.md).
- ADR 0048 de barrierkit: dependencia git.
