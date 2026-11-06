# What BarrierKit lacked when building CurseTerm

BarrierKit version/commit used: `ba58ce01ef1f8e07dd915f783290b6ef068c037a` (`main`, 2026-11-05)

Summary: living document; state at the close of phase 26 (2026-11-06), tested on
headless sway with lavapipe (`progreso.en.md`). The most serious: the
`use_menu_action` handlers run outside Freya's context and abort the app if
they touch the clipboard or `WindowControls::get()` (BK-017); the menu is only
published on D-Bus with `org_kde_kwin_appmenu_manager`, contrary to what the
documentation says (BK-018); one window per process and no live title
(BK-001, BK-002); no activation token for apps (BK-019); no IME nor
primary selection; no global shortcuts for the hotkey window (BK-025) nor
bell or notifications (BK-026). The canvas, the tabs, the split panes, the
`MONO` token and the live menu state are solved as blocks; the title,
the shortcuts, the menu on D-Bus, the live top-level menus and the repaint
without focus, as tested patches.

States: **detected** (read in the code), **confirmed** (reproduced by
running CurseTerm), **solved in CurseTerm** (with a workaround).

Each gap says how it is solved (CurseTerm ADR 0001):

- **block**: component in `crates/barrier-kit-next`, with the module path
  it would have in `barrier-kit`; it moves over by moving the file;
- **patch**: runner change in `kit-parches/NNNN-*.patch` against the pinned
  `rev`, tested with `./dev.sh kit-parches`;
- **workaround**: what CurseTerm does with the kit as is.

## Summary table

| ID | Area | Gap | Severity | Workaround in CurseTerm |
|---|---|---|---|---|
| BK-001 | window | One window per process | high | Patch 0008 tested; without it, one process per window |
| BK-002 | window | Fixed window title | medium | Patch 0001 tested; as is, only the `Titlebar` |
| BK-003 | input / global menu | Shortcuts not consumed and `ShortcutKey` limited | high | Patch 0002 (kit tests); as is, the encoder ignores Super |
| BK-004 | IME | No `zwp_text_input_v3` | medium | None |
| BK-005 | clipboard | No primary selection | medium | None |
| BK-006 | containers | No document tabs | medium | Tested block `views::tab_bar` |
| BK-007 | containers | No generic nestable splitter | medium | Tested block `views::split_panes` |
| BK-008 | tokens / text | No monospace face or terminal metrics in Tengen | low | Blocks `tokens::MONO` and `tokens::TERMINAL` |
| BK-009 | text / performance | No public canvas and paragraph cache turned off | medium | Block `canvas` |
| BK-010 | global menu | Static `Action` titles and no live menu replacement | medium | Tested block `app_menu`; patch 0004 tested for top-level menus |
| BK-011 | docs | Documentation that does not match the code | low | — |
| BK-012 | window / persistence | The saved size is one per `app_id` and another cannot be asked for | medium | Tested workaround: `forget_saved_size` |
| BK-013 | window | A close requested by the compositor cannot be intercepted | medium | None |
| BK-014 | platform | `Timers::wakeup` cannot be unregistered | low | Workaround: a single channel per window |
| BK-015 | persistence | Preferences always go to `~/.config/malaysia/` | low | Accepted |
| BK-016 | input / global menu | Shortcuts with Shift compare the produced character | medium | Workaround: text shortcut with `}` and `{` |
| BK-017 | global menu / platform | Menu handlers run outside Freya's context | **blocking** | Tested workaround: handles taken in `render` |
| BK-018 | global menu / docs | Without `org_kde_kwin_appmenu_manager` the menu is not published on D-Bus | high | Patch 0003 tested |
| BK-019 | window / platform | An app cannot request an xdg-activation token | medium | None; no window list in «Window» |
| BK-020 | controls | Pane, row and pop-up labels are `&'static str` | medium | Workaround: `intern` in `preferences.rs` |
| BK-021 | controls | `RadioGroup` is always horizontal | low | Workaround: one single-option group per row |
| BK-022 | platform / persistence | A `.domain` change does not repaint unfocused windows | high | Patch 0005 tested |
| BK-023 | app | No file chooser | medium | Workaround: path in a field and `--import-colors` |
| BK-024 | symbols | Missing symbols for a settings bar, the bell and the trash | low | `SlidersVertical`, `CircleHalf`, `ListBullet`, `SquareGrid` and `SpeakerWave` are used; the trash is drawn on the canvas |
| BK-025 | platform / input | No global shortcuts nor a way to hide and show the window | medium | Tested workaround: `curseterm --hotkey` |
| BK-026 | app / platform | No bell, attention request or notifications | medium | Tested workaround: visual bell; tested block `notifications` |
| BK-027 | controls / input | A view with global keyboard does not know whether a text field or an `Alert` keeps the key | low | Tested workaround: tracked focus and deferred `Alert` dismissal |
| BK-028 | controls | `SegmentedControl` labels wrap onto two lines | low | Workaround: Settings at 980 px |
| BK-029 | controls | `TextField` cannot take the focus when it opens | medium | Patch 0006 tested |
| BK-030 | input | Mouse events carry no modifiers | medium | Workaround: modifiers tracked through the keyboard |
| BK-031 | window / appearance | A window cannot ask for a translucent background | medium | Patch 0007 tested |
| BK-032 | platform | `Timers` callbacks outlive their window | high | Tested workaround: `alive` in `terminal.rs` |
| BK-033 | input | No input method (`zwp_text_input_v3`) | high | Patch 0009 tested |
| BK-034 | clipboard | No primary selection | medium | Patch 0010 tested |
| BK-035 | clipboard | The clipboard only offers plain text | low | Patch 0011 tested |
| BK-036 | controls | No multi-line text view | medium | Tested block `views::text_view` |
| BK-037 | controls / alerts | `Alert` has only two buttons and cannot be silenced | low | Tested workaround: own sheet with `sheet` |
| BK-038 | controls | `SearchField` has no options menu | low | Tested workaround: a `PopUpButton` beside it |
| BK-039 | controls / windows | No outline view nor floating panel | medium | Tested workaround: a panel inside the window |
| BK-040 | controls | `PopUpButton` neither scrolls nor searches | medium | Tested workaround: groups and a filter |

## Gaps

### BK-001 — One window per process
- **Area**: window
- **What CurseTerm needed**: «Shell › New Window» (⌘N) and the window list
  in «Window», like iTerm2, which opens several windows in one process.
- **What BarrierKit offers today**: `platform::run_window`
  (`crates/barrier-kit/src/platform/window.rs:346`) creates a window and blocks.
  `Runtime::create_window` (`:360`) already knows how to create more, but it is `pub(super)`.
- **What is missing**: opening and closing new windows from the app in the same
  runner, and listing them.
- **How CurseTerm solves it**: workaround, tested on sway: New Window
  opens a new process and Quit closes them all. Each window is a process (`crates/curseterm/src/windows.rs`): «New Window»,
  «About» and «Settings…» launch the executable with `--about` or `--settings`; each
  process registers by pid in `$XDG_RUNTIME_DIR/curseterm/windows/` and «Quit»
  sends `SIGTERM` to the others (it checks `/proc/<pid>/exe` first). Settings
  writes the `.domain` and the windows read it with `use_defaults`.
- **Cost of the workaround**: one Vulkan and Skia context per window; shared
  state goes through files; «Quit» cannot ask before closing in
  the other processes; tabs cannot be listed or moved between windows.
