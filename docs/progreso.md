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

### Resultado (2026-11-05)

Probado como en la fase 1 (sway sin pantalla, lavapipe, `wtype`, el puntero
virtual del scratchpad y el menú por D-Bus con el parche 0003). `./dev.sh
check` limpio en los dos modos del kit; 17 tests de la app y 1 del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Split Vertically (Super+D) y Horizontally (Super+Mayús+D), anidados | ✓; cada panel tiene su PTY (`stty size` da 14×53 en un cuarto) | `fase2-paneles.png` |
| Select Pane Left/Right/Above/Below (Super+Alt+flechas), Next/Previous Pane (Super+] y Super+[) | ✓ | — |
| Move Divider (Ctrl+Super+flechas) | ✓, el panel reajusta el texto | `fase2-divisor-movido.png` |
| Arrastrar el divisor con el ratón; clic para activar un panel | ✓ | `fase2-divisor-arrastrado.png` |
| Maximize Active Pane (Super+Mayús+Return), con marca | ✓ | `fase2-panel-maximizado.png` |
| Close (Super+W) cierra el panel; el último cierra la pestaña y luego la ventana | ✓ | — |
| Close All Panes in Tab (Super+Alt+W) | ✓, solo activo con divisiones | — |
| Barra de pestañas con «+» | ✓ | `fase2-barra-de-pestanas.png` |
| Select Next/Previous Tab (Super+Mayús+} y {), Super+1…9, Move Tab Left/Right | ✓ | — |
| «Select Tab» ▸ con la lista de pestañas y una marca de radio en la activa | ✓ por D-Bus | — |
| Show Tab Bar con una sola pestaña, con marca | ✓ | `fase2-mostrar-barra.png` |
| Duplicate Tab abre en el directorio de la sesión activa | ✓ | `fase2-duplicar-y-texto-mayor.png` |
| Make Text Bigger/Smaller/Normal Size (Super+=, Super+-, Super+0) | ✓, el PTY se reajusta | ídem |
| Toggle Full Screen (Ctrl+Super+F) | ✓ en sway | — |
| Estados vivos por D-Bus: activado/desactivado según pestañas y divisiones, marcas | ✓ | — |

**No probado:**

- **Minimize y Zoom:** sway no minimiza y en mosaico no maximiza; se llaman
  `WindowControls::minimize` y `toggle_maximize`, sin efecto visible aquí.
- **Malaysia:** sigue sin poder construirse (fase 1). En Malaysia, Super+M y
  Super+W los atiende el compositor.

**Difiere de iTerm2:**

- **Lista de ventanas en «Window»:** no está. Cada ventana es un proceso
  (BK-001) y una app no puede pedir un token de activación para traer otra al
  frente (BK-019), así que un ítem así no haría nada. En Malaysia la barra ya
  añade al menú «Window» la lista de ventanas de la app.
- **Cerrar un panel** activa el primero del árbol, no el vecino.
- **Los paneles inactivos** no se atenúan; solo su cursor sale hueco.
- **Atajos con Mayús:** se declaran con el carácter producido (`Super+Shift+}`)
  por BK-016; en un teclado no US pueden no coincidir.
- **Make Text Bigger** se muestra como Super+= y no ⌘+.
- **Faltan** Undo Close, Edit Tab Title, mover pestañas entre ventanas y
  arrastrarlas.

## Fase 3 — Perfiles y preferencias

### Plan

1. **Perfiles** en el `.domain` de la app: nombre, familia y tamaño de fuente,
   esquema de color (o «seguir al sistema»), orden inicial y directorio
   (inicio, el de la sesión anterior o uno fijo), y cuál es el perfil por
   omisión. Sin perfiles guardados hay uno, «Default».
2. **Cada sesión con su perfil:** la orden y el directorio al arrancar; la
   fuente y el esquema al dibujar. Divisiones y pestañas nuevas usan el perfil
   de la sesión activa («with Current Profile»).
3. **Esquemas importables** desde `.itermcolors` (plist XML) con el crate
   `plist`. Los importados se guardan en el mismo `.domain`, así llegan en vivo
   a todas las ventanas. Se importan desde Preferencias (ruta en un campo,
   porque el kit no tiene selector de archivos) o con
   `curseterm --import-colors <archivo>`.
