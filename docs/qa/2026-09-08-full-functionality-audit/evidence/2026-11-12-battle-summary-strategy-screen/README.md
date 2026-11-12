# The Battle Summary over the Command Center (2026-11-12)

This adds to TAC-01-C013 (return to strategy) and CMD-04-C018 (the Battle
Summary is modal). The ledger JSON is not changed.

User decision (2026-11-12, approved architecture change): the Battle
Summary is shown as in the original, a strategy window over the live
Command Center, not inside the tactical view over the bare shell bitmap.

## A0 used

| Capture | SHA-256 | What it shows |
| --- | --- | --- |
| `interface-parity/a0/TAC-01-C013/original` (main checkout `.artifacts/`) | `cd4eda7b3b4f45406eef7a8f9f2bbf95606fa39d8a04a85e23edd7a83bc77b21` | The Battle Summary after the last Alliance ship died at Mon Calamari, over the Command Center: status bar, message rail, consoles, droids and the open Sluis sector window around it |

No new capture was taken.

## Trace

Read from the typed corpus (`~/Games/rebexe-decompile/decompiled-typed`).

**Who creates the summary, and where.** `FUN_00488b90` is the player
object's battle hook. When its battle source reports state 1 (the result
is in) and `FUN_0041d6d0` finds the Command Center ready (`FUN_0042d490`:
its window visible, `+0xdc` set, `DAT_006b1470` clear), it calls
`FUN_0041d730`. That takes the Command Center (`FUN_00422ca0` returns
`DAT_006b14cc`, set by its constructor `FUN_00421c70`) and calls
`FUN_0042a640` on it (read-in-full):

1. `PostMessageA(+0x18, 0x409)`: the Command Center's `0x409` case
   (`FUN_00422ce0`) runs `FUN_00442430`, as every cockpit window opener
   does (`FUN_0042a000`, `FUN_0042a0c0`, `FUN_00429ce0`, ...);
2. `FUN_00600f90(7)` destroys an open child with id 7;
3. `FUN_0046f140(parent +0x1c, owner = the Command Center, id 8,
   report)` builds the window (vtable `0x0065a300`): `FUN_00606380` with
   size `0x1d6` by `0x14b` (470 by 331);
4. `FUN_005ffce0(0)` creates its Win32 window, slot `+0x2c(5)` shows it,
   `FUN_005f4f10` adds it to the view's window list, `SetFocus` gives it
   the keyboard.

This is how the System Finder opens (`FUN_0042a000`: `FUN_00460090`,
`FUN_005ffce0(0)`, `FUN_005f4f10`, `+0x2c(5)`). So the summary is a child
window of the Command Center, opened after the tactical module has
returned and the Command Center shows again. Its place on the screen is
A0's: (62, 49) over the Alliance's Command Center (TAC-01-C013), (125, 52)
over the Empire's (TAC-07-C002, the battle windows' corner).

**Modality (CMD-04-C018).** Manual p. 64, "Modal vs. Modaless Windows":
"Modal windows include Status windows, Finder windows, the Battle Summary
window, the Encyclopedia window, and Message windows." In the trace,
`FUN_0046f140` passes 1 as `FUN_00606380`'s thirteenth argument, which
`FUN_00606380` stores at `+0xb8`; the System Finder's constructor
`FUN_00460090` sets the same bit (`+0xb8 |= 1`, the Finders' modal flag in
`ghidra/notes/system-finder.md`). A0 (`a0-vm/modal`, `a0-vm/modal2`)
showed the Finders modal: no click on the cockpit controls, the GID
control, the rail, the droids or the map until they close. The summary
gets the same gate. Read: spot-checked (the flag's reader in the Command
Center is not traced; the reading rests on the manual and the shared
flag). A0 has no capture of a click under the summary.

**The clock while it is open.** `FUN_0040a050`'s state 3 sets the game
object's `+0x14` (`DAT_006b1218 + 0x14 = 1`). The day step's phase 0
(`FUN_0041e390`) calls `FUN_0051dea0` (the day advance) only while `+0x14`
is 0. State `0xc` clears it (`FUN_0041e320`). Between them, state `0xb`
waits until both players are done (`FUN_00436400`: each player's `+0x38`
is 3). The summary's Close (`0xc9`), fleet (`0xca`) and system (`0xcb`)
buttons all end in `FUN_0041ce80` -> `FUN_004360b0` -> `FUN_00487890`,
which sets `+0x38` = 3 (`FUN_00470ea0`, read-in-full). So no day passes
while the summary is open. Read: spot-checked; that `FUN_0041e320`'s and
`FUN_0041e390`'s `this` are both `DAT_006b1218` is inferred.