- **Tested patch**:
  [`kit-parches/0008-varias-ventanas.patch`](../kit-parches/0008-varias-ventanas.patch)
  adds `WindowControls::open_window(app, options)`. With it, `windows::open`
  opens the window in the same process and Quit closes them all; the paste
  history is shared. The patch also fixes a `close_window` panic, which wrote
  the status while the read was still borrowed.
- **Proposed kit API**: `WindowControls::open(app: AppComponent, options: WindowOptions) -> WindowControls` and `Windows::get().list() -> State<Vec<WindowInfo>>`.
- **Severity**: high
- **How to reproduce**: search `barrier_kit::platform` for a public function
  that creates a second `xdg_toplevel` after `run_window`: it does not exist.

### BK-002 — Fixed window title
- **Area**: window
- **What CurseTerm needed**: the window title follows the session
  (OSC 0/2, current directory); the menu bar, the Dock and the «Window»
  list use it.
- **What BarrierKit offers today**: `WindowOptions::new(title, …)`; the runner
  calls `set_title` once (`platform/window.rs:382`). `WindowCommand`
  (`:256`) has no title command.
- **What is missing**: changing the `xdg_toplevel` title live.
- **How CurseTerm solves it**: patch
  [`kit-parches/0001-titulo-en-vivo.patch`](../kit-parches/0001-titulo-en-vivo.patch)
  (`WindowControls::set_title` and `WindowCommand::SetTitle`, with its line in
  `docs/barrierkit/06-ventanas{,.en}.md`). With the `kit-parcheado` feature,
  `terminal.rs` calls it when the title changes; without it only the
  `Titlebar` changes. Tested: on sway, with the patch, the compositor bar shows
  `root@vm: /tmp` after `cd /tmp`; without it, always «CurseTerm».
- **Cost of the workaround**: without the patch, the compositor and the shell always show
  «CurseTerm».
- **Proposed kit API**: `WindowControls::set_title(&self, title: impl Into<String>)`.
- **Severity**: medium
- **How to reproduce**: `grep -rn set_title crates/barrier-kit/src` gives a
  single call, at creation.

### BK-003 — Shortcuts that are not consumed and keys that cannot be declared
- **Area**: input / global menu
- **What CurseTerm needed**: shortcuts such as ⌘D, ⌥⌘←, ⌘[, ⌘] or ⇧⌘↓; and that
  pressing a shortcut does not also reach the terminal shell.
- **What BarrierKit offers today**: `ShortcutKey` (`app/action.rs:9`) has
  `Char`, `Left` and `Right`. The menu text (`MenuItem::Action::shortcut`)
  accepts more keys (`F11`, `Page_Down`), but untyped. In
  `platform/input/seat.rs:424-468` the shortcut is dispatched and the key is sent
  to the app anyway (`:464`).
- **What is missing**: `Up`, `Down`, `Tab`, `Home`/`End`, `PageUp`/`PageDown`, `F1–F12`
  and symbol keys in `ShortcutKey`; and that a dispatched shortcut is not
  forwarded as a key.
- **How CurseTerm solves it**: patch
  [`kit-parches/0002-atajos-con-nombre-y-consumidos.patch`](../kit-parches/0002-atajos-con-nombre-y-consumidos.patch):
  `ShortcutKey::Named(NamedKey)` and `key_event` does not forward the key of a dispatched
  shortcut; it extends the test `a_typed_shortcut_and_its_menu_text_match_the_same_keys`
  and `docs/barrierkit/11-app{,.en}.md`. With the kit as is,
  `crates/curseterm/src/keys.rs` never sends a key with Super to the PTY
  (ADR 0003), so CurseTerm's shortcuts do not reach the shell. The patch
  passes `cargo test -p barrier-kit --lib` (including the extended test) and
  `clippy -D warnings`; Super shortcuts work in both modes.
- **Cost of the workaround**: it only works because every shortcut uses Super; a
  shortcut without Super would arrive twice. Untyped keys are declared as
  text in `MenuItem`.
- **Proposed kit API**: `enum ShortcutKey { Char(char), Named(NamedKey) }` and that `key_event` does not push the `PlatformEvent::Keyboard` when `shortcut_action.is_some()`.
- **Severity**: high
- **How to reproduce**: an `Action` with `Shortcut::ctrl(ShortcutKey::Char('d'))`
  and an `on_global_key_down` at the root: both fire with Ctrl+D.

### BK-004 — No input method
- **Area**: IME
- **What CurseTerm needed**: typing in Chinese, Japanese or Korean and seeing the
  preedit text at the cursor position.
- **What BarrierKit offers today**: keyboard with xkb via SCTK; no module uses
  `zwp_text_input_v3`. Freya 0.4.3 already has the `on_ime_preedit` event
  (`ImePreeditEventData`), but the runner never emits it.
- **What is missing**: the text input protocol and a preedit event.
- **How CurseTerm solves it**: pending.
- **Cost of the workaround**: without IME there is no workaround inside the app.
- **Proposed kit API**: events `Ime::Preedit { text, cursor }` and `Ime::Commit(String)`, and `ime_area(rect)` to place the IME window.
- **Severity**: medium
- **How to reproduce**: `grep -rn text_input crates/barrier-kit/src` gives no
  results.

### BK-005 — No primary selection
- **Area**: clipboard
- **What CurseTerm needed**: copy on select and paste with the middle
  button, like any Linux terminal.
- **What BarrierKit offers today**: a text clipboard over `wl_data_device`
  (`platform/clipboard.rs`), exposed as `freya_clipboard::Clipboard`. The
  primary selection is pending in ADR 0047 («Bloqueado por Freya 0.4.3», blocked by Freya 0.4.3).
- **What is missing**: `zwp_primary_selection_device_v1`.
- **How CurseTerm solves it**: pending.
- **Cost of the workaround**: —
- **Proposed kit API**: `PrimarySelection::set(String)` and `PrimarySelection::get() -> Option<String>`, separate from Freya's `Input`.
- **Severity**: medium
- **How to reproduce**: read ADR 0047, «Pendiente» (Pending).

### BK-006 — No document tabs
- **Area**: containers
- **What CurseTerm needed**: iTerm2's tab bar: live title
  per session, close button, «+», activity indicator and reordering.
- **What BarrierKit offers today**: `views::tabs::TabView`
  (`views/tabs.rs:13-35`), toolbar tabs with a symbol and a
  `&'static str` title.
- **What is missing**: a document `TabBar` with `String` titles and per-tab
  actions.
- **How CurseTerm solves it**: block `barrier_kit_next::views::tab_bar::TabBar`
  (`crates/barrier-kit-next/src/views/tab_bar.rs`): `String` titles, a close
  button on the selected tab or the one under the pointer, `use_interaction` and
  `Palette`; its height is `tokens::TAB_BAR`; `on_add(label, ..)` adds the
  «+» button. Tested: tabs with title by OSC, select, close and «+»
  (`docs/capturas/fase2-barra-de-pestanas.png`). Drag reordering
  and the activity indicator are missing.
- **Cost of the workaround**: —
- **Proposed kit API**: `TabBar::new(label).tab(Tab::new(id, title: String).closable(true)).on_select(..).on_close(..).on_add(..)`.
- **Severity**: medium
- **How to reproduce**: `TabView::tab` does not accept a `String`.

### BK-007 — No generic splitter
- **Area**: containers
- **What CurseTerm needed**: vertical and horizontal split panes,
  nested, with a resize handle.
- **What BarrierKit offers today**: `views::split::SplitView`
  (`views/split.rs:94`): sidebar + detail + inspector.
