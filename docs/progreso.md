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

## Fase 4 — Lo característico de iTerm2

### Plan

1. **Bucle de E/S propio** (ADR 0005) con un separador de OSC 7, 133 y 1337.
2. **Shell integration:** directorio actual por OSC 7 y `1337;CurrentDir`
   (lo usan Duplicate Tab y «Reuse previous session's directory»); marcas
   por OSC 133;A y `1337;SetMark`, con el triángulo de iTerm2 en el margen,
   Set Mark, Jump to Mark, Next y Previous Mark.
3. **Find** (Edit › Find ▸): barra de búsqueda en el panel con resaltado,
   Find Next, Find Previous y Use Selection for Find, con la búsqueda regex
   de `alacritty_terminal`.
4. **Broadcast input** (Shell › Broadcast Input ▸): solo la sesión actual,
   todos los paneles de la pestaña o de todas, con un indicador en los
   paneles que reciben.
5. **Imágenes en línea** (`1337;File=…;inline=1`), decodificadas con Skia.
6. **Triggers** por perfil (regex y acción: resaltar, campana o enviar
   texto), editables en Ajustes y activables desde Session › Triggers ▸.
7. **Hotkey window:** Wayland no tiene atajos globales salvo el portal
   `GlobalShortcuts`, que el portal de Malaysia no implementa y el kit no
   usa. Se anota como gap y se ofrece `curseterm --hotkey`, que abre o cierra
   una ventana dedicada, para enlazarlo a un atajo del escritorio.
8. **Prueba** en sway sin pantalla, como en las fases anteriores.

### Resultado (2026-11-05)

Probado en sway sin pantalla como en las fases anteriores. `./dev.sh check`
limpio en los dos modos del kit; 26 tests de la app y 1 del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Bucle de E/S propio: shell, `seq`, colores y redimensionado como antes | ✓ | — |
| 40 prompts con `OSC 133;A`: triángulo azul en el margen de cada uno | ✓ | `fase4-marcas-e-imagen.png` |
| Previous Mark (Super+Mayús+↑) tres veces: de la marca 27 a la 24 | ✓ | — |
| Dividir el panel reajusta las líneas largas y las marcas siguen en su prompt | ✓ tras convertir las marcas a líneas lógicas (`io::reflow`) | `fase4-broadcast.png` |
| `1337;File=inline=1;width=20` con un PNG: 20 celdas de ancho y el texto sigue debajo | ✓ | `fase4-marcas-e-imagen.png` |
| `OSC 7` con `%20`: una pestaña nueva con «Reuse previous session's directory» abre en `/tmp/con espacio` aunque la shell estaba en `~` | ✓ | — |
| Find (Super+F): búsqueda al escribir hacia arriba; Intro y ↑ la anterior, ↓ la siguiente; Escape cierra | ✓ | `fase4-buscar.png` |
| Broadcast Input to All Panes in All Tabs (Super+Alt+I): franjas en los dos paneles y `echo` en los dos | ✓ | `fase4-broadcast.png` |
| Triggers del perfil: resaltar `error [0-9]+`, «Send Text» `si\n` ante `¿seguir?`, campana con `^campana$` | ✓; la campana aclara el panel (píxel `#002B36` → `#1A4049`) | `fase4-triggers.png` |
| Ajustes › Perfiles › Triggers: lista, acción, texto, «−» y «+», guardado en el `.domain` | ✓ | `fase4-ajustes-triggers.png` |
| Menú por D-Bus (parches 0003 y 0004): radio de Broadcast Input, Jump/Next/Previous Mark activas tras Set Mark, Enable Triggers se desmarca | ✓ | — |
| `curseterm --hotkey` abre la ventana; otro `--hotkey` la cierra y sale con 0; un tercero la vuelve a abrir | ✓ | — |

**Errores encontrados y corregidos al probar:** tras cerrar la búsqueda con
Escape el foco seguía marcado y el terminal ignoraba el teclado; el reajuste
de líneas al dividir movía las marcas una línea; `Timers::get()` en el
callback de la campana abortaba la app (BK-017); `^…$` no casaba con la
búsqueda en flujo de `alacritty_terminal`, que incluye los blancos del final
de fila: los triggers usan ahora el crate `regex` sobre el texto de la fila.

**No probado:** imágenes con `preserveAspectRatio=0` y en porcentaje;
`1337;SetMark` y `1337;CurrentDir` (solo sus tests); Use Selection for Find;
Edit Triggers… (abre Ajustes como Settings…); un `Event` de dbusmenu
enviado sin esperar respuesta (`dbus-send` sin `--print-reply`) no hizo nada
dos veces, con respuesta sí; no se investigó la causa.

**Difiere de iTerm2:**

- Las marcas y las imágenes se anclan a líneas absolutas: cuando el
  historial llega a su límite y empieza a descartar líneas, se desplazan.
- La búsqueda resalta solo la coincidencia actual (con el color de
  selección), sin contar coincidencias ni opciones de mayúsculas o regex;
  distingue mayúsculas solo si la consulta las tiene.
- Los triggers miran cada fila completa, no la línea lógica, y no tienen el
  modo «Instant» para la línea del prompt; solo tres acciones.
- La campana es visual (BK-026); la ventana hotkey se cierra en vez de
  ocultarse (BK-025).
- Broadcast no tiene «Toggle Broadcast Input to Current Session» ni el aviso
  de iTerm2 al activarlo.

## Fase 5 — Perfil Terminal, campana y búsqueda completa

### Plan

1. **Perfil › Terminal** en Ajustes: «Scrollback lines» (por omisión 1000,
   como iTerm2) y, en la campana, «Flash visual bell» y «Show bell icon in
   tabs». Se aplican en vivo a las sesiones abiertas
   (`Term::set_options`). Sin «Unlimited scrollback»:
   `Grid::clear_history` de `alacritty_terminal` vuelve a reservar todas las
   filas del límite al limpiar, así que el límite es finito.
2. **Campana en una pestaña de fondo:** la pestaña muestra un icono hasta que
   se selecciona. El kit no tiene símbolo de campana (BK-024); el bloque
   `TabBar` gana un indicador por pestaña.
3. **Find completo:** todas las coincidencias visibles resaltadas, la actual
   con la selección, y «n de m» o «Sin coincidencias» en la barra.
4. **Perfil › Advanced:** los triggers pasan a la sección Advanced, donde los
   tiene iTerm2.
5. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla como en las fases anteriores. `./dev.sh check`
limpio en los dos modos del kit; 27 tests de la app y 1 del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Find «orden 3» tras 40 órdenes: las 11 coincidencias visibles resaltadas, la actual con la selección y «10 of 11» | ✓ | `fase5-buscar-todas.png` |
| Find «2» tras `seq 1 30`: «13 of 13» (`22` cuenta dos veces) | ✓ | — |
| `scrollback_lines = 50` escrito en el `.domain` con la ventana abierta; tras `seq 1 500` lo más antiguo es la 416, 50 líneas sobre la primera visible | ✓ | — |
| `printf '\a'` en una pestaña de fondo: insignia en esa pestaña, que desaparece al seleccionarla | ✓ | `fase5-campana-en-pestana.png` |
| «Show bell icon in tabs» apagado: sin insignia | ✓ | — |
| Ajustes › Perfiles › Terminal: líneas de historial y los dos interruptores, guardados en el `.domain` | ✓ | `fase5-ajustes-terminal.png` |
| Ajustes › Perfiles › Advanced con los triggers | ✓ | `fase5-ajustes-avanzado.png` |

**No probado:** «Flash visual bell» apagado (la rama es una condición, sin
captura); un historial de 0 líneas; el límite de 100 000 con un valor mayor en
el campo.

**Difiere de iTerm2:**

- Sin «Unlimited scrollback» ni «Silence bell»: no hay campana sonora
  (BK-026).
- La insignia usa `SpeakerWave` en lugar de una campana (BK-024).
- Find no tiene las opciones de mayúsculas ni de expresión regular, y el
  contador se calcula al buscar, no cuando llega texto nuevo.
- El perfil sigue sin Window, Session y Keys.

## Fase 6 — Perfil Window, Session y Keys

### Plan

1. **Window:** «Columns» y «Rows» (80 × 25 por omisión, como iTerm2) para
   las ventanas nuevas. El kit restaura siempre el último tamaño guardado y
   no deja pedir otro (BK-012): la ventana nueva olvida el tamaño guardado y
   calcula el suyo con las métricas de la fuente del perfil.
2. **Session:** «When the session ends» (cerrar, reiniciar o dejar abierta
   con un aviso) y «Prompt before closing» (siempre, nunca o si hay procesos
   además de la shell), con el `Alert` del kit en Shell › Close y al cerrar
   una pestaña.
3. **Keys:** «Alt key acts as» Normal o Esc+. En Linux AltGr compone
   caracteres y no llega como Alt, así que hay una sola opción y no una por
   lado como en iTerm2.
4. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla, con las ventanas de CurseTerm flotantes para
que el compositor respete su tamaño. `./dev.sh check` limpio en los dos modos
del kit; 27 tests de la app y 1 del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Ventana nueva con el perfil por omisión: `tput cols` y `tput lines` dan 80 y 25 | ✓ tras medir la barra de título con `TITLEBAR_CLIENT_HEIGHT` (con `TITLEBAR.thickness` salían 24 filas) | `fase6-ventana-80x25.png` |
| `sleep 100` y Super+W: aviso «Close?»; Escape lo cancela sin mandar `^[` a la shell; Intro cierra | ✓ tras diferir el cierre del aviso (BK-027) | `fase6-preguntar-al-cerrar.png` |
| Super+W en una sesión ya terminada, sin procesos: cierra sin preguntar | ✓ | — |
| «Keep open»: `exit` deja la sesión con «[Process completed]» | ✓ | `fase6-sesion-terminada.png` |
| «Restart»: `exit` abre una shell nueva en el mismo panel | ✓ | — |
| Alt «Normal»: Alt+B en `cat -v` da `b`; «Esc+» da `^[b` | ✓ | — |
| Ajustes › Perfiles con ocho secciones; Window, Session y Keys | ✓ con la ventana a 980 px (BK-028) | `fase6-ajustes-sesion.png` |

**No probado:** «Prompt before closing» en Close All Panes, en Close Window y
en la «×» de una pestaña (comparten `request_close`, sin captura);
«Always» y «Never»; cambiar Columns y Rows desde el campo.

**Difiere de iTerm2:**

- Una sola opción para Alt, no una por lado; AltGr compone.
- «Prompt before closing» no tiene la lista de procesos permitidos de
  iTerm2: cuenta cualquier grupo de primer plano distinto de la shell. Quit
  y el cierre pedido por el compositor no preguntan (BK-013).
- Ninguna ventana recuerda su tamaño ni si estaba maximizada (BK-012).
- Window no tiene estilo, transparencia ni pantalla; Keys no tiene atajos
  por perfil.

### Revisión (2026-11-05)

Una revisión del código de las fases 4 a 6 encontró diez problemas, todos
corregidos:

- Un cierre pendiente guardaba la pestaña por índice; ahora guarda una de
  sus sesiones.
- «Restart» con una orden que termina al instante entraba en bucle; si la
  sesión dura menos de un segundo queda abierta con un aviso.
- El recuento «n de m» recorría todo el historial en cada tecla; ahora se
  calcula tras una pausa de `DURATION_FAST_MS`, y la búsqueda compilada se
  reutiliza mientras no cambia la consulta.
- Con una orden de perfil (`sh -c`), «Prompt before closing» no veía el
  proceso; ahora cuenta como trabajo mientras viva.
- El tamaño de la ventana no limitaba el tamaño de letra como el panel;
  los dos usan `Profile::text_size`.
- Una sesión terminada ya no recibe teclas ni llena el registro de avisos.
- Columns, Rows y Scrollback solo admiten dígitos; 0 columnas o filas se
  guarda como 1.
- Dos comentarios de dos líneas pasan a una.

Al probar el primer punto apareció otro fallo: tras cerrar una pestaña desde
el aviso, el panel que quedaba seguía mostrando la sesión cerrada y la
retenía viva, porque la clave del lienzo no incluía la sesión. Ahora la
incluye. Probado en sway: `false` con «Restart» deja el aviso sin crear
procesos; la «×» de una pestaña con `sleep 100` pregunta, cierra esa pestaña
y termina `sleep`; escribir en una sesión terminada no deja avisos; «13 of
13» llega tras la pausa.

## Fase 7 — Salida del último comando, Copy Mode y Find Cursor

### Plan

1. **Edit › Select Output of Last Command (⇧⌘A):** la integración con la
   shell marca dónde empieza la salida (OSC 133;C) y dónde acaba el comando
   (OSC 133;D o el siguiente prompt). La orden selecciona esas líneas. Sin
   integración, la entrada queda desactivada.
2. **Edit › Copy Mode (⇧⌘C):** un cursor de teclado que mueve y selecciona
   sin ratón. Se apoya en el modo vi de `alacritty_terminal`: h j k l y
   flechas, w b e, 0 ^ $, H M L, g y G, Ctrl+B y Ctrl+F; v o espacio para
   seleccionar caracteres y V para líneas; y copia y sale; Escape o q salen.
3. **View › Find Cursor (⌘/):** oscurece el panel salvo un círculo alrededor
   del cursor durante `DURATION_SLOW_MS`.
4. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con una integración mínima en bash: `PS0` emite
OSC 133;C y `PROMPT_COMMAND` emite 133;D y 133;A. `./dev.sh check` limpio en
los dos modos del kit; 28 tests de la app y 1 del bloque. Ninguna función
nueva necesitó un gap del kit.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `seq 1 5`, `echo hola` y ⇧⌘A con ⌘C: el portapapeles tiene `hola` | ✓ | — |
| ⇧⌘C, k k k, v, j y $: la selección va del cursor de copia al final de la línea; y copia y sale; la x siguiente llega a la shell | ✓ | `fase7-modo-copia.png` |
| ⇧⌘C, g, V, j e y: copia las dos primeras líneas enteras | ✓ | — |
| ⇧⌘C, k y Escape: sale sin escribir nada; la z siguiente llega a la shell | ✓ | — |
| ⌘/: el panel se oscurece salvo un círculo en el cursor y vuelve a la normalidad tras `DURATION_SLOW_MS` | ✓ | `fase7-buscar-cursor.png` |

**No probado:** la entrada «Copy Mode» marcada en el menú global y «Select
Output of Last Command» desactivada sin integración (la prueba usó el kit tal
cual, sin menú en D-Bus, BK-018); Ctrl+B y Ctrl+F en el modo copia; la
salida del último comando tras un cambio de tamaño que reajusta las líneas.

**Difiere de iTerm2:**

- Copy Mode no tiene selección rectangular (Ctrl+V) ni búsqueda dentro del
  modo, y sus atajos no se pueden cambiar.
- Find Cursor es un círculo fijo que dura `DURATION_SLOW_MS`; iTerm2 anima el
  foco y lo mantiene mientras se pulsa ⌘/.
- La salida solo termina con OSC 133;D. El plan contaba también con el
  siguiente prompt (133;A), pero 133;A y `SetMark` llegan como la misma marca;
  una shell que no emite 133;D deja la entrada desactivada.

## Fase 8 — Undo Close e historial de pegado

### Plan

1. **Shell › Undo Close (⇧⌘T):** al cerrar una pestaña o un panel desde el
   menú, el teclado o la «×», sus sesiones siguen vivas cinco segundos.
   Undo Close las devuelve como una pestaña nueva, con sus procesos y su
   título. Una sesión cuyo proceso terminó no se puede deshacer.
2. **Session › Open Paste History (⇧⌘H):** CurseTerm recuerda los últimos
   textos copiados y pegados en la ventana. Una hoja del kit (`sheet`) con un
   `SearchField` que filtra y una `Table` con las entradas: las flechas
   eligen, Intro pega y Escape cierra.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla. `./dev.sh check` limpio en los dos modos del
kit; 30 tests de la app y 1 del bloque. La hoja del historial reutiliza
`sheet`, `SearchField` y `Table` del kit; no hizo falta ningún gap nuevo, y
Intro difiere el cierre como el aviso de cierre (BK-027).

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Pestaña nueva, `X=vivo`, Super+W y ⇧⌘T: la pestaña vuelve y `echo $X` da `vivo` | ✓ | `fase8-deshacer-cierre.png` |
| Super+W y seis segundos de espera: la shell cerrada termina y ⇧⌘T no hace nada | ✓ | — |
| Panel dividido con `Y=panel`, Super+W y ⇧⌘T: vuelve como pestaña nueva y `echo $Y` da `panel` | ✓ | — |
| Pegar `echo alfa` y `echo beta`, ⇧⌘H, escribir `alf` e Intro: pega `echo alfa` sin mandar Intro a la shell | ✓ | `fase8-historial-de-pegado.png` |
| ⇧⌘H y Escape: cierra sin mandar `^[` a la shell | ✓ | — |

**No probado:** las flechas dentro del historial; el estado activado de Undo
Close y Open Paste History en el menú global (BK-018); deshacer una pestaña
cerrada desde el aviso «Close?».

**Difiere de iTerm2:**

- Un panel cerrado vuelve como pestaña nueva, no a su sitio en la división.
- Solo se puede deshacer el último cierre, y el plazo de cinco segundos no se
  puede cambiar.
- Cerrar la última pestaña cierra la ventana y no se puede deshacer (un
  proceso por ventana, BK-001).
- El historial vive en la ventana: no se comparte entre ventanas, no se
  guarda en disco y no muestra cuándo se copió cada entrada. Guarda los 20
  textos más recientes.

## Fase 9 — Títulos editables y menú Help

### Plan

1. **Window › Edit Tab Title y Edit Window Title:** una hoja del kit con un
   `TextField`. El título escrito sustituye al que pone la shell; un campo
   vacío vuelve al automático. Intro acepta y Escape cancela.
2. **Help › Copy Mode Shortcuts:** una hoja con una `Table` de teclas y
   acciones. La tabla sale de la misma lista que usa el modo copia, así que
   no puede desincronizarse.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con el kit parcheado, para disparar las entradas
sin atajo desde el menú en D-Bus (parche 0003). `./dev.sh check` limpio en los
dos modos del kit; 31 tests de la app y 1 del bloque. Un gap nuevo: BK-029,
`TextField` sin `auto_focus`, resuelto con el parche 0006.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Edit Tab Title, escribir `notas` e Intro: la pestaña se llama «notas» y la ventana también | ✓ con el campo enfocado al abrir (parche 0006) | `fase9-editar-titulo.png` |
| Edit Window Title, `proyecto` e Intro: la ventana se llama «proyecto» y la pestaña sigue «notas»; `echo sin-fugas` muestra que el Intro no llegó a la shell | ✓ | — |
| Edit Tab Title con el campo vacío: la pestaña vuelve a «root@vm: ~» | ✓ | — |
| Help › Copy Mode Shortcuts: tabla de teclas y acciones | ✓ | `fase9-atajos-modo-copia.png` |

El menú en D-Bus también cerró pruebas pendientes de las fases 7 y 8:

| Prueba | Resultado |
|---|---|
| «Select Output of Last Command» desactivada sin integración y activada tras `echo hola` con integración | ✓ |
| «Copy Mode» marcada mientras dura el modo | ✓ |
| «Undo Close» activada tras cerrar una pestaña y desactivada a los cinco segundos | ✓ |
| «Paste History» desactivada al empezar y activada tras pegar | ✓ |

**No probado:** con el kit tal cual, el campo sin foco hasta el clic
(BK-029); los botones «Cancel» y «Rename» con el ratón.

**Difiere de iTerm2:**

- El título no admite variables como `\(session.name)`; es texto fijo.
- Help no tiene la ayuda en línea ni las licencias de código abierto.

## Fase 10 — Marcas de tiempo y Reset Character Set

### Plan

1. **View › Show Timestamps (⇧⌘E):** cada sesión guarda cuándo cambió cada
   línea por última vez; con la opción activa, la hora local (`HH:MM:SS`) se
   dibuja a la derecha de cada línea. Las marcas siguen a sus líneas al
   redimensionar, como las marcas de prompt, y se limitan al tamaño del
   historial.
2. **Session › Reset Character Set:** devuelve G0 y G1 a ASCII, para cuando
   un programa deja la pantalla con caracteres de dibujo de líneas.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla; Reset Character Set, que no tiene atajo, con el
kit parcheado y el menú en D-Bus. `./dev.sh check` limpio en los dos modos del
kit; 32 tests de la app y 1 del bloque. Ningún gap nuevo. La hora local sale
de `localtime_r` con `libc`, que ya estaba en el árbol de dependencias.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `echo uno`, dos segundos, `echo dos` y ⇧⌘E: cada línea lleva su hora a la derecha, tres segundos más tarde la segunda pareja, y coincide con `date` | ✓ | `fase10-marcas-de-tiempo.png` |
| ⇧⌘E otra vez: las horas desaparecen; «Show Timestamps» marcada en el menú solo mientras están | ✓ | — |
| `printf '\033(0'` deja el prompt en dibujo de líneas; Reset Character Set y `echo despues`: lo nuevo sale en ASCII | ✓ | `fase10-juego-de-caracteres.png` |

**No probado:** las horas tras redimensionar con reajuste de líneas; una
sesión que llega al límite del historial.

**Difiere de iTerm2:**

- La hora es la del último trozo de salida que tocó la línea, no la de cada
  carácter; las líneas que llegan en el mismo trozo comparten hora.
- No hay formato relativo («hace 5 minutos») ni ajuste para elegirlo.
- Las horas tapan el final de las líneas largas en lugar de apartarlas.

## Fase 11 — Enlaces y guardar la selección

### Plan

1. **Super+clic en un enlace:** abre con `xdg-open` el enlace OSC 8 de la
   celda o, si no hay, la URL (`http`, `https`, `ftp`, `file`, `mailto`) que
   pasa por la columna pulsada. Los eventos de ratón de Freya no traen
   modificadores, así que la ventana sigue Super con los eventos de teclado.
2. **Edit › Open Selection:** abre con `xdg-open` el texto seleccionado.
3. **Shell › Save Selected Text… (⌘S):** una hoja con la ruta del archivo
   (empieza en la carpeta personal; `~/` se expande) que guarda la selección.
4. **Prueba** en sway sin pantalla, con un `xdg-open` falso que anota lo que
   recibe, y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con un `xdg-open` falso al principio del `PATH`
que anota sus argumentos; Open Selection, sin atajo, con el kit parcheado y
el menú en D-Bus. `./dev.sh check` limpio en los dos modos del kit; 33 tests
de la app y 1 del bloque. Un gap nuevo: BK-030, los eventos de ratón no
traen modificadores.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Clic normal en `https://example.org/prueba,`: no abre nada | ✓ | — |
| Super+clic en esa URL: `xdg-open https://example.org/prueba`, sin la coma | ✓ tras contar la propia tecla Meta como pulsada (BK-030) | — |
| Super+clic en un texto con OSC 8: `xdg-open https://osc8.example/doc` | ✓ | — |
| Doble clic en `seleccion-abierta`, ⌘S, `rel.txt` e Intro: `~/rel.txt` guarda la palabra; `~/sel.txt` y `/tmp/abs.txt` también | ✓ | `fase11-guardar-seleccion.png` |
| Open Selection y Save Selected Text desactivadas sin selección y activadas con ella; Open Selection abre el texto elegido | ✓ | — |

**No probado:** un fallo al guardar (solo deja un aviso en el registro);
la hoja de guardar con el foco automático del parche 0006 tras dejar el
campo vacío al abrir.

**Difiere de iTerm2:**

- No hay panel para elegir el archivo (BK-023): la ruta se escribe, y una
  relativa parte de la carpeta personal.
- Un error al guardar no se muestra en la ventana.
- Las URL se buscan en la fila pulsada; una URL partida en dos filas solo se
  encuentra si tiene OSC 8.
- El doble clic separa en `:` y `/`, así que Open Selection abre lo
  seleccionado tal cual; no hay selección inteligente de URL.

## Fase 12 — Registro de la sesión y Paste Special

### Plan

1. **Session › Log › Log to File… y Stop Logging:** una hoja con la ruta
   (relativa a la carpeta personal, como Save Selected Text); el bucle de E/S
   añade al archivo los bytes tal como llegan del PTY hasta que se para.
2. **Edit › Paste Special:**
   - **Paste Slowly:** manda el portapapeles en trozos pequeños con una pausa
     entre ellos, para programas que pierden lo que llega de golpe.
   - **Paste Escaping Special Characters:** antepone `\` a los caracteres
     especiales de la shell.
   - **Paste Selection:** pega la selección del terminal sin pasar por el
     portapapeles.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con el kit parcheado, porque las entradas nuevas
no tienen atajo y se pulsaron desde el menú en D-Bus. `./dev.sh check` limpio
en los dos modos del kit; 34 tests de la app y 1 del bloque. Ningún gap
nuevo.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Log to File…, `sesion.log`, `echo registrado`, Stop Logging y `echo fuera`: el archivo tiene `registrado` (orden y salida) y no `fuera`; las dos entradas se activan y desactivan a la vez | ✓ | — |
| Copiar `mi archivo (1).txt`, `echo ` y Paste Escaping Special Characters: `echo` imprime el nombre entero | ✓ | — |
| Paste Slowly de 63 bytes con un lector en modo raw: llegan 16, 16, 16 y 15 bytes separados 30 ms | ✓ | `fase12-pegar-despacio.png` |
| Doble clic en `palabra`, `echo ` y Paste Selection: imprime `palabra` sin tocar el portapapeles | ✓ | — |

Durante la prueba, un script propio que restauraba el tty desde un descriptor
ya en modo raw dejó la shell sin eco; `strace` mostró que CurseTerm seguía
escribiendo las teclas en el PTY, así que no era un fallo del terminal.

**No probado:** un fallo de escritura en el registro (para el registro y deja
un aviso); Paste Slowly con varias sesiones en difusión.

**Difiere de iTerm2:**

- El registro guarda los bytes crudos, con las secuencias de escape; no hay
  opción de texto plano ni de registro automático por perfil.
- El tamaño del trozo (16 bytes) y la pausa (30 ms) de Paste Slowly son fijos.
- Paste Special no tiene el editor avanzado de iTerm2 (transformaciones,
  base64, tabuladores).

## Fase 13 — Insignia del perfil y rutas con Super+clic

### Plan

1. **Profiles › General › Badge:** un texto del perfil que se dibuja grande
   y translúcido en la esquina superior derecha de cada panel, como la
   insignia de iTerm2. `\(session.name)` se sustituye por el título de la
   sesión; el resto es texto fijo.
2. **Super+clic en una ruta:** si no hay enlace bajo el clic, la palabra
   pulsada (sin `:línea` ni puntuación final) se busca relativa al directorio
   de la sesión; si existe, se abre con `xdg-open`, como el «semantic
   history» de iTerm2.
3. **Transparencia:** queda fuera; el kit no deja que una ventana pida fondo
   translúcido (se anota como gap).
4. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con el `xdg-open` falso de la fase 11.
`./dev.sh check` limpio en los dos modos del kit; 35 tests de la app y 1 del
bloque. Un gap nuevo: BK-031, la ventana no puede pedir fondo translúcido.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `badge = '\(session.name) · prod'` en el `.domain`: el panel muestra «root@vm: ~ · prod» arriba a la derecha | ✓ | `fase13-insignia.png` |
| Ajustes › Perfiles › General tiene la fila Badge; escribir `dev` cambia la insignia de la ventana abierta | ✓ | `fase13-ajustes-insignia.png` |
| `echo 'ver notas.txt:3 y nada.txt'` en `~`; Super+clic en `notas.txt:3` abre `/root/notas.txt`; en `nada.txt`, que no existe, no abre nada | ✓ | — |

**No probado:** la insignia en paneles divididos estrechos, donde el texto
puede salirse por la izquierda.

**Difiere de iTerm2:**

- La insignia solo entiende `\(session.name)`; iTerm2 admite muchas
  variables y su tamaño y posición se ajustan.
- Super+clic en una ruta abre el archivo con `xdg-open`; iTerm2 deja elegir
  el editor y salta a la línea.
- Sin Use Transparency (BK-031).

## Fase 14 — Transparencia

### Plan

1. **Parche 0007:** `WindowControls::set_background_alpha` multiplica el alfa
   del fondo con que el runner limpia la ventana (BK-031).
2. **Profiles › Window › Transparency:** de 0 % a 90 % en pasos de 10. Cada
   panel pinta su fondo por omisión con la opacidad de su perfil; la barra de
   título y la de pestañas siguen opacas.
3. **View › Use Transparency (⌘U):** activa o apaga la transparencia de la
   ventana. Con el kit tal cual la entrada queda desactivada.
4. **Prueba** en sway sin pantalla, que compone el alfa sobre el fondo del
   escritorio, y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con el kit parcheado y un fondo de escritorio
naranja. `./dev.sh check` limpio en los dos modos del kit; 35 tests de la app
y 1 del bloque. BK-031 queda resuelto con el parche 0007.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `transparency = 50`: el panel deja ver el naranja; la barra de título y la de pestañas siguen opacas | ✓ | `fase14-transparencia.png` |
| ⌘U apaga y vuelve a encender la transparencia | ✓ tras corregir un pánico: `set(!*peek())` pedía la escritura con la lectura aún prestada | — |
| Ajustes › Window › Transparency al 80 %: el `.domain` guarda 80 y la ventana abierta cambia en vivo | ✓ | `fase14-ajustes-transparencia.png` |

**No probado:** con el kit tal cual, la entrada de menú desactivada (no hay
menú en D-Bus sin el parche 0003); paneles de perfiles con transparencias
distintas en la misma pestaña; el desenfoque, que sway no hace.

**Difiere de iTerm2:**

- La transparencia afecta al fondo por omisión; los colores de fondo que
  pone un programa siguen opacos, sin la opción de iTerm2 para elegirlo.
- No hay desenfoque propio: depende del compositor.

## Fase 15 — Open Quickly y Open Profiles

### Plan

1. **View › Open Quickly (⇧⌘O):** una hoja con filtro y la lista de las
   sesiones de la ventana («título — perfil»); Intro lleva a la pestaña y al
   panel elegidos.
2. **Profiles › Open Profiles… (⌘O):** la misma hoja con la lista de
   perfiles; Intro abre una pestaña nueva con el perfil elegido.
3. **Una sola hoja de selección:** el historial de pegado, Open Quickly y
   Open Profiles comparten la hoja (filtro, tabla, flechas, Intro, Escape);
   cada uno solo aporta sus entradas y qué hacer con la elegida.
4. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla. `./dev.sh check` limpio en los dos modos del
kit; 36 tests de la app y 1 del bloque. Ningún gap nuevo: las tres listas
usan la misma hoja con `SearchField` y `Table`, y el filtro es una función
pura con su test.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Tres pestañas (dos «Default», una «New Profile»), Super+1, ⇧⌘O, `new` e Intro: va a la pestaña de «New Profile» | ✓ | `fase15-open-quickly.png` |
| ⌘O lista los dos perfiles; `def` e Intro abre una cuarta pestaña con «Default» | ✓ | `fase15-open-profiles.png` |
| ⇧⌘H tras pegar `echo historial`: la entrada sigue en la hoja e Intro la pega | ✓ | — |

**No probado:** Open Profiles en el menú global con el parche 0004, que
reconstruye el menú Profiles; Open Quickly con paneles divididos.

**Difiere de iTerm2:**

- Open Quickly solo busca sesiones de esta ventana (un proceso por ventana,
  BK-001) y no ofrece órdenes, perfiles ni arreglos.
- Open Profiles abre siempre una pestaña; iTerm2 deja elegir ventana nueva.

## Fase 16 — Terminal State, Edit Session y ayuda

### Plan

1. **Session › Terminal State:** casillas con los modos que la aplicación de
   la sesión ha pedido (pantalla alternativa, informe de foco, informe del
   ratón, pegado entre corchetes, cursor y teclado de aplicación); pulsar una
   la cambia mandando al terminal la misma secuencia que mandaría un programa.
2. **Session › Edit Session… (⌘I):** abre Ajustes con el perfil de la sesión
   activa ya elegido.
3. **Help › CurseTerm Help:** abre el repositorio del proyecto con
   `xdg-open`.
4. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con el kit parcheado para leer y pulsar el
submenú desde D-Bus, y con el `xdg-open` falso de la fase 11. `./dev.sh
check` limpio en los dos modos del kit; 36 tests de la app y 1 del bloque.
Ningún gap nuevo.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `printf '\e[?2004h\e[?1h'; cat`: Paste Bracketing y Application Cursor marcadas; el resto, no | ✓ | — |
| Pulsar Application Cursor la desmarca; pulsar Alternate Screen vacía la vista y volver a pulsarla devuelve el texto | ✓ | — |
| Pestaña con «New Profile» y ⌘I: Ajustes se abre con «New Profile» elegido | ✓ | `fase16-editar-sesion.png` |
| ⇧⌘? abre `https://github.com/HBlanqueto/curseterm` | ✓ | — |

**No probado:** Focus Reporting y Mouse Reporting con un programa que los
use; CurseTerm aún no manda eventos de foco ni de ratón a la aplicación,
solo muestra y cambia el modo.

**Difiere de iTerm2:**

- Edit Session edita el perfil, no una copia propia de la sesión.
- Terminal State no tiene «Standard Key Reporting», «Literal mode» ni el
  restablecimiento de todo el estado.
- La ayuda abre el repositorio; no hay manual integrado.

## Fase 17 — Ratón y foco para las aplicaciones

### Plan

1. **Informe del ratón:** si el programa lo pide (modos 1000, 1002 y 1003),
   los clics, el arrastre, el movimiento y la rueda le llegan como
   secuencias, en el formato clásico o en SGR (1006) si lo pide. Con Mayús
   pulsada, el ratón vuelve a seleccionar texto, como en iTerm2.
2. **Modificadores:** la ventana sigue Mayús, Ctrl y Alt igual que Super
   (BK-030), para el salto con Mayús y para añadirlos al informe.
3. **Informe del foco (1004):** al ganar o perder el foco la ventana, la
   sesión activa recibe `CSI I` o `CSI O`.
4. **Prueba** en sway sin pantalla con un lector que muestra lo que llega, y
   documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con un lector en Python que activa los modos,
pone el tty en crudo y guarda lo que recibe. `./dev.sh check` limpio en los
dos modos del kit; 39 tests de la app y 1 del bloque. Ningún gap nuevo; BK-030
ahora cubre todos los modificadores.

**Funciona:**

| Prueba | Resultado |
|---|---|
| Modos 1000, 1006 y 1004; clic en la columna 6 de la fila 2: llegan `CSI <0;6;2M` y `CSI <0;6;2m` | ✓ |
| Abrir Ajustes y cerrarlos: llegan `CSI O` y `CSI I` | ✓ |
| Modos 1000 y 1002 en formato clásico: pulsación (32), dos arrastres (64) al cambiar de celda, soltar (35), rueda arriba (96) y abajo (97) | ✓ |
| Con Mayús pulsada, arrastrar selecciona texto y no llega nada al programa | ✓ |

**No probado:** el modo 1003 (movimiento sin botón) con un programa real
como `htop` o `vim`; el botón central y el derecho; Ctrl y Alt dentro del
informe fuera de los tests.

**Difiere de iTerm2:**

- No hay ajuste para desactivar el informe del ratón por perfil.
- El modo UTF-8 (1005) no está; las celdas más allá de la columna 222 no se
  informan en el formato clásico.

## Fase 18 — Coprocesos

### Plan

1. **Session › Run Coprocess…:** una hoja pide la orden; se lanza con
   `sh -c` en el directorio de la sesión. Lo que sale del terminal le llega
   por la entrada estándar y lo que escribe entra en la sesión como si se
   tecleara.
2. **Session › Stop Coprocess:** termina el coproceso. Las dos entradas se
   activan según haya uno en marcha; cuando acaba solo, se recoge.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla con el kit parcheado, porque las entradas no
tienen atajo y se pulsaron por D-Bus. `./dev.sh check` limpio en los dos
modos del kit; 39 tests de la app y 1 del bloque. Ningún gap nuevo.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Coproceso que espera `PING` con `grep -m1` y escribe `echo PONG`; `echo PING` en la shell: se teclea `echo PONG` y sale `PONG` | ✓ | `fase18-coproceso.png` |
| Stop Coprocess: no queda ningún proceso del coproceso | ✓ tras lanzarlo en su propio grupo y terminar el grupo; antes el `sh` interior quedaba huérfano | — |
| Stop Coprocess activado solo mientras hay uno; uno que acaba solo (`echo ya`) se recoge y la entrada se desactiva | ✓ | — |

**No probado:** cerrar la ventana con un coproceso en marcha (el proceso
termina sin pasar por `Drop`; el coproceso recibe fin de archivo en la
entrada, pero sigue si no la lee).

**Difiere de iTerm2:**

- No hay coprocesos automáticos desde triggers ni la opción de que el
  coproceso no reciba la salida.
- Lo que escribe el coproceso se manda tal cual, sin el modo de pegado entre
  corchetes.

## Fase 19 — Indicador de desplazamiento y selección inteligente

### Plan

1. **Indicador de desplazamiento:** al subir por el historial, una barra fina
   en el borde derecho del panel muestra qué parte del historial se ve.
2. **Selección inteligente (cuatro clics):** selecciona la URL entera bajo
   el puntero o, si no hay, la palabra separada por espacios (una ruta con
   `/` y `:`), como el «smart selection» de iTerm2. Uno, dos y tres clics
   siguen seleccionando carácter, palabra y línea.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-05)

