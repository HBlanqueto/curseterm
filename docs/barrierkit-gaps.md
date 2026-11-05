# Lo que le faltó a BarrierKit al construir CurseTerm

Versión/commit de BarrierKit usado: `ba58ce01ef1f8e07dd915f783290b6ef068c037a` (`main`, 2026-11-05)

Resumen: documento vivo. Esta primera versión sale del estudio previo (lectura
de código y documentación, sin ejecutar nada): los gaps están **detectados**,
no medidos, y aún no tienen workaround. Lo más serio para un terminal: una sola
ventana por proceso y sin título en vivo; atajos que no se consumen y con pocas
teclas; sin IME ni selección primaria; sin pestañas de documento ni divisor
genérico; sin cara monoespaciada en Tengen.

Estados: **detectado** (leído en el código), **confirmado** (reproducido al
ejecutar CurseTerm), **resuelto en CurseTerm** (con workaround).

## Tabla resumen

| ID | Área | Gap | Gravedad | Workaround en CurseTerm |
|---|---|---|---|---|
| BK-001 | ventana | Una ventana por proceso | alta | Pendiente (fase 2) |
| BK-002 | ventana | Título de ventana fijo | media | Pendiente (fase 1) |
| BK-003 | input / menú global | Atajos no consumidos y `ShortcutKey` limitado | alta | Pendiente (fase 1) |
| BK-004 | IME | Sin `zwp_text_input_v3` | media | Pendiente |
| BK-005 | portapapeles | Sin selección primaria | media | Pendiente (fase 1) |
| BK-006 | contenedores | Sin pestañas de documento | media | Pendiente (fase 2) |
| BK-007 | contenedores | Sin divisor genérico anidable | media | Pendiente (fase 2) |
| BK-008 | tokens / texto | Sin cara monoespaciada en Tengen | baja | Pendiente (fase 1) |
| BK-009 | texto / rendimiento | Sin lienzo público y caché de párrafos apagada | media | Pendiente (fase 1) |
| BK-010 | menú global | Títulos de `Action` estáticos y menú sin reemplazo en vivo | media | Pendiente (fase 2) |
| BK-011 | docs | Documentación que no coincide con el código | baja | — |

## Gaps

### BK-001 — Una ventana por proceso
- **Área**: ventana
- **Qué necesitaba CurseTerm**: «Shell › New Window» (⌘N) y la lista de
  ventanas de «Window», como iTerm2, que abre varias ventanas en un proceso.
- **Qué ofrece hoy BarrierKit**: `platform::run_window`
  (`crates/barrier-kit/src/platform/window.rs:346`) crea una ventana y bloquea.
  `Runtime::create_window` (`:360`) ya sabe crear más, pero es `pub(super)`.
- **Qué falta**: abrir y cerrar ventanas nuevas desde la app en el mismo
  runner, y enumerarlas.
- **Cómo lo resolví en CurseTerm**: pendiente. Plan: un proceso por ventana y
  coordinación entre procesos por D-Bus (`launch` ya da instancia única).
- **Coste del workaround**: más memoria (un contexto Vulkan y Skia por
  proceso), estado compartido por IPC y la lista de «Window» con retardo.
- **API propuesta para el kit**: `WindowControls::open(app: AppComponent, options: WindowOptions) -> WindowControls` y `Windows::get().list() -> State<Vec<WindowInfo>>`.
- **Gravedad**: alta
- **Cómo reproducirlo**: buscar en `barrier_kit::platform` una función pública
  que cree una segunda `xdg_toplevel` después de `run_window`: no existe.

### BK-002 — Título de ventana fijo
- **Área**: ventana
- **Qué necesitaba CurseTerm**: el título de la ventana sigue a la sesión
  (OSC 0/2, directorio actual); lo usan la barra de menús, el Dock y la lista
  de «Window».
- **Qué ofrece hoy BarrierKit**: `WindowOptions::new(title, …)`; el runner
  llama a `set_title` una vez (`platform/window.rs:382`). `WindowCommand`
  (`:256`) no tiene orden de título.
- **Qué falta**: cambiar el título de `xdg_toplevel` en vivo.
- **Cómo lo resolví en CurseTerm**: pendiente. El título visible de la
  `Titlebar` sí se puede cambiar; el de `xdg_toplevel`, no.
- **Coste del workaround**: el compositor y la shell muestran siempre
  «CurseTerm».
- **API propuesta para el kit**: `WindowControls::set_title(&self, title: impl Into<String>)`.
- **Gravedad**: media
- **Cómo reproducirlo**: `grep -rn set_title crates/barrier-kit/src` da una
  sola llamada, en la creación.

### BK-003 — Atajos que no se consumen y teclas que no se pueden declarar
- **Área**: input / menú global
- **Qué necesitaba CurseTerm**: atajos como ⌘D, ⌥⌘←, ⌘[, ⌘] o ⇧⌘↓; y que la
  pulsación de un atajo no llegue además a la shell del terminal.
