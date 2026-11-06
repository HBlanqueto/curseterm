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

## Phase 19 — Scroll indicator and smart selection

### Plan

1. **Scroll indicator:** when scrolled up through the scrollback, a thin bar
   on the pane's right edge shows which part of the scrollback is visible.
2. **Smart selection (four clicks):** selects the whole URL under the
   pointer or, when there is none, the space-separated word (a path with `/`
   and `:`), like iTerm2's smart selection. One, two and three clicks still
   select character, word and line.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-05)

Tested on headless sway. `./dev.sh check` clean in both kit modes; 40 app
tests and 1 block test. No new gap.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `seq 1 200` with 50 lines of scrollback and the wheel up: the bar shows on the right edge, a third of the height and at the bottom; it goes away when back at the bottom | ✓ | `fase19-indicador.png` |
| Four clicks on `(https://a.org/x).` and ⌘C: the clipboard holds `https://a.org/x` | ✓ | — |

**Not tested:** dragging the bar (it cannot be done: it is only an
indicator); four clicks on a path outside the `word_span` test.

**Differs from iTerm2:**

- The indicator cannot be dragged or clicked, and there is no option to hide
  the bar.
- Smart selection uses two fixed rules (URL or space-separated word); iTerm2
  ships a list of rules editable per profile.

## Phase 20 — Several windows in one process

### Plan

1. **Patch 0008:** `WindowControls::open_window(app, options)` opens another
   window in the same process (BK-001). The runner already kept several
   windows and ends when the last one closes.
2. **One description per window:** terminal, Settings and About are
   described once (`windows::describe`); `main` and new windows use it.
3. **With the patch:** New Window, profile windows, Settings, About and Edit
   Session open in the same process, and Quit closes them all. Without the
   patch it stays one process per window.
4. **What sharing the process brings:** the paste history is common to all
   the process's windows.
5. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway in both kit modes. `./dev.sh check` clean in both;
40 app tests and 1 block test. One new gap: BK-032, `Timers` callbacks
outlive their window.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Patched kit: ⌘N and ⌘, open two more windows in the same process (3 windows, 1 process) | ✓ | — |
| Paste `echo compartido` in the first and ⇧⌘H in the second: the entry is there | ✓ | `fase20-historial-compartido.png` |
| Closing Settings and a terminal window: the process stays, the closed window's shell ends and the other window responds | ✓ after fixing two panics: `close_window`'s (in patch 0008) and the closed window's I/O wakeup (BK-032) | — |
| Quit with two windows: both close and the process ends | ✓ | — |
| Kit as is: ⌘N opens a second process and Quit closes both | ✓ | — |

**Not tested:** closing a window with pending timers (bell, undo close) in
flight; the hotkey window, which is still a separate process.

**Differs from iTerm2:**

- Open Quickly and Undo Close still do not see the other windows; tabs
  cannot be moved between windows either.

## Phase 21 — Input method

### Plan

1. **Patch 0009:** the runner binds `zwp_text_input_v3`, publishes the
   composition text and delivers the committed one as a key (BK-033).
2. **CurseTerm:** draws the composition underlined at the active session's
   cursor and tells the input method where the cursor is.
3. **Test** with a test input method (`vime`, a `zwp_input_method_v2` client
   in the lab) and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
kit modes; 40 app tests and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `echo ` and composition `ñand`: it shows underlined at the cursor and the shell does not get it yet | ✓ after attaching the initial seat; sctk only reports new ones | `fase21-composicion.png` |
| Committing `ñandú`: the shell gets the text and the composition goes away | ✓ | — |

**Not tested:** that the compositor gets `set_cursor_rectangle` (the trace
did not show it); a real input method such as fcitx5 or IBus; composition
in the sheets' text fields.

**Differs from iTerm2:** the composition is drawn over the line, without
moving aside the text right of the cursor.

## Phase 22 — Primary selection

### Plan

1. **Patch 0010:** the primary selection (`zwp_primary_selection`) with the
   kit's same `Clipboard` and `WindowControls::primary_selection()` (BK-034).
2. **CurseTerm:** on mouse release with text selected, it publishes it; the
   middle button pastes the primary selection into the session.
3. **Test** with `wl-copy --primary` and `wl-paste --primary`.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
kit modes; 40 app tests and 1 block test.

| Test | Result |
|---|---|
| Double click on `primaria`: `wl-paste --primary` gives `primaria` | ✓ |
| `wl-copy --primary "echo desde-fuera"` and the middle button: the shell gets the command | ✓ |

**Not tested:** the middle button when the program asks for the mouse (the
click is sent to it and nothing is pasted, as in iTerm2); the primary
selection in the sheets' text fields.

## Phase 23 — Copy with Styles

### Plan

1. **Patch 0011:** `WindowControls::copy_rich(text, html)` offers
   `text/html` besides plain text (BK-035).
2. **Edit › Copy with Styles (⌥⌘C):** turns the selection into HTML with each
   cell's resolved colours, bold, italic and underline.
3. **Test** with `wl-paste -l` and `wl-paste -t text/html`.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
kit modes; 41 app tests (the HTML one only with the patched kit) and 1 block
test.

| Test | Result |
|---|---|
| Bold red `rojo` and ` <normal>`, triple click and ⌥⌘C: there is `text/html` with a bold `span` coloured `#cb4b16`, and `<normal>` escaped; the plain text is `rojo <normal>` | ✓ after keeping dark mode in `Handles`: `theme::resolved()` aborts outside Freya's context (BK-017) |
| Underlined `verde`: the `span` has `text-decoration:underline` | ✓ |
| ⌘C afterwards: only the text types remain | ✓ |

**Not tested:** pasting into an app that reads `text/html` (there is none in
the lab); block selections.

**Differs from iTerm2:** iTerm2 copies RTF; CurseTerm copies HTML, the rich
format Linux apps understand.

## Phase 24 — Instant Replay

### Plan

1. **Recording:** the I/O loop keeps a snapshot of the visible screen (cells
   and cursor) after output, at most one every half second and the last 120,
   which cover at least the last minute of activity.
2. **View › Start Instant Replay (⌥⌘B):** the active pane shows the chosen
   snapshot instead of the screen; a bar at the bottom has a slider, the
   snapshot's time and "Done". ← and → change snapshot; Escape or Done go
   back to the present.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes; 40 and 41 app tests and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `clear; echo uno`, `echo dos` and `echo tres` one second apart; ⌥⌘B shows the last snapshot, ← four times reaches the first (just the prompt), with its time | ✓ after fixing a panic: the `if let`'s `peek` was still borrowed when writing the state | `fase24-repeticion.png` |
| Escape goes back to the present and `echo presente` writes into the shell | ✓ | — |

**Not tested:** the slider with the mouse; a session with more than 120
snapshots.

**Differs from iTerm2:**

- Screen snapshots are kept every half second, not on every change; memory
  is capped by snapshot count and there is no setting to change it.
- Only the visible screen is replayed, without each moment's scrollback.

## Phase 25 — Toolbelt

### Plan

1. **View › Toolbelt (⇧⌘B):** a panel on the window's right with three
   tools:
   - **Jobs:** the active session's processes (those in its terminal
     session, without the shell), read from `/proc`; "Terminate" sends
     `SIGTERM` to the chosen one.
   - **Paste History:** the paste history; clicking an entry pastes it.
   - **Recent Directories:** the directories the window's sessions went
     through; clicking one types `cd` with the escaped path.
2. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes; 41 and 42 app tests and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `sleep 300 &`, `cd /tmp`, `cd /root` and pasting `echo pegado`; ⇧⌘B: Jobs lists `sleep` with its PID, Paste History the entry and Recent Directories `/root` and `/tmp` | ✓ after sharing the height among the three tables; with a fixed height the last one was cut off | `fase25-toolbelt.png` |
| Choosing `sleep` and Terminate: the process ends and the shell says "Terminated" | ✓ | — |
| Clicking `/tmp`: `cd /tmp` is typed; clicking `echo pegado`: it is pasted | ✓ | — |

Test clicks with no pause between pressing and releasing did not select
`Table` rows; with 150 ms they did. It is a limit of the lab's virtual
mouse, not noticeable with a real mouse.

**Not tested:** processes of another tab (Jobs follows the active session);
paths with spaces in Recent Directories.

**Differs from iTerm2:** there are no notes, no captured output tool and no
recent commands tool; the tools cannot be reordered.

## Phase 26 — Status bar

### Plan

1. **Profiles › Session › Status bar:** with the option on, the window shows
   under the panes a bar with the active session's data: the directory, the
   foreground process, the directory's git branch, the host name and the
   time, which updates every second.
2. **Git branch:** `.git` is looked up walking up from the directory and
   `HEAD` is read (also when `.git` is a `gitdir:` file), without running
   `git`.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes; 42 and 43 app tests and 1 block test.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `status_bar = true`, `cd /tmp/repo-demo; sleep 30` with a `.git/HEAD` on `rama-demo`: the bar shows `/tmp/repo-demo`, `sleep`, `rama-demo`, `vm` and the time | ✓ | `fase26-barra-de-estado.png` |
| The time goes from 00:34:09 to 00:34:11 between two screenshots; about 5 events per second at rest | ✓ after keeping a single timer: one per render would have multiplied the chains with each output | — |

**Not tested:** the option from Settings (only from the `.domain`); a real
repository with `git worktree` outside the `git_branch` test.

**Differs from iTerm2:** the components are fixed; there is no bar editor,
no CPU, memory or network, and no top position.

## Phase 27 — Profile cursor

### Plan

1. **Profiles › Text › Cursor:** Box, Vertical Bar or Underline, and
   **Blinking cursor**. It is alacritty's default style
   (`default_cursor_style`): an app that asks for another with DECSCUSR
   (`\e[2 q`) wins, and `\e[0 q` goes back to the profile's.
2. **Blinking:** only in the active pane of the focused window. A single
   timer per window repaints every `TERMINAL.cursor_blink_ms` (500 ms), the
   same mechanism as the status bar clock.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `cursor = "vertical_bar"` and `blinking_cursor = true`: vertical bar; six screenshots 250 ms apart alternate between two images | ✓ | `fase27-cursor-parpadeo.png` |
| `printf '\e[2 q'`: steady block, four identical screenshots; `printf '\e[0 q'`: the blinking bar comes back | ✓ | — |
| Settings › Profiles › Text shows Cursor and Blinking cursor; turning the switch off writes `blinking_cursor = false` and the terminal, in another process, stops blinking | ✓ | `fase27-ajustes-cursor.png` |

**Not tested:** picking another type in the pop-up (only from the `.domain`).

**Differs from iTerm2:** typing does not restart the blink with the cursor
visible; there is no profile cursor color of its own (it comes from the
scheme), and no Cursor Guide.

## Phase 28 — Send text at start

### Plan

1. **Profiles › General › Send text at start:** the text is written to the
   shell, followed by Return, each time a session of the profile starts:
   window, tab, pane or restart.
2. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `initial_text = "echo hola-inicio"`: the first tab and a new one (⌘T) show `hola-inicio` after the first prompt | ✓ | `fase28-texto-al-iniciar.png` |

**Not tested:** editing the field from Settings (only from the `.domain`); a
new pane and the restart when the session ends.

**Differs from iTerm2:** the text's echo shows before the first prompt,
because it is written as soon as the pty exists, without waiting for the
shell to ask for it.

## Phase 29 — General and Pointer settings

### Plan

1. **Settings › General › Selection › Copy to clipboard on selection:**
   releasing the mouse after selecting copies to the clipboard and to the
   paste history, besides the primary selection. On by default, as in
   iTerm2.
2. **Settings › Pointer › Focus follows mouse:** when the pointer enters a
   pane, that pane gets the focus. Off by default.
3. Both are read when the event happens, not when painting: a pane that is
   not repainted does not keep the old value.
4. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Dragging over a line: the clipboard goes from `vacio` to `copia-al-seleccionar`; with `copy_on_select = false` it stays `vacio` | ✓ | — |
| ⌘D and `focus_follows_mouse = true`: moving the pointer to the left pane and typing types there; with the option off, the text stays in the active pane | ✓ after reading the option on the fly: with the captured value, the pane that was inactive when it changed kept the old one | `fase29-foco-sigue-al-raton.png` |
| Settings shows General, Appearance, Profiles and Pointer; the General switch writes `copy_on_select = true` | ✓ | `fase29-ajustes-general.png` |

**Fixed on the way:** `instant-replay` was twice in the `.ftl` files and
Fluent warned about it at start.

**Not tested:** the Pointer switch (only from the `.domain`).

**Differs from iTerm2:** focus only moves between the panes of a window;
focus between windows is up to the compositor. The General and Pointer
icons are not iTerm2's (BK-024). The other options of both panes are
missing.

## Phase 30 — Cursor guide

### Plan

1. **View › Show Cursor Guide (⌥⌘;):** a translucent stripe in the cursor
   color crosses the cursor line. It is per session, like Show Timestamps.
2. **Profiles › Colors › Cursor guide:** the value new sessions of the
   profile start with.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| ⌥⌘; after `ls /`: the cursor line pixel goes from (0, 43, 54) to a lighter one and the line above does not change | ✓ with alpha 0.25; with 0.15 it was barely visible | — |
| `cursor_guide = true` in the profile: the first session already starts with the guide; ⌥⌘; removes it | ✓ | `fase30-guia-del-cursor.png` |

**Not tested:** the menu checkbox over D-Bus and the Settings switch (only
from the `.domain`).

**Differs from iTerm2:** the color is the scheme's cursor color, not a
profile color of its own; changing the profile option does not affect open
sessions.

## Phase 31 — Recent directories

### Plan

1. **Session › Open Recent Directories… (⌥⌘/):** the Paste History picker
   with the window's recent directories, the most recent first.
2. **Choosing one** types its shell-escaped path on the command line, without
   `cd` or Return. That is what iTerm2 does: `DirectoriesPopup.m` returns
   `shouldEscapeShellCharacters = YES` and `PTYSession.m`
   (`popupInsertText:`) inserts the text.
3. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `cd '/tmp/dir con espacio'`, `cd /var`, `cd` and ⌥⌘/: the list shows `/root`, `/var` and `/tmp/dir con espacio` | ✓ | `fase31-directorios-recientes.png` |
| Typing `ls `, filtering «espacio» and Return: the line becomes `ls /tmp/dir\ con\ espacio` and runs without error | ✓ | — |

**Not tested:** the global menu item over D-Bus.

**Differs from iTerm2:** the path is always escaped; iTerm2 only does it when
the cursor is at the prompt. The list belongs to the window and is not kept
when it closes, nor sorted by how often it is used.

## Phase 32 — Clear to a mark or the selection

### Plan

The three iTerm2 «Edit» items (`sources/MainMenu/Base.lproj/MainMenu.xib`)
that were missing:

1. **Clear to Last Mark (⌥⌘L):** clears from the last mark above the cursor
   down to the cursor; the cursor line moves up. The marked line stays unless
   it is the one just above the cursor, like `clearToLastMark` in
   `sources/VT100Screen/VT100Screen.m`. Enabled only when there is such a
   mark.
2. **Clear to Start of Selection (⌘L):** the same from the first line of the
   selection (`clearToStartOfSelection:` in `PseudoTerminal.m`).
3. **Clear Instant Replay (⌥⌘K):** drops the stored frames and leaves replay
   if it was open.
4. Marks, times, images and highlights of the cleared lines are forgotten and
   those below move up with the text (`Extras::forget_lines`, which replaces
   `forget_lines_above`).
5. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the kit as is: it needs no patch. `./dev.sh
check` clean in both kit modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `echo A`, ⇧⌘M, `seq 5` and ⌥⌘L: the `seq 5` line stays with its mark and 1–5 go away; the prompt moves up | ✓ | `fase32-borrar-hasta-la-marca.png` |
| Then `echo B`, selecting the `A` line and ⌘L: `echo A` and the prompt remain; the `seq 5` mark goes with its line | ✓ | — |
| ⌥⌘B opens replay, ⌥⌘K closes it and ⌥⌘B no longer does anything; the app stays alive | ✓ | — |

**Not tested:** the disabled items in the global D-Bus menu.

**Differs from iTerm2:** only visible lines are cleared. If the mark or the
selection starts in the scrollback, clearing starts at the first screen line,
because alacritty cannot remove single lines from the scrollback.

## Phase 33 — Save contents

### Plan

1. **Shell › Save Contents…:** like Save Selected Text, but it saves the whole
   buffer (scrollback and screen) without the trailing empty lines. No
   shortcut, as in iTerm2's `MainMenu.xib`.
2. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway with the patched kit, to click the item through the
D-Bus menu. `./dev.sh check` clean in both kit modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `seq 80`, Save Contents… over D-Bus: the sheet with the path field shows | ✓ | `fase33-guardar-contenido.png` |
| Typing the path and Return: the file has 84 lines, from `seq 80` to the last prompt, and ends in a newline | ✓ | — |

**Not tested:** with the kit as is the item is only reachable from a global
menu (no shortcut).

**Differs from iTerm2:** the path is typed in a field; there is no file
chooser (BK-023).

## Phase 34 — Window arrangements

### Plan

iTerm2's Window › Arrangements (`sources/iTermController/iTermController.m`:
`saveWindowArrangementForWindow:`, `loadWindowArrangementWithName:` and
`tryOpenArrangement:named:asTabsInWindow:`):

1. **Save Current Window as Arrangement… (⌥⇧⌘S):** asks for a name and saves
   the window's tabs: title, pane tree with its ratios and, in each pane,
   profile and current directory. A repeated name replaces the old one.
2. **Restore Window Arrangement › name:** opens a new window with those tabs
   (in the same process with patch 0008; otherwise another process with
   `--arrangement name`).
3. **Restore Window Arrangement as Tabs › name:** adds the tabs to the
   current window.
4. Arrangements are stored in the app's `.domain`, like iTerm2 in its
   preferences. `arrangement.rs` holds capture and rebuild, with tests.
5. **Test** on headless sway and documentation in both languages.

### Result (2026-11-06)

Tested on headless sway in both kit modes. `./dev.sh check` clean in both
modes; 44 app tests.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| ⌘D, `cd /tmp`, ⌘T, `cd /var` and ⌥⇧⌘S «trabajo»: the `.domain` stores one tab with a `row` `split` 0.5/0.5 (`/root` and `/tmp`) and another in `/var` | ✓ with the kit as is the field must be clicked before typing (BK-029) | — |
| `curseterm --arrangement trabajo`: two tabs; the first one split, with `pwd` `/root` and `/tmp`; the second in `/var` | ✓ | `fase34-disposicion-restaurada.png` |
| Patched kit, D-Bus menu: «as Tabs › trabajo» goes from 1 to 3 tabs; «Restore › trabajo» opens a second window in the same process | ✓ | — |

**Not tested:** an arrangement with a deleted profile (it opens with the
default profile).

**Differs from iTerm2:** there is no Save Window Arrangement (⇧⌘S) for every
window, because each window only knows its own tabs; neither size nor
position is saved (Wayland does not let apps place windows), nor contents,
and there is no default arrangement at start, nor import or export to a file.

## Phase 35 — Annotations

### Plan

iTerm2's Edit › Marks and Annotations and View › Show Annotations, copied
from `sources/Annotations/PTYNoteViewController.m`,
`sources/TerminalView/PTYNoteView.m`, `PTYSession.m` (`addNoteAtCursor`,
`nextMarkOrNote:`) and `iTermTextDrawingHelper.m` (`drawNoteRangesOnLine:`):

1. **Model.** An annotation is a range of cells (start and end on absolute
   lines) with its text, anchored like marks: it follows scrolling and resizing
   and is cleared with the buffer.
2. **Add Annotation at Cursor (⌥⌘M):** annotates the smart selection at the
   cursor, then the one of the next cell, the one of the previous cell and, if
   there is none, the whole line. **Annotate Selection:** annotates the
   selection.
3. **Underline:** a 1-point yellow line, 2 points above the bottom of each
   annotated cell.
4. **Bubble** (`note_view.rs`): a rectangle of radius 5 with a 7×7 pointer to
   the end of the range, 40 points from its left edge when it fits; above the
   text when it does not fit below. Background (250, 250, 250) and black border
   at 10 % in light; (50, 50, 52) and white at 40 % in dark; black shadow at
   15 %, 2 points down and 8 of blur. 12-point text 250 wide that grows in
   height; a 17-point trash at the top right.
5. **Editing** with the `views::text_view` block (BK-036): a new note takes
   the focus; Enter breaks the line; Escape gives the keyboard back to the
   terminal.
6. **Trash:** asks «Really remove annotation?» (OK, Cancel) and deletes it.
7. **Next/Previous Annotation (⌥⇧⌘↓/↑):** walks by position, wraps around,
   shows it if hidden, centers it when it is off screen and highlights the
   background towards (0.85, 0.92, 1.0), which returns in 0.75 s on a
   squared curve.
8. **Show Annotations (⌘\\):** if any is visible it hides them all;
   otherwise it shows them; 0.25 s fade. The menu checkbox follows «there are
   annotations», as in iTerm2.
9. **Test** on headless sway in both kit modes and documentation.

### Result (2026-11-06)

Tested on headless sway in both kit modes. `./dev.sh check` clean in both
modes; 44 app tests.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `echo palabra` and ⌥⌘M with the cursor after it: underlines `palabra` (columns 20–26), the bubble takes the focus and typing does not reach the shell | ✓ | `fase35-anotacion.png` |
| Enter inside the note breaks the line and the bubble grows; Escape gives the keyboard back to the shell | ✓ | — |
| `seq 1 120` and ⌥⇧⌘↑: goes back to the note, centers it and the blue background fades | ✓ | `fase35-anotacion.png` |
| ⌘\\ twice: fades to 0 and back to 1 | ✓ | `fase35-mostrar-anotaciones.png` |
| Trash: «Really remove annotation?» alert; OK removes the bubble and the underline | ✓ | `fase35-confirmar.png` |
| Patched kit, D-Bus menu: Annotate Selection enabled only with a selection; annotates «seleccion de varias» with the pointer at the end | ✓ | — |
| Dragging inside the bubble selects text of the note, not of the terminal | ✓ | — |
| Dark appearance: background (50, 50, 52) and white border | ✓ | `fase35-anotacion-oscura.png` |

**Not tested:** several annotations on the same line or Next wrapping to the
start; a note in an unfocused pane.

**Fails:** with a full scrollback, the lines that leave at the top shift the
annotations, like marks: `alacritty_terminal` does not count the lines it
drops. Seen with `scrollback_lines = 50`.

**Differs from iTerm2:** the bubble cannot be resized with the mouse nor
dragged; it has no scroll bar; no «wrap» flash when Next or Previous wraps
around; smart selection is CurseTerm's (URL or word), not iTerm2's rules;
annotations are not saved on close.

## Phase 36 — Composer

### Plan

iTerm2's View › Show Composer (⇧⌘.), copied from
`sources/Composer/iTermComposerManager.m`,
`iTermMinimalComposerViewController.m` and its `.xib`, `ComposerTextView.swift`,
`iTermStatusBarLargeComposerViewController.xib` and `PTYSession.m`
(`frameForDesiredHeight:`, `composerManagerAppearance:`, `sendCommand:`):

1. **Model** (`composer.rs`): each session keeps whether the Composer is open
   and its text. ⇧⌘. opens or closes it keeping the text.
2. **Box:** at the top of the pane, 20 points from each side (at least 217
   wide), 135 high and never more than the pane minus 8. Inside, 9 points in,
   a «sheet» material: translucent window background over the blurred
   terminal, radius 6 and a 1-point grey border. The appearance follows the
   terminal background (perceived brightness 0.30/0.59/0.11 under 0.5 = dark).
3. **Text** with the `views::text_view` block (BK-036): the profile's font,
   size, colour and cursor; 5-point margin; vertical scrolling.
4. **Bottom row** of 19 points with «Press ⇧↩ to send.» on the right, hidden
   under 180 points; a 24×23 close button at the top right.