Probado en sway sin pantalla. `./dev.sh check` limpio en los dos modos del
kit; 40 tests de la app y 1 del bloque. Ningún gap nuevo.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `seq 1 200` con 50 líneas de historial y la rueda hacia arriba: aparece la barra en el borde derecho, de un tercio del alto y abajo del todo; al volver abajo desaparece | ✓ | `fase19-indicador.png` |
| Cuatro clics en `(https://a.org/x).` y ⌘C: el portapapeles tiene `https://a.org/x` | ✓ | — |

**No probado:** arrastrar la barra (no se puede: es solo un indicador);
cuatro clics en una ruta fuera del test de `word_span`.

**Difiere de iTerm2:**

- El indicador no se puede arrastrar ni pulsar, y no hay opción para ocultar
  la barra.
- La selección inteligente usa dos reglas fijas (URL o palabra entre
  espacios); iTerm2 trae una lista de reglas editable por perfil.

## Fase 20 — Varias ventanas en un proceso

### Plan

1. **Parche 0008:** `WindowControls::open_window(app, opciones)` abre otra
   ventana en el mismo proceso (BK-001). El runner ya guardaba varias
   ventanas y termina al cerrar la última.
2. **Una sola descripción de cada ventana:** terminal, Ajustes y Acerca de
   se describen una vez (`windows::describe`); `main` y las ventanas nuevas
   la usan.