- **What is missing**: a nestable splitter of N children along one direction.
- **How CurseTerm solves it**: block
  `barrier_kit_next::views::split_panes::SplitPanes`
  (`crates/barrier-kit-next/src/views/split_panes.rs`): N panes on one axis
  with `Size::flex(ratio)`, `tokens::SPLIT_PANES.divider` dividers that can be
  dragged (`ColResize`/`RowResize`), a `pane_min` minimum per pane; controlled
  with `.ratios(..)` and `.on_ratios(..)`; it nests by putting a `SplitPanes` as a
  pane. CurseTerm keeps the tree in `panes.rs`. Tested: nested splits,
  dragging and keyboard (`docs/capturas/fase2-paneles.png`,
  `fase2-divisor-arrastrado.png`).
- **Cost of the workaround**: dragging uses the ratios of the last
  render; with many events between two frames some travel is lost.
- **Proposed kit API**: the block's: `SplitPanes::new(label, Direction).pane(a).pane(b).ratios(Vec<f32>).on_ratios(cb)`.
- **Severity**: medium
- **How to reproduce**: `SplitView` has no direction option.

### BK-008 — No monospace face in Tengen
- **Area**: tokens / text
- **What CurseTerm needed**: the default terminal font and code text in
  Preferences.
- **What BarrierKit offers today**: `vow_design::Font` with `family`; in
  `crates/vow-design/tengen/tokens.json:285-321` all are `sans`. ADR 0047
  mentions a «Mono» style that does not exist.
- **What is missing**: a `MONO` token (family, size, line height) and the
  metrics of a terminal view: alpha of the broadcast stripes, the highlight
  and the bell, stripe width, multi-click interval and font size range; also
  the sizes of the terminal, Settings and About windows.
- **How CurseTerm solves it**: block `barrier_kit_next::tokens::MONO`
  (Geist Mono 13/16, which the kit's dev shell already ships in `geist-font`),
  and `tokens::TERMINAL`, `SETTINGS_WINDOW_SIZE` and `ABOUT_WINDOW_SIZE` (the
  terminal is measured in cells, BK-012).
- **Cost of the workaround**: —
- **Proposed kit API**: `vow_design::MONO: Font` in `tokens.json` and `vow_design::TERMINAL: TerminalMetrics` in `components.json`.
- **Severity**: low
- **How to reproduce**: `grep -n '"family"' crates/vow-design/tengen/tokens.json`.

### BK-009 — No public canvas and paragraph cache turned off
- **Area**: text / performance
- **What CurseTerm needed**: drawing a grid of 80×24 to 300×100 cells
  at 60 Hz.
- **What BarrierKit offers today**: Freya's `style::text`, `label` and `paragraph`. The kit draws by hand only in private modules (`glass.rs:176`). The
  font collection turns off the paragraph cache (`fonts.rs:20`), and Freya
  repaints the whole tree on every frame (ADR 0047).
- **What is missing**: a documented canvas primitive that gives Skia's `Canvas`,
  the kit's font collection and the scale.
- **How CurseTerm solves it**: block `barrier_kit_next::canvas::canvas(revision, painter)`
  (`crates/barrier-kit-next/src/canvas.rs`): an `ElementExt` that fills its parent,
  clips, translates and scales to logical pixels and calls the painter with Skia's
  `Canvas` and the `FontCollection`; it paints again when
  `revision` changes. The terminal grid (`crates/curseterm/src/grid.rs`) uses it.
  Tested at scale 1 and 1.5, without seams; at rest it produces no frames. The app
  must put into `revision` everything that changes the drawing (focus, scheme,
  font): the canvas does not see the state the painter reads.
- **Cost of the workaround**: it depends on `ElementExt`, a Freya API the kit does not
  document; the grid draws cell by cell.
- **Proposed kit API**: `canvas(|ctx: &mut CanvasContext| { .. })` with `ctx.canvas()`, `ctx.fonts()` and `ctx.scale()`.
- **Severity**: medium
- **How to reproduce**: search for «canvas» in `docs/barrierkit/`: it does not appear.

### BK-010 — Static action titles and no live menu replacement
- **Area**: global menu
- **What CurseTerm needed**: «Profiles» fills with the saved
  profiles and «Window» with the open windows.
- **What BarrierKit offers today**: `Action::new(id, title: fn() -> &'static str)`
  (`app/action.rs:79`); `WindowOptions::menu` fixes the menu when the
  window is created. `MenuHandle::update(id, f)` (`app_menu.rs`) changes an item or the
  children of a `Submenu` with an id; `replace_menu` exists only under `#[cfg(test)]`
  (`app_menu.rs:110`).
- **What is missing**: actions with a computed title and a documented hook for
  dynamic menus.
- **How CurseTerm solves it**: block
  `barrier_kit_next::app_menu::use_menu_enabled(id, enabled)`, the public
  version of the private `use_menu_enabled` in `app/undo.rs`; «Copy» turns off with no
  selection. In phase 2 it was generalized: `use_menu_update(id, value, apply)`,
  `use_menu_checked`, `use_submenu_items` and `submenu(id, ..)` (there is no
  `MenuItem::Submenu` constructor with an id). Tested over D-Bus: «Select Tab»
  fills with the tabs and checks the active one; «Show Tab Bar» and «Maximize
  Active Pane» carry a check mark; a dozen entries enable according to the
  context. For the «Profiles» menu, which is top-level, `update` does not
  work: the patch
  [`kit-parches/0004-items-de-un-menu-en-vivo.patch`](../kit-parches/0004-items-de-un-menu-en-vivo.patch)
  adds `MenuHandle::set_menu_items(título, ítems)` and documents `MenuHandle`
  in `11-app{,.en}.md`. Tested over D-Bus: «+» in Settings adds «New Profile»
  to the terminal menu at once. With the kit as is, only items that already
  existed are renamed.
- **Cost of the workaround**: dynamic entries are not `Action`s, so they do not
  have a typed `Shortcut`.
- **Proposed kit API**: `use_menu(|menu: &mut AppMenu| ..)` that is re-evaluated when its state changes.
- **Severity**: medium
- **How to reproduce**: try `Action::new("profile-1", || nombre_cargado)`:
  it does not compile, because the title cannot capture.

### BK-011 — Documentation that does not match the code
- **Area**: docs
- See «BarrierKit documentation», below.
- **Severity**: low

### BK-012 — The saved size is one per `app_id` and another cannot be asked for
- **Area**: window / persistence
- **What CurseTerm needed**: opening «About» (fixed size) without changing the
  size the next terminal window opens with; and, since phase 6, opening each
  new window with the profile's columns and rows, like iTerm2.
- **What BarrierKit offers today**: `platform/window.rs:333` saves
  `<app_id>.window` when any window closes and restores it in all windows of the
  same `app_id` (`:373`).
  The restored size always wins over `WindowOptions::size`, and
  `WindowControls` cannot ask for another size later.
- **What is missing**: a restoration key per window kind and opening at an
  own size even when one is saved.
- **How CurseTerm solves it**: `windows::forget_saved_size` empties that
  state before each `run_window`; the terminal computes its size from the
  profile's font metrics (`main.rs::terminal_size`) and each auxiliary
  window uses its own.
- **Cost of the workaround**: it writes a private kit file by its name; no
  window remembers its size or whether it was maximized.
- **Proposed kit API**: `WindowOptions::restoration_key(impl Into<String>)` and `WindowOptions::restore(false)`.
- **Severity**: medium
- **How to reproduce**: two windows with the same `app_id`, one fixed at 320×280;
  close the fixed one and open the other: it opens at 320×280 clamped to its minimum.

### BK-013 — A close requested by the compositor cannot be intercepted
- **Area**: window
- **What CurseTerm needed**: asking «Close the window? N
  sessions are running», like iTerm2.
- **What BarrierKit offers today**: `WindowHandler::request_close`
  (`platform/window.rs`) calls `close_window` without telling the app.
- **What is missing**: a notice the app can cancel.
- **How CurseTerm solves it**: none; the window closes without asking.
- **Cost of the workaround**: —
- **Proposed kit API**: `use_close_request(|| CloseResponse::Allow | Cancel)`.
- **Severity**: medium
- **How to reproduce**: in Malaysia, Super+W on a window with a program
  running.

### BK-014 — `Timers::wakeup` cannot be unregistered
- **Area**: platform
- **What CurseTerm needed**: waking the UI thread from the I/O thread of
  each session.
- **What BarrierKit offers today**: `Timers::wakeup(callback) -> WakeHandle`
  (`platform/timer.rs`); the callback stays registered forever and runs on
  every wakeup from any source (`platform/runtime.rs:213`).
- **What is missing**: unregistering the callback on unmount.
- **How CurseTerm solves it**: a single `wakeup` per window and a channel
  shared by all its sessions (`terminal.rs`, `listen`).
- **Cost of the workaround**: none worth noting.
- **Proposed kit API**: `use_wakeup(callback) -> WakeHandle` that unregisters with `use_drop`.
- **Severity**: low
- **How to reproduce**: call `wakeup` in a component that mounts and
  unmounts: the callback keeps running.

### BK-015 — Preferences always go to `~/.config/malaysia/`
- **Area**: persistence
- **What CurseTerm needed**: the brief suggested `~/.config/curseterm/`.
- **What BarrierKit offers today**: `load_defaults`, `save_defaults` and
  `use_defaults` (`app/defaults.rs`) use `~/.config/malaysia/<app_id>.domain`,
  and `use_watched` only watches Malaysia's folders.
- **What is missing**: nothing for a desktop app; for a third-party app
  that runs on other desktops, a folder of its own.
- **How CurseTerm solves it**: accepted; the preferences live in
  `~/.config/malaysia/org.curseterm.CurseTerm.domain`.
- **Cost of the workaround**: —
- **Proposed kit API**: none for now.
- **Severity**: low
- **How to reproduce**: read `app/defaults.rs:65`.

### BK-016 — Shortcuts with Shift compare the produced character
- **Area**: input / global menu
- **What CurseTerm needed**: iTerm2's ⇧⌘] and ⇧⌘[ (next and previous
  tab) as Super+Shift+] and Super+Shift+[.
- **What BarrierKit offers today**: `shortcut_matches` (`menu/shortcut.rs`)
  compares the shortcut glyph with `key.to_string()`. With Shift, on a
  US keyboard, the `]` key arrives as `}`.
- **What is missing**: comparing by physical key (`Code`) or by keysym without Shift.
- **How CurseTerm solves it**: «Select Next Tab» and «Select Previous
  Tab» are declared as text as `Super+Shift+}` and `Super+Shift+{`, and
  «Show Composer» (⇧⌘.) as `Super+Shift+>` (`crates/curseterm/src/menu.rs`).
  Tested with `wtype`.
- **Cost of the workaround**: it depends on the keyboard layout; the menu shows `}` instead of `]`.
- **Proposed kit API**: that `find_menu_shortcut` also receives the `Code` and compares by it when Shift is held.
- **Severity**: medium
- **How to reproduce**: a `MenuItem` with `"Super+Shift+bracketright"`;
  pressing Super+Shift+] does not fire it.

