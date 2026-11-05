# Lo que le faltó a BarrierKit al construir CurseTerm

Versión/commit de BarrierKit usado: `ba58ce01ef1f8e07dd915f783290b6ef068c037a` (`main`, 2026-11-05)

Resumen: documento vivo; estado al cerrar la fase 7 (2026-11-05), probado en
sway sin pantalla con lavapipe (`progreso.md`). Lo más serio: los handlers de
`use_menu_action` corren fuera del contexto de Freya y hacen abortar la app si
tocan el portapapeles o `WindowControls::get()` (BK-017); el menú solo se
publica en D-Bus con `org_kde_kwin_appmenu_manager`, al contrario de lo que
dice la documentación (BK-018); una ventana por proceso y sin título en vivo
(BK-001, BK-002); sin token de activación para apps (BK-019); sin IME ni
selección primaria; sin atajos globales para la ventana hotkey (BK-025) ni
campana o notificaciones (BK-026). El lienzo, las pestañas, los paneles divididos, el
token `MONO` y el estado vivo del menú se resolvieron como bloques; el título,
los atajos, el menú en D-Bus, los menús de primer nivel en vivo y el repintado
sin foco, como parches probados.

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
| BK-001 | ventana | Una ventana por proceso | alta | Workaround probado: un proceso por ventana (`windows.rs`) |
| BK-002 | ventana | Título de ventana fijo | media | Parche 0001 probado; tal cual, solo la `Titlebar` |
| BK-003 | input / menú global | Atajos no consumidos y `ShortcutKey` limitado | alta | Parche 0002 (tests del kit); tal cual, el codificador ignora Super |
| BK-004 | IME | Sin `zwp_text_input_v3` | media | Ninguno |
| BK-005 | portapapeles | Sin selección primaria | media | Ninguno |
| BK-006 | contenedores | Sin pestañas de documento | media | Bloque probado `views::tab_bar` |
| BK-007 | contenedores | Sin divisor genérico anidable | media | Bloque probado `views::split_panes` |
| BK-008 | tokens / texto | Sin cara monoespaciada ni métricas de terminal en Tengen | baja | Bloques `tokens::MONO` y `tokens::TERMINAL` |
| BK-009 | texto / rendimiento | Sin lienzo público y caché de párrafos apagada | media | Bloque `canvas` |
| BK-010 | menú global | Títulos de `Action` estáticos y menú sin reemplazo en vivo | media | Bloque probado `app_menu`; parche 0004 probado para menús de primer nivel |
| BK-011 | docs | Documentación que no coincide con el código | baja | — |
| BK-012 | ventana / persistencia | El tamaño guardado es uno por `app_id` y no se puede pedir otro | media | Workaround probado: `forget_saved_size` |
| BK-013 | ventana | El cierre pedido por el compositor no se puede interceptar | media | Ninguno |
| BK-014 | plataforma | `Timers::wakeup` no se puede dar de baja | baja | Workaround: un solo canal por ventana |
| BK-015 | persistencia | Las preferencias van siempre a `~/.config/malaysia/` | baja | Se acepta |
| BK-016 | input / menú global | Los atajos con Mayús comparan el carácter producido | media | Workaround: atajo por texto con `}` y `{` |
| BK-017 | menú global / plataforma | Los handlers del menú corren fuera del contexto de Freya | **bloqueante** | Workaround probado: handles tomados en `render` |
| BK-018 | menú global / docs | Sin `org_kde_kwin_appmenu_manager` el menú no se publica en D-Bus | alta | Parche 0003 probado |
| BK-019 | ventana / plataforma | Una app no puede pedir un token de xdg-activation | media | Ninguno; sin lista de ventanas en «Window» |
| BK-020 | controles | Las etiquetas de paneles, filas y desplegables son `&'static str` | media | Workaround: `intern` en `preferences.rs` |
| BK-021 | controles | `RadioGroup` es siempre horizontal | baja | Workaround: un grupo de una opción por fila |
| BK-022 | plataforma / persistencia | Un cambio de `.domain` no repinta las ventanas sin foco | alta | Parche 0005 probado |
| BK-023 | app | Sin selector de archivos | media | Workaround: ruta en un campo y `--import-colors` |
| BK-024 | símbolos | Faltan símbolos para una barra de ajustes y para la campana | baja | Se usan `CircleHalf`, `ListBullet` y `SpeakerWave` |
| BK-025 | plataforma / input | Sin atajos globales ni forma de ocultar y mostrar la ventana | media | Workaround probado: `curseterm --hotkey` |
| BK-026 | app / plataforma | Sin campana, petición de atención ni notificaciones | baja | Workaround probado: campana visual |
| BK-027 | controles / input | Una vista con teclado global no sabe si un campo de texto o un `Alert` se quedan la tecla | baja | Workaround probado: foco seguido y cierre diferido del `Alert` |
| BK-028 | controles | Las etiquetas de `SegmentedControl` se parten en dos líneas | baja | Workaround: Ajustes a 980 px |

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
- **Cómo lo resolví en CurseTerm**: workaround, probado en sway: New Window
  abre un proceso nuevo y Quit cierra todos. Cada ventana es un proceso (`crates/curseterm/src/windows.rs`): «New Window»,
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
  `Titlebar`. Probado: en sway, con el parche, la barra del compositor muestra
  `root@vm: /tmp` tras `cd /tmp`; sin él, siempre «CurseTerm».
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
  (ADR 0003), así que los atajos de CurseTerm no llegan a la shell. El parche
  pasa `cargo test -p barrier-kit --lib` (incluido el test ampliado) y
  `clippy -D warnings`; los atajos Super funcionan en los dos modos.
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
  `Palette`; su alto es `tokens::TAB_BAR`; `on_add(label, ..)` añade el
  botón «+». Probado: pestañas con título por OSC, elegir, cerrar y «+»
  (`docs/capturas/fase2-barra-de-pestanas.png`). Falta reordenar arrastrando
  e indicador de actividad.
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
- **Cómo lo resolví en CurseTerm**: bloque
  `barrier_kit_next::views::split_panes::SplitPanes`
  (`crates/barrier-kit-next/src/views/split_panes.rs`): N paneles en un eje
  con `Size::flex(ratio)`, divisores de `tokens::SPLIT_PANES.divider` que se
  arrastran (`ColResize`/`RowResize`), mínimo `pane_min` por panel; controlado
  con `.ratios(..)` y `.on_ratios(..)`; se anida poniendo un `SplitPanes` como
  panel. CurseTerm guarda el árbol en `panes.rs`. Probado: divisiones
  anidadas, arrastre y teclado (`docs/capturas/fase2-paneles.png`,
  `fase2-divisor-arrastrado.png`).
