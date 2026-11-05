# Estudio previo

**Objetivo:** fijar lo que hay que saber de BarrierKit, de la barra de menús de
Malaysia y de iTerm2 antes de escribir CurseTerm.
**Alcance:** desde la lectura de los tres repos hasta las decisiones abiertas de
la fase 1. No describe código de CurseTerm, que aún no existe.
**Lector:** quien implementa CurseTerm o revisa `barrierkit-gaps.md`.

**Entradas (2026-11-05):**

| Repo | Commit | Uso |
|---|---|---|
| `barrierkit` | `ba58ce01ef1f8e07dd915f783290b6ef068c037a` (`main`) | Dependencia, solo lectura |
| `malaysia` | `8a7eb92` (`main`); fija barrierkit en `9ff264b` | Barra de menús, solo lectura |
| `iterm2` | `468c393` (`master`, clon superficial) | Referencia de comportamiento, solo lectura |

**Salida:** este documento y la primera versión de
[`barrierkit-gaps.md`](barrierkit-gaps.md).

## 1. Convenciones que CurseTerm adopta

Salen de `barrierkit/AGENTS.md` y `barrierkit/CLAUDE.md`:

1. **Código mínimo.** Reutilizar antes de escribir; nada «por si acaso»; sin
   comentarios salvo `// SAFETY:` y el porqué de un invariante; diff mínimo.
2. **Tests solo para lógica pura con riesgo**, con el nombre como
   especificación (`a_resize_sends_sigwinch_with_the_new_grid`).
3. **Diseño solo con tokens.** Ningún literal de color ni medida: todo sale de
   `vow_design` y de `Palette::current()`. La excepción son los colores del
   esquema del terminal (ANSI, fondo y tinta del perfil), que son datos del
   usuario y no tokens de interfaz (ver §6).
4. **Texto visible por Fluent:** `i18n/{en,es}/curseterm.ftl` y `tr!`.
5. **Commits** `tipo(ámbito): …` (`feat`, `fix`, `docs`, `refactor`, `test`,
   `perf`, `chore`, `build`).
6. **Docs en español e inglés** (`nombre.md` y `nombre.en.md`, en el mismo
   commit), como en barrierkit; código, comentarios y rustdoc solo en inglés.
7. **ADR** con el formato del kit: título `NNNN — …`, «Estado», «Contexto»,
   «Decisión», «Consecuencias» y «Referencias».

## 2. Cómo se usa BarrierKit

### Arranque y modelo

- Una app es un `App` de Freya (`render(&self) -> impl IntoElement`) envuelto en
  `AppComponent` y lanzado con `platform::run_window(app, WindowOptions)`.
  `run_window` es el único arranque y bloquea hasta cerrar.
- La interfaz es una función del estado (`use_state`, `State<T>`); los
  controles son controlados (`value`/`on_change`).
- Capas (ADR 0047): 0 plataforma, 1 primitivas, 2 vistas, 3 patrones y la
  columna App. Sin `prelude`: cada pieza se importa de su módulo.
- Freya `=0.4.3`, solo `freya-core`, `freya-components` sin features,
  `freya-animation`, `freya-engine`, `torin` y `ragnarok`. Nunca `freya` ni
  `freya-winit`.
- Dibujo con Skia sobre Vulkan (ADR 0056); sin Vulkan no abre la ventana
  (`RunError::Vulkan`). Lavapipe sirve.
- Freya repinta el árbol entero en cada frame (sin damage parcial).

### Piezas útiles para un terminal

