# The Personnel Finder's SpecForces view against A0 (2026-11-02)

Surface ledger OBJ-03 C018, "SpecForces Finder type-by-system grid".

## A0

The A0 run used the Windows 10 VM and the Alliance's "DS_flip_allian" save, days 165..170. The captures are in `.artifacts/a0-vm/specforces/`, with hashes in `a0-sha256.txt`.

- **`01-alliance-grid-day165`.** F5 opens the Personnel Finder at (62, 50). Its second view button opens the SpecForces view:
  - one row per system: Drall, Mon Calamari, Tralus, Yavin;
  - four type columns under icons;
  - a count in a red box, or "-".

  The boxes of units en route or on a mission (Mon Calamari, Tralus, Yavin) are dark red. Drall's is bright.
- **`03-mon-calamari-personnel`.** Display on Mon Calamari opens its Defenses window. The Longprobe Y-wing Recon Team is listed last. Its menu greys Mission because the team is en route.
- **`02-recon-mission-praesitlyn-day170`.** Display on Tralus opens the window of the team's running mission instead: "Mission at Praesitlyn", Reconnaissance.

## The port's view

`encyclopedia-placement.mjs --window=finders` (Alliance) now presses the SpecForces view button: the finder's (423, 147), 0x36 lower.

- **Header strip.** It compares the strip of the four type icons, the window's (256..370, 114..142), with A0 `01`: 0 of 3,192 pixels differ.
- **The rest of the view.** It matches A0 by eye in `personnel-finder-specforces.png`: the row and column places, the boxes, the dashes and the grey system names. The port's world has other systems (Ghorman, Ithor).
- **Not compared: dark red boxes.** No unit in the port's run was moving or on a mission, so the dark red state of en-route and on-mission units was not compared.

Run `encyclopedia-placement-2026-11-02T12-38-36-124Z` (Arch Chromium, release WASM `e1ad53f8…`): pass.

## The dark boxes (ported the same day)

`FUN_00465540` counts each column's units. A unit whose `+0x50` bit 0 is clear (en route or on a mission) sets the column's flag 4. Such a column is painted with the side's dark brush: (128, 0, 0) for the Alliance, (0, 128, 0) for the Empire. These are the brushes 3 and 4 of the Troop Finder's `FUN_0046d8d0`, which has the same test.

The port's `count_sf` now clears `all_usable` for a unit on a mission. A unit moves as a Move mission in the port, so en route is covered too. `sf_cell_colours` picks the dark brush for that column. Before this change, every box was bright.

Test: `a_column_with_a_unit_away_is_drawn_in_the_sides_dark_brush` (`personnel_finder.rs`).

The Troop Finder keeps its bright boxes. A0's Troop Finder (DS_flip_allian, day 164) shows Xyquine's three regiments dark, but why their bit 0 is clear is not known.

## The Alliance's Personnel Finder (OBJ-03 C001)

**A0.** `00-personnel-finder-alliance-day161` (DS_flip_allian, day 161): F5 opens the Alliance's Personnel Finder at (62, 50) on the Characters view. Its rows read "Name - System", with the state in parentheses when there is one, for example "( Enroute )", "( On Mission )" or "( General )".

**The port.** `encyclopedia-placement.mjs --window=finders` (Alliance) compares the port's Finder with this capture pixel for pixel. The list, its bar, the name box and the three texts are left out: "Personnel Finder", "Name" and "Alliance Personnel" differ only by GDI hinting. Every other pixel of the window matches but 2. Run `encyclopedia-placement-2026-11-02T19-54-39-200Z`: pass.

## A travelling character names its destination (OBJ-03 C007)

**A0.** In `00-personnel-finder-alliance-day161`, the characters on their way read like "Adar Tallon - Mon Calamari ( Enroute )" and "Garm Bel Iblis - Mon Calamari ( Enroute )". Each names its destination, which in the original is already the character's container.

**Before this change.** The port keeps a travelling character outside any system, with the destination in the missions' member transits. Its row read "Adar Tallon - Location Unknown ( Enroute )".

**The port now.**

- `PersonnelFinderState::set_destinations` takes each travelling character's destination, the transit's `to`. `main.rs` sets it every frame from `MissionState::en_route`.
- The row names that system: `character_rows_travelling`, `character_text`.
- Test: `a_travelling_character_names_its_destination_and_enroute`. The function is new, so the test does not build without the change, and the old path's "Location Unknown" is asserted beside it.
- No browser case has a travelling character yet.

Batch `b28` passed every gate: workspace 3333, fixtures 116. The mutant that empties `set_destinations` is caught by the test's state check.

## The Characters rows' states in the browser (OBJ-03 C005..C008)

**A0.** `personnel-states/00` and `02` (`.artifacts/a0-vm/personnel-states/`, DS_flip_allian, day 180) show the Alliance's list from top to bottom. The states are:

- available, no state: "Chewbacca - Yavin", "Crix Madine - Drall";
- "( Enroute )": "Adar Tallon - Mon Calamari ( Enroute )";
- "( On Mission )": "Drayson - Corellia ( On Mission )", "Han Solo - Drall ( On Mission )";
- "( General )": "Huoba Neva - Pzob ( General )", "Sarin Virgilio - Mon Calamari ( General )".

No character in the save is captured, injured or dead.

**The fixture.** `PersonnelStates` (code 93) takes three of the player's characters:

- the first is on a mission at the primary system;
- the second travels to the secondary system with a member transit (`MissionState::push_transit`, now public for the fixtures);
- the third is a General.

**The gate.** `personnel-states.mjs` presses F5 for each side and reads the Characters rows the fixture reports (`emit_personnel_finder`, `PersonnelFinderState::shown_character_rows`). It asserts one row of each state, that the traveller is not "Location Unknown", and, for the Alliance, rows with no state.

## The Finders' right-click menu

**A0.** A right click on a Personnel Finder row opens a pop-up menu with Encyclopedia and Status. Both are grey, and hovering or clicking Status does nothing (`personnel-states/03`, `04`). The Fleet Finder opens the same menu over an empty part of its list (`05`). Escape closes the menu and leaves the Finder open (`06`). The menu is 2 rows high, about 42 pixels, as A0's 7-row character menu is 142.

**The source.** The System, Fleet, Personnel and Troop Finders' vtables (`0x65a000`, `0x65a090`, `0x65a130`, `0x65a270`, read from REBEXE.EXE) put `FUN_004ac5c0` in slot 7, the shared object pop-up menu, and `FUN_00461740` in the selection slot `+0x58`. `FUN_00461740` returns nothing, so `FUN_0051d990` lists only Encyclopedia and Status, both disabled for an empty selection.

**Before this change.** A right click on a Finder did nothing in the port.

**The port now.**

- `ObjectMenuState::for_finder` opens the menu with no selection on a right-button release over an open Finder.
- `main.rs` calls it for the four Finders. While a Finder is open, the menu takes the input, and its Escape closes only the menu.
- Test: `a_right_release_on_a_finder_opens_encyclopedia_and_status_disabled`. The function is new, so the test does not build without the change.
- `personnel-states.mjs` right-clicks the list and asserts the two grey rows and the 42-pixel height. It then presses Escape and checks that the menu closes while the Finder stays open.

## The Fleet Finder's tabs, name box and empty list (OBJ-04 C004, C005)

**A0** (DS_flip_allian, day 182, `.artifacts/a0-vm/personnel-states/`):

- **`08`.** The Alliance tab, with the name box empty, lists Fleets 4, 6 and 7 and chooses the first.
- **`09`.** Typing "Fleet 6" chooses Fleet 6 and leaves Fleets 4 and 7 listed. The name box chooses a row; it filters nothing.
- **`07`.** The Imperial tab lists nothing, because no Imperial fleet is known, and shows no scroll bar.

**The port's gates.**

- `fleet-finder.mjs` "tabs" checks the tab's choice (FUN_00609650 on the box's text).
- `fleet-finder.mjs` "type-and-enter" checks the typed choice. It now also asserts that typing keeps every row.
- `personnel-states.mjs` opens the Fleet Finder in a galaxy that shows no fleet of the other side. It clicks that side's tab and asserts an empty list.

## The Troop Finder's other tab (OBJ-04 C011)

`encyclopedia-placement.mjs --window=finders` (Alliance) reopens the Troop Finder with the name box empty and presses the Imperial tab at the window's (88, 72). It checks that:

- the heading changes;
- the first row, Corfai, is white and the others are grey.

A0 `troop-finder-first-row/alliance-tab` shows the same for the Empire's player pressing the Alliance tab: Yavin is chosen. Run `encyclopedia-placement-2026-11-02T22-13-36-521Z` (Alliance): pass.

## Runs

Every gate passed in batch `b29`, except `personnel-states`. That gate was edited after its WASM was built and expected the General before the fixture had one.

Batch `b30` ran with the test WASM that has the General and the Finder menu:

- workspace 3335, fixtures 116, provenance clean;
- mutants: 6 caught, 1 unviable;
- `fleet-finder`, `system-finder`, `personnel-orders`, `status-window`, `encyclopedia-placement --window=finders` and `agent-objectives`: pass.

In `b30`, `personnel-states` failed for the Empire on a race in the gate: it cleared the menu report before the Escape's frame. The failure screenshot shows the menu closed and the Finder open. Now the gate clears the report after that frame. The rerun `personnel-states-2026-11-02T22-02-26-773Z` passed on both sides (test WASM `480e1913…`).

The placement step's first version moved the mouse away within the release's frame, so egui saw no click. The step now waits a frame and checks the heading. Rerun: `encyclopedia-placement-2026-11-02T22-13-36-521Z`.
