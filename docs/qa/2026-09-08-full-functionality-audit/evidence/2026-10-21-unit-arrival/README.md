# A delivery is Deployed at departure and reports its arrival (0x107, 0x105)

Date: 2026-10-21 (overnight run, "Produkcja i naprawy"). This corrects
the timing in `801de8b3` (`evidence/2026-10-21-units-deployed/`).

## The original

- `FUN_0052bee0` completes a product: it sets Deployed (bit 16,
  `FUN_004f7aa0`, which posts `GameObjDeployedNotif` `0x107`), then
  EnrouteActive when the product is bound elsewhere.
  - The product is already a child of its destination
    (`build-delivery.md`).
  - So Units Deployed (`FUN_00497940`) speaks at departure and names the
    destination: "Strike Cruiser Deployed at F'tral".
- On arrival EnrouteActive clears, and `GameObjEnrouteActiveNotif`
  (`0x105`) maps to message type `0xd` (`FUN_00488de0`), Unit Arrival
  (`FUN_004981c0`). Its texts:
  - a capital ship: TEXTSTRA `0x7076` "Ships arrive at {2:1}" and
    `0x7077`;
  - a squadron, regiment or facility: `0x707a` "Units Arrive at {2:1}"
    and `0x707b`;
  - each adds a `0x707e` line for the unit;
  - characters and special forces have no message here;
  - the class is silent while the system has a battle pending (`+0x88`
    bit 12).

## What the port did before

- At departure the port posted its own line, "Construction complete at
  X, en route to Y".
- `801de8b3` posted Units Deployed at the arrival instead.

## Port

- The departure posts Units Deployed naming the destination.
- The arrival posts `story_messages::arrival_texts`.
- port: the port keeps no battle-pending bit, so the arrival is never
  silenced by a pending battle.

## Tests

- New: `an_arrival_reports_ships_or_units_and_lists_the_unit`.
- `cargo test --workspace`: 2507 passed, 0 failed.
- `cargo mutants --in-diff`: 7 mutants, 7 caught.

## Browser

The Fleet window suite's `delivery` case now checks, for both sides
(`delivery-browser.json`, 0 errors):
- departure: "Mon Calamari Cruiser Deployed at F'tral" (Alliance) or
  "Strike Cruiser Deployed at F'tral" (Empire), with no ship at F'tral
  yet;
- "Shipyard Idle on Akrit'tar";
- arrival: "Ships arrive at F'tral", with one ship there.

## Gates (`f32`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `1259e55dc13883cb59253569b7a9b5a6110ac9a0ec0a12ae1d73af46bebba4ae`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.

Hashes of the committed files are in `SHA256SUMS`.
