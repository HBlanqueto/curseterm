# Auditoría de CurseTerm, BarrierKit y su integración

Fecha: 2026-11-07. Auditoría de solo lectura: no cambia código. Las
correcciones se harán aparte, cuando se aprueben.

## Objetivo y alcance

Revisar todo lo relacionado con CurseTerm: la app, BarrierKit (`barrier-kit`,
`vow-design`, `vow-appearance`, `infinity-protocol`), su integración en
Malaysia y su llegada a dotfiles. El alcance empieza en el código de los
cuatro repos y acaba en el paquete de Nix que instala el sistema.

Los criterios son las reglas de los `AGENTS.md` de los cuatro repos
(«Código mínimo» 1–10, docs bilingües, tokens, Fluent, atajos con Super,
nada probado sin ejecutarlo) y el comportamiento de iTerm2.

## Lector

Quien mantiene CurseTerm y BarrierKit y va a aprobar la lista de
correcciones.

## Entradas y salidas

Entradas, en estas revisiones:

| Repo | Rama | Rev |
|---|---|---|
| curseterm | `main` | `f4e804c` |
| barrierkit | `main` | `21c532d` (CurseTerm fija `4e23c81`; los dos commits de diferencia solo tocan paneles) |
| malaysia | `main` | `bd839e2` |
| dotfiles | `master` / `claude/dazzling-sagan-7gd075` | `003129e` / `e16c41c` |
| iTerm2 (referencia) | `master` | `6c7024a` (clon superficial de `gnachman/iTerm2`) |

Salidas: este documento, su versión inglesa (`auditoria.en.md`) y las
capturas `capturas/auditoria-*.png`.

## Pasos

1. Fase 0, línea base: checks, tests y builds de Nix (sección 1).
2. Fase 1, lo básico: compilación, dependencias, reglas, i18n y docs.
3. Fase 2, CurseTerm módulo a módulo.
4. Fase 3, paridad con iTerm2.
5. Fase 4, seguridad.
6. Fase 5, integración CurseTerm ↔ BarrierKit y estado de cada BK.
7. Fase 6, BarrierKit en profundidad.
8. Fase 7, empaquetado e instalación.
9. Fase 8, pruebas en sway sin pantalla (sección 7).

Las fases 1–7 combinan lectura propia y seis revisiones en paralelo de solo
lectura. Lo más grave se volvió a comprobar a mano o en el laboratorio; la
columna «¿verificado?» dice cómo: «Sí, ejecutado» es reproducido; «Sí» o
«código» es leído de punta a punta sin ejecutarlo.

## 1. Línea base (fase 0)

| Comando | Resultado | Tiempo |
|---|---|---|
| `git fetch` en los cuatro repos | Las ramas locales coinciden con `origin/main`; dotfiles `master` y la rama de Malaysia, como en la tabla de entradas | — |
| CurseTerm: `nix develop -c ./dev.sh check` | ✓ fmt, clippy `-D warnings` y 87 tests | 584 s |
| barrierkit: `nix develop -c ./dev.sh check` | ✗ fmt y clippy limpios; para en `scripts/check-design-tokens.sh` con 113 líneas, así que sus tests no llegan a correr | 276 s |
| barrierkit: `cargo test --workspace --no-fail-fast --features barrier-kit/shell,barrier-kit/tracy` | ✗ 15 fallos: 8 en `tests/kit`, 6 en `tests/sidebar` y 1 en `marea/tests/media`; el resto pasa (203 de la lib, 71 de vow-appearance…) | 271 s |
| CurseTerm: `nix build .#curseterm` | ✓ `/nix/store/c65fxsjm…-curseterm-0.1.0` (el primer intento falló por disco lleno) | 259 s |
| Malaysia: `nix build .#curseterm` | ✓ la misma salida `c65fxsjm…` | 3 s |
| dotfiles: `nix eval .#nixosConfigurations.keel-x86_64-linux.config.system.build.toplevel.drvPath` | ✗ sin terminar: evaluar compila por IFD skia, Malaysia y los paquetes de dotfiles (299 construcciones en `master` antes de parar); se detuvo al quedar 1,2 GB de disco. Hasta ahí no hubo errores de evaluación; los fallos anteriores fueron del entorno (CA y disco) | — |

Para llegar a esta línea base hubo que rodear el entorno, sin tocar los
repos (sección 8): el sandbox de Nix no confía en la CA del proxy para
github.com y el proxy niega los tarballs de GitHub. Las fuentes afectadas
se prepararon con el git del host y se añadieron a la tienda con su hash
(`nix-store --add-fixed`), así que Nix comprobó cada una.

## 2. Hallazgos, por gravedad

Hay 7 críticas, 22 altas, 70 medias y 68 bajas. Las filas `X (= Y)` agrupan dos hallazgos iguales de dos
revisiones.

Gravedad: **crítica**, pérdida de datos, seguridad o pánico alcanzable;
**alta**, función rota o divergencia clara de iTerm2; **media**,
degradación, regla de `AGENTS.md` o deuda que estorba; **baja**, estilo,
docs o limpieza.

Rutas: en `LAB`, `SEC`, `RUL` e `INT`, un archivo sin repo es de
`crates/curseterm/src/`, y `src/…` en `PAR` también. En `PLT` es de
`crates/barrier-kit/src/platform/` de barrierkit; en `KIT`, de
`crates/` de barrierkit. `fc/`, `fa/`, `fw/`, `wb/` y `at/` son
freya-core, freya-animation y freya-winit 0.4.3, wayland-backend 0.3.17 y
alacritty_terminal 0.26.0, leídos en la versión de `Cargo.lock`.