- **Coste del workaround**: el arrastre usa las proporciones del último
  render; con muchos eventos entre dos frames se pierde algo de recorrido.
- **API propuesta para el kit**: la del bloque: `SplitPanes::new(label, Direction).pane(a).pane(b).ratios(Vec<f32>).on_ratios(cb)`.
- **Gravedad**: media
- **Cómo reproducirlo**: `SplitView` no tiene opción de dirección.

### BK-008 — Sin cara monoespaciada en Tengen
- **Área**: tokens / texto
- **Qué necesitaba CurseTerm**: la fuente del terminal por defecto y textos de
  código en Preferencias.
- **Qué ofrece hoy BarrierKit**: `vow_design::Font` con `family`; en
  `crates/vow-design/tengen/tokens.json:285-321` todas son `sans`. ADR 0047
  menciona un estilo «Mono» que no existe.
- **Qué falta**: un token `MONO` (familia, tamaño, alto de línea) y las
  métricas de una vista de terminal: alfa de las franjas de broadcast, del
  resaltado y de la campana, ancho de franja, intervalo de multiclic y rango
  de tamaños de letra; también los tamaños de las ventanas de terminal,
  Ajustes y Acerca de.
- **Cómo lo resolví en CurseTerm**: bloque `barrier_kit_next::tokens::MONO`
  (Geist Mono 13/16, que ya trae el dev shell del kit en `geist-font`), y
  `tokens::TERMINAL`, `SETTINGS_WINDOW_SIZE` y `ABOUT_WINDOW_SIZE` (el
  terminal se mide en celdas, BK-012).