3. **Con el parche:** New Window, las ventanas de perfil, Ajustes, Acerca de
   y Edit Session se abren en el mismo proceso, y Quit cierra todas. Sin el
   parche sigue un proceso por ventana.
4. **Lo que gana la ventana compartida:** el historial de pegado es común a
   todas las ventanas del proceso.
5. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla en los dos modos del kit. `./dev.sh check`
limpio en los dos; 40 tests de la app y 1 del bloque. Un gap nuevo: BK-032,
los avisos de `Timers` sobreviven a su ventana.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Kit parcheado: ⌘N y ⌘, abren dos ventanas más en el mismo proceso (3 ventanas, 1 proceso) | ✓ | — |
| Pegar `echo compartido` en la primera y ⇧⌘H en la segunda: la entrada está | ✓ | `fase20-historial-compartido.png` |
| Cerrar Ajustes y una ventana de terminal: el proceso sigue, la shell de la ventana cerrada termina y la otra ventana responde | ✓ tras corregir dos pánicos: el de `close_window` (en el parche 0008) y el del aviso de E/S de la ventana cerrada (BK-032) | — |
| Quit con dos ventanas: se cierran las dos y el proceso termina | ✓ | — |
| Kit tal cual: ⌘N abre un segundo proceso y Quit cierra los dos | ✓ | — |