5. **Keys** (`standardActions`): ↩ breaks the line; ⇧↩ sends everything, each
   line with its return, and closes without the fade; ⌥⇧↩ sends the command at
   the cursor (its line and those joined by a trailing `\`) and removes it
   from the text; Escape and the button close with a 0.125 s fade and keep
   the text.
6. **Test** on headless sway in both kit modes and documentation.

### Result (2026-11-06)

Tested on headless sway in both kit modes. `./dev.sh check` clean in both
modes; 45 app tests (a new one for the command at the cursor).

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| ⇧⌘. (in `wtype`, Super+Shift+`greater`): it opens at the top with the focus, dark on the dark scheme, and typing does not reach the shell | ✓ | `fase36-composer.png` |
| A four-line `for` and ⇧↩: the shell runs it (`uno`, `dos`, `tres`) and the box closes | ✓ | `fase36-composer-enviado.png` |
| ⌥⇧↩ with the cursor on the first line: sends `echo uno` and removes it; the box stays open | ✓ | — |
| Escape and the close button: it closes, Escape does not reach the shell and ⇧⌘. brings it back with the text | ✓ | — |
| The «>» of ⇧⌘> does not get into the text | ✓ | — |
| Patched kit, D-Bus menu: View › Show Composer opens it | ✓ | — |

**Not tested:** a light scheme (light material), the closing fade in a
screenshot, pasting with ⌘V inside, the IME, and scrolling following the
cursor with a lot of text.

**Differs from iTerm2:** no handles to change the height (always 135) nor
dragging; no ⌥↩ (enqueue), completions, history, help, AI, multiple cursors
nor sending only the selection; no Auto Composer nor status bar Composer;
the text stops before the close button because Freya gives the pointer to
the scroll view under it; after ⌥⇧↩ the cursor does not go back to the start
of the removed command.

## Phase 37 — Named marks

### Plan

iTerm2's Edit › Set Named Mark… (⌥⇧⌘M) and Edit › Named Marks
(`PseudoTerminal.m -addNamedMark:`, `BookmarkDialogViewController.swift`,
`PTYSession.m -saveScrollPositionWithName:` and `-scrollToNamedMark:`,
`iTermApplicationDelegate.m` for the submenu):

1. **«Enter Mark Name» dialog** with the date and time
   (`yyyy-MM-dd HH:mm:ss`) as the default name; OK and Cancel; an empty name
   does nothing.
2. **Mark** on the first visible line (the blue triangle of marks) and a
   «Named Mark name» annotation across that line, visible and without focus.
3. **Named Marks submenu:** one entry per mark in the order they were set;
   choosing one centers it when it is off screen.
4. Named marks are anchored like the others and cleared with the buffer.
5. **Test** on headless sway and documentation.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `seq 1 60`, scroll up with the wheel and ⌥⇧⌘M: the dialog brings `2026-11-06 03:23:49`; with Enter, a triangle on line `18` and the note «Named Mark 2026-11-06 03:23:49» | ✓ | `fase37-marca-con-nombre.png` |
| D-Bus menu: Named Marks lists `2026-11-06 03:23:49` | ✓ | — |
| After `seq 1 80`, choosing the mark in the submenu goes back to line `18` | ✓ | — |

**Not tested:** with the kit as is (no D-Bus menu, and the field only takes
the focus when clicked, BK-029); several marks with the same name.

**Differs from iTerm2:** no Named Marks tool in the Toolbelt nor renaming or
deleting from it; jumping does not flash the mark; named marks are not in
Open Quickly.

## Phase 38 — Find URLs

### Plan

iTerm2's Edit › Find › Find URLs (⌥⌘U) (`PseudoTerminal.m -findUrls:`,
`findUrlsRegex` in `iTermAdvancedSettingsModel.m`,
`PTYTextView+ARC.m -convertVisibleSearchResultsToContentNavigationShortcutsWithAction:`,
`ContentNavigationShortcut.swift` and `iTermShortcutNavigationModeHandler` in
`iTermCopyModeHandler.m`):

1. **Search:** iTerm2's own expression over the visible lines, with the
   `alacritty_terminal` search, which follows wrapped lines. Results are
   highlighted like Find results.
2. **Shortcuts** (`hints.rs`): 1 to 9 and then letters, as many as the number
   of results needs (A…Z; AA, AB… beyond 26).
3. **Labels** (`hint_view.rs`): an accent capsule with a white border at 95 %
   and a black shadow at 40 %, 11-point white text; left of the result, 2
   points away, and top-aligned when the row is taller than the label. They
   come in scaling 0.25 → 1.3 → 1 in 0.2 s.
4. **Keys** (`validKeyEquivalentForEvent:`): with ⌘ or ⌃ they pass; Escape
   leaves; prefix plus key, or the key alone, that matches opens the URL (with
   ⌥ it copies it); a prefix leaves the typed part at 50 % and removes those
   that do not fit; any other key goes to the terminal. A click leaves.
5. **Exit:** the chosen one pops (×2, fade, half a turn) and the rest
   dissolve (×0.1, fade) in 0.2 s; then the highlight goes away.
6. **Test** on headless sway and documentation.

### Result (2026-11-06)

Tested on headless sway with the kit as is. `./dev.sh check` clean in both
modes; a new test for the shortcuts and the prefix.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Two lines with three URLs and ⌥⌘U: highlighted, with 1, 2 and 3 on their left | ✓ | `fase38-find-urls.png` |
| The URL in the command, wrapped over two lines, is found whole | ✓ | — |
| Twelve URLs: shortcuts 1–9, A, B and C | ✓ | — |
| ⌥5 copies `https://github.com/gnachman/iTerm2` (`wl-paste`) and leaves | ✓ | — |
| `q`, which is not a shortcut, reaches the shell and the mode stays | ✓ | — |
| Escape and a click leave without typing anything into the shell | ✓ | — |

**Not tested:** that «1» opens the browser (the lab has none; the command
runs without error), two-letter shortcuts in the app (they are in the test)
and the patched kit.

**Differs from iTerm2:** copying neither selects the text nor shows the
«Copied» toast; the pop and dissolve animation has not been seen in a
screenshot; the expression cannot be changed in Settings. The capsule and
its text are drawn on a canvas because Freya paints the terminal canvas,
deeper in the tree, over them.

## Phase 39 — Smart selections and picking a result

### Plan

iTerm2's Edit › Find › Find All Smart Selection Matches (⌥⇧⌘U) and Pick Result
to Open (⌥⇧⌘O) (`PTYSession.swift -smartSelectAllVisible`,
`PTYSession.m -regularExpressonForNonLowPrecisionSmartSelectionRulesCombined`,
`SmartSelectionRules.plist` and `PTYTextView.m -revealContentNavigationShortcuts:`),
on the labels of phase 38:

1. **Smart selections:** the rules of normal precision or more joined in one
   expression: `a::b`, paths, quoted text, dotted names, `mailto:`,
   `@selector(...)`, email addresses, HTTP URLs, `ssh:` and `telnet:`. The
   action is copy; with ⌥ the text goes to the shell.
2. **Picking a result:** with a search in progress, the visible results get
   labels; the action is open (a relative path from the shell's folder); with
   ⌥ it copies. The find field gives up focus and the Find highlight stays on
   exit. It is only enabled with a search.
3. **Test** on headless sway and documentation.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes; a new test checks that both expressions compile in the
`alacritty_terminal` search.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| A line with `/etc/hosts`, `foo::bar`, an email, `"entre comillas"` and `com.example.App` and ⌥⇧⌘U: five labels | ✓ | `fase39-selecciones-inteligentes.png` |
| `2` copies `foo::bar` (`wl-paste`) | ✓ | — |
| ⌥3 types `yo@ejemplo.org` into the shell | ✓ | — |
| ⌘F `conf` and ⌥⇧⌘O: label 1 on `conf`; the field gives up focus | ✓ | `fase39-elegir-resultado.png` |
| `1` runs `xdg-open` (the lab lacks it: a warning is logged) and the Find highlight stays | ✓ | — |
| D-Bus menu: Pick Result to Open enabled only with a search | ✓ | — |

**Not tested:** with the kit as is; actually opening a path or a URL.

**Differs from iTerm2:** the rules cannot be edited in Settings; the search
engine has no look-behind, so the HTTP URL rule is the branch that does not
take a trailing `)`, `[[:letter:]]` and `[[:number:]]` become `\p{L}` and
`\p{N}`, and word boundaries are ASCII; copying neither selects the text nor
shows «Copied». No Select Matches (several selections at once).

## Phase 40 — Mark alerts

### Plan

iTerm2's Edit › Marks and Annotations › Alerts (`MainMenu.xib`,
`PseudoTerminal.m -toggleAlertOnNextMark:`, `PTYSession.m
-screenDidAddMark:alert:completion:` and `-showMarkSetAlert`,
`iTermApplicationDelegate.m -markAlertAction`):

1. **Alert on Next Mark (⌥⌘A):** a per-session checkbox; the next mark alerts
   and clears it.
2. **Alert on Marks in Offscreen Sessions:** an app setting (off by default);
   a mark in a tab that is not shown alerts too.
3. **How to alert** (radio, app setting): Show Modal Alert Box (an «Alert»
   alert with Reveal and OK), Post Notification (the default; «Mark Set»,
   «Session … #n had a mark set.», and clicking it reveals the session) and
   Play a Sound (success or error by the exit code of the previous command,
   now read from `OSC 133;D;code`).
4. **Notifications and sounds** in the `notifications` block of
   `barrier-kit-next` (BK-026).
5. **Generic test**, without Malaysia: headless sway with `mako` as the
   notification server, `dbus-monitor` to see the calls and a fake
   `canberra-gtk-play` that logs what it is asked for.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes; the `OSC 133` test now checks the exit code. The marks were made by
hand with `printf '\033]133;D;code\007\033]133;A\007'`.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| D-Bus menu: Alerts with both checkboxes and the three radios, Post Notification checked | ✓ | — |
| ⌥⌘A checks the box; the next mark unchecks it | ✓ | — |
| With ⌥⌘A, a mark calls `Notify` with «Mark Set», «Session root@vm: ~ #1 had a mark set.», the `default`/«Reveal» action and `desktop-entry` | ✓ | — |
| With offscreen marks on, a mark in tab 1 seen from tab 2 shows up in mako | ✓ | `fase40-notificacion.png` |
| `makoctl invoke` (`ActionInvoked`) goes back to tab 1 | ✓ | `fase40-mostrar.png` |
| Play a Sound: code 1 asks for `dialog-error` and code 0 for `complete` | ✓ | — |
| Show Modal Alert Box: «Alert», «Mark set in session “root@vm: ~.”», OK and Reveal; Reveal goes back to tab 1 | ✓ | `fase40-alerta.png` |

**Not tested:** the kit as is; the activation token (mako 1.11 sends no
`ActivationToken`, so the window is not brought forward); real sound (the lab
has no audio: `canberra-gtk-play` says «Not available»); Malaysia, which has
no notification server yet.

**Differs from iTerm2:** the sound file cannot be chosen
(`successSound`/`errorSound`); there is no alert indicator in the pane corner;
a session restored from an arrangement does not wait for the first key before
alerting about offscreen marks; nothing is sent to the iPhone.

## Phase 41 — Profile notifications

### Plan

iTerm2's Profiles › Terminal › Notifications
(`ProfilesTerminalPreferencesViewController.m`, `PTYSession.m
-shouldPostUserNotification`, `-notificationsSuppressed`, `-setBell:`,
`-brokenPipeWithError:` and `-screenPostUserNotification:rich:`, `PTYTab.m
-setLabelAttributesForActiveTab:` and `-setLabelAttributesForIdleTab`):

1. **Settings** in the profile's Terminal tab, with iTerm2's defaults:
   Notification Center alerts (off), Bell (on), Idle and New Output (off),
   Session Ended and escape-sequence alerts (on) and Suppress alerts from
   active session (off).
2. **When one is posted:** with alerts on, when not suppressed and the
   session is not shown (another tab) or the window is not active.
3. **Bell, session ended, new output and idle**, with iTerm2's texts. New
   output alerts once per burst in a background tab (not while broadcasting
   input); idle, 3 s after the last output (`idleTimeSeconds` plus 1).
4. **Escape sequences:** `OSC 9;text` («Alert», «Session … #n: text»; a
   numeric first field is ConEmu's progress and does not alert) and
   `OSC 1337;Notification=title=…;subtitle=…;message=…` in base64. They do
   not check whether the session is shown, as in iTerm2.
5. **Clicking** any of them reveals the session, as in phase 40.
6. **Generic test** with mako and `dbus-monitor`, without Malaysia.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes; new tests for `OSC 9` and `1337;Notification` and for the new output
and idle logic (`activity.rs`).

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| Settings › Profiles › Terminal: a Notifications section with iTerm2's values; the switches are saved | ✓ | `fase41-ajustes.png` |
| `OSC 9;hola desde OSC 9` in the shown tab: «Alert», «Session root@vm: ~ #1: hola desde OSC 9» | ✓ | `fase41-osc9.png` |
| `OSC 9;4;1;50` does not alert | ✓ | — |
| `1337;Notification` with title, subtitle and message: «Hecho» and «make» plus «todo listo» | ✓ | `fase41-osc9.png` |
| A background tab with `sleep 3; echo salida; printf '\a'`: Bell and New Output on arrival, Idle 3 s later | ✓ | `fase41-fondo.png` |
| The new tab, which is shown, does not alert about its own output | ✓ | — |
| `exit` in a background tab: «Session Ended», «Session “root@vm: ~” in tab #1 just terminated.» | ✓ | — |
| With Suppress alerts from active session, an `OSC 9` in the active session does not show; with it off, it does | ✓ | — |

**Not tested:** the kit as is; an inactive window as the reason to alert
(headless sway does not move focus between windows in the test); Malaysia,
which has no notification server yet.

**Differs from iTerm2:** the filters sit in the same section, not in the
Filter Alerts popover; there are no per-kind suppressed alerts («Suppressed
Alerts…»), no custom session-end text (`sessionEndMessageText`), no
`simpleNotifications` and no `1337;Notification` image; the subtitle goes on
the first line of the body; output after a window resize is not discarded;
nothing is counted on a Dock badge or sent to the iPhone.

## Phase 42 — Session indicators

### Plan

iTerm2's top-right corner indicators (`iTermIndicatorsHelper.m` and
`PTYTextView.m -updateIndicators`), for the states CurseTerm already has:

1. **Which ones**, in iTerm2's order with the first rightmost: maximized
   pane, receives broadcast input, coprocess, Alert on Next Mark and copy
   mode.
2. **Drawing:** 26-point squares with a sixth-of-a-side radius, the symbol 4
   points smaller, 4 points apart and 5 from the top, all at half opacity;
   black with a light grey border and a white symbol on a dark terminal
   background, and the other way round on a light one. Metrics in the
   `INDICATOR` token and symbols drawn on the canvas (BK-024).
3. **Test** on headless sway and documentation.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| ⌘D, ⌥⌘A, ⌥⌘I, ⇧⌘↩ and ⇧⌘C: from right to left maximized, broadcast, eye and copy mode | ✓ | `fase42-indicadores.png` |
| ⌥⌘A in the first pane does not mark the second: the indicator belongs to its session | ✓ | — |
| Escape leaves copy mode and its indicator goes; Run Coprocess `cat > /dev/null` adds the coprocess one between broadcast and the eye | ✓ | `fase42-coproceso.png` |
| Solarized Dark: black squares with a white symbol | ✓ | — |

**Not tested:** the kit as is; a light background scheme.

**Differs from iTerm2:** there are no indicators for states CurseTerm lacks
(filter, suppressed output, zoom, debug logging, secure input, pinned
window, AI, channels, buffered input, screenshots); clicking an indicator
shows no help; stroke ends are not round; the symbols are our own drawings,
not SF Symbols.

## Phase 43 — Window themes

### Plan

iTerm2's Settings › Appearance › General › Theme (`iTermTheme.m`,
`PSMYosemiteTabStyle.m`, `PSMDarkTabStyle.m`, `PSMLightHighContrastTabStyle.m`,
`PSMDarkHighContrastTabStyle.m`, `PSMMinimalTabStyle.m`,
`iTermRootTerminalView.m` and `PseudoTerminal.m`), with the classic tab family,
the one iTerm2 uses before macOS 26 and always for Compact:

1. **Theme setting** with the seven themes in the order of iTerm2's menu;
   Regular by default.
2. **Tab bar** like `PSMTabBarControl`: 24 points, equal tabs that fill the
   bar (minimum 75), close button on the left on hover only, a centered
   11-point title (12 in high contrast), 1-point separators, a top line over
   unselected tabs, a 4-point shadow and a 24-point «+». Colors of Light,
   Dark and both high-contrast styles in tokens (`TAB_STYLE_*`).
3. **Regular and Compact** follow the system appearance; **Light, Dark** and
   the high-contrast ones force it on the whole window, title bar included
   (`use_appearance`).
   With CSD, the title bar is a macOS window's (iTerm2 uses a plain
   `NSWindow`): 28 points, a centered bold 13-point title,
   `windowBackgroundColor` (236 light, 50 dark), no line below when there are
   tabs and `iTermRootTerminalView.m`'s 1-point division view when there are
   none. The tabs take that same background, so the selected one continues
   the title bar.
4. **Compact and Minimal** (with CSD): no title bar; the window buttons and
   the tabs share one strip with 75 points on the left for the buttons, and
   with a single tab a fake title bar as tall, with the centered 13-point
   title. Dragging the strip moves the window.
5. **Minimal:** the strip is 38 points and takes the terminal background;
   unselected tabs are 0.05 darker (lighter on near black), with no
   separators, a 20 % outline notch and 0.95 or 0.05 text by the background.
6. **Test:** sway does not accept CSD (it answers SSD even when the app asks
   for CSD and with `border csd`), so Compact and Minimal are tested on
   headless weston, which has no `xdg-decoration`.