### BK-017 — Menu handlers run outside Freya's context
- **Area**: global menu / platform
- **What CurseTerm needed**: copy, paste and close from «Edit › Copy»,
  «Paste» and «Shell › Close», or from their shortcuts.
- **What BarrierKit offers today**: `use_menu_action` (`app/action.rs`). The
  runner calls `MenuActions::dispatch` from `Runtime::key_event`
  (`platform/input/seat.rs:454`) and from the D-Bus actions, outside
  Freya's context. `docs/barrierkit/11-app.md` only says it runs «en el
  hilo de la interfaz» (on the UI thread). The same happens with the `Timers::wakeup` callbacks.
- **What is missing**: running the handlers inside the tree's context, like an
  event, or documenting which APIs cannot be used in them.
- **How CurseTerm solves it**: `Handles` in `terminal.rs` keeps the clipboard
  `State` (`consume_root_context`) and `WindowControls` during
  `render`; the handlers and the `wakeup` callbacks use those copies. The
  auxiliary windows capture `WindowControls` the same way. In phase 4,
  `Timers::get()` inside a `wakeup` callback also aborted the app
  (bell from a trigger): `Handles` now keeps `Timers` as well.
- **Cost of the workaround**: the app uses the internal `State` of
  `freya-clipboard` instead of `Clipboard`.