| Necesidad | Pieza del kit | Nota |
|---|---|---|
| Titlebar CSD y semáforos | `titlebar::Titlebar`, `chrome::WindowChrome` | Zonas `left`/`right`, título; estados de foco, maximizada y pantalla completa ya resueltos |
| Estado y órdenes de ventana | `platform::WindowControls::get()` | `toggle_fullscreen`, `minimize`, `close`… |
| Menú global | `app_menu::{AppMenu, Menu}` + `WindowOptions::menu` | dbusmenu + `org_kde_kwin_appmenu_manager` |
| Acciones y atajos | `app::{Action, Shortcut, use_menu_action}` | El atajo solo llega si la acción está en el menú |
| Estado vivo del menú | `app_menu::MenuHandle` (contexto raíz) y `MenuHandle::update(id, f)` | Lo usa `use_undo`; no está documentado |
| Menú contextual | `context_menu(fila, items, on_select)` | `MenuView` en el puntero |
| Preferencias | `patterns::preferences::PreferencesWindow`, `views::form::*` | Barra lateral + buscador + `Form` |
| Pestañas | `views::tabs::TabView` | Pestañas de barra de herramientas con títulos `&'static str` |
| Divisiones | `views::split::SplitView` | Barra lateral + detalle + inspector; no es un divisor genérico |
| Alertas y hojas | `views::feedback::{Alert, sheet}` | Dentro de la misma ventana |
| «Acerca de» | `patterns::about::about_window` | |
| Persistencia | `app::{load_defaults, save_defaults, use_defaults, read_toml, write_toml}` | `.domain` TOML en `~/.config/malaysia/<app_id>.domain` |
| Instancia única | `app::launch` | D-Bus; reenvía la activación |
| Portapapeles | `freya_clipboard::Clipboard` | Solo texto sobre `wl_data_device` |
| Cursor | `Cursor::set(CursorIcon::Text)` | Requiere `wp_cursor_shape_manager_v1` |
| Fuente propia | `register_font(bytes)` | |
| Idiomas | `barrier_kit::i18n`, `tr!` | |
| Dibujo propio | `freya_core::element::ElementExt::render(RenderContext)` | Público en Freya; el kit lo usa internamente en `glass.rs` |

### Menú global, de punta a punta

1. La app pasa un `AppMenu` (lista de `Menu { title, items }`) en
   `WindowOptions::menu`.
2. El runner crea un `MenuHandle`, lo publica en D-Bus como
   `com.canonical.dbusmenu` y, si el compositor ofrece
   `org_kde_kwin_appmenu_manager`, anuncia servicio y ruta para la superficie.
   Sin el protocolo, el menú existe solo en D-Bus (`tracing::info!`).
3. Una pulsación del menú llega a la app por `use_menu_action(|id| …)`, en el
   hilo de interfaz.
4. Un atajo de teclado se busca en el menú de la ventana con foco
   (`find_menu_shortcut`) y se despacha igual que un clic. Los atajos se
   guardan como texto (`"Ctrl+Shift+S"`); `Action` usa `Shortcut` tipado.
5. Para cambiar el menú en vivo, la app toma `MenuHandle` con
   `try_consume_root_context::<MenuHandle>()` y llama a `update(id, f)`: cambia
   `enabled`, `check`, `label` o los hijos de un `Submenu` con id. El servidor
   emite `LayoutUpdated`.

### La barra de menús de Malaysia (`malaysia-shell menubar`)

- Lee el menú remoto de la ventana enfocada (`menubar/dbusmenu.rs`), atiende
  `LayoutUpdated`, `ItemsPropertiesUpdated` y `AboutToShow`.
- `MenuSources::compose` (`menubar/remote.rs`) mezcla el menú de la app con
  los suyos:
  - siempre pone primero el menú del sistema (copo) y luego uno con el nombre
    de la app en negrita, con ranuras propias (Acerca de, Ajustes, Servicios,
    Ocultar, Ocultar otras, Mostrar todo, Salir);
  - un submenú remoto cuyo título coincide con el nombre de la app rellena esas
    ranuras por etiqueta y añade el resto;
  - «Window»/«Ventana» y «Help»/«Ayuda» remotos se añaden a los de la shell, que
    ya traen controles de ventana y búsqueda;
  - el resto va en medio, en orden.
- El nombre de la app sale del `.desktop` con ese `app_id` o `StartupWMClass`;
  si no hay, del último segmento del `app_id` con mayúscula inicial.
- Por eso CurseTerm publica su menú «CurseTerm» con el mismo nombre que su
  `.desktop`, y «Window» y «Help» se fusionan con los de la shell.

### Atajos fijos de Malaysia

`malaysia/docs/configuracion/opciones.md` reserva en el compositor `Super+Q`
(cerrar la app), `Super+W` (cerrar la ventana), `Super+M`, `Super+H`,
`Alt+Super+H`, `Super+Space`, `Ctrl+Super+F`, `Shift+Super+3/4` y varios
`Ctrl+Alt+…`. Una app no los recibe.

