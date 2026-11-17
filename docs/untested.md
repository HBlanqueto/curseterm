# Untested

**Goal:** list what CurseTerm has never been run against, so that nothing
counts as tested by default (AGENTS.md rule 6).
**Reader:** whoever changes, reviews or releases CurseTerm.
**Input:** the notes of the former phase log, kept verbatim (last copy:
`git show bc27a76:docs/progreso.md`, which also holds each phase's plan and
results).
**Output:** per phase, what was not run and the failures still open.

Entries that talk about the "kit as is" or the "patched kit" predate
[ADR 0006](decisiones/0006-el-kit-se-completa-en-barrierkit.md), which removed
those two modes. Read them as "not tested", not as "broken".

## Steps

1. Run one of the items below, in Malaysia or in the headless lab.
2. Delete its entry in the same commit that fixes or confirms it.
3. Add the untested part of a new change under its own heading.


## Phase 1 — Core

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

## Phase 2 — Organization

**Not tested:**

- **Minimize and Zoom:** sway does not minimize and does not maximize in
  tiling; `WindowControls::minimize` and `toggle_maximize` are called, with
  no visible effect here.
- **Malaysia:** it still cannot be built (phase 1). In Malaysia, Super+M and
  Super+W are handled by the compositor.

## Phase 3 — Profiles and preferences

**Not tested:** renaming a profile with open sessions (sessions with the old
name fall back to the default profile); the "Directory path" row with the
keyboard; New Window with Current Profile (it launches
`curseterm --profile NOMBRE`; `--profile` was tested only through that menu
command, without a screenshot).

## Phase 4 — What makes iTerm2 iTerm2

**Not tested:** images with `preserveAspectRatio=0` and in percent;
`1337;SetMark` and `1337;CurrentDir` (only their tests); Use Selection for
Find; Edit Triggers… (opens Settings like Settings…); a dbusmenu `Event`
sent without waiting for a reply (`dbus-send` without `--print-reply`) did
nothing twice, with a reply it did; the cause was not investigated.

## Phase 5 — Terminal profile, bell and complete Find

**Not tested:** "Flash visual bell" off (the branch is one condition, no
screenshot); a scrollback of 0 lines; the 100,000 limit with a larger value in
the field.

## Phase 6 — Window, Session and Keys profile

**Not tested:** "Prompt before closing" on Close All Panes, Close Window and a
tab's "×" (they share `request_close`, no screenshot); "Always" and "Never";
changing Columns and Rows from the field.

## Phase 7 — Last command output, Copy Mode and Find Cursor

**Not tested:** the "Copy Mode" item checked in the global menu and "Select
Output of Last Command" disabled without integration (the test used the kit
as is, with no menu on D-Bus, BK-018); Ctrl+B and Ctrl+F in copy mode; the
last command's output after a resize that reflows the lines.

## Phase 8 — Undo Close and paste history

**Not tested:** the arrows inside the history; the enabled state of Undo
Close and Open Paste History in the global menu (BK-018); undoing a tab
closed from the "Close?" alert.

## Phase 9 — Editable titles and Help menu

**Not tested:** with the kit as is, the field without focus until clicked
(BK-029); the "Cancel" and "Rename" buttons with the mouse.

## Phase 10 — Timestamps and Reset Character Set

**Not tested:** the times after a resize that reflows lines; a session that
reaches the scrollback limit.

## Phase 11 — Links and saving the selection

**Not tested:** a failed save (it only leaves a warning in the log); the
save sheet with patch 0006's automatic focus now that the field starts
empty.

## Phase 12 — Session log and Paste Special

**Not tested:** a write failure in the log (it stops logging and leaves a
warning); Paste Slowly with several broadcast sessions.

## Phase 13 — Profile badge and paths with Super+click

**Not tested:** the badge in narrow split panes, where the text can spill
out on the left.

## Phase 14 — Transparency

**Not tested:** with the kit as is, the disabled menu item (there is no D-Bus
menu without patch 0003); panes with profiles of different transparency in
the same tab; blur, which sway does not do.

## Phase 15 — Open Quickly and Open Profiles

**Not tested:** Open Profiles in the global menu with patch 0004, which
rebuilds the Profiles menu; Open Quickly with split panes.

## Phase 16 — Terminal State, Edit Session and help

**Not tested:** Focus Reporting and Mouse Reporting with a program that uses
them; CurseTerm does not yet send focus or mouse events to the program, it
only shows and changes the mode.

## Phase 17 — Mouse and focus for programs