- **Proposed kit API**: that `MenuActions::dispatch` runs inside `with_context` of the window's tree, or that the handler receives a `MenuContext` with clipboard and window.
- **Severity**: blocking (the app aborts with
  «Your trying to access Freya's current context outside of it»).
- **How to reproduce**: `use_menu_action(|id| { let _ = Clipboard::set(id.into()); })`
  and press that action's shortcut.

### BK-018 — Without `org_kde_kwin_appmenu_manager` the menu is not published on D-Bus
- **Area**: global menu / docs
- **What CurseTerm needed**: the global menu on GNOME or KDE without Malaysia, and
  being able to check it over D-Bus.
- **What BarrierKit offers today**: `README.md` and `docs/barrierkit/11-app.md`
  say that without the protocol the menu «sigue existiendo en D-Bus» (still exists on D-Bus), but
  `platform/window.rs:407-419` only calls `app_menu::server::register` if
  `appmenu_manager` exists.
- **What is missing**: always registering the menu and announcing it only if the protocol is present.
- **How CurseTerm solves it**: patch
  [`kit-parches/0003-menu-en-dbus-sin-appmenu.patch`](../kit-parches/0003-menu-en-dbus-sin-appmenu.patch).
  Tested on sway: `com.canonical.dbusmenu.GetLayout` on `/MenuBar/0` returns
  CurseTerm, Shell and Edit with their shortcuts; «Copy» goes from `enabled=false` to
  `true` on selection; `Event(clicked)` on «New Tab» opens the tab.
- **Cost of the workaround**: with the kit as is there is no way to check the
  menu outside Malaysia.
- **Proposed kit API**: none; it is the patch's change.
- **Severity**: high
- **How to reproduce**: open any app with `.menu(..)` on sway and list
  the objects of its name on the bus: there is no `/MenuBar/*`.

### BK-019 — An app cannot request an xdg-activation token
- **Area**: window / platform
- **What CurseTerm needed**: the list of open windows in iTerm2's
  «Window» menu, which brings the chosen one to the front.
- **What BarrierKit offers today**: `WindowControls::present(token)`
  (`platform/window.rs:207`) uses a token it already has; requesting one
  (`xdg_activation_v1.get_activation_token`) only exists in
  `shell::Activation` (`shell/activation.rs`), behind the `shell` feature.
- **What is missing**: `Activation::request` for any app, with the window's surface and
  last input serial.
- **How CurseTerm solves it**: the list is not shown; an entry that does not
  bring the window forward would be decorative. In Malaysia, the menu bar already adds
  the app's windows to «Window».
- **Cost of the workaround**: outside Malaysia there is no way to jump to another
  CurseTerm window from the menu.
- **Proposed kit API**: move `Activation` from `shell/` to `platform/` and give it `WindowControls::activation_token(callback)`.
- **Severity**: medium
- **How to reproduce**: search for `get_activation_token` outside
  `crates/barrier-kit/src/shell/`: it does not appear.

### BK-020 — Pane, row and pop-up labels are `&'static str`
- **Area**: controls
- **What CurseTerm needed**: profile names and imported scheme names, which the user types, as `PopUpButton` items and `Pane` titles.
- **What BarrierKit offers today**: `PopUpButton::item(&'static str)`
  (`popup_button.rs:38`), `Row::new(&'static str)` (`views/form.rs:31`),
  `Pane::new(&'static str, &'static str, ..)` (`patterns/preferences.rs:35`),
  `RadioGroup::option(&'static str)`. Designed for `tr!`, which returns
  `&'static str`.
- **What is missing**: accepting `impl Into<Cow<'static, str>>` as
  `Button::new` and `about_window` already do.
- **How CurseTerm solves it**: `intern` in `crates/curseterm/src/preferences.rs`
  leaks each name once into a `&'static str` and reuses it.
- **Cost of the workaround**: memory that is never freed for each distinct name
  (small in practice).
- **Proposed kit API**: `fn item(self, label: impl Into<Cow<'static, str>>)` and the same in `Row`, `Pane` and `RadioGroup`.
- **Severity**: medium
- **How to reproduce**: `PopUpButton::new("x").item(nombre.as_str())` does not compile.

### BK-021 — `RadioGroup` is always horizontal
- **Area**: controls
- **What CurseTerm needed**: iTerm2's «Initial directory»: three options in a column.
- **What BarrierKit offers today**: `choice_group` fixes `Direction::Horizontal`
  (`choices.rs:50`); with three long options the last one overflows the row.
- **What is missing**: `.direction(Direction::Vertical)` or a column variant.
- **How CurseTerm solves it**: `directory_choices` in `preferences.rs`
  stacks three single-option `RadioGroup`s, controlled from the profile.
- **Cost of the workaround**: the screen reader sees three groups instead of one.
- **Proposed kit API**: `RadioGroup::new(label).vertical(true)`.
- **Severity**: low
- **How to reproduce**: a `RadioGroup` with three options in a narrow `Row`.

### BK-022 — A `.domain` change does not repaint unfocused windows
- **Area**: platform / persistence
- **What CurseTerm needed**: that the terminal changes scheme as soon as
  it is chosen in Settings, which is another process and has focus.
- **What BarrierKit offers today**: when a `.domain` changes, the runner bumps
  `defaults_revision` and does a `ping` (`platform/runtime.rs:255`), but does not set
  `request_redraw_all`, as it does for appearance (`:263`).
- **What is missing**: requesting the repaint in that case too.
- **How CurseTerm solves it**: patch
  [`kit-parches/0005-domain-repinta-sin-foco.patch`](../kit-parches/0005-domain-repinta-sin-foco.patch).
  Tested on sway: with Settings in front and the terminal unfocused, editing the
  `.domain` does not change the terminal with the kit as is and does with the patch
  (`docs/capturas/fase3-repintado-sin-foco-parche-0005.png`).
- **Cost of the workaround**: with the kit as is, the change appears when the
  window receives any other event.
- **Proposed kit API**: none; it is the patch's change.
- **Severity**: high
- **How to reproduce**: two windows that use `use_defaults`; with focus on
  one, change the `.domain` from it: the other does not repaint.

### BK-023 — No file chooser
- **Area**: app
- **What CurseTerm needed**: «Import…» of `.itermcolors` schemes.
- **What BarrierKit offers today**: nothing; ADR 0047 leaves «`Open` de archivos y
  URLs» (`Open` for files and URLs) pending until the first app that uses it. Freya has
  `on_file_drop`, but the runner does not emit it.
- **What is missing**: an open dialog through the
  `org.freedesktop.portal.FileChooser` portal (Malaysia has its
  `xdg-desktop-portal-malaysia`).
- **How CurseTerm solves it**: a `TextField` with the path and the
  «Import» button in Colors, and `curseterm --import-colors <archivo>`.
- **Cost of the workaround**: the path must be typed by hand.
- **Proposed kit API**: `app::open_file(filters: &[(&str, &[&str])], callback: impl FnOnce(Option<PathBuf>))`.
- **Severity**: medium
- **How to reproduce**: search for `FileChooser` in `crates/barrier-kit/src`: it does not appear.

### BK-024 — Missing symbols for a settings bar, the bell and the trash
- **Area**: symbols
- **What CurseTerm needed**: the icons of iTerm2's Settings bar
  (gear, eye, person, keyboard, pointer, bolt), the bell that marks a
  background tab where it rang (phase 5), the trash that deletes an
  annotation (phase 35) and the session indicators (phase 42): maximize
  arrows, broadcast waves, two swapping rectangles, an eye and two
  documents.
- **What BarrierKit offers today**: 31 symbols in `symbol.rs`; none of those.
  `Symbol::path` is private, so a canvas cannot draw the ones there are
  either.
- **What is missing**: extending the registry and exposing each symbol's
  path.
- **How CurseTerm solves it**: `SlidersVertical` for General,
  `CircleHalf` for Appearance, `ListBullet` for Profiles, `SquareGrid` for
  Pointer and `SpeakerWave` in the tab badge (`Badge` of
  the `views::tab_bar` block). The note's trash is drawn on its canvas with
  the grid and stroke of `SYMBOL` (`note_view.rs`), like the indicators
  (`indicators.rs`).
- **Cost of the workaround**: less like iTerm2; SVG paths outside the
  registry, with unrounded ends because `freya-engine` does not re-export
  `paint::Cap`.
- **Proposed kit API**: `Symbol::{Gear, Eye, Person, Keyboard, Pointer, Bolt, Bell, Trash, Broadcast, Swap, DocOnDoc}` and a public `Symbol::path`.
- **Severity**: low
- **How to reproduce**: read `enum Symbol` in `crates/barrier-kit/src/symbol.rs`.

### BK-025 — No global shortcuts nor a way to hide and show the window
- **Area**: platform / input
- **What CurseTerm needed**: iTerm2's hotkey window: a system-wide
  shortcut that shows and hides a terminal window without losing its
  sessions.
- **What BarrierKit offers today**: nothing. `Action` shortcuts only arrive while
  the window is focused; `WindowControls` can minimize, but xdg-shell does not
  let a client unminimize itself. Malaysia's portal
  (`crates/xdg-desktop-portal-malaysia/src`) only implements `Settings`, and its
  audit (`docs/auditoria-claude-2026-11-03.md:613`) notes that the GTK
  fallback does not provide `GlobalShortcuts`.
- **What is missing**: registering shortcuts with the
  `org.freedesktop.portal.GlobalShortcuts` portal (and Malaysia implementing it), and
  hiding and showing one's own window again.
- **How CurseTerm solves it**: `curseterm --hotkey` (`windows.rs`)
  saves its pid in `$XDG_RUNTIME_DIR/curseterm/hotkey`; if a hotkey window
  is already alive, it closes it with `SIGTERM` and exits; if not, it becomes it. The
  user binds it to a desktop shortcut.
- **Cost of the workaround**: closing the window loses its sessions; no
  animation or position of its own; the shortcut must be configured outside the app.
- **Proposed kit API**: `app::global_shortcut(id, description, preferred: Shortcut, on_activated: impl Fn())` over the portal, and `WindowControls::{hide, show}`.
- **Severity**: medium
- **How to reproduce**: search for `GlobalShortcuts` in `crates/`: it does not appear.

### BK-026 — No bell, attention request or notifications
- **Area**: app / platform
- **What CurseTerm needed**: the bell (BEL and the «Ring Bell» trigger),
  which in iTerm2 sounds, requests attention if the window has no focus and can
  post a notification; and the mark and profile alerts (phases 40 and 41),
  which use `NSUserNotificationCenter` and `NSSound` and reveal the session
  when the notification is clicked.
- **What BarrierKit offers today**: nothing; in `crates/barrier-kit/src` there is no
  sound, urgency or notifications (only «notifies» appears, in a menu
  test).
- **What is missing**: an event sound from the freedesktop theme and a notification
  through `org.freedesktop.portal.Notification`.
- **How CurseTerm solves it**: visual bell: the pane lightens by
  20 % with the text color for `DURATION_FAST_MS` and `Timers::after`
  repaints it when done. For notifications and sounds, the `notifications`
  block in `barrier-kit-next`: `post(Notice, clicked)` calls `Notify` on
  `org.freedesktop.Notifications` on a thread, with the `default` action and
  the `desktop-entry` hint; one thread listens for `ActivationToken`,
  `ActionInvoked` and `NotificationClosed` and calls `clicked` with the
  activation token when the server sends one. `play_event_sound(name)` runs
  `canberra-gtk-play --id name` with a name from the freedesktop sound
  naming specification.
- **Cost of the workaround**: the bell still has no sound nor attention
  request; `canberra-gtk-play` is an external program and, without it, only a
  warning is logged; `zbus` was already in the tree through the global menu.
- **Proposed kit API**: `app::beep()`, and `notifications::post` and
  `notifications::play_event_sound` like the block, with the name and the
  `desktop-entry` taken from the app.
- **Severity**: medium
- **How to reproduce**: `grep -rni "bell\|attention\|notif" crates/barrier-kit/src`.

### BK-027 — A view with global keyboard does not know whether a text field has focus
- **Area**: controls / input
- **What CurseTerm needed**: the search bar (Edit › Find): what is
  typed in it must not reach the shell, and Escape closes it. In phase 6,
  the "Close?" alert: its Enter or Escape must not reach the shell.
- **What BarrierKit offers today**: `SearchField` with `auto_focus`,
  `on_focus_change` and `on_navigate` (Up, Down, Enter). Its
  `stop_propagation` does not stop the `on_global_key_down` handlers, which Freya derives from
  each `KeyDown` (`freya-core` 0.4.3, `events/name.rs:144`), and Escape does not reach
  `on_navigate` (`controls.rs:951`). `Alert` (`views/feedback.rs`) handles
  Enter and Escape in its own `on_global_key_down`, which does not stop the
  others either.
- **What is missing**: knowing from the view whether a kit text field has
  focus, and a cancel callback.
- **How CurseTerm solves it**: `terminal.rs` tracks the focus of the
  `SearchField` with `on_focus_change`; with the bar open and focused, the
  global handler ignores keys and Escape closes the bar. With the alert up
  the handler ignores everything, and the alert is removed with
  `Timers::after(Duration::ZERO)` (`dismiss_later`) so the handler still
  sees it open when it gets that same key. Notes and the Composer report
  focus with `on_focus_change` of the `views::text_view` block, and the
  Composer stops editing one loop after Escape closes it.
- **Cost of the workaround**: one more piece of state that goes out of sync if the bar
  closes without going through `Find::close`.
- **Proposed kit API**: `SearchField::on_cancel` and
  `use_text_input_focused() -> bool`, and an `Alert` that consumes its keys.
- **Severity**: low
- **How to reproduce**: a `rect().on_global_key_down(…)` with a
  `SearchField` inside: each key in the field also reaches the handler.
  Detected in the code; the workaround went in before reproducing it. With
  `Alert` it was reproduced: without `dismiss_later`, Escape cancelled the
  alert and the shell got `^[`.

### BK-028 — `SegmentedControl` labels wrap onto two lines
- **Area**: controls
- **What CurseTerm needed**: iTerm2's eight profile sections (General,
  Colors, Text, Window, Terminal, Session, Keys, Advanced) in a
  `SegmentedControl`.
