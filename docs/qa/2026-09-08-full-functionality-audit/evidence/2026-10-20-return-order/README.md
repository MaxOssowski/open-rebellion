# The Return order: members leave an ended mission (F-040, save v91)

Date: 2026-10-20. The user granted all permissions for this run, save
format changes included.

This is stage M5b of the "Misje" area (tracker item "Powrót, automatyczna
trasa i dryf postaci"). The reading is in `ghidra/notes/mission-lifecycle.md`,
"Leaving a mission".

## The original

- After every phase change, `FUN_00548120` asks `FUN_00521220` how each
  member of an ended mission leaves. The answer depends on MISSNSD column
  10 (record `+0x68`, `FUN_00520ba0`):
  - 0 for the movement orders: the member stays;
  - 1 for agent missions: it returns from a container of another side;
  - 2 for Dagobah, Palace, Vacation, Pickup and Bounty: it always returns.
- `FUN_00548910` takes no member with a remove request (`+0x78` bit 2).
  It groups the returning members by the first one's destination
  (`FUN_00542ee0`): the container, then the character's `+0x70`, then the
  autoroute search `FUN_00543040`.
- Each group goes on a Return order: `0x42000002` (`FUN_005475f0`), vtable
  `0x00664388`, the same slots as Move.

## Port

- `MissionRecord::leave_rule` holds column 10 (save v91), and `dat-dumper`
  names the field.
- `MissionKind::Return` (record `0x42000002`): no phase-10 roll and
  own-side targets, as Move.
- `MissionSystem::finish` takes a rule-1 mission's team and decoy members
  standing in a container of another side and groups them by destination
  (`return_destination`, `nearest_place`). Each group gets a Return order
  that steps at once. Its members stay on a mission
  (`MissionEffect::MemberOnMission`, hidden as the record says); the others
  are released.
- Telemetry: `return_ordered` (`record_return_orders`).
- port:
  - the container and `+0x70` candidates are skipped, as for an escape;
  - a member with nowhere to go is released where it is, not sent to its
    jail (`FUN_00549580`);
  - rule 2 keeps the port's abode trips;
  - a prisoner on the captured list stays;
  - the Mandatory flag is not kept.

## Save v91

- `SAVE_VERSION` 90 -> 91.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:0d8727a7d2a2ee81` -> `v1:68bc58d8f3fb696d`;
  - final `v1:3f8a4e8cc1d73a06` -> `v1:5fc59cefe817d1b4`;
  - two recordings are identical, and only the fingerprints changed.

## Tests

- `missions.rs`:
  - `an_agent_returns_home_from_another_side_s_system_on_a_return_order`;
  - `the_return_order_ends_on_arrival_and_frees_its_member`;
  - `members_sharing_a_destination_share_one_return_order`;
  - `an_agent_on_its_own_side_s_system_or_under_rule_zero_stays_where_it_is`;
  - `a_return_order_s_members_are_put_on_its_hidden_mission`.
- `integrator.rs`:
  `a_return_order_of_the_day_is_recorded_with_its_team_and_destination`.
- The loader test reads column 10.
- The first two fail without the Return (`fail-without.txt`).
- `cargo test --workspace`: 2475 passed, 0 failed.

## Campaigns (named cause)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- every campaign changes;
- 11,979 Return orders (Empire 6,831, Alliance 5,148);
- the AI's own Move orders fall from 44,841 to 32,470, since the Return now
  brings the agents home;
- mission results are about the same (22,568 successes against 22,667);
- `campaigns.txt` has the totals.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `910bbd8a413f65e8ab5726fba31772de59aece06e15ac989f55a220db2c25721`).
- The Fleet window gate passes all 48 cases (interface WASM
  `d171359b9495122c209a918061e02c80dd07231fca9cf823fea6e1bf09e3dc3f`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 24.0 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - the first run: 57 mutants, 39 caught, 9 unviable, 9 missed;
  - after three tests were added and a redundant second step of the new
    orders was removed: 54 mutants, 45 caught, 9 unviable;
  - the campaigns are identical before and after that removal.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
