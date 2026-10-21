# The system's last arrived side (F-033, W2l, save v102)

Date: 2026-10-21. This ports the original's tie-break. The reading is in
`ghidra/notes/battle-flow.md`, "The winner and the loser's withdrawal".

## Change

- `System::last_arrival` (save v102) keeps the system's `+0x78` bits
  6..7: the side of the last fleet or squadron to join it (slot `+0xd0`,
  `FUN_00508660` -> `FUN_0050a1b0`). `System::unit_joined` sets it for:
  - a fleet's departure, because the move puts it in its destination at
    once (`FUN_00556390`, `movement::begin_fleet_transit`);
  - a new fleet from a split;
  - a capital ship completed into a new fleet, or a completed squadron;
  - a squadron moved onto a planet.
- `AbstractBattle::finish`: with both retreat words set, the winner is the
  last arrived side: the Alliance for 1, else the Empire
  (`FUN_0040c780`).
- `withdraw_loser`: with no winner and both sides present, the side that
  did not arrive last leaves (`FUN_00516ef0`).
- `port:` seeded systems start with no side.

Two tests changed their expectation for this cause, from no tie-break to
the original's:
- `the_result_words_name_the_winner` (both retreats);
- `without_a_winner_the_last_arrived_side_holds_the_system`, which
  replaces `a_battle_without_a_winner_sends_nobody_away`.

The rule is `FUN_0040c780` / `FUN_00516ef0` (`battle-flow.md`).

## Golden

The seed-42 replay golden is regenerated for this cause: save v102 and
the new field. Only the fingerprints changed: `v1:0174e1ddb8a28d5b` ->
`v1:00f7cdfb6462bcb2`.

## Gates

- Campaigns (24 seeds, 1500 days, dual AI) against HEAD `cbe297e2`: 24 of
  24 identical. No tie came up and telemetry does not record the field.
- Tests:
  - new tests and assertions: the departure, the split, the squadron
    landing, the two completions, the tie in both places, and
    `AbstractBattle::at_system`;
  - with the change neutralized, 5 core and 2 data tests fail;
  - workspace 2542 passed, 0 failed; replay goldens pass. The only
    failure is the long-standing
    `seeds::tests::fixed_fleet_tables_only_target_special_systems`.
- Mutants on the diff: 9 caught, 6 unviable, 1 missed (`mutants.txt`).
  The missed one is `load_game_data_with_options` -> `Ok(Default)`. The
  function is in the diff only for an inserted `last_arrival: None`. The
  data-backed tests outside `--lib` catch it: the replay golden needs the
  loaded galaxy.
- `provenance-scan check`: 0 uncited.
- Browser, Arch Chromium 155.0.8059.84:
  - Fleet window 58 of 58;
  - replay equivalence passed (`v1:0174e1ddb8a28d5b` ->
    `v1:00f7cdfb6462bcb2`, no browser errors, 4 requests);
  - frame timing: very slow median 0.5 ms, p99 0.8 ms; fast median 0.5 ms,
    p99 1.6 ms; 0 frames over 50 ms.
- Release WASM: `af3403b12f49f7dacb123ef880c0384eeda745fd535cc2cd6f8bc433162047ea`.
