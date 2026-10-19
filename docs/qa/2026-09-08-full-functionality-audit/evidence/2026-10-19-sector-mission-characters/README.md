# Sector planners' Abduction and Assassination targets, stage 3 (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "The Abduction and Assassination
targets".

## The original

- For Abduction and Assassination (`0x17`, `0x81`), state 2
  (`FUN_004bdbc0` -> `FUN_00476e80`) asks the plan object's slot
  `+0x24`.
- The slot tries steps until one gives a system of the plan's sector:
  - kinds 1 and 2 (`FUN_004ba630`, `FUN_004b94b0`) take a system with an
    opponent's character on the planet (`+0x2c` `0x400`), then one
    aboard a fleet (`0x800`), by `FUN_0041a340` on `v[29]`, order 2;
  - kinds 3, 5 and 6 (`FUN_004b3880`, `FUN_004b1880`, `FUN_004b4e40`)
    first want an officer there (`+0x30` `0x1000`; kind 3 draws by
    `FUN_0041a460`), with the fleet step taking only officers (mask
    `0x7000000`), then kind 1's steps;
  - kind 4 (`FUN_004b7370`) first takes its own target `+0x5c` when
    that record has `+0x2c & 0xc00`, then kind 5's steps, its first with
    mask `0x7000000`.
- `FUN_004af1c0` then draws a character there: record `+0x30`
  `0x20000000` and 4, a bit of the mask, and no `0x20`. It looks on the
  planet with bits `0x400`, and aboard fleets not the side's with fleet
  record `+0x38 & 0x600000` when the system record has `0x800`.

## Port

- `sector_character_target` and `opponent_at`, called by state 2 in
  `AISystem::mission_planner_cycle`. The planner keeps the system and
  the character.
- port: `adopt_mission_orders` passes over a sector planner's planner.
  Without that, a heuristic Assassination aimed at the same target was
  taken as already running and never sent, because these planners do
  not order yet.
- port: the Sabotage slot is still not ported, and states 3 to 8 still
  are not, so the held planners order nothing.
- No save format change. The seed-42 golden is unchanged.

## Tests

- `ai.rs`:
  - `an_opponent_at_a_system_is_any_character_of_the_other_side_where_the_bits_look`;
  - `each_plan_kind_looks_for_an_opponent_in_its_own_order`;
  - `a_sector_planner_s_assassination_planner_takes_a_system_and_its_character`;
  - `each_order_is_its_own_planner_unless_one_runs_for_it_and_held_members_stay_put`
    gains the owner case.
- `fail-without.txt`:
  - with the slot and `FUN_004af1c0` stubbed out, the three new tests
    fail;
  - without the adoption change, the fourth fails.

## Campaigns (`playtest-sector-characters.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the stage-2 commit.

- All 24 diverge.
- In a first run without the adoption change, the stored targets
  mattered: dropping them changed seeds 1, 2, 14 and 23. The cause was
  the swallowed heuristic orders.
- With the change, the picks only shift the draws. With the targets
  dropped, those four seeds run event for event as with them kept.

| | Before | After |
|---|---|---|
| Missions dispatched | 13,470 | 12,764 |
| Two-member teams | 4,471 | 3,659 |
| Successes | 18,609 | 18,404 |
| Failures | 7,392 | 7,195 |
| Abductions / Assassinations dispatched | 569 / 477 | 602 / 423 |
| Systems at day 1500, summed (Alliance / Empire) | 567 / 569 | 584 / 567 |

## Gates

- `cargo test --workspace`: 2448 passed, 0 failed.
- `cargo test -p rebellion-data --test replay_manifest --
  --include-ignored`: 2 passed. The golden is unchanged:
  - initial `v1:20ee82a2951ad9aa`;
  - final `v1:aae0de7bbef402c1`.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `8a4742096e5657cef777568bf597ae5eda949c855fdf9cf256690987ff2c95e1`).
- **The Fleet window gate is unstable.** See `fleet-window-runs.txt`.
  - On this change it passed all 48 cases in 1 of 4 runs (interface WASM
    `5b8187d374339ce39714ba783288cacc441460aa02067bbb42f9d8617483ffb2`).
  - The other three each failed one case: `join-fleet` twice and
    `travel` once.
  - The stage-2 commit (`077a997d`), rebuilt unchanged, also failed
    `join-fleet` in 1 of 1 runs.
  - `join-fleet`: the joined fleet's entry is listed expanded (4 rows
    for 1 fleet of 3 ships).
  - `travel`: another regiment's arrival message (Svivren) came last
    alongside the player's (F'tral).
  - The cause is not found. It is reported as an open problem and not
    fixed here.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.4 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 5.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`):
  - first run: 52 mutants, 9 missed;
  - second run: 6 missed;
  - one more was closed by a test, and the last 5 are equivalent:
    - `record.summary & 0x800` with `|` or `^`: whenever a fleet not the
      side's carries an opponent (`+0x38 & 0x600000`), the record has
      `+0x2c` `0x800` (`FUN_00433780`);
    - `&&` to `||` in `carrying`: the side's own fleet with an
      opponent's prisoner aboard has no `0x600000`, because a prisoner
      gives its ship `0x200000`;
    - `& 0x600000` with `|` or `^`: a fleet not the side's that carries
      no opponent has no character `opponent_at` takes.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