## 3. Lo que el kit no ofrece hoy (detectado al leer)

Cada punto tiene su entrada en [`barrierkit-gaps.md`](barrierkit-gaps.md); aquí
solo el resumen:

1. **Una ventana por proceso.** `run_window` (`platform/window.rs:346`) crea
   una ventana; `Runtime::create_window` es `pub(super)`. iTerm2 abre varias
   ventanas en un proceso y las lista en «Window».
2. **Título de ventana fijo.** No hay `set_title` en vivo; un terminal cambia
   el título con OSC 0/2.
3. **Atajos limitados y no consumidos.** `ShortcutKey` solo tiene `Char`,
   `Left` y `Right` (`app/action.rs:9`); el título de `Action` es
   `fn() -> &'static str` (`:66`). Cuando un atajo dispara su acción, la tecla
   llega igualmente a la app (`platform/input/seat.rs:464`).
4. **Sin IME** (`zwp_text_input_v3`) ni selección primaria (pendiente por
   Freya en ADR 0047). El texto compuesto depende de lo que entregue xkb en
   `utf8`.
5. **Sin pestañas de documento.** `TabView` (`views/tabs.rs:16`) es el selector
   de barra de herramientas con títulos estáticos; no hay cerrar, «+»,
   reordenar ni títulos vivos como la barra de pestañas de iTerm2.
6. **Sin divisor genérico.** `SplitView` (`views/split.rs:94`) es barra
   lateral + detalle; no anida ni divide en vertical.
7. **Sin cara monoespaciada en Tengen.** Las tipografías de `tokens.json` son
   todas `sans`.
8. **Sin primitiva pública de lienzo.** El kit dibuja a mano solo en módulos
   privados (`glass.rs:176`); una app que necesita una rejilla implementa
   `ElementExt` de Freya.
9. **Caché de párrafos apagada** (`fonts.rs:20`): cada `label` se vuelve a
   maquetar en cada frame.
10. **Sin arrastrar y soltar entre apps, sin abrir URLs, sin portapapeles con
    tipos** (pendientes en ADR 0047).
11. **Diálogos solo dentro de la ventana** (`sheet`, `Alert`); una ventana de
    Preferencias aparte depende del punto 1.

Contradicciones entre documentación y código (manda el código):

- `docs/barrierkit/04-apariencia-y-tokens.md:33` cita `tengen/generated/` con
  Slint, CSS y QML; la carpeta no existe.
- `docs/barrierkit/01-que-es.md:30` habla de «cinco apps de prueba»; el
  workspace tiene tres (`tengen-app-show`, `ondina`, `marea`).
- `MenuHandle::update` es la única forma de cambiar el menú en vivo y no
  aparece en la documentación; `replace_menu` existe pero solo en tests
  (`app_menu.rs:110`).

## 4. Inventario de iTerm2

### Menús (`sources/MainMenu/Base.lproj/MainMenu.xib`)