| ID | gravedad | área | archivo:línea | qué pasa | evidencia | propuesta | ¿verificado? |
|---|---|---|---|---|---|---|---|
| LAB-01 | crítica | kit / controles | barrierkit `crates/barrier-kit/src/controls.rs:692`, `:1313` (también en `4e23c81`, desde `5f0fc43`) | Pulsar cualquier `Switch` o `Checkbox` no controlado aborta la app. `commit(on, &on_toggle, !*on.peek())`: el préstamo temporal de `peek()` vive hasta el final de la sentencia y `commit` hace `set` (`controlled.rs:13-23`). Con `panic = "abort"` se pierden todas las ventanas y shells. CurseTerm usa 21 `Switch`/`Checkbox` en Ajustes y hojas. | Laboratorio: clic en «Abrir la disposición de ventanas por omisión» (Ajustes › General) → `panicked at freya-core-0.4.3/src/lifecycle/writable_utils.rs:96:14: Writing to the State failed because it is already borrowed` y el proceso muere (antes del clic: `capturas/auditoria-ajustes-general.png`). También 3 de los 8 tests que fallan en `tests/kit` (`checkbox_click_toggles…`, `switch_click_toggles…`, `tab_reaches_each_control…`). | Sacar el valor a una variable antes: `let next = !*on.peek(); commit(on, &on_toggle, next);` en los dos sitios; subir el `rev` en CurseTerm. | Sí, ejecutado |
| SEC-01 (= PAR-16) | crítica | OSC 1337 File= (imágenes) | curseterm `io.rs:863-867`, `io.rs:887`, `io.rs:903`, `integration.rs:170-178` | Una salida remota bloquea la aplicación entera. `height` llega sin límite: `Cells(u32)`, `Pixels(u32)` y `Percent(u32)`. Con `preserveAspectRatio=0`, nada recorta el alto: solo se recorta el ancho (`io.rs:906`). Luego `apply` ejecuta `term.linefeed()` una vez por cada línea del alto (`io.rs:865`) con el `FairMutex` del `Term` y el `Mutex<Extras>` tomados. Ejemplo: `ESC]1337;File=inline=1;width=1;height=4294967295;preserveAspectRatio=0:<png de 1×1>BEL` son unos 4,3·10⁹ saltos de línea. Mientras tanto, el hilo de la interfaz (compartido por todas las ventanas del proceso) se queda esperando el candado. Basta con `cat` de un fichero o con salida de `ssh`. | `let lines = (placed.height / extras.cell.1).ceil() as usize; … for _ in 0..lines { term.linefeed(); }` Laboratorio: `cat` de un `File=` con `height=4294967295` congela todas las ventanas ~55 s (hilo `curseterm-pty-r` al 100 %, principal en `futex_do_wait`) y deja el historial vacío (`capturas/auditoria-imagen-alta.png`). | Limitar el alto colocado a `term.screen_lines()` filas (o al área visible, como ya se hace con el ancho) antes de calcular `lines`. iTerm2 también recorta el tamaño a la ventana. | Sí, ejecutado |
| PAR-01 | crítica | Session › Reset | `src/session.rs:568-571`, `src/io.rs:242-251` | Session › Reset (Super+R) llama a `Term::reset_state`. Eso borra todo el historial (`Grid::reset` → `clear_history`). Además, `forget_everything` borra las marcas, las marcas con nombre, los sellos de hora, las imágenes y las **anotaciones** que escribió el usuario. En iTerm2, Reset pasa la pantalla al historial y no borra nada de él. Es pérdida de datos sin aviso ni forma de deshacerla. | CurseTerm: `alacritty: term/mod.rs:1835-1842`, `alacritty: grid/mod.rs:336-341`. iTerm2: `PTYSession/PTYSession.m:7347-7353` (`userInitiatedReset`), `VT100/VT100Terminal.m:491-499` (`preservePrompt:userInitiated`), `VT100Screen/VT100ScreenMutableState.m:2128-2137` y `2312-2327` (`resetWithLineBuffer:` pasa las filas al `linebuffer` y solo quita los objetos de la zona de la pantalla) Laboratorio: `seq 1 300`, Super+R y tres Mayús+RePág: no queda ninguna línea del historial. | Reiniciar los modos, los juegos de caracteres, los tabuladores y los colores sin `reset_state`: subir la pantalla al historial, como `clear_buffer`, y llamar a `forget_lines` solo para las filas de la pantalla | Sí, ejecutado |
| PLT-02 | crítica | menú / contexto | `input/seat.rs:458-460`; `app_menu_export.rs:48-50`; `../app/application.rs:49-53` | BK-017: `MenuActions::dispatch` corre fuera de `CurrentContext`. Un handler que use `Clipboard`, `Timers::get`, `Platform::get`, `spawn` o `open_utility_window` aborta. | `fc/current_context.rs:55-57`; `fc/lifecycle/context.rs:65-77` | `AppInstance::run_in` en los tres sitios. | Sí, por lectura; el pánico está confirmado por CurseTerm (gaps BK-017) |
| PLT-03 | crítica | timers | `timer.rs:47-57,69-80`; `runtime.rs:232-239` | Los callbacks de `Timers::after`/`wakeup` corren fuera de contexto y sobreviven a su ventana. Un `State::write` sobre un scope ya liberado entra en pánico. `wakeup` nunca se da de baja: crece y corre en cada wake para siempre. | `fc/lifecycle/state.rs:386-394` (`subscribers.write()` de un `GenerationalBox` liberado); `animation.rs:10` (`use_alive`, workaround) | `owner` en `Timers`, `run_in` y descarte si la superficie no existe. | Sí, por lectura |
| KIT-01 | crítica | SplitPanes | barrier-kit/src/views/split_panes.rs:108 | **Pánico alcanzable** al arrastrar un divisor: `clamp(minimum, pair - minimum)` con `minimum > pair - minimum`. `f32::clamp` hace pánico si `min > max`, y el perfil release tiene `panic = "abort"`, así que CurseTerm se cierra | Con `pane_min = 60` (components.json): 2 paneles en < 123 px; o `ratios=[0.8,0.1,0.1]` y `free=500` → `minimum=0.12`, `pair-minimum=0.08`. CurseTerm lo usa en terminal.rs:1454 Laboratorio: arrastrar el divisor entre dos paneles estrechos aborta CurseTerm: `min > max, or either was NaN. min = 0.102564104, max = -0.040064104`. | Si `pair < 2*minimum`, devolver `ratios` sin cambios, o limitar con `minimum.min(pair/2.0)`. Añadir el caso al test existente | Sí, ejecutado |
| RUL-01 | crítica (legal) | licencia | `throughput.rs:27-62`, `profile_switching.rs:66-112`, `smart_selection.rs:49-91`, `hints.rs:6`, `key_bindings.rs:307-380` | Hay código y datos de iTerm2 (GPL-2.0-o-posterior, en la práctica GPLv3) traducidos o copiados en un proyecto MIT. No hay `NOTICE`. | `throughput.rs` es `iTermThroughputEstimator.m:50-105` línea a línea (tope 10, peso ×2, `sum/(weight-1)/bucket`; el comentario `:36` parafrasea `.m:53-55`). `score()` reproduce `iTermRule.m:142-222` con las mismas constantes 1/2/4/8/16/32 y las mismas ramas. Las 11 reglas salen de `plists/SmartSelectionRules.plist`. `URL_PATTERN` es `findUrlsRegex` (`iTermAdvancedSettingsModel.m:446-448`). Los presets salen de `PresetKeyMappings.plist` y `DefaultGlobalKeyMap.plist`. `/home/user/iterm2/COPYING`: «GPL v2 or later… effectively GPL v3». Lista: `f1-gpl-evidencia.txt` | Decidirlo antes de publicar. Hay dos salidas: (a) cumplir la regla del propio README: pasar a GPL-2.0-o-posterior (o GPLv3) y crear `NOTICE`; (b) reescribir esos módulos en sala limpia, a partir de la especificación y sin el `.m` delante, y sustituir los datos copiados | sí (5 módulos); el resto, no (ver «No verificado») |
| LAB-02 | alta | coprocesos / E/S | curseterm `crates/curseterm/src/io.rs:500-516`, `session.rs:415-440` | El hilo lector del PTY escribe la salida al stdin del coproceso con una escritura bloqueante y con el `Mutex<Extras>` tomado. Un coproceso que no lee su stdin llena la tubería (64 KiB) y bloquea el hilo; la interfaz se queda esperando el candado y se congelan todas las ventanas del proceso hasta que el coproceso muere. iTerm2 escribe al coproceso con un búfer no bloqueante (`Coprocess.m`). | Laboratorio: trigger `coprocess` con `sleep 600` y 400 KB de salida → hilo `curseterm-pty-r` en `anon_pipe_write`, hilo principal en `futex_do_wait`, pantalla congelada; al matar `sleep` se recupera (`capturas/auditoria-coproceso.png`). | Escritura no bloqueante (`O_NONBLOCK`) con un búfer acotado que se descarta si se llena, fuera del candado de `Extras`. | Sí, ejecutado |
| LAB-03 (= INT-02) | alta | Quit / pérdida de trabajo | curseterm `windows.rs:173-185`, `windows.rs:203-226`, `terminal.rs:4536`; barrierkit `platform/window.rs:265` | Super+Q cierra todas las ventanas con `WindowCommand::Close` sin preguntar, aunque haya trabajos en marcha, y manda `SIGTERM` a los demás procesos CurseTerm, que mueren sin aviso ni archivo de sesión. iTerm2 pregunta por omisión (`iTermPreferences.m:732-733` `OnlyWhenMoreTabs`/`PromptOnQuit` = YES; `iTermApplicationDelegate.m:1080-1100`). `progreso.md:456` lo atribuye a BK-013, pero Quit es una acción de la app y no depende del kit. | Laboratorio: `sleep 999 &` y Super+Q → la app y el `sleep` terminan en < 3 s, sin diálogo. | Pasar Quit por `asks_before_closing` de todas las ventanas (y por las de los otros procesos, o dejar de matarlos); corregir `progreso.md:456`. Agrupa INT-02. | Sí, ejecutado |
| LAB-04 | alta | dibujo / texto | curseterm `grid.rs:600-672` | `draw_cell` solo dibuja `cell.c`; nunca dibuja los caracteres de anchura cero (`cell.zerowidth()`, acentos combinantes, ZWJ, selectores de variación). El texto en NFD (nombres de archivo de macOS, `git log` con autores en NFD, emoji con tono de piel) sale sin acentos o partido. Tampoco distingue los estilos de subrayado (doble, ondulado, punteado) ni su color. iTerm2 compone la celda entera (`ScreenChar` con `complexChar`). | Laboratorio: `printf 'cafe\314\201 n\314\203 e\314\201 👍🏽'` → «cafe n e» sin acentos y el modificador de tono como caja aparte (`capturas/auditoria-combinantes.png`); `grep zerowidth crates/curseterm/src` no da nada. | Dibujar `c` más `zerowidth()` como una sola cadena en `draw_char` (o un `TextBlob` por celda compuesta); dibujar los subrayados según `Flags`. | Sí, ejecutado |
| SEC-02 (= PAR-02) | alta | OSC 52 (escritura del portapapeles) | curseterm `session.rs:206` y `session.rs:282-287` (`Config::default()`); `terminal.rs:3198`; at `term/mod.rs:367-382`, `1705-1723` | Cualquier programa del PTY, también uno remoto, sobrescribe el portapapeles sin aviso ni permiso. `Config::default()` deja `osc52 = Osc52::OnlyCopy` y `ClipboardStore` acaba en `set_clipboard` sin más comprobaciones. Además se ignora el `ClipboardType`, así que `52;p` (la selección primaria) también escribe en el portapapeles. iTerm2 lo niega por defecto: `kPreferenceKeyAllowClipboardAccessFromTerminal: @NO` (`iTermPreferences.m:750`), con un aviso (`PTYSession.m:21211-21240`). Es la base del «pastejacking»: el usuario pega después un comando que no copió. | `TermEvent::ClipboardStore(_, text) => set_clipboard(handles.clipboard, text)` Laboratorio: `printf '\033]52;c;…'` deja «rm -rf ~ # pegado» en el portapapeles sin aviso (`wl-paste`). | Añadir una preferencia «Las apps del terminal pueden usar el portapapeles», desactivada por defecto. Pasarla a `Config.osc52` (`Disabled`, o `OnlyCopy` con permiso) y avisar con un anuncio, como iTerm2. | Sí, ejecutado |
| SEC-03 (= PAR-03) | alta | Pegado normal | curseterm `session.rs:1545-1551` (`paste_bytes`), `terminal.rs:2003-2006` | El pegado normal (Ctrl/Super+V, menú y selección) no quita los caracteres de control. Si el programa no usa pegado entre corchetes, pasan ESC, `^C`, `^U`, `^D`, `^Z` y los demás. Con pegado entre corchetes solo se quita ESC. iTerm2 aplica siempre `kPasteFlagsRemovingUnsafeControlCodes` (0x00-0x1F salvo TAB, LF, FF y CR; `iTermPasteHelper.m:136-142` y `:440-442`). CurseTerm solo lo hace en Pegado avanzado (`paste_transform.rs:91-94`). | `text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()` sin filtrar | Aplicar a `paste_bytes` el mismo `retain` que `paste_transform::apply` con `remove_controls` (o llamar a `apply` con `Transform::default()`). Así hay una sola regla en un solo sitio. | Sí |
| SEC-04 | alta | Aviso de eco del gestor de contraseñas | curseterm `session.rs:317-324`, `terminal.rs:4371`, `terminal.rs:4384-4404` | (a) `echoes()` lee el `termios` del PTY local. Dentro de `ssh` el pty local está siempre en modo crudo (sin ECHO), así que el aviso no salta nunca, aunque el shell remoto muestre lo que se escribe. Ese es el caso típico de un gestor de contraseñas. iTerm2 usa `iTermEchoProbe`, que sondea la salida y funciona en remoto. (b) Solo se comprueba la sesión activa (`passwords.active()`), pero la contraseña va a `password_targets()`, que con difusión son todas las sesiones receptoras. | `if passwords.active().is_some_and(\|session\| session.echoes())` frente a `for session in password_targets(handles) { session.write(bytes.to_vec()); }` | (b) Comprobar `echoes()` en cada destino y preguntar si alguno tiene eco. (a) Implementar la sonda de eco de iTerm2 (escribir un carácter de prueba y mirar si vuelve), o al menos avisar cuando el proceso en primer plano sea `ssh`/`mosh`. | (b) sí; (a) sí en el código, pero lo de ssh y el modo crudo se razona sin ejecutar |
| SEC-05 (= PAR-08) | alta | Splitter OSC (7/9/133/1337) | curseterm `osc.rs:59-67`, `osc.rs:84-88` | Una secuencia `ESC ] 1337;` (o `7;`, `9;`, `133;`) sin terminar se traga **toda** la salida siguiente hasta 64 MiB, y después la sigue descartando hasta que llegue un BEL o `ESC \`. Un ESC que no va seguido de `\` se guarda en la carga, y CAN/SUB no abortan. vte termina una OSC en cualquier ESC y aborta con 0x18/0x1A (vte `lib.rs:405-428`). Un fichero hostil o binario que se muestra con `cat` deja el terminal «mudo» y puede esconder lo que imprime un comando. | `State::PayloadEscape => { self.hold(ESC); self.hold(byte); self.state = State::Payload; }`; no hay rama para 0x18/0x1A Laboratorio: tras `ESC]133;A` sin terminar desaparece toda la salida del comando hasta el siguiente BEL (`capturas/auditoria-osc-sin-terminar.png`). | Igualar a vte: ESC sin `\` termina la OSC (se descarta o se despacha) y vuelve a `Escape`; 0x18/0x1A abortan. | Sí, ejecutado |
| PAR-04 | alta | Emulación: OSC 4/10/11/12 `?` | `src/terminal.rs:3296` | `Event::ColorRequest` cae en `_ => {}`, así que las consultas `OSC 4;n;?`, `OSC 10;?`, `OSC 11;?` y `OSC 12;?` no reciben respuesta. vim, neovim, emacs, delta o bat las usan para saber si el fondo es claro u oscuro, y esperan hasta agotar el tiempo o se equivocan. iTerm2 responde. | `vte: ansi.rs:1422-1445`, `alacritty: term/mod.rs:1675-1690`. iTerm2: `VT100/VT100Terminal.m:2842-2862` (`executeSetDynamicColor` con `?`), `VT100/VT100Output.m:1195` (`reportColor:atIndex:prefix:`) | Atender `ColorRequest(index, format)`: el color del `term.colors()[index]` o, si no hay, el del esquema (`grid.rs:124-136`); escribir `format(rgb)` en el PTY | sí |
| PAR-05 | alta | Emulación: DECSET 2026 + integración | `src/io.rs:466-482`, `src/io.rs:524-556` | Durante una actualización sincronizada, el `Processor` de vte guarda los bytes y no toca el `Term`. Los OSC que separa `osc.rs` (133, 1337, 7, 9) se aplican en el acto sobre el `Term` sin actualizar. La marca cae en el cursor antiguo, la orden de `133;C` se lee de la rejilla vieja y la imagen inserta sus saltos de línea antes del texto que la precede. Lo mismo pasa con los triggers y con la cuenta de líneas perdidas, que se calculan en cada LF sobre la rejilla vieja. Fish 4, neovim y los prompts que redibujan con 2026 lo disparan. En iTerm2 los tokens se ejecutan en orden y 2026 solo aplaza el dibujo. | `vte: ansi.rs:298-310` (`advance` desvía a `advance_sync`). iTerm2: `VT100/VT100Terminal.m:1082-1084, 1110-1113` y `3346-3352` (solo avisa al delegado de que pare el dibujo) | Mientras `processor.sync_timeout().pending_timeout()`, guardar las piezas `Osc` (y aplazar `check_line`) y aplicarlas, en orden, cuando el procesador vacíe su búfer; o pasar los OSC al `Processor` con un `Handler` envoltorio | sí (orden del código; no ejecutado) |
| PAR-07 | alta | Imágenes en línea | `src/integration.rs:56-81` | Solo se entiende `1337;File=…:base64`. El `imgcat` actual de iTerm2 manda por omisión `MultipartFile=` + `FilePart=` × n + `FileEnd` (sin `--legacy`), y CurseTerm no muestra nada. La fase 4 lo probó con `File=` a mano. Las descargas (`inline=0`) también se descartan sin aviso. | iTerm2: `VT100/VT100Terminal.m:4472-4473`, `OtherResources/Utilities/imgcat:67-101` y `:194` (`legacy=0`) | Ensamblar `MultipartFile`/`FilePart`/`FileEnd` en `Extras` (con límite de tamaño) y reutilizar `inline_image`; documentar que no hay descargas | sí |
| PAR-09 | alta | Emulación: ED 3 y RIS desde la app | `src/io.rs:466-482` (sin trato), `src/io.rs:253-269` | `CSI 3 J` (lo manda `clear` de ncurses) y `ESC c` (`reset`) vacían el historial en alacritty sin preguntar. Las anclas de `Extras` (marcas, anotaciones, imágenes, sellos, pliegues, resaltados, salida del último comando) siguen en líneas absolutas que ahora apuntan a otro texto. iTerm2 pregunta antes de que una app borre el historial (por omisión) y mueve o quita sus objetos. | `alacritty: term/mod.rs:1806-1812` (`ClearMode::Saved` → `clear_history`), `:1835-1842`. iTerm2: `VT100/VT100Terminal.m:2244-2246`, `VT100Screen/VT100ScreenMutableState+TerminalDelegate.m:2073-2086`, `PTYSession/PTYSession.m:21435-21444` (`clearScrollbackAllowed`: NO si no se cambió `preventEscapeSequenceFromClearingHistory`), `VT100Screen/VT100ScreenMutableState.m:2194-2214` | Detectar en `feed` que bajó `history_size()` y llamar a `forget_lines(0..perdidas)`; ofrecer el aviso de iTerm2 (o, al menos, documentar la diferencia) | sí (el pánico por anclas caducadas no está verificado) |
| PAR-10 | alta | Ratón | `src/mouse.rs:14-16`, `docs/progreso.md:926` | Con el informe de ratón activo, la tecla que vuelve a seleccionar texto es **Mayús**. En iTerm2 es **⌥ (Alt)**: Mayús se informa al programa y ⌥ nunca. `progreso.md` dice «como en iTerm2». | iTerm2: `TerminalView/PTYMouseHandler.m:1530-1538` («Not reporting mouse event because you pressed option») | Usar `Modifiers::ALT` en `reported` y enviar el bit 4 de Mayús en `report`; corregir la frase de la fase 17 | sí |
| PAR-11 | alta | Menú: Broadcast Input | `src/menu.rs:727-742` | Los atajos no son los de iTerm2. CurseTerm: All Tabs = Super+Alt+I, Current Tab = Super+Mayús+Alt+I, Current Session Only sin atajo. iTerm2: All Tabs = ⇧⌘I, Current Tab = ⌥⌘I, Current Session Only = ⌥⇧⌘I. Con la memoria de iTerm2, ⌥⌘I difunde a todas las pestañas y ⌥⇧⌘I, que debería cortar la difusión, la activa. | iTerm2: `MainMenu/Base.lproj/MainMenu.xib:382-383, 389-390, 395-396` | Super+Mayús+I (todas), Super+Alt+I (pestaña), Super+Mayús+Alt+I (solo la sesión) | sí |
| PLT-01 | alta | runtime / animación | `runtime.rs:442`; `app.rs:236-241,357` | BK-047: `update()` solo corre si `poll()` dio `Ready`. Los scopes y tasks drenados mientras el futuro está `Pending` se quedan colgados y la animación salta al final. | `fc/runner.rs:565-627,643`; `fa/hook.rs:256-319`; `fw/renderer.rs:456-480` | Ver «Causa raíz BK-047». | Sí, por lectura (no ejecutado) |
| PLT-05 | alta | Vulkan / cierre | `runtime.rs:140-161`; `window.rs:422-434`; SAFETY falso en `vulkan.rs:383` y `vulkan.rs:724` | Al terminar `run()`, primero cae el `event_loop` local, que tiene el único `Connection` y el `EventQueue`; eso hace `wl_display_disconnect`. Después cae `self`, y `Graphics::drop` destruye swapchain y `VkSurfaceKHR` sobre un `wl_display`/`wl_surface` ya liberados: uso tras liberar al salir. El SAFETY «display and wl_surface are alive while this Graphics lives» es falso en el cierre. | Orden de drop: locales antes que parámetros. `wb/sys/client_impl/mod.rs:1149-1152` desconecta al caer el último `Backend`. `wc/event_queue.rs:318-321` (`EventQueue` tiene `Connection`). `Runtime` no guarda ningún `Connection` (grep). | Vaciar `self.surfaces` (y soltar `graphics`) antes de que caiga `event_loop` (p. ej. `self.surfaces.clear()` tras `event_loop.run`), o guardar un `Connection` en `Runtime` que caiga después de `surfaces`. | Sí el orden, por lectura; el efecto en Mesa no está verificado |
| PLT-06 | alta | multiventana | `window.rs:781-789,822-830`; `input/seat.rs:446-449` | `close_window` cuenta los paneles de utilidad como ventanas. Si se cierra la ventana principal con un panel abierto, el proceso no termina. En Malaysia el panel `hides_on_deactivate` se oculta y queda un proceso invisible. El panel pierde sus atajos porque `parent` apunta a una superficie que ya no existe. | `window.rs:783-786` filtra solo `SurfaceRole::Window` | Contar solo las ventanas sin `context.parent` y cerrar los hijos (`parent == surface_id`) al cerrar el padre. | Sí, por lectura |
| KIT-02 | alta | tests y CI | flake.nix:63, 112-130; sin `.github/` | No hay CI que ejecute `cargo test` (`doCheck = false`), y el único check de estilo (`design-tokens`) falla. Así los tests llegaron rotos a `main` | `sh scripts/check-design-tokens.sh` → exit 1; `ls -a` sin `.github` | Un check `cargo test` en el flake con `LANG=C`, `MALAYSIA_*_DIR` temporales y fuentes fijadas | sí |
| KIT-03 | alta | tests | tests/sidebar.rs:1010,1043,1088; tests/kit/controls.rs:1081-1093 | Los tests dependen del idioma del host. Cuatro fallan en `C/en` y el del DatePicker falla en `es` | Ver «Causas» | Buscar por `id`/`probe` o con el texto de Fluent, como en eee8287; fijar el idioma en el harness | sí (código), no ejecutado |
| KIT-04 | alta | tests | tests/kit/controls.rs:457-484, 524-526; flake.nix:96-103 | Golden no herméticos: fuentes del host (`/etc/fonts/fonts.conf`), idioma, y un golden ausente se crea y el test pasa | Ver «Causas» | Fuentes solo del store (sin el include del host), idioma fijo, y fallar si falta el golden salvo con `UPDATE_GOLDEN` | sí (código) |
| KIT-05 | alta | ADR 0047 | crates/tengen-app-show (0 usos); barrier-kit/tests (0 usos) | `canvas`, `SplitPanes`, `FloatingPane`, `TextView` y los hooks `use_menu_*` no tienen página en Tengen App Show ni test de render, como exige el ADR 0047 §2 (docs/decisiones/0047…md:75-77). Lo reconoce el ADR 0006 de CurseTerm | `grep -rlw` en `tengen-app-show` y `tests` → 0 para cada uno. Tests unitarios: `split_panes` tiene 1; `canvas`, `floating_pane`, `text_view`, `notifications` y `secrets` tienen 0 | Página y golden claro/oscuro/tintado, más un test de teclado donde sea interactivo | sí |
| INT-01 | alta | entrada / menú | CT `menu.rs:66,78,114`; `terminal.rs:4799-4804,1830-1835`; kit `app_menu.rs:213-224`, `seat.rs:439-460` | Con el Composer, una nota, Find o una hoja enfocados, Super+V, C, A, X o Z disparan la acción del **terminal**: Paste pega en el PTY, no en el campo. El kit solo deja pasar al campo las órdenes de edición con **Ctrl** (`edits_text`); el atajo se consume y la tecla nunca llega al campo | `edits_text` exige `CONTROL` y sin `ALT`/`META`; `on_action` no mira `handles.editing` ni `find.focused`; `progreso.md:1633` «No probado: pegar con ⌘V dentro» | Kit: que `edits_text` acepte el modificador de órdenes de la app (Super) o exponer `use_text_input_focused` (BK-027). CurseTerm: mientras tanto, mandar COPY, PASTE y SELECT_ALL al campo enfocado. Anotarlo como BK-048 | No (solo lectura de código) |
| RUL-02 | alta | docs/UI | `README.md:12-15`, `README.en.md:12-15`, `i18n/en/curseterm.ftl:18`, `i18n/es/curseterm.ftl:18` | El README y la ventana «Acerca de» afirman «no porta ni traduce código de iTerm2» / «no iTerm2 code». RUL-01 lo contradice. | Ver RUL-01 | Corregirlo en el mismo commit que resuelva RUL-01 | sí |
| LAB-05 | media | Instant Replay / memoria | curseterm `io.rs:66-67`, `io.rs:183-201` | Instant Replay guarda 120 copias completas de la pantalla (una cada 500 ms con salida). Con `Cell` de unos 24 B, una pantalla de 200×50 ocupa ~29 MB por sesión, y crece con las columnas. iTerm2 limita la memoria por sesión a 4 MB por omisión (`iTermPreferences.m:735` `IRMemory` = 4) y guarda diferencias. | Lectura de código y cuenta; no medido en ejecución. | Guardar solo las filas que cambian (o un tope de bytes como `IRMemory`) y exponer el ajuste. | Código, no ejecutado |
| LAB-06 | media | coprocesos | curseterm `session.rs:442-450` | `stop_coprocess` manda `SIGTERM` al grupo y hace `child.wait()` en el hilo de la interfaz. Un coproceso que ignora `SIGTERM` bloquea la interfaz para siempre. | Lectura de código. | Esperar en un hilo aparte o con un plazo y `SIGKILL` después. | Código, no ejecutado |
| LAB-07 | media | ventana hotkey | curseterm `windows.rs:59-77` | El segundo `curseterm --hotkey` mata con `SIGTERM` el proceso de la ventana hotkey: sus sesiones mueren sin preguntar. `progreso.md:359` dice «se cierra en vez de ocultarse», pero no que se pierden los trabajos. | Lectura de código. | Hasta que exista el portal (BK-025), aplicar «Prompt before closing» antes de matar, o documentar la pérdida. | Código, no ejecutado |
| LAB-08 | media | arquitectura (regla 4) | curseterm `terminal.rs` (5259 líneas), `on_action` `terminal.rs:4441-5193` | `terminal.rs` mezcla vista, menús, hojas, pegado, cierre, archivos de sesión y gestor de contraseñas; `on_action` tiene ~750 líneas. Malaysia fija ~1500 líneas por archivo como límite; barrierkit pide un motivo de cambio por módulo. | `wc -l`, `grep -n '^fn on_action'`. | Repartir por tema (pegado, cierre, hojas, archivo de sesión) como ya se hizo con `paste_warning.rs` o `archive.rs`. | Sí |
| LAB-09 | media | empaquetado | malaysia `flake.nix:12-16`; malaysia `flake.lock` (`barrierkit_2`, `crane_2`, `nixpkgs_2`); dotfiles rama `claude/dazzling-sagan-7gd075` `flake.lock` (`barrierkit_2`, `crane_3`, `nixpkgs` 31ecbaca) | La entrada `curseterm` de Malaysia sigue `nixpkgs` y `crane`, pero no `barrierkit`. Por eso hay un segundo barrierkit (`4e23c81`) con su propio `crane` y, en dotfiles, su propio nixpkgs (31ecbaca frente a 55bfb4d del sistema): skia se compila dos veces y se evalúan dos nixpkgs. | `flake.lock` leídos con un script (`nodes`); misma salida `c65fxsjm…-curseterm-0.1.0` en CurseTerm y Malaysia. | `curseterm.inputs.barrierkit.follows = "barrierkit";` en Malaysia (el `Cargo.lock` de CurseTerm sigue fijando su rev). | Sí |
| LAB-10 | media | empaquetado / fuentes | curseterm `flake.nix:16-23`; malaysia `nix/malaysia.nix:70-73` | El paquete `curseterm` no depende de Geist Mono: solo la instala el módulo de Malaysia. Con `nix run` o en otro escritorio cae a una fuente proporcional y las letras salen separadas (agrava KIT: `MONO` pierde la reserva `monospace`). | Laboratorio: captura `capturas/auditoria-escribir.png` (sin Geist en el sistema). | Envolver el binario con un `FONTCONFIG_FILE` que incluya `geist-font`, o declarar la fuente en el paquete, y conservar `monospace` como reserva. | Sí, ejecutado |
| LAB-11 | media | dotfiles / docs | dotfiles `AGENTS.md:118`, `:122`; dotfiles `flake.nix:120-138`; `.github/workflows/` | AGENTS.md dice `nixosConfigurations.nixdots` y variantes de CI `nixdots-x86_64-linux`/`-aarch64-linux`, pero el flake define `keel`, `keel-x86_64-linux` y `keel-aarch64-linux`, y ningún workflow evalúa esas configuraciones (solo `installer-release.yml`). | `grep` en `AGENTS.md`, `flake.nix` y `.github/workflows`. | Actualizar AGENTS.md (regla «manda el código») y añadir un job que evalúe `keel-x86_64-linux`. | Sí |
| LAB-12 | media | dotfiles / integración | dotfiles `master` `flake.lock` (malaysia `c62e4d2`); rama `claude/dazzling-sagan-7gd075` (malaysia `eccbf2f`) | En `master` CurseTerm no llega al sistema: Malaysia `c62e4d2` no lo instala. La rama lo trae con Malaysia `eccbf2f`, que ya va por detrás de `main` (`bd839e2`, barrierkit `21c532d`). | `git diff origin/master origin/claude/dazzling-sagan-7gd075 -- flake.lock`. | Subir Malaysia a `main` en la rama (con LAB-09 resuelto) y unirla a `master`. | Sí |
| LAB-13 | media | tests del kit | barrierkit `crates/barrier-kit/src/video.rs:270-276`, `:215-221` | Causa real de `marea/tests/media`: `VideoStream::start` crea el pipeline con `parse::launch` y pone `location` después (`set_locations`). Con GStreamer 1.28.7 y `decodebin3`, `filesrc` arranca durante el `parse::launch` sin ruta («No file name specified for reading»), el pipeline falla y no se prueba otro decodificador. No es el decodificador: `avdec_h264` está en el dev shell. | `GST_DEBUG=2` sobre el test; `gst-launch-1.0 filesrc location=…orilla.mp4 ! decodebin3 ! … ! fakesink` sale con 0. | Poner `location=` (escapado) en la descripción del pipeline en lugar de fijarlo después. | Sí, ejecutado |
| SEC-06 | media | Triggers: Run Command, coprocesos y Send Text | curseterm `triggers.rs:86-118`, `io.rs:722`, `io.rs:771-772`, `io.rs:791-806` | Las capturas `\0`…`\9` del texto del PTY se interpolan **sin escapar** en `$SHELL -c` (Run Command), `/bin/sh -c` (coprocesos) y en el PTY (Send Text). Si un trigger usa `\1` y el texto lo controla un tercero (un log o un nombre de rama remota), hay inyección de órdenes. iTerm2 hace lo mismo en el modo heredado (`Trigger.m:455-463`), así que no es una divergencia. Lo que sí diverge: iTerm2 limita la concurrencia con `commandRunnerPool` (`ScriptTrigger.m:21-28` y `:80`), y CurseTerm lanza un proceso por cada coincidencia sin límite. 10 000 líneas que coinciden son 10 000 shells. | `TriggerAction::RunCommand => run_command(&parameter)` y `expand` copia `groups[index]` tal cual | Escapar las capturas con `session::shell_escaped` en Run Command y Coprocess, lo que mejora a iTerm2 (o documentarlo en `docs/progreso.md`). Poner un tope de procesos simultáneos, como `maximumNumberOfTriggerCommands`. | Sí |
| SEC-07 | media | Semantic History y triggers: intérprete | curseterm `io.rs:791-793`, `session.rs:700-704` | Las acciones Command y RawCommand de Semantic History, y el trigger Run Command, usan `$SHELL -c`. El escapado de `shell_escaped` (`session.rs:1603-1612`) es para POSIX `sh`. Con `$SHELL` = nushell, elvish o xonsh, la barra invertida no escapa y el texto de pantalla en `\1`, `\3`, `\4` y `\5` vuelve a ser inyectable. iTerm2 siempre usa `/bin/sh -c` (`iTermSemanticHistoryController.m`, `launchTaskWithPath:@"/bin/sh"`). El coprocesador de CurseTerm también usa `/bin/sh` (`session.rs:417`), así que hay dos reglas distintas. | `let shell = std::env::var("SHELL").unwrap_or_else(\|_\| "/bin/sh".into());` | Usar `/bin/sh` en `run_command`, igual que iTerm2 y que `start_coprocess`. | Sí (código); nushell no probado |
| SEC-08 | media | OSC 8 (hipervínculos) | curseterm `session.rs:669-679`, `links.rs:42-49`, `pane_view.rs:270` | Super+clic abre la URI de OSC 8 con `xdg-open`, sea cual sea el esquema (`file:`, `smb:`, manejadores propios…). No hay vista previa del destino, y el texto visible puede no coincidir con la URI. iTerm2 enseña la URL al pasar el ratón con Cmd en los enlaces OSC 8 (`PTYTextView+ARC.m:729`, `action.osc8`). | `link.or_else(…)` → `links::open(&url)` sin mirar el esquema ni enseñar la URL | Mostrar la URI (tooltip o barra de estado) con Super pulsado sobre un enlace OSC 8. Pedir confirmación para esquemas que no sean `http`, `https` o `mailto`. | Sí |
| SEC-09 | media | Imágenes en línea: memoria | curseterm `osc.rs:7`, `integration.rs:177-178`, `io.rs:864`, `io.rs:299` | Cada imagen puede traer hasta 64 MiB de carga. Al decodificarla hay picos de unos 200 MB: `held`, `clean`, `decode` y `Data::new_copy`. Cada `Placed` conserva su `Image` mientras la línea siga en el historial, así que N imágenes de una línea retienen N × 48 MiB, sin límite global. | `extras.images.push(placed)`; solo `retain(kept(line))` | Poner un tope total de bytes de imágenes por sesión (descartando las más antiguas) y un tope de píxeles decodificados. | Sí (código); consumo no medido |
| SEC-10 | media | Gestor de contraseñas: `Zeroizing` | curseterm `password_manager.rs:170`, `terminal.rs:4397-4403`; barrierkit `password.rs:28-31`, `secrets.rs:134-141` | Tres sitios dejan copias del secreto en un `String` o `Vec` normal. (1) «Mostrar contraseña» la pasa a `String` normal (`password.to_string()`) para las filas de `Table` y la caché de texto de freya. (2) `send_password` hace `bytes.to_vec()` por sesión: un `Vec` normal que cruza el canal y el hilo de escritura. (3) `PasswordField::insert` hace `push` sobre un `String` vacío, y cada realocación libera un búfer con un prefijo de la contraseña sin borrarlo. El `secret.to_vec()` de `wrapped` y los búferes de mensaje de zbus no se borran (algoritmo `plain`, `secrets.rs:50-58`). No se encontró en el portapapeles, en el historial de pegado ni en `tracing`: ver «Protegido». | `(Some(password), Some(at)) if at == *index => password.to_string()`, `session.write(bytes.to_vec())`, `self.value.push(ch)` | (1) Aceptarlo como deuda o mostrarlo con un control que reciba `&Zeroizing`. (2) Pasar a `write` un `Zeroizing<Vec<u8>>` que el hilo escritor borre al terminar. (3) Crear el campo con `String::with_capacity(N)` y realocar a mano con borrado. | Sí |
| SEC-11 | media | Gestor de contraseñas: llavero bloqueado | curseterm `terminal.rs:4366-4379`; barrierkit `secrets.rs:78-89` | Si el llavero se vuelve a bloquear después de listar, `GetSecret` falla (`IsLocked`) y el error solo va a `tracing::warn!`. La hoja no da ninguna respuesta y el usuario no sabe por qué no pasa nada. En cambio, `load()` sí muestra `password-manager-locked`. `prompted` devuelve `Locked` justo después de lanzar el diálogo, sin esperar a la señal `Completed`. El texto pide «Recargar», que es correcto pero manual. | `Err(error) => tracing::warn!(%error, "could not read the password")` | Mostrar `problem.set(message(error))` también aquí. Cuando el ítem esté bloqueado, llamar a `Unlock` y reintentar al recibir `Completed`. | Sí |
| SEC-12 | media | Archivos de sesión, registro y «Guardar contenido» | curseterm `archive.rs:48-54`, `session.rs:406-413`, `terminal.rs:2327-2338`, `terminal.rs:2309` | El archivo de sesión (con todo el historial: `contents`), el registro de sesión (salida en bruto) y «Guardar contenido/selección» se escriben con los permisos por defecto: 0666 con la umask, es decir, 0644 de costumbre. Ninguno es atómico: `fs::write` trunca antes de escribir. El registro abre en `append` siguiendo enlaces simbólicos. «Guardar contenido» sobrescribe sin preguntar cualquier ruta tecleada. `path_for` comprueba `exists()` y escribe después (TOCTOU). La ruta de `SaveArchive` es relativa al cwd del proceso, a diferencia de `from_home` en las demás. | `std::fs::write(path, text)`; `OpenOptions::new().create(true).append(true).open(path)` | `OpenOptions` con `.mode(0o600)` (`std::os::unix::fs::OpenOptionsExt`). En el archivo de sesión, `create_new(true)` y escritura a un temporal con `rename`. Confirmar antes de sobrescribir. | Sí |
| SEC-13 | media | Preferencias (`*.domain`) | barrierkit `app/defaults.rs:55-63`; curseterm `config.rs:499-508` | `write_toml` es atómico por `rename`, pero el temporal tiene un nombre fijo (`with_extension("tmp")`) y no hay `fsync`. `change_defaults` hace leer, modificar y escribir sin bloqueo. Con varios procesos de CurseTerm (hay registro multiproceso en `windows.rs`), dos escrituras pueden pisarse y perder cambios. Un TOML a medias se respalda en `.bak` y se carga el valor por defecto. Las preferencias son código en la práctica (triggers, coprocesos, Semantic History, `initial_text`), pero viven en `~/.config` del propio usuario: no es una escalada. | `let staged = path.with_extension("tmp"); std::fs::write(&staged, text)?; std::fs::rename(staged, path)` | Temporal con pid y sufijo aleatorio, `sync_all` antes de `rename`, y `flock` en `change_defaults`. | Sí |
| SEC-14 | media | OSC 7 y CurrentDir | curseterm `integration.rs:121-125`, `integration.rs:59-61`, `io.rs:829` | Se descarta el host de `file://host/path`, así que un `ssh` que emite OSC 7 fija como directorio actual una ruta **remota**. Esa ruta se usa en local para las pestañas nuevas («reutilizar directorio»), el `cwd` del coprocesador, Semantic History y la rama git de la barra de estado (`status.rs` lee `.git/HEAD` en esa ruta). No es inyección, pero un programa remoto elige qué ficheros locales se abren con Super+clic. Está «no verificado» si iTerm2 compara el host con el local (ver abajo). | `let path = &rest[rest.find('/')?..];` (el host se ignora) | Guardar el host y solo usar la ruta en local si el host es vacío, `localhost` o el `hostname` local. | Sí (código) |
| PAR-06 | media | Emulación: OSC 133 | `src/integration.rs:47-55`, `src/io.rs:832-859` | Todo `133;A` crea una marca, también los prompts secundarios o de continuación (`k=s`, `k=c`, `k=r`), y se aceptan formas como `133;Afoo`. No se tratan `133;N` ni `133;P` ni el salto a línea nueva de `A`. Un `133;D` sin `C` (Ctrl+C o Intro vacío en el prompt) cuenta como orden terminada y sobrescribe `last_status`. Un `133;D` sin `;` pone el estado a `None`. | iTerm2: `VT100/VT100Terminal.m:5173-5184` (la orden debe tener 1 carácter), `:5190-5225` (A/N/P, `k=`, fresh-line), `:5149-5170` (`promptKindFromArgs`), `VT100Screen/VT100ScreenMutableState+TerminalDelegate.m:2723-2760` (solo el tipo inicial o desconocido crea la marca), `VT100/VT100Terminal.m:5264-5300` (D con `inCommand_` aborta; D sin `;` se descarta) | Analizar los argumentos `;k=`, `aid=`; marca solo con `k=i`, desconocido o sin `k`; aceptar N/P; seguir `in_command` entre B y C | sí |
| PAR-12 | media | Menú: atajo duplicado | `src/menu.rs:112-113`, `src/menu.rs:760`, `docs/progreso.md:3180` | Super+Alt+A está a la vez en Selection Respects Soft Boundaries (fase 71) y en Alert on Next Mark (fase 40); una de las dos no se puede usar con el teclado. En iTerm2, Soft Boundaries es ⌥⇧⌘A y Alert on Next Mark ⌥⌘A. La fase 71 da ⌥⌘A como si fuera de iTerm2. | iTerm2: `MainMenu/Base.lproj/MainMenu.xib:788-789` (`A` + ⌥⌘ = ⌥⇧⌘A), `:973-974` (`a` + ⌥⌘) | `SOFT_BOUNDARIES` con `super_with(ALT \| SHIFT, 'a')` | sí (no verificado cuál gana en el kit) |
| PAR-14 | media | Selección: doble clic | `src/pane_view.rs:277`, `src/session.rs:281-286` | El doble clic usa `SelectionType::Semantic` con los separadores por omisión de alacritty (`,│`\|:"' ()[]{}<>\t`). Así, `=`, `@`, `#`, `$`, `%`, `;`, `!`… cuentan como parte de la palabra: `clave=valor` o `usuario@host` se seleccionan enteros. En iTerm2, una palabra son alfanuméricos más `WordCharacters` (`/-+\~_.`). No hay preferencia para cambiarlo. `progreso.md:711` dice que el doble clic separa en `/`, y no es cierto. | `alacritty: term/mod.rs:45`. iTerm2: `ContentAnalysis/iTermWordExtractor.m:594-613`, `Settings/iTermPreferences.m:72, 751` | Pasar `semantic_escape_chars` con todo carácter que no sea alfanumérico ni esté en `WordCharacters`, o una selección de palabra propia; añadir la preferencia General › Selection | sí |
| PAR-15 | media | Selección: rectangular y extensión | `src/pane_view.rs:259-285`, `src/session.rs:533-552` | Con el ratón no hay selección rectangular (⌘⌥-arrastre en iTerm2, Super+Alt aquí): nunca se crea `SelectionType::Block`. Mayús+clic no amplía la selección, empieza otra. | iTerm2: `TerminalView/PTYMouseHandler.m:29` (`kRectangularSelectionModifiers`), `:372-384` (`isExtension` con Mayús; `kiTermSelectionModeBox`), `:933-947` | En `on_mouse_down`: Super+Alt → `SelectionType::Block`; Mayús con selección → `extend_selection` | sí |
| PAR-17 | media | Fold | `src/io.rs:242-251`, `src/session.rs:1642-1648` | `forget_everything` (Reset, Clear Buffer) no vacía `folds`, cuando `remap_lines` sí los trata (`io.rs:302`). Queda un pliegue en una línea absoluta que ahora tiene otro texto. Al seleccionarla y hacer Fold, `unfold` cambia esa línea por las filas viejas: se pierde una línea y vuelve texto borrado. | `src/io.rs:253-269` (los pliegues son anclas) | Añadir `self.folds.clear()` en `forget_everything` | sí |
| PAR-19 | media | Emulación: títulos | `vte: ansi.rs:1350-1362`, `src/terminal.rs:3189-3196` | Solo se atienden OSC 0 y 2, con un único título. `OSC 1` (título del icono o pestaña) se ignora, y OSC 0/2 no distinguen pestaña y ventana, cuando CurseTerm ya tiene los dos títulos (Edit Tab Title / Edit Window Title). | iTerm2: `VT100/VT100Terminal.m:2780-2790` (WIN_TITLE, WINICON_TITLE), `:2915` (ICON_TITLE → pestaña), `:3049-3070` (pila de títulos por tipo) | Separar OSC 1 en `osc.rs` y llevarlo al título de pestaña | sí |
| PAR-20 | media | Emulación: identificación | `alacritty: term/mod.rs:1257-1272`; `vte: ansi.rs` (no hay brazo `('q', [b'>'])`) | XTVERSION (`CSI > q`) no tiene respuesta (iTerm2: `DCS >\|iTerm2 x.y ST`). DA1 responde `CSI ?6c` (VT102); iTerm2 responde `CSI ?62;…;4…c` (VT220, sixel…). DA2 responde `CSI >0;2600;1c`; iTerm2 responde `CSI >{vt};2500;0c`. Neovim, tmux o emacs deciden funciones con estas respuestas. | iTerm2: `VT100/VT100Terminal.m:2124-2149`, `VT100/VT100Output.m:1086-1112, 1115-1172, 1189-1192`, `Settings/iTermAdvancedSettingsModel.m:486` | Contestar `CSI > q` con `DCS >\|CurseTerm <versión> ST` desde la capa propia; documentar DA1/DA2 | sí |
| PAR-21 | media | Emulación: OSC 1337 | `src/integration.rs:55-81` | Faltan subórdenes cuya función ya existe en CurseTerm: `CursorShape`, `ClearScrollback`, `SetBadgeFormat` (fase 13), `HighlightCursorLine` (fase 30), `AddAnnotation`/`AddHiddenAnnotation` (fase 35), `SetProfile` (fase 65), `StealFocus`, `Copy`/`CopyToClipboard`/`EndCopy`, `RequestAttention`. Faltan también `SetUserVar`, `ReportCellSize`, `ShellIntegrationVersion`, `UnicodeVersion`, `SetColors`, `ReportVariable`, `Block`/`Button`/`OpenURL`… `progreso.md` no los menciona. | iTerm2: `VT100/VT100Terminal.m:4429-4609` | Añadir al menos las que reutilizan funciones existentes; listar el resto en «Difiere» | sí |
| PLT-04 | media | popups | `popup.rs:495-499` | `on_dismissed` corre fuera de contexto. Lo llama también `PopupHandler::done` (`:527-535`). | `fc/current_context.rs:55` | Ejecutarlo con el `run_in` del `owner`. | Sí, por lectura |
| PLT-07 | media | multiventana / D-Bus | `window.rs:513-525,822-828`; `../app_menu/server.rs:54-62,126-134`; `../app_menu.rs:78-88` | El menú solo se des-registra si hubo `appmenu_address`, que solo existe con `org_kde_kwin_appmenu_manager`. Sin él, el objeto `/MenuBar/N` sigue exportado tras cerrar la ventana y sus clics llenan un `action_queue` que nadie vacía. El `status_handle` nunca se des-registra. Tampoco se cierran los popups de la ventana cerrada. | `window.rs:513` (`if self.appmenu_manager.is_some()`) | Guardar el `path` del `MenuHandle` (`handle.path()`) y llamar siempre a `server::unregister`. Retirar el status item y cerrar los popups con `owner` igual al de la ventana. | Sí, por lectura |
| PLT-08 | media | memoria | `window.rs:531-559` | Cada ventana crea unos 10 `State::create_global`, y cada uno filtra un `UnsyncStorage` owner con `Box::leak`. Abrir y cerrar ventanas o paneles filtra memoria sin límite. | `fc/lifecycle/state.rs:519-527` | Crear esos `State` con `create_in_scope(ROOT)` del runner de la ventana, o reutilizar un owner por ventana. | Sí, por lectura |
| PLT-09 | media | escala | `surface.rs:338-345`; `surface.rs:156`; `surface.rs:99-105` | En `scale_factor_changed` (escala entera, sin fractional) se llama a `configure` y queda `set_buffer_scale(n)` pendiente sin pedir frame. El siguiente commit de `request_frame_callback` aplica la escala nueva al buffer viejo: tamaño erróneo, y `invalid_size` si el ancho o alto es impar. | Comparar con la ruta fraccional, que sí llama a `request_redraw` (`surface.rs:439`) | Dibujar justo tras `configure` (como en `apply_pending_window_configures`), no con un commit vacío. | Por lectura; el error de protocolo no está verificado |
| PLT-11 | media | configure | `window.rs:721-727`; `surface.rs:358-363` | Si hay un frame pendiente, el configure (ya confirmado por sctk, `sctk/shell/xdg/window/inner.rs:76`) se aplica en el próximo `frame`. Un compositor que no envía `done` a superficies ocultas lo retiene sin límite, y las transacciones de resize/maximize esperan. El orden «configure antes del commit» sí se cumple: no hay commits intermedios. | Código citado | Aplicarlo de inmediato con un commit que lleve la geometría, o fijar un plazo (timer) si no llega el frame. | No verificado (depende del compositor) |
| PLT-12 | media | Vulkan | `vulkan.rs:427-447`; `surface.rs:139-146` | La swapchain no valida `image_format`, `image_usage` (`SAMPLED\|TRANSFER_*`) ni `composite_alpha PRE_MULTIPLIED` contra `get_physical_device_surface_{formats,capabilities}` (VUID-…-01273/01276/01280). Si falla, `configure` registra el error y la ventana no se dibuja nunca. | Código citado | Elegir formato y alfa entre los soportados y quitar los usos no necesarios. | Sí, por lectura (la VUID es de la especificación) |
| PLT-14 | media | portapapeles | `clipboard.rs:265-286,322-332,204-216`; `primary.rs:58-78` | Cada cambio de selección ajena lanza un hilo que lee **todo** el contenido con `read_to_string`, sin límite ni timeout. Una fuente que no cierra el pipe deja un hilo y un fd vivos para siempre. No hace ping al terminar: pegar justo después de copiar devuelve «clipboard is empty». Con UTF-8 inválido queda vacío en silencio. | Código citado | Leer bajo demanda (en `get_contents`) con límite y timeout, o hacer `ping` al terminar y poner un tope de bytes. | Sí, por lectura |
| PLT-16 | media | IME | `ime.rs:43-51,45,52-57,61-62,110-122` | Habilita `text_input` al recibir el foco aunque no haya un campo de texto, y con `ContentPurpose::Terminal` fijo para cualquier app del kit. Ignora el serial de `done` y `delete_surrounding_text`. En `Leave` borra el preedit de la ventana con `keyboard_focus`, que puede haberse vaciado ya: el preedit viejo se queda. | Código citado | Habilitar o deshabilitar según `text_field_focused`; tomar el propósito del campo; guardar la superficie de `Enter`. | Sí, por lectura |
| PLT-17 | media | entrada (BK-030) | `input/seat.rs:270-298`; `runtime.rs:91`; `input/seat.rs:403-414`; `../views/collection.rs:66-95` | BK-030 **sigue**: `PlatformEvent::Mouse`/`MouseEventData` de Freya 0.4.3 no llevan modificadores. El kit sí los tiene (`self.modifiers`) pero no los expone, y repite internamente el workaround de CurseTerm (regla 1). | `fc/events/platform.rs:90-94`; `fc/events/data.rs:33-37` | Contexto raíz `State<Modifiers>` actualizado en `update_modifiers`; `collection.rs` lo consume. | Sí, por lectura |
| PLT-18 | media | teclado | `input/mod.rs:13-49,51-79,85-104`; `../menu/shortcut.rs:113-120` | Los atajos se comparan por glifo: con una distribución no latina (cirílico, griego), Ctrl+C da `"с"` y no casa. `Code` sale `Unidentified` para letras y dígitos. Faltan F13-F24, Print, Pause, Menu y `KP_*` sin NumLock, que acaban en `Unidentified`. | Código citado | Comparar también por el keysym de la primera distribución latina o por `raw_code`; ampliar `named`/`code`. | Sí, por lectura |
| PLT-21 | media | arranque | `runtime.rs:258-262` | Si falla `AppearanceWatch::new` (`create_dir_all` en un `$HOME` de solo lectura, o `inotify` ENOSPC por `max_user_watches`), **la app no arranca**. Es una función opcional que debería degradarse. | `vow-appearance/src/watch.rs:36-46` | Registrar el error y seguir sin vigilar. | Sí, por lectura |
| PLT-23 | media | secrets | `../secrets.rs:135-142`; `../secrets.rs:50-58` | `wrapped()` copia el secreto con `secret.to_vec()`, sin `Zeroizing`. La API es bloqueante (`zbus::blocking`) y CurseTerm la llama desde handlers de la interfaz (`curseterm/src/password_manager.rs:342,402`), así que congela el bucle si el llavero tarda o pide desbloqueo. `Locked` se devuelve aunque el usuario complete la petición. | Código citado | Envolver la copia en `Zeroizing`; exponer una variante async o documentar que va en un hilo propio. | Sí, por lectura |
| PLT-27 | media | multiventana / vídeo | `../video.rs:53-66` | `REDRAW` es un `thread_local` con un solo `WakeHandle` y captura el `Platform` de la **primera** ventana. El vídeo de otras ventanas pide redibujos a la primera. | Código citado | Un handle por ventana (contexto raíz). | Sí, por lectura |
| KIT-06 | media | ADR 0047 y a11y | barrier-kit/src/views/split_panes.rs:113-170 | El `Divider` de `SplitPanes` repite el agarre de `SplitView` (cursor, arrastre global) pero **sin foco ni teclado**: sin `a11y_id` ni `on_key_down`. El grip de `SplitView` sí tiene ambos (split.rs:354-392, `SPLIT_VIEW.key_step`). Incumple WCAG 2.1.1 | Comparación de los dos ficheros | Sacar el grip de `SplitView` a una primitiva y componerla en los dos | sí |
| KIT-07 | media | ADR 0047, regla 1 | barrier-kit/src/views/text_view.rs:100-263 frente a text_editor.rs:94-420 | `TextView` vuelve a implementar el editor (`use_editable`, preedit, `cursor_blink`, `Down`/`Move`/`Release`, Escape) en lugar de componer `TextEditor` o un núcleo común. Además recibe `family`/`size`/`line_height`/`ink` sueltos en vez de `Font` y `Palette` | `grep use_editable` → 2 ficheros con el mismo flujo | Extraer el núcleo editable que comparten | sí |
| KIT-08 | media | FloatingPane | barrier-kit/src/views/floating_pane.rs:31-55, 186-235 | Tres problemas: (a) sin tamaño mínimo, al redimensionar `width`/`height` pueden quedar negativos; (b) sombra con negro literal (`Rgba::rgb8(0,0,0)`, l.188) en vez de `ShadowStyle`/`SHADOW` de vow-design; (c) sin rol ni etiqueta a11y. API de campos públicos, distinta del resto de vistas (constructores encadenados) | Lectura del código y salida del script | Limitar a un `min_size` de token, usar el token de sombra, añadir `a11y_role` y `a11y_alt` con el título | sí (código) |
| KIT-09 | media | SplitPanes y FloatingPane | split_panes.rs:84-88,152-159; floating_pane.rs:200-208 | El arrastre aplica un delta incremental sobre `ratios`/`frame` capturados en el último render. Si llegan dos movimientos antes de repintar, se pierde el primero y el panel se queda atrás del puntero | Lectura del código | Guardar el origen del arrastre (frame/ratios y punto inicial) y calcular el delta total | no (comportamiento en ejecución) |
| KIT-10 | media | docs | docs/barrierkit/09-vistas.md:128 (y .en) | La documentación dice que `on_ratios` avisa al soltar el divisor, pero el código lo llama en cada movimiento (split_panes.rs:87, invocado desde l.158). También dice que «ningún panel baja de `pane_min`», y cuando no cabe hay pánico (KIT-01). El ejemplo de `TextView` usa `13.0, 16.0` literales en vez de `MONO.size`/`MONO.line_height` (09-vistas.md:141) | Lectura | Corregir el texto y el ejemplo | sí |
| KIT-11 | media | WCAG, vow-appearance | vow-appearance/src/resolve.rs:348-407 | En modo «Fondo de pantalla», el acento se ajusta a ≥ 3:1 contra la ventana **sin tinte** (`bg_window`, l.357), pero la ventana real se tiñe con el dominante (l.226-229, 342). En oscuro con tinte normal, el 44 % de los dominantes realistas (L∈[0.45,0.65]) quedan < 3:1 (mínimo 2.53:1). `on_accent` < 4.5:1 en el 12 % (oscuro, mínimo 4.05) y el 5 % (claro, mínimo 4.26), porque el bucle se rinde sin plan B. Tras subir la saturación (l.400-405) no se vuelve a comprobar `on_accent` | Simulación Python de la función (scratchpad `wa.py`, 2160 dominantes) | Medir contra `surfaces.window` ya tintada y, si `on_accent` no llega, cambiar `on_accent` a tinta oscura | sí (simulación, no en Rust) |
| KIT-12 | media | a11y | vow-appearance/src/resolve.rs:283; barrier-kit (0 usos) | `Resolved::contrast` («Aumentar contraste») solo se copia. Ningún control del kit lo usa; solo CurseTerm (terminal.rs:623) y el portal de malaysia. La preferencia de accesibilidad no cambia nada en las apps del kit | `grep '\.contrast\b'` en barrier-kit/src → 0 | Aplicarlo en `Palette`: líneas, separadores y `ink_faint` | sí |
| KIT-13 | media | a11y | barrier-kit/src/titlebar.rs:281,290,311,353,374,391; sidebar.rs:271,349,376 | Identificadores de test expuestos como etiqueta accesible: `a11y_alt("titlebar-title")`, `"titlebar-left-slot"`, `"sidebar-traffic-lights"`… Un lector de pantalla lee «titlebar-title» en lugar del título | Lectura; los tests los buscan por `label()` (tests/kit/chrome.rs:89) | Usar el `probe`/`author_id` que ya existe (sidebar.rs:134, controls.rs:149) | sí |
| KIT-14 | media | vow-design (fuentes) | vow-design/build/tengen.rs:72-78 | `font()` se queda con la **primera** familia de la pila: `MONO.family = "Geist Mono"` y se pierde `monospace` (tokens.json `font.families.mono`). Sin Geist Mono, CurseTerm y `TextView` caen a una fuente proporcional. Solo sans tiene pila completa (`SANS_FAMILIES`) | Lectura de build.rs:436-443 y tengen.rs | Generar `MONO_FAMILIES` (o `families: &[&str]` en `Font`) y usarlo en `font_family` | sí (código) |
| KIT-15 | media | Secret Service | barrier-kit/src/secrets.rs:135-143 | `wrapped()` copia el secreto a un `Vec<u8>` normal (`secret.to_vec()`), que se libera sin borrar. `GetSecret` sí devuelve `Zeroizing` (l.126-133). `Error` no implementa `Display` ni `std::error::Error` (l.18-22) | Lectura | Envolver en `Zeroizing` y borrar tras la llamada; implementar `Display`/`Error` (o `thiserror`, que ya está en las dependencias) | sí |
| KIT-16 | media | watcher `.domain` | vow-appearance/src/watch.rs:55-57, 78-103 | El renombrado atómico de un fichero sí se cubre (vigila el directorio con `IN_MOVED_TO`, `IN_CLOSE_WRITE` e `IN_DELETE`). Pero no se tratan `IN_DELETE_SELF`/`IN_MOVE_SELF`/`IN_IGNORED`: si se borra, se mueve o se sustituye por un symlink el directorio `~/.config/malaysia` (o el de datos), el watch muere en silencio y no se rearma. Sin `IN_CREATE`, un fichero creado por `symlink(2)` sin renombrar no avisa. No hay fugas: `Inotify` es dueño del fd, y el fd clonado en runtime pertenece a la fuente | Lectura | Atender `IN_IGNORED`/`IN_DELETE_SELF`: recrear el directorio y volver a llamar a `add_watch`; añadir `IN_CREATE` | parcial (código; la secuencia de syscalls de home-manager no se comprobó) |
| KIT-17 | media | vow-appearance | vow-appearance/src/config.rs:394-396 | Si `appearance.domain` tiene un error de sintaxis, `load_appearance` vuelve a los valores por defecto en silencio, sin `warn!`. Una errata cambia todo el tema sin rastro | Lectura | Registrar el error de `toml::de` | sí |
| KIT-18 | media | regla 2 (código muerto) | views/feedback.rs:83 `suppression`, views/split.rs:127 `background_extension`, drag.rs:52 `use_drag_source`, menu/model.rs:133 `with_badge`, print.rs:131 `text_pages`, print.rs:412 `text_pdf` | Seis funciones `pub` sin ningún uso en barrierkit (apps, tests, ejemplos), curseterm ni malaysia. Solo aparecen en docs/ | `grep -rnw` en los tres repos → solo la definición y la documentación | Una página o test en Tengen App Show (ADR 0047), o borrarlas | sí |
| KIT-19 | media | script de tokens | scripts/check-design-tokens.sh:15-24 | (a) Falso positivo en backdrop.rs:91, una conversión de píxeles. (b) No ve `Color::BLACK/WHITE` (print.rs:104,142) ni `Color::from_argb` con literales. (c) `awk … /#\[cfg\(test\)\]/{exit}` deja de leer en el primer `cfg(test)` y no revisa el código de producción que venga después. (d) No hay manera de marcar datos de muestra de las apps | Lectura y ejecución del script | Excluir los datos de muestra con una ruta explícita (p. ej. `*/catalog.rs`, `albums.rs`) o moverlos a datos (`.toml`); ajustar el patrón | sí |
| KIT-20 | media | tokens en apps | marea/src/{player,hero,shelf}.rs, tengen-app-show/src/main.rs:587-600 | Además de los colores: velos y scrims `rgba8(0,0,0,0.66)`, `(255,255,255,0.7)`… y medidas literales en Tengen App Show (`320×150`, `corner_radius(12.0)`, `padding(16.0)`). En barrier-kit: `Gaps::new(6.0, 8.0, 6.0, 8.0)` en `PasswordField` (controls.rs:88), un literal que `TEXT_FIELD` debería dar | Salida del script y grep | Llevar velos y medidas a vow-design | sí |
| KIT-21 | media | dependencia externa | barrier-kit/src/notifications.rs:54-63 | `play_event_sound` lanza `canberra-gtk-play`, que no está en ningún flake (barrierkit, curseterm ni malaysia). En NixOS falla con un `warn`. `WAITING` (l.15,47) crece si el servidor nunca emite `NotificationClosed` | `grep canberra` en los flakes → 0 | Declarar la dependencia (libcanberra-gtk3) o reproducir con GStreamer, que ya está; limpiar `WAITING` cuando caduque | sí (grep) |
| INT-03 | media | plataforma | kit `timer.rs:52-57`, `runtime.rs:231-236`; CT `terminal.rs:3109,3463`, `windows.rs:191` | Cada ventana de terminal deja **dos** callbacks de `wakeup` registrados para siempre, con su `Handles` (estados, `Rc`, receptor del canal). Como las ventanas ya se abren en el mismo proceso, la cola crece con cada ventana y cada despertar la recorre entera | `wakeup` solo hace `push`; nadie hace `retain`; `use_alive` solo convierte la llamada en un no-op | Kit: `use_wakeup` que se dé de baja en `use_drop` (la API que propone BK-014). Doc: BK-014 y BK-032 como «abierto» | Sí (código) |
| INT-04 | media | regla 1 | `links.rs:41-48` | `links::open` lanza `xdg-open`; el kit ya trae `portal::open_uri` (OpenURI) | kit `portal.rs:28-46` | Usar `spawn(portal::open_uri(..))` y `portal::file_uri` para rutas | Sí |
| INT-05 | media | regla 1 / UX | `terminal.rs:2216-2229,2300-2310`, `2326-2365` | Save Selection, Save Contents, Log, Save Archive y Restore Archive piden una ruta tecleada en una hoja. El kit ya trae `portal::save_file` y `open_files`. No está anotado como gap | kit `portal.rs:15,23` | Usar el selector del portal; quitar `from_home` y las hojas de ruta | Sí |
| INT-06 | media | regla 1 / BK-039 | `global_search.rs:61-170` (`Position::new_absolute()` en `:110`) | Find Globally es un panel fijo dentro de la ventana con filas hechas a mano. El kit trae `OutlineView` y `open_utility_window` con `PanelBehavior`. Con ventanas en el mismo proceso, además podría buscar en todas | kit `views/tree.rs:115-135`; `window.rs:416-420`; `06-ventanas.md:54-94` | Panel utilitario con `OutlineView`; actualizar BK-039 | Sí (API); el resultado no se verificó |
| INT-07 | media | docs | `docs/barrierkit-gaps{,.en}.md` (líneas 95-1003, 26 referencias) | El doc no dice la verdad: cuerpos con los parches y el bloque borrados, rodeos ya retirados (BK-023, 042 y 046), reproducciones que ya no fallan, líneas del kit viejas, rango de commits incompleto y resumen inicial incompleto. Choca con las reglas 3 y 5 de AGENTS (fuente real) | Ver la tabla del §1 | Reescribir cada cuerpo con el estado real y el commit del kit; borrar «Parche probado»; fundir 033 en 004 y 034 en 005 | Sí |
| INT-15 | media | plataforma | `password_manager.rs:44-63,139-142,342-356,402`; kit `secrets.rs:50` (`zbus::blocking`) | Todas las llamadas al Secret Service son bloqueantes y van en el hilo de la interfaz, incluida la carga al montar (`use_hook`); cada una abre conexión y sesión nuevas. Si el servicio tarda (activación D-Bus), la interfaz se congela. `14-plataforma.md:65-67` no avisa | — | Llamar en un hilo con canal y `WakeHandle`, o API asíncrona en el kit; documentarlo | No |
| INT-20 | media | kit / persistencia | kit `app/defaults.rs:55-62,36-45`; CT `config.rs:499-508` | `write_toml` usa un temporal fijo (`<nombre>.tmp`). Dos procesos de CurseTerm que guardan a la vez (Ajustes en otro proceso, o `curseterm` lanzado desde el escritorio) pueden renombrar un temporal a medias. `read_toml` lo daría por inválido y devolvería valores por defecto, y el siguiente `change_defaults` los guardaría (perfiles perdidos; solo queda un `.bak`). La lectura, cambio y escritura entre procesos también pierde cambios | `path.with_extension("tmp")`; `back_up` y `T::default()` | Kit: temporal único (pid o aleatorio) y no sobrescribir tras un fallo de lectura | No (carrera) |
| RUL-03 | media | comentarios (regla 6) | 275 bloques; p. ej. `session.rs:174`, `workspace.rs:313`, `terminal.rs:2566`, `io.rs:522`, `composer_view.rs:71` | Citan el método de iTerm2 y después explican qué hace el código («PTYSession.m -foo:: the session keeps running…»). La regla 6 lo prohíbe. Además, documentan una traducción método a método (agrava RUL-01). | `comments-iterm-que-hace.txt` (275) | Dejar solo la cita (`// iTerm2: PTYSession.m -foo:`) o llevar el mapa de paridad a una tabla de `docs/progreso.md`. Conservar la parte que explica un porqué que el código no expresa | sí (clasificación por script + lectura) |
| RUL-05 | media | comentarios (regla 6) | 28 bloques; `workspace.rs:232`, `io.rs:642`, `key_report.rs:331`, `terminal.rs:194`, `theme.rs:107` | Comentarios de «qué hace» sin cita de iTerm2. Repiten el nombre de la función o del dato. | `comments-que-hace-otros.txt` | Borrarlos o renombrar lo que explican (regla 5) | sí |
| RUL-08 | media | duplicación (reglas 1 y 3) | `terminal.rs:1796,1975,2150,2181,2295,2594,2625,4043,4184` | El rodeo BK-027 (`handles.after(Duration::ZERO, …)` + `sheet.set(None)`) se repite 9 veces, con 6 comentarios casi idénticos (`:1972,2147,2292,2593,2624,4040`). | `grep -n "after(Duration::ZERO"` → 9 | Crear un único `Handles::next_loop(f)` con un solo comentario BK-027 | sí |
| RUL-09 | media | duplicación (regla 1) | `pane_view.rs:441` `is_dark`, `theme.rs:123` `perceived_dark` | Las dos funciones calculan la misma fórmula (0.30/0.59/0.11 < 0.5) y citan a la vez `NSColor+iTerm.m` `-isDark` y `-perceivedBrightness`. | Lectura directa | Dejar una sola (`theme::perceived_dark`) | sí |
| RUL-14 | media | tokens | `indicators.rs:52`, `hint_view.rs:119,152`, `global_search.rs:114,116`, `drop_target.rs:130`, `tab_bar.rs:558-560`, `theme.rs:134-136` | Hay literales de diseño fuera de `tokens.rs`: el radio `size / 6.0` (el comentario de `tokens.rs:280` lo describe, pero `INDICATOR` no tiene ese campo), la rotación `180.0`, los máximos del 95 % y el 90 %, los insets 0.5/1.5 y el blanco/negro con `rgba8(255,…)`/`(0,…)`. | `literales-diseno.txt` | Pasarlos a campos de `INDICATOR`, `SHORTCUT_LABEL`, `GLOBAL_SEARCH` y `MOVE_PANE`; blanco y negro como constantes de `tokens.rs` o `vow_design` | sí |
| RUL-18 | media | i18n | `itermcolors.rs:32-36` → `preferences.rs:1596` | Los mensajes de `ImportError` («cannot read the file», «not an .itermcolors property list», «the scheme has no «…»») llegan a Ajustes dentro de `tr!("import-failed", error = …)`, así que salen en inglés con la interfaz en español. | Lectura directa | Una clave Fluent por variante | sí |
| RUL-22 | media | docs (regla 3) | `docs/barrierkit-gaps.md:197-198,218,263,598,810` (+ `.en.md`) | El cuerpo de BK-006, 007, 009, 026 y 036 describe como solución actual el crate `crates/barrier-kit-next`, que el ADR 0006 borró. La tabla resumen (`:30-31`) dice otra cosa. El resumen (`:5-11`) da 5 gaps abiertos, pero la tabla tiene unos 15 con workaround (BK-012, 016, 020, 021, 027, 030, 037–040, 043…). | `grep barrier-kit-next` → 18 menciones; la ruta no existe | Reescribir el «Cómo lo resolví» de esos gaps con la API del kit y alinear el resumen con la tabla | sí |
| RUL-26 | media | pánico | `grid.rs:91` | `.expect("the font manager has no fonts")` hace que la app aborte si el sistema no tiene fuentes. Es la única ruta no de test que depende del sistema y puede entrar en pánico. Las otras dos (`config.rs:533`, `links.rs:10`) son constantes internas. | `grep unwrap/expect` fuera de tests: 3 | Devolver un error y avisar en el log en lugar de abortar | sí; que se alcance en la práctica, no |
| LAB-14 | baja | marcas / orden capturada | curseterm `io.rs:253-270`, `io.rs:836-850` | `command_start` no está en `anchored_lines`: con el historial lleno o tras un reajuste de líneas, el texto de la orden que se guarda en el historial de órdenes sale desplazado. No hay pánico (`start <= end`). | Lectura de código. | Añadir `command_start` a `anchored_lines`. | Código, no ejecutado |
| LAB-15 | baja | paneles | curseterm `terminal.rs:1454`; barrierkit `views/split_panes.rs` | Dividir no tiene tamaño mínimo: tras cinco divisiones quedan paneles de 1–2 columnas (iTerm2 rechaza dividir por debajo de un mínimo). Con esos paneles, arrastrar un divisor dispara KIT-01. | Laboratorio: `capturas/auditoria-paneles-estrechos.png`. | Rechazar `split` si algún panel queda por debajo de `SPLIT_PANES.pane_min`. | Sí, ejecutado |
| LAB-16 | baja | escritorio | curseterm `share/applications/org.curseterm.CurseTerm.desktop` | La acción «New Window» ejecuta `curseterm`, que abre otro proceso en lugar de una ventana del proceso vivo; con LAB-03, Quit en uno mata al otro. Falta `StartupNotify` y el icono es el genérico `utilities-terminal`. | Lectura del archivo. | Hacer que un segundo `curseterm` pida la ventana al proceso vivo (el registro de `windows.rs` ya existe) o documentarlo. | Sí |
| SEC-15 | baja | Notificaciones OSC 9 y 1337 | curseterm `terminal.rs:3372-3404`, `config.rs:395`; barrierkit `notifications.rs:27-50` | El texto remoto va sin escapar al `body` de `Notify`, y la especificación de notificaciones de freedesktop permite marcado en el cuerpo (`<b>`, `<a href>`, `<img>` según el servidor). El `summary` de 1337 es libre (puede imitar otra alerta). No hay límite de frecuencia: cada notificación lanza un hilo. Está mitigado porque `profile.notifications` vale `false` por defecto (`config.rs:395`). | `notice.body.as_str()` pasado tal cual | Escapar `&`, `<` y `>` en `body`. Limitar a N notificaciones por segundo y sesión. | Sí (código); lo que hace cada servidor no se ha comprobado |
| SEC-16 | baja | D-Bus: dbusmenu | barrierkit `app_menu/server.rs:285-297`, `platform/window.rs:525` | El menú se exporta en el bus de sesión (`com.canonical.dbusmenu`). Cualquier proceso del mismo usuario puede mandar `Event(id,"clicked")` y disparar acciones del menú: Pegar (pega el portapapeles en el shell), Enviar fragmento, Abrir selección (`xdg-open`)… Un proceso del mismo usuario ya puede hacer más que esto (escribir en el PTY con `ptrace` y otros), así que el riesgo es menor. CurseTerm no publica ninguna otra interfaz propia en D-Bus (`app::launch` y `StatusItem` no se usan). | `fn event(&self, id: i32, event_id: String, …) { … push_action(action_id) }` | Aceptado. Como mucho, ignorar `Event` si la ventana no tiene foco o si no llega un `AboutToShow` previo. | Sí |
| SEC-17 | baja | Entorno de los hijos | curseterm `io.rs:793`, `session.rs:417`, `links.rs:43`, `windows.rs:198`; at `tty/unix.rs:240-241` | El shell del PTY quita `XDG_ACTIVATION_TOKEN` y `DESKTOP_STARTUP_ID`, pero `run_command`, el coprocesador, `xdg-open` y la ventana nueva heredan el entorno de CurseTerm completo: el token de activación de arranque se reutiliza. También heredan `PATH` (`xdg-open` y `canberra-gtk-play` se resuelven por `PATH`, lo normal en escritorio). El shell recibe `ALACRITTY_WINDOW_ID`, aunque no es Alacritty. | `std::process::Command::new(shell).args(["-c", command])` sin `env_remove` | `env_remove("XDG_ACTIVATION_TOKEN")` y `env_remove("DESKTOP_STARTUP_ID")` en estos `Command`. | Sí |
| SEC-18 | baja | Aviso de pegado | curseterm `terminal.rs:1852-1859`, `paste_warning.rs:32-50` | El aviso solo cuenta líneas. No avisa de caracteres de control (iTerm2 tampoco, pero los quita: ver SEC-03). El `at_prompt` y el modo entre corchetes se miran en la sesión activa, aunque se pega en todas las `receivers()` (difusión). | `handles.active().is_some_and(\|session\| session.at_prompt())` y después `for session in handles.receivers()` | Evaluar el aviso en cada destino de la difusión, o con el peor caso. | Sí |
| SEC-19 | baja | Reglas de AGENTS.md | curseterm `integration.rs:62`, `paste_transform.rs:92` | Hay comentarios que explican qué hace el código («iTerm2 shell integration: OSC 1337;RemoteHost=user@host.», «Tab, line feed, form feed and carriage return stay.»), y la regla 6 de «Código mínimo» los prohíbe. | — | Quitarlos o convertirlos en el *porqué* (referencia a iTerm2). | Sí |
| PAR-13 | baja | Menú: Clear Find | `src/menu.rs:138-139`, `docs/progreso.md:2249` | Clear Find es Super+Alt+E; en iTerm2 es ⌥⇧⌘E (⌥⌘E es Render Selection Natively). | iTerm2: `MainMenu/Base.lproj/MainMenu.xib:838-839` | `super_with(ALT \| SHIFT, 'e')` | sí |
| PAR-18 | baja | Imágenes en línea / anclas | `src/io.rs:861-868` | Los saltos de línea de una imagen se dan con `term.linefeed()` directamente, sin `line_feed_counting_lost_lines`. Con el historial lleno, las marcas, las anotaciones y los resaltados se desplazan la altura de la imagen. La fase 61 solo lo corrigió para la salida normal. | `src/io.rs:558-577` | Pasar los saltos de la imagen por `line_feed_counting_lost_lines` | sí |
| PAR-22 | baja | Emulación: DECSCUSR | `src/session.rs:276-287`, `docs/progreso.md:1281` | Una app cambia el parpadeo con DECSCUSR (`\e[2 q` lo para). En iTerm2, «Allow changing cursor blink» viene en NO: el tipo cambia, pero el parpadeo sigue el del perfil. La fase 27 lo da por bueno. | iTerm2: `PTYSession/PTYSession.m:17119-17125`, `Settings/iTermProfilePreferences.m:1182` | Opción de perfil y conservar `blinking` del perfil si está apagada | sí |
| PAR-23 | baja | Entorno del PTY | `src/session.rs:198-200` | Solo `TERM`, `COLORTERM` y `TERM_PROGRAM=CurseTerm`. iTerm2 exporta `LC_TERMINAL=iTerm2`, `LC_TERMINAL_VERSION`, `ITERM_SESSION_ID`, `TERM_SESSION_ID`, `TERM_PROGRAM_VERSION` e `ITERM_PROFILE`. Las herramientas que deciden usar `1337;File` mirando `LC_TERMINAL` o `TERM_PROGRAM` no muestran imágenes en CurseTerm. | iTerm2: `PTYSession/PTYSession.m:3644-3675` | Exportar `TERM_PROGRAM_VERSION`, un `TERM_SESSION_ID` y documentar por qué no `LC_TERMINAL` | sí |
| PAR-24 | baja | Perfil: valores por omisión | `src/config.rs:384, 389, 390` | `flash_visual_bell = true` (iTerm2 NO, `DefaultBookmark.plist` false), `prompt_before_closing = IfJobs` (iTerm2 `PROMPT_NEVER`) y `alt_key = Escape` (iTerm2 `OPT_NORMAL`). Son elecciones razonables en Linux, pero no constan como diferencia. | iTerm2: `Settings/iTermProfilePreferences.m:1225, 1230, 1243`, `Settings/PreferencePanel.h:52-56`, `plists/DefaultBookmark.plist:1058-1059` | Anotarlas en «Difiere de iTerm2» de las fases 5 y 6 | sí |
| PAR-25 | baja | Emulación: XTWINOPS | `vte: ansi.rs:1739-1745` | Solo 14t, 18t, 22t y 23t. Faltan 16t (tamaño de celda, que usan las herramientas de imágenes), 19t, 21t, 11t, 13t… | iTerm2: `VT100/VT100Terminal.m:2985-3047` | Añadir 16t con `cell_width`/`cell_height` (ya están en `TextAreaSizeRequest`) | sí |
| PAR-26 | baja | Documentación | `docs/progreso.md:711, 926, 2249, 3180` | Afirmaciones de paridad falsas: «el doble clic separa en `/`» (PAR-14), «con Mayús… como en iTerm2» (PAR-10), «Clear Find (⌥⌘E)» (PAR-13) y «Soft Boundaries (⌥⌘A) de iTerm2» (PAR-12). | ver los ID citados | Corregir en `progreso.md` y `progreso.en.md` | sí |
| PLT-10 | baja | escala | `surface.rs:150-157,288-295,335-337` | Si hay `wp_fractional_scale_manager_v1` pero no `wp_viewporter`, se crea el objeto fraccional: el buffer se dimensiona con escala fraccional y se declara con `set_buffer_scale(round)`, y se ignoran las escalas enteras. | Código citado | Crear `fractional` solo si también hay viewporter. | Sí, por lectura |
| PLT-13 | baja | Vulkan | `vulkan.rs:465-481`; `vulkan.rs:41` y `:700` | Si falla a mitad de `recreate_swapchain` (imágenes o semáforos), se pierden los semáforos ya creados y queda un estado incoherente. `B8G8R8A8_UNORM` está duplicado en vez de usar `SWAPCHAIN_FORMAT` (regla 1). | Código citado | Liberar en el `Err`. Usar la constante. | Sí, por lectura |
| PLT-15 | baja | portapapeles / DnD | `clipboard.rs:403-423`; `primary.rs:90-96`; `clipboard.rs:343-354` | `send_request` lanza un hilo por petición, que se bloquea si el lector no lee (pipe lleno). En `drop_performed`, si la lectura falla no hay ping, y `pending_drop`/la oferta nunca reciben `finish`/`destroy`. El orden de publicación es correcto: la fuente con sus MIME se crea antes de `set_selection` (`clipboard.rs:257-260`, `primary.rs:49-52`). | Código citado | Escritura no bloqueante en calloop y limpieza de `pending_drop` cuando falla la lectura. | Sí, por lectura |
| PLT-19 | baja | teclado | `input/seat.rs:439-456,462-472,481-487` | El `KeyDown` de un atajo se consume, pero su `KeyUp` se entrega a la app. Escape en un panel siempre lo cierra y nunca llega al contenido (p. ej. un campo con autocompletado). | Código citado | Consumir el `KeyUp` emparejado; dejar Escape a la app si lo gestiona. | Sí, por lectura |
| PLT-20 | baja | repetición | `input/seat.rs:501-531,346,398-400` | Hay una sola repetición global (no por ventana). Se para al perder el foco (`:346`) y al soltar la tecla (`:398`). Si `insert_source` falla (`let _`), `pending` queda en `true` y la repetición muere. Mantener un atajo repite la acción de menú (p. ej. cerrar). | Código citado | Comprobar el error; no repetir atajos de menú. | Sí, por lectura |
| PLT-22 | baja | protocolos | `popup.rs:470` | `xdg_popup.reposition` (desde v3) se envía sin comprobar la versión de `xdg_wm_base`, que sctk enlaza con `1..=MAX` (`sctk/shell/xdg/mod.rs:63`). El backend no comprueba `since`, así que con v<3 es un error de protocolo fatal. El resto de globals opcionales (appmenu, infinity, fractional, viewporter, text-input, primary, activation, cursor-shape, data-device) usan `.ok()` y degradan sin pánico (`runtime.rs:306-410`). `set_panel`/`set_popover`/`set_overlay` comprueban la versión (`window.rs:470`, `popup.rs:438`, `window.rs:549-552`). | Código citado; `wb/sys/client_impl/mod.rs` no valida `since` en `send_request` | Cerrar y reabrir el popup si `version() < 3`. | Sí, por lectura |
| PLT-24 | baja | notificaciones | `../notifications.rs:15,27-50,79-82,99-110` | Un hilo por `post`. `WAITING` crece si el servidor no emite `NotificationClosed`. El callback `clicked` corre en el hilo de señales de zbus (el `Send` lo exige, pero no está documentado). El hilo de señales nunca termina. | Código citado | Tope o limpieza de `WAITING`; documentar el hilo del callback. | Sí, por lectura |
| PLT-25 | baja | StatusNotifierItem | `../status_item.rs:11,146-150,167-181` | `ITEM_PATH` es fijo: un segundo status item del proceso no se exporta (`at` devuelve `Ok(false)`), pero registra su nombre igualmente. La tarea `status-item-watcher` queda en `detach` para siempre y no hay forma de retirarla. | Código citado | Ruta por índice; mantener el handle de la tarea para retirarla. | Sí, por lectura (la semántica de `at` en zbus no está verificada en su código) |
| PLT-26 | baja | dbusmenu | `../app_menu/server.rs:45-48,88-95` | `.expect("barrier-dbusmenu thread")` hace pánico si no se puede crear el hilo. Sin bus de sesión, el hilo termina y los `send` posteriores se pierden en silencio (`let _ =`, `:51`). | Código citado | Registrar el error en vez de `expect`. | Sí, por lectura |
| PLT-28 | baja | portal | `../portal.rs:39,138,165,185` | `parent_window` siempre es `""`: diálogos sin padre (sin `xdg-foreign`). | Código citado | Exportar la toplevel con `xdg_foreign` cuando exista. | Sí, por lectura |
| PLT-29 | baja | rendimiento / vídeo | `dmabuf.rs:178-185`; `vulkan.rs:251-255` | Cada `Imported` que cae hace `flush_and_submit` + `device_wait_idle`: un vaciado de GPU por fotograma de vídeo. | Código citado | Diferir la destrucción con un fence por fotograma. | Sí, por lectura |
| PLT-30 | baja | Vulkan SAFETY | `vulkan.rs:449-451,724-741` | Los SAFETY de destrucción suponen que `device_wait_idle` cubre al motor de presentación, que no es una operación de cola. Con maintenance1 disponible, el `Drop` no espera los fences de `presents`. | Especificación de `vkDestroySwapchainKHR` | Con `present_fences`, esperar `presents` antes de destruir. | Por lectura |
| PLT-31 | baja | multiventana | `runtime.rs:194-198`; `window.rs:443-463` | `open_utility_window` desde un popup, o desde una ventana ya cerrada, crea una toplevel normal (sin `parent`), no un panel. | Código citado | Resolver el padre del popup a su `owner`. | Sí, por lectura |
| PLT-32 | baja | rendimiento | `input/seat.rs:450-451`; `../menu/shortcut.rs:113-114` | Cada `KeyDown` clona el `AppMenu` entero (`snapshot`) y vuelve a parsear todos los atajos. | Código citado | Cachear los atajos parseados por revisión. | Sí, por lectura |
| PLT-33 | baja | mapeo | `surface.rs:99-124` | Si el primer `draw` falla (p. ej. swapchain que no se pudo crear), `draw_count` sigue en 0 y `request_frame_callback` no hace commit. La superficie queda sin mapear hasta otro configure. | Código citado | Reintentar con un timer o propagar el error. | Sí, por lectura |
| PLT-34 | baja | estilo | `runtime.rs:310,323,328,352`; `clipboard.rs:420`; `vulkan.rs:216` | Los mensajes de log mezclan español e inglés (el resto del crate los tiene en inglés). | Código citado | Unificar. | Sí |
| KIT-22 | baja | vow-design (duplicados) | tengen/components.json | Hay literales iguales a tokens base en lugar de referencias: 4 radios (`image-well.radius 8` = `radius.sm`, `traffic-lights.radius 4` = `radius.xs`, `launcher.corner-radius 16` = `radius.lg`, `popover.arrow-tip-radius 4`), 2 duraciones (`launcher.close-duration 120` = `duration.fast`, `cursor-shake.shrink-ms 220` = `duration.base`) y 41 márgenes iguales a `space.N`. `SIDEBAR_*` y `BG_SUNKEN_*` salen de la misma ruta (build.rs, tabla THEMED) | Script Python sobre los JSON | Usar referencias (`"radius.sm"`, como ya hace `button.radius`) donde el valor deba seguir al token | sí |
| KIT-23 | baja | vow-design (uso) | build.rs (255 constantes generadas) | Ningún token generado queda sin uso: todos aparecen en barrierkit, curseterm o malaysia. 4 campos de `ShadowStyle` solo se usan dentro de `vow-design` (lib.rs:848-860), y es correcto | Script de recuento (`usage.tsv`) | — | sí |
| KIT-24 | baja | comentarios (regla 6) | 68 comentarios `//` fuera de `platform` | Casi todos explican un porqué con su fuente. Unos 8 explican el qué o pasan de una línea: sidebar.rs:231, titlebar.rs:262, marea/player.rs:198, charts/space.rs:7, vow-appearance/backdrop.rs:84, notifications.rs:53; de dos líneas: marea/hero.rs:31-32, backdrop.rs:263-264, text_view.rs:144-145. No hay `///`, `//!`, `TODO` ni `#[allow(dead_code)]` | `grep` | Recortar o renombrar | sí |
| KIT-25 | baja | regla 3 | vow-appearance/src/watch.rs:123-125 | `into_io_error` envuelve `io::Error::from` sin aportar nada | Lectura | `.map_err(io::Error::from)` | sí |
| KIT-26 | baja | Fluent | trazo/i18n/{en,es}/trazo.ftl:27 | La clave `disk` no se usa. Por lo demás, las claves en y es coinciden en los 5 crates (121, 91, 63, 346 y 56) y no hay ninguna usada sin definir | Script `comm` sobre las claves y los usos de `tr!`/`text!` | Borrar `disk` | sí |
| KIT-27 | baja | API | views/split_panes.rs:9,18 | `label: &'static str` obliga a pasar un literal o el `&'static` filtrado de `tr!`. El resto de vistas aceptan `impl Into<String>` | Lectura | `impl Into<Cow<'static, str>>` | sí |
| KIT-28 | baja | Cargo.lock | Cargo.toml:47-48 | `toml = "0.8"` y `toml_edit = "0.22"` del workspace duplican `toml 1.1.8` y `toml_edit 0.25.15`, que traen `freya-skia-bindings`, `system-deps` y `proc-macro-crate`. Además `png 0.17` trae `bitflags 1.3.2`. Hay otros 20 duplicados transitivos (itertools ×3, syn 2/3, thiserror 1/2…) | `awk` sobre Cargo.lock | Subir a `toml 1`, `toml_edit 0.25` y `png 0.18` | sí |
| KIT-29 | baja | dependencias | barrier-kit/Cargo.toml:21 | `freya-skia-safe` es obligatoria, pero solo la usa `video.rs` (feature `video`). Las demás dependencias declaradas se usan. `i18n-embed-fl` en las apps la exige la macro `tr!` | grep por crate | Hacerla opcional dentro de `video` | sí |
| KIT-30 | baja | licencias | NOTICE; sin `deny.toml` | NOTICE cubre las fuentes de Marea, Sprite Fright, vulkan.rs y Suzanne. Faltan `controls.rs:1149` («Mirrors Input's default pre-handler», freya-components, MIT) y `glass_container.rs:92` (fórmula de Inigo Quilez). No hay `cargo-deny` | Lectura | Valorar si son copias; añadir `deny.toml` | sí |
| KIT-31 | baja | docs | docs/ | Todos los `.md` tienen su `.en.md` (salvo AGENTS.md y CLAUDE.md, exentos), con los mismos encabezados, bloques de código, tablas y listas. Los enlaces relativos se resuelven, y los nombres `barrier_kit::…`/`vow_design::…` citados existen. `./dev.sh kit-local/kit-remote` (ADR 0048) existen en el `dev.sh` de malaysia, no en el de barrierkit | Scripts de comparación | Indicar en el ADR 0048 que es el `dev.sh` de malaysia | sí |
| INT-08 | baja | docs | `barrierkit-gaps.md:159-186,740-781` | BK-004 = BK-033 y BK-005 = BK-034, con estados contradictorios | Mismo protocolo, commit y API | Dejar uno y redirigir el otro | Sí |
| INT-09 | baja | docs / BK-037 | `barrierkit-gaps.md:827-845`; `terminal.rs:1889-1891` | `Alert` ya se puede silenciar (`suppression`), y la hoja propia podría llevar la casilla con `Checkbox` del kit | kit `views/feedback.rs:83-86,130-134` | Corregir BK-037 a «parcial»; añadir la casilla a `paste_question_sheet` | Sí |
| INT-10 | baja | regla 1/3 | `composer_view.rs:217-246`; `session_note.rs:344-382` | Dos copias del mismo material (desenfoque `SHEET_MATERIAL_BLUR`, tinte y borde). La tercera es `glass::material` del kit, que no admite tinte ni borde | kit `glass.rs:272-319` | Kit: `material(radio).tint(..).border(..)`; CurseTerm lo usa en los dos sitios | Sí |
| INT-11 | baja | regla 1 | `terminal.rs:1752-1763` | `use_profiles_menu` repite el patrón privado `use_menu_update` del kit (estado publicado y handle) para `set_menu_items` | kit `app_menu.rs:259-273` | Kit: `use_menu_items(título, ítems)` público | Sí |
| INT-12 | baja | regla 3 | `pane_view.rs:535-550` | `Primary` es un envoltorio de un solo uso de `WindowControls::primary_selection()` | — | Llamar a `primary_selection()` directamente | Sí |
| INT-13 | baja | regla 1 | `tab_bar.rs:546-554`; `pick.rs:1-7` | `over` repite `Rgba::mix` (`vow-design/src/lib.rs:14`); `pick::matching` no pliega acentos como `views::search::fold` (`views/search.rs:13`) | — | `bottom.mix(top, top.a)` con alfa 1; `fold` en el filtro | Sí |
| INT-14 | baja | vistas | `session_note.rs:66-104`; `terminal.rs:1272-1290`; kit `views/floating_pane.rs:31-55,195-203` | `SessionNoteView` reimplementa mover y redimensionar como `FloatingPane`. El `FloatingPane` del kit no tiene mínimo; CurseTerm limita el marco después, así que al estrechar por la izquierda el panel se desliza en vez de pararse. Además, aplica el delta al marco del último render y pierde recorrido si llegan varios eventos por frame | `Sides::apply` sin mínimo; `clamped` solo recorta el ancho | Kit: `FloatingPane::min_size` y delta desde el inicio del arrastre; CurseTerm reutiliza `Sides` | No (comportamiento) |
| INT-16 | baja | regla 1 / BK-001 | `windows.rs:25-56,60-98,203-230`; `main.rs:74-85` | La instancia única se hace a mano: registro por pid, `/proc/<pid>/exe` y `SIGTERM`. El kit trae `app::launch`, pero solo reenvía `Activate` y su `Open` está vacío, así que no sirve para `--profile` y demás | kit `app/application.rs:80,122-140` | Anotar el gap («`launch` no reenvía argumentos») y usarlo cuando esté | Sí |
| INT-17 | baja | docs del kit | `docs/barrierkit/*` | `Timers`, `after`, `wakeup` y `use_alive` no aparecen en ningún capítulo. `11-app.md:30` no avisa del contexto (BK-017). `05-controles.md` dice 31 símbolos (hay 36). `01-que-es.md:30` dice «cinco apps» (hay 4) | `grep` vacío para `Timers` y `use_alive` | Documentarlo en `14-plataforma` y `11-app` (en el kit) | Sí |
| INT-18 | baja | docs | `docs/progreso.md:225-227` | Cita `curseterm --import-colors <archivo>` y «ruta en un campo», que ya no existen | `windows.rs:101-114` no trata esa opción | Corregir (`.md` y `.en.md`) | Sí |
| INT-19 | baja | seguridad | `styled.rs:21-24` | `family` entra sin escapar en un atributo `style` del HTML copiado; una familia con `"` o `'` rompe el HTML del portapapeles | — | Escapar comillas, o `font-family` entre comillas dobles con `&quot;` | Sí (código) |
| RUL-04 | baja | comentarios | 70 bloques; p. ej. `panes.rs:134`, `terminal.rs:3355`, `snippets.rs:34`, `smart_selection.rs:106` | Son citas sin explicación, útiles como referencia de paridad. La regla 6 no las contempla. | `comments-iterm-cita.txt` | Criterio: se aceptan. Hay que añadirlas a la regla 6 de `AGENTS.md` como excepción explícita, o moverlas a docs | sí |
| RUL-06 | baja | comentarios | `auxiliary.rs:1`, `preferences.rs:1`, `windows.rs:1`, `tab_bar.rs:1-2`, `throughput.rs:1-2` (y `io.rs:1`, `osc.rs:1`) | Hay 7 `//!` que describen el módulo. Solo `io.rs` y `osc.rs` citan un ADR. | `comments-classified.txt` § inner-doc | Borrar los cinco que no citan ADR | sí |
| RUL-07 | baja | comentarios | `workspace.rs:26,217`, `arrangement.rs:31`, `terminal.rs:1271`, `tokens.rs:20`, `menu.rs:274` | Citan `floating-panes-design.md` y `docs/notes-3.7.0beta4.txt` como si fueran rutas del repo. Son del repo de iTerm2 y no existen en CurseTerm. | `find` solo los encuentra en `/home/user/iterm2/docs/` | Prefijar con `iTerm2:` o enlazar la URL | sí |
| RUL-10 | baja | duplicación | `hint_view.rs:125`, `pane_view.rs:508` | `progress(since, duration_ms)` aparece dos veces; solo cambia el `Option`. | Lectura directa | Una sola función; `pane_view` llama con `map_or` | sí |
| RUL-11 | baja | duplicación | `pane_view.rs:436` vs `theme.rs:51,132`; `tab_bar.rs:556` vs `theme.rs:133-137` | La conversión `Rgb→Rgba` está en una función y además en línea. La elección de tinta blanca o negra también está dos veces. | Lectura directa | Reutilizar `pane_view::rgb` (o moverla a `theme.rs`). Valorar `vow_appearance::backdrop::ink_on` | sí; la equivalencia con `ink_on`, no |
| RUL-12 | baja | reglas 2 y 3 | `pick.rs:1`, `key_bindings_pane.rs:574,589` | `pick::matching<T>` es un genérico con una sola llamada (`terminal.rs:2086`, T = `Choice`). El mismo filtro (minúsculas + `contains`) se repite dos veces más en `MenuItemChooser`. | `grep` | Usar una sola función de filtro (tercera repetición); quitar el genérico si sigue habiendo un solo tipo | sí |
| RUL-13 | baja | regla 3 | `buffer_edit.rs:168` | `fold_all_placeholder` es un envoltorio puro de `placeholder` y tiene una sola llamada. | `wrappers.txt` (30 funciones de una línea y un solo uso; el resto son accesores bajo mutex o conversiones, aceptables) | Llamar a `placeholder` directamente o hacerla `pub` | sí |
| RUL-15 | baja | tokens | 14 sitios; `composer_view.rs:236`, `note_view.rs:284`, `session_note.rs:378`, `tab_bar.rs:200,330,332,444`, `terminal.rs:1071` | Los trazos y separadores de 1 px son literales (`set_stroke_width(1.0)`, `Size::px(1.0)`). | `literales-diseno.txt` | Un token `HAIRLINE` (o el de `vow_design` si existe) | sí |
| RUL-16 | baja | tokens / regla 3 | `indicators.rs:25-34`, `note_view.rs:17` | La app trae su propia geometría de iconos en SVG (rodeo de BK-024), fuera de tokens y del kit. | Lectura directa | Llevar los símbolos al kit (ADR 0006 punto 5) y borrarlos de aquí | sí |
| RUL-17 | baja | tokens | `terminal.rs:101-103,2739`, `io.rs:68`, `session.rs:120`, `triggers.rs:12` | Las duraciones (1 s, 5 s, 500 ms ×3) y `DIVIDER_STEP` 0.05 son literales locales. `terminal.rs:103` `UNDO_CLOSE` (un `Duration`) choca de nombre con `menu::UNDO_CLOSE` (una `Action`). | `grep` | Reunirlas en `tokens.rs` y cambiar el nombre de la constante | sí |
| RUL-19 | baja | i18n | `config.rs:20,369`, `itermcolors.rs:42`, `smart_selection.rs:50-89`, `key_bindings.rs:431,511` | Hay nombres por defecto visibles en inglés: el perfil «Default», el esquema «Imported», las notas de las reglas de selección inteligente (se ven en `preferences.rs:1087`) y la tecla «Space». | Lectura directa | Traducir solo al mostrar (son datos persistidos) o por `tr!` al crearlos | sí; que se vean en pantalla, no |
| RUL-20 | baja | i18n | `en/es curseterm.ftl:20,21,22,30` | Las claves `pane-text`, `pane-colors`, `section-font` y `section-scheme` están definidas, pero nadie las usa. | `i18n.py`; `grep -rn` sin resultados | Borrarlas | sí |
| RUL-21 | baja | i18n | `es/curseterm.ftl:364,374,437` | «Triggers» y «trigger» siguen en inglés. Los otros 17 valores idénticos (General, Normal, Terminal, PID, Zoom, Cursor, Shell…) son aceptables. | `i18n-same.txt` | «Disparadores» o glosario explícito | sí |
| RUL-23 | baja | docs (regla 3) | `docs/decisiones/0002-…md:31`, `docs/progreso.md:7-8` | El ADR 0002 (aceptado) remite al lienzo de `barrier-kit-next` y no dice que lo sustituyen en parte los ADR 0005 y 0006. `progreso.md` sigue pidiendo `check` «en los dos modos del kit (ADR 0001)», aunque el ADR 0001 está sustituido. | Lectura directa | Marcar el ADR 0002 como sustituido en parte y actualizar `progreso.md:7-8` | sí |
| RUL-24 | baja | dependencias | `Cargo.toml:21`, `docs/decisiones/0002-…md:46-47` | `alacritty_terminal = "0.26"` no está fijado con `=`, aunque el ADR dice que su API no es estable y habla de un «rev» que en crates.io no existe. | `Cargo.lock`: 0.26.0 | `=0.26.0` y corregir «rev» por «versión» | sí |
| RUL-25 | baja | dependencias | `Cargo.lock` | 22 paquetes tienen varias versiones, todas transitivas (`toml` 0.5/0.8/1.2 por `find-crate` y `freya-skia-bindings`; `base64` 0.22/0.23 y `quick-xml` por `plist`; `thiserror` 1/2 por `i18n-embed`; `objc2*` y `windows-sys` de otras plataformas). CurseTerm no puede unificarlas sin tocar sus dependencias. No hay `deny.toml`. | Script sobre `Cargo.lock`; la lista está en la salida de la sesión | Añadir `cargo-deny` (licencias y duplicados) al `check`. No detectaría RUL-01 | sí |

## 3. Causa de los fallos conocidos de barrierkit

Ejecutado con `nix develop`, con y sin `LANG=es_ES.UTF-8`, y con
`--test-threads=1`.

| Test | Causa | ¿Verificado? |
|---|---|---|
| `scripts/check-design-tokens.sh` | 113 líneas: colores de muestra en Marea (85), Ondina (23) y Tengen App Show (9); `views/floating_pane.rs:188` y `platform/doc_shots.rs:232` con negro literal; `vow-appearance/src/backdrop.rs:91` es un falso positivo (bytes de píxel, no un color de diseño) | Sí, ejecutado |
| `tests/kit`: `checkbox_click_toggles…`, `switch_click_toggles…`, `tab_reaches_each_control…` | No es el entorno: pánico real de LAB-01 (`controls.rs:692`, `:1313`) | Sí, ejecutado |
| `tests/kit`: `controls_match_their_golden_images`, `keyboard_focus_ring…`, `menu_matches_its_golden_images`, `menu_renders_accents…` | Golden de píxeles que dependen de las fuentes del host (`fontsConf` incluye `/etc/fonts/fonts.conf`); fallan igual en español | Sí, ejecutado (la causa exacta de cada píxel, no) |
| `tests/kit`: `preferences_search_opens_the_result_chosen_with_the_arrows` | Sin contexto `Resolved`: lee el `appearance.domain` del host y pulsa en coordenadas fijas; sale «Apariencia» en vez de «Sonido» | Código; fallo ejecutado |
| `tests/kit`: `typing_digits_fills_each_date_part…` | Supone el orden `mdy`: pasa sin idioma y falla con `LANG=es_ES.UTF-8` (`dmy`) | Sí, ejecutado |
| `tests/sidebar`: 4 tests (`the_sidebar_toggle…`, `the_traffic_lights_stay…`, `an_overlaid_sidebar…`, `a_pushed_sidebar…`) | Buscan la etiqueta española «Mostrar u ocultar la barra lateral»; sin `LANG` el kit carga el inglés. Pasan con `LANG=es_ES.UTF-8` | Sí, ejecutado |
| `tests/sidebar`: `sidebar_section_chevron_changes_shape_when_toggled` | Solo falla en paralelo: pasa con `--test-threads=1` (carrera con `set_var("MALAYSIA_CONFIG_DIR")` en `with_config`) | Sí, ejecutado |
| `tests/sidebar`: `sidebar_section_toggles_with_enter_and_space` | Falla siempre, también en español y con un hilo: Tab + Intro no pliega la sección. Causa no aislada | Fallo ejecutado; causa no verificada |
| `marea/tests/media` | LAB-13: `location` se fija después de `parse::launch`; no falta el decodificador | Sí, ejecutado |

## 4. Causas raíz de BK-047 y BK-017

Salen de leer el código de freya 0.4.3 y del kit; no se han reproducido
con una animación en pantalla.

### Causa raíz BK-047 (las animaciones de `freya-animation` no reciben fotogramas)

#### Cómo Freya 0.4.3 programa un tick de animación

1. `use_animation*` registra un `Effect::create_sync_with_gen` (`fa/hook.rs:445`). Cuando su dependencia cambia, el efecto **no corre en línea**. Corre como *task*: `rx.notified().await` y después `callback(gen)` (`fc/lifecycle/effect.rs:31-41`). Ese callback llama a `UseAnimation::run` (`fa/hook.rs:453-456`).
2. `run()` hace `spawn(async { platform.send(RequestRedraw); loop { ticker.tick().await; platform.send(RequestRedraw); …advance… } })` (`fa/hook.rs:256-319`). `spawn` solo encola `Message::PollTask` en el canal del `Runner` (`fc/lifecycle/task.rs:40-57`).
3. `RenderingTicker::tick` espera a un `async_watch` (`fc/rendering_ticker.rs:22-24`). El waker de la task vuelve a encolar `PollTask`.
4. `Runner::handle_events` (`fc/runner.rs:565-627`) tiene tres comportamientos:
   - Devuelve `Ready` **en cuanto hay `dirty_scopes`**, sin hacer poll de las tasks pendientes (`:578`, `:612`).
   - Hace poll de `dirty_tasks` solo si no hay scopes sucios.
   - Si no, entra en `while let Some(msg) = self.receiver.next().await` (`:616-625`). Ese bucle **mueve `MarkScopeAsDirty`/`PollTask` a `dirty_scopes`/`dirty_tasks` y devuelve `Pending`**.
5. `sync_and_update` → `handle_events_immediately` tampoco hace poll de tasks si hay scopes sucios (`fc/runner.rs:643-645`).
6. El runner oficial (freya-winit) cubre los dos casos:
   - Tras cada poll llama **siempre** a `sync_and_update` (`fw/renderer.rs:480`).
   - Si el poll dio `Ready`, se vuelve a encolar `PollRunner` (`fw/renderer.rs:460-468`).
   - `RequestRedraw` → `request_redraw` (`:527-529`), y el ticker se envía tras pintar (`:879`).

#### Qué hace el kit

- `AppInstance::poll` (`app.rs:236-267`) hace poll de `handle_events()` una vez. Devuelve `true` solo si el futuro dio `Ready` o llegaron eventos de entrada.
- `Runtime::poll_all` llama a `update()` (→ `sync_and_update`) **solo si `poll()` devolvió `true`** (`runtime.rs:442`). `settle_sized_reactions` hace lo mismo (`app.rs:357`).
- Tras un `Ready` el kit no vuelve a hacer poll: no hay equivalente del `PollRunner` re-encolado.
- Sí envía el ticker tras `present` (`surface.rs:242`). `RequestRedraw` sí produce un frame aunque el árbol no cambie (`runtime.rs:175-178` → `redraw_all` → `surface.rs:358-366`). Por eso la API propuesta en `barrierkit-gaps.md` («que `RequestRedraw` produzca un fotograma») **no es la causa**.

#### Cadena que se cuelga (cambio de `State<bool>` sin mover el ratón)

1. Se escribe el `State` → `PollTask(efecto)`.
2. Ping → `poll()`. El efecto corre dentro de `handle_events`:
   - `animated_value.set` → `MarkScopeAsDirty(componente)`.
   - `spawn` → `PollTask(animación)`.
3. Esos mensajes los drena el bucle interno (`fc/runner.rs:616-625`) a `dirty_scopes`/`dirty_tasks`, y el futuro devuelve `Pending`.
4. `poll()` devuelve `false` y no se llama a `update()` (`runtime.rs:442`). No queda ningún ping pendiente, porque el waker del canal ya se consumió antes de esos envíos. Nadie hace poll de la task de animación, así que nadie manda `RequestRedraw`.
5. Todo queda congelado hasta el siguiente evento.
6. Cuando llega, `prev_frame.elapsed()` acumula todo el tiempo parado (`fa/hook.rs:280-283`) y la animación **salta al final**: es justo el síntoma de BK-047.

La misma pérdida se repite cuando un `poll` da `Ready` temprano: las tasks recién despertadas (tick tras `ticker_sender.send`) quedan en `dirty_tasks`. Si ese `update()` no produce mutaciones (p. ej. el primer paso con `index≈0`), no hay frame, ni `settle`, ni poll.

#### Arreglo mínimo (igual que freya-winit)

```rust
// app.rs — poll(): un Ready significa que hay más trabajo; volver a hacer poll
if ready { self.waker.wake_by_ref(); }

// runtime.rs:442 — sincronizar siempre, como fw/renderer.rs:480
window.app.poll();
if window.app.update() { window.needs_redraw = true; }

// app.rs:357 — igual en settle_sized_reactions
self.poll();
if !self.update() { return; }
```

- `update()` sin mutaciones es barato: `sync_and_update` con el conjunto vacío.
- Con esto `accessibility_pending()` deja de ser necesario en `poll_all`, porque `update()` ya consume las acciones de AT-SPI.
- El propio kit también lo sufre: `animation.rs:21` (`use_switch_progress`) usa `use_animation_transition`.

---

### Causa raíz BK-017 (handlers de menú fuera del contexto de Freya)

#### Contexto requerido

- Toda la API de contexto de Freya pasa por `CurrentContext::with`, que hace `expect("Your trying to access Freya's current context outside of it…")` (`fc/current_context.rs:55-57`).
- Ese contexto solo existe dentro de `CurrentContext::run`/`run_with_reactive`, es decir, en `Runner::{handle_event, handle_events, sync_and_update, run_in}`.
- De ahí dependen `consume_root_context`, **incluido `try_consume_root_context`** (`fc/lifecycle/context.rs:56-77`), `spawn`, `Clipboard::set` (`freya-clipboard/src/clipboard.rs:33,47`), `Timers::get`, `WindowControls::get`, `Popups::get` y `Platform::get`.

#### Dónde invoca el kit código de la app sin entrar en el runner

1. **Atajo de teclado:** `KeyboardHandler::press_key` → `Runtime::key_event` → `self.surfaces[idx].app.menu_actions.dispatch(action_id)` (`input/seat.rs:458-460`). Es un callback de dispatch de Wayland dentro de calloop, sin `run_in`.
2. **Menú global (dbusmenu) y StatusNotifierItem:**
   - El hilo `barrier-dbusmenu` (`app_menu/server.rs:45-48`) recibe `Event` → `handle.push_action` + `wake.wake()`.
   - Eso llega a la fuente del *wake ping* (`runtime.rs:235-244`) → `process_menu_actions` → `surface.app.menu_actions.dispatch(&id)` (`app_menu_export.rs:40-51`).
   - El hilo correcto, pero fuera de contexto.
3. **`org.freedesktop.Application.ActivateAction`:** el callback de `Timers::wakeup` (`app/application.rs:36-53`) llama a `menu_actions.dispatch` desde la fuente wake (`runtime.rs:236-239`).
4. Igual de expuestos, aunque no son menús:
   - Callbacks de `Timers::after` (`timer.rs:72-79`).
   - Callbacks de `Timers::wakeup` (`runtime.rs:236-239`).
   - `PopupSpec::on_dismissed` (`popup.rs:495-499`).

Ninguno de estos puntos pasa por `Runner::run_in`. Hasta el `poll()` posterior, el `CurrentContext` del hilo está vacío (`fc/current_context.rs:45-52` lo limpia al salir).

#### Arreglo propuesto

1. Añadir `AppInstance::run_in(&self, f) { self.runner.run_in(f) }` (`fc/runner.rs:758-769`, contexto del scope ROOT) y envolver con él:
   - `seat.rs:459`;
   - `app_menu_export.rs:49`;
   - el `on_dismissed` de `popup.rs:498`, con el runner del `owner`.
2. Dar a `Timers` un `owner: ObjectId`, como ya tiene `Popups` (`app.rs:86-97`). Guardar `(owner, callback)` en `TimerQueue`/`WakeQueue`. Al disparar, buscar la superficie y ejecutar `surface.app.run_in(callback)`. Si la superficie ya no existe, descartar el callback (y en `wakeup`, quitarlo).
3. El punto 2 también cubre `application.rs:51` y cierra BK-032 sin `use_alive`.
4. Advertencia: `CurrentContext::run` no restaura un contexto anterior (`fc/current_context.rs:45-52`). No hay que anidar `run_in`.

---

## 5. Estado real de BK-001…BK-047

Contra barrierkit `21c532d`. Sale de leer el kit, CurseTerm y
`barrierkit-gaps.md`; el estado de BK-047 no se ha ejecutado.

| BK | Estado en el doc | Estado real (21c532d) | ¿Hace falta el rodeo? | ¿Doc correcto? | Evidencia |
|---|---|---|---|---|---|
| 001 | En barrierkit: `open_window` | **Resuelto a medias**. Abrir ventanas en el mismo proceso: sí (`d08deb7`). Listarlas: no hay `Windows::list`. | En parte. Se queda la lista propia `OPEN` (`windows.rs:20-23,164-170`). El registro por pid con `SIGTERM` (`windows.rs:82-91,203-230`) solo sirve ya para otros procesos | **No**: el cuerpo describe «un proceso por ventana» y el parche 0008 borrado | kit `platform/window.rs:292`; CT `windows.rs:186-192` |
| 002 | En barrierkit: `set_title` | Resuelto (`ba76e23`) | No (`terminal.rs:5253-5258`) | Tabla sí; el cuerpo cita el parche 0001 y la feature `kit-parcheado`, que ya no existen. La reproducción («una sola llamada») ya no vale | `window.rs:273` |
| 003 | En barrierkit: `NamedKey`, atajo consumido | Resuelto (`15c8926`) | No; los atajos con Super siguen por ADR 0003 (`keys.rs:18`) | Tabla sí; cuerpo con el parche y líneas viejas (`seat.rs:424-468` → `439-488`) | `app/action.rs:9-14`; `seat.rs:480-486` (no reenvía la tecla) |
| 004 | En barrierkit: `ime_preedit` | Resuelto (`643bb46`). **Duplicado de BK-033** | No (`terminal.rs:5225-5243`) | **No**: el cuerpo dice «pendiente»; `grep text_input` ya da resultados | `platform/ime.rs:118`; `window.rs:285` |
| 005 | En barrierkit: `primary_selection()` | Resuelto (`f11792b`). **Duplicado de BK-034** | No (`pane_view.rs:535-550`) | **No**: el cuerpo dice «pendiente» y cita la ADR 0047 | `window.rs:281`; `platform/primary.rs` |
| 006 | Vista propia `tab_bar.rs`; el kit tiene `WindowTabs` | Abierto para lo de iTerm2: `WindowTabs` no tiene insignias, grupos, estilos PSM, menú de pestaña ni `leading` | Sí (ADR 0006 §3) | **No**: el cuerpo cita `barrier_kit_next::views::tab_bar` y `crates/barrier-kit-next/...` | kit `views/window_tabs.rs:12-45`; CT `tab_bar.rs:21-140` |
| 007 | En barrierkit: `split_panes` | Resuelto (`dda9e12`) | No (`terminal.rs:1454`) | Tabla sí; el cuerpo cita `barrier_kit_next` | `views/split_panes.rs:8-48` |
| 008 | `vow_design::MONO`; `TERMINAL` en `tokens.rs` | Resuelto `MONO` (`dda9e12`); `TERMINAL` es de CurseTerm | No | El cuerpo cita `barrier_kit_next::tokens::MONO` | `vow-design/tengen/tokens.json:285-289` |
| 009 | En barrierkit: `canvas` | Resuelto (`dda9e12`); la firma `canvas(revision: u64, painter)` coincide | No (6 usos: `pane_view.rs:162`, `note_view.rs:106`…) | Tabla sí; el cuerpo cita `barrier_kit_next` y la reproducción («canvas no aparece en docs») ya no vale | `canvas.rs:14-35`; `docs/barrierkit/05-controles.md:457-466` |
| 010 | Hooks de `app_menu` y `set_menu_items` | Resuelto (`dda9e12`, `cc26672`). Siguen estáticos `Action::new(id, fn()->&'static str)` y `use_menu_label(&'static str)` | En parte: `use_profiles_menu` copia el `use_menu_update` privado del kit (INT-11) | El cuerpo cita `barrier_kit_next` y el parche 0004 | `app_menu.rs:126,259-307`; CT `terminal.rs:1752-1763` |
| 011 | — | Parcial: el kit corrigió 3 de 9 puntos (`MenuHandle` documentado, atajo consumido, menú en D-Bus sin appmenu) | — | **No**: la lista mezcla puntos corregidos y abiertos; «hay tres apps» (hay 4: marea, ondina, tengen-app-show y trazo) | `11-app.md:31,51-60`; abiertos: `04:33`, `01-que-es.md:30`, `07-titlebars.md:34` frente a `traffic.rs:124-129`, `11-app.md:30` |
| 012 | Rodeo: `forget_saved_size` | Abierto | Sí (`windows.rs:102,232-235`) | Sí en el fondo; líneas viejas (`:333/:373` → `:798-818/:475-479`) | `window.rs:398-400,475-479,795-818` |
| 013 | Ninguno | Abierto | — | Sí | `window.rs:693-695` (`request_close` → `close_window` sin aviso) |
| 014 | Un solo canal por ventana | Abierto | Sí | **No**: hay **dos** `wakeup` por ventana de terminal (`terminal.rs:3109,3463`), y con `open_window` el coste ya no es «ninguno apreciable» (INT-03) | `platform/timer.rs:52-57`; `runtime.rs:231-236` |
| 015 | Se acepta | Abierto (aceptado) | — | Sí (`app/defaults.rs:65`) | idem |
| 016 | Atajo por texto `}` / `{` | Abierto | Sí (`menu.rs:554,634,645-646`) | Sí | `menu/shortcut.rs:113-121` compara el glifo con `key.to_string()` |
| 017 | Rodeo: `Handles` tomados en `render` | **Abierto**: `dispatch` corre fuera del contexto de Freya | Sí (`terminal.rs:113-160,443-487,570-577`) | Sí; línea vieja (`seat.rs:454` → `459`) | `seat.rs:458-460`; `platform/app_menu_export.rs:40-51`; `app/action.rs:122-128`; `app/application.rs:36-53` (el kit hace lo mismo) |
| 018 | En barrierkit | Resuelto (`82208dc`) | No | Tabla sí; el cuerpo cita el parche 0003 | `window.rs:510-526` (registra siempre) |
| 019 | Ninguno | Abierto: pedir un token solo se puede con la feature `shell`. `21c532d` lo pide por dentro, `pub(crate)` | — | Sí; línea vieja (`window.rs:207` → `261`) | `shell/activation.rs:30,54`; `lib.rs:11`; `platform/activation.rs:43-57` |
| 020 | `intern` | Abierto | Sí (`preferences.rs:51-64`) | Sí | `popup_button.rs:36`; `views/form.rs:29`; `choices.rs:82` |
| 021 | Grupos de una opción | Abierto | Sí (`preferences.rs:600`) | Sí | `choices.rs:44-48` |
| 022 | En barrierkit | Resuelto (`482a7de`) | No | El cuerpo cita el parche 0005 | `runtime.rs:274-278` |
| 023 | `portal::open_files` | Resuelto (`768c336`, **anterior** a `ba76e23`) | No (`preferences.rs:1565-1571`) | **No**: el cuerpo describe el campo de ruta y `--import-colors`, que ya no existen; el rango de commits es falso | `portal.rs:15` |
| 024 | Símbolos sustitutos | Abierto: 36 símbolos sin campana, papelera ni engranaje; `path` privado | Sí (`note_view.rs:290`, `indicators.rs`) | Parcial: dice «31 símbolos» (hay 36) y cita el `Badge` de `views::tab_bar` del bloque | `symbol.rs` (enum), `:50` |
| 025 | `curseterm --hotkey` | Abierto | Sí (`windows.rs:60-81`) | Sí | `GlobalShortcuts` no aparece en `crates/barrier-kit/src` |
| 026 | Campana visual + `notifications` | Parcial: `notifications` ya está en el kit; faltan el pitido y la petición de atención | Sí, la campana visual | El cuerpo cita `barrier-kit-next` | `notifications.rs:27,54` |
| 027 | Foco seguido y cierre diferido | Abierto. El kit tiene `text_field_focused()`, pero es `pub(crate)` | Sí (`terminal.rs:2593,2624,4041`) | Sí | `platform/app.rs:211`; `views/feedback.rs:124-128` |
| 028 | Ajustes a 980 px | Abierto | Sí | Sí; línea vieja (`769-791` → `789-825`) | `controls.rs:814-823` (`label` sin `max_lines`) |
| 029 | `TextField::auto_focus` | Resuelto (`f1ac264`) | No (`terminal.rs:2289`) | El cuerpo cita el parche 0006 y `./dev.sh kit-tal-cual`, que no existe | `controls.rs:860` |
| 030 | Modificadores seguidos por teclado | Abierto | Sí | Sí | No hay consulta pública de modificadores (`platform/input/mod.rs:4` es interno) |
| 031 | `set_background_alpha` | Resuelto (`f207665`) | No (`terminal.rs:5245-5251`) | El cuerpo cita el parche 0007 | `window.rs:296` |
| 032 | En barrierkit: `use_alive` | **Mitigado, no resuelto**: `Timers` sigue siendo global y los `wakeup` no se dan de baja. `use_alive` es el rodeo pasado al kit (`5f0fc43`) | Sí (`terminal.rs:288-296,3110,3464`; `tab_bar.rs:364-381`) | **No**: «En barrierkit» exagera y el commit cae fuera del rango | `animation.rs:10-17`; `timer.rs:47-57` |
| 033 | En barrierkit | Resuelto; **duplicado de 004** | No | El cuerpo cita el parche 0009 | `platform/ime.rs` |
| 034 | En barrierkit | Resuelto; **duplicado de 005** | No | El cuerpo cita el parche 0010 | `platform/primary.rs` |
| 035 | `copy_rich` | Resuelto (`8dee7be`) | No (`terminal.rs:1789`) | El cuerpo cita el parche 0011; línea vieja | `window.rs:277`; `clipboard.rs:27,259` |
| 036 | `views::text_view` | Resuelto (`dda9e12`) | No (`composer_view.rs:134`, `note_view.rs:140`, `session_note.rs`) | El cuerpo cita `barrier-kit-next` | `views/text_view.rs:18-92` |
| 037 | Hoja propia | **Parcial**: `Alert::suppression(State<bool>)` existe (`410fa6d`); siguen 2 botones | Sí, la hoja (`terminal.rs:1889-1891`) | **No**: dice «no se puede silenciar» | `views/feedback.rs:59-99` |
| 038 | `PopUpButton` al lado | Abierto | Sí | Sí; línea vieja (`controls.rs:850` → `947`) | `controls.rs:947-` |
| 039 | Panel dentro de la ventana | **Hay API**: `OutlineView` (`768c336`), `open_utility_window` (`dfc9f68`, ya en `4e23c81`) y `PanelBehavior` (`21c532d`) | No haría falta (INT-06) | **No**: «no hay ninguno» es falso | `views/tree.rs:115-135`; `window.rs:416-420`; `06-ventanas.md:54-94` |
| 040 | Grupos y filtro | Abierto: `ComboBox` no filtra. No verificado si los menús ya se desplazan | Sí (`key_bindings_pane.rs:557-560`) | Probablemente; no verificado | `combo_box.rs:17-40`; `menu/mod.rs:333` |
| 041 | En barrierkit | Resuelto (`13313e6`) | No | El cuerpo cita el parche 0012 | `window.rs:731` |
| 042 | `MenuButton` | Resuelto (`768c336`, anterior al rango) | No (`key_bindings_pane.rs:492`) | **No**: el cuerpo describe la `key` que sube, ya retirada | `buttons.rs:15-22` |
| 043 | Sin rodeo | Abierto | — | Sí | `TextField` no tiene `select_on_focus` |
| 044 | `views::floating_pane` | Resuelto (`dda9e12`) | No (`terminal.rs:1237`) | El cuerpo cita `barrier_kit_next` | `views/floating_pane.rs:68-79` |
| 045 | `secrets` | Resuelto (`dda9e12`) | No (`password_manager.rs:8`) | El cuerpo cita `barrier_kit_next` | `secrets.rs:43-66` |
| 046 | `Resolved::contrast` | Resuelto (`4e23c81`) | No (`terminal.rs:623`) | **No**: el cuerpo cita `theme::use_system_contrast`, que no existe | `vow-appearance/src/resolve.rs:67,283` |
| 047 | Fundido con `Timers` | Abierto según el doc. Sin cambios en `surface.rs` desde `4e23c81`. No verificado | Sí (`tab_bar.rs:356-386`, `hint_view.rs:125-170`) | No verificable sin ejecutar | `surface.rs:242`; `runtime.rs:175-178,440-462` |

Resumen del doc: el párrafo inicial cita como abiertos solo BK-013, 017, 019, 025
y 047. Siguen abiertos también 006, 012, 014, 016, 020, 021, 024, 027, 028, 030,
032, 038, 040 y 043, y 026 y 037 están a medias.

## 6. Paridad con iTerm2

### Funciones de iTerm2 que faltan y no constan en `progreso.md`

En `progreso.md` y `estudio-previo.md` no aparecen como pendientes ni como diferencias. Algunas salen en el inventario de `estudio-previo.md` §4, pero sin estado.

Emulación y secuencias:

- **Sixel** (`DCS … q`): `VT100/VT100Terminal.m:3358`. iTerm2 además lo anuncia en DA1 (`VT100/VT100Output.m:956-960`).
- **DECRQSS** (`DCS $ q`): `VT100/VT100Terminal.m:3366`.
- **XTGETTCAP** (`DCS + q`): `VT100/VT100Terminal.m:3354`.
- **XTVERSION** (`CSI > q`): `VT100/VT100Terminal.m:2145-2149` (PAR-20).
- **XTPUSHCOLORS/XTPOPCOLORS/XTREPORTCOLORS** y **XTPUSHSGR/XTPOPSGR**: `VT100/VT100Terminal.m:2400-2410, 2847-2853`.
- **OSC 1** (título de pestaña): `VT100/VT100Terminal.m:2915` (PAR-19).
- **OSC 6** (color de pestaña, extensión propia): `VT100/VT100Terminal.m:3203-3205`, `VT100/VT100XtermParser.m:280`.
- **OSC 9;4** (barra de progreso de ConEmu): `VT100Screen/VT100ScreenMutableState+TerminalDelegate.m:1763-1800`. CurseTerm solo evita avisar.
- **OSC 21337** (estado de pestaña) y **OSC 7501** (estado del programa): `VT100/VT100XtermParser.m:309-310`, `VT100/VT100Terminal.m:3215-3221`.
- **OSC 72** (arrastrar y soltar de kitty): `VT100/VT100Terminal.m:2807`.
- **OSC 1337 `MultipartFile`/`FilePart`/`FileEnd`** y **descarga de archivos** (`File=` con `inline=0`): `VT100/VT100Terminal.m:4472-4475` (PAR-07).
- El resto de subórdenes de **OSC 1337**: `VT100/VT100Terminal.m:4429-4609` (PAR-21).
- **Consulta de colores OSC 4/10/11/12** (PAR-04).
- **Permiso para OSC 52** (PAR-02).
- **Aviso antes de que una app borre el historial** (PAR-09).

Selección:

- **Selección rectangular con el ratón** (⌘⌥) y **Mayús+clic para ampliar**: `TerminalView/PTYMouseHandler.m:29, 372-384`.
- **Preferencia «Characters considered part of a word»** y su modo de expresión regular: `Settings/GeneralPreferencesViewController.m:2667-2700`.

Menús. En `MainMenu/Base.lproj/MainMenu.xib`, sin rastro en `progreso.md`:

- Shell:
  - New Tab Next to Current Tab (⌥⌘T).
  - Split Horizontally… y Split Vertically… con selector de perfil (⌥⇧⌘H/V).
  - Workgroups ▸.
  - Broadcast Input ▸ Show Background Pattern Indicator, Current Session is Broadcast Source y Toggle Buffer Input.
  - ssh ▸ (Disconnect, Download Files…, Remote host can control iTerm2).
  - Print ▸ (Page Setup…, Screen, Selection, Buffer).
- Edit:
  - Undo, Redo y Cut.
  - Paste Special ▸ Paste Faster/Slower, Paste Slowly Faster/Slower y Prompt to Convert Tabs to Spaces when Pasting.
  - Engage Artificial Intelligence y Explain Output with AI.
- View:
  - Restore Text and Session Size (⌥⌘0) y Size Changes Update Profile.
  - Disable Transparency for Active Window.
  - Toolbelt ▸ Set Default Width.
  - Show Clippings y Show Inline Chat.
- Session:
  - Move Session to Floating Pane.
  - Log ▸ Import Recording y Export Recording.
  - Make Screenshot….
  - Terminal State ▸ Literal Controls y Emulation Level ▸ VT100–VT500.
  - Open AI Chat….
- Window:
  - Load Arrangement from File… y Change Profile in Arrangement….
  - Previous Peer y Next Peer (⌥⌘[ y ⌥⌘]).
  - Resize Split Pane ▸ (Decrease/Increase Height…).
  - Select Next Floating Pane y Lock Split Pane Width.
  - Lock Size, Lock Location y Notify on Status Change.
  - Arrange Windows ▸ Arrange Windows Horizontally.
  - Resize Window ▸.
  - Pin Hotkey Window (⇧⌘-).
  - Bring All To Front.
  - AI Chats y Cockpit.
- iTerm2:
  - Secure Keyboard Entry.
  - Install Shell Integration (CurseTerm no trae guiones de integración; `share/` solo tiene el `.desktop`).
  - Make iTerm2 Default Term.
  - Show Tip of the Day, Suppressed Alerts… y Check for Updates.
- Help: GPU Renderer Availability.
- Atajos que faltan en entradas que sí existen:
  - Run Coprocess (⌥⌘R) y Stop Coprocess (⌥⇧⌘R).
  - Paste Selection (⇧⌘V).
  - Open Selection (⌥⌘O).
  - Move Tab Left y Move Tab Right (⌥⇧⌘[ y ⌥⇧⌘]).

### Comprobado y coincide

- DECSCUSR 0 vuelve al cursor del perfil: `alacritty` pone `cursor_style = None` y se usa `default_cursor_style` (`src/session.rs:276-287`); iTerm2 hace lo mismo con `terminalResetCursorTypeAndBlink` (`VT100/VT100Terminal.m:2424-2428`).
- Bordes suaves (fase 71): los cuatro divisores y la racha de 8 coinciden (`src/session.rs:1652-1655` frente a `ContentAnalysis/iTermTextExtractor.m:29, 2264-2306`).
- Automatic Profile Switching: el análisis de reglas sigue a `iTermRule.m +ruleWithString:` (`src/profile_switching.rs:22-58`).
- Valores por omisión: 1000 líneas de historial, 80×25 y copiar al seleccionar (`src/config.rs:63, 383, 386-387` frente a `Settings/iTermProfilePreferences.m:1167-1168, 1189`, `Settings/iTermPreferences.m:748`); Paste Slowly 16 B / 0,125 s (`src/advanced.rs:37-38` frente a `Settings/iTermAdvancedSettingsModel.m:837-838`).
- El alfabeto y la longitud de Generate en el gestor de contraseñas coinciden con `+randomPassword` (`src/password_manager.rs:24-25, 75-91` frente a `PasswordManager/iTermPasswordManagerWindowController.m:553-570`), y el muestreo no tiene sesgo de módulo.
- Atajos que coinciden con `MainMenu.xib`: New Window/Tab with Current Profile, Close Terminal Window, Undo Close, Copy Mode, Select Output, Find Globally, Filter, Find URLs, Smart Select All, Pick Result, Set Named Mark, Fold, Clear to Selection o a la Last Mark, Clear Instant Replay, Zoom In on Selection, Jump to Selection, Show Composer, Maximize Pane, Select Pane, Move Divider, Next/Previous Tab, Next/Previous Pane, Save Current Window as Arrangement (⌥⇧⌘S), Password Manager, Bury Session y New Floating Pane.

## 7. Pruebas en el laboratorio (fase 8)

Sway 1.12 sin pantalla (`WLR_BACKENDS=headless`, 1600×1000), lavapipe
(Mesa 26.2.5), grim, `wtype -s`, wl-clipboard y un puntero virtual
persistente escrito para la ocasión (`zwlr_virtual_pointer_v1`, órdenes por
una FIFO). Binario: el paquete de `nix build .#curseterm`, con
`LANG=es_ES.UTF-8` y `MALAYSIA_CONFIG_DIR`/`MALAYSIA_DATA_DIR` vacíos.

| Prueba | Resultado | Hallazgo | Captura |
|---|---|---|---|
| Escribir y ejecutar `echo` | ✓ (sin Geist Mono, fuente proporcional) | LAB-10 | `auditoria-escribir.png` |
| Pestaña nueva (Super+T) y dividir (Super+D) | ✓ | — | `auditoria-pestanas-paneles.png` |
| Ventana nueva (Super+N) | ✓ en el mismo proceso: dos toplevels con el mismo pid | — | — |
| Find (Super+F) | ✓ resalta y cuenta «1 de 1» | — | `auditoria-find.png` |
| Doble clic: selección primaria y «copiar al seleccionar» | ✓ `wl-paste --primary` y `wl-paste` dan «rojo» | — | — |
| Copy with Styles (Super+Alt+C) | ✓ `text/html` con `<pre>` y el color de la celda | — | — |
| Ajustes en el mismo proceso | ✓; la segmentada se parte en dos líneas (BK-028) | — | `auditoria-ajustes-general.png` |
| Pulsar un interruptor de Ajustes | ✗ la app aborta | LAB-01 | — |
| Arrastrar un divisor entre paneles estrechos | ✗ la app aborta | KIT-01, LAB-15 | `auditoria-paneles-estrechos.png` |
| `ESC]133;A` sin terminar | ✗ se oculta la salida hasta el siguiente BEL | SEC-05 | `auditoria-osc-sin-terminar.png` |
| `File=` con `height=4294967295` | ✗ ~55 s congelada y el historial vacío | SEC-01 | `auditoria-imagen-alta.png` |
| Coproceso que no lee stdin | ✗ congelada hasta matar el coproceso | LAB-02 | `auditoria-coproceso.png` |
| OSC 52 desde el PTY | ✗ escribe el portapapeles sin aviso | SEC-02 | — |
| Super+Q con `sleep 999 &` | ✗ sale sin preguntar y mata el trabajo | LAB-03 | — |
| Reset (Super+R) tras `seq 1 300` | ✗ el historial desaparece | PAR-01 | — |
| Acentos combinantes y emoji con tono | ✗ salen sin acento y con caja | LAB-04 | `auditoria-combinantes.png` |

`progreso.md` tiene 88 párrafos «No probado». Esta auditoría probó uno de
ellos: cambiar una opción desde Ajustes con el ratón, que las fases 26 a 29
solo probaron desde el `.domain` (`progreso.md:1253`, `:1285`, `:1311`,
`:1347`); el interruptor aborta la app (LAB-01). El resto sigue sin probar
por los motivos de la sección 8.

## 8. Lo que no se pudo verificar y por qué

### Entorno

- **IME:** no hay ayudante `vime` en este entorno ni un cliente de
  `zwp_input_method_v2`; no se probó la preedición.
- **Mover una sesión a otra ventana y el menú global:** sin bus de sesión
  D-Bus en el laboratorio (`barrier-dbusmenu: no session bus`), el menú solo
  es local y no se puede accionar desde fuera; no se probó.
- **BK-047 en pantalla:** la causa raíz es de lectura; no se grabó una
  animación.
- **Pantalla real de 60 Hz, GPU real y Malaysia:** el laboratorio es sway
  con lavapipe; no se ejecutaron `./dev.sh demo` ni `lab` de Malaysia.
- **dotfiles:** evaluar `keel-x86_64-linux` exige compilar Malaysia entero
  durante la evaluación (IFD de `system.replaceDependencies` en
  `nix/malaysia.nix`), y sus dependencias git de Cargo chocan con la CA del
  proxy en el sandbox; ver la línea base.
- **Disco:** el espacio de la sesión se agotó a mitad de la fase 0 (caché
  git de Nix de 6,5 GB); se borró la caché y los `target/*/incremental` y se
  repitieron los builds.

### Revisiones de solo lectura

#### Seguridad

- **Bombas de descompresión en Skia:** `Image::from_encoded` decodifica en diferido. No se ha comprobado qué hace freya-engine/Skia con un PNG de 100 000 × 100 000 al dibujarlo: si falla limpio o aborta.
- **ssh y el modo crudo (SEC-04 a):** que el pty local quede sin ECHO dentro de `ssh` es el comportamiento conocido de `ssh -t`, pero no se ha ejecutado aquí.
- **`xdg-open` y ficheros `.desktop` o ejecutables:** no se ha comprobado si GNOME y KDE ejecutan un `file:///…/x.desktop` abierto desde OSC 8 o desde Semantic History, o si solo lo abren en un editor.
- **Marcado en el cuerpo de las notificaciones (SEC-15):** depende del servidor (GNOME Shell filtra casi todo; KDE acepta `<a>` y `<img>`). No se ha probado.
- **OSC 7 en iTerm2 (SEC-14):** no se ha localizado en `/home/user/iterm2` el sitio donde iTerm2 decide si un `currentDir` remoto vale para las pestañas locales. La divergencia es probable, pero no está confirmada.
- **Tiempo real del bloqueo de SEC-01:** depende del coste de `Term::linefeed` con el historial lleno. No se ha medido porque no se puede ejecutar `cargo`.
- **Caché de texto de freya con la contraseña revelada (SEC-10.1):** no se ha seguido hasta el layout de Skia.
- **Destino de `tracing`:** `barrier_kit::logging::init_tracing` no muestra ninguna escritura a fichero en lo leído (`logging.rs:4-16`). Se supone que va a stderr.

#### Paridad con iTerm2

- Si una línea anclada caducada tras ED 3 o RIS (PAR-09) puede indexar fuera de la rejilla y provocar un pánico. Revisé `select_output` (`src/session.rs:1076-1087`), pero no todos los usos.
- Qué entrada gana en el kit cuando dos acciones comparten Super+Alt+A (PAR-12).
- El límite que aplica iTerm2 al alto de una imagen en línea (PAR-16).
- `line_feed_counting_lost_lines` (`src/io.rs:558-577`) con una región de desplazamiento cuya fila superior está en blanco e igual a la última del historial. Podría contar una línea perdida que no lo es; no lo comprobé.
- La paridad completa del protocolo de teclado de kitty, modifyOtherKeys y CSI u (fase 56) frente a `iTermModifyOtherKeysMapper*.m`. Solo comprobé el orden de prioridad en `src/key_report.rs:96-126`.
- DECRQM: alacritty responde a los modos privados conocidos (`alacritty: term/mod.rs:2046-2094`). No lo comparé modo a modo con `VT100/VT100Terminal.m:2724-2740, 5986`.
- Disposiciones, archivos de sesión, gestor de contraseñas, pestañas, paneles flotantes y grupos: solo revisé los atajos y los textos de «Difiere». No comparé su comportamiento con `PseudoTerminal.m`, `PTYSession.swift` ni `iTermPasswordManagerWindowController.m` más allá de lo citado.
- El valor efectivo de «Prompt before closing» en un iTerm2 recién instalado (PAR-24). `DefaultBookmark.plist` no trae la clave, así que vale `PROMPT_NEVER` según el código.

#### Integración y BK

- INT-01, 02, 14, 15 y 20: deducidos del código; no se ejecutó CurseTerm.
- BK-047: el doc dice abierto; no se puede comprobar sin ejecutar.
- BK-040: si los menús de `PopUpButton` ya se desplazan (`menu/mod.rs:333`
  tiene `menu_scroll`).
- `key_bindings_pane.rs:392-400`: let-chain en edición 2024 con
  `draft.peek()` en el segundo operando y `draft.write()` en el cuerpo.
  Según la referencia, el operando `&&` es su propio ámbito de temporales y no
  debería entrar en pánico. No comprobado.
- Escribir un `State` desde los callbacks de `wakeup` y del menú, fuera del
  contexto: funciona según `progreso.md`, pero el kit no lo garantiza.
- `windows.rs:232-235`: si vaciar `<app_id>.window` antes de cada
  `open_window` se adelanta a un `close_window` concurrente. En un solo hilo
  parece seguro.
- Que el `text_field_focused()` del kit reconozca la `TextView` (rol
  `MultilineTextInput`, `text_view.rs:248`): no afecta a INT-01, que falla
  igual por exigir Ctrl.
- Oportunidad no usada, sin gravedad: el kit entrega `on_file_drop`
  (`c8c56b3`, `05-controles.md:144-146`) y CurseTerm no acepta archivos
  soltados en el terminal (iTerm2 pega la ruta).

#### Plataforma del kit

- **Efecto real de PLT-05 en Mesa:** que `vkDestroySurfaceKHR` toque los proxies del display tras `wl_display_disconnect` (sale de leer `wsi_common_wayland.c` de memoria; no lo tengo localmente). El orden de drop sí está verificado.
- **PLT-09:** que el compositor (wlroots, mutter) emita `invalid_size` en ese commit concreto.
- **PLT-11:** si Malaysia, sway o mutter retienen `frame.done` a superficies ocultas mientras esperan el commit del configure.
- **PLT-25:** la semántica exacta de `ObjectServer::at` en la versión de zbus fijada (no descargada).
- **Comportamiento en ejecución de BK-047 tras el arreglo propuesto:** no compilé ni ejecuté, por instrucción.
- **Destrucción de una toplevel con `xdg_popup` hijos vivos (PLT-07):** si algún compositor lo trata como error de protocolo.
- **`doc_shots.rs` y `tests.rs`:** son `#[cfg(test)]` (`mod.rs:8,18`) y no los audité a fondo.
- **Directorio `shell/`** (feature `shell`): solo lo miré de pasada por `parent_popup`; queda fuera de este informe.

#### Resto del kit

- El conjunto exacto de los 8 fallos de `tests/kit` y de los 2 de `tests/sidebar` que no dependen del idioma. Hacen falta `cargo test -p barrier-kit --test kit -- --test-threads=1` con `LANG=C` y con `LANG=es_ES.UTF-8`, y comparar.
- La causa de `marea/tests/media`: depende de qué plugins de GStreamer haya y de si hay `/dev/dri`. Comprobar con `gst-inspect-1.0 avdec_h264` y `vah264dec` en el mismo entorno.
- Si `canvas` (canvas.rs:56-73) usa bien la escala: divide `visible_area()` entre `scale_factor` y vuelve a escalar el lienzo. Depende de si torin trabaja en píxeles físicos o lógicos en freya 0.4.3.
- La pérdida de deltas en el arrastre (KIT-09) y el busy-loop si `watch.drain()` devuelve un error persistente con `Mode::Level` (platform/runtime.rs:265-289; otro auditor).
- La secuencia de syscalls con la que home-manager reemplaza `appearance.domain` (`ln -sf` atómico o no), que decide si hace falta `IN_CREATE` (KIT-16).
- Las cifras de contraste de KIT-11 vienen de una réplica en Python con `colorsys` (HSL estándar). La implementación de HSL de vow-design (lib.rs:102-158) es equivalente, pero no la ejecuté en Rust.
- No revisé JSON de vow-design que nadie lea, más allá de los campos de los structs: no hay mapa inverso de rutas JSON a constantes sin compilar.

#### Reglas, i18n y docs

- No se compiló ni se pasó clippy: el código muerto que solo detecta `rustc` (campos o variantes que no se usan) se da por cubierto por `./dev.sh check` (`-D warnings`), pero no se ejecutó.
- RUL-01 solo se cotejó en 5 módulos. Hay otros candidatos, señalados por sus comentarios, sin comparar con iTerm2: `key_report.rs`, `note_view.rs` (bocadillo y esquinas), `paste_transform.rs`, `snippets.rs` (escapes), `buffer_edit.rs` y `tab_groups.rs` (port de algoritmo, revisado por encima). La copia local de iTerm2 está en HEAD 6c7024a, que puede no ser la versión que se estudió.
- RUL-19: no se comprobó en ejecución que «Default», «Imported» y las notas de las reglas se vean así en pantalla.
- RUL-11: no se comprobó si `vow_appearance::backdrop::ink_on` equivale a la elección por brillo HSP de iTerm2.
- `docs/estudio-previo.md` y `docs/progreso.md`: se comprobaron rutas, capturas y la paridad, pero no cada afirmación de comportamiento.
- Fuera del alcance de la fase, solo observado: varios fallos se quedan en `tracing::warn` y el usuario no ve ningún aviso (`terminal.rs:1822,2337,2354,2365,4308,4377`).