### Result (2026-11-06)

Tested with the patched kit on headless weston 16 (CSD) and on sway (SSD).
`./dev.sh check` clean in both modes; a new test for Minimal's colors.

**Works** (screenshots in [`capturas/`](capturas/); `fase43-temas.png`, top to
bottom: Regular with two tabs and with one, Compact with two and with one,
Minimal with two and with one, Light, Dark, Light HC and Dark HC):

| Test | Result | Screenshot |
|---|---|---|
| Settings › Appearance: Theme menu with the seven themes, Regular checked | ✓ | `fase43-menu-tema.png` |
| Choosing Dark on sway changes the tab bar at once, with the active window text (1.0 and 0.65) | ✓ | — |
| The nine cases of `fase43-temas.png` on weston | ✓ | `fase43-temas.png` |
| Minimal: a 38-point strip with the Solarized Dark background, centered buttons, a notch around the selected tab and a white «+» | ✓ | `fase43-minimal.png` |
| Dark and Dark HC force the dark title bar; Light and Light HC, the light one | ✓ | `fase43-temas.png` |
| Regular: a 28-point bar with the bold title; the selected tab is the same `#ECECEC` as the bar (it was white under the kit's 44 px bar); with one tab, the 1-point line | ✓ | `fase43-temas.png` |
| On sway (SSD), Compact and Minimal keep the compositor's bar on top, with their tab colors | ✓ | — |

**Not tested:** hover (close button and hover color) and dragging or
double-clicking the strip, because weston has no virtual keyboard or pointer;
active-window colors on weston (it does not focus the window, so the inactive
ones show); the kit as is; Malaysia.

**Differs from iTerm2:** the macOS 26 (Tahoe) styles are not there; hover
changes at once, without the 0.2 s animation; Regular and Compact do not
switch to high contrast by themselves with the system's «Increase contrast»;
pane dividers do not take the theme's colors; Minimal ignores transparency
and the dimming of background windows; there are no tab colors, tab numbers,
subtitles or overflow menu; the title bar and tabs use flat
`windowBackgroundColor`, without macOS's translucent material; the window
buttons are the kit's.

## Phase 44 — Duplicating, even panes and selecting the command

### Plan

Four entries of iTerm2's menus:

1. **Duplicate Tab** (Shell) copies the whole tab, with its panes, profiles
   and folders (`PseudoTerminal.m -createDuplicateOfTab:`). It used to open a
   tab with only the active session, which is what **Duplicate Session**
   (Session, `-duplicateSession:`) does.
2. **Arrange Split Panes Evenly** (Window) gives every pane the same share,
   at every level (`PTYTab.m -arrangeSplitPanesEvenlyInSplitView:`). It is
   only enabled with panes.
3. **Select Current Command** (Edit) selects, at a prompt, what was typed
   since `OSC 133;B`, and otherwise the last command, from `133;B` to
   `133;C` (`PTYSession.m -textViewRangeOfCurrentCommand`). `133;B` is now
   read.
4. **Test** on sway and documentation.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes; the `OSC 133` test now checks `133;B`.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| ⌘D twice leaves panes of 1/2, 1/4 and 1/4; Arrange Split Panes Evenly makes them thirds | ✓ | `fase44-paneles.png` (top and middle) |
| Duplicate Tab opens a tab with the same three panes | ✓ | `fase44-paneles.png` (bottom) |
| Duplicate Session and Arrange Split Panes Evenly appear in their menus; the latter is disabled without panes | ✓ | — |
| With a `PS1` that emits `133;B` and `echo hola` typed: Select Current Command is enabled and, after Copy, the clipboard holds what was typed since the mark | ✓ | — |
| The same in the first pane of a duplicated tab | ✓ | — |

**Not tested:** the kit as is; the last finished command, which needs a
shell that emits `133;C`; a multi-line command.

**Differs from iTerm2:** the selection is not copied by itself even with
copy on select on; continuation prompts (PS2) and the right prompt are not
taken out of the range; Duplicate Tab activates the first pane, not the one
that was active; Duplicate Window and Replace Selection (Base64,
JSON) are not there.

## Phase 45 — Filter

### Plan

iTerm2's Edit › Find › Filter (⌥⇧⌘F) (`PseudoTerminal.m -filter:`,
`PTYSession.m -setFilter:`, `AsyncFilter.swift`):

1. **Filter field** in a bar under the find bar, focused when it opens;
   Escape or the button close it and drop the filter.
2. **The pane** shows only the logical lines (with their wrapping) that hold
   the text, case-sensitive only when the text has a capital (smart case),
   the newest at the bottom, and it updates with the output.
3. **Filter indicator** between Alert and Copy Mode, as in
   `iTermIndicatorsHelper.m`.
4. **Test** on sway and documentation.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| 60 `linea N` lines, ⌥⇧⌘F and «linea 1»: `linea 1` and `linea 10`–`19` remain, with the filter indicator | ✓ | `fase45-filtro.png` |
| With «linea 10» and `(sleep 3; echo linea 1000) &`, the new line shows up by itself with the filter open | ✓ | `fase45-vivo.png` (top) |
| Escape closes the bar and the normal screen comes back | ✓ | `fase45-vivo.png` (bottom) |

**Bugs found while testing:** the app froze because two functions locked
`extras` and then the terminal, the other way round from the reader thread:
`current_command` (phase 44, Select Current Command, which the menu asks on
every repaint) and the session indicators (phase 42, copy mode). They now
take the data out of `extras` before locking the terminal.

**Not tested:** the kit as is; a 100,000-line history (the filter walks all
of it on every repaint).

**Differs from iTerm2:** it is not a separate session: the result cannot be
scrolled, searched or selected; no regular-expression mode nor progress bar;
the filter follows the active pane, not the session where it opened; the bar sits under the find bar
instead of inside the same drop-down.

## Phase 46 — Warning before a multi-line paste

### Plan

