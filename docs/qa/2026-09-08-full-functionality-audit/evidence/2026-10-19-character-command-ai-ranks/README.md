# Character command: the AI records' rank bits (F-038, stage 4c)

Date: 2026-10-19.

`ghidra/notes/character-command.md`, `ghidra/notes/ai-mission-planning.md`.
This is stage 4c of the tracker item "Dowodzenie i jego utrata przy
ranie". It also settles the officer-rank hyp in stage 5 of AI module 4.

## The original

`FUN_00402230` sets the record flags from `+0x96`:
- 1 (Commander) sets `0x4000000`;
- 2 (Admiral) sets `0x1000000`;
- 3 (General) sets `0x2000000`.

Most mission queries forbid `0x7000000` (`ai-mission-planning.md`, "Team
selectors"), which keeps a character holding a command off missions.

## Port

`mission_planning.rs`: `OFFICER_RANKS` by `Character::command`. It
replaces the `hyp:` that set rank 1 for every character in a fleet.

## Tests

- `a_character_s_command_sets_its_rank_bit_and_keeps_it_off_a_diplomacy_team`.
  It fails with the old hyp (`fail-without.txt`).
- `ai.rs` `opponents_world`: Leia, aboard the fleet, now holds the Admiral's
  command. Three tests had read her rank from the hyp:
  - `an_opponent_at_a_system_is_any_character_of_the_other_side_where_the_bits_look`;
  - `each_plan_kind_looks_for_an_opponent_in_its_own_order`;
  - `a_sector_planner_s_assassination_planner_takes_a_system_and_its_character`.
- Their assertions are unchanged. Only the fixture gives her the rank the
  original reads (`FUN_00402230`), and the comment no longer cites the hyp.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 4b. In these
campaigns the hyp's bit changed no pick.

## Gates

- `cargo test --workspace`: 2469 passed, 0 failed.
- The seed-42 golden is unchanged (`v1:094e77fcd3b4b666` ->
  `v1:beb63be8a01b2699`).
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `01907d8a121ec85bf2ea8c6b792a178c71241dce4655f095f1836a097ec031c7`).
- The Fleet window gate passes all 48 cases (interface WASM
  `23eb0dc51cc30b18aa643d4d8860d48eb98245c27e6acf262778d70bbf3b26e0`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 24.4 ms |
| Fast | 3601 | 0.6 ms | 1.7 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`): 3 mutants, 2 caught,
  1 unviable.
- `provenance-scan check`: 85 uncited items, unchanged.

## Found for the next stage

`FUN_00560d50`, run by `FUN_00514a60` when an object moves, takes the
command away:
- a moved capital ship clears every command aboard;
- a moved character loses its own, except between two capital ships of the
  same fleet.

This is stage 5 of the item.

Hashes of the committed files are in `SHA256SUMS`.
