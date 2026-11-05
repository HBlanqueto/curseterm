# Preliminary study

**Goal:** settle what we need to know about BarrierKit, Malaysia's menu bar
and iTerm2 before writing CurseTerm.
**Scope:** from reading the three repos to the open decisions of phase 1.
It does not describe CurseTerm code, which does not exist yet.
**Reader:** whoever implements CurseTerm or reviews `barrierkit-gaps.md`.

**Inputs (2026-11-05):**

| Repo | Commit | Use |
|---|---|---|
| `barrierkit` | `ba58ce01ef1f8e07dd915f783290b6ef068c037a` (`main`) | Dependency, read only |
| `malaysia` | `8a7eb92` (`main`); pins barrierkit at `9ff264b` | Menu bar, read only |
| `iterm2` | `468c393` (`master`, shallow clone) | Behavior reference, read only |

**Output:** this document and the first version of
[`barrierkit-gaps.en.md`](barrierkit-gaps.en.md).

## 1. Conventions CurseTerm adopts

They come from `barrierkit/AGENTS.md` and `barrierkit/CLAUDE.md`:

1. **Minimal code.** Reuse before writing; nothing "just in case"; no
   comments except `// SAFETY:` and the why of an invariant; minimal diff.
2. **Tests only for risky pure logic**, with the name as the specification
   (`a_resize_sends_sigwinch_with_the_new_grid`).
3. **Design with tokens only.** No color or size literal: everything comes
   from `vow_design` and `Palette::current()`. The exception is the terminal
   scheme colors (ANSI, profile background and ink), which are user data and
   not interface tokens (see §6).
4. **Visible text through Fluent:** `i18n/{en,es}/curseterm.ftl` and `tr!`.
5. **Commits** `tipo(ámbito): …` (`feat`, `fix`, `docs`, `refactor`, `test`,
   `perf`, `chore`, `build`).
6. **Docs in Spanish and English** (`name.md` and `name.en.md`, in the same
   commit), as in barrierkit; code, comments and rustdoc only in English.
7. **ADR** in the kit's format: title `NNNN — …`, "Status", "Context",
   "Decision", "Consequences" and "References".

## 2. How BarrierKit is used

### Startup and model

- An app is a Freya `App` (`render(&self) -> impl IntoElement`) wrapped in
  `AppComponent` and launched with `platform::run_window(app, WindowOptions)`.
  `run_window` is the only entry point and blocks until close.
- The interface is a function of state (`use_state`, `State<T>`); controls
  are controlled (`value`/`on_change`).
- Layers (ADR 0047): 0 platform, 1 primitives, 2 views, 3 patterns and the
  App column. No `prelude`: each piece is imported from its module.
- Freya `=0.4.3`, only `freya-core`, `freya-components` without features,
  `freya-animation`, `freya-engine`, `torin` and `ragnarok`. Never `freya` nor
  `freya-winit`.
- Drawing with Skia on Vulkan (ADR 0056); without Vulkan the window does not
  open (`RunError::Vulkan`). Lavapipe works.
- Freya repaints the whole tree on every frame (no partial damage).

### Useful pieces for a terminal

| Need | Kit piece | Note |
|---|---|---|
| CSD titlebar and traffic lights | `titlebar::Titlebar`, `chrome::WindowChrome` | `left`/`right` zones, title; focus, maximized and full-screen states already handled |
| Window state and commands | `platform::WindowControls::get()` | `toggle_fullscreen`, `minimize`, `close`… |
| Global menu | `app_menu::{AppMenu, Menu}` + `WindowOptions::menu` | dbusmenu + `org_kde_kwin_appmenu_manager` |
| Actions and shortcuts | `app::{Action, Shortcut, use_menu_action}` | The shortcut only arrives if the action is in the menu |
| Live menu state | `app_menu::MenuHandle` (root context) and `MenuHandle::update(id, f)` | Used by `use_undo`; not documented |
| Context menu | `context_menu(fila, items, on_select)` | `MenuView` at the pointer |
| Preferences | `patterns::preferences::PreferencesWindow`, `views::form::*` | Sidebar + search field + `Form` |
| Tabs | `views::tabs::TabView` | Toolbar tabs with `&'static str` titles |
| Splits | `views::split::SplitView` | Sidebar + detail + inspector; not a generic divider |
| Alerts and sheets | `views::feedback::{Alert, sheet}` | Inside the same window |
| "About" | `patterns::about::about_window` | |
| Persistence | `app::{load_defaults, save_defaults, use_defaults, read_toml, write_toml}` | TOML `.domain` in `~/.config/malaysia/<app_id>.domain` |
| Single instance | `app::launch` | D-Bus; forwards the activation |
| Clipboard | `freya_clipboard::Clipboard` | Text only over `wl_data_device` |
| Cursor | `Cursor::set(CursorIcon::Text)` | Requires `wp_cursor_shape_manager_v1` |
| Own font | `register_font(bytes)` | |
| Languages | `barrier_kit::i18n`, `tr!` | |
| Custom drawing | `freya_core::element::ElementExt::render(RenderContext)` | Public in Freya; the kit uses it internally in `glass.rs` |

