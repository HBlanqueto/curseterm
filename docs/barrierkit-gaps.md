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

Cada gap dice cómo se resuelve (ADR 0001 de CurseTerm):

- **bloque**: componente en `crates/barrier-kit-next`, con la ruta de módulo
  que tendría en `barrier-kit`; se traslada moviendo el archivo;
- **parche**: cambio del runner en `kit-parches/NNNN-*.patch` contra el `rev`
  fijado, probado con `./dev.sh kit-parches`;
- **workaround**: lo que hace CurseTerm con el kit tal cual.

## Tabla resumen

| ID | Área | Gap | Gravedad | Workaround en CurseTerm |
|---|---|---|---|---|
| BK-001 | ventana | Una ventana por proceso | alta | Workaround: un proceso por ventana (`windows.rs`) |
| BK-002 | ventana | Título de ventana fijo | media | Parche 0001 `set_title`; tal cual, solo la `Titlebar` |
| BK-003 | input / menú global | Atajos no consumidos y `ShortcutKey` limitado | alta | Parche 0002; tal cual, el codificador ignora Super |
| BK-004 | IME | Sin `zwp_text_input_v3` | media | Ninguno |
| BK-005 | portapapeles | Sin selección primaria | media | Ninguno |
| BK-006 | contenedores | Sin pestañas de documento | media | Bloque `views::tab_bar` |
| BK-007 | contenedores | Sin divisor genérico anidable | media | Pendiente (fase 2) |
| BK-008 | tokens / texto | Sin cara monoespaciada en Tengen | baja | Bloque `tokens::MONO` |
| BK-009 | texto / rendimiento | Sin lienzo público y caché de párrafos apagada | media | Bloque `canvas` |
| BK-010 | menú global | Títulos de `Action` estáticos y menú sin reemplazo en vivo | media | Bloque `app_menu::use_menu_enabled`; submenús en la fase 2 |
| BK-011 | docs | Documentación que no coincide con el código | baja | — |
| BK-012 | ventana / persistencia | El tamaño guardado es uno por `app_id` | media | Workaround: `KeepTerminalSize` |
| BK-013 | ventana | El cierre pedido por el compositor no se puede interceptar | media | Ninguno |
| BK-014 | plataforma | `Timers::wakeup` no se puede dar de baja | baja | Workaround: un solo canal por ventana |
| BK-015 | persistencia | Las preferencias van siempre a `~/.config/malaysia/` | baja | Se acepta |
| BK-016 | input / menú global | Los atajos con Mayús comparan el carácter producido | media | Pendiente (fase 2) |

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
- **Cómo lo resolví en CurseTerm**: workaround, sin probar todavía. Cada
  ventana es un proceso (`crates/curseterm/src/windows.rs`): «New Window»,
  «About» y «Settings…» lanzan el ejecutable con `--about` o `--settings`; cada
  proceso se registra por pid en `$XDG_RUNTIME_DIR/curseterm/windows/` y «Quit»
  manda `SIGTERM` a los demás (comprueba antes `/proc/<pid>/exe`). Ajustes
  escribe el `.domain` y las ventanas lo leen con `use_defaults`.
- **Coste del workaround**: un contexto Vulkan y Skia por ventana; el estado
  compartido pasa por ficheros; «Quit» no puede preguntar antes de cerrar en
  los otros procesos; no se puede listar ni mover pestañas entre ventanas.
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
- **Cómo lo resolví en CurseTerm**: parche
  [`kit-parches/0001-titulo-en-vivo.patch`](../kit-parches/0001-titulo-en-vivo.patch)
  (`WindowControls::set_title` y `WindowCommand::SetTitle`, con su línea en
  `docs/barrierkit/06-ventanas{,.en}.md`). Con la feature `kit-parcheado`,
  `terminal.rs` lo llama al cambiar el título; sin ella solo cambia la
  `Titlebar`. Sin probar todavía.