- **Qué ofrece hoy BarrierKit**: `ShortcutKey` (`app/action.rs:9`) tiene
  `Char`, `Left` y `Right`. El texto del menú (`MenuItem::Action::shortcut`)
  admite más teclas (`F11`, `Page_Down`), pero sin tipo. En
  `platform/input/seat.rs:424-468` el atajo se despacha y la tecla se envía
  igualmente a la app (`:464`).
- **Qué falta**: `Up`, `Down`, `Tab`, `Home`/`End`, `PageUp`/`PageDown`, `F1–F12`
  y teclas de símbolo en `ShortcutKey`; y que un atajo despachado no se
  reenvíe como tecla.
- **Cómo lo resolví en CurseTerm**: pendiente. Plan: atajos por texto en
  `MenuItem` para las teclas sin tipo, y descartar en el codificador de
  teclado del terminal las teclas con el modificador del esquema de atajos.
- **Coste del workaround**: el filtro del terminal duplica la tabla de atajos.
- **API propuesta para el kit**: `enum ShortcutKey { Char(char), Named(NamedKey) }` y que `key_event` no empuje el `PlatformEvent::Keyboard` cuando `shortcut_action.is_some()`.
- **Gravedad**: alta
- **Cómo reproducirlo**: una `Action` con `Shortcut::ctrl(ShortcutKey::Char('d'))`
  y un `on_global_key_down` en la raíz: los dos se disparan con Ctrl+D.

### BK-004 — Sin método de entrada
- **Área**: IME
- **Qué necesitaba CurseTerm**: escribir en chino, japonés o coreano y ver el
  texto en preparación en la posición del cursor.
- **Qué ofrece hoy BarrierKit**: teclado con xkb vía SCTK; ningún módulo usa
  `zwp_text_input_v3`.
- **Qué falta**: el protocolo de entrada de texto y un evento de preedición.
- **Cómo lo resolví en CurseTerm**: pendiente.
- **Coste del workaround**: sin IME no hay workaround dentro de la app.
- **API propuesta para el kit**: eventos `Ime::Preedit { text, cursor }` y `Ime::Commit(String)`, y `ime_area(rect)` para colocar la ventana del IME.
- **Gravedad**: media
- **Cómo reproducirlo**: `grep -rn text_input crates/barrier-kit/src` no da
  resultados.

### BK-005 — Sin selección primaria
- **Área**: portapapeles
- **Qué necesitaba CurseTerm**: copiar al seleccionar y pegar con el botón
  central, como cualquier terminal de Linux.
- **Qué ofrece hoy BarrierKit**: portapapeles de texto sobre `wl_data_device`
  (`platform/clipboard.rs`), expuesto como `freya_clipboard::Clipboard`. La
  selección primaria está pendiente en ADR 0047 («Bloqueado por Freya 0.4.3»).
- **Qué falta**: `zwp_primary_selection_device_v1`.
- **Cómo lo resolví en CurseTerm**: pendiente.
- **Coste del workaround**: —
- **API propuesta para el kit**: `PrimarySelection::set(String)` y `PrimarySelection::get() -> Option<String>`, separados del `Input` de Freya.
- **Gravedad**: media
- **Cómo reproducirlo**: leer ADR 0047, «Pendiente».

### BK-006 — Sin pestañas de documento
- **Área**: contenedores
- **Qué necesitaba CurseTerm**: la barra de pestañas de iTerm2: título vivo
  por sesión, botón de cerrar, «+», indicador de actividad y reordenar.
- **Qué ofrece hoy BarrierKit**: `views::tabs::TabView`
  (`views/tabs.rs:13-35`), pestañas de barra de herramientas con símbolo y
  título `&'static str`.
- **Qué falta**: un `TabBar` de documento con títulos `String` y acciones por
  pestaña.
- **Cómo lo resolví en CurseTerm**: pendiente (fase 2).
- **Coste del workaround**: —
- **API propuesta para el kit**: `TabBar::new(label).tab(Tab::new(id, title: String).closable(true)).on_select(..).on_close(..).on_add(..)`.
- **Gravedad**: media
- **Cómo reproducirlo**: `TabView::tab` no acepta un `String`.

### BK-007 — Sin divisor genérico
- **Área**: contenedores
- **Qué necesitaba CurseTerm**: paneles divididos en vertical y horizontal,
  anidados, con asa redimensionable.
- **Qué ofrece hoy BarrierKit**: `views::split::SplitView`
  (`views/split.rs:94`): barra lateral + detalle + inspector.
- **Qué falta**: un divisor de N hijos en una dirección, anidable.
- **Cómo lo resolví en CurseTerm**: pendiente (fase 2).
- **Coste del workaround**: —
- **API propuesta para el kit**: `SplitPanes::new(Direction::Horizontal).pane(a).pane(b).ratios(State<Vec<f32>>)`.
- **Gravedad**: media
- **Cómo reproducirlo**: `SplitView` no tiene opción de dirección.

