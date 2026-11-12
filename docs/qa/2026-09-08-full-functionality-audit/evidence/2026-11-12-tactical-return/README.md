# The battle's end and the way back to strategy (2026-11-12)

This adds to TAC-01-C013 (return to strategy), TAC-07-C014 (strategic
state persistence) and TAC-01-C008 (planet backdrop). The ledger JSON is
not changed.

## A0 used

All captures are in the main checkout's `.artifacts/` (git-ignored). No new
capture was taken for this work.

| Capture | SHA-256 | What it shows |
| --- | --- | --- |
| `interface-parity/a0/TAC-01-C013/original` | `cd4eda7b3b4f45406eef7a8f9f2bbf95606fa39d8a04a85e23edd7a83bc77b21` | The Battle Summary over the strategy screen after the last Alliance ship died at Mon Calamari |
| `interface-parity/a0/TAC-07-C014/original` | `344669c2f1b8cdc970b3499b8c0d00158a7780900b62f698dc2d9e907cd1c03e` | The Sluis sector window, opened from the summary's system button |

## The port's fault: the system button opened the wrong window

The Battle Summary's command handler is `FUN_00470ea0` (vtable
`0x0065a300`, the window of `FUN_0046f360`). Its system button `0xcb` finds
the battle's system (`FUN_004f3220`) and calls `FUN_00429ce0`. That
function opens the sector window: a 235 by 360 window (`FUN_004591d0`
with `0xeb`, `0x168`), the same one the Assault Summary's `0xb` opens. The
fleet button `0xca` calls `FUN_00429ce0` too, then opens the Fleet window
(`FUN_0045c8e0`, kind 4).

The port opened the system's own window (the planet's production window)
at (85, 55) instead. A0 TAC-07-C014 shows the Sluis sector window in the
right column.

The fix: both buttons now open the system's sector window
(`open_battle_result_sector` in `main.rs`). The fleet button still opens
the Fleet window after it.

- New test `the_battle_summarys_system_button_opens_the_systems_sector_window`
  (`main.rs`). It fails when the helper opens no sector window, as the old
  route did. A second press raises the open window and opens no other.