- **Coste del workaround**: sin el parche, el compositor y la shell muestran
  siempre «CurseTerm».
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
- **Cómo lo resolví en CurseTerm**: parche
  [`kit-parches/0002-atajos-con-nombre-y-consumidos.patch`](../kit-parches/0002-atajos-con-nombre-y-consumidos.patch):
  `ShortcutKey::Named(NamedKey)` y `key_event` no reenvía la tecla de un atajo
  despachado; amplía el test `a_typed_shortcut_and_its_menu_text_match_the_same_keys`
  y `docs/barrierkit/11-app{,.en}.md`. Con el kit tal cual,
  `crates/curseterm/src/keys.rs` nunca envía al PTY una tecla con Super
  (ADR 0003), así que los atajos de CurseTerm no llegan a la shell. Sin probar
  todavía.
- **Coste del workaround**: solo funciona porque todos los atajos usan Super; un
  atajo sin Super llegaría dos veces. Las teclas sin tipo se declaran como
  texto en `MenuItem`.
- **API propuesta para el kit**: `enum ShortcutKey { Char(char), Named(NamedKey) }` y que `key_event` no empuje el `PlatformEvent::Keyboard` cuando `shortcut_action.is_some()`.
- **Gravedad**: alta
- **Cómo reproducirlo**: una `Action` con `Shortcut::ctrl(ShortcutKey::Char('d'))`
  y un `on_global_key_down` en la raíz: los dos se disparan con Ctrl+D.

### BK-004 — Sin método de entrada
- **Área**: IME
- **Qué necesitaba CurseTerm**: escribir en chino, japonés o coreano y ver el
  texto en preparación en la posición del cursor.
- **Qué ofrece hoy BarrierKit**: teclado con xkb vía SCTK; ningún módulo usa
  `zwp_text_input_v3`. Freya 0.4.3 ya tiene el evento `on_ime_preedit`
  (`ImePreeditEventData`), pero el runner nunca lo emite.
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
- **Cómo lo resolví en CurseTerm**: bloque `barrier_kit_next::views::tab_bar::TabBar`
  (`crates/barrier-kit-next/src/views/tab_bar.rs`): títulos `String`, botón de
  cerrar en la pestaña elegida o bajo el puntero, `use_interaction` y
  `Palette`; su alto es `tokens::TAB_BAR`. Falta «+», reordenar e indicador de
  actividad (fase 2). Sin probar todavía.
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
- **Cómo lo resolví en CurseTerm**: bloque `barrier_kit_next::tokens::MONO`
  (Geist Mono 13/16, que ya trae el dev shell del kit en `geist-font`).
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
- **Cómo lo resolví en CurseTerm**: bloque `barrier_kit_next::canvas::canvas(revision, painter)`
  (`crates/barrier-kit-next/src/canvas.rs`): un `ElementExt` que llena su padre,
  recorta, traslada y escala a píxeles lógicos y llama al pintor con el
  `Canvas` de Skia y la `FontCollection`; vuelve a pintar cuando cambia
  `revision`. La rejilla del terminal (`crates/curseterm/src/grid.rs`) lo usa.
  Sin probar todavía.
- **Coste del workaround**: depende de `ElementExt`, API de Freya que el kit no
  documenta; la rejilla dibuja celda a celda.
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
- **Cómo lo resolví en CurseTerm**: bloque
  `barrier_kit_next::app_menu::use_menu_enabled(id, enabled)`, la versión
  pública del `use_menu_enabled` privado de `app/undo.rs`; «Copy» se apaga sin
  selección. Para «Profiles» y «Window» (fase 2): `MenuItem::action` con
  `String` y `MenuHandle::update` sobre el `Submenu`.
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

### BK-012 — El tamaño guardado es uno por `app_id`
- **Área**: ventana / persistencia
- **Qué necesitaba CurseTerm**: abrir «About» (tamaño fijo) sin cambiar el
  tamaño con el que se abre la siguiente ventana de terminal.
- **Qué ofrece hoy BarrierKit**: `platform/window.rs:333` guarda
  `<app_id>.window` al cerrar cualquier ventana y lo restaura en todas las del
  mismo `app_id` (`:373`).
- **Qué falta**: una clave de restauración por tipo de ventana.
- **Cómo lo resolví en CurseTerm**: `windows::KeepTerminalSize` lee ese estado
  antes de `run_window` en las ventanas auxiliares y lo vuelve a escribir al
  salir.
- **Coste del workaround**: lee y escribe un fichero privado del kit por su
  nombre.
