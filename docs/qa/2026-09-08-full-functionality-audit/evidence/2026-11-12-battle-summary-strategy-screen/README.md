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
- Escape does nothing while it is open (port: its key slot is not traced;
  before, Escape in the galaxy would have left for the main menu);
- a tactical fixture that opens on the summary reports ready over the
  Command Center.

The frame in which the battle's last step runs still draws the summary
over the tactical backdrop, once; the next frame moves it to the Command
Center. The Battle Alert is unchanged (still over the screen copy or the
shell).

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
- **Keys.** The galaxy's F-key accelerators still open the Finders while
  the summary is open (they open under it). Whether the original's
  accelerators answer while a modal window is up is not traced; neither
  is the summary's own key slot (Escape does nothing).
- **The Empire's side** has no A0 capture of the summary; only the modal
  probe and the report text run for it.
- **One frame** of the summary over the tactical backdrop when the battle
  ends (see above).

## Proposed status

**TAC-01-C013: pass, with a note.** The summary opens over the live
Command Center as in A0: its scene 0 of 66,314 pixels off, the strategy
chrome around it 0 of 118,168 off; the rest follows game state.

**CMD-04-C018: pass, with a note.** The summary is modal per the manual
(p. 64) and the shared `+0xb8` flag; the browser case shows the cockpit
taking no click while it is open. The flag's reader is not traced.