### BK-008 — Sin cara monoespaciada en Tengen
- **Área**: tokens / texto
- **Qué necesitaba CurseTerm**: la fuente del terminal por defecto y textos de
  código en Preferencias.
- **Qué ofrece hoy BarrierKit**: `vow_design::Font` con `family`; en
  `crates/vow-design/tengen/tokens.json:285-321` todas son `sans`. ADR 0047
  menciona un estilo «Mono» que no existe.
- **Qué falta**: un token `MONO` (familia, tamaño, alto de línea).
- **Cómo lo resolví en CurseTerm**: pendiente.
- **Coste del workaround**: —
- **API propuesta para el kit**: `vow_design::MONO: Font` en `tokens.json`.
- **Gravedad**: baja
- **Cómo reproducirlo**: `grep -n '"family"' crates/vow-design/tengen/tokens.json`.

### BK-009 — Sin lienzo público y caché de párrafos apagada
- **Área**: texto / rendimiento
- **Qué necesitaba CurseTerm**: dibujar una rejilla de 80×24 a 300×100 celdas
  a 60 Hz.
- **Qué ofrece hoy BarrierKit**: `style::text`, `label` y `paragraph` de
  Freya. El kit dibuja a mano solo en módulos privados (`glass.rs:176`). La
  colección de fuentes apaga la caché de párrafos (`fonts.rs:20`), y Freya
  repinta el árbol entero en cada frame (ADR 0047).
- **Qué falta**: una primitiva de lienzo documentada que dé el `Canvas` de
  Skia, la colección de fuentes del kit y la escala.
- **Cómo lo resolví en CurseTerm**: pendiente. Plan: `ElementExt` propio de
  `freya-core`.
- **Coste del workaround**: depender de una API interna de Freya que el kit no
  documenta.
- **API propuesta para el kit**: `canvas(|ctx: &mut CanvasContext| { .. })` con `ctx.canvas()`, `ctx.fonts()` y `ctx.scale()`.
- **Gravedad**: media
- **Cómo reproducirlo**: buscar «canvas» en `docs/barrierkit/`: no aparece.

### BK-010 — Títulos de acción estáticos y menú sin reemplazo en vivo
- **Área**: menú global
- **Qué necesitaba CurseTerm**: «Profiles» se rellena con los perfiles
  guardados y «Window» con las ventanas abiertas.
- **Qué ofrece hoy BarrierKit**: `Action::new(id, title: fn() -> &'static str)`
  (`app/action.rs:79`); `WindowOptions::menu` fija el menú al crear la
  ventana. `MenuHandle::update(id, f)` (`app_menu.rs`) cambia un ítem o los
  hijos de un `Submenu` con id; `replace_menu` existe solo bajo `#[cfg(test)]`
  (`app_menu.rs:110`).
- **Qué falta**: acciones con título calculado y un hook documentado para
  menús dinámicos.
- **Cómo lo resolví en CurseTerm**: pendiente. Plan: `MenuItem::action` con
  `String` para las entradas dinámicas y `MenuHandle::update` sobre el
  `Submenu`.
- **Coste del workaround**: las entradas dinámicas no son `Action`, así que no
  tienen `Shortcut` tipado.
- **API propuesta para el kit**: `use_menu(|menu: &mut AppMenu| ..)` que se vuelve a evaluar cuando cambia su estado.
- **Gravedad**: media
- **Cómo reproducirlo**: intentar `Action::new("profile-1", || nombre_cargado)`:
  no compila, porque el título no puede capturar.

### BK-011 — Documentación que no coincide con el código
- **Área**: docs
- Ver «Documentación de BarrierKit», abajo.
- **Gravedad**: baja

## Cosas que funcionaron bien

Pendiente: se rellena al ejecutar CurseTerm. Se comprobarán explícitamente la
titlebar y sus estados, el menú global (submenús, checks, estados), el
portapapeles de texto, el cursor por forma, el menú contextual, la escala
fraccional, el modo claro/oscuro con `vow-appearance` y el arranque fuera de
Malaysia.

## Documentación de BarrierKit

- `docs/barrierkit/04-apariencia-y-tokens.md:33` dice que los JSON generan
  Slint, CSS y QML en `tengen/generated/`; esa carpeta no existe en `ba58ce0`.
- `docs/barrierkit/01-que-es.md:30` cuenta «cinco apps de prueba»; hay tres.
- `docs/barrierkit/11-app.md` no menciona `MenuHandle` ni `update`, la única
  vía para cambiar el menú en vivo (la usa `app/undo.rs:111`).
- `docs/barrierkit/11-app.md` no dice que un atajo despachado llega también
  como tecla a la app.