- **What BarrierKit offers today**: each segment takes `Size::flex(1.0)` and
  its `label()` has no line limit (`controls.rs:769-791`): at 860 px,
  "Advanced" wraps in two.
- **What is missing**: one line per label, with widths by content or
  scrolling when they do not fit.
- **How CurseTerm solves it**: `SETTINGS_WINDOW_SIZE` becomes 980 × 600.
- **Cost of the workaround**: below that width the label wraps again.
- **Proposed kit API**: `label().max_lines(1)` in the segment and widths by content.
- **Severity**: low
- **How to reproduce**: CurseTerm Settings with `SETTINGS_WINDOW_SIZE` at 860 × 560, Profiles.

### BK-029 — `TextField` cannot take the focus when it opens
- **Area**: controls
- **What CurseTerm needed**: Window › Edit Tab Title and Edit Window Title,
  a sheet with a single field that takes typing as soon as it opens.
- **What BarrierKit offers today**: `SearchField::auto_focus`, but
  `TextField` (`controls.rs:805`) does not pass `auto_focus` to its `Input`.
- **What is missing**: `TextField::auto_focus(bool)`, as in `SearchField`.
- **How CurseTerm solves it**: the patch
  [`kit-parches/0006-textfield-auto-focus.patch`](../kit-parches/0006-textfield-auto-focus.patch)
  adds the method and its documentation. With the kit as is, the field only
  takes the focus on a click.
- **Workaround cost**: without the patch, whatever is typed before the
  click is lost.
- **Proposed kit API**: `TextField::auto_focus(bool)`.
- **Severity**: medium
- **How to reproduce**: Window › Edit Tab Title with `./dev.sh kit-tal-cual` and type without clicking.

### BK-030 — Mouse events carry no modifiers
- **Area**: input
- **What CurseTerm needed**: Super+click to open a link, like ⌘-click in
  iTerm2; Shift to select when the program asks for the mouse, and Ctrl and
  Alt inside the mouse report.
- **What BarrierKit offers today**: the runner keeps the keyboard modifiers
  (`platform/input/seat.rs:399`), but Freya 0.4.3's `MouseEventData` only
  carries position and button.
- **What is missing**: the modifiers in mouse and wheel events.
- **How CurseTerm solves it**: the window tracks Super, Shift, Ctrl and Alt
  with `on_global_key_down` and `on_global_key_up`. When a modifier is
  pressed, the compositor sends the key before the modifiers, so the key
  itself counts as held.
- **Workaround cost**: if a modifier is released while the window has no
  focus, the window thinks it is held until the next key.
- **Proposed kit API**: `modifiers` in the pointer data, or a query for the current modifiers.
- **Severity**: medium
- **How to reproduce**: read `MouseEventData` in `freya-core-0.4.3/src/events/data.rs`.

### BK-031 — A window cannot ask for a translucent background
- **Area**: window / appearance
- **What CurseTerm needed**: View › Use Transparency (⌘U) and iTerm2's
  per-profile transparency.
- **What BarrierKit offers today**: the runner clears each window with
  `surface_background` (`theme.rs:30`), which comes from the global
  appearance; with alpha 1 it declares the region opaque
  (`platform/surface.rs:238`). The app cannot change it.
- **What is missing**: a per-window background, or an alpha the app can ask
  for, composed by the compositor with its blur.
- **How CurseTerm solves it**: the patch
  [`kit-parches/0007-fondo-translucido.patch`](../kit-parches/0007-fondo-translucido.patch)
  adds `WindowControls::set_background_alpha`, which multiplies that
  background's alpha, and its documentation. With the kit as is, View › Use
  Transparency stays disabled.
