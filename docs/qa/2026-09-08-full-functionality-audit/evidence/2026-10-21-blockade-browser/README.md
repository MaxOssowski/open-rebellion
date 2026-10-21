# A fleet blockading the other side's system is told so, in the browser

Date: 2026-10-21 (overnight run). This is the browser pass for
`c5649a75` (`evidence/2026-10-21-blockade-messages/`).

## The original

A blockade's start (`0x14e` with the bit set) reaches Blockade
(`FUN_004960f0`). The blockading side reads TEXTSTRA `0x70a0` "Fleet
Initiates Blockade of {1:1}" and `0x70a1` "{3:1} has initiated a blockade
of {2:1}.".

## The gate

- A new fixture scenario, `BlockadeArrival` (76, code 77), is
  `FleetLoad` with two changes:
  - the loading target (F'tral) is the other side's, held by its
    support;
  - the player's fleet is already bound there, and the clock runs Fast.
- The observation adds the player's last Blockade title.
- `fleet-window.mjs` `blockade` checks, for both sides:
  - before the arrival, no Blockade title;
  - after it, "Fleet Initiates Blockade of F'tral";
  - 0 page errors, and 4 requests, all HTTP 200.
- The screenshots were inspected: the player's fleet stands at F'tral, a
  system of the other side's colour. Their hashes are in
  `screenshots.sha256`; the checks are in `blockade-browser.json`.

## Tests

- New: `the_blockade_scenario_sends_the_fleet_to_a_system_the_other_side_holds`.
  It checks the target's holder and support, the fleet's order, and that
  the observation keeps the player's title only.
- `cargo mutants --in-diff` on the fixture:
  - the first run missed five;
  - the new test was added, and the rerun caught 14 of 16 (2 unviable).

## Gates (`f41`)

- The Fleet window suite passes 56 cases (the 54 before, plus `blockade`
  for both sides). The Status window gate passes.
- The 24 dual-AI campaigns and the seed-42 golden are unchanged; the
  release WASM is
  `dc7af1ae64c042c5609cc83ef96f3b535e22477257c8f8cf13e280c4b2fbd025`.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.

Hashes of the committed files are in `SHA256SUMS`.
