# The Adrift order and its detection mode word (F-040)

Date: 2026-10-20. Stage M9b-1 of the "Misje" area: the core of the Adrift
order, before the battle's path that creates it.

## The original

- The Adrift order is `0x44000004` (`FUN_00547880`, vtable `0x00668d58`,
  TEXTSTRA 11268), with Move's slots.
- Its init (`FUN_00576a20`) writes the mode word `0x4103`, not `0x4112`:
  - phase 2 is mode 3;
  - phase 3 is none;
  - phase 7 is mode 1;
  - phase 9 is mode 4.
- Mode 3 runs mode 1's body on the first pass and mode 2's on the second
  (`FUN_00589a40`). Setup only sets flags, so the second pass keeps the
  first's.
- Decoys and detection act only while detected (`+0x4c`) is clear; the
  detected outcome runs whenever it is set (`FUN_00589970`,
  `ghidra/notes/decoy-roll.md`).

## Port

- `MissionKind::Adrift`, with Move's rules and no phase-10 roll.
- `mission_detection::detection_passes` reads the modes per kind and
  phase.
- `one_pass` runs each pass with the inherited flags and the team's
  detected state. Mode 1 now only sets `at_system`, never clears it.
- For every other kind one pass acts, as before: the 24 campaigns are
  identical.
- port: the second pass's own validator is left to the run's closing
  validator. A member the first pass captured already has its remove
  request, so the second pass neither counts nor exposes it. The mutants
  of that check were equivalent, and the code says so.
- Nothing creates an Adrift order yet: which handler sends a
  battle-destroyed fleet's characters adrift is still to read
  (`mission-lifecycle.md`).

## Tests

- `mission_detection.rs`:
  - `the_adrift_order_s_mode_word_runs_two_passes_in_phase_two`;
  - `an_adrift_team_meets_the_enemy_fleets_as_it_sets_out`;
  - `an_adrift_team_the_first_pass_ends_meets_no_second_pass`.
- `panels/missions.rs`: `the_movement_orders_show_their_own_badges`.
- The first two fail without the Adrift passes (`fail-without.txt`).
- `cargo test --workspace`: 2492 passed, 0 failed.

## Gates

- The 24 dual-AI campaigns are identical to the stage before.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `3090f37a66bf6044865bbdaff82b6f0521767733da4be5bcd8000857d93a5937`;
  golden unchanged).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.9 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-render`): 44
  mutants, 40 caught, 4 unviable.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