- **Workaround cost**: without the patch, the window is always opaque.
- **Proposed kit API**: `WindowControls::set_background(Rgba)` or `WindowOptions::background`.
- **Severity**: medium
- **How to reproduce**: paint the app root with alpha 0.5; the window stays opaque.

### BK-032 — `Timers` callbacks outlive their window
- **Area**: platform
- **What CurseTerm needed**: closing a terminal window while the others in
  the same process stay open (patch 0008).
- **What BarrierKit offers today**: `Timers::wakeup` and `Timers::after`
  (`platform/timer.rs:47-58`) keep the function in a runtime queue shared by
  every window; each ping calls all the `wakeup` ones and none is removed when
  its window closes.
- **What is missing**: tying callbacks to the window that asked for them and
  dropping them when it closes.
- **How CurseTerm solves it**: an `alive: Rc<Cell<bool>>` turned off in
  `use_drop`; the I/O wakeup and `Handles::after` do nothing when it is off.
  Without it, closing a window made the next ping read freed states and the
  process panicked.
- **Workaround cost**: the closed windows' functions stay in the queue and
  run on every ping until the process ends.
- **Proposed kit API**: make `Timers` per window, or have `wakeup` return a guard that removes it when dropped.
- **Severity**: high
- **How to reproduce**: with patch 0008, open two terminal windows and close one without the `alive` flag.

### BK-033 — No input method (`zwp_text_input_v3`)
- **Area**: input
- **What CurseTerm needed**: typing with an input method (Chinese, Japanese,
  composed accents), with the composition text at the cursor.
- **What BarrierKit offers today**: Freya 0.4.3 has `ImePreeditEventData`,
  but the runner does not bind `zwp_text_input_manager_v3`, so no input
  method reaches the app.
- **What is missing**: the protocol's text input, the composition text for
  the app and the cursor position for the input method.
- **How CurseTerm solves it**: the patch
  [`kit-parches/0009-metodo-de-entrada.patch`](../kit-parches/0009-metodo-de-entrada.patch)
  adds `platform/ime.rs`: the composition text is published in
  `WindowControls::ime_preedit`, the committed text arrives as
  `Key::Character` and `set_ime_cursor_area` places the input method's
  window. CurseTerm draws the composition underlined at the cursor.
  smithay-client-toolkit only reports seats that appear after start-up, so
  the initial one is attached when the runtime is built.
- **Workaround cost**: without the patch there is no input method; the kit's
  text fields get the committed text but do not show the composition.
- **Proposed kit API**: the patch's, or Freya's `ImePreedit` events delivered to the focused element.
- **Severity**: high
- **How to reproduce**: search `crates/barrier-kit/src/platform` for `text_input`: it does not appear.

### BK-034 — No primary selection
- **Area**: clipboard
- **What CurseTerm needed**: the usual Linux behaviour: selecting with the
  mouse publishes the text and the middle button pastes it, also across apps.
- **What BarrierKit offers today**: the clipboard (`platform/clipboard.rs`)
  only uses `wl_data_device`; smithay-client-toolkit 0.20 already ships
  `primary_selection`, but the runner does not bind it.
- **What is missing**: the `zwp_primary_selection` device and a provider for
  the app.
- **How CurseTerm solves it**: the patch
  [`kit-parches/0010-seleccion-primaria.patch`](../kit-parches/0010-seleccion-primaria.patch)
  adds `platform/primary.rs`, which reuses the kit's `Clipboard` with that
  device, and `WindowControls::primary_selection()`. CurseTerm publishes the
  selection on mouse release and pastes the primary one with the middle
  button.
- **Workaround cost**: without the patch, the middle button does nothing.
- **Proposed kit API**: the patch's.
- **Severity**: medium
- **How to reproduce**: select in CurseTerm with the kit as is and run `wl-paste --primary`: it gives nothing.

### BK-035 — The clipboard only offers plain text
- **Area**: clipboard
- **What CurseTerm needed**: Edit › Copy with Styles (⌥⌘C), which copies the
  selection with its colours and styles to paste it formatted.
- **What BarrierKit offers today**: `ClipboardProvider::set_contents` only
  publishes `TEXT_MIMES` (`platform/clipboard.rs:20`).
- **What is missing**: offering several formats at once and serving each.
- **How CurseTerm solves it**: the patch
  [`kit-parches/0011-copiar-con-formato.patch`](../kit-parches/0011-copiar-con-formato.patch)
  adds `WindowControls::copy_rich(text, html)`, which also offers
  `text/html`. CurseTerm builds the HTML from each cell's resolved colours
  (`styled.rs`). Without the patch, the item copies plain text.
- **Workaround cost**: none with the patch.
- **Proposed kit API**: the patch's, or a list of format and data pairs.
- **Severity**: low
- **How to reproduce**: copy with the kit as is and run `wl-paste -l`: there are only text types.

### BK-036 — No multi-line text view
- **Area**: controls
- **What CurseTerm needed**: the text of an annotation, which in iTerm2 is a
  frameless `NSTextView`: Enter breaks the line, the text wraps to the width
  and Escape gives the focus back to the terminal (`PTYNoteViewController.m`).
- **What BarrierKit offers today**: `TextField`, `SearchField` and
  `SecureField`, all single-line on Freya's `Input`, which uses
  `max_lines(1)` and turns Enter into `on_submit`.
- **What is missing**: a multi-line text control.
- **How CurseTerm solves it**: the `views::text_view::TextView` block in
  `barrier-kit-next`, on `use_editable` from `freya-edit` 0.4.3, with the same
  editing, drag selection, IME and cursor blink as `Input`. It reports text
  and focus changes through callbacks; `on_key` lets the app take a key with
  the cursor position (the Composer uses ⇧↩ and ⌥⇧↩), and it types no
  characters with Super or Ctrl held, except the editing commands.
- **Cost of the workaround**: two more dependencies in the block
  (`freya-components` without default features and `freya-edit`, already in
  the tree through the kit) and no scroll bar: it grows with the text.
- **Proposed kit API**: `TextView::new(value, family, size, line_height,
  ink)` with `cursor`, `width`, `height`, `auto_focus`, `on_change`,
  `on_focus_change` and `on_key`, like the block. `height` lets the field
  fill its box, so a click anywhere in it (Edit Snippet's Value) focuses it.
- **Severity**: medium
- **How to reproduce**: look in `crates/barrier-kit/src/controls.rs` for a
  control that takes Enter as a line break.

### BK-037 — `Alert` has only two buttons and cannot be silenced
- **Area**: controls / alerts
- **What CurseTerm needed**: iTerm2's multi-line paste warning
  (`iTermPasteHelper.m`), with Paste, Paste Without Newline, Cancel and
  Advanced…, and `iTermWarning`'s do-not-ask-again checkbox.
- **What BarrierKit offers today**: `views::feedback::Alert` with a confirm
  button and a cancel button (`feedback.rs:60`).
- **What is missing**: more actions in an `Alert` and a checkbox to silence
  it.
- **How CurseTerm solves it**: the paste warnings are a sheet of its own
  (`paste_question_sheet` in `terminal.rs`) built from the kit's `sheet` and
  `Button`, with every iTerm2 button; to stop them, uncheck the boxes in
  Edit › Paste Special.
- **Cost of the workaround**: a hand-made sheet instead of the kit's
  `Alert`, and no «don't ask again» from the warning itself.
- **Proposed kit API**: `Alert::action(label, callback)` to add buttons and
  `Alert::suppression(label, State<bool>)`.
- **Severity**: low
- **How to reproduce**: read `pub struct Alert` in
  `crates/barrier-kit/src/views/feedback.rs`.

### BK-038 — `SearchField` has no options menu
- **Area**: controls
- **What CurseTerm needed**: iTerm2's find mode (Smart Case Substring,
  Case-Sensitive Substring, Case-Insensitive Substring, Case-Sensitive Regex
  and Case-Insensitive Regex), which in `FindView.xib` is the menu of the
  search field's magnifier.