**Not tested:** mode 1003 (motion without a button) with a real program such
as `htop` or `vim`; the middle and right buttons; Ctrl and Alt inside the
report outside the tests.

## Phase 18 — Coprocesses

**Not tested:** closing the window with a running coprocess (the process
exits without going through `Drop`; the coprocess gets end of file on its
input, but keeps running if it does not read it).

## Phase 19 — Scroll indicator and smart selection

**Not tested:** dragging the bar (it cannot be done: it is only an
indicator); four clicks on a path outside the `word_span` test.

## Phase 20 — Several windows in one process

**Not tested:** closing a window with pending timers (bell, undo close) in
flight; the hotkey window, which is still a separate process.

## Phase 21 — Input method

**Not tested:** that the compositor gets `set_cursor_rectangle` (the trace
did not show it); a real input method such as fcitx5 or IBus; composition
in the sheets' text fields.

## Phase 22 — Primary selection

**Not tested:** the middle button when the program asks for the mouse (the
click is sent to it and nothing is pasted, as in iTerm2); the primary
selection in the sheets' text fields.

## Phase 23 — Copy with Styles

**Not tested:** pasting into an app that reads `text/html` (there is none in
the lab); block selections.

## Phase 24 — Instant Replay

**Not tested:** the slider with the mouse; a session with more than 120
snapshots.

## Phase 25 — Toolbelt

**Not tested:** processes of another tab (Jobs follows the active session);
paths with spaces in Recent Directories.

## Phase 26 — Status bar

**Not tested:** the option from Settings (only from the `.domain`); a real
repository with `git worktree` outside the `git_branch` test.

## Phase 27 — Profile cursor

**Not tested:** picking another type in the pop-up (only from the `.domain`).

## Phase 28 — Send text at start

**Not tested:** editing the field from Settings (only from the `.domain`); a
new pane and the restart when the session ends.

## Phase 29 — General and Pointer settings

**Not tested:** the Pointer switch (only from the `.domain`).

## Phase 30 — Cursor guide

**Not tested:** the menu checkbox over D-Bus and the Settings switch (only
from the `.domain`).

## Phase 31 — Recent directories

**Not tested:** the global menu item over D-Bus.

## Phase 32 — Clear to a mark or the selection

**Not tested:** the disabled items in the global D-Bus menu.

## Phase 33 — Save contents

**Not tested:** with the kit as is the item is only reachable from a global
menu (no shortcut).

## Phase 34 — Window arrangements

**Not tested:** an arrangement with a deleted profile (it opens with the
default profile).

## Phase 35 — Annotations

**Not tested:** several annotations on the same line or Next wrapping to the
start; a note in an unfocused pane.

## Phase 36 — Composer

**Not tested:** a light scheme (light material), the closing fade in a
screenshot, pasting with ⌘V inside, the IME, and scrolling following the
cursor with a lot of text.

## Phase 37 — Named marks

**Not tested:** with the kit as is (no D-Bus menu, and the field only takes
the focus when clicked, BK-029); several marks with the same name.

## Phase 38 — Find URLs

**Not tested:** that «1» opens the browser (the lab has none; the command
runs without error), two-letter shortcuts in the app (they are in the test)
and the patched kit.

## Phase 39 — Smart selections and picking a result

**Not tested:** with the kit as is; actually opening a path or a URL.

## Phase 40 — Mark alerts

**Not tested:** the kit as is; the activation token (mako 1.11 sends no
`ActivationToken`, so the window is not brought forward); real sound (the lab
has no audio: `canberra-gtk-play` says «Not available»); Malaysia, which has
no notification server yet.

## Phase 41 — Profile notifications

**Not tested:** the kit as is; an inactive window as the reason to alert
(headless sway does not move focus between windows in the test); Malaysia,
which has no notification server yet.

## Phase 42 — Session indicators

**Not tested:** the kit as is; a light background scheme.

## Phase 43 — Window themes

**Not tested:** hover (close button and hover color) and dragging or
double-clicking the strip, because weston has no virtual keyboard or pointer;
active-window colors on weston (it does not focus the window, so the inactive
ones show); the kit as is; Malaysia.

## Phase 44 — Duplicating, even panes and selecting the command

**Not tested:** the kit as is; the last finished command, which needs a
shell that emits `133;C`; a multi-line command.

## Phase 45 — Filter

**Not tested:** the kit as is; a 100,000-line history (the filter walks all
of it on every repaint).

## Phase 46 — Warning before a multi-line paste

