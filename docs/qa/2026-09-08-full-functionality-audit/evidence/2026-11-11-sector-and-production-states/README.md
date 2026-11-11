# Sector and production states against A0 (2026-11-11)

Cells: CMD-03-C006 (uninhabited), CMD-03-C007 (blockaded),
OBJ-10-C003 (training), OBJ-07-C010 (inactive), OBJ-09-C008 (destroyed).

## A0

Windows 10 VM, original REBEXE. The PNGs stay in the git-ignored
`.artifacts/a0-vm/`. SHA-256:

| File | SHA-256 |
| --- | --- |
| `headquarters/00-moddell.png` | `7546142eb0d26048dd3d3d4b794f3fb9a162753f73eee75d98aaebce9b467005` |
| `fleet4/bortras-blockade-ships.png` | `7ae98b0f20ffbacdbaa25cc1fd53d25e911edd439e3def82a6806a6195830f67` |
| `2026-10-28-nav/sector-b.png` | `4d4c3debd1080ef2cbf9a55a1aa6aa5b59663b53d35ab4fdaba3c189d6e445f9` |
| `sluis-van-768/07-sectors.png` | `d7b85d6a7fee7c953a984d6186da8cc2191ce7eb1f12065bbffc34a919210674` |
| `build-selection2/22-fac-menu.png` | `1c67b0a86a29db4be936ae237ff7187549e3388a9f9b6f0029680f19e5add553` |
| `2026-11-11-manufacturing-focus/00-averam-focused-day27.png` | `20ae6658c477ce88476d38f1cf08542262218b96fd60013eec6a8b3ed5a5004c` |
| `2026-11-11-manufacturing-focus/02-averam-with-yavin-defenses-opened.png` | `ecd57caf2b9feee4191c567ade1a33fec2cd528704a4288c443451e406cc9156` |
| `2026-11-11-manufacturing-focus/06-training-later2.png` | `c5949e3bd340f718d9fe950cfafb69a0458728decb42a88513ef172c29e431ba` |

`2026-11-11-manufacturing-focus/` is new today (the lead's capture: a new
Alliance game, Averam's Manufacturing window at (81, 86); its own
`NOTES.txt` and `SHA256SUMS`).

## Uninhabited planets (CMD-03-C006)

- **A0.** `headquarters/00-moddell` (Alliance, Moddell on the left
  half). Agrilat, Adega, Khuiumin, Endor and Annaj are explored. Each
  shows its energy and raw-material tracks and no support track. Basilisk,
  Vjun, Pzob, Hozrel XI and Gandolo IV show all three.
- **The original.** `FUN_0045baf0` shows the support track only for a
  populated system (`+0x88` bit 0) with a side (`+0x24 & 0xc0`).
- **A port defect found.** Each energy and raw-material slot starts with a
  1-pixel grey tick, (128, 128, 128). The port left that column empty.
  `FUN_00459e30` gives both tracks the pen `0x808080` (`FUN_0060e3b0`);
  `FUN_0045c240` and `FUN_0045c450` set the tick step to 1
  (`FUN_0060e400`); the paint (`Class_0065c058_vt14`, `FUN_004ad210`)
  draws a line at x = slot * width / slots. The port now draws the ticks
  (`sector_window::status_tracks`).
- **Browser.** New fixture 102 (`Scenario::ModdellUninhabited`) gives
  Agrilat, Khuiumin, Endor and Annaj A0's slots, no side and no facility.
  `sector-windows.mjs` opens Moddell and checks the rows between each
  picture and its name. Every pixel either game draws in a track colour
  must be the other's. Result: 0 differing at all four. (Before the tick
  fix: 273, 375, 386 and 283.) The shaded galaxy behind differs between
  the two games, so it is not compared. Adega is left out: A0's cursor
  hides the ends of its tracks.

## Blockaded system (CMD-03-C007)