**No probado:** cerrar una ventana con temporizadores pendientes (campana,
deshacer cierre) en vuelo; la ventana hotkey, que sigue siendo un proceso
aparte.

**Difiere de iTerm2:**

- Open Quickly y Undo Close siguen sin ver las otras ventanas; tampoco se
  pueden mover pestañas entre ventanas.

## Fase 21 — Método de entrada

### Plan

1. **Parche 0009:** el runner enlaza `zwp_text_input_v3`, publica el texto en
   composición y entrega el confirmado como una tecla (BK-033).
2. **CurseTerm:** dibuja la composición subrayada en el cursor de la sesión
   activa y dice al método de entrada dónde está el cursor.
3. **Prueba** con un método de entrada de prueba (`vime`, cliente de
   `zwp_input_method_v2` en el laboratorio) y documentación en los dos
   idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos del kit; 40 tests de la app y 1 del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `echo ` y composición `ñand`: aparece subrayada en el cursor y la shell aún no la recibe | ✓ tras enlazar la seat inicial; sctk solo avisa de las nuevas | `fase21-composicion.png` |
| Confirmar `ñandú`: la shell recibe el texto y la composición desaparece | ✓ | — |

**No probado:** que el compositor reciba `set_cursor_rectangle` (la traza no
llegó a mostrarlo); un método de entrada real como fcitx5 o IBus; la
composición en los campos de texto de las hojas.

**Difiere de iTerm2:** la composición se dibuja encima de la línea, sin
apartar el texto que hay a la derecha del cursor.

## Fase 22 — Selección primaria

### Plan

1. **Parche 0010:** la selección primaria (`zwp_primary_selection`) con el
   mismo `Clipboard` del kit y `WindowControls::primary_selection()` (BK-034).
2. **CurseTerm:** al soltar el ratón con texto seleccionado, lo publica; el
   botón central pega la primaria en la sesión.
3. **Prueba** con `wl-copy --primary` y `wl-paste --primary`.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos del kit; 40 tests de la app y 1 del bloque.

| Prueba | Resultado |
|---|---|
| Doble clic en `primaria`: `wl-paste --primary` da `primaria` | ✓ |
| `wl-copy --primary "echo desde-fuera"` y botón central: la shell recibe la orden | ✓ |

**No probado:** el botón central cuando el programa pide el ratón (se le
manda el clic y no se pega nada, como en iTerm2); la primaria en los campos
de texto de las hojas.

## Fase 23 — Copy with Styles

### Plan

1. **Parche 0011:** `WindowControls::copy_rich(texto, html)` ofrece
   `text/html` además del texto plano (BK-035).
2. **Edit › Copy with Styles (⌥⌘C):** convierte la selección en HTML con los
   colores resueltos de cada celda, negrita, cursiva y subrayado.
3. **Prueba** con `wl-paste -l` y `wl-paste -t text/html`.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos del kit; 41 tests de la app (el de HTML solo con el kit
parcheado) y 1 del bloque.

| Prueba | Resultado |
|---|---|
| `rojo` en negrita roja y ` <normal>`, triple clic y ⌥⌘C: hay `text/html` con un `span` en negrita y color `#cb4b16`, y `<normal>` escapado; el texto plano es `rojo <normal>` | ✓ tras guardar el modo oscuro en `Handles`: `theme::resolved()` aborta fuera del contexto de Freya (BK-017) |
| `verde` subrayado: el `span` lleva `text-decoration:underline` | ✓ |
| ⌘C después: solo quedan los tipos de texto | ✓ |

**No probado:** pegar en una app que lea `text/html` (no hay ninguna en el
laboratorio); selecciones en bloque.

**Difiere de iTerm2:** iTerm2 copia RTF; CurseTerm copia HTML, el formato
con formato que entienden las apps de Linux.

## Fase 24 — Instant Replay

### Plan

1. **Grabación:** el bucle de E/S guarda una foto de la pantalla visible
   (celdas y cursor) tras la salida, como mucho una cada medio segundo y las
   120 últimas, que cubren al menos el último minuto de actividad.
2. **View › Start Instant Replay (⌥⌘B):** el panel activo muestra la foto
   elegida en lugar de la pantalla; una barra abajo tiene un deslizador, la
   hora de la foto y «Done». ← y → cambian de foto; Escape o Done vuelven al
   presente.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit; 40 y 41 tests de la app y 1
del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `clear; echo uno`, `echo dos` y `echo tres` con un segundo entre cada una; ⌥⌘B muestra la última foto, ← cuatro veces llega a la primera (solo el prompt), con la hora | ✓ tras corregir un pánico: el `peek` del `if let` seguía prestado al escribir el estado | `fase24-repeticion.png` |
| Escape vuelve al presente y `echo presente` escribe en la shell | ✓ | — |

**No probado:** el deslizador con el ratón; una sesión con más de 120 fotos.

**Difiere de iTerm2:**

- Se guardan fotos de la pantalla cada medio segundo, no cada cambio; la
  memoria se limita por número de fotos y no hay ajuste para cambiarlo.
- Solo se reproduce la pantalla visible, sin el historial de cada momento.

## Fase 25 — Toolbelt

### Plan

1. **View › Toolbelt (⇧⌘B):** un panel a la derecha de la ventana con tres
   herramientas:
   - **Jobs:** los procesos de la sesión activa (los de su sesión de
     terminal, sin la shell), leídos de `/proc`; «Terminate» manda `SIGTERM`
     al elegido.
   - **Paste History:** el historial de pegado; pulsar una entrada la pega.
   - **Recent Directories:** los directorios por los que pasaron las
     sesiones de la ventana; pulsar uno escribe `cd` con la ruta escapada.
2. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit; 41 y 42 tests de la app y 1
del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `sleep 300 &`, `cd /tmp`, `cd /root` y pegar `echo pegado`; ⇧⌘B: Jobs lista `sleep` con su PID, Paste History la entrada y Recent Directories `/root` y `/tmp` | ✓ tras repartir el alto entre las tres tablas; con alto fijo la última quedaba cortada | `fase25-toolbelt.png` |
| Elegir `sleep` y Terminate: el proceso termina y la shell avisa «Terminated» | ✓ | — |
| Pulsar `/tmp`: se teclea `cd /tmp`; pulsar `echo pegado`: se pega | ✓ | — |

Los clics de prueba sin pausa entre bajar y subir el botón no seleccionaban
filas de `Table`; con 150 ms sí. Es un límite del ratón virtual del
laboratorio, no se nota con un ratón real.

**No probado:** procesos de otra pestaña (Jobs sigue a la sesión activa);
rutas con espacios en Recent Directories.

**Difiere de iTerm2:** no hay notas, ni la herramienta de capturas, ni la de
órdenes recientes; las herramientas no se pueden reordenar.

## Fase 26 — Barra de estado

### Plan

1. **Profiles › Session › Status bar:** con la opción activa, la ventana
   muestra bajo los paneles una barra con datos de la sesión activa: el
   directorio, el proceso en primer plano, la rama de git del directorio, el
   nombre del equipo y la hora, que se actualiza cada segundo.
2. **Rama de git:** se busca `.git` subiendo desde el directorio y se lee
   `HEAD` (también si `.git` es un archivo `gitdir:`), sin ejecutar `git`.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit; 42 y 43 tests de la app y 1
del bloque.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `status_bar = true`, `cd /tmp/repo-demo; sleep 30` con un `.git/HEAD` en `rama-demo`: la barra muestra `/tmp/repo-demo`, `sleep`, `rama-demo`, `vm` y la hora | ✓ | `fase26-barra-de-estado.png` |
| La hora pasa de 00:34:09 a 00:34:11 entre dos capturas; unos 5 eventos por segundo en reposo | ✓ tras dejar un solo temporizador: uno por render habría multiplicado las cadenas con cada salida | — |

**No probado:** la opción desde Ajustes (solo desde el `.domain`); un
repositorio real con `git worktree` fuera del test de `git_branch`.

**Difiere de iTerm2:** los componentes son fijos; no hay editor de la barra,
ni CPU, memoria o red, ni posición arriba.

## Fase 27 — Cursor del perfil

### Plan

1. **Profiles › Text › Cursor:** Box, Vertical Bar o Underline, y
   **Blinking cursor**. Es el estilo por omisión de alacritty
   (`default_cursor_style`): una app que pide otro con DECSCUSR (`\e[2 q`)
   gana, y `\e[0 q` vuelve al del perfil.
2. **Parpadeo:** solo en el panel activo de la ventana con foco. Un único
   temporizador por ventana repinta cada `TERMINAL.cursor_blink_ms` (500 ms),
   el mismo mecanismo que el reloj de la barra de estado.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `cursor = "vertical_bar"` y `blinking_cursor = true`: barra vertical; seis capturas cada 250 ms alternan entre dos imágenes | ✓ | `fase27-cursor-parpadeo.png` |
| `printf '\e[2 q'`: bloque fijo, cuatro capturas iguales; `printf '\e[0 q'`: vuelve la barra parpadeante | ✓ | — |
| Ajustes › Profiles › Text muestra Cursor y Blinking cursor; apagar el interruptor escribe `blinking_cursor = false` y el terminal, en otro proceso, deja de parpadear | ✓ | `fase27-ajustes-cursor.png` |

**No probado:** elegir otro tipo en el desplegable (solo desde el `.domain`).

**Difiere de iTerm2:** escribir no reinicia el parpadeo con el cursor
visible; no hay color de cursor propio del perfil (sale del esquema), ni
guía de cursor (Cursor Guide).

## Fase 28 — Texto al iniciar

### Plan

1. **Profiles › General › Send text at start:** el texto se escribe en la
   shell, seguido de Intro, cada vez que arranca una sesión del perfil:
   ventana, pestaña, panel o reinicio.
2. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `initial_text = "echo hola-inicio"`: la primera pestaña y una nueva (⌘T) muestran `hola-inicio` tras el primer prompt | ✓ | `fase28-texto-al-iniciar.png` |

**No probado:** editar el campo desde Ajustes (solo desde el `.domain`); un
panel nuevo y el reinicio al terminar la sesión.

**Difiere de iTerm2:** el eco del texto sale antes del primer prompt, porque
se escribe en cuanto existe el pty, sin esperar a que la shell lo pida.

## Fase 29 — Ajustes General y Pointer

### Plan

