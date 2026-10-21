# A unit's deployment and arrival go to the rail of its family

Date: 2026-10-21 (overnight run, "Produkcja i naprawy").

## The original

- Units Deployed (`FUN_00497940`) and Unit Arrival (`FUN_004981c0`) set
  their message's category mask (`+0x34`) by the unit's family.
- The Message Index files a message on the rail of its mask
  (`FUN_0042d8d0`, `message-index-rows.md`). The masks:

| Unit | Deployed | Arrival |
|---|---|---|
| capital ship, Death Star (`0x14`..`0x1b`) | `0x80` Fleet | `0x80` Fleet |
| squadron, regiment at a system | `0x40` Defense | `0x40` Defense |
| special force (`0x3c`..`0x3f`) | `0x10` Mission | no message |
| defense facility (`0x22`..`0x27`) | `0x40` Defense | `0x40` Defense |
| shipyard, training, construction (`0x28`..`0x2b`) | `0x08` Manufacturing | `0x08` Manufacturing |
| mine, refinery (`0x2c`..`0x2f`) | `0x04` Resource | `0x08` Manufacturing |

## What the port did before

Every completion, departure and arrival went to the Manufacturing rail.

## Port

- `story_messages::unit_message_rail(kind, arriving)` gives the rail.
- The app files the completion, departure (Units Deployed) and arrival
  (Unit Arrival) messages on that rail.
- port: a completed or delivered squadron or regiment stands at its
  system, so it takes the system mask `0x40`.

## Tests

- New: `a_unit_s_message_goes_to_the_rail_of_its_family`.
- `cargo test --workspace`: 2509 passed, 0 failed.
- `cargo mutants --in-diff`: 3 mutants, 2 caught, 1 unviable.

## Gates (`f36`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `d21e013d3d3c8a0454e375cd91a5284436451d00314f4b0c7e0b3e7b94eee315`).
- The Fleet window suite passes 54 cases, including `delivery`. That
  case reads the messages by audience, not by rail.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.

Hashes of the committed files are in `SHA256SUMS`.
