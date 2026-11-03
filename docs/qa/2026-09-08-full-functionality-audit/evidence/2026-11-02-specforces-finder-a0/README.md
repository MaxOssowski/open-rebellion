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

## Character Status: Commanding names the commanded object

**A0** (day 182, `.artifacts/a0-vm/personnel-states/`):

- **`10-general-neva-menu`.** General Neva at Pzob, the Alliance's General, opened from the Personnel Finder: her menu has Retire grey, because she holds a command.
- **`11-general-neva-status`.** Her Character Status reads "Commanding: Pzob" and "Attached: Pzob".

**The source.** `FUN_004486f0` names `+0xa8`. The Command order (`FUN_00561bb0`) keys `+0xa8` (`FUN_004ee740`) to the place `FUN_0055ed30` finds for the character: the fleet of the capital ship it is aboard, otherwise its system. A move clears the command (`FUN_00560d50`).

**Before this change.** The port always wrote "None".

**The port now.** `character_rows` names the character's fleet, or else its system, while it holds a command; otherwise "None". The port already greys Retire for a commander (`character_retire::order_allowed`, `FUN_004ed460` 1).

Test: `a_commander_commands_its_system_or_the_fleet_it_is_aboard`. With the old "None" put back, it fails.

## Force Ranking in Character Status (OBJ-03 C012, C013)

**A0.**

- **`personnel-states/19-luke-status-trainee`** (Alliance, day 185). Luke Skywalker at Xyquine, Force-aware but not a Jedi, reads "Force Ranking: Trainee". His other rows are Diplomacy 75, Espionage 75, Combat 135 and Leadership 70, the rows of `a_character_at_a_system_lists_the_rows_of_figure_3_46`.
- **`status/33-emperor`** (Empire). Emperor Palpatine reads "Force Ranking: Jedi Master".

**The fixture.** `ForceStatus` (code 94) is `MissionTargeting` with a Force user as its agent. The Alliance's agent has a Force value of 50, a Trainee. The Empire's has 120, a Jedi Master (`FUN_004486f0`'s steps 10, 20, 80, 100, 120). While the Character Status window is open, `emit_status_rows` reports its rows.

**The gate.** The new `force-character` case of `status-window.mjs` opens the agent's Status from its menu, as the `character` case does. It runs that case's chrome checks and asserts the Force Ranking row: "Trainee" for the Alliance and "Jedi Master" for the Empire.

**Run.** `status-window-2026-11-02T23-23-34-648Z-1314139` (batch `b32`, both sides): pass. It read "Trainee" for the Alliance and "Jedi Master" for the Empire.

Batch `b31` ran every gate with the Commanding change and passed: workspace 3336, fixtures 116.

Batch `b32` added the ForceStatus fixture and the `force-character` and `ship-scrap` cases. It passed `status-window`, `personnel-states`, `fleet-window`, `sector-quadrants` and `agent-objectives`. The mutant that empties `StatusWindowState::object` survived the unit tests, so the test `the_window_reports_the_object_it_shows` was added; it catches the mutant.

## Travelling members in their destination's Defenses window (2026-11-03)

**A0.** `specforces/03-mon-calamari-personnel` lists these on Mon Calamari's Personnel page:

- among the characters, Garm Bel Iblis and Adar Tallon, which the Personnel Finder shows as "( Enroute )" to Mon Calamari (`00`);
- last, after the characters, the Longprobe Y-wing Recon Team, which is en route.

The team's cell is its plain GOKRES mini 17730: 0 of its 558 non-key pixels differ at (283, 307), and the en-route variant 21826 differs in 46. So the Defenses window draws no en-route mark.

**The source.** A travelling member's container is its destination. Its Move is a hidden mission (MISSNSD `+0x5c`, one of the ten hidden kinds), which page 1 keeps (`FUN_00536e20`, `+0x78` bit 8).

**The port before.** A traveller has no `current_system` while its member transit lasts, so its destination's page left it out.

**The port now.**

- `page_rows` adds each member transit bound for the system (not for an abode), as a hidden-mission member of its side.
- The page lists characters before special forces, as A0 does.

**Test.** `members_on_their_way_are_listed_at_their_destination_characters_first`:

- an Alliance traveller and a travelling Longprobe team are listed, and one bound elsewhere is not;
- an Empire system lists its own traveller.

Without the change it fails. Mutants: 5 caught, 1 unviable.