### Global menu, end to end

1. The app passes an `AppMenu` (a list of `Menu { title, items }`) in
   `WindowOptions::menu`.
2. The runner creates a `MenuHandle`, publishes it on D-Bus as
   `com.canonical.dbusmenu` and, if the compositor offers
   `org_kde_kwin_appmenu_manager`, announces service and path for the
   surface. Without the protocol, the menu exists only on D-Bus
   (`tracing::info!`).
3. A menu click reaches the app through `use_menu_action(|id| …)`, on the
   interface thread.
4. A keyboard shortcut is looked up in the focused window's menu
   (`find_menu_shortcut`) and dispatched like a click. Shortcuts are stored
   as text (`"Ctrl+Shift+S"`); `Action` uses typed `Shortcut`.
5. To change the menu live, the app takes `MenuHandle` with
   `try_consume_root_context::<MenuHandle>()` and calls `update(id, f)`: it
   changes `enabled`, `check`, `label` or the children of a `Submenu` with an
   id. The server emits `LayoutUpdated`.

### Malaysia's menu bar (`malaysia-shell menubar`)

- It reads the remote menu of the focused window (`menubar/dbusmenu.rs`),
  handles `LayoutUpdated`, `ItemsPropertiesUpdated` and `AboutToShow`.
- `MenuSources::compose` (`menubar/remote.rs`) merges the app's menu with
  its own:
  - it always puts the system menu (snowflake) first and then one with the
    app name in bold, with its own slots (About, Settings, Services, Hide,
    Hide Others, Show All, Quit);
  - a remote submenu whose title matches the app name fills those slots by
    label and adds the rest;
  - remote "Window"/"Ventana" and "Help"/"Ayuda" are added to the shell's,
    which already bring window controls and search;
  - the rest goes in between, in order.
- The app name comes from the `.desktop` with that `app_id` or
  `StartupWMClass`; if there is none, from the last segment of the `app_id`
  with an initial capital.
- That is why CurseTerm publishes its "CurseTerm" menu with the same name as
  its `.desktop`, and "Window" and "Help" merge with the shell's.

### Malaysia's fixed shortcuts

`malaysia/docs/configuracion/opciones.md` reserves in the compositor `Super+Q`
(close the app), `Super+W` (close the window), `Super+M`, `Super+H`,
`Alt+Super+H`, `Super+Space`, `Ctrl+Super+F`, `Shift+Super+3/4` and several
`Ctrl+Alt+…`. An app does not receive them.

## 3. What the kit does not offer today (found while reading)

Each point has its entry in [`barrierkit-gaps.en.md`](barrierkit-gaps.en.md);
here only the summary:

1. **One window per process.** `run_window` (`platform/window.rs:346`)
   creates one window; `Runtime::create_window` is `pub(super)`. iTerm2 opens
   several windows in one process and lists them in "Window".
2. **Fixed window title.** There is no live `set_title`; a terminal changes
   the title with OSC 0/2.
3. **Limited, unconsumed shortcuts.** `ShortcutKey` only has `Char`, `Left`
   and `Right` (`app/action.rs:9`); the `Action` title is
   `fn() -> &'static str` (`:66`). When a shortcut fires its action, the key
   still reaches the app (`platform/input/seat.rs:464`).
4. **No IME** (`zwp_text_input_v3`) nor primary selection (pending on Freya
   in ADR 0047). Composed text depends on what xkb delivers in `utf8`.
5. **No document tabs.** `TabView` (`views/tabs.rs:16`) is the toolbar
   selector with static titles; there is no close, "+", reordering or live
   titles like iTerm2's tab bar.
6. **No generic divider.** `SplitView` (`views/split.rs:94`) is sidebar +
   detail; it does not nest or split vertically.
7. **No monospace face in Tengen.** The typefaces in `tokens.json` are all
   `sans`.
8. **No public canvas primitive.** The kit draws by hand only in private
   modules (`glass.rs:176`); an app that needs a grid implements Freya's
   `ElementExt`.
9. **Paragraph cache turned off** (`fonts.rs:20`): each `label` is laid out
   again on every frame.
10. **No drag and drop between apps, no opening URLs, no typed clipboard**
    (pending in ADR 0047).
11. **Dialogs only inside the window** (`sheet`, `Alert`); a separate
    Preferences window depends on point 1.

Contradictions between documentation and code (the code wins):