**Not tested:** the kit as is; Cancel; the checkboxes from the menu; the
one-line-with-newline cases (they are in the test).

## Phase 47 — Advanced Paste

**Not tested:** the kit as is; Base64, the regular expression and shell
escaping from the panel (they are in the tests).

## Phase 48 — Duplicate Window

**Not tested:** the kit as is (another process).

## Phase 49 — Full paste warnings

**Not tested:** the kit as is; Advanced… from the warnings; OK and Cancel in
the tabs sheet.

## Phase 50 — Find modes and Clear Find

**Not tested:** the kit as is; Clear Find enabled or disabled in the D-Bus
menu.

## Phase 51 — Find Globally

**Not tested:** the kit as is; a snippet longer than 256 characters.

## Phase 52 — Select Matches

**Not tested:** the kit as is; Copy with Styles with several selections.

## Phase 53 — Snippets

**Not tested:** the kit as is; that an open window's menu changes when the
snippets are edited in Settings.

## Phase 54 — Review: bugs in phases 50 to 53

**Not tested:** the kit as is.

## Phase 55 — Session Note

**Not tested:** the kit as is; the dark theme.

## Phase 56 — Key reporting modes

**Not tested:** the kit as is; the profile switches in the Settings window
(the value is, in the `.domain`); Report Alternate Keys and Report
Associated Text with real keys (only in the tests).

## Phase 57 — Key bindings

**Not tested:** the kit as is; the full screen, scroll, pane and Run
Coprocess actions from a binding.

## Phase 58 — Moving and burying sessions

**Not tested:** the kit as is; cancelling with Escape or a click on the
source; a buried session that ends.

## Phase 59 — Full screen, Show Tabs in Fullscreen and Window Style

**Not tested:** the kit as is; the title bar revealed by focus mode and
`fullscreen_push`: Malaysia does not start in the lab (llvmpipe lacks
`EGL_ANDROID_native_fence_sync`), and neither sway nor weston has the
`infinity_focus_v1` protocol; ⌃⌘F with weston's desktop-shell: without a
seat, weston does not raise the window to the full-screen layer
(`desktop_surface_committed` activates it once per seat), so it was tested
with kiosk-shell.

## Phase 60 — Command history and autocomplete

**Not tested:** the kit as is; the ⌘; and ⇧⌘; shortcuts (the lab's virtual
keyboard has no semicolon).

## Phase 61 — Triggers like iTerm2's

**Not tested:** the kit as is; Prompt Detected, Report Directory, Set Mark and
Run Coprocess; Highlight Line on a wrapped line.

## Phase 62 — Settings › Arrangements and the default arrangement

**Not tested:** the kit as is; deleting the default arrangement.

## Phase 63 — Shortcuts › Actions and Edit › Actions

**Not tested:** the kit as is; Edit and «−» on an action; a menu or profile
action.

## Phase 64 — The profile's Smart Selection

**Not tested:** the kit as is; editing a rule, removing it and Load Defaults;
a match across a wrapped line.

## Phase 65 — Automatic Profile Switching

**Not tested:** the kit as is; `OSC 1337;RemoteHost` and a host rule; a
sticky rule; editing the rules in Settings.

## Phase 66 — Semantic History

**Not tested:** the kit as is; Open URL, Always run command, Run coprocess and
Send text; the Settings row.

## Phase 67 — Profile tags

**Not tested:** the kit as is (BK-010: the menu does not change shape after
the window opens); the Tags field in Settings; tagged snippets after the
change.

## Phase 68 — Add Trigger…, triggers in interactive apps and highlights in the cells

**Not tested:** the kit as is; Cancel in the sheet.

## Phase 69 — Copy with Control Sequences, Paste File Base64-Encoded and Jump to Selection

**Not tested:** the kit as is.

## Phase 70 — Stuck key reporting offer

**Not tested:** the kit as is; Always and Never; a shell that manages the
flags (fish).

## Phase 71 — Selection Respects Soft Boundaries

**Not tested:** the kit as is; boundaries with box-drawing `│`.

## Phase 72 — Split pane dividers in the theme's colours

**Not tested:** the kit as is; the high-contrast ones.

## Phase 73 — Blurred material in Session Note

**Not tested:** the kit as is; the dark theme.

## Phase 74 — More binding actions

**Not tested:** the kit as is; Undo, Change Profile, Resize Pane, Swap
Next/Previous, Move Session, Send Text without Broadcasting with broadcast on,
Compose, Find Regex, Paste… and Paste or Send.