## The change

`crates/rebellion-render/src/tactical_view.rs`:
- `TacticalState::battle_summary_open` (the session in its results, no
  Battle Alert), `battle_summary_screen_rect` and
  `battle_summary_contains_screen_point`;
- `battle_summary_window_rect` (A0's corner, `FUN_0046f140`'s 470 by 331);
- `draw_battle_summary_window`: the same window code as before
  (`draw_original_battle_results`), without the backdrop;
- `battle_summary_input_hold`: an egui area over the window, so strategy
  windows under it take no pointer.

`crates/rebellion-app/src/main.rs`:
- each frame, a battle whose summary is open runs in the Command Center's
  mode (`battle_summary_mode`): the galaxy, the cockpit, the droids and
  every open strategy window draw as usual, and the summary is drawn over
  them after the egui pass;
- the summary's actions (Close, system, fleet) go through one route after
  the frame, from either mode (the code moved out of the tactical arm
  unchanged);
- the clock holds while it is open (`galaxy_clock_runs`);
- the summary joins the Finders' modal gate (`window_modal_open`), and the
  map takes no pointer over it;
- its score (MDATA.308/309/314/315) plays over the Command Center; the
  galaxy theme is not restarted under it;
- Escape closes it (see "Keys" below); the galaxy's own Escape chain
  does nothing while it is open (before, it would have left for the main
  menu);
- a tactical fixture that opens on the summary reports ready over the
  Command Center.

The frame in which the battle's last step runs draws the battle, not the
summary (see "The battle's last frame" below); the next frame opens the
summary on the Command Center. The Battle Alert is unchanged (still over
the screen copy or the shell).

No save-format change.

## Tests

- `the_battle_summary_becomes_a_470_by_331_strategy_window_when_the_battle_ends`
  (render): the rect, the open state, edges at 640 x 480, the screen rect
  at 1280 x 960 and 800 x 480,
  the Empire's corner, and none while the Battle Alert is open. Without
  the change it does not compile (no such API).
- `the_battle_summary_opens_over_the_command_center_not_the_tactical_view`
  and `the_galaxy_clock_holds_while_the_battle_summary_is_open` (app). With
  the helpers returning the old behaviour (mode unchanged, clock by mode
  only), both fail (shown).
- `cargo test -p rebellion-app -p rebellion-render`: app 160, render 1,493,
  0 failures. `cargo check --workspace`: pass. `provenance-scan check`: 0
  uncited items.
- `cargo mutants --in-place` on the new functions: `battle_summary_mode`
  and `galaxy_clock_runs` (main.rs), 7 of 7 caught;
  `battle_summary_open`, `battle_summary_screen_rect`,
  `battle_summary_contains_screen_point`, `battle_summary_window_rect` and
  `draw_battle_summary_window` (tactical_view.rs), all caught. Missed:
  `battle_summary_input_hold` replaced by `()`. It needs an egui frame on
  a macroquad window, which unit tests do not have; no browser probe
  presses a strategy window under the summary yet. The tool also listed
  missed field deletions in functions outside this change
  (`window_conf`, `draw_fullscreen_texture`, `draw_original_bitmap`,
  `paint_tactical_tip`, text-parameter helpers), as before.

## Browser case

`production-battle-last-ship-lost` (`run.mjs --battle`) now also:

1. requires the log `[tactical-results] summary_window owner=command_center`;
2. for the Alliance at 640 x 480, compares the screen around the window
   with A0 TAC-01-C013: the status bar less its three numbers
   (x 230..530, y 8..26), the left column to x 52 less the message rail
   (x 0..32, y 104..310), the consoles and droids (y 396..480, x < 630)
   and the right column (x 540..630, y < 396). A0's pointer at x 630..,
   y 392..411 is left out;
3. presses the System Finder control (Alliance (119.5, 416), Empire
   (161.5, 446)) and requires no `command=0x12d` and an unchanged frame.

| Check (Alliance, 640 x 480) | Before (bare shell, run `2026-11-12T02-02-27-259Z-3391357`) | Now |
| --- | --- | --- |
| Summary scene (66,314 px) | 0 differ | 0 differ |
| Strategy screen around it (118,168 px) | 13,717 differ | 0 differ |
| Every pixel outside the window (151,630 px, reported) | 36,012 differ | 8,777 differ |