- `docs/barrierkit/04-apariencia-y-tokens.md:33` cites `tengen/generated/`
  with Slint, CSS and QML; the folder does not exist.
- `docs/barrierkit/01-que-es.md:30` speaks of "five test apps"; the workspace
  has three (`tengen-app-show`, `ondina`, `marea`).
- `MenuHandle::update` is the only way to change the menu live and does not
  appear in the documentation; `replace_menu` exists but only in tests
  (`app_menu.rs:110`).

## 4. iTerm2 inventory

### Menus (`sources/MainMenu/Base.lproj/MainMenu.xib`)

| Menu | Main entries (⌘ = Command, ⌥ = Option, ⇧ = Shift, ⌃ = Control) |
|---|---|
| iTerm2 | About · Preferences… ⌘, · Services · Hide ⌘H · Hide Others ⌥⌘H · Show All · Secure Keyboard Entry · Install Shell Integration · Quit ⌘Q |
| Shell | New Window ⌘N · New Tab ⌘T · New Tab with Current Profile ⌥⇧⌘T · Duplicate Tab · Split Horizontally with Current Profile ⇧⌘D · Split Vertically with Current Profile ⌘D · Save Selected Text… ⌘S · Close ⌘W · Close Terminal Window ⇧⌘W · Close All Panes in Tab ⌥⌘W · Undo Close ⇧⌘T · Broadcast Input ▸ · tmux ▸ · ssh ▸ · Print ▸ |
| Edit | Undo · Redo · Cut · Copy ⌘C · Copy with Styles ⌥⌘C · Copy Mode ⇧⌘C · Paste ⌘V · Paste Special ▸ · Open Selection · Select All ⌘A · Select Output of Last Command ⇧⌘A · Find ▸ (Find… ⌘F, Find Next ⌘G, Find Previous ⇧⌘G, Use Selection for Find ⌘E…) · Marks and Annotations ▸ (Set Mark ⇧⌘M, Jump to Mark ⇧⌘J, Next/Previous Mark ⇧⌘↓/↑) · Clear Buffer ⌘K · Clear Scrollback Buffer ⇧⌘K |
| View | Show Tabs in Fullscreen ⇧⌘U · Toggle Full Screen ⌃⌘F · Toolbelt ▸ · Use Transparency ⌘U · Find Cursor ⌘/ · Show Timestamps ⇧⌘E · Open Quickly ⇧⌘O · Maximize Active Pane · Make Text Bigger ⌘+ · Make Text Normal Size ⌘0 · Make Text Smaller ⌘- · Start Instant Replay |
| Session | Edit Session… ⌘I · Run Coprocess… · Restart Session · Duplicate Session · Open Paste History… ⇧⌘H · Triggers ▸ · Reset ⌘R · Reset Character Set · Move Session ▸ · Log ▸ · Terminal State ▸ · Bury Session |
| Scripts | Python scripts (out of scope) |
| Profiles | Open Profiles… ⌘O · dynamic profile list (⌃⌘1…) |
| Window | Minimize ⌘M · Zoom · Arrangements ▸ · Edit Window Title · Tab ▸ (Select Next/Previous Tab ⇧⌘] / ⇧⌘[, Move Tab Left/Right) · Split Pane ▸ (Select Pane Above/Below/Left/Right ⌥⌘↑↓←→, Next/Previous Pane ⌘] / ⌘[, Move Divider ⌃⌘↑↓←→) · Window Style ▸ · Arrange Windows ▸ · Resize Window ▸ · Password Manager · Pin Hotkey Window · Bring All To Front · dynamic window list |
| Help | iTerm2 Help ⌘? · Copy Mode Shortcuts · Open Source Licenses |

Notes:

- In the xib an uppercase letter implies ⇧ (for example `D` is ⇧⌘D).
- "Profiles" and the window list in "Window" are filled at run time.
- Many entries are hidden or are ⌥ alternates (`isAlternate`); CurseTerm
  does not reproduce them.

### Preferences (`PreferencePanel.xib`)

- Toolbar: **General · Appearance · Profiles · Keys · Arrangements
  · Pointer · Advanced** (and Shortcuts in the current version).
- General: Startup · Closing · Magic · Selection · Window · Settings · tmux ·
  Experimental.
- Appearance: General · Windows · Tabs · Panes · Dimming.
- Profiles (per-profile editor): General · Colors · Text · Window · Terminal ·
  Session · Keys · Advanced.
- Keys: Key Bindings · Navigation Shortcuts · Hotkey · Remap Modifiers.

### Reference code