iTerm2's Edit › Paste Special (`iTermPasteHelper.m
-maybeWarnAboutMultiLinePaste:`, `iTermApplicationDelegate.m
-toggleMultiLinePasteWarning:`):

1. **Three checkboxes**, with iTerm2's values: Warn Before Multi-Line Paste
   (on), Limit Multi-Line Paste Warning to Shell Prompt (on; disabled when
   the first is off) and Warn Before Pasting One Line Ending in a Newline at
   Shell Prompt (on).
2. **The warning** «Confirm Multi-Line Paste» with «OK to paste N lines at
   shell prompt?», «OK to paste N lines?» or the one-line-with-newline ones,
   by what is pasted and whether the session is at the prompt (between
   `133;B` and `133;C`).
3. **Paste** only after Paste; Cancel pastes nothing.
4. **Test** and documentation; a test for the decision.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes; a new test for when to warn.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `echo uno` and `echo dos` on the clipboard and ⌘V with no marked prompt: pasted without asking (warning limited to the prompt) | ✓ | — |
| With a `PS1` that emits `133;B`, ⌘V: «Confirm Multi-Line Paste», «OK to paste 2 lines at shell prompt?», Cancel and Paste | ✓ | `fase46-aviso-pegar.png` |
| Paste runs both commands | ✓ | — |

**Not tested:** the kit as is; Cancel; the checkboxes from the menu; the
one-line-with-newline cases (they are in the test).

**Differs from iTerm2:** no Paste Without Newline, Advanced… nor «don't ask
again» (BK-037); with a shell
that only emits `133;B` the session stays «at the prompt» while a command
runs too.

**Fix (2026-11-06):** Paste Slowly now goes past the same warning and, after
Paste, pastes slowly (tested on sway with two lines at a marked prompt);
Paste Escaped already warned.

## Phase 47 — Advanced Paste

### Plan

iTerm2's Edit › Paste Special › Advanced Paste… (⌥⌘V)
(`iTermPasteSpecialViewController.xib` and `iTermPasteHelper.m
+sanitizePasteEvent:encoding:`):

1. **Transforms** in iTerm2's order, in a pure module with tests
   (`paste_transform.rs`): remove newlines or convert them to CR, Unicode
   punctuation to ASCII (quotes and dashes), remove control codes except
   tab, LF, FF and CR, tabs (as is, ^V or 4 spaces), escape the shell with
   `\` (leaving tabs escaped with ^V alone), regular-expression substitution
   (`$1`, and `\n`, `\t`, `\r`, `\\` as in vim) and Base64 in 77-character
   lines with CR.
2. **The panel**, with iTerm2's defaults (CR and removing control codes on),
   bracketed paste mode, a preview of the result and Cancel or Paste.
3. **Test** and documentation.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes; two new tests for the transforms.

**Works** (screenshots in [`capturas/`](capturas/)):

| Test | Result | Screenshot |
|---|---|---|
| `echo “hola mundo”<tab>—fin` on the clipboard and ⌥⌘V: panel with CR and control codes on | ✓ | `fase47-pegado-avanzado.png` |
| Unicode punctuation and «Convert tabs to 4 spaces»: the preview becomes `echo "hola mundo"    -fin` | ✓ | `fase47-resultado.png` (top) |
| Paste writes exactly that into the shell | ✓ | `fase47-resultado.png` (bottom) |

**Not tested:** the kit as is; Base64, the regular expression and shell
escaping from the panel (they are in the tests).

**Differs from iTerm2:** no Speed section (chunk size and delay) nor «Wait
for shell prompt before pasting each line»; the original text cannot be
edited in the panel; the number of spaces is fixed (4), with no control to
change it; the regular expression is the `regex` crate's, not ICU's.

## Phase 48 — Duplicate Window

### Plan

iTerm2's Shell › Duplicate Window (`PseudoTerminal.m -duplicateWindow:` and
`-arrangementForDuplicateWindow`): a new window with the arrangement of
every tab (panes, profiles and folders), without the contents. The
arrangement travels as TOML with `--layout`, so it works the same in the
same process (patched kit) as launching another one (kit as is).

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result | Screenshot |
|---|---|---|
| Two panes, the second in `/tmp`, and Duplicate Window: another window with both panes and the second in `/tmp` | ✓ | `fase48-duplicar-ventana.png` |

**Not tested:** the kit as is (another process).

**Differs from iTerm2:** the new window copies neither the size and position
nor the window's own title.

## Phase 49 — Full paste warnings

### Plan

iTerm2's two paste warnings with all their buttons, in a sheet of its own
because the kit's `Alert` only holds two (BK-037):

1. **Tabs** (`PTYSession.m -pasteString:flags:` and `iTermPasteHelper.m
   +pasteWithTabsWarningWithAccessoryController:`): when the program does not
   ask for bracketed paste and the text has tabs, «You're about to paste a
   string with tabs.» with OK, Cancel, Convert tabs to spaces (with the number
   of spaces, 4 by default and saved) and Advanced….
2. **Multiple lines** (phase 46): Paste, Paste Without Newline (only for one
   line ending in a newline), Cancel and Advanced….
3. **iTerm2's order:** tabs first, then lines.

### Result (2026-11-06)

Tested on headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result | Screenshot |
|---|---|---|
| `echo a<tab>b` and ⌘V: the tabs sheet with its four buttons | ✓ | `fase49-avisos-pegado.png` (bottom) |
| Raising it to 6 spaces and Convert tabs to spaces: `echo a      b` is pasted and `paste_tab_spaces = 6` is saved | ✓ | — |
| `echo sin salto` with a trailing newline at a marked prompt: Paste, Paste Without Newline, Cancel and Advanced… | ✓ | `fase49-avisos-pegado.png` (top) |
| Paste Without Newline leaves the command typed without running it | ✓ | — |

**Not tested:** the kit as is; Advanced… from the warnings; OK and Cancel in
the tabs sheet.

**Differs from iTerm2:** no «don't ask again» in the warnings; the sheet is
our own, not an `NSAlert`.

## Phase 50 — Find modes and Clear Find

### Plan

iTerm2's find mode (`iTermFindViewController.h` `iTermFindMode`,
`iTermFindDriver.m` and `FindView.xib`) and Edit › Find › Clear Find
(`iTermApplicationDelegate.m -clearFindString:`):

1. **Five modes:** Smart Case Substring (the default: case-sensitive only if
   the query has a capital), Case-Sensitive Substring, Case-Insensitive
   Substring, Case-Sensitive Regex and Case-Insensitive Regex. Find and Filter
   each save their own, like `findMode_iTerm` and `NoSyncFilterMode`. The mode
   is chosen in a `PopUpButton` beside the field (BK-038).
2. **Filter with expressions:** the filter stops comparing text and uses the
   same pattern as Find.
3. **Clear Find (⌥⌘E):** clears the query; it is enabled only when there is
   something to clear.
4. **iTerm2's Find submenu order:** Find URLs, Find All Smart Selection
   Matches and Pick Result to Open in their group, and Filter last.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes; a new test checks each mode with the `regex` crate and that
`alacritty_terminal`'s search accepts its patterns.

| Test | Result | Screenshot |
|---|---|---|
| `error` in Smart Case: 13 results with `Error`, `error` and `ERROR` | ✓ | — |
| Case-Sensitive Substring: 5 of 5, only `error`; the search reruns when the mode changes | ✓ | `fase50-distingue.png` |
| ⌥⌘E leaves the field empty and without highlights | ✓ | — |
| `FO+.BAR` in Case-Insensitive Regex: `foo.bar` and `fooxbar` | ✓ | `fase50-regex.png` |
| `find_mode` and `filter_mode` are saved separately in the `.domain` | ✓ | — |
| Filter `^E` in Case-Sensitive Regex: only `Error one` and `ERROR 3` | ✓ | `fase50-filtro.png` |

**Not tested:** the kit as is; Clear Find enabled or disabled in the D-Bus
menu.

**Differs from iTerm2:** the mode is in a `PopUpButton`, not in the
magnifier's menu (BK-038); no Erase Search History.

## Phase 51 — Find Globally

### Plan

iTerm2's Edit › Find › Find Globally… (⇧⌘F)
(`iTermGlobalSearchWindowController.m`, `iTermGlobalSearch.xib`,
`iTermGlobalSearchEngineCursor.m` and `iTermGlobalSearchResult.m`):

1. **Panel** of 741 × 567 with a title, the field, the mode (Substring —
   Automatic, Substring — Case Sensitive, Substring — Case Insensitive, Regex
   — Case Sensitive and Regex — Case Insensitive; saved separately, like
   `globalSearchMode`) and the list. It lives inside the window because the
   kit has no floating panel nor outline view (BK-039).
2. **Results** from each session of the window, tab by tab: the session name
   and under it one snippet per match. Each snippet has up to 20 characters
   before it on the same logical line, the match underlined on yellow at
   0.35, and up to 256 characters after it, with «…» if it goes on.
3. **Reveal:** pressing a result, or moving with the arrows from the field,
   focuses its session, scrolls to it and selects it. Escape closes the panel.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result | Screenshot |
|---|---|---|
| Two tabs, ⇧⌘F and `needle`: two groups with two snippets each, with text before and after | ✓ | `fase51-panel.png` |
| Pressing the first result: switches to the first tab, scrolls up into the history and selects `needle` | ✓ | `fase51-mostrar.png` |
| Two down arrows from the field: the third result, in the other tab | ✓ | — |
| Escape closes the panel | ✓ | — |
| Substring — Case Sensitive: only the tab with lowercase `needle` stays; `global_search_mode` is saved | ✓ | `fase51-modo.png` |

**Bugs found and fixed while testing:** the yellow background filled the
whole row and covered the text after it; the spaces between the snippet's
three parts vanished (they are drawn as no-break spaces); and
`text_decoration` does not underline a label in Freya 0.4.3, so the underline
is a line of our own.

**Not tested:** the kit as is; a snippet longer than 256 characters.

**Differs from iTerm2:** it is a panel inside the window, not a floating
window, and it cannot be moved (BK-039). It searches only the sessions of its
window, all at once rather than bit by bit, without a progress bar. It does
not include the alternate screen nor folded content.

## Phase 52 — Select Matches

### Plan

iTerm2's Edit › Find › Select Matches (`PTYTextView+ARC.m
-convertMatchesToSelections` and `iTermSelection.m
-enumerateSelectedAbsoluteRanges:`):

1. **Selections:** each result of the current search, over the whole history,
   becomes a selection. `alacritty_terminal` has only one selection, so the
   others are kept in the session and painted as selected while there is no
   other selection.
2. **Copy:** ⌘C joins the selections, each on its own line.
3. **Clear:** starting another selection, typing or resetting the session
   removes them.
4. **Menu:** enabled only with a search in progress, like Pick Result to
   Open.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result | Screenshot |
|---|---|---|
| ⌘F `red` and Select Matches through the D-Bus menu: all six `red` selected | ✓ | `fase52-selecciones.png` |
| ⌘C: `wl-paste` gives six `red` lines | ✓ | — |
| Typing a letter removes the selections | ✓ | — |

**Not tested:** the kit as is; Copy with Styles with several selections.

**Differs from iTerm2:** Copy with Styles copies only the normal selection;
the selections do not follow the text when the history drops lines, they are
cleared.

## Phase 53 — Snippets

### Plan

iTerm2's snippets (`iTermSnippetsModel.m`, `iTermSnippetsMenuController.m`,
`PTYTextView+ARC.m -sendSnippet:`, `iTermKeyBindingAction.m
+escapedText:mode:`, `iTermSnippetsEditingViewController.xib` and
`iTermEditSnippetWindowController.xib`):

1. **Data:** title, value, escaping and tags, saved in the `.domain`.
   Escaping: Unescaped Literal Text; Backward Compatibility Escaping (`\n`,
   `\e`, `\a` and `\t`); and C-Style Backslash Escaping, the one new
   snippets use, which adds `\b`, `\r`, `\\` and `\x` with two digits.
2. **Edit › Snippets:** a disabled «Press Alt to Edit Before Sending», the
   untagged snippets and one folder per tag, split at `/` and sorted by name.
   Without a title the value shows, cut to 30 characters.
3. **Send:** the value with its escapes goes to the active session. With Alt
   held it opens in Advanced Paste.
4. **Settings › Shortcuts › Snippets:** the Title and Text table, +, − and
   Edit, and the Edit Snippet sheet with Title, a multi-line Value, Escaping
   and Tags.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes; two new tests check the escapes and the cut title.

| Test | Result | Screenshot |
|---|---|---|
| D-Bus menu: the disabled hint, `Hola` and `git › log` with the cut untitled snippet | ✓ | — |
| `Hola` (`echo hola\tmundo\n`): arrives with a tab and runs | ✓ | `fase53-enviar.png` |
| The snippet without `\n` stays typed without running | ✓ | `fase53-enviar.png` |
| With Alt held, `Hola` opens Advanced Paste with the text already expanded | ✓ | `fase53-alt.png` |
| The Shortcuts tab lists the snippets | ✓ | `fase53-ajustes.png` |
| +, Title, a two-line Value, Tags and OK: saved in the `.domain`; Edit loads it again | ✓ | `fase53-editar.png` |
| − removes the chosen snippet | ✓ | — |

**Bug found and fixed while testing:** a click in the empty part of Value did
not focus it, because the text view measured only its text. The `text_view`
block now takes `height` (BK-036).

**Not tested:** the kit as is; that an open window's menu changes when the
snippets are edited in Settings.

**Differs from iTerm2:** Edit Snippet is a sheet, not a window; tags are
typed separated by commas, not in a token field; no Snippets toolbelt tool,
no per-profile tag filter, no Open Quickly, no drag to reorder and no Actions
tab.

## Phase 54 — Review: bugs in phases 50 to 53

### Plan

Look for bugs in the latest work, check it against iTerm2's and
`alacritty_terminal`'s code, and fix what reproduces.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Bug | Cause | Fix | Test | Screenshot |
|---|---|---|---|---|
| A snippet reached only the active session with Broadcast Input on | `PTYSession.m -writeTask:` broadcasts it (hence `-writeTaskNoBroadcast:`); ours wrote to one session | The snippet goes to every session that hears the active one | Two panes, Broadcast Input to All Panes in Current Tab and `Hola`: it shows in both | `fase54-difusion.png` |
| A tab was drawn as a box | `alacritty_terminal` (`term/mod.rs`, `put_tab`) keeps a `'\t'` in the first cell so copying gives the tab back, and the grid painted it as a glyph | The grid treats that cell as a space | `printf 'a\tb\tc\n'`: columns without boxes | `fase54-tab.png` |

**Checked, no bug:** a pattern that can match nothing (`a*`, `^`, `x?`) in
the regex modes does not hang the search; a new test covers it.

**Not tested:** the kit as is.

## Phase 55 — Session Note

### Plan

iTerm2's View › Show Session Note (⌥⌘G) (`SessionNoteModel.swift`,
`SessionNoteView.swift`, `SessionView.m -ensureSessionNoteViewWithModel:` and
`PTYSession.swift -textViewEditSessionNote`):

1. **One note per session:** the text, the frame, whether it is collapsed and
   whether it shows. The menu checkbox shows it (expanded and focused) or
   hides it.
2. **Look:** 280 × 180, 20 points from the top-right corner, a 28-point title
   bar with «Session Note» and a chevron, radius 8, a black shadow at 0.3 and,
   in dark mode, a grey border. The text uses the terminal font.
3. **Move and resize:** the title bar moves it; the edges, with a 5-point
   strip, resize it down to 150 × 80.
4. **Collapse:** the chevron or ⇧↩ leave only the title bar, with the first
   line as title, and give focus back to the terminal.
5. **Empty:** if it loses focus without text, it hides.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes; a new test checks dragging, the minimums and the initial position.

| Test | Result | Screenshot |
|---|---|---|
| ⌥⌘G and two lines: the note at the top right, with a shadow | ✓ | `fase55-nota.png` |
| Dragging the title bar: it moves −300 and +242 | ✓ | — |
| Bottom, left and right edges: the size changes | ✓ | `fase55-redimensionar.png` |
| The chevron collapses it with `Recordar:` as title | ✓ | `fase55-plegada.png` |
| ⇧↩ collapses it and what is typed next goes to the shell | ✓ | — |
| ⌥⌘G hides it and again shows it with its text and frame | ✓ | — |
| An empty note goes away when the terminal is clicked | ✓ | — |

**Bugs found and fixed while testing:** the background and shadow did not
show, because Freya paints the terminal canvas over backgrounds, and they are
now drawn on a canvas of their own; the side edges did not respond because
`Size::fill()` gave an absolute strip a height of 0, and they now use
`Size::percent(100.0)`; ⇧↩ also reached the shell (BK-027), and editing now
ends one loop later.

**Not tested:** the kit as is; the dark theme.

**Differs from iTerm2:** no material blur, no collapse animation, not saved
in window arrangements and no API; corners do not resize on both axes at
once.

## Phase 56 — Key reporting modes

### Plan

iTerm2's Session › Terminal State (`MainMenu.xib`, `PTYSession.m
-updateKeyMapper` and `-screenSendModifiersDidChange`) and its encoders
(`iTermModifyOtherKeysMapper.m`, `iTermModifyOtherKeysMapper1.m`,
`iTermTermkeyKeyMapper.m` and `iTermModernKeyMapper.swift`):

1. **Session mode** (radio): Standard Key Reporting Mode, modifyOtherKeys
   Mode 1, modifyOtherKeys Mode 2 and CSI u Mode. An app changes it with
   `CSI > 4 ; n m`; `alacritty_terminal` does not keep that sequence, so the
   reader thread looks for it in the output.
2. **kitty keyboard protocol** (checkboxes): Disambiguate Escape, Report All
   Event Types, Report Alternate Keys, Report All Keys as Escape Codes and
   Report Associated Text. `alacritty_terminal` keeps the flags and answers
   `CSI ? u` with `kitty_keyboard`; CurseTerm encodes keys, repeats and
   releases. With any flag set, kitty wins, as in iTerm2.
3. **Profile › Keys:** Report keys using CSI u (off) and Apps can change how
   keys are reported (on), with `iTermProfilePreferences.m`'s defaults.
4. **Raw Key Reporting Mode** is not ported: it reports macOS key codes.

### Result (2026-11-06)

Tested in headless sway with the patched kit, with a raw-mode Python reader.
`./dev.sh check` clean in both modes; new tests for each encoder and for the
`CSI > 4 ; n m` detector split across two reads.

| Test | Bytes received | Screenshot |
|---|---|---|
| No modes: Ctrl+A and Escape | `\x01`, `\x1b` | — |
| `CSI > 1 u`: Ctrl+A, Escape, Enter and `a` | `\x1b[97;5u`, `\x1b[27u`, `\r`, `a` | `fase56-kitty.png` |
| `CSI ? u` | the reply `CSI ? 1 u` | `fase56-kitty.png` |
| `CSI = 3 u` and `a` | `a` and on release `\x1b[97;1:3u` | — |
| The D-Bus menu checkboxes follow the flags and turning them off there clears them | ✓ | — |
| `CSI > 4 ; 2 m`: Ctrl+A, ⇧↩ and `A`; the radio moves to modifyOtherKeys Mode 2 | `\x1b[27;5;97~`, `\x1b[27;2;13~`, `A` | `fase56-mok.png` |
| CSI u Mode from the menu: Ctrl+I, Ctrl+A, ⇧⇥, Ctrl+⇧+A and Ctrl+Alt+X | `\x1b[105;5u`, `\x01`, `\x1b[Z`, `\x1b[65;5u`, `\x1b[120;7u` | `fase56-csiu.png` |
| Report keys using CSI u in the profile: Ctrl+I and ⇧↩ in a new session | `\x1b[105;5u`, `\x1b[13;2u` | — |

**Bug found and fixed while testing:** with Control held the key arrives
without Shift applied, and Ctrl+⇧+A came out as `97`; iTerm2 uses the
shifted letter (`-charactersIgnoringModifiers`), so CSI u, modifyOtherKeys
and kitty's alternate key uppercase it.

**Not tested:** the kit as is; the profile switches in the Settings window
(the value is, in the `.domain`); Report Alternate Keys and Report
Associated Text with real keys (only in the tests).

**Differs from iTerm2:** no Raw Key Reporting Mode; kitty's base key comes
from the US layout; Super never reaches the shell (ADR 0003), not even with
kitty; no warning when an app leaves key reporting on when it exits.

## Phase 57 — Key bindings

### Plan

iTerm2's Settings › Keys › Key Bindings and Profile › Keys › Key Mappings
(`iTermKeyBindingAction.h`, `iTermKeyMappings.m`, `PTYSession.m
-performKeyBindingAction:event:` and `iTermEditKeyActionWindowController.m`):

1. **Bindings**, global and per profile; the profile's come first. They are
   checked before the key is encoded.
2. **Actions** from iTerm2's groups that CurseTerm has: Ignore, Select Menu
   Item…, New Window/Tab with Profile, Duplicate Tab, Split
   Horizontally/Vertically with Profile, Next/Previous Tab, Move Tab
   Left/Right, Next/Previous Pane, Select Split Pane Above/Below/On Left/On
   Right, the six Scroll ones, Send ^H and ^? Backspace, Send Escape Sequence,
   Send Hex Code, Send Text (C-style escapes), Send Snippet, Run Coprocess,
   Start Instant Replay, Alert on Next Mark, Find Again Down/Up, Copy Mode and
   Toggle Fullscreen and Mouse Reporting. What sends text honours Broadcast
   Input.
3. **Settings:** the Keys tab between Profiles and Pointer, and Key Mappings
   under the profile's Keys section: the Keyboard Shortcut and Action table
   with iTerm2's glyphs (⌃⌥⇧⌘), +, − and Edit, and the sheet with the
   shortcut recorder («Click to Set»), the action and its parameter.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes; a new test checks binding names, the profile's priority and the hex
codes.

| Test | Result | Screenshot |
|---|---|---|
| Global Ctrl+Alt+T, Send Text `echo global\n` | `echo global` runs | `fase57-combinaciones.png` |
| Ctrl+Alt+P global and in the profile with Send Hex Code: the profile wins | `echo perfil` | `fase57-combinaciones.png` |
| Ctrl+Alt+K, New Tab with Profile | a tab opens | — |
| The Keys tab lists the bindings with their glyphs | ✓ | `fase57-ajustes.png` |
| +, record Ctrl+Alt+Y, Send Text and OK: saved and works in the terminal | ✓ | — |
| Select Menu Item…, filter `vertically` and OK without touching the pop-up: saves `split-vertically` and Ctrl+Alt+M splits | ✓ | `fase57-editar.png` |
| Key Mappings in Profile › Keys | ✓ | `fase57-perfil.png` |

**Bugs found and fixed while testing:** the 37-action pop-up did not scroll
and the last ones could not be chosen; there is now a group pop-up and an
action pop-up (BK-040). With Select Menu Item…, OK saved the first entry of
the whole menu if the pop-up was not touched; it now saves the first one
shown.

**Not tested:** the kit as is; the full screen, scroll, pane and Run
Coprocess actions from a binding.

**Differs from iTerm2:** no Bypass Terminal, modifier remapping, recent
tabs, next and previous window, changing colors, selection moves nor scripts
(phases 74, 76 and 77 add the rest); a Super binding that is already a
menu shortcut does both; the Keys icon repeats General's because the kit has
no keyboard symbol (BK-024).

## Phase 58 — Moving and burying sessions

### Plan

iTerm2's Session › Move Session, Bury Session and Buried Sessions
(`MovePaneController.m`, `SplitSelectionView.m`, `PTYSession.m -bury` and
`iTermBuriedSessions.m`):

1. **Move Session to Tab:** the pane leaves its tab and opens a new one.
2. **Move Session to Split Pane:** the source pane turns green with «Select a
   destination pane»; over the others, the half nearest the pointer turns dark
   red with «Click to move source pane to this split», with the
   `splitPane*Color` colors, white and black frames and 0.9 opacity. A click
   leaves it in that half; a click on the source or Escape cancels.
3. **Bury Session (⌥⇧⌘B):** the session leaves the view and keeps running;
   Buried Sessions lists it by name and choosing it brings it back in a new
   tab. The only visible session is not buried, because the window would
   close.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes; new tests for the workspace moves and the nearest half.

| Test | Result | Screenshot |
|---|---|---|
| Two panes and Move Session to Tab: two tabs | ✓ | — |
| Move Session to Split Pane: green source and red left half of the other, with their messages | ✓ | `fase58-origen.png` |
| Click on that half: the moved pane ends up on the left and focused | ✓ | `fase58-movido.png` |
| ⌥⇧⌘B: the session disappears; Bury and Move turn disabled and Buried Sessions lists it | ✓ | `fase58-enterrada.png` |
| Choosing it in Buried Sessions: it comes back in a new tab with its content | ✓ | `fase58-desenterrada.png` |

**Bugs found and fixed while testing:** the overlay was 0 high with
`Size::fill()` in absolute position and now uses `Size::percent(100.0)`; the
source pane's message did not show because the focused terminal's canvas
repaints over the text, so it is drawn on the overlay's canvas.

**Not tested:** the kit as is; cancelling with Escape or a click on the
source; a buried session that ends.

**Differs from iTerm2:** no Move Session to Window, Move Tab to New Window
nor Merge All Windows: without patch 0008 each window is its own process
(BK-001) and with it a session is tied to its window's event channel; buried
sessions belong to each window and are not saved on quit; no moving by
dragging the tab.

## Phase 59 — Full screen, Show Tabs in Fullscreen and Window Style

### Plan

iTerm2's Lion full screen (`PseudoTerminal.m
-toggleTraditionalFullScreenMode:`, `-toggleShowTabsInFullScreen:` and
`-initWithContentRect:…windowType:`), with Malaysia's focus mode as the macOS
menu bar:

1. **No title bar:** in full screen the window loses its title bar and
   traffic lights; the tabs sit at the very top.
2. **Show Tabs in Fullscreen (⇧⌘U):** a View checkbox that hides or shows the
   tabs in full screen; it is saved in the preferences.
3. **Focus mode:** with Malaysia's bar down (pointer at the top edge), the
   title bar slides in over the terminal and fades with the bar's progress
   (`WindowControls::titlebar_overlay_top`); with `fullscreen_push` it pushes
   the tabs instead of covering them.
4. **Profile Window Style:** Normal, Full Screen, Maximized and No Title Bar,
   in Settings › Profiles › Window; it applies when the window opens.

### Result (2026-11-06)

Tested with the patched kit in headless sway (server-side decoration) and
in headless weston 16 (client-side decoration; commands go through the D-Bus
menu, because weston has no virtual keyboard). `./dev.sh check` clean in
both modes.

| Test | Result | Screenshot |
|---|---|---|
| sway, two tabs and ⌃⌘F: no title bar, the tabs at the top | ✓ | `fase59-pantalla-completa.png` |
| weston with kiosk-shell, two tabs in full screen: no title bar nor traffic lights, the tabs at the top | ✓ | `fase59-csd.png` |
| weston, Show Tabs in Fullscreen unchecked: only the terminal | ✓ | — |
| weston, Window Style No Title Bar: a window without a title bar | ✓ | `fase59-sin-barra.png` |
| weston, Window Style Maximized: opens maximized with its bar | ✓ | `fase59-maximizada.png` |
| ⇧⌘U in full screen: the tabs hide; again, they come back | ✓ | — |
| Window Style Full Screen: the window opens in full screen | ✓ | — |
| A single tab in full screen: only the terminal | ✓ | — |

**Bug found and fixed while testing:** with Maximized, weston closed the
connection («xdg_surface geometry (656 x 470) does not match the configured
maximized state»): the kit committed after acking the configure without
applying it. Patch 0012 fixes it (BK-041).

**Not tested:** the kit as is; the title bar revealed by focus mode and
`fullscreen_push`: Malaysia does not start in the lab (llvmpipe lacks
`EGL_ANDROID_native_fence_sync`), and neither sway nor weston has the
`infinity_focus_v1` protocol; ⌃⌘F with weston's desktop-shell: without a
seat, weston does not raise the window to the full-screen layer
(`desktop_surface_committed` activates it once per seat), so it was tested
with kiosk-shell.

**Differs from iTerm2:** no traditional (pre-Lion) full screen nor the Top,
Bottom, Left, Right or Compact styles; with server-side decoration No Title
Bar does not remove the bar the compositor draws.

## Phase 60 — Command history and autocomplete

### Plan

iTerm2's Session › Open Command History… (⇧⌘;) and Open Autocomplete… (⌘;)
(`iTermShellHistoryController.m`, `CommandHistoryPopup.m`, `Autocomplete.m`
and `PseudoTerminal.m -openCommandHistory:`):

1. **History:** with shell integration, the command at `OSC 133;C` goes into
   a history shared by the window's sessions, with its number of uses.
2. **Open Command History:** the commands that start like what is typed at
   the prompt, the most used first; choosing one types the rest.
3. **Open Autocomplete:** the words on the screen and in the history that
   continue the one before the cursor, the nearest first; choosing one types
   the rest.
4. Both use Paste History's picker.

### Result (2026-11-06)

Tested in headless sway with the patched kit, with `PS1=$'$ \e]133;B\a'` and
`PS0=$'\e]133;C\a'`. `./dev.sh check` clean in both modes; a new test checks
the history order and the words.

| Test | Result | Screenshot |
|---|---|---|
| `git status` twice and `git log`; `gi` and Open Command History: `git status` first and `git log` | ✓ | `fase60-historial.png` |
| Enter: the line becomes `git status` | ✓ | — |
| `echo car` and Open Autocomplete: `cargo-build` and `carbonara`; Down and Enter: `echo carbonara` | ✓ | `fase60-autocompletar.png` |

**Bug found and fixed while testing:** Down and Up in the picker crashed the
app: the `peek` guard was still alive during `set` («already borrowed»). It
also happened in Paste History, Open Quickly and Recent Directories.

**Not tested:** the kit as is; the ⌘; and ⇧⌘; shortcuts (the lab's virtual
keyboard has no semicolon).

**Differs from iTerm2:** the history is not saved on quit nor kept per
remote host; no Auto Command Completion nor the Command History toolbelt
tool; a command does not remember its folder or exit code.

## Pass with the kit as is (2026-11-06)

Built without `kit-parches` and tested in headless sway.

| Test | Result |
|---|---|
| ⌘T, ⌘D, Find (⌘F) with «hola»: 1 of 1 and highlighted | ✓ |
| Open Quickly (⇧⌘O) with the three sessions; Down and Escape | ✓ |
| Settings (⌘,) in its own process, Profiles tab | ✓ |
| Instant Replay (⌥⌘B) | ✓ |
| ⌘A and ⌘C: `wl-paste` returns the text | ✓ |
| ⌘N: another window in another process | ✓ |

**Failures seen, already known:** the title stays «CurseTerm» (without patch
0001); closing Settings with ⌘W panics in `close_window` (BK-001, fixed by
patch 0008).

**Detail:** with the Settings window below its minimum width (tiled sway
ignores it), the profile tabs break their text.

## Phase 61 — Triggers like iTerm2's

### Plan

iTerm2's Profiles › Advanced › Triggers (`Trigger.m`, `PTYTriggerEvaluator.m`,
`TriggerController.m` and each `*Trigger.m`), on top of phase 4's triggers:

1. **When:** each logical line (its wrapped rows joined) is checked just
   before the line feed that ends it (`VT100Screen.m -linefeed`). «Instant»
   triggers also look at the unfinished line, at most twice a second, also
   when the output stops (a waiting prompt). An instant trigger fires once per
   line; highlights follow the line as it grows.
2. **Parameters:** `\0` to `\9` with the expression's groups and the escapes
   `\a \b \e \n \r \t \\ \xNN` (`Trigger.m`).
3. **Actions**, in iTerm2's order by title: Annotate, Highlight Line,
   Highlight Text, Make Hyperlink, Post Notification, Prompt Detected, Report
   Directory, Ring Bell, Run Command, Run Coprocess, Send Text, Set Mark, Set
   Title, Show Alert and Stop Processing Triggers.
4. **Highlight:** a text and a background colour, each optional, saved as
   `{#rrggbb,#rrggbb}`; a new one paints the background red. They replace the
   cell's colours.
5. **Show Alert:** OK, Show Session and Disable This Alert, which turns it off
   in that session; alerts and notifications, at most one a second per
   trigger.
6. **Settings:** a table with Enabled, the expression, the action, the
   parameter (text or the two colours) and Instant, plus «+» and «−».

### Result (2026-11-06)

Tested in headless sway with the patched kit, a profile with
`scrollback_lines = 20` and a script that prints each case. `./dev.sh check`
clean in both modes; new tests for the parameters, the colours and the
once-per-line rule.

| Test | Result | Screenshot |
|---|---|---|
| `error ([0-9]+)` white on red, also at the end of a 115-character line | ✓ | `fase61-triggers.png` |
| `^WARN` with Highlight Line: the whole row yellow | ✓ | `fase61-triggers.png` |
| With a full history (60 lines, 20 of history), each highlight stays on its line | ✓ | `fase61-triggers.png` |
| `title:(\w+)` with Set Title `T-\1`: title «T-Probando» | ✓ | — |
| `read -p "continue? "` and Instant Send Text `yes\n`: `answer=yes` without touching the keyboard | ✓ | `fase61-triggers.png` |
| Annotate «annotated» over «note this» | ✓ | `fase61-triggers.png` |
| Run Command `touch …`: the file appears | ✓ | — |
| `STOP` with Stop ahead of a `STOP\|later` highlight: the STOP line stays plain and «only later here» is highlighted | ✓ | `fase61-aviso.png` |
| Show Alert `Alert: \0`: an alert with the three buttons; Disable This Alert and another `ALERTME`: no alert | ✓ | `fase61-aviso.png` |
| Post Notification `Saw \0`: `Notify` with «Saw NOTIFYME» and «A trigger fired in session “root@vm: ~” in tab #1.» | ✓ | — |
| Make Hyperlink `https://example.com/issues/\1` and Super+click: `xdg-open https://example.com/issues/123` | ✓ | — |
| An unchecked trigger does nothing | ✓ | — |
| Settings: the table; checking Enabled and picking a Cyan background saves `enabled = true` and `{,#00ffff}` | ✓ | `fase61-ajustes.png` |

**Bugs found and fixed while testing:**
- With a full history, triggers stopped looking at new lines and highlights
  drifted: `alacritty_terminal` does not count the lines it drops. Now a line
  is checked before its line feed, and a line feed that drops the top row
  into the history moves everything anchored up one. That also fixes, for
  normal output, phase 35's drift of marks and annotations.
- A waiting prompt never got its Instant trigger because the line was only
  checked when output arrived; now it is also checked 0.5 s after the output
  stops.
- In Settings, the columns changed width with the action; the table spans the
  full width with fixed action and parameter columns.

**Not tested:** the kit as is; Prompt Detected, Report Directory, Set Mark and
Run Coprocess; Highlight Line on a wrapped line; drift with scrolls that are
not line feeds (`ESC D`, `CSI S`).

**Differs from iTerm2:** no actions that need macOS, the Python API or the
browser (Bounce Dock Icon, Open Password Manager, Invoke Script Function,
Capture Output, Set User Variable and the workgroup ones); no Run Silent
Coprocess; no trigger names, job filter or «Use interpolated strings»; Set
Mark without «Stop scrolling»; the highlight colours are AppKit's fourteen
named ones, not a free picker; an alert arriving while another is open is
dropped instead of waiting.

## Phase 62 — Settings › Arrangements and the default arrangement

### Plan

iTerm2's Arrangements pane (`WindowArrangements.m`,
`ArrangementPreviewView.m`) and Startup › Open default window arrangement
(`GeneralPreferencesViewController.m`, `iTermApplicationDelegate.m`):

1. **List** of the saved arrangements, by name, with ★ on the default one;
   the first one saved becomes it.
2. **Set Default**, **−** to delete (if it was the default, the next one
   takes over) and the editable **name**, which refuses one already used.
3. **Preview:** the screen in blue on grey, and on it the window with its
   tabs and the first tab's panes.
4. **Window › Restore Window Arrangement** sorts by name and puts ⇧⌘R on the
   default one.
5. **Settings › General › Startup:** with the checkbox, the first window
   CurseTerm opens restores the default arrangement.

### Result (2026-11-06)

Tested in headless sway with the patched kit and two arrangements written in
the `.domain`. `./dev.sh check` clean in both modes.

| Test | Result | Screenshot |
|---|---|---|
| The list sorts «Aaa simple» before «Trabajo»; the preview of «Trabajo» shows the tabs «editor» and «Tab 2» and the three panes at 60/40 | ✓ | `fase62-disposiciones.png` |
| Set Default: ★ and `default_arrangement` in the `.domain`; the button greys out | ✓ | — |
| Typing « y mas cosas» at the end of the name: it is saved whole and the ★ follows it | ✓ | — |
| − on «Aaa simple»: one arrangement is left | ✓ | — |
| General › Startup on and CurseTerm opened without arguments: two tabs, the first with its three panes | ✓ | `fase62-arranque.png` |
| ⇧⌘R: another window with the default arrangement | ✓ | — |

**Bug found and fixed while testing:** typing fast in the name lost or
shuffled letters: each keystroke renamed and the selection followed the
name, which the `.domain` did not have yet. Now the selection follows the
position in the saved list and the field only starts over when another
arrangement is chosen.

**Not tested:** the kit as is; deleting the default arrangement.

**Differs from iTerm2:** the name is changed in a field, not in the table
itself, and a name already used is refused instead of asking whether to
replace it; no undo; the preview does not know the real screens or the
window's position (Wayland gives neither); after restoring, the last tab is
active, not the one that was when saving.

## Phase 63 — Shortcuts › Actions and Edit › Actions

### Plan

iTerm2's actions (`iTermActionsModel.m`,
`iTermActionsEditingViewController.m`, `iTermActionsMenuController.m` and
`iTermShortcutsViewController.m`):

1. **An action** is a key action with a title instead of a shortcut: the same
   actions and parameters as Keys › Key Bindings.
2. **Settings › Shortcuts** now has two tabs, Actions and Snippets. In
   Actions, the Title and Action table with «+», «−» and Edit, and the Edit
   Action sheet with the title, the action and its parameter (the same editor
   as the keys).
3. **Edit › Actions** lists the actions in order; choosing one applies it to
   the active session.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result | Screenshot |
|---|---|---|
| Action «Saludar», Send Text `echo hola desde accion\n`: Edit › Actions lists it and, over D-Bus, types and runs the command | ✓ | — |
| Settings › Shortcuts › Actions: the table with «Saludar» and its action | ✓ | `fase63-acciones.png` |
| «+»: the Edit Action sheet with Title, Action and OK disabled until a title is typed; «Nada» and OK saves it | ✓ | `fase63-editar-accion.png` |

**Not tested:** the kit as is; Edit and «−» on an action; a menu or profile
action.

**Differs from iTerm2:** Open Quickly, the status bar and the toolbelt's
Actions tool do not list them; no import or export; no reordering by
dragging.

## Phase 64 — The profile's Smart Selection

### Plan

iTerm2's smart selection (`iTermTextExtractor.m
-smartSelectionAt:withRules:…`, `SmartSelectionController.m` and
`SmartSelectionRules.plist`):

1. **Rules** with notes, a regular expression and a precision (Very Low
   0.00001, Low 0.001, Normal 1, High 1000, Very High 1,000,000); iTerm2's
   eleven built-in ones, translated to the `regex` engine.
2. **Quadruple click:** every match of every rule that covers the clicked
   character, in the logical line, scores precision × length and the best one
   wins; with none, word selection.
3. Annotate at the cursor and Smart Select All (the hints) use the profile's
   rules; the hints, those of Normal precision or more.
4. **Profiles › Advanced › Smart Selection:** one row per rule with notes,
   expression and precision, «−», «+» and Load Defaults.

### Result (2026-11-06)

Tested in headless sway with the patched kit, with «copy on select».
`./dev.sh check` clean in both modes; a new test for the choice between
rules.

| Test | Result | Screenshot |
|---|---|---|
| Quadruple click on `https://example.org/a,b`: `https://example.org/a` | ✓ | — |
| On `me@example.com`: the whole address | ✓ | — |
| On `/usr/lib/x.so`: the whole path | ✓ | — |
| On `ok`: the word | ✓ | — |
| Settings › Profiles › Advanced: the eleven rules with their precision | ✓ | `fase64-seleccion-inteligente.png` |

**Not tested:** the kit as is; editing a rule, removing it and Load Defaults;
a match across a wrapped line.

**Differs from iTerm2:** no per-rule actions (context menu and ⌘+click) nor
the sheet's playground; the text looked at is the clicked logical line, not
two lines above and below; `\b` is ASCII and the HTTP URL rule loses its
look-behind branch.

## Phase 65 — Automatic Profile Switching

### Plan

iTerm2's automatic profile switching (`iTermRule.m`,
`iTermAutomaticProfileSwitcher.m` and `PTYSession.m
-tryAutoProfileSwitchWithHostname:…`):

1. **Rules** in the profile with the syntax `[!][user@][host][:path][&job]`
   or `/path[&job]`, with `*` and `?`; `!` makes it sticky.
2. **Scoring** from `iTermRule.m`: exact host 32, host with a wildcard 16 and
   a bit, job 8, user 4, exact path 2, a lone `*` 1; a part that does not
   match rules the rule out. The profile with the best-scoring rule wins.
3. **The stack:** switching saves the previous profile; when no rule matches
   any more, the first one in the stack comes back; a sticky rule empties the
   stack.
4. **Where the session is:** host and user from `OSC 1337;RemoteHost` or,
   without it, this machine's; the folder; the foreground job and its command
   line. It is checked as output arrives, at most twice a second, and again
   when the interval runs out if needed.
5. **Settings › Profiles › Advanced › Automatic Profile Switching:** one rule
   per row with «−» and «+».

### Result (2026-11-06)

Tested in headless sway with the patched kit and three profiles: Default,
Editor (`&less`, badge «EDITOR») and Tmp (`/tmp*`, badge «TMP»).
`./dev.sh check` clean in both modes; new tests for the rules, the scoring
and the return to the previous profile.

| Test | Result | Screenshot |
|---|---|---|
| `less /etc/hostname`: «EDITOR» appears | ✓ | `fase65-editor.png` |
| `q`: back to Default, without a badge | ✓ | — |
| `cd /tmp`: «TMP» appears; `cd /`: it goes away | ✓ | `fase65-tmp.png` |

**Bug found and fixed while testing:** `less` did not switch the profile: it
writes only once when it starts and that check fell inside the half-second
wait. Now a held-back check runs again when the interval runs out.

**Not tested:** the kit as is; `OSC 1337;RemoteHost` and a host rule; a
sticky rule; editing the rules in Settings.

**Differs from iTerm2:** no `{expression}` rules; the stack keeps only the
profile's name, not Edit Session changes; it is not saved on quit; no
«Prevent Automatic Profile Switching» checkbox nor the debug log.

## Phase 66 — Semantic History

### Plan

What Super+click on a file name does in iTerm2
(`iTermSemanticHistoryController.m -openPath:…`,
`iTermSemanticHistoryPrefsController.m` and `iTermPathCleaner.m`):

1. An OSC 8 link or a URL opens as before; otherwise the clicked word is a
   file, relative to the session's folder, and a trailing `:line[:column]` is
   the position.
2. **Actions** in the profile: Open with default app (`xdg-open`), Open URL…,
   Run command…, Always run command… (also when the file does not exist),
   Run coprocess… and Send text….
3. **Substitutions:** `\1` file, `\2` line, `\3` text before the click, `\4`
   text after and `\5` working directory; shell-escaped in commands and
   percent-encoded in the URL.
4. **Settings › Profiles › Advanced › Semantic History:** the action and its
   text.

### Result (2026-11-06)

Tested in headless sway with the patched kit and a fake `xdg-open`.
`./dev.sh check` clean in both modes; new tests for the position and the
substitutions.

| Test | Result | Screenshot |
|---|---|---|
| Run command `echo \1 \2 \5 >> sem.log` and Super+click on `sem.rs:42:3` inside `/tmp/semdir`: `/tmp/semdir/sem.rs 42 /tmp/semdir` | ✓ | — |
| Open with default app: `xdg-open /tmp/semdir/sem.rs` | ✓ | — |

**Not tested:** the kit as is; Open URL, Always run command, Run coprocess and
Send text; the Settings row.

**Differs from iTerm2:** no Open with editor… (its choices are macOS apps);
the default app does not get the line; the file is the clicked word, not
`iTermPathFinder.m`'s path search with spaces; the substitutions are the
numeric ones, not `\(…)` expressions.

## Phase 67 — Profile tags

### Plan

iTerm2's tags (`ProfilesGeneralPreferencesViewController.m`,
`iTermProfilesMenuController.m -addBookmark:…withTags:`):

1. **Profiles › General › Tags:** separated by commas.
2. **Profiles menu:** untagged profiles at the top, with ⌃⌘1…9; the rest in a
   submenu per tag, nested at each `/` and sorted by name.
3. The tag tree is the same as Edit › Snippets', which now shares it
   (`menu::TagFolder`).

### Result (2026-11-06)

Tested in headless sway with the patched kit and the D-Bus menu.
`./dev.sh check` clean in both modes.

| Test | Result | Screenshot |
|---|---|---|
| Default untagged, Prod in `servers/prod` and Dev in `servers` and `work`: only Default at the top; `servers` with `prod › Prod` and Dev; `work` with Dev | ✓ | — |

**Not tested:** the kit as is (BK-010: the menu does not change shape after
the window opens); the Tags field in Settings; tagged snippets after the
change.

**Differs from iTerm2:** the Settings profile list and Open Quickly do not
filter by tag; no tag suggestions while typing.

## Phase 68 — Add Trigger…, triggers in interactive apps and highlights in the cells

### Plan

The rest of iTerm2's Session › Triggers (`PseudoTerminal.m -addTrigger:`,
`iTermAddTriggerViewController`, `PTYSession.m
-triggerEvaluatorShouldUseTriggers:` and `HighlightTrigger.m`):

1. **Add Trigger…:** a sheet with the selection as a literal (escaped)
   expression, the action, its parameter and Instant; OK adds it to the
   session's profile.
2. **Enable Triggers in Interactive Apps:** a submenu checkbox, on by
   default; off, triggers do not look at the alternate screen.
3. **Highlight Text and Highlight Line** change the colours of the cells
   themselves, as iTerm2 does: the highlight goes with the text that had it.

### Result (2026-11-06)

Tested in headless sway with the patched kit and the D-Bus menu.
`./dev.sh check` clean in both modes.

| Test | Result | Screenshot |
|---|---|---|
| Selecting «alla (x2) aqui» and Add Trigger…: the expression `alla \(x2\) aqui` and Highlight Text | ✓ | `fase68-anadir-trigger.png` |
| OK: the trigger goes into the profile and the next line with that text is red | ✓ | — |
| `less` with the checkbox on: the line highlighted; off: not highlighted, and after quitting the normal screen still highlights | ✓ | — |

**Bugs found and fixed while testing:**
- Going back into `less` left an old highlight on another line: highlights
  were an overlay anchored to line numbers. Now they are painted in the
  cells.
- With the checkbox off `less` was still highlighted: whether to look at the
  lines was decided at the start of each chunk, before its `\e[?1049h`. Now it
  is decided at each line feed.
- The sheet asked for text for Highlight Text; now it asks for no parameter
  and the highlight gets its default red background.

**Not tested:** the kit as is; Cancel in the sheet.

**Differs from iTerm2:** the sheet has no name nor the two colours (they are
changed later in Settings) nor the option to add it to the session only; no
Enable All and Disable All, which here is the Enable Triggers checkbox.

## Phase 69 — Copy with Control Sequences, Paste File Base64-Encoded and Jump to Selection

### Plan

1. **Edit › Copy with Control Sequences** (`PTYTextView
   -copyWithControlSequences:`): the selection with the SGR that draws it (RGB
   colours, bold, italic and underline); it also works with the kit as is,
   since it is plain text.
2. **Edit › Paste Special › Paste File Base64-Encoded** (`PTYTextView
   -pasteBase64Encoded:`): the clipboard's first file (a path or a `file://`)
   in base64, in 77-character pieces ended by `\r`, as
   `NSData+iTerm.m -stringWithBase64EncodingWithLineBreak:` does.
3. **Edit › Find › Jump to Selection** (⌘J): the view goes back to the
   selection.

### Result (2026-11-06)

Tested in headless sway with the patched kit and the D-Bus menu.
`./dev.sh check` clean in both modes.

| Test | Result | Screenshot |
|---|---|---|
| «rojo» in red and « normal»: the clipboard holds `ESC[0;38;2;196;48;43;48;2;251;251;252mrojo` and then the normal colour | ✓ | — |
| Copying the path of 300 random bytes, `base64 -d > out.dat` and Paste File Base64-Encoded: `cmp` says they are equal | ✓ | — |
| Selecting «300», going up with ⇧PgUp and ⌘J: the view goes back to «300» | ✓ | `fase69-ir-a-la-seleccion.png` |

**Not tested:** the kit as is.

**Differs from iTerm2:** the SGR always uses RGB colours, not the palette
indices the cell had; Copy with Control Sequences is its own item, not Copy
with Styles' ⌥ alternate; the file is read from a path written to the
clipboard, not from a file copied in the file manager (`text/uri-list`).

## Phase 70 — Stuck key reporting offer

### Plan

`iTermStuckKeyReportingDetector.swift` and `iTermNaggingController.m
-shouldResetKeyReportingMode`:

1. With shell integration, the kitty flags are noted when each command
   starts (`OSC 133;C`) and ends (`OSC 133;D`).
2. If the command ends with flags it did not start with, and the shell does
   not manage them itself between commands, it asks whether to restore the
   mode, with Yes, Always and Never; the last two are remembered.
3. Restoring clears the kitty flags and goes back to standard reporting.
4. The question sheet is the same as the trigger alert's, and its text now
   wraps, as the paste warnings' does.

### Result (2026-11-06)

Tested in headless sway with the patched kit and bash with `PS0`, `PS1` and
`PROMPT_COMMAND` sending `OSC 133`. `./dev.sh check` clean in both modes; a
new test for the detector.

| Test | Result | Screenshot |
|---|---|---|
| `printf '\033[>1u'`: iTerm2's question with Yes, Always and Never | ✓ | `fase70-informe-de-teclas.png` |
| Yes: Terminal State › Disambiguate Escape goes from checked to unchecked | ✓ | — |
| `true` afterwards: no question | ✓ | — |
| A trigger alert still shows with its three buttons and Disable This Alert turns it off | ✓ | — |

**Bug found and fixed while testing:** the question's long text ran out of
the sheet on a single line; now it wraps.

**Not tested:** the kit as is; Always and Never; a shell that manages the
flags (fish).

**Differs from iTerm2:** it is a sheet, not the announcement bar over the
session; it only looks at the kitty flags, not modifyOtherKeys.

## Phase 71 — Selection Respects Soft Boundaries

### Plan

iTerm2's Edit › Selection Respects Soft Boundaries (⌥⌘A)
(`iTermTextExtractor.m -restrictToLogicalWindowIncludingCoord:`,
`-coordContainsColumnDivider:`):

1. A soft boundary is a column with `|`, `│`, `├` or `┤` in at least eight
   rows in a row, like a vim or tmux split.
2. With the checkbox, a triple click selects the line only between the
   boundaries around the click, and smart selection looks only at that part.
3. The checkbox applies to every session and is saved.

### Result (2026-11-06)

Tested in headless sway with the patched kit and twelve lines with `|` in
column 30. `./dev.sh check` clean in both modes.

| Test | Result | Screenshot |
|---|---|---|
| Without the checkbox, triple click: `izquierda 5 … \|derecha 5` | ✓ | — |
| With it, triple click on the left: `izquierda 5` and its spaces; on the right: `derecha 5` | ✓ | `fase71-bordes-suaves.png` |
| Quadruple click on «derecha»: `derecha` with the checkbox and `\|derecha` without it | ✓ | — |

**Not tested:** the kit as is; boundaries with box-drawing `│`.

**Differs from iTerm2:** dragging is not limited to the window
(`alacritty_terminal`'s selection has no column windows), nor is search; the
copied line keeps its spaces up to the boundary.

## Phase 72 — Split pane dividers in the theme's colours

### Plan

`PseudoTerminal.m -terminalWindowDecorationControlColor` and `PTYSplitView.m
-dividerColor`:

1. Light and Light High Contrast: AppKit's light grey (170); Dark and Dark
   High Contrast: dark grey (85); Regular and Compact, one or the other by
   the system appearance.
2. Minimal: the terminal background moved 15 % towards white if it is dark or
   black if it is light (`minimalSplitPaneDividerProminence`).
3. `barrier-kit-next`'s `SplitPanes` takes the divider colour.

### Result (2026-11-06)

Tested in headless sway with the patched kit and a ⌘D split, reading the
divider's pixel. `./dev.sh check` clean in both modes.

| Theme | Background | Divider | Result |
|---|---|---|---|
| Regular, light system | (251, 251, 252) | (170, 170, 170) | ✓ |
| Dark | (23, 25, 30) | (85, 85, 85) | ✓ |
| Minimal | (251, 251, 252) | (213, 213, 214) | ✓ |

**Not tested:** the kit as is; the high-contrast ones.

**Differs from iTerm2:** no `splitPaneColor` or `splitPaneDividerWidth`
(advanced settings); Minimal does not hide the divider when inactive panes
are dimmed.

## Phase 73 — Blurred material in Session Note

### Plan

`SessionNoteView.swift` uses an `NSVisualEffectView` with the `.sheet`
material within the window, like the Composer:

1. Session Note blurs what lies behind it before painting its tint.
2. The `.sheet` material's blur radius is a single token,
   `SHEET_MATERIAL_BLUR`, shared by the Composer and Session Note.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result | Screenshot |
|---|---|---|
| Note over an empty background: grey 243, deviation 0.44 | ✓ | — |
| Note over 40 lines of 130 digits: grey 239, deviation 0.63, no glyph edges | ✓ | `fase73-nota-desenfocada.png` |

**Not tested:** the kit as is; the dark theme.

**Not done:** the tabs' 0.2 s hover animation (`PSMTabBarCell.m
-highlightAmount`). With `use_animation_transition` the value stays at 0
although the hover arrives, so the instant colour change stays.

## Phase 74 — More binding actions

### Plan

`iTermEditKeyActionWindowController.m` groups the pop-up's actions and
`PTYSession.m -performKeyBindingAction:` runs them. The ones with a Linux
counterpart are added:

1. **Miscellaneous:** Undo, which undoes the last close.
2. **Profile:** Change Profile changes the active session's profile.
3. **Resize Pane:** Decrease/Increase Height/Width move the divider, like the
   menu shortcuts.
4. **Split Panes:** Swap With Split Pane Above/Below/Left/Right and Swap With
   Next/Previous Pane (`PTYTab.m -swapSession:withSession:`): the active
   session changes place and keeps the focus. Move Session to Split Pane
   starts the move mode.
5. **Send Keystrokes:** Send Text with “vim” Special Chars expands like
   `NSString+iTerm -stringByExpandingVimSpecialCharacters` (octal, `\x`,
   `\u`, `\b` = DEL, `\e`, `\f`, `\n`, `\r`, `\t`, `\\`, `\"`,
   `\<C-x>` and `\<M-x>`); Send Text without Broadcasting expands the same
   way and writes only to the active session; Open Composer.
6. **Search:** Find Regular Expression runs a temporary case-sensitive regex
   search and closes the find bar.
7. **Pasteboard:** Paste… opens Advanced Paste; Copy Selection or Send ^C and
   Paste or Send ^V copy or paste when they can and otherwise send the control
   character.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result |
|---|---|
| F5 with `echo X\x41\101\e[0m\<C-w>Y\n` sends `echo XAA\E[0m\027Y\n` | ✓ |
| F7 (Copy or Send) without a selection sends `\003` | ✓ |
| F6 (Swap Left) in the right pane moves it to the left and the focus follows | ✓ |

Also, `arrangement-name` was twice in the `.ftl` files and the save
arrangement sheet said “Name”; it now uses `save-arrangement-name`.

**Not tested:** the kit as is; Undo, Change Profile, Resize Pane, Swap
Next/Previous, Move Session, Send Text without Broadcasting with broadcast on,
Compose, Find Regex, Paste… and Paste or Send.

## Decision — Dialog corners (2026-11-06)

CurseTerm's sheets and alerts are born inside the window, like macOS sheets,
so they carry their own rounded border (the kit's `sheet` and `Alert`, radius
`DIALOG.corner_radius`). The Malaysia compositor only rounds dialogs that are
surfaces of their own; if a dialog becomes one, the app stops rounding it.

## Phase 76 — Sequence

### Plan

`PTYSession.m` `KEY_ACTION_SEQUENCE` runs a list of actions, and
`KeyActionSequenceTableViewController.swift` edits it:

1. The Sequence… action (Composition group) keeps its steps in `steps`.
2. Each step runs on the session focused after the previous one: split, then
   send text, types into the new pane.
3. The editor shows the steps in a table with + and −; a new step is Next Tab
   and goes after the selected one. Below, the chosen step's action and
   parameter are edited; a step cannot be another sequence.
4. The bindings table describes the sequence as “A, then B”.
5. OK fills untouched pop-up parameters with the first choice, also in the
   steps.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result |
|---|---|
| F8 = split and send `echo NUEVO\n`: the text appears in the new pane | ✓ |
| Editing a step's text and adding one saves the three steps in the `.domain` | ✓ |

**Not tested:** the kit as is; removing steps; a sequence from Edit › Actions.

## Phase 77 — Key binding presets

### Plan

`iTermPresetKeyMappings.m` loads `PresetKeyMappings.plist` into the profile
and `DefaultGlobalKeyMap.plist` into the global bindings, with ⌘ as Super:

1. Keys › Key Bindings offers Factory Defaults; the profile's Keys, Terminal.app
   Compatibility and Natural Text Editing. Edit › Actions has no presets.
2. If some current binding is not in the preset, it asks “Remove all key
   mappings before loading a preset?” with Keep, Remove and Cancel. Keep
   leaves the others, and the preset wins on the same keys.
3. After a choice, the pop-up says “Presets…” again (BK-042).

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result |
|---|---|
| Factory Defaults with F8 bound: it asks; Keep leaves 13 bindings | ✓ |
| Again with Remove: the preset's 12 remain | ✓ |
| ⌘↑ ×3 scrolls up three lines | ✓ |

**Not tested:** the kit as is; the profile presets in the terminal.

**Differs from iTerm2:** no Simulate Numlock, because a binding does not tell
the keypad apart; Factory Defaults has no ⌃Tab nor ⌃⇧Tab, because CurseTerm
does not order tabs by recent use.

## Decision — Modifier remapping (2026-11-06)

`iTermModifierRemapper.m` remaps Control, Option and Command with a macOS
event tap, ahead of every app and of the menu shortcuts. On Wayland an app
cannot do that: BarrierKit dispatches the menu shortcuts with the modifiers as
they arrive, and remapping only the terminal input would leave the menu
unremapped. Remapping belongs to the compositor (Malaysia) or to xkb
(`altwin:swap_alt_win`, `ctrl:swap_lwin_lctl`…); CurseTerm does not implement
it.

## Phase 78 — Zoom In on Selection and Zoom Out

### Plan

`PseudoTerminal.m -zoomOnSelection:` shows the selection's lines in a
synthetic session that only displays; `-zoomOut:` (Esc) goes back:

1. View › Zoom In on Selection (⌥⌘Z), enabled with a selection and outside
   Instant Replay, copies the selected lines and clears the selection.
2. Meanwhile the pane paints only those lines and sends no keys to the shell.
3. View › Zoom Out (Esc), enabled while zoomed or filtering: it clears the
   filter first and, if there is none, ends the zoom.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result | Screenshot |
|---|---|---|
| Select 6–9 of `seq 30` and ⌥⌘Z: only 6–9 remain | ✓ | `fase78-zoom-seleccion.png` |
| Esc: the full session comes back | ✓ | `fase78-zoom-seleccion.png` |

**Not tested:** the kit as is; Zoom Out with an active filter.

**Differs from iTerm2:** the zoomed view does not scroll and cannot be
selected or searched; if the selection does not fit, its first lines show.

## Phase 79 — Settings › Advanced

### Plan

`iTermAdvancedSettingsViewController.m` shows the settings of
`iTermAdvancedSettingsModel.m` in a searchable table. Only those CurseTerm
already has behaviour for are in, with iTerm2's defaults:

| Section | Setting | Before |
|---|---|---|
| General | `idleTimeSeconds` (2 s, plus the second of `-isIdle`) | fixed 3 s |
| General | `alertTriggerRateLimit` (1 s) | fixed 1 s |
| General | `userNotificationTriggerRateLimit` (0 s) | 1 s, unlike iTerm2 |
| Tabs | `moveLeftAfterClosingTab` (off) | always to the right |
| Terminal | `findUrlsRegex` | constant |
| Terminal | `timeBetweenBlinks` (0.5 s) | token |
| Pasteboard | `slowPasteBytesPerCall` (16) | fixed 16 |
| Pasteboard | `slowPasteDelayBetweenCalls` (0.125 s) | 30 ms, unlike iTerm2 |
| Pasteboard | `pasteHistoryMaxOptions` (20) | fixed 20 |

1. The values live in `Defaults.advanced` (`[advanced]` in the `.domain`).
2. The search needs every word in the section or the description; “Show only
   non-default values” keeps the changed ones. Order: section, then
   description.
3. A number is saved once it reads as a number.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result | Screenshot |
|---|---|---|
| The pane shows the four sections in order | ✓ | — |
| Idle at 5 and Move Left on are saved in `[advanced]` | ✓ | — |
| “Show only non-default values” keeps those two | ✓ | `fase79-avanzado.png` |
| With Move Left, closing tab B of A·B·C selects A | ✓ | — |

**Not tested:** the kit as is; the other settings at run time.

**Differs from iTerm2:** only the settings with behaviour in CurseTerm are
there; the icon repeats General's (BK-024).

## Phase 80 — Lock Layout

### Plan

`PseudoTerminal.m -toggleLayoutLocked:` freezes the window's layout (iTerm2
3.7.0beta4 notes):

1. Window › Lock Layout, checkable, per window.
2. Locked, nothing splits, no tab is added, closed or reordered, and no pane
   is closed, moved, buried or swapped. Those menu items are disabled and the
   action is ignored from bindings, the tab bar and “+” too. One list,
   `menu::LAYOUT_CHANGES`.
3. Resizing panes, opening windows and closing the window still work.
4. A saved arrangement remembers the lock (`layout_locked`,
   `TERMINAL_ARRANGEMENT_LAYOUT_LOCKED`) and the restored window starts
   locked.

### Result (2026-11-06)

Tested in headless sway with the patched kit. `./dev.sh check` clean in both
modes.

| Test | Result |
|---|---|
| Locked: New Tab disabled; ⌘T, ⌘D and ⌘W leave the window unchanged | ✓ |
| Unlocked: ⌘T opens the tab | ✓ |
| Saving “Bloq” while locked writes `layout_locked = true` | ✓ |
| `--arrangement Bloq` opens the window with Lock Layout checked | ✓ |

**Not tested:** the kit as is; the swap bindings with the window locked.

## Decision — Pause Pane (2026-11-06)

`PseudoTerminal.m -toggleTmuxPausePane:` is only enabled in a tmux client
session with the gateway's pause mode (`tmuxController.gateway
.pauseModeEnabled`). Without tmux integration there is nothing to pause; it
comes with it.