- **Coste del workaround**: —
- **API propuesta para el kit**: `vow_design::MONO: Font` en `tokens.json` y `vow_design::TERMINAL: TerminalMetrics` en `components.json`.
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
  Probado a escala 1 y 1,5, sin costuras; en reposo no produce frames. La app
  debe meter en `revision` todo lo que cambia el dibujo (foco, esquema,
  fuente): el lienzo no ve el estado que lee el pintor.
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
  selección. En la fase 2 se generalizó: `use_menu_update(id, value, apply)`,
  `use_menu_checked`, `use_submenu_items` y `submenu(id, ..)` (no hay
  constructor de `MenuItem::Submenu` con id). Probado por D-Bus: «Select Tab»
  se rellena con las pestañas y marca la activa; «Show Tab Bar» y «Maximize
  Active Pane» llevan marca; una docena de entradas se activan según el
  contexto. Para el menú «Profiles», que es de primer nivel, `update` no
  sirve: el parche
  [`kit-parches/0004-items-de-un-menu-en-vivo.patch`](../kit-parches/0004-items-de-un-menu-en-vivo.patch)
  añade `MenuHandle::set_menu_items(título, ítems)` y documenta `MenuHandle`
  en `11-app{,.en}.md`. Probado por D-Bus: «+» en Ajustes añade «New Profile»
  al menú del terminal al momento. Con el kit tal cual solo se renombran los
  ítems que ya existían.
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

### BK-012 — El tamaño guardado es uno por `app_id` y no se puede pedir otro
- **Área**: ventana / persistencia
- **Qué necesitaba CurseTerm**: abrir «About» (tamaño fijo) sin cambiar el
  tamaño con el que se abre la siguiente ventana de terminal; y, desde la
  fase 6, abrir cada ventana nueva con las columnas y filas del perfil, como
  iTerm2.
- **Qué ofrece hoy BarrierKit**: `platform/window.rs:333` guarda
  `<app_id>.window` al cerrar cualquier ventana y lo restaura en todas las del
  mismo `app_id` (`:373`).
  El tamaño restaurado gana siempre sobre `WindowOptions::size`, y
  `WindowControls` no puede pedir otro tamaño después.
- **Qué falta**: una clave de restauración por tipo de ventana y poder
  abrir con un tamaño propio aunque haya uno guardado.
- **Cómo lo resolví en CurseTerm**: `windows::forget_saved_size` vacía ese
  estado antes de cada `run_window`; el terminal calcula su tamaño con las
  métricas de la fuente del perfil (`main.rs::terminal_size`) y cada ventana
  auxiliar usa el suyo.
- **Coste del workaround**: escribe un fichero privado del kit por su nombre;
  ninguna ventana recuerda su tamaño ni si estaba maximizada.
- **API propuesta para el kit**: `WindowOptions::restoration_key(impl Into<String>)` y `WindowOptions::restore(false)`.
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
- **Cómo lo resolví en CurseTerm**: «Select Next Tab» y «Select Previous
  Tab» se declaran por texto como `Super+Shift+}` y `Super+Shift+{`
  (`crates/curseterm/src/menu.rs`). Probado con `wtype`.
- **Coste del workaround**: depende de la distribución de teclado; el menú muestra `}` en vez de `]`.
- **API propuesta para el kit**: que `find_menu_shortcut` reciba también el `Code` y compare por él cuando hay Mayús.
- **Gravedad**: media
- **Cómo reproducirlo**: un `MenuItem` con `"Super+Shift+bracketright"`;
  pulsar Super+Mayús+] no lo dispara.

### BK-017 — Los handlers del menú corren fuera del contexto de Freya
- **Área**: menú global / plataforma
- **Qué necesitaba CurseTerm**: copiar, pegar y cerrar desde «Edit › Copy»,
  «Paste» y «Shell › Close», o desde sus atajos.
- **Qué ofrece hoy BarrierKit**: `use_menu_action` (`app/action.rs`). El
  runner llama a `MenuActions::dispatch` desde `Runtime::key_event`
  (`platform/input/seat.rs:454`) y desde las acciones de D-Bus, fuera del
  contexto de Freya. `docs/barrierkit/11-app.md` solo dice que corre «en el
  hilo de la interfaz». Lo mismo pasa con los callbacks de `Timers::wakeup`.