- **API propuesta para el kit**: `WindowOptions::restoration_key(impl Into<String>)`, o no guardar tamaño en ventanas de tamaño fijo.
- **Gravedad**: media
- **Cómo reproducirlo**: dos ventanas del mismo `app_id`, una fija de 320×280;
  cerrar la fija y abrir la otra: abre a 320×280 recortado a su mínimo.

### BK-013 — El cierre pedido por el compositor no se puede interceptar
- **Área**: ventana
- **Qué necesitaba CurseTerm**: preguntar «¿Cerrar la ventana? Hay N
  sesiones en marcha», como iTerm2.
- **Qué ofrece hoy BarrierKit**: `WindowHandler::request_close`
  (`platform/window.rs`) llama a `close_window` sin avisar a la app.
- **Qué falta**: un aviso que la app pueda cancelar.
- **Cómo lo resolví en CurseTerm**: ninguno; la ventana se cierra sin preguntar.
- **Coste del workaround**: —
- **API propuesta para el kit**: `use_close_request(|| CloseResponse::Allow | Cancel)`.
- **Gravedad**: media
- **Cómo reproducirlo**: en Malaysia, Super+W sobre una ventana con un programa
  en marcha.

### BK-014 — `Timers::wakeup` no se puede dar de baja
- **Área**: plataforma
- **Qué necesitaba CurseTerm**: despertar el hilo de interfaz desde el hilo de
  E/S de cada sesión.
- **Qué ofrece hoy BarrierKit**: `Timers::wakeup(callback) -> WakeHandle`
  (`platform/timer.rs`); el callback queda registrado para siempre y corre en
  cada despertar de cualquier origen (`platform/runtime.rs:213`).
- **Qué falta**: dar de baja el callback al desmontar.
- **Cómo lo resolví en CurseTerm**: un solo `wakeup` por ventana y un canal
  compartido por todas sus sesiones (`terminal.rs`, `listen`).
- **Coste del workaround**: ninguno apreciable.
- **API propuesta para el kit**: `use_wakeup(callback) -> WakeHandle` que se dé de baja con `use_drop`.
- **Gravedad**: baja
- **Cómo reproducirlo**: llamar a `wakeup` en un componente que se monta y
  desmonta: el callback sigue corriendo.

### BK-015 — Las preferencias van siempre a `~/.config/malaysia/`
- **Área**: persistencia
- **Qué necesitaba CurseTerm**: el encargo sugería `~/.config/curseterm/`.
- **Qué ofrece hoy BarrierKit**: `load_defaults`, `save_defaults` y
  `use_defaults` (`app/defaults.rs`) usan `~/.config/malaysia/<app_id>.domain`,
  y `use_watched` solo vigila las carpetas de Malaysia.
- **Qué falta**: nada para una app del escritorio; para una app de terceros
  que corre en otros escritorios, una carpeta propia.
- **Cómo lo resolví en CurseTerm**: se acepta; las preferencias están en
  `~/.config/malaysia/org.curseterm.CurseTerm.domain`.
- **Coste del workaround**: —
- **API propuesta para el kit**: ninguna por ahora.
- **Gravedad**: baja
- **Cómo reproducirlo**: leer `app/defaults.rs:65`.

### BK-016 — Los atajos con Mayús comparan el carácter producido
- **Área**: input / menú global
- **Qué necesitaba CurseTerm**: ⇧⌘] y ⇧⌘[ de iTerm2 (pestaña siguiente y
  anterior) como Super+Mayús+] y Super+Mayús+[.
- **Qué ofrece hoy BarrierKit**: `shortcut_matches` (`menu/shortcut.rs`)
  compara el glifo del atajo con `key.to_string()`. Con Mayús, en un teclado
  US, la tecla `]` llega como `}`.
- **Qué falta**: comparar por tecla física (`Code`) o por keysym sin Mayús.
- **Cómo lo resolví en CurseTerm**: pendiente (fase 2).
- **Coste del workaround**: —
- **API propuesta para el kit**: que `find_menu_shortcut` reciba también el `Code` y compare por él cuando hay Mayús.
- **Gravedad**: media
- **Cómo reproducirlo**: un `MenuItem` con `"Super+Shift+bracketright"`;
  pulsar Super+Mayús+] no lo dispara.

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