The tactical fixtures now reveal their fleets' systems in both sides' fog
(`FogSystem::seed` after the fixture's fleets exist), as a new game does.
Without it, the Alliance's sector window hid the Imperial fleet that
survived at Mon Calamari. port: the fixture never ran a strategic day.

## The browser case

New scenario `production-battle-last-ship-lost` in `run.mjs --battle`
(tactical fixture 41, codes 65833 and 66089, appended at the end of the
catalog). It is A0 TAC-01-C003's battle at Mon Calamari: one Alliance
Medium Transport against one Imperial Victory Destroyer, no fighters. As
in A0, the Empire holds Mon Calamari and its fleet arrived last. The
transport keeps one hull point and no shield, so the first hit kills it.
The fixture skips A0's minutes of fire, nothing else.

The probe:

1. presses the pause control (575, 319) to resume the battle;
2. waits until the battle ends by itself and the summary opens;
3. checks the report's words, logged by the app;
4. for the Alliance at 640 x 480, compares the summary's scene with A0
   TAC-01-C013 outside the text boxes and the frame;
5. opens the Display page (the fourth tab) and presses the system button
   at window (29, 182);
6. requires the log `[tactical-results] sector_window system=Mon Calamari`;
7. for the Alliance at 640 x 480, compares the screen with A0 TAC-07-C014.

Results (Alliance, 640 x 480):

| Check | Result |
| --- | --- |
| Report words | "The Alliance fleet is defeated." / "Imperial forces have broken the blockade of Mon Calamari. The Alliance fleet has been completely destroyed." Word for word as A0. |
| Summary scene against A0 TAC-01-C013 | 66,314 pixels checked, 0 differ |
| Sector window place and chrome against A0 TAC-07-C014 | outline rows, outer bands, right edge and title buttons: 0 differ |
| Mon Calamari's Imperial fleet icon and facility icon | 0 differ |
| Whole screen against A0 TAC-07-C014 | about 6,500 pixels differ (see below) |

The Empire's report reads "The Imperial fleet is victorious." over the same
second sentence. A0 has no Imperial capture of this battle, so no pixels
are compared for the Empire.

The third line of the report ("destroyed.") reaches two rows below the
earlier text box in the port's font. The box grows by two rows for this
comparison. The glyphs are the accepted deviation (Liberation Sans for
GDI's Arial).

## What still differs

**The screen under the Battle Summary (TAC-01-C013).** In A0 the summary
is a strategy window over the live Command Center: the status bar's
numbers, the side bar's icons, the droids and the open sector window show
around it. The port draws the summary inside the tactical view, over the
bare shell bitmap. Only the window's inside is compared. Moving the summary
onto the strategy screen is an architecture change; it is not made here.

**The sector window's contents (TAC-07-C014).** The window's place and
chrome match. The rest follows each game's state. A0 is day 49 of a played
game. The fixture is a new game's galaxy. So the planets' bars, Bpfassh's
owner, and the names' colours differ.

A0 also draws an orange icon at Mon Calamari's bottom right; the port
draws none there. The icon lies in the planet's bottom-right quadrant
rect, `(cx + 1, cy + 1, cx + 29, cy + 20)`. That quadrant is the Missions
icon (`FUN_0045d090`, `ghidra/notes/sector-quadrants.md`): it shows when
a character or special force at the system is on a visible mission. A0's
game had such a mission there; the fixture has none. hyp: so this is game
state, not a battle mark. Its art was not matched to a resource.

## The planet backdrop after 16 turns (TAC-01-C008)

A0 TAC-01-C008 (`d0e829ffa18b40ce1ee72d4b9fcd70fefb4255507ae52d2892a722122f6879be`)
is the Empire's Battle at Umgul, paused, after Rotate Left 16 times. The
holocube's near corner sits at the bottom left, and Umgul's planet at the
top right.

New scenario `production-umgul-backdrop` (tactical fixture 42, codes
65834 and 66090). One hull and one squadron a side at Umgul, so the planet
decal is Umgul's. The probe presses the camera sphere's left control
(506, 402) 16 times, rests the cursor at A0's place (310, 470), and
measures in both images:

- the corner: the first row, from the top, where the holocube's blue
  lines run three pixels wide;
- the planet's left edge on rows 30, 40 and 60, and its bottom edge on
  columns 420 and 450.

| Measure | A0 | Port |
| --- | --- | --- |
| Corner (first 3-wide row) | (106, 298) | (108, 297) |
| Planet left edge, rows 30 / 40 / 60 | 334 / 339 / 352 | 330 / 336 / 355 |
| Planet bottom edge, columns 420 / 450 | 107 / 116 | 95 / 100 |

The probe requires the planet at the top right and the corner within 3
pixels of A0's, the unit test's tolerance. The port's lines sit one pixel
right and one pixel up of A0's: A0's vertical edge runs at x 105 above the
corner, the port's at x 106. The planet's edge differs by up to 16 pixels.
The planet's frame takes random offsets in y and z (`FUN_005d4d10`,
`rand(0.2e) - 0.1e`); the port takes their centre (port:). So its framing
cannot match A0's exactly. The 1-pixel line offset is not traced; it may
be the original renderer's pixel-centre rule.

## Proposed status

**TAC-01-C013: partial.** The battle now ends by the last ship's death in
the browser. The report's words and its scene match A0 (0 pixels differ).
The summary still opens over the tactical shell, not over the live
strategy screen.

**TAC-07-C014: pass, with a note.** The system button now opens the
sector window, as the original does. Its place and chrome match A0. Mon
Calamari's surviving Imperial fleet and facility icons match A0, and the
Alliance's lost fleet is gone, as in A0. The other differences follow the
two games' state (see above). Still open from the ledger: launched
squadrons of a lost carrier stay (port:).

**TAC-01-C008: partial.** The browser case turns the camera as A0 did.
The corner is 1 pixel off on each axis. The planet's framing depends on
random offsets the port does not reproduce.

## Gate runs (Linux, Arch Chromium)

Run ids are UTC.

- `2026-11-12T02-31-54-659Z-3643474`: `run.mjs --battle --all`, all 168
  executions pass (42 scenarios, both sides, both viewports). It holds
  both new scenarios.
- `2026-11-12T02-20-27-163Z-3455227`: `run.mjs --battle` (640 x 480 only),
  84 of 84 pass. It holds both new scenarios in their final form.
- `2026-11-12T02-19-40-739Z-3454123`: `--scenario=production-umgul-backdrop`, 2 of 2.
- `2026-11-12T02-02-27-259Z-3391357`: `--scenario=production-battle-last-ship-lost`,
  2 of 2 (before the icon checks were added).
- `2026-11-12T02-19-15-631Z-3453418`: `--scenario=production-battle-alert-entry`, 2 of 2.

An earlier run, `2026-11-12T02-04-57-140Z-3401329`, failed the score
checks of the alert-entry and results scenarios (MDATA.307/308/313/314
`loaded=false`) and was stopped. The worktree's `web/data/sounds/music`
was a link into the main checkout. `build-wasm.sh` copies the score into
that directory, so several worktrees' builds rewrote the same files
(with the same bytes) while the runtime pack read them. With a copy of
its own, the same checks pass.

## Checks

- `cargo test -p rebellion-app -p rebellion-render`: pass (app 157, render
  1,476).
- `cargo run -q -p provenance-scan -- check`: 0 uncited items.
- `cargo mutants --in-place -p rebellion-app --file
  crates/rebellion-app/src/main.rs --re open_battle_result_sector`: both
  mutants of the new helper caught. The tool also listed field-deletion
  mutants of `window_conf` and `draw_fullscreen_texture`, outside this
  change.
- `cargo mutants` on `configure_last_alliance_ship_lost_fixture`
  (`--features interface-test-fixtures`): its one mutant (an empty body)
  has no unit test; it is test-only code that the browser case exercises.
  The fixture file `tactical_test_fixture.rs` is compiled only for the
  WASM test build, so mutants do not reach it.

## Batch b195

Full gate batch b195 (2026-11-12 05:18) is green: 30 gates, check, workspace
(3605) and fixture (157) tests with 0 failures, 168 battle PASS (42
scenarios). Passed: CMD-02-C002, CMD-02-C003, TAC-07-C014, OBJ-14-C008.