1. **Settings › General › Selection › Copy to clipboard on selection:**
   soltar el ratón tras seleccionar copia al portapapeles y al historial de
   pegado, además de la selección primaria. Activado por omisión, como en
   iTerm2.
2. **Settings › Pointer › Focus follows mouse:** al entrar el puntero en un
   panel, ese panel recibe el foco. Apagado por omisión.
3. Los dos se leen en el momento del evento, no al pintar: un panel que no
   se repinta no se queda con el valor viejo.
4. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Arrastrar sobre una línea: el portapapeles pasa de `vacio` a `copia-al-seleccionar`; con `copy_on_select = false` se queda en `vacio` | ✓ | — |
| ⌘D y `focus_follows_mouse = true`: mover el puntero al panel izquierdo y escribir lo escribe allí; con la opción apagada, el texto sigue en el panel activo | ✓ tras leer la opción al vuelo: con el valor capturado, el panel inactivo al cambiarla seguía con el viejo | `fase29-foco-sigue-al-raton.png` |
| Ajustes muestra General, Appearance, Profiles y Pointer; el interruptor de General escribe `copy_on_select = true` | ✓ | `fase29-ajustes-general.png` |

**Corregido de paso:** `instant-replay` estaba dos veces en los `.ftl` y
Fluent lo avisaba al arrancar.

**No probado:** el interruptor de Pointer (solo desde el `.domain`).

**Difiere de iTerm2:** el foco solo pasa entre paneles de una ventana; el
foco entre ventanas lo decide el compositor. Los iconos de General y
Pointer no son los de iTerm2 (BK-024). Faltan el resto de opciones de los
dos paneles.

## Fase 30 — Guía del cursor

### Plan

1. **View › Show Cursor Guide (⌥⌘;):** una franja translúcida del color del
   cursor cruza la línea del cursor. Es por sesión, como Show Timestamps.
2. **Profiles › Colors › Cursor guide:** el valor con que empiezan las
   sesiones nuevas del perfil.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| ⌥⌘; tras `ls /`: el píxel de la línea del cursor pasa de (0, 43, 54) a otro más claro y la línea de encima no cambia | ✓ con alfa 0.25; con 0.15 apenas se distinguía | — |
| `cursor_guide = true` en el perfil: la primera sesión ya sale con la guía; ⌥⌘; la quita | ✓ | `fase30-guia-del-cursor.png` |

**No probado:** la casilla del menú en D-Bus y el interruptor de Ajustes
(solo desde el `.domain`).

**Difiere de iTerm2:** el color es el del cursor del esquema, no uno propio
del perfil; cambiar la opción del perfil no afecta a las sesiones abiertas.

## Fase 31 — Directorios recientes

### Plan

1. **Session › Open Recent Directories… (⌥⌘/):** el selector de Paste History
   con los directorios recientes de la ventana, el más reciente primero.
2. **Al elegir uno** se escribe su ruta escapada para la shell en la línea de
   órdenes, sin `cd` ni Intro. Es lo que hace iTerm2:
   `DirectoriesPopup.m` devuelve `shouldEscapeShellCharacters = YES` y
   `PTYSession.m` (`popupInsertText:`) inserta el texto.
3. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `cd '/tmp/dir con espacio'`, `cd /var`, `cd` y ⌥⌘/: la lista muestra `/root`, `/var` y `/tmp/dir con espacio` | ✓ | `fase31-directorios-recientes.png` |
| Escribir `ls `, filtrar «espacio» e Intro: la línea queda `ls /tmp/dir\ con\ espacio` y se ejecuta sin error | ✓ | — |

**No probado:** el ítem del menú global en D-Bus.

**Difiere de iTerm2:** la ruta se escapa siempre; iTerm2 solo lo hace cuando
el cursor está en el prompt. La lista es de la ventana y no se guarda al
cerrarla, ni se ordena por frecuencia de uso.

## Fase 32 — Borrar hasta una marca o la selección

### Plan

Los tres ítems de «Edit» de iTerm2 (`sources/MainMenu/Base.lproj/MainMenu.xib`)
que faltaban:

1. **Clear to Last Mark (⌥⌘L):** borra desde la última marca por encima del
   cursor hasta el cursor; la línea del cursor sube. La línea marcada se
   queda salvo que sea la justo encima del cursor, como `clearToLastMark` en
   `sources/VT100Screen/VT100Screen.m`. Solo activo si hay esa marca.
2. **Clear to Start of Selection (⌘L):** lo mismo desde la primera línea de la
   selección (`clearToStartOfSelection:` en `PseudoTerminal.m`).
3. **Clear Instant Replay (⌥⌘K):** descarta las fotos guardadas y sale de la
   repetición si estaba abierta.
4. Las marcas, horas, imágenes y resaltados de las líneas borradas se
   olvidan y los de debajo suben con el texto (`Extras::forget_lines`, que
   sustituye a `forget_lines_above`).
5. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual: no necesita parche.
`./dev.sh check` limpio en los dos modos del kit.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `echo A`, ⇧⌘M, `seq 5` y ⌥⌘L: queda la línea `seq 5` con su marca y desaparecen 1–5; el prompt sube | ✓ | `fase32-borrar-hasta-la-marca.png` |
| Después `echo B`, seleccionar la línea `A` y ⌘L: quedan `echo A` y el prompt; la marca de `seq 5` desaparece con su línea | ✓ | — |
| ⌥⌘B abre la repetición, ⌥⌘K la cierra y ⌥⌘B ya no hace nada; la app sigue viva | ✓ | — |

**No probado:** los ítems desactivados en el menú global de D-Bus.

**Difiere de iTerm2:** solo se borran líneas visibles. Si la marca o la
selección empiezan en el historial, se borra desde la primera línea de la
pantalla, porque alacritty no permite quitar líneas sueltas del historial.

## Fase 33 — Guardar el contenido

### Plan

1. **Shell › Save Contents…:** como Save Selected Text, pero guarda el búfer
   entero (historial y pantalla) sin las líneas vacías del final. Sin atajo,
   como en `MainMenu.xib` de iTerm2.
2. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado, para pulsar el ítem por el
menú de D-Bus. `./dev.sh check` limpio en los dos modos del kit.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `seq 80`, Save Contents… por D-Bus: sale la hoja con el campo de ruta | ✓ | `fase33-guardar-contenido.png` |
| Escribir la ruta e Intro: el archivo tiene 84 líneas, de `seq 80` al último prompt, y acaba en un salto de línea | ✓ | — |

**No probado:** con el kit tal cual el ítem solo es alcanzable desde un menú
global (sin atajo).

**Difiere de iTerm2:** la ruta se escribe en un campo; no hay selector de
archivos (BK-023).

## Fase 34 — Disposiciones de ventana

### Plan

Window › Arrangements de iTerm2 (`sources/iTermController/iTermController.m`:
`saveWindowArrangementForWindow:`, `loadWindowArrangementWithName:` y
`tryOpenArrangement:named:asTabsInWindow:`):

1. **Save Current Window as Arrangement… (⌥⇧⌘S):** pide un nombre y guarda
   las pestañas de la ventana: título, árbol de paneles con sus proporciones
   y, en cada panel, perfil y directorio actual. Un nombre repetido sustituye
   al anterior.
2. **Restore Window Arrangement › nombre:** abre una ventana nueva con esas
   pestañas (en el mismo proceso con el parche 0008; si no, otro proceso con
   `--arrangement nombre`).
3. **Restore Window Arrangement as Tabs › nombre:** añade las pestañas a la
   ventana actual.
4. Las disposiciones se guardan en el `.domain` de la app, como iTerm2 en sus
   preferencias. `arrangement.rs` tiene la captura y la reconstrucción, con
   tests.
5. **Prueba** en sway sin pantalla y documentación en los dos idiomas.

### Resultado (2026-11-06)

Probado en sway sin pantalla en los dos modos del kit. `./dev.sh check`
limpio en los dos modos; 44 tests de la app.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| ⌘D, `cd /tmp`, ⌘T, `cd /var` y ⌥⇧⌘S «trabajo»: el `.domain` guarda una pestaña con un `split` `row` 0.5/0.5 (`/root` y `/tmp`) y otra en `/var` | ✓ con el kit tal cual hay que pulsar el campo antes de escribir (BK-029) | — |
| `curseterm --arrangement trabajo`: dos pestañas; la primera dividida, con `pwd` `/root` y `/tmp`; la segunda en `/var` | ✓ | `fase34-disposicion-restaurada.png` |
| Kit parcheado, menú D-Bus: «as Tabs › trabajo» pasa de 1 a 3 pestañas; «Restore › trabajo» abre una segunda ventana en el mismo proceso | ✓ | — |

**No probado:** una disposición con un perfil borrado (sale con el perfil por
omisión).

**Difiere de iTerm2:** no hay Save Window Arrangement (⇧⌘S) para todas las
ventanas, porque cada ventana solo conoce sus pestañas; no se guardan ni el
tamaño ni la posición (Wayland no deja colocar ventanas), ni el contenido,
ni hay disposición por omisión al arrancar, ni importar o exportar a un
archivo.

## Fase 35 — Anotaciones

### Plan

Edit › Marks and Annotations y View › Show Annotations de iTerm2, copiados de
`sources/Annotations/PTYNoteViewController.m`, `sources/TerminalView/PTYNoteView.m`,
`PTYSession.m` (`addNoteAtCursor`, `nextMarkOrNote:`) y
`iTermTextDrawingHelper.m` (`drawNoteRangesOnLine:`):

1. **Modelo.** Una anotación es un rango de celdas (inicio y fin en líneas
   absolutas) con su texto, anclado como las marcas: sigue al scroll y al
   redimensionado y se borra con el búfer.
2. **Add Annotation at Cursor (⌥⌘M):** anota la selección inteligente en el
   cursor, después la de la celda siguiente, la de la anterior y, si no hay
   ninguna, la línea entera. **Annotate Selection:** anota la selección.
3. **Subrayado:** una línea amarilla de 1 punto, 2 puntos por encima del pie
   de cada celda anotada.
4. **Burbuja** (`note_view.rs`): rectángulo de radio 5 con flecha de 7×7
   hacia el final del rango, 40 puntos desde su borde izquierdo si cabe;
   arriba del texto si no cabe debajo. Fondo (250, 250, 250) y borde negro al
   10 % en claro; (50, 50, 52) y blanco al 40 % en oscuro; sombra negra al
   15 %, 2 puntos abajo y 8 de desenfoque. Texto de 12 puntos y 250 de ancho
   que crece en alto; papelera de 17 puntos arriba a la derecha.
5. **Edición** con el bloque `views::text_view` (BK-036): la nota nueva toma
   el foco; Enter parte la línea; Escape devuelve el teclado al terminal.
6. **Papelera:** pide «Really remove annotation?» (OK, Cancel) y la borra.
7. **Next/Previous Annotation (⌥⇧⌘↓/↑):** recorre por posición, da la vuelta,
   la muestra si estaba oculta, la centra si no está en pantalla y resalta el
   fondo hacia (0.85, 0.92, 1.0), que vuelve en 0,75 s con la curva al
   cuadrado.
8. **Show Annotations (⌘\\):** si alguna está visible las oculta todas; si
   no, las muestra; fundido de 0,25 s. La casilla del menú sigue a «hay
   anotaciones», como en iTerm2.
9. **Prueba** en sway sin pantalla en los dos modos del kit y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla en los dos modos del kit. `./dev.sh check`
limpio en los dos modos; 44 tests de la app.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `echo palabra` y ⌥⌘M con el cursor detrás: subraya `palabra` (columnas 20–26), la burbuja toma el foco y lo tecleado no llega a la shell | ✓ | `fase35-anotacion.png` |
| Enter dentro de la nota parte la línea y la burbuja crece; Escape devuelve el teclado a la shell | ✓ | — |
| `seq 1 120` y ⌥⇧⌘↑: vuelve a la nota, la centra y el fondo azul se desvanece | ✓ | `fase35-anotacion.png` |
| ⌘\\ dos veces: se funde a 0 y vuelve a 1 | ✓ | `fase35-mostrar-anotaciones.png` |
| Papelera: aviso «Really remove annotation?»; OK borra la burbuja y el subrayado | ✓ | `fase35-confirmar.png` |
| Kit parcheado, menú D-Bus: Annotate Selection activo solo con selección; anota «seleccion de varias» con la flecha al final | ✓ | — |
| Arrastrar dentro de la burbuja selecciona texto de la nota, no del terminal | ✓ | — |
| Apariencia oscura: fondo (50, 50, 52) y borde blanco | ✓ | `fase35-anotacion-oscura.png` |

**No probado:** varias anotaciones en la misma línea ni la vuelta de Next al
principio; una nota en un panel sin foco.