## Phase 76 — Sequence

**Not tested:** the kit as is; removing steps; a sequence from Edit › Actions.

## Phase 77 — Key binding presets

**Not tested:** the kit as is; the profile presets in the terminal.

## Phase 78 — Zoom In on Selection and Zoom Out

**Not tested:** the kit as is; Zoom Out with an active filter.

## Phase 79 — Settings › Advanced

**Not tested:** the kit as is; the other settings at run time.

## Phase 80 — Lock Layout

**Not tested:** the kit as is; the swap bindings with the window locked.

## Phase 81 — Tab groups

**Not tested:** the kit as is; Select Next/Previous Tab Group in the window;
⌃A in the name field.

## Phase 82 — Floating panes

**Not tested:** the kit as is; resizing with the band; Bring to Front and
Send to Back with two floats; Lock Layout with floats.

## Phase 83 — Fold and Replace Selection

**Not tested:** the kit as is; folding with a full history.

## Phase 84 — Session archives

**Not tested:** the kit as is; a custom folder; Restore Archive… with a typed
path.

## Phase 85 — Password manager

**Not tested:** the kit as is; Edit; Reveal Password; Broadcast; the trigger;
a locked keyring.

## Phase 86 — Moving sessions between windows

**Not tested:** the Close Other Tabs question; Select ▸, Rename Group… and
Save Tab as Window Arrangement; with the kit as is, the tab menu without Move
to New Window.

## Phase 87 — Animated tab hover and automatic high contrast

**Not tested:** “reduce motion”; high contrast in dark mode; a `contrast`
change with the window open (BK-046).

## Phase 88 — Maximize throughput

**Not tested:** on a real 60 Hz or faster display, where the default slow rate
saves redraws; here the headless compositor already limits them to about 34
per second.

## Phase 89 — The kit is completed in barrierkit

**Not tested:** choosing an `.itermcolors` file in the portal dialog,
«Presets…» with `MenuButton`, the high contrast read from `Resolved` and the
hover fade with `use_alive`.

## Phase 90 — Traffic lights away from the edge

**Not tested:** it was not run on Malaysia.

## Run Command and Coprocess triggers: captures as variables

**Not tested:** a trigger fired in the running app (only `shell_script` is
tested, against `/bin/sh`); a user whose `$SHELL` is fish or nushell, because
Run Command now runs under `/bin/sh`, as Run Coprocess already did.

## Kit 70968a9 (`bk`, `ctdesign`, `ctappearance`) and Freya 0.5.0

**Not tested:** typing, pointer and menu input on this `rev`. It builds
(`nix build .#curseterm`), passes clippy and the unit tests, and the window
opens and stays up for 30 s in a headless Weston on lavapipe; nothing was sent
to it. The Freya 0.5.0 changes that compile but behave differently: copy and
paste through `Clipboard::get` and `Clipboard::set` (the root context of 0.4.3
panicked at start-up), the declarative `.cursor` (text cursor over a pane,
arrow over the note's delete button), `on_global_pointer_up` ending a pane
divider drag and a note drag, the stepped `Slider` of Window Transparency and
of the instant replay bar (value and callback are `f32` steps), and
`Symbol::Bell` in the tab bar's bell badge. The key per mode of the Password
Manager body was a workaround for a freya-core 0.4.3 runner bug and was not
re-checked on 0.5.0.

## History limit kept by CurseTerm

**Not tested:** marks, annotations, timestamps, inline images and folds in the
running app with a full history (only `io::tests`, on a headless `Term`);
output speed with a full history (the limit is enforced after every line
instead of by `alacritty_terminal`); lowering «Scrollback lines» in Settings
with a session open.

## OSC 52 behind a profile switch

**Not tested:** a program copying with OSC 52 (`printf '\e]52;c;aGk=\a'`) with
the switch off and on; the switch in Settings › Profiles › Keys.

## Title bars and tab bar measured on macOS 26

**Not tested:** not compiled (`skia-bindings` 0.104.0 could not be downloaded
or built in the session that made the change); the 32-point Regular title
bar, the left-aligned title at 84 points, the traffic lights' inset in a
Minimal window and the ⌘N label of each tab, in Malaysia.

## Show mark indicators profile switch

**Not tested:** not compiled (the dev shell could not build skia in the session
that made the change); only `cargo fmt --check` ran. Clippy, the tests and the
switch in Settings › Profiles › Terminal hiding and showing the prompt
triangles at once in an open session, with `OSC 133;A` from the shell.