The second probe fails on the pre-modal build: run
`2026-11-12T11-54-07-110Z-853212` (both sides, "the cockpit took a click
while the Battle Summary was open").

## Gate runs (Linux, Arch Chromium, 640 x 480)

Run ids are UTC; artifacts in the worktree's `.artifacts/interface-parity/`.

- `2026-11-12T12-15-47-920Z-1240693`: `--scenario=production-battle-last-ship-lost`, 2 of 2.
- `2026-11-12T12-15-35-194Z-1239996`: `--scenario=production-battle-results-presentation`, 2 of 2.
- `2026-11-12T12-17-47-057Z-1257551`: `--scenario=production-trench-run-success`, 2 of 2.
- `2026-11-12T12-17-54-142Z-1258015`: `--scenario=production-battle-alert-entry`, 2 of 2.
- `2026-11-12T12-18-09-746Z-1258574`: `run.mjs --battle` (all 43 scenarios
  at 640 x 480), 84 of 86 executions pass. The two failures
  (`production-participants-3d-off` Alliance, `production-battle-last-ship-lost`
  Empire) are "paused tactical render stalled during the 60-frame timing
  sample", before any battle ends, with the machine's load average at 24
  (other sessions). Rerun alone, both pass on both sides:
  `2026-11-12T12-29-49-101Z-1324114` and `2026-11-12T12-30-05-377Z-1326849`.
- After the last refactor (`battle_summary_screen_rect`, no change in
  behaviour), rebuilt from the committed tree:
  `2026-11-12T13-01-48-209Z-1629612` (`production-battle-last-ship-lost`,
  2 of 2) and `2026-11-12T13-03-45-166Z-1634315`
  (`production-battle-results-presentation`, 2 of 2).

An earlier full run, `2026-11-12T11-58-52-004Z-946007`, failed 8 of 88:
the results-presentation fixture never reported ready (it opens on the
summary, and the ready report was tied to the tactical mode; fixed), the
trench-run scenarios had no cutscenes staged in the worktree (404 on
`201.webm`; environment), the left-column region took in the galaxy
display's edge at x 56..61 (narrowed to x 52), and one alert-entry timing
sample stalled while cargo ran beside the gate.

## What still differs

- **Outside the window, 8,777 pixels against A0.** Game state: the status
  bar's numbers (A0 0 / 58 / 256, the fixture 0 / 0 / 739), the message
  rail's lit icons, the galaxy display (A0 shows Popular Support with the
  Sluis sector window open; the fixture opened none), and A0's pointer.
- **Z-order.** The summary is drawn over every strategy window. In the
  original a later window could be raised over it; the port's z-order
  across window kinds is unchanged (not approved).
- **Alt accelerators.** Accelerator table 11 (Alt+1..9 GID displays,
  Alt+P/G/U/A/... ) goes through `TranslateAcceleratorA` in the message
  loop (`FUN_00413560`) as `WM_COMMAND`, not through the `WM_KEYDOWN`
  check below; whether the Command Center refuses those commands while a
  modal window is up is not traced. The port still answers them.
- **The Empire's side** has no A0 capture of the summary; only the modal
  probe and the report text run for it.

## Follow-up (2026-11-12, second pass): keys and the battle's last frame

### Keys

Read-in-full: the summary's key slot and the Command Center's `WM_KEYDOWN`.

