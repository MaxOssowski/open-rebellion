# Character command: losing it on a move (F-038, stage 5)

Date: 2026-10-19.

`ghidra/notes/character-command.md`. This is stage 5 of the tracker item
"Dowodzenie i jego utrata przy ranie".

## The original

`FUN_00560d50(object, from, to)` is run by the move event `FUN_00514a60`
(from `FUN_0057ba50`):
- a capital ship (`0x14..0x1b`) that moves clears the command of every
  character aboard (`FUN_004f25c0(ship, 3)`, `FUN_004ee350(0)`);
- a character holding a command that moves loses it, except from one
  capital ship to another of the same fleet.

## Port

- `Character::move_to(fleet, system)` drops the command when the
  character's fleet or system changes.
- The real moves go through it:
  - `missions::move_member`: missions, and the AI's `MoveCharacters`;
  - the abode move;
  - `fleet_join::merge_fleet_into`, where each moved ship takes the
    commands aboard;
  - the fleet panel's assign and remove (`main.rs`);
  - the story's `TransferCharacter` (`integrator.rs`).
- port: the port keeps characters on the fleet, not on a ship. So a split
  moves no one, and a move between two ships of the same fleet cannot
  happen.
- A capture still clears the command through `release_unfit`.

## Correction to the notes

`FUN_004eeed0` was read again from the image (`0x004eeed0..0x004eef64`).
The character keeps its command only when it is not on a mission (its
container `+0x68` is outside families `0x50..0x7f`). The first reading,
in both `character-command.md` and `character-injury.md`, had this test
the other way round and called `+0x68` the commanding key. The commanding
key is `+0xa8` (`FUN_004ee740`).

The port's `keeps` does not make the mission test yet. That is stage 6,
together with the two other hooks.

## Tests

- `character_command.rs`: `a_character_that_moves_loses_its_command`.
- `missions.rs`:
  `a_moved_member_leaves_its_fleet_or_system_and_lands_at_the_target` now
  also checks the admiral's command.
- `fleet_join.rs`:
  `a_fleet_moved_onto_another_in_its_system_hands_over_everything_and_disbands`
  now also checks the admiral's command.
- All three fail without the drop in `move_to` (`fail-without.txt`).

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 4c.

## Gates

- `cargo test --workspace`: 2470 passed, 0 failed.
- The seed-42 golden is unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `3426e21e3bb58b016e5ac8a0da164baa7e857a940a34e918ae404b3489c16a4b`).
- The Fleet window gate passes all 48 cases (interface WASM
  `d6af1b9608b7d7ab17c20845bdb0c63a6c24bf7824283d8ad2575faf75da55f0`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.9 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.9 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-data`):
  6 mutants, 6 caught.
- `provenance-scan check`: 85 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