4. **Preferencias** con la estructura de iTerm2 hecha con `PreferencesWindow`:
   «Appearance» (barra de pestañas con una sola pestaña) y un grupo
   «Profiles» con un panel por perfil, cuyas secciones siguen las pestañas
   General, Colors y Text del editor de perfiles de iTerm2; duplicar, borrar y
   elegir el perfil por omisión.
5. **Menú «Profiles»** con un ítem por perfil, que abre una pestaña con él
   (Ctrl+Super+1…9). El kit no deja cambiar los ítems de un menú de primer
   nivel: con el kit tal cual se actualizan las etiquetas y la lista nueva
   aparece en la siguiente ventana; un parche (0004) añade
   `MenuHandle::set_menu` para cambiarla en vivo.
6. **Prueba** en sway sin pantalla, como en las fases anteriores.

### Cambios al plan

Con las capturas de iTerm2 que mandó el usuario, Ajustes no usa
`PreferencesWindow` (barra lateral) sino lo que hace iTerm2: pestañas de
barra de herramientas arriba (`TabView`) y, en Profiles, la lista de perfiles
(`Table`, ★ en el de por omisión, + y −) junto a un `SegmentedControl` con
General, Colors y Text. También se reordenaron los menús según esas capturas:
Shell con «with Current Profile», Session (Restart Session, Reset), Edit con
Clear Buffer y Clear Scrollback Buffer, y en Window el submenú «Tab ▸».

### Resultado (2026-11-05)

Probado en sway sin pantalla como en las fases anteriores. `./dev.sh check`
limpio en los dos modos del kit; 20 tests de la app y 1 del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `curseterm --import-colors 'Solarized Dark.itermcolors'` (sacado de `ColorPresets.plist` de iTerm2) | ✓; un archivo que no es plist sale con error y código 1 | — |
| Ajustes: pestañas Appearance y Profiles; lista, ★, + y −; General, Colors y Text | ✓ | `fase3-ajustes-perfil-general.png`, `fase3-ajustes-perfil-colores.png` |
| Elegir un esquema importado; el terminal (otro proceso) cambia en vivo | ✓ con el terminal con foco; sin foco, solo con el parche 0005 (BK-022) | `fase3-repintado-sin-foco-parche-0005.png` |
| «+» crea un perfil; el menú «Profiles» del terminal lo muestra al momento | ✓ con el parche 0004; con el kit tal cual aparece en la siguiente ventana | — |
| Ctrl+Super+2 abre una pestaña con ese perfil y su esquema | ✓ | `fase3-pestana-con-otro-perfil.png` |
| Split with Current Profile hereda el perfil | ✓ | `fase3-division-con-el-perfil.png` |
| Orden del perfil y directorio fijo | ✓ | `fase3-orden-directorio-y-restart.png` |
| Restart Session (shell nueva, mismo perfil y directorio) | ✓ | ídem |
| Clear Buffer (Super+K) deja la línea del prompt; Clear Scrollback Buffer se activa con historial | ✓ | `fase3-clear-buffer-y-reset.png` |
| Reset (Super+R) tras romper el juego de caracteres con `ESC (0` | ✓ | ídem |
| Appearance › «Show tab bar even when there is only one tab» y la marca de View › Show Tab Bar en el terminal | ✓ | `fase3-ajustes-apariencia.png` |

**No probado:** cambiar el nombre de un perfil con sesiones abiertas (las
sesiones con el nombre viejo pasan al perfil por omisión); la fila
«Directory path» con el teclado; New Window with Current Profile (lanza
`curseterm --profile NOMBRE`; se probó `--profile` solo a través de esa orden
del menú, sin captura).

**Difiere de iTerm2:**

- En Ajustes solo hay Appearance y Profiles: General, Keys, Arrangements,
  Pointer, Shortcuts y Advanced no tienen todavía nada real detrás.
- El editor de perfil tiene General, Colors y Text; faltan Window, Terminal,
  Session, Keys y Advanced, y en General faltan Shortcut key, Tags, Badge,
  Title, Icon y Send text at start.
- «Command» es un campo (vacío = shell de inicio) y no un desplegable «Login
  Shell / Command».
- Importar pide la ruta en un campo: el kit no tiene selector de archivos
  (BK-023). Los colores `Calibrated` de un `.itermcolors` se toman como sRGB.
- «Show Tab Bar» está en View; en iTerm2 actual solo existe en Appearance.
- La ventana de Ajustes no tiene buscador.
