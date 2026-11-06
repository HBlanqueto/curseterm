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
