# Four abstract-battle terms from the executable (P24)

Date: 2026-10-27. This is part of the review of the remaining `hyp:` tags.
Four tags in `abstract_battle.rs` are settled by static reading of
REBEXE.EXE, using the disassembly and the decompile corpus. Three of them
change the simulated battle.

## The original

### A fighter group's shield ratio (`FUN_005ed8e0` -> `FUN_005b1710`)

- `FUN_005b1710` returns `slot +0x64() / +0x3e8`, or 0 when `+0x3e8` is 0.
- The base `FUN_005b0f70` sets `+0x3e8` to 50.0, and the fighter
  constructor `FUN_005b9c60` (vtable `0x66c028`) sets it to 1.0.
- The fighter's slot `+0x64` (`0x5b9c30`) computes hull × 0.025 / 8,
  stores it at `+0x3e8` (`fstp [ecx+0x3e8]`), and returns it.
- So the ratio is 1 while the group has hull. The port took it to be the
  hull ratio.

### The travel term's speed (`FUN_005ed170`)

- `+0x54` is the task force's formation:
  - `FUN_005c72b0`, vtable `0x66cb50`, for fighters;
  - `FUN_005ec310`, vtable `0x66da08`, for ships.
- Its slot `+0x34` (`0x5ec0b0` / `0x5ec3c0`) returns 0.0 with no members,
  else `+0x124`.
- `+0x124` starts at 0 (`FUN_005ea030`). Only slot `+0x40`
  (`FUN_005ea740`, the slowest member's power) sets it, and its only caller
  is the tactical manager's turn `FUN_005a9d40`, which the abstract step
  `FUN_005ae9c0` never runs.
- So the speed is 0 and the travel seconds are 0. That 0 is raised to 1.0
  (`DAT_0066da90`) and weighted 0.001: any distance adds exactly 0.001.
- The port divided the distance by the leader's engine power.

### The trench run's commander (`FUN_0040b0e0` -> `FUN_0040c200`)

- `FUN_0040b0e0` stores the Commanders `FUN_00509530(system, 1/2)` at the
  battle's `+0x1c`/`+0x24`.
- It gives a battle person only to the characters aboard a capital ship of
  a fleet at the system:
  - the fleets come from `FUN_004ffe70(system, 3)`;
  - the ships from `FUN_00502db0`;
  - the characters from `FUN_004f25c0(ship, 1)`;
  - and `FUN_0040c200` is called for each, at `0x40b635`.
- A system Commander on the planet that no fleet's Commander beats has no
  person, so the term is 1.0. The port counted it.

### The trench run's group (`0x5d1550`)

- The kind 4 that `FUN_005d14d0` looks for is the task force's own class
  (slot `+4`).
- A fighter task force (vtable `0x66bee0`) returns 4 (`FUN_005a6700`); a
  capital one returns 3 (`FUN_005a6730`).
- The port's choice was right; only its comment changes.

## The port

- `target_priority`: a fighter group's shield ratio is 1 while it has
  hull.
- `score`: the travel term is the bare 0.001 for any distance.
  `engine_speed` is gone.
- `trench_commander_term`: only a Commander picked from a fleet at the
  system counts.
- `death_star_step`: the comment is settled.

## Tests

These encode the traced terms where they had asserted the old ones:
- `a_distant_target_adds_the_least_travel_term` (renamed);
- `the_travel_term_measures_from_the_leader_on_every_axis`;
- `the_priority_is_fifteen_times_strength_less_the_ratios`.

`a_system_commander_off_the_fleets_has_no_battle_person` is new. Without
the change it gives 9.0.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 3001 passed, 0 failed |
| `replay_manifest -- --ignored` (original data) | 2 passed; the golden is unchanged |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | first run 20 tested: 19 caught, 1 missed. The priority test then gained a squadron with no hull, and `target_priority` reran at 19 tested, 19 caught |
| Release WASM SHA-256 | `c5dcb8a4f177d44bc7388a666f37596fbd2b73babfc7586e6b9876f20249c885` |
