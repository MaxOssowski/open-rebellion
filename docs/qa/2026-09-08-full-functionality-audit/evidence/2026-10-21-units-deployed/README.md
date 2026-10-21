# A completed or delivered unit posts Units Deployed (0x107)

Date: 2026-10-21 (overnight run, "Produkcja i naprawy").

## The original

- Units Deployed (`FUN_00497940`) answers `GameObjDeployedNotif`
  (`0x107`) for a unit put in place, to the unit's side, when a system is
  found up its containers. Its texts follow the unit's family:
  - a capital ship (`0x14`..`0x17`): TEXTSTRA `0x7086` "{1:3} Deployed
    at {2:1}" and `0x7087` "The new {1:3}, {1:1}, has been deployed at
    {2:1}.";
  - the Death Star (`0x18`..`0x1b`): `0x708e` / `0x708f`;
  - a squadron (`0x1c`..`0x1f`): `0x708a` / `0x708b`;
  - a regiment (`0x10`..`0x13`) or special force (`0x3c`..`0x3f`):
    `0x7088` / `0x7089`, then a `0x707f` line for the unit. The special
    force `0x3c000009` stays silent;
  - a facility (`0x20`..`0x2f`): `0x708c` / `0x708d`.
- The unit's text provider (`FUN_004f6030`) gives kind 1 its own name,
  or its class's when it has none (`FUN_004f62d0`). Kind 3 is the class's
  name.

## What the port did before

- A completion posted "Construction complete at X".
- A delivery's arrival posted "Unit arrived at Y".

## Port

- `story_messages::deployed_texts` picks the texts by the built kind.
  `MessageParameter::Unit` gives kinds 1 and 3.
- The app posts them for a completion and for a delivery's arrival. The
  port's lines stay only as a fallback when TEXTSTRA is missing.
- port: the port names no unit, so kind 1 is the class's name. A
  completed or delivered unit stands at its system, so the body's
  container is the system.
- The departure line ("Construction complete at X, en route to Y") is
  the port's and stays; no original class answers a departure.

## Tests

- New: `a_deployed_unit_takes_the_texts_of_its_family`.
- `cargo test --workspace`: 2503 passed, 0 failed.
- `cargo mutants --in-diff`:
  - the first run missed the special-force guard;
  - a second special force was added, and the rerun caught 14 of 14.

## Browser

The Fleet window suite's `delivery` case now waits for the arrival's
Units Deployed title (`delivery-browser.json`, 0 errors):
- Alliance: "Mon Calamari Cruiser Deployed at F'tral";
- Empire: "Strike Cruiser Deployed at F'tral";
- "Shipyard Idle on Akrit'tar" for both sides.

## Gates (`f28`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `fe417cc878023c772be672af427eb4371c48a1b7c2b3c45559c21b8bfde28482`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.

Hashes of the committed files are in `SHA256SUMS`.
