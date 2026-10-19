# Character command: the detector's officer (F-038, stage 4b)

Date: 2026-10-19.

`ghidra/notes/character-command.md`, `ghidra/notes/decoy-roll.md`. This is
stage 4b of the tracker item "Dowodzenie i jego utrata przy ranie".

## The original

- A detector's slot `+0x1bc` gives the rank of the officer that backs it:
  - 2 Admiral for a capital ship (`0x0043c1f0`);
  - 1 Commander for a fighter squadron (`FUN_0040f340`);
  - 3 General for a regiment (`FUN_00526f00`).
- The officer is found in the detector's fleet (`FUN_004fd790`) or, for a
  detector the walk hands without a fleet, on the system (`FUN_00509330`
  on the pool's system).
- The decoy roll (`FUN_00588b90` -> `FUN_0055e410`):
  - x = decoy espionage - detection - officer espionage * G3588 / 100;
  - the officer's espionage is 0 when there is no officer;
  - the roll runs either way.
- FOILTB (`FUN_00588a90` -> `FUN_0055e470`) does the same with G3589.
- Both G3588 and G3589 ship as 35. `FUN_0053e190` -> `FUN_0053e150`
  truncates.
- Neither function is in the decompile corpus. The push orders were read
  from the image (`0x00588c3c..0x00588c4e`, `0x00588b3a..0x00588b46`).
- Exposure (`FUN_005888f0` -> `FUN_005349e0`): RLEVADTB at member combat
  less the officer's effective combat (slot `+0x1f0`). The captor is the
  officer; without one it is the defender, at 0.

## Port

`mission_detection.rs`:
- `Defender::rank`;
- `Defender::officer`, through `character_command::fleet_holder` and
  `system_holder`;
- the officer terms in `decoy_walk` and `detect`;
- `expose` reads the officer's combat and side.

## Tests

- `a_ship_is_backed_by_its_fleet_s_admiral_a_squadron_by_a_commander_and_a_regiment_by_a_general`
- `a_regiment_s_general_takes_part_of_its_espionage_off_the_decoy_and_foiltb_rolls`
- `an_exposed_member_rolls_against_and_is_captured_by_the_detector_s_officer`
- All three fail when `Defender::officer` finds none (`fail-without.txt`).

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 4a. The AI gives no
command yet (module `0xc`).

## Gates

- `cargo test --workspace`: 2468 passed, 0 failed.
- The seed-42 golden is unchanged (`v1:094e77fcd3b4b666` ->
  `v1:beb63be8a01b2699`).
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `ef015f6ac0a65d79ec75c5b5b7dc8a09627b6813774c023401b2cb95dbaf472d`).
- The Fleet window gate passes all 48 cases (interface WASM
  `c33d97879e65dc4697caca7ded244870ae085646e7cc0039ed4c6409182c9202`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.7 ms | 23.8 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`): 29 mutants, 28 caught,
  1 unviable.
- `provenance-scan check`: 85 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