- **A0.** The same new Alliance game: `2026-10-28-nav/sector-b` (day 103)
  and `fleet4/bortras-blockade-ships` (day 120, "Bortras is now under
  blockade by Alliance forces"). Around Bortras (x 170..239, y 318..393)
  the only change is Fleet 8's icon at (211..227, 332..340). No blockade
  mark appears. `sluis-van-768/07` shows the same for the Empire's view of
  Sluis Van.
- **The original.** No sector-window function reads the blockade bit
  (`+0x88` bit `0x20`): `FUN_0045bbb0`, `FUN_0045baf0`, `FUN_0045c240`,
  `FUN_0045c450`.
- **Browser.** `fleet-window.mjs` case `a0-blockade` (fixture 87,
  `FleetWindowBlockade`) now also opens Sesswenna through the System
  finder. Against A0 `bortras-blockade-ships`:
  - Bortras's picture (x 184..218, 41 rows): 0 differing;
  - the fleet icon: 0 differing;
  - right of the planet and the two rows under it: 0 differing;
  - A0 day 103 against day 120, outside the fleet icon: 0 differing.

  A0's Imperial icons left of the planet and the tracks' values are each
  game's own state and are not compared.
- The fixture test now also asserts that Bortras is blockaded
  (`BlockadeSystem::system_is_blockaded`).

## Training band's progress bar (OBJ-10-C003)

- **A0.**
  - `2026-11-11-manufacturing-focus/06` (Averam, day 38): one Alliance Army
    Regiment, one point of its six made, "Training: 1". The bar fills 26
    pixels of 160.
  - `build-selection2/22` (Coruscant): three Imperial Army Regiments, three
    points of six, "Training: 3". The bar fills 80 pixels.
- **The original.** The bar fills `(width * position) / range` in unsigned
  integer division (`Class_0065c058_vt14`, `FUN_004ad210`; the class
  `FUN_004acec0` builds).
- **A port defect found.** The port filled `160 * fraction` as a float:
  26.67 pixels, so a blended 27th column. Now
  `manufacturing_window::progress_pixels` rounds down, and `Band::progress`
  carries the points and the cost.
- **Browser.** New fixture 101 (`Scenario::BuildSelectionTraining`).
  `build-selection.mjs` case `trainingBand` checks the band's lines and
  the bar's frame and well (window (55, 207)..(217, 213), 1141 pixels):
  - Alliance against A0 06: "Alliance Army Regiment", "Training: 1", 26
    yellow pixels, 0 differing;
  - Empire against A0 22: "Imperial Army Regiment", "Training: 3", 80
    yellow pixels, 0 differing.

## Inactive Manufacturing window (OBJ-07-C010)

- **The reading.** In `FUN_00457690`, the "inactive shell" is the title
  strip. `+0x180` is drawn when the window is the galaxy view's focused
  child (`+0x14c` `+0xb8` equals the window's handle), otherwise `+0x184`.
  `FUN_00456230` loads them: side 1 10299 / 10200, side 2 10201 / 10302,
  otherwise 10303 / 10304. Here `+0x160` is the window's page
  (`0x67..0x6c`), not a manager's idleness.
- **A0.** `2026-11-11-manufacturing-focus/00`: Averam's strip is focused,
  (255, 0, 0). `01`: a click on a sector window does not take the focus.
  `02`: opening Yavin's Defenses window beside it turns Averam's strip to
  (179, 4, 4), the unfocused art. The bands do not change.
- **The port.** Each window kind keeps its own focus: the last Manufacturing
  window is always drawn focused, even with a Defenses or Fleet window
  opened after it. The art per state is right (`title_resource`), but the
  cross-kind focus is not modelled. Not fixed here; it touches the System,
  Defenses, Fleet and Missions windows together. No browser case yet.

## Facility Lost (OBJ-09-C008)

No A0 capture. Nothing changed.

## A0 still needed

- **OBJ-09-C008.** Any side. Order a facility (a Mine or Refinery) with its
  Destination set to another system, then have that system lost (taken by
  the enemy, or out of free slots) before it arrives. Capture the Facility
  Lost message (TEXTSTRA `0x70b0` "Facility Lost", `0x70b1` "New {1:1}
  could not be deployed to {2:1}. The facility has been scrapped.") on the
  Manufacturing rail, its Message Index row, and the opened message.
- **OBJ-07-C010.** Done (`2026-11-11-manufacturing-focus/00`, `02`). The
  port needs the cross-kind focus, then a browser case.

## Tests

- New: `a_bands_progress_bar_fills_whole_pixels_rounded_down`
  (`system_window.rs`). It failed before the fix: 53.33 against 52 at
  scale 2.
- Changed with the evidence above:
  `the_status_tracks_stack_energy_raw_and_support_under_the_planet` now
  expects the grey ticks. It failed before the fix.
- New fixture tests:
  `the_training_scenario_puts_a0s_regiments_part_made_in_the_primarys_training_queue`,
  `the_moddell_scenario_leaves_a0s_four_systems_explored_uninhabited_and_unheld`;
  the A0 blockade scenario test also asserts the blockade.

## Runs (Linux Chromium, this worktree's `.artifacts/interface-parity/`)

All four on the same fixture WASM, SHA-256
`9391521734f8d879d3a1335ebb13770a9991e241e02cbbc7cc267c43183cac26`.

| Gate | Run | Result |
| --- | --- | --- |
| `sector-windows.mjs` | `sector-windows-2026-11-11T14-37-32-254Z` | pass; uninhabited 0/0/0/0 |
| `build-selection.mjs` | `build-selection-2026-11-11T14-38-11-731Z-855904` | pass, both sides; `trainingBand` 0 differing |
| `sector-quadrants.mjs` | `sector-quadrants-2026-11-11T14-42-16-673Z-942028` | pass (the ticks change nothing there) |
| `fleet-window.mjs` | `fleet-window-2026-11-11T14-46-12-946Z-1048051` | pass, all cases, both sides; `a0-blockade` sector 0 differing |

- `cargo test`: rebellion-render 1458 passed; rebellion-core 1728;
  rebellion-app with `interface-test-fixtures` 153.
- `cargo run -q -p provenance-scan -- check`: 0 uncited items.
- `cargo mutants --in-place -p rebellion-render`: `status_tracks`,
  `progress_pixels` and `band`, 120 mutants, 120 caught; the
  `system_window.rs` diff, 3 mutants, 3 caught.
- Not rerun here: the other interface gates (the lead's batch).

## Batch b190

- 29 of 29 interface gates on Linux, among them `mission-dialog`
  (`mission-dialog-2026-11-11T15-24-49-907Z-1240209`), `status-window`
  (`status-window-2026-11-11T15-19-30-211Z-1232384`), `sector-windows`
  (`sector-windows-2026-11-11T15-42-55-703Z`), `build-selection`
  (`build-selection-2026-11-11T15-21-54-673Z-1237841`) and `fleet-window`
  (`fleet-window-2026-11-11T15-08-23-158Z-1206459`).
- Workspace tests: 3580 passed, 0 failed. Fixture tests: 155. Provenance
  is ok.
- Tactical matrix `2026-11-11T16-06-34-266Z-1301758`: 148 of 148.

## Shared window focus (OBJ-07-C010), 2026-11-11 evening

The user approved the architectural change on 2026-11-11.

- **The port.** `rebellion-render/src/window_focus.rs`: the System, Fleet,
  Defenses and Missions window kinds each keep a `FocusStamp`. A kind
  stamps itself whenever it raises a window: it opens, restores or
  focuses one, and a press on a window focuses it. Each frame `main.rs`
  gives the shared focus to the kind with the latest stamp among those
  with a window open (`shared_focus`). Only that kind's top window draws
  the focused strip (`FUN_00457690`, `+0x180` against `+0x184`). The
  sector window keeps its own focus: A0 `01` shows that a press on it does
  not take the focus.
- port: the per-kind stacks and the stamps are the port's. The original
  keeps one z-order of child windows. The painting order across kinds is
  unchanged: it is still fixed by kind.
- **Browser.** `sector-quadrants.mjs` `shared-focus`, both sides. The
  Defenses window is opened and dragged aside, then the System window is
  opened.
  - The System strip is focused (10299 / 10201) and the Defenses strip
    unfocused (10200 / 10302).
  - A press on the Defenses title bar swaps them.
  - A press on the System title bar swaps them back.
  - Each strip region (40 by 16, right of the title) is 0 pixels off its
    bitmap.
  - Run `sector-quadrants-2026-11-11T21-06-04-376Z-2115663`, pass.
- **Tests.**
  - `window_focus::tests::*`.
  - `system_window::tests::the_window_draws_its_unfocused_strip_while_another_kind_holds_the_focus`
    fails without the change (the strip stays 10299).
  - `the_kind_claims_the_shared_focus_only_while_a_window_is_open` in each
    of the four window modules.
- **Mutants.**
  - `window_focus.rs`: 10 of 10 caught.
  - The accessors and draw functions of the four windows: 45 caught, 4
    unviable, 0 missed.

## Batch b193

Full gate batch b193 (2026-11-11 23:34) is green: 30 gates, check, workspace
(3593) and fixture (156) tests with 0 failures, 152 battle PASS. Its
`sector-quadrants` run `sector-quadrants-2026-11-11T21-29-32-961Z-2350269`
includes the `shared-focus` case. OBJ-07-C010 passes on it with A0
manufacturing-focus/00 and /02.