**Falla:** con el historial lleno, las líneas que salen por arriba desplazan
las anotaciones, igual que las marcas: `alacritty_terminal` no cuenta las
líneas que descarta. Se vio con `scrollback_lines = 50`.

**Difiere de iTerm2:** la burbuja no se redimensiona con el ratón ni se puede
arrastrar; no tiene barra de desplazamiento; sin destello de «vuelta» al dar
la vuelta Next o Previous; la selección inteligente es la de CurseTerm (URL o
palabra), no las reglas de iTerm2; las anotaciones no se guardan al cerrar.

## Fase 36 — Composer

### Plan

View › Show Composer (⇧⌘.) de iTerm2, copiado de
`sources/Composer/iTermComposerManager.m`,
`iTermMinimalComposerViewController.m` y su `.xib`, `ComposerTextView.swift`,
`iTermStatusBarLargeComposerViewController.xib` y `PTYSession.m`
(`frameForDesiredHeight:`, `composerManagerAppearance:`, `sendCommand:`):

1. **Modelo** (`composer.rs`): cada sesión guarda si el Composer está abierto
   y su texto. ⇧⌘. lo abre o lo cierra conservando el texto.
2. **Caja:** arriba del panel, a 20 puntos de cada lado (mínimo 217 de ancho),
   135 de alto y nunca más que el panel menos 8. Dentro, a 9 puntos, un
   material «sheet»: fondo de ventana translúcido sobre el terminal
   desenfocado, radio 6 y borde gris de 1 punto. La apariencia sigue al fondo
   del terminal (brillo percibido 0,30/0,59/0,11 bajo 0,5 = oscuro).
3. **Texto** con el bloque `views::text_view` (BK-036): fuente, tamaño, color
   y cursor del perfil; margen de 5 puntos; desplazamiento vertical.
4. **Fila inferior** de 19 puntos con «Press ⇧↩ to send.» a la derecha, que se
   oculta por debajo de 180 puntos; botón de cerrar de 24×23 arriba a la
   derecha.
