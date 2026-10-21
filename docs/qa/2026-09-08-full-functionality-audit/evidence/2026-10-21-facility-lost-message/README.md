# Only a facility lost on arrival is reported (Facility Lost, 0x303)

Date: 2026-10-21 (overnight run, "Produkcja i naprawy").

## The original

- `GameObjDestroyedOnArrivalNotif` (event `0x303`, `FUN_004fc080`) maps
  to message type `0x10` (`FUN_00488de0`), Facility Lost
  (`FUN_00497690`).
- That class speaks only for an object of family `0x20`..`0x2f`, a
  facility. Its texts are TEXTSTRA `0x70b0` "Facility Lost" and `0x70b1`
  "New {1:1} could not be deployed to {2:1}.  The facility has been
  scrapped.", naming the facility and the destination.
- Any other unit lost on arrival posts nothing.

## What the port did before

Every lost delivery posted "Unit lost on arrival: X no longer exists" to
the log.

## Port

- `story_messages::facility_lost_texts` returns the texts for a facility
  and `None` for any other kind.
- The app files the message on the Manufacturing rail for the delivery's
  side (`Delivery::side`).

## Tests

- New: `only_a_facility_lost_on_arrival_is_reported`.
- `cargo test --workspace`: 2506 passed, 0 failed.
- `cargo mutants --in-diff`: 6 mutants, 6 caught.

## Gates (`f31`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `e2b4be84e804bfc1f41849c621fccf6c367cdeda80cfdd0b4bcb515ba72f9396`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- Not checked: the message in the browser.

Hashes of the committed files are in `SHA256SUMS`.
