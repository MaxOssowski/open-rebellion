# What a Finder's Display opens (2026-11-03)

Surface ledger OBJ-04 C012, "Troop Finder Display opens System window".

## A0

The A0 run used the Alliance's DS_flip_allian save, days 184 and 185. The captures are in `.artifacts/a0-vm/personnel-states/` and `.artifacts/a0-vm/specforces/`, with hashes in `a0-sha256.txt`.

**From the Troop Finder.** A double click on a row opens that system's Defenses window on its Trooper Regiments page:

- `14` shows the Troop Finder;
- `15` is Mon Calamari's window, from the Alliance tab;
- `17` is Pzob's window, from the Alliance tab;
- `18` is Umgul's window, from the Imperial tab.

**From the Personnel Finder's SpecForces view** (2026-11-02, `specforces/02` and `03`):

- Display on Mon Calamari opens its Defenses window, with the en-route Longprobe team listed;
- Display on Tralus opens "Mission at Praesitlyn", the window of the team's running mission.

`FUN_00429440` routes a special force that way (`personnel-finder.md`): to its running mission's Missions window, otherwise to the Defenses window.

## The port before

- The Troop Finder opened the Defenses window on its Personnel page, the window's default.
- A SpecForces system row opened the sector window alone.

## The port now

**The Troop Finder.** Its Display, Enter or double click goes to `FinderTarget::Regiments`. That opens the Defenses window and shows its regiment page (`DefensesWindowState::show_page`).

**A SpecForces system row.** `PersonnelFinderAction::OpenSystem` carries the tab's first special force at the system (hyp: the first in the system's list).

- If that unit is on a mission, `main.rs` opens the Missions window showing it.
- Otherwise it opens the Defenses window.

port: special forces carry no hidden-mission flag.

## Tests

- `sector-quadrants.mjs` `troop-finder-display` (Linux, both sides) presses F4, then Enter on the first row, and asserts a Defenses window on the `regiments` page with rows.
  - Against the old test WASM it failed on both sides with page `personnel`.
  - Against the new one it passed: run `sector-quadrants-2026-11-02T23-49-25-430Z-1357942`.
- `the_action_for_a_character_row_opens_character_and_sf_row_its_first_unit` (`personnel_finder.rs`) checks that the row picks the tab's first unit, for each tab.
- Mutants on the change: 1 caught, 2 unviable.