5. **Teclas** (`standardActions`): ↩ parte la línea; ⇧↩ envía todo, cada línea
   con su retorno, y cierra sin fundido; ⌥⇧↩ envía la orden del cursor (su
   línea y las unidas con `\` al final) y la quita del texto; Escape y el
   botón cierran con un fundido de 0,125 s y conservan el texto.
6. **Prueba** en sway sin pantalla en los dos modos del kit y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla en los dos modos del kit. `./dev.sh check`
limpio en los dos modos; 45 tests de la app (uno nuevo para la orden del
cursor).

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| ⇧⌘. (en `wtype`, Super+Mayús+`greater`): se abre arriba con el foco, oscuro sobre el esquema oscuro y lo tecleado no llega a la shell | ✓ | `fase36-composer.png` |
| Un `for` de cuatro líneas y ⇧↩: la shell lo ejecuta (`uno`, `dos`, `tres`) y la caja se cierra | ✓ | `fase36-composer-enviado.png` |
| ⌥⇧↩ con el cursor en la primera línea: envía `echo uno` y la quita; la caja sigue abierta | ✓ | — |
| Escape y el botón de cerrar: se cierra, Escape no llega a la shell y ⇧⌘. vuelve con el texto | ✓ | — |
| El «>» de ⇧⌘> no entra en el texto | ✓ | — |
| Kit parcheado, menú D-Bus: View › Show Composer lo abre | ✓ | — |

**No probado:** un esquema claro (material claro), el fundido al cerrar en
una captura, pegar con ⌘V dentro, el IME y que el desplazamiento siga al
cursor con mucho texto.

**Difiere de iTerm2:** sin asas para cambiar la altura (siempre 135) ni
arrastre; sin ⌥↩ (poner en cola), autocompletado, historial, ayuda, IA,
varios cursores ni enviar solo la selección; sin Auto Composer ni Composer
en la barra de estado; el texto termina antes del botón de cerrar porque
Freya da el puntero al desplazamiento que tiene debajo; tras ⌥⇧↩ el cursor
no vuelve al inicio de la orden quitada.

## Fase 37 — Marcas con nombre

### Plan

Edit › Set Named Mark… (⌥⇧⌘M) y Edit › Named Marks de iTerm2
(`PseudoTerminal.m -addNamedMark:`, `BookmarkDialogViewController.swift`,
`PTYSession.m -saveScrollPositionWithName:` y `-scrollToNamedMark:`,
`iTermApplicationDelegate.m` para el submenú):

1. **Diálogo «Enter Mark Name»** con la fecha y la hora
   (`yyyy-MM-dd HH:mm:ss`) como nombre por omisión; OK y Cancel; un nombre
   vacío no hace nada.
2. **Marca** en la primera línea visible (el triángulo azul de las marcas) y
   una anotación «Named Mark nombre» a lo ancho de esa línea, visible y sin
   foco.
3. **Submenú Named Marks:** una entrada por marca en el orden en que se
   pusieron; elegirla la centra si no está en pantalla.
4. Las marcas con nombre se anclan como las demás y se borran con el búfer.
5. **Prueba** en sway sin pantalla y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `seq 1 60`, subir con la rueda y ⌥⇧⌘M: el diálogo trae `2026-11-06 03:23:49`; con Enter, triángulo en la línea `18` y la nota «Named Mark 2026-11-06 03:23:49» | ✓ | `fase37-marca-con-nombre.png` |
| Menú D-Bus: Named Marks lista `2026-11-06 03:23:49` | ✓ | — |
| Tras `seq 1 80`, elegir la marca en el submenú vuelve a la línea `18` | ✓ | — |

**No probado:** con el kit tal cual (sin menú por D-Bus y el campo toma el
foco solo al pulsarlo, BK-029); varias marcas con el mismo nombre.

**Difiere de iTerm2:** sin la herramienta Named Marks del Toolbelt ni
renombrar o borrar desde ella; al saltar no destella la marca; las marcas
con nombre no van en Open Quickly.

## Fase 38 — Find URLs

### Plan

Edit › Find › Find URLs (⌥⌘U) de iTerm2 (`PseudoTerminal.m -findUrls:`,
`findUrlsRegex` de `iTermAdvancedSettingsModel.m`,
`PTYTextView+ARC.m -convertVisibleSearchResultsToContentNavigationShortcutsWithAction:`,
`ContentNavigationShortcut.swift` e `iTermShortcutNavigationModeHandler` en
`iTermCopyModeHandler.m`):

1. **Búsqueda:** la misma expresión de iTerm2 sobre las líneas visibles, con la
   búsqueda de `alacritty_terminal`, que sigue las líneas partidas. Los
   resultados se resaltan como los de Find.
2. **Atajos** (`hints.rs`): 1 a 9 y después letras, tantas como pida el número
   de resultados (A…Z; AA, AB… si hay más de 26).
3. **Etiquetas** (`hint_view.rs`): cápsula del color de acento con borde blanco
   al 95 % y sombra negra al 40 %, texto blanco de 11 puntos; a la izquierda del
   resultado, a 2 puntos, y alineada arriba si la fila es más alta que ella.
   Entran escalando 0,25 → 1,3 → 1 en 0,2 s.
4. **Teclas** (`validKeyEquivalentForEvent:`): con ⌘ o ⌃ pasan; Escape sale;
   prefijo más tecla, o la tecla sola, que coincida abre la URL (con ⌥ la
   copia); un prefijo deja la parte tecleada al 50 % y quita las que no
   encajan; otra tecla va al terminal. Un clic sale.
5. **Salida:** la elegida estalla (×2, fundido, media vuelta) y las demás se
   disuelven (×0,1, fundido) en 0,2 s; después se quita el resaltado.
6. **Prueba** en sway sin pantalla y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit tal cual. `./dev.sh check` limpio en
los dos modos; un test nuevo para los atajos y el prefijo.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Dos líneas con tres URL y ⌥⌘U: resaltadas, con 1, 2 y 3 a su izquierda | ✓ | `fase38-find-urls.png` |
| La URL del comando, partida en dos líneas, se encuentra entera | ✓ | — |
| Doce URL: atajos 1–9, A, B y C | ✓ | — |
| ⌥5 copia `https://github.com/gnachman/iTerm2` (`wl-paste`) y sale | ✓ | — |
| `q`, que no es atajo, llega a la shell y el modo sigue | ✓ | — |
| Escape y un clic salen sin escribir nada en la shell | ✓ | — |

**No probado:** que «1» abra el navegador (en el laboratorio no hay ninguno;
la orden sale sin error), atajos de dos letras con la app (sí en el test) y
el kit parcheado.

**Difiere de iTerm2:** al copiar no se selecciona el texto ni sale el aviso
«Copied»; la animación de estallar y disolver no se ha visto en una captura;
la expresión no se puede cambiar en Ajustes. La cápsula y su texto se
dibujan en un lienzo porque Freya pinta encima de ellos el lienzo del
terminal, que está más hondo en el árbol.

## Fase 39 — Selecciones inteligentes y elegir un resultado

### Plan

Edit › Find › Find All Smart Selection Matches (⌥⇧⌘U) y Pick Result to Open
(⌥⇧⌘O) de iTerm2 (`PTYSession.swift -smartSelectAllVisible`,
`PTYSession.m -regularExpressonForNonLowPrecisionSmartSelectionRulesCombined`,
`SmartSelectionRules.plist` y `PTYTextView.m -revealContentNavigationShortcuts:`),
sobre las etiquetas de la fase 38:

1. **Selecciones inteligentes:** las reglas de precisión normal o más unidas
   en una expresión: `a::b`, rutas, texto entre comillas, nombres con puntos,
   `mailto:`, `@selector(...)`, correos, URL HTTP, `ssh:` y `telnet:`. La acción
   es copiar; con ⌥ el texto va a la shell.
2. **Elegir un resultado:** con una búsqueda en curso, los resultados visibles
   reciben etiquetas; la acción es abrir (una ruta relativa, desde la carpeta
   de la shell); con ⌥ se copia. El campo de búsqueda suelta el foco y el
   resaltado de Find se queda al salir. Solo está activo con una búsqueda.
3. **Prueba** en sway sin pantalla y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos; un test nuevo comprueba que las dos expresiones compilan en la
búsqueda de `alacritty_terminal`.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Una línea con `/etc/hosts`, `foo::bar`, un correo, `"entre comillas"` y `com.example.App` y ⌥⇧⌘U: cinco etiquetas | ✓ | `fase39-selecciones-inteligentes.png` |
| `2` copia `foo::bar` (`wl-paste`) | ✓ | — |
| ⌥3 escribe `yo@ejemplo.org` en la shell | ✓ | — |
| ⌘F `conf` y ⌥⇧⌘O: etiqueta 1 sobre `conf`; el campo suelta el foco | ✓ | `fase39-elegir-resultado.png` |
| `1` llama a `xdg-open` (el laboratorio no lo tiene: queda un aviso) y el resaltado de Find sigue | ✓ | — |
| Menú D-Bus: Pick Result to Open activo solo con búsqueda | ✓ | — |

**No probado:** con el kit tal cual; abrir de verdad una ruta o una URL.

**Difiere de iTerm2:** las reglas no se editan en Ajustes; el motor de
búsqueda no tiene mirada atrás, así que la regla de URL HTTP es la rama que
no admite `)` al final, `[[:letter:]]` y `[[:number:]]` pasan a `\p{L}` y
`\p{N}`, y los límites de palabra son ASCII; al copiar no se selecciona el
texto ni sale «Copied». Sin Select Matches (varias selecciones a la vez).

## Fase 40 — Alertas de marca

### Plan

Edit › Marks and Annotations › Alerts de iTerm2 (`MainMenu.xib`,
`PseudoTerminal.m -toggleAlertOnNextMark:`, `PTYSession.m
-screenDidAddMark:alert:completion:` y `-showMarkSetAlert`,
`iTermApplicationDelegate.m -markAlertAction`):

1. **Alert on Next Mark (⌥⌘A):** casilla por sesión; la próxima marca avisa y
   la apaga.
2. **Alert on Marks in Offscreen Sessions:** ajuste de la app (apagado por
   defecto); una marca en una pestaña que no se ve también avisa.
3. **Cómo avisar** (radio, ajuste de la app): Show Modal Alert Box (alerta
   «Alert» con Reveal y OK), Post Notification (por defecto; «Mark Set»,
   «Session … #n had a mark set.», y pulsarla muestra la sesión) y Play a
   Sound (éxito o error según el código de salida del comando anterior, que
   ahora se lee de `OSC 133;D;código`).
4. **Notificaciones y sonidos** en el bloque `notifications` de
   `barrier-kit-next` (BK-026).
5. **Prueba** genérica, sin Malaysia: sway sin pantalla con `mako` como
   servidor de notificaciones, `dbus-monitor` para ver las llamadas y un
   `canberra-gtk-play` falso que apunta lo que se le pide.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos; el test de `OSC 133` comprueba ahora el código de salida. Las
marcas se hicieron a mano con
`printf '\033]133;D;código\007\033]133;A\007'`.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Menú D-Bus: Alerts con las dos casillas y los tres radios, Post Notification marcado | ✓ | — |
| ⌥⌘A marca la casilla; la marca siguiente la desmarca | ✓ | — |
| Con ⌥⌘A, una marca llama a `Notify` con «Mark Set», «Session root@vm: ~ #1 had a mark set.», la acción `default`/«Reveal» y `desktop-entry` | ✓ | — |
| Con las marcas fuera de la vista, una marca en la pestaña 1 vista desde la 2 sale en mako | ✓ | `fase40-notificacion.png` |
| `makoctl invoke` (`ActionInvoked`) vuelve a la pestaña 1 | ✓ | `fase40-mostrar.png` |
| Play a Sound: código 1 pide `dialog-error` y código 0 `complete` | ✓ | — |
| Show Modal Alert Box: «Alert», «Mark set in session “root@vm: ~.”», OK y Reveal; Reveal vuelve a la pestaña 1 | ✓ | `fase40-alerta.png` |

**No probado:** con el kit tal cual; el token de activación (mako 1.11 no
manda `ActivationToken`, así que la ventana no se trae al frente); el sonido
de verdad (el laboratorio no tiene audio: `canberra-gtk-play` dice «Not
available»); Malaysia, que aún no tiene servidor de notificaciones.

**Difiere de iTerm2:** no se puede elegir el archivo de sonido
(`successSound`/`errorSound`); no hay indicador de alerta en la esquina del
panel; una sesión restaurada de una disposición no espera a la primera tecla
antes de avisar de marcas fuera de la vista; no se manda al iPhone.

## Fase 41 — Notificaciones del perfil

### Plan

Profiles › Terminal › Notifications de iTerm2
(`ProfilesTerminalPreferencesViewController.m`, `PTYSession.m
-shouldPostUserNotification`, `-notificationsSuppressed`, `-setBell:`,
`-brokenPipeWithError:` y `-screenPostUserNotification:rich:`, `PTYTab.m
-setLabelAttributesForActiveTab:` y `-setLabelAttributesForIdleTab`):

1. **Ajustes** en la pestaña Terminal del perfil, con los valores por defecto
   de iTerm2: Notification Center alerts (apagado), Bell (encendido), Idle y
   New Output (apagados), Session Ended y las de secuencias de escape
   (encendidos) y Suppress alerts from active session (apagado).
2. **Cuándo se envía:** con las alertas encendidas, si no está suprimida y la
   sesión no se ve (otra pestaña) o la ventana no está activa.
3. **Campana, fin de sesión, salida nueva e inactividad**, con los textos de
   iTerm2. Salida nueva avisa una vez por tanda en una pestaña de fondo (no
   con difusión de entrada); inactiva, 3 s después de la última salida
   (`idleTimeSeconds` más 1).
4. **Secuencias de escape:** `OSC 9;texto` («Alert», «Session … #n: texto»;
   un primer campo numérico es el progreso de ConEmu y no avisa) y
   `OSC 1337;Notification=title=…;subtitle=…;message=…` en base64. No miran
   si la sesión se ve, como en iTerm2.
5. **Pulsar** cualquiera muestra la sesión, como en la fase 40.
6. **Prueba** genérica con mako y `dbus-monitor`, sin Malaysia.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos; tests nuevos para `OSC 9` y `1337;Notification` y para la
lógica de salida nueva e inactividad (`activity.rs`).

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| Ajustes › Perfiles › Terminal: sección Notifications con los valores de iTerm2; los interruptores se guardan | ✓ | `fase41-ajustes.png` |
| `OSC 9;hola desde OSC 9` en la pestaña visible: «Alert», «Session root@vm: ~ #1: hola desde OSC 9» | ✓ | `fase41-osc9.png` |
| `OSC 9;4;1;50` no avisa | ✓ | — |
| `1337;Notification` con título, subtítulo y mensaje: «Hecho» y «make» más «todo listo» | ✓ | `fase41-osc9.png` |
| Pestaña de fondo con `sleep 3; echo salida; printf '\a'`: Bell y New Output al llegar, Idle 3 s después | ✓ | `fase41-fondo.png` |
| La pestaña nueva, que se ve, no avisa de su propia salida | ✓ | — |
| `exit` en una pestaña de fondo: «Session Ended», «Session “root@vm: ~” in tab #1 just terminated.» | ✓ | — |
| Con Suppress alerts from active session, un `OSC 9` en la sesión activa no sale; al apagarlo, sí | ✓ | — |

**No probado:** con el kit tal cual; la ventana inactiva como motivo para
avisar (sway sin pantalla no cambia el foco entre ventanas en la prueba);
Malaysia, que aún no tiene servidor de notificaciones.

**Difiere de iTerm2:** los filtros van en la misma sección y no en el
desplegable Filter Alerts; no hay alertas suprimidas por tipo («Suppressed
Alerts…»), ni texto propio de fin de sesión (`sessionEndMessageText`), ni
`simpleNotifications`, ni la imagen de `1337;Notification`; el subtítulo va
en la primera línea del cuerpo; la salida tras cambiar el tamaño de la
ventana no se descarta; no se cuenta en la insignia del Dock ni se manda al
iPhone.

## Fase 42 — Indicadores de sesión

### Plan

Los indicadores de la esquina de arriba a la derecha de iTerm2
(`iTermIndicatorsHelper.m` y `PTYTextView.m -updateIndicators`), para los
estados que CurseTerm ya tiene:

1. **Cuáles**, en el orden de iTerm2 y el primero más a la derecha: panel
   maximizado, recibe la difusión de entrada, coproceso, Alert on Next Mark
   y modo copia.
2. **Dibujo:** cuadros de 26 puntos con radio de un sexto, el símbolo 4
   puntos más pequeño, 4 puntos entre ellos y 5 desde arriba, todo a media
   opacidad; negro con borde gris claro y símbolo blanco sobre un fondo de
   terminal oscuro, y al revés sobre uno claro. Medidas en el token
   `INDICATOR` y símbolos dibujados en el lienzo (BK-024).
3. **Prueba** en sway sin pantalla y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| ⌘D, ⌥⌘A, ⌥⌘I, ⇧⌘↩ y ⇧⌘C: de derecha a izquierda maximizado, difusión, ojo y modo copia | ✓ | `fase42-indicadores.png` |
| ⌥⌘A en el primer panel no marca el segundo: el indicador es de su sesión | ✓ | — |
| Escape sale del modo copia y su indicador se va; Run Coprocess `cat > /dev/null` añade el del coproceso entre la difusión y el ojo | ✓ | `fase42-coproceso.png` |
| Solarized Dark: cuadros negros con símbolo blanco | ✓ | — |

**No probado:** con el kit tal cual; un esquema de fondo claro.

**Difiere de iTerm2:** no hay indicadores de los estados que CurseTerm no
tiene (filtro, salida suprimida, zoom, registro de depuración, entrada
segura, ventana fijada, IA, canales, entrada en espera, capturas); pulsar un
indicador no muestra su ayuda; los extremos de los trazos no son redondos;
los símbolos son dibujos propios, no los SF Symbols.

## Fase 43 — Temas de ventana

### Plan

Settings › Appearance › General › Theme de iTerm2 (`iTermTheme.m`,
`PSMYosemiteTabStyle.m`, `PSMDarkTabStyle.m`, `PSMLightHighContrastTabStyle.m`,
`PSMDarkHighContrastTabStyle.m`, `PSMMinimalTabStyle.m`,
`iTermRootTerminalView.m` y `PseudoTerminal.m`), con la familia de pestañas
clásica, la que iTerm2 usa antes de macOS 26 y siempre en Compact:

1. **Ajuste Theme** con los siete temas en el orden del menú de iTerm2;
   Regular por defecto.
2. **Barra de pestañas** como `PSMTabBarControl`: 24 puntos, pestañas iguales
   que llenan la barra (mínimo 75), cierre a la izquierda solo al pasar el
   ratón, título centrado de 11 puntos (12 en alto contraste), separadores de
   1 punto, línea de arriba en las pestañas no elegidas, sombra de 4 puntos y
   «+» de 24 puntos. Colores de Light, Dark y los dos de alto contraste en
   tokens (`TAB_STYLE_*`).
3. **Regular y Compact** siguen la apariencia del sistema; **Light, Dark** y
   los de alto contraste fuerzan la de la ventana entera, barra de título
   incluida (`use_appearance`).
   Con CSD, la barra de título es la de una ventana de macOS (iTerm2 usa una
   `NSWindow` normal): 28 puntos, título en negrita de 13 puntos centrado,
   fondo `windowBackgroundColor` (236 claro, 50 oscuro), sin línea debajo si
   hay pestañas y la «division view» de 1 punto de `iTermRootTerminalView.m`
   si no. Las pestañas toman ese mismo fondo, así que la elegida continúa la
   barra de título.
4. **Compact y Minimal** (con CSD): sin barra de título; los botones de
   ventana y las pestañas comparten una franja con 75 puntos a la izquierda
   para los botones, y con una sola pestaña una barra falsa del mismo alto con
   el título de 13 puntos centrado. Arrastrar la franja mueve la ventana.
5. **Minimal:** la franja mide 38 puntos y toma el fondo de la terminal; las
   pestañas no elegidas, 0,05 más oscuras (más claras sobre casi negro), sin
   separadores, con la muesca de contorno al 20 % y texto 0,95 o 0,05 según
   el fondo.
6. **Prueba:** sway no acepta CSD (contesta SSD aunque la app pida CSD y con
   `border csd`), así que Compact y Minimal se prueban en weston sin pantalla,
   que no tiene `xdg-decoration`.

### Resultado (2026-11-06)

Probado con el kit parcheado en weston 16 sin pantalla (CSD) y en sway (SSD).
`./dev.sh check` limpio en los dos modos; un test nuevo para los colores de
Minimal.

**Funciona** (capturas en [`capturas/`](capturas/); `fase43-temas.png`, de
arriba abajo: Regular con dos pestañas y con una, Compact con dos y con una,
Minimal con dos y con una, Light, Dark, Light HC y Dark HC):

| Prueba | Resultado | Captura |
|---|---|---|
| Settings › Appearance: menú Theme con los siete temas, Regular marcado | ✓ | `fase43-menu-tema.png` |
| Elegir Dark en sway cambia la barra de pestañas al momento, con el texto de la ventana activa (1,0 y 0,65) | ✓ | — |
| Los nueve casos de `fase43-temas.png` en weston | ✓ | `fase43-temas.png` |
| Minimal: franja de 38 puntos con el fondo de Solarized Dark, botones centrados, muesca alrededor de la pestaña elegida y «+» blanco | ✓ | `fase43-minimal.png` |
| Dark y Dark HC fuerzan la barra de título oscura; Light y Light HC, la clara | ✓ | `fase43-temas.png` |
| Regular: barra de 28 puntos con el título en negrita; la pestaña elegida es del mismo `#ECECEC` que la barra (antes era blanca bajo la barra de 44 px del kit); con una pestaña, la línea de 1 punto | ✓ | `fase43-temas.png` |
| En sway (SSD), Compact y Minimal se quedan con la barra del compositor encima y sus colores de pestaña | ✓ | — |

**No probado:** el paso del ratón (cierre y color de hover) y arrastrar o
hacer doble clic en la franja, porque weston no tiene teclado ni ratón
virtuales; los colores de ventana activa en weston (no da el foco a la
ventana, así que salen los de inactiva); el kit tal cual; Malaysia.

**Difiere de iTerm2:** no están los estilos de macOS 26 (Tahoe); el hover
cambia de golpe, sin la animación de 0,2 s; Regular y Compact no pasan solos
a alto contraste con «Aumentar contraste» del sistema; los divisores de
paneles no toman los colores del tema; Minimal no tiene en cuenta la
transparencia ni la atenuación de ventanas de fondo; no hay colores de
pestaña, números de pestaña, subtítulos ni menú de desbordamiento; la
barra de título y las pestañas usan `windowBackgroundColor` liso, sin el
material translúcido de macOS; los botones de ventana son los del kit.

## Fase 44 — Duplicar, repartir paneles y seleccionar el comando

### Plan

Cuatro entradas de los menús de iTerm2:

1. **Duplicate Tab** (Shell) copia la pestaña entera, con sus paneles,
   perfiles y carpetas (`PseudoTerminal.m -createDuplicateOfTab:`). Antes
   abría una pestaña con la sesión activa sola, que es lo que hace
   **Duplicate Session** (Session, `-duplicateSession:`).
2. **Arrange Split Panes Evenly** (Window) da a cada panel la misma parte, en
   cada nivel (`PTYTab.m -arrangeSplitPanesEvenlyInSplitView:`). Solo está
   activo con paneles.
3. **Select Current Command** (Edit) selecciona en un prompt lo escrito desde
   `OSC 133;B` y, si no, el último comando, de `133;B` a `133;C`
   (`PTYSession.m -textViewRangeOfCurrentCommand`). Ahora se lee `133;B`.
4. **Prueba** en sway y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos; el test de `OSC 133` comprueba ahora `133;B`.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| ⌘D dos veces deja paneles de 1/2, 1/4 y 1/4; Arrange Split Panes Evenly los deja en tercios | ✓ | `fase44-paneles.png` (arriba y en medio) |
| Duplicate Tab abre una pestaña con los mismos tres paneles | ✓ | `fase44-paneles.png` (abajo) |
| Duplicate Session y Arrange Split Panes Evenly salen en sus menús; este último desactivado sin paneles | ✓ | — |
| Con `PS1` que emite `133;B` y `echo hola` escrito: Select Current Command se activa y, tras Copy, el portapapeles tiene lo escrito desde la marca | ✓ | — |
| Lo mismo en el primer panel de una pestaña duplicada | ✓ | — |

**No probado:** con el kit tal cual; el último comando ya ejecutado, que
necesita una shell que emita `133;C`; un comando de varias líneas.

**Difiere de iTerm2:** la selección no se copia sola aunque esté activado
copiar al seleccionar; no se quitan del rango los prompts de continuación
(PS2) ni el prompt de la derecha; Duplicate Tab deja activo el primer panel,
no el que lo estaba; Duplicate Window y Replace
Selection (Base64, JSON) no están.

## Fase 45 — Filter

### Plan

Edit › Find › Filter (⌥⇧⌘F) de iTerm2 (`PseudoTerminal.m -filter:`,
`PTYSession.m -setFilter:`, `AsyncFilter.swift`):

1. **Campo Filter** en una barra bajo la de búsqueda, con el foco al abrirse;
   Escape o el botón la cierran y quitan el filtro.
2. **El panel** enseña solo las líneas lógicas (con su ajuste de línea) que
   tienen el texto, con distinción de mayúsculas solo si el texto lleva
   alguna (smart case), las más nuevas abajo, y se actualiza con la salida.
3. **Indicador Filter** entre Alert y Copy Mode, como en
   `iTermIndicatorsHelper.m`.
4. **Prueba** en sway y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| 60 líneas `linea N`, ⌥⇧⌘F y «linea 1»: quedan `linea 1` y `linea 10`–`19`, con el indicador del filtro | ✓ | `fase45-filtro.png` |
| Con «linea 10» y `(sleep 3; echo linea 1000) &`, la línea nueva aparece sola con el filtro abierto | ✓ | `fase45-vivo.png` (arriba) |
| Escape cierra la barra y vuelve la pantalla normal | ✓ | `fase45-vivo.png` (abajo) |

**Bugs encontrados al probar:** la app se quedaba congelada porque dos
funciones bloqueaban `extras` y luego el terminal, al revés que el hilo de
lectura: `current_command` (fase 44, Select Current Command, que el menú
consulta en cada repintado) y los indicadores de sesión (fase 42, el modo
copia). Ahora sacan el dato de `extras` antes de bloquear el terminal.

**No probado:** con el kit tal cual; un historial de 100 000 líneas (el
filtro recorre todo en cada repintado).

**Difiere de iTerm2:** no es una sesión aparte: no se puede desplazar, buscar
ni seleccionar dentro del resultado; sin modo de expresión regular ni barra
de progreso; el filtro sigue al panel activo, no a la sesión donde se abrió; la barra va debajo de
la búsqueda en vez de dentro del mismo desplegable.

## Fase 46 — Aviso antes de pegar varias líneas

### Plan

Edit › Paste Special de iTerm2 (`iTermPasteHelper.m
-maybeWarnAboutMultiLinePaste:`, `iTermApplicationDelegate.m
-toggleMultiLinePasteWarning:`):

1. **Tres casillas**, con los valores de iTerm2: Warn Before Multi-Line Paste
   (sí), Limit Multi-Line Paste Warning to Shell Prompt (sí; inactiva si la
   primera está apagada) y Warn Before Pasting One Line Ending in a Newline
   at Shell Prompt (sí).
2. **El aviso** «Confirm Multi-Line Paste» con «OK to paste N lines at shell
   prompt?», «OK to paste N lines?» o los de una línea con salto final, según
   lo pegado y si la sesión está en el prompt (entre `133;B` y `133;C`).
3. **Pegar** solo tras Paste; Cancel no pega nada.
4. **Prueba** y documentación; un test para la decisión.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos; un test nuevo para cuándo avisar.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `echo uno` y `echo dos` en el portapapeles y ⌘V sin prompt marcado: se pega sin preguntar (aviso limitado al prompt) | ✓ | — |
| Con `PS1` que emite `133;B`, ⌘V: «Confirm Multi-Line Paste», «OK to paste 2 lines at shell prompt?», Cancel y Paste | ✓ | `fase46-aviso-pegar.png` |
| Paste ejecuta las dos órdenes | ✓ | — |

**No probado:** con el kit tal cual; Cancel; las casillas desde el menú; los
casos de una línea con salto final (están en el test).

**Difiere de iTerm2:** sin Paste Without Newline, Advanced… ni «no volver a
preguntar» (BK-037); con una shell
que solo emite `133;B` la sesión se queda «en el prompt» también mientras
corre un comando.

**Corrección (2026-11-06):** Paste Slowly pasa ahora por el mismo aviso y,
tras Paste, pega despacio (probado en sway con dos líneas en un prompt
marcado); Paste Escaped ya avisaba.

## Fase 47 — Advanced Paste

### Plan

Edit › Paste Special › Advanced Paste… (⌥⌘V) de iTerm2
(`iTermPasteSpecialViewController.xib` y `iTermPasteHelper.m
+sanitizePasteEvent:encoding:`):

1. **Transformaciones** en el orden de iTerm2, en un módulo puro con tests
   (`paste_transform.rs`): quitar o convertir los saltos de línea a CR,
   puntuación Unicode a ASCII (comillas y rayas), quitar los códigos de
   control salvo tab, LF, FF y CR, tabuladores (tal cual, ^V o 4 espacios),
   escapar la shell con `\` (sin tocar los tabuladores escapados con ^V),
   sustitución con expresión regular (`$1` y `\n`, `\t`, `\r`, `\\` como
   en vim) y Base64 en líneas de 77 con CR.
2. **El panel**, con los valores por defecto de iTerm2 (CR y quitar códigos de
   control activos), el modo de pegado entre corchetes, la vista previa del
   resultado y Cancel o Paste.
3. **Prueba** y documentación.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos; dos tests nuevos para las transformaciones.

**Funciona** (capturas en [`capturas/`](capturas/)):

| Prueba | Resultado | Captura |
|---|---|---|
| `echo “hola mundo”<tab>—fin` en el portapapeles y ⌥⌘V: panel con CR y códigos de control activos | ✓ | `fase47-pegado-avanzado.png` |
| Puntuación Unicode y «Convert tabs to 4 spaces»: la vista previa pasa a `echo "hola mundo"    -fin` | ✓ | `fase47-resultado.png` (arriba) |
| Paste escribe eso mismo en la shell | ✓ | `fase47-resultado.png` (abajo) |

**No probado:** con el kit tal cual; Base64, la expresión regular y el
escape de la shell desde el panel (están en los tests).

**Difiere de iTerm2:** sin la sección Speed (tamaño de bloque y pausa) ni
«Wait for shell prompt before pasting each line»; el texto original no se
edita en el panel; el número de espacios es fijo (4), sin el control para
cambiarlo; la expresión regular es la del crate `regex`, no la de ICU.

## Fase 48 — Duplicate Window

### Plan

Shell › Duplicate Window de iTerm2 (`PseudoTerminal.m -duplicateWindow:` y
`-arrangementForDuplicateWindow`): una ventana nueva con la disposición de
todas las pestañas (paneles, perfiles y carpetas), sin el contenido. La
disposición viaja en TOML con `--layout`, así que sirve igual en el mismo
proceso (kit parcheado) que lanzando otro (kit tal cual).

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos.

| Prueba | Resultado | Captura |
|---|---|---|
| Dos paneles, el segundo en `/tmp`, y Duplicate Window: otra ventana con los dos paneles y el segundo en `/tmp` | ✓ | `fase48-duplicar-ventana.png` |

**No probado:** con el kit tal cual (otro proceso).

**Difiere de iTerm2:** la ventana nueva no copia el tamaño ni la posición, ni
los títulos propios de la ventana.

## Fase 49 — Avisos de pegado completos

### Plan

Los dos avisos de pegado de iTerm2 con todos sus botones, en una hoja propia
porque el `Alert` del kit solo admite dos (BK-037):

1. **Tabuladores** (`PTYSession.m -pasteString:flags:` y `iTermPasteHelper.m
   +pasteWithTabsWarningWithAccessoryController:`): si el programa no pide
   pegado entre corchetes y el texto tiene tabuladores, «You're about to paste
   a string with tabs.» con OK, Cancel, Convert tabs to spaces (con el número
   de espacios, 4 por defecto y guardado) y Advanced….
2. **Varias líneas** (fase 46): Paste, Paste Without Newline (solo con una
   línea que acaba en salto), Cancel y Advanced….
3. **Orden de iTerm2:** primero los tabuladores y después las líneas.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos.

| Prueba | Resultado | Captura |
|---|---|---|
| `echo a<tab>b` y ⌘V: la hoja de tabuladores con sus cuatro botones | ✓ | `fase49-avisos-pegado.png` (abajo) |
| Subir a 6 espacios y Convert tabs to spaces: se pega `echo a      b` y `paste_tab_spaces = 6` queda guardado | ✓ | — |
| `echo sin salto` con salto final en un prompt marcado: Paste, Paste Without Newline, Cancel y Advanced… | ✓ | `fase49-avisos-pegado.png` (arriba) |
| Paste Without Newline deja la orden escrita sin ejecutarla | ✓ | — |

**No probado:** con el kit tal cual; Advanced… desde los avisos; OK y Cancel
en la hoja de tabuladores.

**Difiere de iTerm2:** sin «no volver a preguntar» en los avisos; la hoja es
propia, no un `NSAlert`.

## Fase 50 — Modos de búsqueda y Clear Find

### Plan

El modo de búsqueda de iTerm2 (`iTermFindViewController.h` `iTermFindMode`,
`iTermFindDriver.m` y `FindView.xib`) y Edit › Find › Clear Find
(`iTermApplicationDelegate.m -clearFindString:`):

1. **Cinco modos:** Smart Case Substring (por defecto: distingue mayúsculas
   solo si la búsqueda tiene alguna), Case-Sensitive Substring,
   Case-Insensitive Substring, Case-Sensitive Regex y Case-Insensitive Regex.
   Find y Filter guardan cada uno el suyo, como `findMode_iTerm` y
   `NoSyncFilterMode`. El modo se elige en un `PopUpButton` junto al campo
   (BK-038).
2. **Filter con expresiones:** el filtro deja de comparar texto y usa el mismo
   patrón que Find.
3. **Clear Find (⌥⌘E):** borra la búsqueda; solo está activo si hay algo que
   borrar.
4. **Orden del submenú Find de iTerm2:** Find URLs, Find All Smart Selection
   Matches y Pick Result to Open en su grupo, y Filter al final.

### Resultado (2026-11-06)

Probado en sway sin pantalla con el kit parcheado. `./dev.sh check` limpio en
los dos modos; un test nuevo comprueba cada modo con el crate `regex` y que
la búsqueda de `alacritty_terminal` acepta sus patrones.

| Prueba | Resultado | Captura |
|---|---|---|
| `error` en Smart Case: 13 resultados con `Error`, `error` y `ERROR` | ✓ | — |
| Case-Sensitive Substring: 5 de 5, solo `error`; la búsqueda se repite al cambiar el modo | ✓ | `fase50-distingue.png` |
| ⌥⌘E deja el campo vacío y sin resaltado | ✓ | — |
| `FO+.BAR` en Case-Insensitive Regex: `foo.bar` y `fooxbar` | ✓ | `fase50-regex.png` |
| `find_mode` y `filter_mode` quedan guardados por separado en el `.domain` | ✓ | — |
| Filter `^E` en Case-Sensitive Regex: solo `Error one` y `ERROR 3` | ✓ | `fase50-filtro.png` |

**No probado:** con el kit tal cual; Clear Find activo o inactivo en el menú
D-Bus.

**Difiere de iTerm2:** el modo está en un `PopUpButton` y no en el menú de la
lupa (BK-038); sin Erase Search History.
