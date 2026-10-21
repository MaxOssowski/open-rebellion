# Each production area reports its own idleness (0x160)

Date: 2026-10-21 (overnight run, "Produkcja i naprawy").

## The original

- A system has one manufacturing manager per area (`FUN_00509670`).
  Each manager posts `ManuMgrRemainingGameObjCountNotif` (`0x160`,
  `FUN_0052baa0`) when its remaining count changes.
- Facility Idle (`FUN_0048be60`) speaks when a manager's count is 0. It
  needs three things:
  - the viewer's side holds the system;
  - the walk finds a facility of the area's family at the system
    (`FUN_00520580`): `0x28` shipyards, `0x29` training facilities,
    `0x2a` construction yards;
  - the manager's area, by slot `+0x1ec` (1 construction, 2 training,
    otherwise shipyards).
- Its texts are TEXTSTRA `0x71f8` / `0x71f9` (construction yards),
  `0x71fa` / `0x71fb` (training facilities) and `0x71fc` / `0x71fd`
  (shipyards), with the system as `{1:1}`.
- Its picture is the facility the walk found (`FUN_004c5000`).

## What the port did before

- `ManufacturingAdvance::newly_idle` held a system only when all its
  areas were empty, so it reported once per system, after the last area.
- The app posted "Manufacturing queue idle at X" to the log, not to the
  Message Index, with the picture of the system's first yard.

## Port

- `newly_idle` holds `(system, area)` for each manager whose queue
  emptied that day.
- The headless telemetry (`EVT_MANUFACTURING_IDLE`) records the area.
- `story_messages::facility_idle_texts` picks the texts by area, and
  `message_pictures::idle_yard` takes the first facility of the area's
  family.
- The app posts the message on the Manufacturing rail to the player when
  the player holds the system and it still has such a facility.

## Tests

- Changed, with the evidence above: `a_system_goes_idle_once_when_its_last_area_finishes`
  becomes `each_area_goes_idle_when_its_own_queue_empties`. The training
  area now reports idle on day 2 and the shipyards on day 4. The old
  semantics fail it (`fail-without.txt`).
- New: `an_idle_area_names_its_own_kind_of_facility`.
- The idle picture test now asks for the shipyard area and finds no
  training facility.
- `cargo test --workspace`: 2502 passed, 0 failed.
- `cargo mutants --in-diff`: 10 mutants, 9 caught, 1 unviable.

## Browser

- The Fleet window suite's `delivery` case now also waits for the
  player's "Shipyard Idle on Akrit'tar" after the ship departs. It
  passes for both sides (`delivery-browser.json`, 0 errors).
- Screenshot hashes are in `screenshots.sha256`.

## Campaigns (`f27`)

- `rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days.
  Every seed is identical to the stage before except the idle telemetry
  (`campaigns.txt`):
  - it now has one record per area, with its area;
  - 18557 records become 32484.
- The simulation state and the seed-42 golden are unchanged; the replay
  equivalence check passes with 0 browser errors (release WASM SHA-256
  `9e93f6f4100b4f9a935241cc476e4c37c0512bdc048a503e32f7c2f64cb45f7f`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.

Hashes of the committed files are in `SHA256SUMS`.
