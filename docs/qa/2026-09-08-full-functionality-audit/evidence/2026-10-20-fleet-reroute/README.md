# A traveller follows its target fleet (F-040)

Date: 2026-10-20. Stage M5h of the "Misje" area.

## The original

A Move's target fleet is the trip's container. When that container moves,
the traveller is rerouted (`FUN_004fbc20` notifies
`GameObjParentReroutedNotif` for an object en route whose parent changes).

## Port

- `reroute`: when a Move's traveller arrives and its target fleet of the
  mission's side stands at another system, the traveller sets out again
  for that system at the mission's speed.
- The mission's place (`target_system`) follows the fleet, so the
  validator's place rules hold.
- On the arrival where the fleet is, the member boards it.
- port: the port reroutes on arrival, not when the fleet moves.
- No save format change.

## Tests

- `missions.rs`:
  `a_traveller_follows_its_fleet_to_its_new_system_and_boards_there`. It
  replaces `a_move_lands_at_the_system_when_its_fleet_has_left`, whose
  expectation recorded the earlier port simplification, now removed with
  `FUN_004fbc20` as the evidence. It fails without the reroute
  (`fail-without.txt`).
- `cargo test --workspace`: 2487 passed, 0 failed.

## Gates

- The 24 dual-AI campaigns are identical to the stage before (only the
  player's orders target a fleet).
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `e3c54fc406b28de42f9ff897a8018a707d1279bf9185deb798c001c8f3fe6bc1`;
  golden unchanged).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.4 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 10 mutants, 9 caught, 1 unviable.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