| Area | `sources/` folder |
|---|---|
| Window, tabs and bar | `iTermController`, `TerminalView`, `TabGroups`, `SplitPanel` |
| Session and PTY | `PTYSession`, `SessionCreation`, `ProcessInfo` |
| Emulation | `VT100`, `VT100Screen`, `LineBuffer`, `ScreenChar` |
| Selection and copy | `Selection`, `CopyMode`, `Pasting` |
| Search | `SearchingFiltering`, `GlobalSearch` |
| Marks and shell integration | `Marks`, `ShellIntegration` |
| Inline images | `InlineImages` |
| Triggers | `Triggers` |
| Broadcast | `InputBroadcasting` |
| Hotkey window | `Hotkey` |
| Profiles and preferences | `ProfilesPanel`, `Settings` |

## 5. Emulation and PTY: candidates

The decision closes in an ADR when phase 1 starts. What there is:

| | `alacritty_terminal` 0.26.0 | `vte` 0.15.0 |
|---|---|---|
| What it is | Parser + grid + scrollback + selection + regex search + PTY (`tty`, with `rustix-openpty`) + I/O loop | Only the sequence parser (Paul Williams' state machine) |
| What would have to be written with it | Rendering, input, own OSCs (1337, 133) | Grid, scrollback, reflow, selection, search, PTY |
| Risk | Semi-stable API, designed for Alacritty; unknown OSCs are ignored | Reimplementing what the brief asks not to reinvent |

Leaning: `alacritty_terminal`, with its `tty` for the PTY (it covers
`TIOCSWINSZ`/`SIGWINCH` on resize), and a separate `vte::Parser` in parallel
only for the OSCs that Alacritty ignores (1337, 133) in phase 4.
`portable-pty` is unnecessary if `alacritty_terminal::tty` is enough.

## 6. Open decisions for phase 1

1. **Kit dependency.** The barrierkit flake exports neither `lib` nor an
   overlay; only `packages.skia-binaries`. Plan: `barrierkit` as an input of
   the CurseTerm flake (to reuse `skia-binaries` and the dev shell) and the
   crates as a git dependency in `Cargo.toml` with the same `rev`, as
   malaysia does (ADR 0048 §2).
2. **Shortcut scheme.** Leaning: **Super** as ⌘. Reasons:
   - Malaysia's shell already translates ⌘ to Super in its menus (Super+C,
     Super+V, Super+Q, Super+H) and its compositor reserves Super+Q/H/M just
     like macOS;
   - a terminal does not send Super to the shell, so no app shortcut steals
     a combination from programs like `vim` or `emacs`, which does happen
     with Ctrl+Shift in CSI u modes.
   Costs: Malaysia reserves Super+W (close window) and GNOME reserves
   Super+A, Super+H, Super+L, Super+V, etc. It is measured in phase 1 and
   documented in the ADR.
3. **App-id.** `org.curseterm.CurseTerm`. Malaysia only treats as "its own" a
   window with the `io.github.hblanqueto.malaysia.` prefix (ADR 0035); with
   the `force_server` policy it would add a second titlebar. With the default
   policy it respects the CSD the kit requests. It is noted as an observation
   about the desktop, not the kit.
4. **Terminal colors.** The ANSI palette and the background are profile data
   (like `.itermcolors`), not interface tokens. They go in the user's profile
   and in schemes bundled as data; the surrounding interface (titlebar, tabs,
   preferences) uses only `Palette` and tokens. We must check that the kit's
   `scripts/check-design-tokens.sh` does not apply to CurseTerm or justify the
   exception in an ADR.
5. **Grid.** A custom element (`ElementExt`) that draws the grid with Skia:
   one background rectangle per color run and one `TextBlob` per style run,
   instead of one `label` per cell.

## 7. Environment of this work

- There is no graphical session in the container. Testing needs a headless
  compositor (Weston or headless sway, as in
  `docs/barrierkit/14-plataforma.md`) and software Vulkan (lavapipe).
- barrierkit's `nix develop` does not finish here: it builds `skia-binaries`
  from source (`nix/skia.nix`) and needs downloads from GitHub.
  1. The `fetchgit` calls failed on TLS (the proxy re-signs); this is solved
     with `NIX_GIT_SSL_CAINFO=/root/.ccr/ca-bundle.crt` and
     `--option extra-sandbox-paths /root/.ccr`.
  2. The tarball `github.com/rust-skia/skia/archive/refs/tags/m148-0.97.0.tar.gz`
     returns **403**: the environment's network policy does not allow
     downloading archives or releases from `github.com` (it does allow
     `git clone`). Without Skia, `freya-skia-bindings` does not build, and so
     neither does the kit.
  Until `github.com` (and `codeload.github.com`) are allowed in the
  environment's network, nothing that depends on the kit can be built or run
  here. Phase 1 notes it in `progreso.md` and declares nothing tested that
  has not been run.
- malaysia's `dev.sh lab` also needs Vulkan, a private D-Bus bus and the
  nested compositor; it will be tested in phase 1.
