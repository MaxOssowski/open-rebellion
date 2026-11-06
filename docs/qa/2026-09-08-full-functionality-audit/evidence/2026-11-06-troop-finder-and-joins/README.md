# The Troop Finder's other side, and System Joins against a clean A0 game (2026-11-06)

This covers OBJ-04-C011 (the Troop Finder's faction tabs) and EVT-01-C004 / CMD-08-C008 (System Joins).

## The Troop Finder lists the other side's regiments only where seen

**A0.** The VM's unedited "test" save (Alliance, day 220):
- the Alliance tab lists Averam, Fleet 1, Fleet 3, Umgul, Vjun and Yavin (`.artifacts/a0-vm/troop-other-side/test-day220-alliance-tab.png`, sha256 `729658350fd899bab0e9fda0f2305f3a766f02b08ca5baf583237a61dc17fa64`);
- the Imperial tab lists none (`test-day220-imperial-tab-empty.png`, sha256 `89bdf2e2cec91559b941214f47ca6b40aca2884ea9bd0307b95776878a4e82b9`).

The Empire surely holds regiments by day 220, so the walks (`FUN_0046ea10`, `FUN_00504cc0`) run over the player's side's views. The port's comment had said they test no visibility.

**Before.** `troop_finder::rows` listed every regiment of the tab's side.

**Now.** `rows` takes the fog and the player's side, and skips a system of the other side's tab unless `opposing_contents_visible` holds there. That is the Fleet Finder's and the Personnel Finder's rule. `draw_troop_finder` takes the player's fog.

**hyp.** Per-side views are not ported.

**Tests.**
- `the_troop_finder_lists_systems_holding_the_tabs_regiments`:
  - Coruscant's Imperial regiment is not listed for the Alliance until its fog reveals Coruscant;
  - the Empire lists it either way;
  - the test fails without the change.
- `the_opening_and_a_new_tab_choose_the_row_the_name_begins_or_the_first` puts its Imperial regiment on a planet the Alliance holds, so the regiment is seen.

**Mutants.**
- In `rows` and in the new side choice in `draw_troop_finder`, every mutant is caught.
- The 38 missed mutants are all in `draw_troop_finder`'s existing paint arithmetic, from line 595 on, which this change does not touch.

**Browser** (`personnel-states.mjs`, Linux, Alliance):
- F4, then the Imperial tab.
- The empty list (99..449, 192..353) and the side tabs differ from A0 in **0 pixels**.
- Screenshot: `.artifacts/interface-parity/personnel-states-2026-11-06T18-33-20-196Z/alliance/troop-finder-other-side.png` (sha256 `ae47cd2e255a47090859d8d010558156fa027534139a0e25c7a3f3a9a712bd60`).

`encyclopedia-placement.mjs --window=finders` still finds rows on its fixture's Imperial tab, and its first row is chosen. That fixture reveals Corfai, Corsin and Coruscant to the Alliance, so the rows are seen ones.

## System Joins opened, against a clean A0 game

**A0.**
- `.artifacts/a0-vm/joins-clean/test-day214-index.png` (sha256 `5aa785b87db1cd0cf47ba9294564ca9c70583e831bf6f5acbe2fe50d746c5438`) lists "Xyquine Joins Enemy".
- `test-day215-xyquine-joins-enemy.png` (sha256 `c253dbf1512ced8f99a43cf7571f7c747a379342d2880f77ead4e4323c398c6c`) shows it over "Popular dissent on Xyquine has caused that world to join the Empire.".
- That is `0x7039` / `0x703c` for the Alliance reader, on picture STRATEGY 1005 (`0x3ed`), exact at (74, 82).

**Browser** (`message-display.mjs`, Linux, Alliance):
- The gate opens the fixture's "Commenor Joins Enemy" and reads "Popular dissent on Commenor has caused that world to join the Empire.", with background `0x3ed`.
- Its picture differs from STRATEGY 1005 and from A0 in 0 pixels.
- The title's item image differs from A0 in 0 pixels.
- Screenshot: `.artifacts/interface-parity/message-display-2026-11-06T18-33-33-716Z/alliance/joins-enemy.png` (sha256 `f82b384b862cec5335cce8600aebaa936376cbfd411a74632a534453ca1ae10e`).

## Also seen

A clean Enemy Mission Foiled for the Alliance (`.artifacts/a0-vm/foiled-clean/test-day357-enemy-mission-foiled.png`, sha256 `8fc715fdafb8bb330eaab91c1b3b02a2e1f37c08cf7ff479b886288f5e70cb3f`) reads "An enemy Espionage mission to Selonia has been foiled by our forces.". Its picture STRATEGY 1073 (`0x431`) is exact, as the port gives the Alliance reader. `message-display.mjs` already compares this message with A0 `enemy-foiled-chandrila-day739`.

## The batch (b104)

- 20 of 21 gates passed.
- `encyclopedia-placement.mjs --window=finders` failed once: its fixture's Imperial tab was empty. Before the batch, the same gate on the same tree had found rows for Corfai, Corsin and Coruscant. That galaxy's fog moves while it runs, so the tab is empty or not depending on the moment.
- The gate now accepts either state: the first row chosen, as A0 `troop-finder-first-row/alliance-tab` shows, or no row at all (every row band under 100, the list's background).
- A row shown but not chosen (grey, about 117) still fails.
- The rerun passed.
