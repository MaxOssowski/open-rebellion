# Death by injury for minor characters (F-038)

Date: 2026-10-18.

The assassination stage (`2026-10-18-assassination`) ported slot
`+0x2e0`'s death roll for the assassination's injury only. This stage
applies it to every other injury that passes slot `+0x2f0`
(`ghidra/notes/character-injury.md`).

## The original

- Slot `+0x2f0` (`FUN_004ef6d0`) adds an injury, then asks slot `+0x2e0`
  whether it kills:
  - a minor character (vtable `0x00665be0`, `FUN_0056f160` ->
    `FUN_0053eb30`) dies with a chance of GNPRTB 2577 (25);
  - a major one (`FUN_006158b0`) never dies.
- The injuries that pass it:
  - the detection's evasion and capture injury (slot `+0x2e4`,
    `FUN_004ef5f0`, reason 10);
  - the Force duel's injuries (`FUN_0053ea70`, `encounter.md`);
  - the uprising incident's code 3 (slot `+0x2e4`,
    `uprising-incident.md`);
  - the assassination (previous stage).

## Port

- `mission_detection.rs`: after the evasion or capture injury and after
  each duel injury, `dies_of_injury` (a major character draws nothing)
  adds `MissionEffect::DiedOfInjury`.
- `uprising.rs`: code 3 adds `IncidentLoss::CharacterKilled`. The
  incident's roll budget rises from 11 to 13: two outcome codes, each
  with one more draw.
- `missions::kill_character` (off every fleet roster, out of its abode,
  `mark_killed`) applies both. The integrator and the app route
  `DiedOfInjury` through `apply_advance_effects`.
- Telemetry:
  - `character_killed` with cause `injury`;
  - the uprising incident's `killed` list.
- Luke's encounter wound, the Final Battle and the Bounty hit injure only
  major characters, which never die, so they draw nothing.
- No save format change. The seed-42 golden is unchanged: no minor
  character is injured in its 25 days.

## Tests

- `mission_detection.rs`:
  `an_injured_minor_member_dies_on_the_death_roll_and_a_major_one_never`.
- `uprising.rs`:
  `code_three_s_injury_kills_a_minor_character_on_the_death_roll`,
  applied to the world.
- `missions.rs`: `a_character_dead_of_its_injury_leaves_its_fleet_and_abode`.
- Two expectations changed:
  - `an_evading_member_s_injury_draws_after_the_detection_and_evasion_rolls`
    now expects the death roll after the injury, since its member is a
    minor character;
  - the uprising roll-budget test expects 13 for an incident.
- Shown failing without the change (`fail-without.txt`): with the three
  death rolls and the kill reverted, the 3 new tests fail.

## Campaigns (`playtest-death-by-injury.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days.

- 26 minor characters die of injuries: Jerjerrod, Bin Essada and Pter
  Thanas 5 times each, Pellaeon 3, Drayson and Piett 2, Vanden Willard,
  Bevel Lemelisk, Carlist Rieekan and Jan Dodonna once. No major
  character dies.
- The uprising incident kills no one in these runs.
- The shifted draws change seed 16's palace story:
  - the hunters capture Han on day 399, and he is in the palace on day
    400;
  - Chewbacca's rescue fails on day 409, and Jabba keeps him;
  - Luke's rescue succeeds on day 603 and frees both.

  This is the first campaign with a failed rescue.
- Seed 19 is unchanged: Luke frees Han on day 584.

## Gates

- `cargo test --workspace`: 2398 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:5474ae69134d1255` -> `v1:4cd75bad1e5062b8`) and 0 browser errors.
  Release WASM SHA-256
  `8f9223103f4ed6489e08faacb532199e29d1639dc83502a4a5ec7551ca889b43`.
- The Fleet window gate passes all 48 cases (interface WASM
  `76202b24c22017b8b3f7cd15f397f90fc43fe923db0d51e69213a0695fb8a46c`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.8 ms | 25.9 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 4.1 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`missions.rs`, `mission_detection.rs`,
  `uprising.rs`): 11 mutants, 11 caught.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt on the changed lines: clean, apart from a
  test's `assert!(… .is_empty())`.
- Native GUI acceptance was not run for this change.

Hashes of the committed files are in `SHA256SUMS`.