- **What BarrierKit offers today**: `SearchField` with `auto_focus`,
  `on_focus_change` and `on_navigate` (`controls.rs:850`); the magnifier is
  only a drawing.
- **What is missing**: a menu on the `SearchField` magnifier, like
  `NSSearchField.searchMenuTemplate`.
- **How I solved it in CurseTerm**: a `PopUpButton` with the five modes beside
  the field, in the Find and Filter bars (`mode_popup` in `terminal.rs`).
- **Cost of the workaround**: the bar loses width and the mode is always
  visible, not only when the menu opens.
- **Proposed kit API**: `SearchField::menu(Vec<MenuItem>)` with a radio for
  the chosen option.
- **Severity**: low
- **How to reproduce**: read `pub struct SearchField` in
  `crates/barrier-kit/src/controls.rs`.

### BK-039 — No outline view nor floating panel
- **Area**: controls / windows
- **What CurseTerm needed**: iTerm2's Find Globally
  (`iTermGlobalSearchWindowController.m`): a 741 × 567 floating panel that
  does not block the windows, with an `NSOutlineView` of sessions and their
  results.
- **What BarrierKit offers today**: a `Table` of flat rows
  (`views/table.rs`), `sheet`, which blocks the window, and `open_window`
  only with patch 0008 (BK-001).
- **What is missing**: an outline view with groups and keyboard selection,
  and a floating auxiliary window (`xdg_toplevel` with a parent and no
  blocking).
- **How I solved it in CurseTerm**: a panel with absolute position at the top
  right of the pane area (`global_search.rs`). Its rows are hand-built, and
  the field's arrows walk the results. It searches only the sessions of its
  window.
- **Cost of the workaround**: the panel covers part of the terminal and
  cannot be moved. It does not search other windows either, which are other
  processes without the patch.
- **Proposed kit API**: `views::outline::Outline` with groups, `selected` and
  `on_select`, and `WindowOptions::utility(parent)`.
- **Severity**: medium
- **How to reproduce**: look for `Outline` or a floating panel in
  `crates/barrier-kit/src`; there is none.

### BK-040 — `PopUpButton` neither scrolls nor searches
- **Area**: controls
- **What CurseTerm needed**: a key binding's action
  (`iTermEditKeyActionWindowController.m`), a searchable list of more than
  thirty actions in groups, and Select Menu Item…, with every menu entry.
- **What BarrierKit offers today**: `PopUpButton` with flat items; its menu
  grows downwards and what does not fit in the window cannot be chosen.
- **What is missing**: scrolling in a long menu, titled sections and a
  search field, like `iTermSearchableComboView`.
- **How I solved it in CurseTerm**: two pop-ups, iTerm2's group and its
  actions (six at most), and for Select Menu Item… a field that filters and
  a pop-up with the first twelve matches (`key_bindings_pane.rs`).
- **Cost of the workaround**: two controls where iTerm2 has one, and the
  filter in a separate field.
- **Proposed kit API**: `PopUpButton::section(title)`, a scrolling menu and
  `PopUpButton::searchable(true)`.
- **Severity**: medium
- **How to reproduce**: a `PopUpButton` with forty items in a 700-high
  window.

## Things that worked well

Checked in phase 1 (headless sway, lavapipe):

- **Startup outside Malaysia:** the runner carries on without
  `org_kde_kwin_appmenu_manager` or the Infinity protocols and logs it with
  `tracing::info!`.
- **High-frequency grid rendering:** with the `canvas` block, `seq 1 300`
  and the colored listing draw with no visible lag in a debug
  build, and at rest there are no frames (1 % CPU).
- **Font metrics and fallback:** Geist Mono from the dev shell and Skia's
  fallback (`match_family_style_character`) for CJK; double widths fit.
- **Fractional scale:** at 1.5 the text is sharp and the cells have no seams.
- **Text clipboard:** copy and paste with other apps (`wl-copy`,
  `wl-paste`) works both ways, also from OSC 52.
- **Live appearance:** changing `appearance.domain` instantly repaints the
  kit's windows (Settings) and CurseTerm.
- **`use_defaults` across processes:** Settings saves the `.domain` and the terminal
  window receives it live.
- **Patterns:** `PreferencesWindow`, `Form`, `Stepper`, `TextField`,
  `PopUpButton` and `about_window` worked unchanged; the sidebar
  collapses by itself below 760 px.
- **Keyboard:** `KeyboardEventData` carries `Key` and enough modifiers for
  an xterm encoder; Alt, Ctrl and the named keys arrive fine.
- **Live menu state:** `MenuHandle::update` publishes the change over D-Bus
  (revision 1 → 3 on selection).
- **Cursor by shape:** `Cursor::set(CursorIcon::Text)` over the grid.

- **Controls for iTerm2-like Settings (phase 3):** `TabView` is
  exactly the icon tab bar of iTerm2's Settings window;
  `Table` with selection, `SegmentedControl`, `Switch`, `Stepper`,
  `PopUpButton` and `Form` covered the profile editor without drawing anything by
  hand.

- **iTerm2's signature features (phase 4):** the `canvas` block drew, without
  extra elements, the broadcast stripes, the mark triangles, the
  trigger highlighting, the bell and the inline images, decoded with
  `Image::from_encoded` from `freya-engine`. `Timers::after` served to turn off
  the bell; `SearchField` with `auto_focus` and `on_navigate` gave the search
  bar; `use_submenu_items` rebuilds the Broadcast
  Input radio group and the host sees it over D-Bus.

Not tested yet: context menu, drag and drop, global menu in the
Malaysia bar, real GNOME and KDE.

## BarrierKit documentation

- `docs/barrierkit/04-apariencia-y-tokens.md:33` says the JSON files generate
  Slint, CSS and QML in `tengen/generated/`; that folder does not exist in `ba58ce0`.
- `docs/barrierkit/01-que-es.md:30` counts «cinco apps de prueba» (five test apps); there are three.
- `docs/barrierkit/11-app.md` does not mention `MenuHandle` or `update`, the only
  way to change the menu live (`app/undo.rs:111` uses it).
- `docs/barrierkit/11-app.md` does not say that a dispatched shortcut also reaches
  the app as a key.
- `README.md` and `docs/barrierkit/11-app.md` say that without
  `org_kde_kwin_appmenu_manager` the menu stays on D-Bus; the code does not
  publish it (BK-018).
- `docs/barrierkit/11-app.md` says an action's handler runs «en el
  hilo de la interfaz» (on the UI thread), but does not warn that it runs outside Freya's context
  (BK-017).
- `docs/barrierkit/07-titlebars.md` says `.close_only(true)` «deja solo el
  botón de cerrar» (leaves only the close button); the code (`traffic.rs:126`) also draws minimize and
  maximize, in gray.
- `docs/barrierkit/16-preguntas.md` invites using Freya's `rect` directly,
  but no chapter says that `Size::flex` only distributes if the parent has
  `.content(Content::flex())`; without it, the first child takes everything (we saw it with
  the panes).
- Nothing documents the sign of `WheelEventData::delta_y`: the runner passes the
  Wayland axis as is (`platform/input/seat.rs:301`), negative upward.