- The summary's slot `+0x4c` (vtable `0x65a300`, `FUN_004710f0`):
  - `0x1b` (Escape): slot `+0x30` (`Class_0065a300_vt30`: stops the
    score, restores the galaxy's, closes), then `FUN_0041ce80` and
    `FUN_00401240`: exactly the Close button `0xc9` of `FUN_00470ea0`.
  - `0x25` / `0x27` (left / right): on the category bar `+0x160`
    (Capital Ships `0x65` .. Personnel `0x68`), press the nearest visible
    button before / after the selected one (`FUN_0060d7e0(id, 1)`). When
    the selected one has no neighbour that way, press the first. The bar
    is shown on the forces pages only, so elsewhere an arrow changes
    nothing unless the selection is at an end.
  - every other key goes to the parent's slot `+0x14` as `WM_KEYDOWN`
    (`0x100`): the Command Center's `FUN_00422ce0`.
- `FUN_00422ce0`'s `WM_KEYDOWN`: for `0x70..0x76` (F1..F7) it walks the
  view's child list (`FUN_005f5060`, `+0x10` next) and returns at once
  when a child has `+0xb8 & 1`, the modal bit the summary and the Finders
  carry. Otherwise F1 posts `0x133` (Game Options), F2 `FUN_0042a000`, F3
  `FUN_0042a0c0`, F4 `FUN_0042a4d0`, F5 `FUN_0042a180`, F6
  `FUN_0042a240(1, 0x79)`, F7 `FUN_00429f30`. So no F-key opens anything
  while the summary is open. The message loop (`FUN_00413560`) translates
  accelerators against the main window before dispatch.

The port: F1..F5 and F7 reach the cockpit only through
`handle_cockpit_egui_input`, and F6 through the rail's `speed_input`; both
are skipped while `modal_window_open`, which since the first pass holds
the summary. So the port already followed the original here; the first
pass's open point ("F-keys still open the Finders") was wrong. New:
Escape and the arrows in `draw_original_battle_results`
(`battle_summary_key`, `battle_category_after_arrow`).

### The battle's last frame

`FUN_0040a050` state `0x16` applies the module's result and closes it
(`FUN_005975a0`); only after that, with the Command Center visible again,
does `FUN_00488b90` -> `FUN_0042a640` create the summary. The original
never shows the summary in the tactical view. The port's frame whose step
ended the battle drew the summary over the tactical backdrop once; it now
draws the battle's last state and takes no input from it
(`battle_frame_draws_summary`); the summary opens over the Command Center
next frame. A summary drawn over the tactical view now logs
`[tactical-results] summary_window over=tactical_view`.

### Tests

- `the_step_that_ends_the_battle_draws_the_battle_not_the_summary`: fails
  when the helper returns the old behaviour (always draw; shown).
- `escape_closes_the_battle_summary_and_the_arrows_step_its_category_bar`
  (new API, does not compile without the change).
- Browser, `production-battle-last-ship-lost`: no `over=tactical_view`
  log; F2, F6 and F7 while the summary is open open nothing
  (`command=0x12d`, `0x131`, `destination=message_index`) and leave the
  frame unchanged.
- Browser, `production-battle-withdrawal-report`: Escape closes the
  summary (`key=escape status=closed`), the result is applied, the
  campaign is not left, and the frame changes.

Checks: `cargo test -p rebellion-app -p rebellion-render` (app 160, render
1,495, 0 failures), `cargo check --workspace`, `provenance-scan check` (0
uncited). `cargo mutants --in-place` on `battle_summary_key`,
`battle_category_after_arrow` and `battle_frame_draws_summary`: all
caught (the misses listed are in untouched drawing helpers, as before).

Gate runs (Linux, 640 x 480), rebuilt from this tree:

- `2026-11-12T13-11-56-655Z-1661539`: `production-battle-last-ship-lost`, 2 of 2.
- `2026-11-12T13-13-56-000Z-1700314`: `production-battle-results-presentation`, 2 of 2.
- `2026-11-12T13-14-11-290Z-1705182`: `production-battle-withdrawal-report`, 2 of 2.

## Alt accelerators under a modal window (2026-11-12, third pass)

Open question from the second pass: the Alt keys arrive as `WM_COMMAND`
through `TranslateAcceleratorA`, not as `WM_KEYDOWN`, so the F-key gate
does not cover them.

### Trace

Read from the typed corpus; vtable slots read from `REBEXE.EXE` (GOG).

1. **The pump.** `FUN_00413560` (the frame's message loop) passes each
   message to slot `+0x24` (`FUN_005fba70`: it only rewrites Enter in an
   edit control and returns 1), then calls `TranslateAcceleratorA` with
   the frame's `HWND` (`+0x18`) and the table at `+0x98`. The Command
   Center's constructor `FUN_00421c70` stores TEXTCOMM.DLL accelerator
   table 11 there. No modal check on this path. Read-in-full.
2. **Table 11** (decoded from TEXTCOMM.DLL `RT_ACCELERATOR` 11): Alt+W
   0xbb9, Alt+VK_ADD/= 0xbba, Alt+VK_SUBTRACT/- 0xbbb, Alt+P 0xbbc, Alt+Y
   0xbbd, Alt+I 0xbbe, Alt+S 0xbbf, Alt+M 0xbc0, Alt+B/T/F/O/G/U 0xbc1..
   0xbc6, Alt+D 0xbc7, Alt+V 0xbc8, Alt+A 0xbc9, Alt+1..9 0xbca..0xbd2,
   Ctrl+Tab 0xbd3, Ctrl+Shift+Tab 0xbd4, Ctrl+X 0xbd5, Ctrl+S 0xbd6, Alt+H
   0xbd7.
3. **The frame.** `Class_006588f8_vt14` (`FUN_004138a0`), `WM_COMMAND`:
   ids 3001..0xbd7 go to the current screen (`+0xc0`, the Command
   Center) as message `0x483`. Read-in-full (that branch).
4. **The Command Center.** `FUN_00422ce0`, case `0x483`, starts with
   `bVar20 = DAT_006be4e4 == 0`. Cases 0xbb9, 0xbbd, 0xbbf, 0xbc0,
   0xbc1..0xbc6, 0xbc8, 0xbc9, 0xbd3/0xbd4 and 0xbd7 act only when
   `bVar20` holds (the Agent items also need mode 1 and the item enabled
   in list `0xdead`). Cases 0xbba..0xbbc (speed), 0xbbe, 0xbca..0xbd2
   (`FUN_00425d00`, the GID displays) and 0xbd5 do not read it. 0xbc7 and
   0xbd6 have no case. Read-in-full (the case).
5. **What `DAT_006be4e4` is.** A window built by `FUN_00606380` (base
   class vtable `0x0066e060`) answers `WM_CREATE` in `Class_0066e060_vt14`
   (`FUN_00606650`) with slot `+0x34`, `FUN_006068f0`: when
   `FUN_00607390(1)` finds `+0xb8 & 1`, it calls `FUN_006010e0`, which
   installs a `WH_MOUSE` hook (`FUN_00600ef0`) and pushes the window with
   `FUN_005f6700` on the list at `0x006be4e0` (bytes `b9 e0 e4 6b 00` before
   the call), whose head is `DAT_006be4e4`. `FUN_006011f0` pops it. The
   summary (vtable `0x0065a300`) and the System Finder (`0x0065a000`) both
   keep slot `+0x34` = `0x006068f0`. So `DAT_006be4e4 != 0` exactly while
   a modal window is open. Read-in-full (`FUN_006068f0`, `FUN_00607390`,
   `FUN_006010e0`, `FUN_005f6700`, `FUN_00601320`); that the Finder's
   `FUN_00461000` passes `WM_CREATE` to the base is read from its default
   branch.

**Verdict.** A modal window (the Battle Summary, a Finder, and the other
`+0xb8` windows) refuses the Agent accelerators Alt+B/T/F/O/G/U/V/A/H
(and Alt+W/Y/S/M, Ctrl+Tab, which the port does not carry). Alt+1..9
still switch the GID display under it, and the speed keys (Alt+P,
Alt+keypad +/-) still answer.

### The change

- `crates/rebellion-render/src/agent_menu.rs`:
  `AgentCommand::accelerator_command` (table 11's id per item) and
  `accelerator_answers_under_modal_window` (the `0x483` case's
  `DAT_006be4e4` gate).
- `crates/rebellion-app/src/main.rs`: the Alt-letter loop skips a command
  the gate refuses while the windows of `window_modal_open` (the Finders,
  Status, Encyclopedia, Message Index, Battle Summary) are open. Alt+1..9
  and the speed keys are unchanged.

### Tests

- `the_agent_alt_keys_are_refused_while_a_modal_window_is_open` and
  `the_gid_display_and_speed_alt_keys_still_answer_under_a_modal_window`
  (render, cite `FUN_00422ce0` 0x483 and table 11). With the gate
  answering every command (the old behaviour), the first fails (shown).
- `cargo test -p rebellion-render -p rebellion-app`: render 1,510, app
  163, 0 failures. `provenance-scan check`: 0 uncited.
- `cargo mutants --in-place` on the two functions: 5 of 5 caught.

The browser gate will be run in the lead's batch; no browser run was
made here. `production-battle-last-ship-lost` now also, with the summary
open: presses Alt+G and Alt+H and requires no `[agent] command=...
source=accelerator` line and an unchanged frame; then presses Alt+2 and
Alt+1 (the campaign starts on Popular Support, Alt+1, so Alt+2 is needed
to see a switch) and requires `[interface] command=0x12` and then
`command=0x11 destination=gid status=selected ... source=accelerator`,
with the Alt+2 frame changed. Whether Alt+1 restores the summary's frame
exactly is recorded (`restored_frame`), not asserted.

## Proposed status

**TAC-01-C013: pass, with a note.** The summary opens over the live
Command Center as in A0: its scene 0 of 66,314 pixels off, the strategy
chrome around it 0 of 118,168 off; the rest follows game state.

**CMD-04-C018: pass, with a note.** The summary is modal per the manual
(p. 64) and the shared `+0xb8` flag; the browser case shows the cockpit
taking no click while it is open. The flag's reader is not traced.
(Third pass: one reader is `FUN_006068f0`, which puts the window on the
`DAT_006be4e4` list and installs the `WH_MOUSE` hook `FUN_00600ef0`; how
that hook picks which clicks to eat, `FUN_0060f070`, is not read.)