- **Qué falta**: ejecutar los handlers dentro del contexto del árbol, como un
  evento, o documentar qué APIs no se pueden usar en ellos.
- **Cómo lo resolví en CurseTerm**: `Handles` en `terminal.rs` guarda el
  `State` del portapapeles (`consume_root_context`) y `WindowControls` durante
  `render`; los handlers y los callbacks de `wakeup` usan esas copias. Las
  ventanas auxiliares capturan `WindowControls` igual. En la fase 4,
  `Timers::get()` dentro de un callback de `wakeup` también abortó la app
  (campana de un trigger): `Handles` guarda además `Timers`.
- **Coste del workaround**: la app usa el `State` interno de
  `freya-clipboard` en vez de `Clipboard`.
- **API propuesta para el kit**: que `MenuActions::dispatch` corra dentro de `with_context` del árbol de la ventana, o que el handler reciba un `MenuContext` con portapapeles y ventana.
- **Gravedad**: bloqueante (la app aborta con
  «Your trying to access Freya's current context outside of it»).
- **Cómo reproducirlo**: `use_menu_action(|id| { let _ = Clipboard::set(id.into()); })`
  y pulsar el atajo de esa acción.

### BK-018 — Sin `org_kde_kwin_appmenu_manager` el menú no se publica en D-Bus
- **Área**: menú global / docs
- **Qué necesitaba CurseTerm**: el menú global en GNOME o KDE sin Malaysia, y
  poder comprobarlo por D-Bus.
- **Qué ofrece hoy BarrierKit**: `README.md` y `docs/barrierkit/11-app.md`
  dicen que sin el protocolo el menú «sigue existiendo en D-Bus», pero
  `platform/window.rs:407-419` solo llama a `app_menu::server::register` si
  existe `appmenu_manager`.
- **Qué falta**: registrar siempre el menú y anunciarlo solo si hay protocolo.
- **Cómo lo resolví en CurseTerm**: parche
  [`kit-parches/0003-menu-en-dbus-sin-appmenu.patch`](../kit-parches/0003-menu-en-dbus-sin-appmenu.patch).
  Probado en sway: `com.canonical.dbusmenu.GetLayout` en `/MenuBar/0` devuelve
  CurseTerm, Shell y Edit con sus atajos; «Copy» pasa de `enabled=false` a
  `true` al seleccionar; `Event(clicked)` sobre «New Tab» abre la pestaña.
- **Coste del workaround**: con el kit tal cual no hay forma de comprobar el
  menú fuera de Malaysia.
- **API propuesta para el kit**: ninguna; es el cambio del parche.
- **Gravedad**: alta
- **Cómo reproducirlo**: abrir cualquier app con `.menu(..)` en sway y listar
  los objetos de su nombre en el bus: no hay `/MenuBar/*`.

### BK-019 — Una app no puede pedir un token de xdg-activation
- **Área**: ventana / plataforma
- **Qué necesitaba CurseTerm**: la lista de ventanas abiertas del menú
  «Window» de iTerm2, que trae al frente la elegida.
- **Qué ofrece hoy BarrierKit**: `WindowControls::present(token)`
  (`platform/window.rs:207`) usa un token que ya se tiene; pedirlo
  (`xdg_activation_v1.get_activation_token`) solo existe en
  `shell::Activation` (`shell/activation.rs`), tras la feature `shell`.
- **Qué falta**: `Activation::request` para cualquier app, con la superficie y
  el último serial de entrada de la ventana.
- **Cómo lo resolví en CurseTerm**: no se muestra la lista; una entrada que no
  traiga la ventana sería decorativa. En Malaysia, la barra de menús ya añade
  a «Window» las ventanas de la app.
- **Coste del workaround**: fuera de Malaysia no hay forma de saltar a otra
  ventana de CurseTerm desde el menú.
- **API propuesta para el kit**: mover `Activation` de `shell/` a `platform/` y darle `WindowControls::activation_token(callback)`.
- **Gravedad**: media
- **Cómo reproducirlo**: buscar `get_activation_token` fuera de
  `crates/barrier-kit/src/shell/`: no aparece.

### BK-020 — Las etiquetas de paneles, filas y desplegables son `&'static str`
- **Área**: controles
- **Qué necesitaba CurseTerm**: nombres de perfil y de esquemas importados, que escribe el usuario, como ítems de `PopUpButton` y títulos de `Pane`.
- **Qué ofrece hoy BarrierKit**: `PopUpButton::item(&'static str)`
  (`popup_button.rs:38`), `Row::new(&'static str)` (`views/form.rs:31`),
  `Pane::new(&'static str, &'static str, ..)` (`patterns/preferences.rs:35`),
  `RadioGroup::option(&'static str)`. Pensado para `tr!`, que devuelve
  `&'static str`.
- **Qué falta**: aceptar `impl Into<Cow<'static, str>>` como ya hacen
  `Button::new` y `about_window`.
- **Cómo lo resolví en CurseTerm**: `intern` en `crates/curseterm/src/preferences.rs`
  filtra cada nombre una vez a un `&'static str` y lo reutiliza.
- **Coste del workaround**: memoria que no se libera por cada nombre distinto
  (pequeña en la práctica).
- **API propuesta para el kit**: `fn item(self, label: impl Into<Cow<'static, str>>)` y lo mismo en `Row`, `Pane` y `RadioGroup`.
- **Gravedad**: media
- **Cómo reproducirlo**: `PopUpButton::new("x").item(nombre.as_str())` no compila.

### BK-021 — `RadioGroup` es siempre horizontal
- **Área**: controles
- **Qué necesitaba CurseTerm**: «Initial directory» de iTerm2: tres opciones en columna.
- **Qué ofrece hoy BarrierKit**: `choice_group` fija `Direction::Horizontal`
  (`choices.rs:50`); con tres opciones largas la última se sale de la fila.
- **Qué falta**: `.direction(Direction::Vertical)` o una variante en columna.
- **Cómo lo resolví en CurseTerm**: `directory_choices` en `preferences.rs`
  apila tres `RadioGroup` de una opción, controlados desde el perfil.
- **Coste del workaround**: el lector de pantalla ve tres grupos en vez de uno.
- **API propuesta para el kit**: `RadioGroup::new(label).vertical(true)`.
- **Gravedad**: baja
- **Cómo reproducirlo**: un `RadioGroup` con tres opciones en una `Row` estrecha.

### BK-022 — Un cambio de `.domain` no repinta las ventanas sin foco
- **Área**: plataforma / persistencia
- **Qué necesitaba CurseTerm**: que el terminal cambie de esquema en cuanto
  se elige en Ajustes, que es otro proceso y tiene el foco.
- **Qué ofrece hoy BarrierKit**: al cambiar un `.domain`, el runner sube
  `defaults_revision` y hace `ping` (`platform/runtime.rs:255`), pero no marca
  `request_redraw_all`, como sí hace con la apariencia (`:263`).
- **Qué falta**: pedir el repintado también en ese caso.
- **Cómo lo resolví en CurseTerm**: parche
  [`kit-parches/0005-domain-repinta-sin-foco.patch`](../kit-parches/0005-domain-repinta-sin-foco.patch).
  Probado en sway: con Ajustes delante y el terminal sin foco, editar el
  `.domain` no cambia el terminal con el kit tal cual y sí con el parche
  (`docs/capturas/fase3-repintado-sin-foco-parche-0005.png`).
- **Coste del workaround**: con el kit tal cual, el cambio aparece cuando la
  ventana recibe cualquier otro evento.
- **API propuesta para el kit**: ninguna; es el cambio del parche.
- **Gravedad**: alta
- **Cómo reproducirlo**: dos ventanas que usan `use_defaults`; con el foco en
  una, cambiar el `.domain` desde ella: la otra no se repinta.

### BK-023 — Sin selector de archivos
- **Área**: app
- **Qué necesitaba CurseTerm**: «Import…» de esquemas `.itermcolors`.
- **Qué ofrece hoy BarrierKit**: nada; ADR 0047 deja «`Open` de archivos y
  URLs» pendiente hasta la primera app que lo use. Freya tiene
  `on_file_drop`, pero el runner no lo emite.
- **Qué falta**: un diálogo de abrir por el portal
  `org.freedesktop.portal.FileChooser` (Malaysia tiene su
  `xdg-desktop-portal-malaysia`).
- **Cómo lo resolví en CurseTerm**: un `TextField` con la ruta y el botón
  «Import» en Colors, y `curseterm --import-colors <archivo>`.
- **Coste del workaround**: hay que escribir la ruta a mano.
- **API propuesta para el kit**: `app::open_file(filters: &[(&str, &[&str])], callback: impl FnOnce(Option<PathBuf>))`.
- **Gravedad**: media
- **Cómo reproducirlo**: buscar `FileChooser` en `crates/barrier-kit/src`: no aparece.

### BK-024 — Faltan símbolos para una barra de ajustes y para la campana
- **Área**: símbolos
- **Qué necesitaba CurseTerm**: los iconos de la barra de Settings de iTerm2
  (engranaje, ojo, persona, teclado, puntero, rayo) y la campana que marca
  una pestaña de fondo donde ha sonado (fase 5).
- **Qué ofrece hoy BarrierKit**: 31 símbolos en `symbol.rs`; ninguno de esos.
- **Qué falta**: ampliar el registro.
- **Cómo lo resolví en CurseTerm**: `CircleHalf` para Appearance,
  `ListBullet` para Profiles y `SpeakerWave` en la insignia de la pestaña
  (`Badge` del bloque `views::tab_bar`).
- **Coste del workaround**: menos parecido a iTerm2.
- **API propuesta para el kit**: `Symbol::{Gear, Eye, Person, Keyboard, Pointer, Bolt, Bell}`.
- **Gravedad**: baja
- **Cómo reproducirlo**: leer `enum Symbol` en `crates/barrier-kit/src/symbol.rs`.

### BK-025 — Sin atajos globales ni forma de ocultar y mostrar la ventana
- **Área**: plataforma / input
- **Qué necesitaba CurseTerm**: la ventana hotkey de iTerm2: un atajo de todo
  el sistema que muestra y oculta una ventana de terminal sin perder sus
  sesiones.
- **Qué ofrece hoy BarrierKit**: nada. Los atajos de `Action` solo llegan con
  la ventana enfocada; `WindowControls` puede minimizar, pero xdg-shell no
  deja a un cliente desminimizarse. El portal de Malaysia
  (`crates/xdg-desktop-portal-malaysia/src`) solo implementa `Settings`, y su
  auditoría (`docs/auditoria-claude-2026-11-03.md:613`) anota que el respaldo
  GTK no da `GlobalShortcuts`.
- **Qué falta**: registrar atajos con el portal
  `org.freedesktop.portal.GlobalShortcuts` (y que Malaysia lo implemente), y
  ocultar y volver a mostrar una ventana propia.
- **Cómo lo resolví en CurseTerm**: `curseterm --hotkey` (`windows.rs`)
  guarda su pid en `$XDG_RUNTIME_DIR/curseterm/hotkey`; si ya hay una ventana
  hotkey viva, la cierra con `SIGTERM` y sale; si no, se convierte en ella. El
  usuario lo enlaza a un atajo del escritorio.
- **Coste del workaround**: cerrar la ventana pierde sus sesiones; sin
  animación ni posición propia; hay que configurar el atajo fuera de la app.
- **API propuesta para el kit**: `app::global_shortcut(id, description, preferred: Shortcut, on_activated: impl Fn())` sobre el portal, y `WindowControls::{hide, show}`.
- **Gravedad**: media
- **Cómo reproducirlo**: buscar `GlobalShortcuts` en `crates/`: no aparece.

### BK-026 — Sin campana, petición de atención ni notificaciones
- **Área**: app / plataforma
- **Qué necesitaba CurseTerm**: la campana (BEL y el trigger «Ring Bell»),
  que en iTerm2 suena, pide atención si la ventana no tiene el foco y puede
  publicar una notificación.
- **Qué ofrece hoy BarrierKit**: nada; en `crates/barrier-kit/src` no hay
  sonido, urgencia ni notificaciones (solo aparece «notifies» en un test de
  menús).
- **Qué falta**: un sonido de evento del tema freedesktop y una notificación
  por `org.freedesktop.portal.Notification`.
- **Cómo lo resolví en CurseTerm**: campana visual: el panel se aclara un
  20 % con el color del texto durante `DURATION_FAST_MS` y `Timers::after`
  lo repinta al terminar.
- **Coste del workaround**: sin sonido; con la ventana sin foco nadie se
  entera.
- **API propuesta para el kit**: `app::beep()` y `app::notify(title, body)`.
- **Gravedad**: baja
- **Cómo reproducirlo**: `grep -rni "bell\|attention\|notif" crates/barrier-kit/src`.

### BK-027 — Una vista con teclado global no sabe si un campo de texto tiene el foco
- **Área**: controles / input
- **Qué necesitaba CurseTerm**: la barra de búsqueda (Edit › Find): lo que se
  escribe en ella no debe llegar a la shell, y Escape la cierra. En la fase
  6, el aviso «¿Cerrar?»: su Intro o su Escape no deben llegar a la shell.
- **Qué ofrece hoy BarrierKit**: `SearchField` con `auto_focus`,
  `on_focus_change` y `on_navigate` (Arriba, Abajo, Intro). Su
  `stop_propagation` no frena los `on_global_key_down`, que Freya deriva de
  cada `KeyDown` (`freya-core` 0.4.3, `events/name.rs:144`), y Escape no llega
  a `on_navigate` (`controls.rs:951`). `Alert` (`views/feedback.rs`) atiende
  Intro y Escape en su propio `on_global_key_down`, que tampoco frena a los
  demás.
- **Qué falta**: saber desde la vista si un campo de texto del kit tiene el
  foco, y un callback de cancelar.
- **Cómo lo resolví en CurseTerm**: `terminal.rs` sigue el foco del
  `SearchField` con `on_focus_change`; con la barra abierta y enfocada, el
  manejador global ignora las teclas y Escape cierra la barra. Con el aviso
  abierto el manejador ignora todo, y el aviso se quita con
  `Timers::after(Duration::ZERO)` (`dismiss_later`) para que el manejador
  aún lo vea abierto al recibir esa misma tecla.
- **Coste del workaround**: un estado más que se desincroniza si la barra se
  cierra sin pasar por `Find::close`.
- **API propuesta para el kit**: `SearchField::on_cancel` y
  `use_text_input_focused() -> bool`, y que `Alert` consuma sus teclas.
- **Gravedad**: baja
- **Cómo reproducirlo**: un `rect().on_global_key_down(…)` con un
  `SearchField` dentro: cada tecla del campo llega también al manejador.
  Detectado en el código; el workaround se puso antes de reproducirlo. Con
  `Alert` sí se reprodujo: sin `dismiss_later`, Escape cancelaba el aviso y
  la shell recibía `^[`.

### BK-028 — Las etiquetas de `SegmentedControl` se parten en dos líneas
- **Área**: controles
- **Qué necesitaba CurseTerm**: las ocho secciones del perfil de iTerm2
  (General, Colors, Text, Window, Terminal, Session, Keys, Advanced) en un
  `SegmentedControl`.
- **Qué ofrece hoy BarrierKit**: cada segmento ocupa `Size::flex(1.0)` y su
  `label()` no limita las líneas (`controls.rs:769-791`): a 860 px,
  «Advanced» sale partida en dos.
- **Qué falta**: una línea por etiqueta, con anchos según el texto o con
  desplazamiento cuando no caben.
- **Cómo lo resolví en CurseTerm**: `SETTINGS_WINDOW_SIZE` pasa a 980 × 600.
- **Coste del workaround**: por debajo de ese ancho la etiqueta se vuelve a
  partir.
- **API propuesta para el kit**: `label().max_lines(1)` en el segmento y anchos por contenido.
- **Gravedad**: baja
- **Cómo reproducirlo**: Ajustes de CurseTerm con `SETTINGS_WINDOW_SIZE` a 860 × 560, Perfiles.

## Cosas que funcionaron bien

Comprobado en la fase 1 (sway sin pantalla, lavapipe):

- **Arranque fuera de Malaysia:** el runner sigue sin
  `org_kde_kwin_appmenu_manager` ni los protocolos Infinity y lo registra con
  `tracing::info!`.
- **Render de rejilla a alta frecuencia:** con el bloque `canvas`, `seq 1 300`
  y el listado en color se dibujan sin retraso visible en un build de
  depuración, y en reposo no hay frames (1 % de CPU).
- **Métricas de fuente y respaldo:** Geist Mono del dev shell y el respaldo de
  Skia (`match_family_style_character`) para CJK; los anchos dobles encajan.
- **Escala fraccional:** a 1,5 el texto sale nítido y las celdas sin costuras.
- **Portapapeles de texto:** copiar y pegar con otras apps (`wl-copy`,
  `wl-paste`) funciona en los dos sentidos, también desde OSC 52.
- **Apariencia en vivo:** cambiar `appearance.domain` repinta al momento las
  ventanas del kit (Ajustes) y CurseTerm.
- **`use_defaults` entre procesos:** Ajustes guarda el `.domain` y la ventana
  del terminal lo recibe en vivo.
- **Patrones:** `PreferencesWindow`, `Form`, `Stepper`, `TextField`,
  `PopUpButton` y `about_window` sirvieron sin cambios; la barra lateral se
  pliega sola por debajo de 760 px.
- **Teclado:** `KeyboardEventData` trae `Key` y modificadores suficientes para
  un codificador xterm; Alt, Ctrl y las teclas con nombre llegan bien.
- **Estado vivo del menú:** `MenuHandle::update` publica el cambio por D-Bus
  (revisión 1 → 3 al seleccionar).
- **Cursor por forma:** `Cursor::set(CursorIcon::Text)` sobre la rejilla.

- **Controles para unos Ajustes como los de iTerm2 (fase 3):** `TabView` es
  justo la barra de pestañas con icono de la ventana Settings de iTerm2;
  `Table` con selección, `SegmentedControl`, `Switch`, `Stepper`,
  `PopUpButton` y `Form` cubrieron el editor de perfiles sin dibujar nada a
  mano.

- **Lo característico de iTerm2 (fase 4):** el bloque `canvas` dibujó sin
  elementos extra las franjas de broadcast, los triángulos de marca, el
  resaltado de triggers, la campana y las imágenes en línea, decodificadas con
  `Image::from_encoded` de `freya-engine`. `Timers::after` sirvió para apagar
  la campana; `SearchField` con `auto_focus` y `on_navigate` dio la barra de
  búsqueda; `use_submenu_items` reconstruye el grupo de radio de Broadcast
  Input y el host lo ve por D-Bus.

Sin probar todavía: menú contextual, arrastrar y soltar, menú global en la
barra de Malaysia, GNOME y KDE reales.

## Documentación de BarrierKit

- `docs/barrierkit/04-apariencia-y-tokens.md:33` dice que los JSON generan
  Slint, CSS y QML en `tengen/generated/`; esa carpeta no existe en `ba58ce0`.
- `docs/barrierkit/01-que-es.md:30` cuenta «cinco apps de prueba»; hay tres.
- `docs/barrierkit/11-app.md` no menciona `MenuHandle` ni `update`, la única
  vía para cambiar el menú en vivo (la usa `app/undo.rs:111`).
- `docs/barrierkit/11-app.md` no dice que un atajo despachado llega también
  como tecla a la app.
- `README.md` y `docs/barrierkit/11-app.md` dicen que sin
  `org_kde_kwin_appmenu_manager` el menú sigue en D-Bus; el código no lo
  publica (BK-018).
- `docs/barrierkit/11-app.md` dice que el handler de una acción corre «en el
  hilo de la interfaz», pero no avisa de que corre fuera del contexto de Freya
  (BK-017).
- `docs/barrierkit/07-titlebars.md` dice que `.close_only(true)` «deja solo el
  botón de cerrar»; el código (`traffic.rs:126`) dibuja también minimizar y
  maximizar, en gris.
- `docs/barrierkit/16-preguntas.md` invita a usar `rect` de Freya directamente,
  pero ningún capítulo dice que `Size::flex` solo reparte si el padre tiene
  `.content(Content::flex())`; sin él, el primer hijo ocupa todo (lo vimos con
  los paneles).
- Nada documenta el signo de `WheelEventData::delta_y`: el runner pasa el eje
  de Wayland tal cual (`platform/input/seat.rs:301`), negativo hacia arriba.
