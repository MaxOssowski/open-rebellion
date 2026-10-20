# A Death Star's destroyed system keeps its fleets; its characters go adrift (F-032)

Date: 2026-10-20. Stage DS1 of the "Gwiazda Śmierci" area, on the user's
go-ahead.

## The original

- The Destroy System order (`0x223`) destroys the system in
  `FUN_0058d990` -> `FUN_0058ce70`: slot `+0xac` with reason `0x10` (the
  attacker's own system) or `0xf`.
- The system's `+0xac` is `FUN_005080d0`. It writes the reason
  (`FUN_004f71d0`), sets the destroyed bit (`FUN_004f7560`) and calls the
  destroyed hook `FUN_00510950`. That runs the base `FUN_004fb520`, which
  calls slot `+0xb4(1)` on each existing direct child, then raises event
  `0x319`, which only refreshes each side's view (`FUN_0050d5a0`).
- What slot `+0xb4` does to each child:
  - ships, squadrons, regiments and facilities (`FUN_004f8710`) and
    special forces (`FUN_00503e20`): one completed and not en route is
    destroyed with its container's reason (`FUN_004f7ed0`);
  - a fleet (`FUN_00524fb0`) returns 1: it stays with its ships and the
    characters aboard, and one en route there keeps its trip;
  - a character on no mission (`FUN_004ef2e0`) goes adrift.
- After the destruction, `FUN_0058d990` walks the minor characters
  standing in the system (`FUN_0056f3d0`, families `0x38..0x3b`). Each
  one not en route is hit with GNPRTB `0x606` (50, `FUN_0053e2f0`). A hit
  injures it (slot `+0x2f0`, reason 10); if it still exists afterwards
  (disassembly `0x58daeb`), it is destroyed with reason 10. So every hit
  kills.
- The manual (p. 96) agrees: a character "may be injured ... if the system
  on which he is located is blown up by a Death Star", and an injured
  character may be killed.
- The notes are in `bombardment-order.md` ("The destroyed system's
  contents").

## What the port did before

`cleanup_destroyed_system` had no source for three things:
- it removed every fleet at the system;
- it killed every character aboard those fleets;
- it cancelled every fleet trip to the system. A cancelled fleet was in
  no system's list, since its departure had removed it from its origin's
  list.

It left the characters standing in the system untouched.

## Port

- `death_star::cleanup_destroyed_system`:
  - the fleets stay;
  - characters standing in the system on no mission go adrift
    (`Character::adrift`, save v93; `MissionSystem::send_adrift` sends
    them on);
  - the `killed` characters die;
  - units and facilities go as before.
- `bombardment::destruction_hits` draws the hits in phase 1
  (`BombardmentOutcome::killed`, `BombardEnv::killed_at`).
- `MovementState::cancel_orders_to` is removed.
- Both the simulation and the app (Destroy System, and the old Fire Death
  Star panel, which kills no one) pass the dead to the cleanup.
- port:
  - a captive is not set adrift;
  - the characters are taken in the arena's order;
  - the minor characters' injury death roll, which cannot change the
    result, draws nothing.

## Tests

- `death_star.rs`:
  `a_destroyed_systems_fleets_stay_and_the_characters_standing_there_go_adrift`.
- `bombardment.rs`:
  `a_death_stars_destruction_kills_the_minor_characters_it_hits`.
- Both fail on the old rule (`fail-without.txt`).
- `cargo test --workspace`: 2497 passed, 0 failed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all 24
are identical to the stage before. The AI orders bombardments `0x220`
(1194) and `0x222` (87), never Destroy System, so the change shows only in
the tests.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `30f9cab2d1917374fd2a6d3a5b3aaaac04f43a3fe8230830410b56bba88160d0`);
  the seed-42 fingerprints are unchanged (`v1:57cfc0d727199c15` ->
  `v1:4dee6ff02fdbe5bc`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.7 ms | 23.7 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 5.2 ms |

- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors; 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 25 mutants, 25 caught.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
