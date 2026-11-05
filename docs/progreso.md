# Progreso

**Objetivo:** registrar el plan de cada fase antes de empezarla y, al
cerrarla, qué funciona, qué no y qué difiere de iTerm2.
**Lector:** quien sigue el proyecto o revisa una fase.

Una fase se cierra cuando compila, pasa `./dev.sh check` (fmt, clippy con
`-D warnings` y tests) en los dos modos del kit (ADR 0001), se ha probado a
mano y sus gaps están en [`barrierkit-gaps.md`](barrierkit-gaps.md). Lo que no
se pudo ejecutar se dice aquí; no se da por hecho.

## Fase 1 — Núcleo

### Plan

1. **Workspace.** `Cargo.toml` con `crates/curseterm` (la app) y
   `crates/barrier-kit-next` (bloques, ADR 0001); `flake.nix` con barrierkit
   como input (ADR 0004); `dev.sh` con `check`, `run`, `kit-parches` y
   `kit-tal-cual`.
2. **Bloque `canvas`** (BK-009) en `barrier-kit-next`: un elemento que da el
   `Canvas` de Skia, la escala y la colección de fuentes, y se redibuja cuando
   cambia su revisión.
3. **Sesión.** `alacritty_terminal` (ADR 0002): shell del usuario (`$SHELL`,
   si no `/etc/passwd`), `TERM=xterm-256color`, `COLORTERM=truecolor`; hilo de
   E/S con `WakeHandle`; título por OSC 0/2.
4. **Rejilla.** Dibujo por rachas: fondo por racha de color, texto por racha de
   estilo; colores ANSI, 256 y truecolor; negrita, cursiva, subrayado,
   inverso; cursor bloque, barra y subrayado; scrollback con la rueda.
5. **Entrada.** Codificador de teclado (texto, Ctrl, Alt como ESC, flechas y
   modos de cursor, F1–F12, Inicio/Fin, RePág/AvPág); Super nunca llega al PTY
   (ADR 0003); Ctrl+Mayús+C/V.
6. **Selección y portapapeles.** Arrastre (simple), doble clic (palabra),
   triple clic (línea); copiar y pegar con el `Clipboard` del kit; pegado
   entre corchetes si el programa lo pidió.
7. **Redimensionado.** Al cambiar el tamaño del lienzo se recalcula la
   rejilla y se llama a `Term::resize` y `OnResize` del PTY (`SIGWINCH`).
8. **Configuración.** `~/.config/malaysia/org.curseterm.CurseTerm.domain`
   (`load_defaults`/`use_defaults`): familia y tamaño de fuente y esquema de
   color. Esquemas incluidos como datos.
9. **Ventana.** `Titlebar` del kit con el título de la sesión; pestañas
   mínimas para que «New Tab» haga algo real (la barra de iTerm2 completa es
   de la fase 2).
10. **Menú global mínimo** con `Action` y atajos Super:
    - CurseTerm: About CurseTerm, Preferences…, Quit CurseTerm.
    - Shell: New Window, New Tab, Close.
    - Edit: Copy, Paste, Select All.
11. **Workarounds de plataforma** en modo «kit tal cual»: una ventana por
    proceso (New Window lanza otro proceso; Quit cierra los procesos de
    CurseTerm registrados en `$XDG_RUNTIME_DIR/curseterm/`); Preferencias en un
    proceso aparte que escribe el `.domain`, que la ventana vigila con
    `use_defaults`.
12. **Parches** en `kit-parches/`: título en vivo (BK-002) y atajos consumidos
    con `ShortcutKey::Named` (BK-003).
13. **Prueba** en un compositor sin pantalla con lavapipe; capturas en
    `docs/capturas/`; y en `malaysia/dev.sh lab` si el entorno lo permite.

### Resultado (2026-11-05)

**Cómo se probó.** Todo se ejecutó en el contenedor, sin GPU:

1. `nix develop` (dev shell del kit más `CURSETERM_DEV_SHELL`) con Vulkan por
   lavapipe.
2. sway 1.12 sin pantalla (`WLR_BACKENDS=headless WLR_RENDERER=pixman`) y un
   bus de sesión D-Bus privado.
3. `wtype` para el teclado, `grim` para las capturas, `wl-copy`/`wl-paste`
   para el portapapeles, y un cliente mínimo de `zwlr_virtual_pointer_v1`
   para el ratón (`wlrctl` crea un puntero nuevo en cada orden y el asiento
   se queda sin puntero entre una y otra).