| Menú | Entradas principales (⌘ = Command, ⌥ = Option, ⇧ = Shift, ⌃ = Control) |
|---|---|
| iTerm2 | About · Preferences… ⌘, · Services · Hide ⌘H · Hide Others ⌥⌘H · Show All · Secure Keyboard Entry · Install Shell Integration · Quit ⌘Q |
| Shell | New Window ⌘N · New Tab ⌘T · New Tab with Current Profile ⌥⇧⌘T · Duplicate Tab · Split Horizontally with Current Profile ⇧⌘D · Split Vertically with Current Profile ⌘D · Save Selected Text… ⌘S · Close ⌘W · Close Terminal Window ⇧⌘W · Close All Panes in Tab ⌥⌘W · Undo Close ⇧⌘T · Broadcast Input ▸ · tmux ▸ · ssh ▸ · Print ▸ |
| Edit | Undo · Redo · Cut · Copy ⌘C · Copy with Styles ⌥⌘C · Copy Mode ⇧⌘C · Paste ⌘V · Paste Special ▸ · Open Selection · Select All ⌘A · Select Output of Last Command ⇧⌘A · Find ▸ (Find… ⌘F, Find Next ⌘G, Find Previous ⇧⌘G, Use Selection for Find ⌘E…) · Marks and Annotations ▸ (Set Mark ⇧⌘M, Jump to Mark ⇧⌘J, Next/Previous Mark ⇧⌘↓/↑) · Clear Buffer ⌘K · Clear Scrollback Buffer ⇧⌘K |
| View | Show Tabs in Fullscreen ⇧⌘U · Toggle Full Screen ⌃⌘F · Toolbelt ▸ · Use Transparency ⌘U · Find Cursor ⌘/ · Show Timestamps ⇧⌘E · Open Quickly ⇧⌘O · Maximize Active Pane · Make Text Bigger ⌘+ · Make Text Normal Size ⌘0 · Make Text Smaller ⌘- · Start Instant Replay |
| Session | Edit Session… ⌘I · Run Coprocess… · Restart Session · Duplicate Session · Open Paste History… ⇧⌘H · Triggers ▸ · Reset ⌘R · Reset Character Set · Move Session ▸ · Log ▸ · Terminal State ▸ · Bury Session |
| Scripts | Scripts de Python (fuera de alcance) |
| Profiles | Open Profiles… ⌘O · lista dinámica de perfiles (⌃⌘1…) |
| Window | Minimize ⌘M · Zoom · Arrangements ▸ · Edit Window Title · Tab ▸ (Select Next/Previous Tab ⇧⌘] / ⇧⌘[, Move Tab Left/Right) · Split Pane ▸ (Select Pane Above/Below/Left/Right ⌥⌘↑↓←→, Next/Previous Pane ⌘] / ⌘[, Move Divider ⌃⌘↑↓←→) · Window Style ▸ · Arrange Windows ▸ · Resize Window ▸ · Password Manager · Pin Hotkey Window · Bring All To Front · lista dinámica de ventanas |
| Help | iTerm2 Help ⌘? · Copy Mode Shortcuts · Open Source Licenses |

Notas:

- En el xib una letra mayúscula implica ⇧ (por ejemplo `D` es ⇧⌘D).
- «Profiles» y la lista de ventanas de «Window» se rellenan en tiempo de
  ejecución.
- Muchas entradas van ocultas o son alternas con ⌥ (`isAlternate`); CurseTerm
  no las reproduce.

### Preferencias (`PreferencePanel.xib`)

- Barra de herramientas: **General · Appearance · Profiles · Keys · Arrangements
  · Pointer · Advanced** (y Shortcuts en la versión actual).
- General: Startup · Closing · Magic · Selection · Window · Settings · tmux ·
  Experimental.
- Appearance: General · Windows · Tabs · Panes · Dimming.
- Profiles (editor por perfil): General · Colors · Text · Window · Terminal ·
  Session · Keys · Advanced.
- Keys: Key Bindings · Navigation Shortcuts · Hotkey · Remap Modifiers.

### Código de referencia

| Área | Carpeta de `sources/` |
|---|---|
| Ventana, pestañas y barra | `iTermController`, `TerminalView`, `TabGroups`, `SplitPanel` |
| Sesión y PTY | `PTYSession`, `SessionCreation`, `ProcessInfo` |
| Emulación | `VT100`, `VT100Screen`, `LineBuffer`, `ScreenChar` |
| Selección y copia | `Selection`, `CopyMode`, `Pasting` |
| Búsqueda | `SearchingFiltering`, `GlobalSearch` |
| Marcas y shell integration | `Marks`, `ShellIntegration` |
| Imágenes en línea | `InlineImages` |
| Triggers | `Triggers` |
| Broadcast | `InputBroadcasting` |
| Hotkey window | `Hotkey` |
| Perfiles y preferencias | `ProfilesPanel`, `Settings` |

## 5. Emulación y PTY: candidatos

La decisión se cierra en un ADR al empezar la fase 1. Lo que hay:

| | `alacritty_terminal` 0.26.0 | `vte` 0.15.0 |
|---|---|---|
| Qué es | Parser + rejilla + scrollback + selección + búsqueda regex + PTY (`tty`, con `rustix-openpty`) + bucle de E/S | Solo el parser de secuencias (máquina de estados de Paul Williams) |
| Lo que habría que escribir con él | Render, entrada, OSC propios (1337, 133) | Rejilla, scrollback, reflow, selección, búsqueda, PTY |
| Riesgo | API semiestable, pensada para Alacritty; OSC desconocidos se ignoran | Reimplementar lo que el encargo pide no reinventar |

