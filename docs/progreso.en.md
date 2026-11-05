# Progress

**Goal:** record the plan of each phase before it starts and, when it
closes, what works, what does not and what differs from iTerm2.
**Reader:** anyone who follows the project or reviews a phase.

A phase closes when it builds, passes `./dev.sh check` (fmt, clippy with
`-D warnings` and tests) in both kit modes (ADR 0001), has been tested by
hand and its gaps are in [`barrierkit-gaps.en.md`](barrierkit-gaps.en.md).
Whatever could not be run is stated here; nothing is taken for granted.

## Phase 1 — Core

### Plan

1. **Workspace.** `Cargo.toml` with `crates/curseterm` (the app) and
   `crates/barrier-kit-next` (blocks, ADR 0001); `flake.nix` with barrierkit
   as an input (ADR 0004); `dev.sh` with `check`, `run`, `kit-parches` and
   `kit-tal-cual`.
2. **`canvas` block** (BK-009) in `barrier-kit-next`: an element that gives
   the Skia `Canvas`, the scale and the font collection, and redraws when
   its revision changes.
3. **Session.** `alacritty_terminal` (ADR 0002): the user's shell (`$SHELL`,
   else `/etc/passwd`), `TERM=xterm-256color`, `COLORTERM=truecolor`; I/O
   thread with `WakeHandle`; title through OSC 0/2.
4. **Grid.** Drawing by runs: one background per color run, text per style
   run; ANSI, 256 and truecolor colors; bold, italic, underline, inverse;
   block, bar and underline cursor; scrollback with the wheel.
5. **Input.** Keyboard encoder (text, Ctrl, Alt as ESC, arrows and cursor
   modes, F1–F12, Home/End, PgUp/PgDn); Super never reaches the PTY
   (ADR 0003); Ctrl+Shift+C/V.
6. **Selection and clipboard.** Drag (simple), double click (word), triple
   click (line); copy and paste with the kit's `Clipboard`; bracketed paste
   if the program asked for it.
7. **Resize.** When the canvas size changes, the grid is recomputed and
   `Term::resize` and the PTY's `OnResize` (`SIGWINCH`) are called.
8. **Configuration.** `~/.config/malaysia/org.curseterm.CurseTerm.domain`
   (`load_defaults`/`use_defaults`): font family and size and color scheme.
   Schemes are bundled as data.
9. **Window.** The kit's `Titlebar` with the session title; minimal tabs so
   that "New Tab" does something real (the full iTerm2 bar belongs to
   phase 2).
10. **Minimal global menu** with `Action` and Super shortcuts:
    - CurseTerm: About CurseTerm, Preferences…, Quit CurseTerm.
    - Shell: New Window, New Tab, Close.
    - Edit: Copy, Paste, Select All.
11. **Platform workarounds** in "kit as is" mode: one window per process
    (New Window launches another process; Quit closes the CurseTerm
    processes registered in `$XDG_RUNTIME_DIR/curseterm/`); Preferences in a
    separate process that writes the `.domain`, which the window watches
    with `use_defaults`.
12. **Patches** in `kit-parches/`: live title (BK-002) and shortcuts consumed
    with `ShortcutKey::Named` (BK-003).
13. **Test** in a headless compositor with lavapipe; screenshots in
    `docs/capturas/`; and in `malaysia/dev.sh lab` if the environment allows.

### Result (2026-11-05)

**How it was tested.** Everything ran in the container, without a GPU:

1. `nix develop` (the kit's dev shell plus `CURSETERM_DEV_SHELL`) with Vulkan
   through lavapipe.
2. Headless sway 1.12 (`WLR_BACKENDS=headless WLR_RENDERER=pixman`) and a
   private D-Bus session bus.
3. `wtype` for the keyboard, `grim` for screenshots, `wl-copy`/`wl-paste`
   for the clipboard, and a minimal `zwlr_virtual_pointer_v1` client for the
   mouse (`wlrctl` creates a new pointer on each command and the seat is
   left without a pointer between one and the next).
4. `./dev.sh check` clean in both kit modes (ADR 0001).

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| User shell in the PTY, prompt and cursor | ✓ | `fase1-colores-claro.png` |
| 16 ANSI colors, 256 cube, truecolor | ✓ | `fase1-colores-claro.png` |
| Bold, italic, underline, inverse, strikethrough, dim | ✓ | same |
| Font fallback: ñ, double-width CJK | ✓ | same |
| Light or dark scheme from `appearance.domain`, live | ✓ | `fase1-colores-oscuro-y-ajustes.png` |
| Resize: `stty size` gives 39×136 at 1100×700 and 27×98 at 800×500 | ✓ | — |
| Scrollback with the wheel | ✓ | `fase1-scrollback.png` |
| Selection by drag, double click (word) and triple click (line) | ✓ | `fase1-seleccion-linea.png` |
| Copy (Super+C, Ctrl+Shift+C) and paste (Super+V) with the system clipboard | ✓, checked with `wl-paste` and `wl-copy` | — |
| Select All (Super+A) | ✓ | — |
| New Tab (Super+T), titles through OSC, Close (Super+W) | ✓ | `fase1-pestanas.png` |
| New Window (Super+N) in a new process | ✓ | `fase1-dos-ventanas-foco.png` |
| Hollow cursor in the unfocused window | ✓ | same |
| Quit (Super+Q) closes all windows | ✓ | — |
| About and Settings… in their window | ✓ | `fase1-acerca-de.png` |
| Changing the font size in Settings changes the terminal live | ✓ | `fase1-ajustes-fuente-en-vivo.png` |
| Fractional scale 1.5 | ✓, no seams | `fase1-escala-1-5.png` |
| At rest there are no new frames | ✓, 1 % CPU | — |
| Patch 0001: live window title | ✓, sway shows the session title | `fase1-titulo-en-vivo-parcheado.png` |
| Patch 0003: menu on D-Bus without `org_kde_kwin_appmenu_manager` | ✓ `GetLayout` gives CurseTerm, Shell and Edit with shortcuts; Copy becomes enabled on selection; `Event clicked` on New Tab opens the tab | — |

**Not tested:**

- **Global menu in Malaysia** (`malaysia/dev.sh lab`): the malaysia dev
  shell builds `libadwaita` 1.9.3 and its `test-animation-target` test
  aborts in the Nix sandbox. Without Malaysia there is no
  `org_kde_kwin_appmenu_manager`, and with the kit as is the menu is not even
  published on D-Bus (BK-018). The menu was tested over D-Bus with patch
  0003, not in the menu bar.
- **Malaysia's fixed shortcuts** (Super+W, Super+Q): could not be tested.
- **IME, dead keys and primary selection:** the kit does not offer them
  (BK-004, BK-005).
- **Application mouse mode** (`vim`, `htop` with mouse): not implemented;
  the wheel on the alternate screen sends arrows.
- **Real GNOME and KDE.**
- **First key of each `wtype`:** the first key of each invocation is lost
  (each one uploads its own keymap). It was not isolated whether this comes
  from the kit or from the tool; the tests send an Escape first.

**Differs from iTerm2:**

- "Preferences…" is named "Settings…" ("Ajustes…" in Spanish), to fill the
  "Settings" slot of Malaysia's bar.
- Shortcuts with Super (ADR 0003). In Malaysia, Super+W closes the whole
  window.
- The tab bar only appears with two or more tabs, as in iTerm2, but it has
  no "+", reordering or activity indicator.
- Emoji comes out monochrome (there is no color font in the environment) and
  box-drawing characters come from the fallback font and do not fit the cell;
  iTerm2 draws them itself.
- Each window is a process (BK-001): tabs do not move between windows.
- The "Window" menu and the window list belong to phase 2.

**Bugs found and fixed while testing:** menu handlers that asked for Freya
context (BK-017), the wheel sign, a double borrow of `State` on close, and
repainting on focus, font or scheme change.

## Phase 2 — Organization

### Plan

1. **Pane model** (`crates/curseterm/src/panes.rs`, pure logic with tests):
   a split tree with ratios; split next to the active pane, close and
   collapse, walk in order, neighbor above, below, left and right by
   geometry, and move the divider.
2. **`views::split_panes` block** (BK-007) in `barrier-kit-next`: a container
   of N children on one axis, nestable, with draggable dividers; controlled
   (`ratios` and `on_ratios`).
3. **Each pane measures its session:** each pane's PTY follows its own size
   (`SIGWINCH` per pane); the active pane receives the keyboard and is
   chosen with a click.
4. **Tab bar** (BK-006): "+" button, next and previous tab, move left and
   right, Super+1…9, and "Show Tab Bar" to see it with a single tab.
5. **Full menus** for Shell, View and Window, with live states (checkmarks,
   enabled by context) and the dynamic "Select Tab" submenu, on the
   `app_menu` block (BK-010):
   - Shell: New Window, New Tab, Duplicate Tab, Split Vertically/Horizontally
     with Current Profile, Close, Close Terminal Window, Close All Panes in Tab.
   - View: Show Tab Bar, Toggle Full Screen, Maximize Active Pane, Make Text
     Bigger, Make Text Normal Size, Make Text Smaller.
   - Window: Minimize, Zoom, Select Next/Previous Tab, Move Tab Left/Right,
     Select Tab ▸, Split Pane ▸ (Select Pane Above/Below/Left/Right, Next/
     Previous Pane, Move Divider Up/Down/Left/Right).
6. **Window list in "Window":** each window is a process (BK-001) and an app
   cannot request an activation token to bring another to the front (only
   the `shell` feature can). With the kit as is it is not shown; it is tried
   as a patch if the rest of the phase fits.
7. **Test** in headless sway as in phase 1, and over D-Bus with patch 0003
   for the menu.

### Result (2026-11-05)

Tested as in phase 1 (headless sway, lavapipe, `wtype`, the scratchpad's
virtual pointer and the menu over D-Bus with patch 0003). `./dev.sh
check` clean in both kit modes; 17 app tests and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Split Vertically (Super+D) and Horizontally (Super+Shift+D), nested | ✓; each pane has its PTY (`stty size` gives 14×53 in a quarter) | `fase2-paneles.png` |
| Select Pane Left/Right/Above/Below (Super+Alt+arrows), Next/Previous Pane (Super+] and Super+[) | ✓ | — |
| Move Divider (Ctrl+Super+arrows) | ✓, the pane reflows the text | `fase2-divisor-movido.png` |
| Drag the divider with the mouse; click to activate a pane | ✓ | `fase2-divisor-arrastrado.png` |
| Maximize Active Pane (Super+Shift+Return), with checkmark | ✓ | `fase2-panel-maximizado.png` |
| Close (Super+W) closes the pane; the last one closes the tab and then the window | ✓ | — |
| Close All Panes in Tab (Super+Alt+W) | ✓, only enabled with splits | — |
| Tab bar with "+" | ✓ | `fase2-barra-de-pestanas.png` |
| Select Next/Previous Tab (Super+Shift+} and {), Super+1…9, Move Tab Left/Right | ✓ | — |
| "Select Tab" ▸ with the tab list and a radio mark on the active one | ✓ over D-Bus | — |
| Show Tab Bar with a single tab, with checkmark | ✓ | `fase2-mostrar-barra.png` |
| Duplicate Tab opens in the active session's directory | ✓ | `fase2-duplicar-y-texto-mayor.png` |
| Make Text Bigger/Smaller/Normal Size (Super+=, Super+-, Super+0) | ✓, the PTY resizes | same |
| Toggle Full Screen (Ctrl+Super+F) | ✓ in sway | — |
| Live states over D-Bus: enabled/disabled by tabs and splits, checkmarks | ✓ | — |

**Not tested:**

- **Minimize and Zoom:** sway does not minimize and does not maximize in
  tiling; `WindowControls::minimize` and `toggle_maximize` are called, with
  no visible effect here.
- **Malaysia:** it still cannot be built (phase 1). In Malaysia, Super+M and
  Super+W are handled by the compositor.

**Differs from iTerm2:**

- **Window list in "Window":** missing. Each window is a process (BK-001)
  and an app cannot request an activation token to bring another to the
  front (BK-019), so such an item would do nothing. In Malaysia the bar
  already adds the app's window list to the "Window" menu.
- **Closing a pane** activates the first one in the tree, not the neighbor.
- **Inactive panes** are not dimmed; only their cursor turns hollow.
- **Shortcuts with Shift:** they are declared with the produced character
  (`Super+Shift+}`) because of BK-016; on a non-US keyboard they may not
  match.
- **Make Text Bigger** shows as Super+= and not ⌘+.
- **Missing:** Undo Close, Edit Tab Title, moving tabs between windows and
  dragging them.

## Phase 3 — Profiles and preferences

### Plan

1. **Profiles** in the app's `.domain`: name, font family and size, color
   scheme (or "Follow the system"), initial command and directory (home, the
   previous session's or a fixed one), and which profile is the default.
   With no saved profiles there is one, "Default".
2. **Each session with its profile:** the command and directory at startup;
   the font and scheme when drawing. New splits and tabs use the active
   session's profile ("with Current Profile").
3. **Importable schemes** from `.itermcolors` (XML plist) with the `plist`
   crate. Imported schemes are saved in the same `.domain`, so they reach
   every window live. They are imported from Preferences (path in a field,
   because the kit has no file picker) or with
   `curseterm --import-colors <archivo>`.
4. **Preferences** with iTerm2's structure built with `PreferencesWindow`:
   "Appearance" (tab bar with a single tab) and a "Profiles" group with one
   pane per profile, whose sections follow the General, Colors and Text tabs
   of iTerm2's profile editor; duplicate, delete and choose the default
   profile.
5. **"Profiles" menu** with one item per profile, which opens a tab with it
   (Ctrl+Super+1…9). The kit does not allow changing the items of a
   top-level menu: with the kit as is the labels update and the new list
   appears in the next window; a patch (0004) adds `MenuHandle::set_menu` to
   change it live.
6. **Test** in headless sway, as in previous phases.

### Changes to the plan

With the iTerm2 screenshots the user sent, Settings does not use
`PreferencesWindow` (sidebar) but what iTerm2 does: toolbar tabs at the top
(`TabView`) and, in Profiles, the profile list (`Table`, ★ on the default,
+ and −) next to a `SegmentedControl` with General, Colors and Text. The
menus were also reordered after those screenshots: Shell with "with Current
Profile", Session (Restart Session, Reset), Edit with Clear Buffer and Clear
Scrollback Buffer, and the "Tab ▸" submenu in Window.

### Result (2026-11-05)

Tested in headless sway as in previous phases. `./dev.sh check` clean in
both kit modes; 20 app tests and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `curseterm --import-colors 'Solarized Dark.itermcolors'` (taken from iTerm2's `ColorPresets.plist`) | ✓; a file that is not a plist exits with an error and code 1 | — |
| Settings: Appearance and Profiles tabs; list, ★, + and −; General, Colors and Text | ✓ | `fase3-ajustes-perfil-general.png`, `fase3-ajustes-perfil-colores.png` |
| Choose an imported scheme; the terminal (another process) changes live | ✓ with the terminal focused; unfocused, only with patch 0005 (BK-022) | `fase3-repintado-sin-foco-parche-0005.png` |
| "+" creates a profile; the terminal's "Profiles" menu shows it at once | ✓ with patch 0004; with the kit as is it appears in the next window | — |
| Ctrl+Super+2 opens a tab with that profile and its scheme | ✓ | `fase3-pestana-con-otro-perfil.png` |
| Split with Current Profile inherits the profile | ✓ | `fase3-division-con-el-perfil.png` |
| Profile command and fixed directory | ✓ | `fase3-orden-directorio-y-restart.png` |
| Restart Session (new shell, same profile and directory) | ✓ | same |
| Clear Buffer (Super+K) keeps the prompt line; Clear Scrollback Buffer is enabled with history | ✓ | `fase3-clear-buffer-y-reset.png` |
| Reset (Super+R) after breaking the character set with `ESC (0` | ✓ | same |
| Appearance › "Show tab bar even when there is only one tab" and the View › Show Tab Bar checkmark in the terminal | ✓ | `fase3-ajustes-apariencia.png` |

**Not tested:** renaming a profile with open sessions (sessions with the old
name fall back to the default profile); the "Directory path" row with the
keyboard; New Window with Current Profile (it launches
`curseterm --profile NOMBRE`; `--profile` was tested only through that menu
command, without a screenshot).

**Differs from iTerm2:**

- Settings only has Appearance and Profiles: General, Keys, Arrangements,
  Pointer, Shortcuts and Advanced have nothing real behind them yet.
- The profile editor has General, Colors and Text; Window, Terminal,
  Session, Keys and Advanced are missing, and General lacks Shortcut key,
  Tags, Badge, Title, Icon and Send text at start.
- "Command" is a field (empty = login shell) and not a "Login Shell /
  Command" dropdown.
- Import asks for the path in a field: the kit has no file picker
  (BK-023). `Calibrated` colors in a `.itermcolors` are taken as sRGB.
- "Show Tab Bar" is in View; in current iTerm2 it only exists in Appearance.
- The Settings window has no search field.

## Phase 4 — What makes iTerm2 iTerm2

### Plan

1. **Own I/O loop** (ADR 0005) with a splitter for OSC 7, 133 and 1337.
2. **Shell integration:** current directory through OSC 7 and
   `1337;CurrentDir` (used by Duplicate Tab and "Reuse previous session's
   directory"); marks through OSC 133;A and `1337;SetMark`, with iTerm2's
   triangle in the margin, Set Mark, Jump to Mark, Next and Previous Mark.
3. **Find** (Edit › Find ▸): a search bar in the pane with highlighting,
   Find Next, Find Previous and Use Selection for Find, with
   `alacritty_terminal`'s regex search.
4. **Broadcast input** (Shell › Broadcast Input ▸): only the current
   session, all panes in the tab or in all tabs, with an indicator on the
   panes that receive.
5. **Inline images** (`1337;File=…;inline=1`), decoded with Skia.
6. **Triggers** per profile (regex and action: highlight, bell or send
   text), editable in Settings and toggled from Session › Triggers ▸.
7. **Hotkey window:** Wayland has no global shortcuts except the
   `GlobalShortcuts` portal, which Malaysia's portal does not implement and
   the kit does not use. It is noted as a gap and `curseterm --hotkey` is
   offered, which opens or closes a dedicated window, to bind it to a
   desktop shortcut.
8. **Test** in headless sway, as in previous phases.

### Result (2026-11-05)

Tested in headless sway as in previous phases. `./dev.sh check` clean in
both kit modes; 26 app tests and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Own I/O loop: shell, `seq`, colors and resize as before | ✓ | — |
| 40 prompts with `OSC 133;A`: blue triangle in the margin of each one | ✓ | `fase4-marcas-e-imagen.png` |
| Previous Mark (Super+Shift+↑) three times: from mark 27 to 24 | ✓ | — |
| Splitting the pane reflows long lines and marks stay on their prompt | ✓ after converting marks to logical lines (`io::reflow`) | `fase4-broadcast.png` |
| `1337;File=inline=1;width=20` with a PNG: 20 cells wide and the text continues below | ✓ | `fase4-marcas-e-imagen.png` |
| `OSC 7` with `%20`: a new tab with "Reuse previous session's directory" opens in `/tmp/con espacio` even though the shell was in `~` | ✓ | — |
| Find (Super+F): search-as-you-type upward; Enter and ↑ the previous, ↓ the next; Escape closes | ✓ | `fase4-buscar.png` |
| Broadcast Input to All Panes in All Tabs (Super+Alt+I): stripes on both panes and `echo` in both | ✓ | `fase4-broadcast.png` |
| Profile triggers: highlight `error [0-9]+`, "Send Text" `si\n` on `¿seguir?`, bell with `^campana$` | ✓; the bell lightens the pane (pixel `#002B36` → `#1A4049`) | `fase4-triggers.png` |
| Settings › Profiles › Triggers: list, action, text, "−" and "+", saved in the `.domain` | ✓ | `fase4-ajustes-triggers.png` |
| Menu over D-Bus (patches 0003 and 0004): Broadcast Input radio, Jump/Next/Previous Mark enabled after Set Mark, Enable Triggers unchecks | ✓ | — |
| `curseterm --hotkey` opens the window; another `--hotkey` closes it and exits with 0; a third opens it again | ✓ | — |

**Bugs found and fixed while testing:** after closing the search with
Escape, focus stayed marked and the terminal ignored the keyboard; the line
reflow on split moved marks by one line; `Timers::get()` in the bell
callback aborted the app (BK-017); `^…$` did not match with
`alacritty_terminal`'s stream search, which includes the trailing blanks of
the row: triggers now use the `regex` crate on the row text.

**Not tested:** images with `preserveAspectRatio=0` and in percent;
`1337;SetMark` and `1337;CurrentDir` (only their tests); Use Selection for
Find; Edit Triggers… (opens Settings like Settings…); a dbusmenu `Event`
sent without waiting for a reply (`dbus-send` without `--print-reply`) did
nothing twice, with a reply it did; the cause was not investigated.

**Differs from iTerm2:**

- Marks and images are anchored to absolute lines: when the history reaches
  its limit and starts dropping lines, they shift.
- Search highlights only the current match (with the selection color),
  without counting matches or case or regex options; it is case-sensitive
  only if the query has uppercase letters.
- Triggers look at each full row, not the logical line, and lack the
  "Instant" mode for the prompt line; only three actions.
- The bell is visual (BK-026); the hotkey window closes instead of hiding
  (BK-025).
- Broadcast lacks "Toggle Broadcast Input to Current Session" and iTerm2's
  warning when turning it on.

## Phase 5 — Terminal profile, bell and complete Find

### Plan

1. **Profile › Terminal** in Settings: "Scrollback lines" (1000 by default,
   like iTerm2) and, for the bell, "Flash visual bell" and "Show bell icon in
   tabs". They apply live to open sessions (`Term::set_options`). No
   "Unlimited scrollback": `alacritty_terminal`'s `Grid::clear_history`
   reallocates every row up to the limit when clearing, so the limit is
   finite.
2. **Bell in a background tab:** the tab shows an icon until it is selected.
   The kit has no bell symbol (BK-024); the `TabBar` block gains a per-tab
   indicator.
3. **Complete Find:** every visible match highlighted, the current one with
   the selection, and "n of m" or "No matches" in the bar.
4. **Profile › Advanced:** triggers move to the Advanced section, where
   iTerm2 keeps them.
5. **Test** in headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway as in the previous phases. `./dev.sh check` is clean
in both kit modes; 27 app tests and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Find "orden 3" after 40 commands: the 11 visible matches highlighted, the current one with the selection and "10 of 11" | ✓ | `fase5-buscar-todas.png` |
| Find "2" after `seq 1 30`: "13 of 13" (`22` counts twice) | ✓ | — |
| `scrollback_lines = 50` written to the `.domain` with the window open; after `seq 1 500` the oldest line is 416, 50 lines above the first visible one | ✓ | — |
| `printf '\a'` in a background tab: a badge on that tab, which goes away when it is selected | ✓ | `fase5-campana-en-pestana.png` |
| "Show bell icon in tabs" off: no badge | ✓ | — |
| Settings › Profiles › Terminal: scrollback lines and both switches, saved to the `.domain` | ✓ | `fase5-ajustes-terminal.png` |
| Settings › Profiles › Advanced with the triggers | ✓ | `fase5-ajustes-avanzado.png` |

**Not tested:** "Flash visual bell" off (the branch is one condition, no
screenshot); a scrollback of 0 lines; the 100,000 limit with a larger value in
the field.

**Differs from iTerm2:**

- No "Unlimited scrollback" or "Silence bell": there is no audible bell
  (BK-026).
- The badge uses `SpeakerWave` instead of a bell (BK-024).
- Find has no case or regular expression options, and the count is computed
  when searching, not when new text arrives.
- The profile still lacks Window, Session and Keys.

## Phase 6 — Window, Session and Keys profile

### Plan

1. **Window:** "Columns" and "Rows" (80 × 25 by default, like iTerm2) for
   new windows. The kit always restores the last saved size and cannot ask
   for another (BK-012): the new window forgets the saved size and computes
   its own from the profile's font metrics.
2. **Session:** "When the session ends" (close, restart or stay open with a
   notice) and "Prompt before closing" (always, never or if there are
   processes besides the shell), with the kit's `Alert` in Shell › Close and
   when closing a tab.
3. **Keys:** "Alt key acts as" Normal or Esc+. On Linux AltGr composes
   characters and does not arrive as Alt, so there is one option and not one
   per side as in iTerm2.
4. **Test** in headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway, with CurseTerm's windows floating so the compositor
honors their size. `./dev.sh check` is clean in both kit modes; 27 app tests
and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| New window with the default profile: `tput cols` and `tput lines` give 80 and 25 | ✓ after measuring the titlebar with `TITLEBAR_CLIENT_HEIGHT` (with `TITLEBAR.thickness` it gave 24 rows) | `fase6-ventana-80x25.png` |
| `sleep 100` and Super+W: "Close?" alert; Escape cancels it without sending `^[` to the shell; Enter closes | ✓ after deferring the alert's dismissal (BK-027) | `fase6-preguntar-al-cerrar.png` |
| Super+W on a session that already ended, with no processes: closes without asking | ✓ | — |
| "Keep open": `exit` leaves the session with "[Process completed]" | ✓ | `fase6-sesion-terminada.png` |
| "Restart": `exit` opens a new shell in the same pane | ✓ | — |
| Alt "Normal": Alt+B in `cat -v` gives `b`; "Esc+" gives `^[b` | ✓ | — |
| Settings › Profiles with eight sections; Window, Session and Keys | ✓ with the window at 980 px (BK-028) | `fase6-ajustes-sesion.png` |

**Not tested:** "Prompt before closing" on Close All Panes, Close Window and a
tab's "×" (they share `request_close`, no screenshot); "Always" and "Never";
changing Columns and Rows from the field.

**Differs from iTerm2:**

- One option for Alt, not one per side; AltGr composes.
- "Prompt before closing" lacks iTerm2's list of allowed processes: any
  foreground group other than the shell counts. Quit and the close requested
  by the compositor do not ask (BK-013).
- No window remembers its size or whether it was maximized (BK-012).
- Window has no style, transparency or screen; Keys has no per-profile
  shortcuts.

### Review (2026-11-05)

A code review of phases 4 to 6 found ten problems, all fixed:

- A pending close stored the tab by index; it now stores one of its
  sessions.
- "Restart" with a command that exits at once looped; if the session lasts
  less than a second it stays open with a notice.
- The "n of m" count scanned the whole scrollback on every key; it is now
  computed after a `DURATION_FAST_MS` pause, and the compiled search is
  reused while the query does not change.
- With a profile command (`sh -c`), "Prompt before closing" did not see the
  process; it now counts as a job while it lives.
- The window size did not clamp the font size like the pane; both use
  `Profile::text_size`.
- A session that ended no longer takes keys or fills the log with warnings.
- Columns, Rows and Scrollback accept only digits; 0 columns or rows is
  saved as 1.
- Two two-line comments become one line each.

Testing the first item showed another bug: after closing a tab from the
alert, the remaining pane still showed the closed session and kept it alive,
because the canvas key did not include the session. It does now. Tested on
sway: `false` with "Restart" leaves the notice without creating processes;
the "×" of a tab running `sleep 100` asks, closes that tab and ends `sleep`;
typing into an ended session leaves no warnings; "13 of 13" arrives after the
pause.

## Phase 7 — Last command output, Copy Mode and Find Cursor

### Plan

1. **Edit › Select Output of Last Command (⇧⌘A):** shell integration marks
   where the output starts (OSC 133;C) and where the command ends (OSC 133;D
   or the next prompt). The command selects those lines. Without
   integration, the item stays disabled.
2. **Edit › Copy Mode (⇧⌘C):** a keyboard cursor that moves and selects
   without the mouse. It builds on the vi mode of `alacritty_terminal`:
   h j k l and arrows, w b e, 0 ^ $, H M L, g and G, Ctrl+B and Ctrl+F; v or
   space to select characters and V for lines; y copies and leaves; Escape
   or q leave.
3. **View › Find Cursor (⌘/):** dims the pane except for a circle around the
   cursor for `DURATION_SLOW_MS`.
4. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with a minimal bash integration: `PS0` emits OSC
133;C and `PROMPT_COMMAND` emits 133;D and 133;A. `./dev.sh check` clean in
both kit modes; 28 app tests and 1 block test. No new feature needed a kit
gap.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `seq 1 5`, `echo hola` and ⇧⌘A with ⌘C: the clipboard holds `hola` | ✓ | — |
| ⇧⌘C, k k k, v, j and $: the selection runs from the copy cursor to the end of the line; y copies and leaves; the next x reaches the shell | ✓ | `fase7-modo-copia.png` |
| ⇧⌘C, g, V, j and y: copies the first two whole lines | ✓ | — |
| ⇧⌘C, k and Escape: leaves without writing anything; the next z reaches the shell | ✓ | — |
| ⌘/: the pane dims except for a circle at the cursor and goes back to normal after `DURATION_SLOW_MS` | ✓ | `fase7-buscar-cursor.png` |

**Not tested:** the "Copy Mode" item checked in the global menu and "Select
Output of Last Command" disabled without integration (the test used the kit
as is, with no menu on D-Bus, BK-018); Ctrl+B and Ctrl+F in copy mode; the
last command's output after a resize that reflows the lines.

**Differs from iTerm2:**

- Copy Mode has no rectangular selection (Ctrl+V) and no search inside the
  mode, and its keys cannot be changed.
- Find Cursor is a fixed circle that lasts `DURATION_SLOW_MS`; iTerm2
  animates the spotlight and keeps it while ⌘/ is held.
- The output only ends with OSC 133;D. The plan also counted on the next
  prompt (133;A), but 133;A and `SetMark` arrive as the same mark; a shell
  that does not emit 133;D leaves the item disabled.

## Phase 8 — Undo Close and paste history

### Plan

1. **Shell › Undo Close (⇧⌘T):** when a tab or pane is closed from the menu,
   the keyboard or the "×", its sessions stay alive for five seconds. Undo
   Close brings them back as a new tab, with their processes and title. A
   session whose process ended cannot be undone.
2. **Session › Open Paste History (⇧⌘H):** CurseTerm remembers the latest
   texts copied and pasted in the window. A kit sheet (`sheet`) with a
   `SearchField` that filters and a `Table` with the entries: the arrows
   choose, Enter pastes and Escape closes.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway. `./dev.sh check` clean in both kit modes; 30 app
tests and 1 block test. The history sheet reuses the kit's `sheet`,
`SearchField` and `Table`; no new gap was needed, and Enter defers the close
like the close alert (BK-027).

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| New tab, `X=vivo`, Super+W and ⇧⌘T: the tab comes back and `echo $X` prints `vivo` | ✓ | `fase8-deshacer-cierre.png` |
| Super+W and a six-second wait: the closed shell ends and ⇧⌘T does nothing | ✓ | — |
| Split pane with `Y=panel`, Super+W and ⇧⌘T: it comes back as a new tab and `echo $Y` prints `panel` | ✓ | — |
| Paste `echo alfa` and `echo beta`, ⇧⌘H, type `alf` and Enter: pastes `echo alfa` without sending Enter to the shell | ✓ | `fase8-historial-de-pegado.png` |
| ⇧⌘H and Escape: closes without sending `^[` to the shell | ✓ | — |

**Not tested:** the arrows inside the history; the enabled state of Undo
Close and Open Paste History in the global menu (BK-018); undoing a tab
closed from the "Close?" alert.

**Differs from iTerm2:**

- A closed pane comes back as a new tab, not in its place in the split.
- Only the last close can be undone, and the five-second window cannot be
  changed.
- Closing the last tab closes the window and cannot be undone (one process
  per window, BK-001).
- The history lives in the window: it is not shared between windows, not
  saved to disk, and does not show when each entry was copied. It keeps the
  20 most recent texts.

## Phase 9 — Editable titles and Help menu

### Plan

1. **Window › Edit Tab Title and Edit Window Title:** a kit sheet with a
   `TextField`. The typed title replaces the one the shell sets; an empty
   field goes back to the automatic one. Enter accepts and Escape cancels.
2. **Help › Copy Mode Shortcuts:** a sheet with a `Table` of keys and
   actions. The table comes from the same list copy mode uses, so they
   cannot drift apart.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with the patched kit, to trigger the items without a
shortcut from the D-Bus menu (patch 0003). `./dev.sh check` clean in both kit
modes; 31 app tests and 1 block test. One new gap: BK-029, `TextField`
without `auto_focus`, solved with patch 0006.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Edit Tab Title, type `notas` and Enter: the tab is called "notas" and so is the window | ✓ with the field focused on open (patch 0006) | `fase9-editar-titulo.png` |
| Edit Window Title, `proyecto` and Enter: the window is called "proyecto" and the tab stays "notas"; `echo sin-fugas` shows the Enter did not reach the shell | ✓ | — |
| Edit Tab Title with an empty field: the tab goes back to "root@vm: ~" | ✓ | — |
| Help › Copy Mode Shortcuts: table of keys and actions | ✓ | `fase9-atajos-modo-copia.png` |

The D-Bus menu also closed pending tests from phases 7 and 8:

| Test | Result |
|---|---|
| "Select Output of Last Command" disabled without integration and enabled after `echo hola` with integration | ✓ |
| "Copy Mode" checked while the mode lasts | ✓ |
| "Undo Close" enabled after closing a tab and disabled after five seconds | ✓ |
| "Paste History" disabled at start and enabled after pasting | ✓ |

**Not tested:** with the kit as is, the field without focus until clicked
(BK-029); the "Cancel" and "Rename" buttons with the mouse.

**Differs from iTerm2:**

- The title does not accept variables such as `\(session.name)`; it is fixed
  text.
- Help has no online help and no open source licenses.

## Phase 10 — Timestamps and Reset Character Set

### Plan

1. **View › Show Timestamps (⇧⌘E):** each session records when each line
   last changed; with the option on, the local time (`HH:MM:SS`) is drawn at
   the right of each line. The stamps follow their lines on resize, like the
   prompt marks, and are capped at the scrollback size.
2. **Session › Reset Character Set:** sets G0 and G1 back to ASCII, for when
   a program leaves the screen with line-drawing characters.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway; Reset Character Set, which has no shortcut, with the
patched kit and the D-Bus menu. `./dev.sh check` clean in both kit modes; 32
app tests and 1 block test. No new gap. The local time comes from
`localtime_r` through `libc`, already in the dependency tree.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `echo uno`, two seconds, `echo dos` and ⇧⌘E: each line has its time on the right, the second pair three seconds later, matching `date` | ✓ | `fase10-marcas-de-tiempo.png` |
| ⇧⌘E again: the times go away; "Show Timestamps" checked in the menu only while they show | ✓ | — |
| `printf '\033(0'` leaves the prompt in line drawing; Reset Character Set and `echo despues`: the new text is ASCII | ✓ | `fase10-juego-de-caracteres.png` |

**Not tested:** the times after a resize that reflows lines; a session that
reaches the scrollback limit.

**Differs from iTerm2:**

- The time is that of the last output chunk that touched the line, not of
  each character; lines that arrive in the same chunk share a time.
- There is no relative format ("5 minutes ago") nor a setting to choose it.
- The times cover the end of long lines instead of moving them aside.

## Phase 11 — Links and saving the selection

### Plan

1. **Super+click on a link:** opens with `xdg-open` the cell's OSC 8 link
   or, when there is none, the URL (`http`, `https`, `ftp`, `file`,
   `mailto`) that crosses the clicked column. Freya's mouse events carry no
   modifiers, so the window tracks Super through the keyboard events.
2. **Edit › Open Selection:** opens the selected text with `xdg-open`.
3. **Shell › Save Selected Text… (⌘S):** a sheet with the file path (it
   starts at the home folder; `~/` expands) that saves the selection.
4. **Test** on headless sway, with a fake `xdg-open` that records what it
   gets, and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with a fake `xdg-open` at the front of `PATH` that
records its arguments; Open Selection, which has no shortcut, with the
patched kit and the D-Bus menu. `./dev.sh check` clean in both kit modes; 33
app tests and 1 block test. One new gap: BK-030, mouse events carry no
modifiers.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Plain click on `https://example.org/prueba,`: opens nothing | ✓ | — |
| Super+click on that URL: `xdg-open https://example.org/prueba`, without the comma | ✓ after counting the Meta key itself as held (BK-030) | — |
| Super+click on OSC 8 text: `xdg-open https://osc8.example/doc` | ✓ | — |
| Double click on `seleccion-abierta`, ⌘S, `rel.txt` and Enter: `~/rel.txt` saves the word; `~/sel.txt` and `/tmp/abs.txt` too | ✓ | `fase11-guardar-seleccion.png` |
| Open Selection and Save Selected Text disabled without a selection and enabled with one; Open Selection opens the chosen text | ✓ | — |

**Not tested:** a failed save (it only leaves a warning in the log); the
save sheet with patch 0006's automatic focus now that the field starts
empty.

**Differs from iTerm2:**

- There is no panel to choose the file (BK-023): the path is typed, and a
  relative one starts at the home folder.
- A save error is not shown in the window.
- URLs are searched in the clicked row; a URL split across two rows is only
  found when it has OSC 8.
- Double click splits at `:` and `/`, so Open Selection opens the selection
  as is; there is no smart URL selection.

## Phase 12 — Session log and Paste Special

### Plan

1. **Session › Log › Log to File… and Stop Logging:** a sheet with the path
   (relative to the home folder, like Save Selected Text); the I/O loop
   appends the bytes to the file as they come from the PTY until it stops.
2. **Edit › Paste Special:**
   - **Paste Slowly:** sends the clipboard in small chunks with a pause
     between them, for programs that lose what arrives all at once.
   - **Paste Escaping Special Characters:** puts `\` before the shell's
     special characters.
   - **Paste Selection:** pastes the terminal selection without going
     through the clipboard.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with the patched kit, because the new items have no
shortcut and were clicked from the D-Bus menu. `./dev.sh check` clean in both
kit modes; 34 app tests and 1 block test. No new gap.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Log to File…, `sesion.log`, `echo registrado`, Stop Logging and `echo fuera`: the file has `registrado` (command and output) and not `fuera`; both items enable and disable together | ✓ | — |
| Copy `mi archivo (1).txt`, `echo ` and Paste Escaping Special Characters: `echo` prints the whole name | ✓ | — |
| Paste Slowly of 63 bytes with a raw-mode reader: 16, 16, 16 and 15 bytes arrive 30 ms apart | ✓ | `fase12-pegar-despacio.png` |
| Double click on `palabra`, `echo ` and Paste Selection: prints `palabra` without touching the clipboard | ✓ | — |

During the test, a script of mine that restored the tty from a descriptor
already in raw mode left the shell without echo; `strace` showed CurseTerm
still wrote the keys to the PTY, so it was not a terminal bug.

**Not tested:** a write failure in the log (it stops logging and leaves a
warning); Paste Slowly with several broadcast sessions.

**Differs from iTerm2:**

- The log keeps the raw bytes, escape sequences included; there is no plain
  text option nor automatic logging per profile.
- Paste Slowly's chunk size (16 bytes) and pause (30 ms) are fixed.
- Paste Special has none of iTerm2's advanced editor (transforms, base64,
  tabs).

## Phase 13 — Profile badge and paths with Super+click

### Plan

1. **Profiles › General › Badge:** a profile text drawn large and
   translucent in the top-right corner of each pane, like iTerm2's badge.
   `\(session.name)` is replaced by the session title; the rest is fixed
   text.
2. **Super+click on a path:** when there is no link under the click, the
   clicked word (without `:line` or trailing punctuation) is looked up
   relative to the session directory; when it exists, `xdg-open` opens it,
   like iTerm2's "semantic history".
3. **Transparency:** left out; the kit does not let a window ask for a
   translucent background (recorded as a gap).
4. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with phase 11's fake `xdg-open`. `./dev.sh check`
clean in both kit modes; 35 app tests and 1 block test. One new gap: BK-031,
the window cannot ask for a translucent background.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `badge = '\(session.name) · prod'` in the `.domain`: the pane shows "root@vm: ~ · prod" at the top right | ✓ | `fase13-insignia.png` |
| Settings › Profiles › General has the Badge row; typing `dev` changes the badge of the open window | ✓ | `fase13-ajustes-insignia.png` |
| `echo 'ver notas.txt:3 y nada.txt'` in `~`; Super+click on `notas.txt:3` opens `/root/notas.txt`; on `nada.txt`, which does not exist, nothing opens | ✓ | — |

**Not tested:** the badge in narrow split panes, where the text can spill
out on the left.

**Differs from iTerm2:**

- The badge only understands `\(session.name)`; iTerm2 accepts many
  variables and adjusts its size and position.
- Super+click on a path opens the file with `xdg-open`; iTerm2 lets you
  choose the editor and jumps to the line.
- No Use Transparency (BK-031).

## Phase 14 — Transparency

### Plan

1. **Patch 0007:** `WindowControls::set_background_alpha` multiplies the
   alpha of the background the runner clears the window with (BK-031).
2. **Profiles › Window › Transparency:** from 0 % to 90 % in steps of 10.
   Each pane paints its default background with its profile's opacity; the
   title bar and the tab bar stay opaque.
3. **View › Use Transparency (⌘U):** turns the window's transparency on or
   off. With the kit as is the item stays disabled.
4. **Test** on headless sway, which composes the alpha over the desktop
   background, and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with the patched kit and an orange desktop
background. `./dev.sh check` clean in both kit modes; 35 app tests and 1
block test. BK-031 is solved with patch 0007.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `transparency = 50`: the pane shows the orange through; the title bar and the tab bar stay opaque | ✓ | `fase14-transparencia.png` |
| ⌘U turns transparency off and on again | ✓ after fixing a panic: `set(!*peek())` asked for the write while the read was still borrowed | — |
| Settings › Window › Transparency at 80 %: the `.domain` stores 80 and the open window changes live | ✓ | `fase14-ajustes-transparencia.png` |

**Not tested:** with the kit as is, the disabled menu item (there is no D-Bus
menu without patch 0003); panes with profiles of different transparency in
the same tab; blur, which sway does not do.

**Differs from iTerm2:**

- Transparency affects the default background; background colours a program
  sets stay opaque, without iTerm2's option to choose.
- There is no own blur: it depends on the compositor.

## Phase 15 — Open Quickly and Open Profiles

### Plan

1. **View › Open Quickly (⇧⌘O):** a sheet with a filter and the list of the
   window's sessions ("title — profile"); Enter goes to the chosen tab and
   pane.
2. **Profiles › Open Profiles… (⌘O):** the same sheet with the list of
   profiles; Enter opens a new tab with the chosen profile.
3. **A single picker sheet:** the paste history, Open Quickly and Open
   Profiles share the sheet (filter, table, arrows, Enter, Escape); each one
   only brings its entries and what to do with the chosen one.
4. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway. `./dev.sh check` clean in both kit modes; 36 app
tests and 1 block test. No new gap: the three lists use the same sheet with
`SearchField` and `Table`, and the filter is a pure function with its test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Three tabs (two "Default", one "New Profile"), Super+1, ⇧⌘O, `new` and Enter: goes to the "New Profile" tab | ✓ | `fase15-open-quickly.png` |
| ⌘O lists both profiles; `def` and Enter opens a fourth tab with "Default" | ✓ | `fase15-open-profiles.png` |
| ⇧⌘H after pasting `echo historial`: the entry is still in the sheet and Enter pastes it | ✓ | — |

**Not tested:** Open Profiles in the global menu with patch 0004, which
rebuilds the Profiles menu; Open Quickly with split panes.

**Differs from iTerm2:**

- Open Quickly only searches this window's sessions (one process per window,
  BK-001) and offers no commands, profiles or arrangements.
- Open Profiles always opens a tab; iTerm2 lets you choose a new window.

## Phase 16 — Terminal State, Edit Session and help

### Plan

1. **Session › Terminal State:** checkboxes with the modes the session's
   program asked for (alternate screen, focus reporting, mouse reporting,
   bracketed paste, application cursor and keypad); clicking one changes it
   by sending the terminal the same sequence a program would send.
2. **Session › Edit Session… (⌘I):** opens Settings with the active
   session's profile already chosen.
3. **Help › CurseTerm Help:** opens the project repository with `xdg-open`.
4. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with the patched kit to read and click the submenu
over D-Bus, and with phase 11's fake `xdg-open`. `./dev.sh check` clean in
both kit modes; 36 app tests and 1 block test. No new gap.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `printf '\e[?2004h\e[?1h'; cat`: Paste Bracketing and Application Cursor checked; the rest, not | ✓ | — |
| Clicking Application Cursor unchecks it; clicking Alternate Screen empties the view and clicking it again brings the text back | ✓ | — |
| Tab with "New Profile" and ⌘I: Settings opens with "New Profile" chosen | ✓ | `fase16-editar-sesion.png` |
| ⇧⌘? opens `https://github.com/HBlanqueto/curseterm` | ✓ | — |

**Not tested:** Focus Reporting and Mouse Reporting with a program that uses
them; CurseTerm does not yet send focus or mouse events to the program, it
only shows and changes the mode.

**Differs from iTerm2:**

- Edit Session edits the profile, not a copy of its own for the session.
- Terminal State has no "Standard Key Reporting", "Literal mode" nor a reset
  of the whole state.
- Help opens the repository; there is no built-in manual.

## Phase 17 — Mouse and focus for programs

### Plan

1. **Mouse reporting:** when the program asks for it (modes 1000, 1002 and
   1003), clicks, drags, motion and the wheel reach it as sequences, in the
   classic format or in SGR (1006) when asked. With Shift held, the mouse
   selects text again, as in iTerm2.
2. **Modifiers:** the window tracks Shift, Ctrl and Alt like Super (BK-030),
   for the Shift bypass and to add them to the report.
3. **Focus reporting (1004):** when the window gains or loses focus, the
   active session gets `CSI I` or `CSI O`.
4. **Test** on headless sway with a reader that shows what arrives, and
   documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with a Python reader that turns the modes on, puts
the tty in raw mode and records what it gets. `./dev.sh check` clean in both
kit modes; 39 app tests and 1 block test. No new gap; BK-030 now covers every
modifier.

**Works:**

| Test | Result |
|---|---|
| Modes 1000, 1006 and 1004; click on column 6 of row 2: `CSI <0;6;2M` and `CSI <0;6;2m` arrive | ✓ |
| Opening Settings and closing them: `CSI O` and `CSI I` arrive | ✓ |
| Modes 1000 and 1002 in the classic format: press (32), two drags (64) when the cell changes, release (35), wheel up (96) and down (97) | ✓ |
| With Shift held, dragging selects text and nothing reaches the program | ✓ |

**Not tested:** mode 1003 (motion without a button) with a real program such
as `htop` or `vim`; the middle and right buttons; Ctrl and Alt inside the
report outside the tests.

**Differs from iTerm2:**

- There is no setting to turn mouse reporting off per profile.
- UTF-8 mode (1005) is missing; cells past column 222 are not reported in
  the classic format.

## Phase 18 — Coprocesses

### Plan

1. **Session › Run Coprocess…:** a sheet asks for the command; it runs with
   `sh -c` in the session directory. What comes out of the terminal reaches
   its standard input and what it writes enters the session as if typed.
2. **Session › Stop Coprocess:** ends the coprocess. Both items enable
   depending on whether one is running; when it ends on its own, it is
   reaped.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway with the patched kit, because the items have no
shortcut and were clicked over D-Bus. `./dev.sh check` clean in both kit
modes; 39 app tests and 1 block test. No new gap.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Coprocess that waits for `PING` with `grep -m1` and writes `echo PONG`; `echo PING` in the shell: `echo PONG` is typed and `PONG` comes out | ✓ | `fase18-coproceso.png` |
| Stop Coprocess: no coprocess process is left | ✓ after starting it in its own process group and ending the group; before, the inner `sh` was orphaned | — |
| Stop Coprocess enabled only while one runs; one that ends on its own (`echo ya`) is reaped and the item disables | ✓ | — |

**Not tested:** closing the window with a running coprocess (the process
exits without going through `Drop`; the coprocess gets end of file on its
input, but keeps running if it does not read it).

**Differs from iTerm2:**

- There are no automatic coprocesses from triggers nor the option for the
  coprocess not to get the output.
- What the coprocess writes is sent as is, without bracketed paste mode.