4. `./dev.sh check` limpio en los dos modos del kit (ADR 0001).

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Shell del usuario en el PTY, prompt y cursor | ✓ | `fase1-colores-claro.png` |
| 16 colores ANSI, cubo de 256, truecolor | ✓ | `fase1-colores-claro.png` |
| Negrita, cursiva, subrayado, inverso, tachado, tenue | ✓ | ídem |
| Respaldo de fuentes: ñ, CJK de ancho doble | ✓ | ídem |
| Esquema claro u oscuro según `appearance.domain`, en vivo | ✓ | `fase1-colores-oscuro-y-ajustes.png` |
| Redimensionado: `stty size` da 39×136 a 1100×700 y 27×98 a 800×500 | ✓ | — |
| Scrollback con la rueda | ✓ | `fase1-scrollback.png` |
| Selección por arrastre, doble clic (palabra) y triple clic (línea) | ✓ | `fase1-seleccion-linea.png` |
| Copiar (Super+C, Ctrl+Mayús+C) y pegar (Super+V) con el portapapeles del sistema | ✓, comprobado con `wl-paste` y `wl-copy` | — |
| Select All (Super+A) | ✓ | — |
| New Tab (Super+T), títulos por OSC, Close (Super+W) | ✓ | `fase1-pestanas.png` |
| New Window (Super+N) en un proceso nuevo | ✓ | `fase1-dos-ventanas-foco.png` |
| Cursor hueco en la ventana sin foco | ✓ | ídem |
| Quit (Super+Q) cierra todas las ventanas | ✓ | — |
| About y Settings… en su ventana | ✓ | `fase1-acerca-de.png` |
| Cambiar el tamaño de fuente en Settings cambia el terminal en vivo | ✓ | `fase1-ajustes-fuente-en-vivo.png` |
| Escala fraccional 1,5 | ✓, sin costuras | `fase1-escala-1-5.png` |
| En reposo no hay frames nuevos | ✓, 1 % de CPU | — |
| Parche 0001: título de la ventana en vivo | ✓, sway muestra el título de la sesión | `fase1-titulo-en-vivo-parcheado.png` |
| Parche 0003: menú en D-Bus sin `org_kde_kwin_appmenu_manager` | ✓ `GetLayout` da CurseTerm, Shell y Edit con atajos; Copy pasa a activo al seleccionar; `Event clicked` sobre New Tab abre la pestaña | — |

**No probado:**

- **Menú global en Malaysia** (`malaysia/dev.sh lab`): el dev shell de
  malaysia construye `libadwaita` 1.9.3 y su test `test-animation-target`
  aborta en el sandbox de Nix. Sin Malaysia no hay
  `org_kde_kwin_appmenu_manager`, y con el kit tal cual el menú ni siquiera
  se publica en D-Bus (BK-018). El menú se probó por D-Bus con el parche
  0003, no en la barra de menús.
- **Atajos fijos de Malaysia** (Super+W, Super+Q): no se pudieron probar.
- **IME, teclas muertas y selección primaria:** el kit no los ofrece
  (BK-004, BK-005).
- **Modo de ratón de las aplicaciones** (`vim`, `htop` con ratón): no está
  implementado; la rueda en pantalla alternativa manda flechas.
- **GNOME y KDE** reales.
- **Primera tecla de cada `wtype`:** se pierde la primera tecla de cada
  invocación (cada una sube su propio keymap). No se aisló si es del kit o de
  la herramienta; en las pruebas se manda antes un Escape.

**Difiere de iTerm2:**

- «Preferences…» se llama «Settings…»/«Ajustes…», para rellenar la ranura
  «Ajustes» de la barra de Malaysia.
- Atajos con Super (ADR 0003). En Malaysia, Super+W cierra la ventana entera.
- La barra de pestañas solo aparece con dos o más pestañas, como en iTerm2,
  pero no tiene «+», reordenar ni indicador de actividad.
- El emoji sale monocromo (no hay fuente de color en el entorno) y los
  caracteres de caja salen de la fuente de respaldo y no encajan en la celda;
  iTerm2 los dibuja él mismo.
- Cada ventana es un proceso (BK-001): no se mueven pestañas entre ventanas.
- El menú «Window» y la lista de ventanas son de la fase 2.

**Fallos encontrados y corregidos al probar:** handlers del menú que pedían
contexto de Freya (BK-017), signo de la rueda, préstamo doble de `State` al
cerrar, y repintado al cambiar foco, fuente o esquema.

## Fase 2 — Organización

### Plan

1. **Modelo de paneles** (`crates/curseterm/src/panes.rs`, lógica pura con
   tests): árbol de divisiones con proporciones; dividir junto al panel
   activo, cerrar y colapsar, recorrer en orden, vecino arriba, abajo,
   izquierda y derecha por geometría, y mover el divisor.
2. **Bloque `views::split_panes`** (BK-007) en `barrier-kit-next`: un
   contenedor de N hijos en un eje, anidable, con divisores que se arrastran;
   controlado (`ratios` y `on_ratios`).
3. **Cada panel mide su sesión:** el PTY de cada panel sigue su propio tamaño
   (`SIGWINCH` por panel); el panel activo recibe el teclado y se elige con
   un clic.
4. **Barra de pestañas** (BK-006): botón «+», pestaña siguiente y anterior,
   mover a izquierda y derecha, Super+1…9, y «Show Tab Bar» para verla con una
   sola pestaña.
5. **Menús completos** de Shell, View y Window, con estados vivos (marcas,
   activado según contexto) y el submenú dinámico «Select Tab», sobre el
   bloque `app_menu` (BK-010):
   - Shell: New Window, New Tab, Duplicate Tab, Split Vertically/Horizontally
     with Current Profile, Close, Close Terminal Window, Close All Panes in Tab.
   - View: Show Tab Bar, Toggle Full Screen, Maximize Active Pane, Make Text
     Bigger, Make Text Normal Size, Make Text Smaller.
   - Window: Minimize, Zoom, Select Next/Previous Tab, Move Tab Left/Right,
     Select Tab ▸, Split Pane ▸ (Select Pane Above/Below/Left/Right, Next/
     Previous Pane, Move Divider Up/Down/Left/Right).
6. **Lista de ventanas en «Window»:** cada ventana es un proceso (BK-001) y
   una app no puede pedir un token de activación para traer otra al frente
   (solo la feature `shell`). Con el kit tal cual no se muestra; se intenta
   como parche si el resto de la fase cabe.
7. **Prueba** en sway sin pantalla como en la fase 1, y por D-Bus con el
   parche 0003 para el menú.