Inclinación: `alacritty_terminal`, con su `tty` para el PTY (cubre
`TIOCSWINSZ`/`SIGWINCH` al redimensionar), y un `vte::Parser` propio en paralelo
solo para los OSC que Alacritty ignora (1337, 133) en la fase 4. `portable-pty`
sobra si `alacritty_terminal::tty` basta.

## 6. Decisiones abiertas para la fase 1

1. **Dependencia del kit.** El flake de barrierkit no exporta `lib` ni
   overlay; solo `packages.skia-binaries`. Plan: `barrierkit` como input del
   flake de CurseTerm (para reutilizar `skia-binaries` y el dev shell) y las
   crates como dependencia git en `Cargo.toml` con el mismo `rev`, como hace
   malaysia (ADR 0048 §2).
2. **Esquema de atajos.** Inclinación: **Super** como ⌘. Razones:
   - la shell de Malaysia ya traduce ⌘ a Super en sus menús (Super+C, Super+V,
     Super+Q, Super+H) y su compositor reserva Super+Q/H/M igual que macOS;
   - un terminal no envía Super a la shell, así que ningún atajo de la app roba
     una combinación a programas como `vim` o `emacs`, cosa que sí pasa con
     Ctrl+Shift en modos CSI u.
   Costes: Malaysia reserva Super+W (cerrar ventana) y GNOME reserva Super+A,
   Super+H, Super+L, Super+V, etc. Se mide en la fase 1 y se documenta en el ADR.
3. **App-id.** `org.curseterm.CurseTerm`. Malaysia solo trata como «propia» una
   ventana con el prefijo `io.github.hblanqueto.malaysia.` (ADR 0035); con la
   política `force_server` le añadiría una segunda titlebar. Con la política
   por defecto respeta la CSD que pide el kit. Se anota como observación del
   escritorio, no del kit.
4. **Colores del terminal.** La paleta ANSI y el fondo son datos del perfil
   (como los `.itermcolors`), no tokens de interfaz. Van en el perfil del
   usuario y en esquemas incluidos como datos; la interfaz alrededor (titlebar,
   pestañas, preferencias) usa solo `Palette` y tokens. Hay que comprobar que
   `scripts/check-design-tokens.sh` del kit no se aplica a CurseTerm o
   justificar la excepción en un ADR.
5. **Rejilla.** Un elemento propio (`ElementExt`) que dibuja la rejilla con
   Skia: un rectángulo de fondo por racha de color y un `TextBlob` por racha de
   estilo, en vez de un `label` por celda.

## 7. Entorno de este trabajo

- No hay sesión gráfica en el contenedor. Para probar hace falta un compositor
  sin pantalla (Weston o sway headless, como en `docs/barrierkit/14-plataforma.md`)
  y Vulkan por software (lavapipe).
- `nix develop` de barrierkit no termina aquí: construye `skia-binaries` desde
  el código fuente (`nix/skia.nix`) y necesita descargas de GitHub.
  1. Los `fetchgit` fallaban por TLS (el proxy re-firma); se resuelve con
     `NIX_GIT_SSL_CAINFO=/root/.ccr/ca-bundle.crt` y
     `--option extra-sandbox-paths /root/.ccr`.
  2. El tarball `github.com/rust-skia/skia/archive/refs/tags/m148-0.97.0.tar.gz`
     devuelve **403**: la política de red del entorno no deja descargar
     archivos ni releases de `github.com` (sí `git clone`). Sin Skia no compila
     `freya-skia-bindings` ni, por tanto, el kit.
  Hasta que se permita `github.com` (y `codeload.github.com`) en la red del
  entorno, nada que dependa del kit se puede compilar ni ejecutar aquí. La
  fase 1 lo anota en `progreso.md` y no declara probado nada que no se haya
  ejecutado.
- `dev.sh lab` de malaysia necesita además Vulkan, un bus D-Bus privado y el
  compositor anidado; se probará en la fase 1.
