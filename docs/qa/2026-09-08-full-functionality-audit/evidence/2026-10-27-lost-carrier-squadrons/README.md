# A destroyed ship takes its docked squadrons with it (P24, squadrons)

Date: 2026-10-27. This follows `../2026-10-26-squadron-carriers/`, which
gave each squadron the ship it rides on (save v106). That stage left one
port rule as `hyp:`: when a carrier was lost, its squadrons boarded
another ship of the fleet with room (`squadrons::seat_squadrons`).

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository). The full trace is in
`ghidra/notes/planet-squadrons.md`, "A destroyed object's children".

## The original

### The cascade

- `GameObj_SetDestroyedReason` (`FUN_004f71d0`) stores the reason. It then
  calls slot `+0x100`, which for ships and squadrons runs
  `0x004f2ad0` -> `FUN_004fb200` -> `FUN_004f7560`.
- `FUN_004f7560` sets `Flags50` bit `0x8` to (reason != 0) and calls slot
  `+0x11c`.
- A capital ship's `+0x11c` (`CapShip_vt11c`, `0x00502840`) first runs
  the base `GameObj_vt11c` (`FUN_004fb520`). That walks every child and
  calls the child's `+0xb4` (`FUN_004f8710`):
  - a child that is Completed and not EnrouteActive gets `FUN_004f7ed0`,
    which hands it the parent's reason through its own `+0xac`, so it is
    destroyed too;
  - any other child that is still alive turns on autorouting.

### Who reaches it

- **The bombardment's ship losses.** `FUN_00501ae0` calls `+0xac`.
- **A sabotaged ship or Death Star.** `FUN_005746e0` calls `+0xac(6)`.

Not traced to `+0xac`:
- the scrap order;
- the battle's commit. `FUN_0040c780` reparents each squadron before the
  ship losses (`FUN_0040d320`): to its carrier when it is in the hangar or
  docked, and otherwise to the system.

## The port

- `crates/rebellion-core/src/scrap.rs`: `destroy_ship` drops the fleet's
  squadron entries whose carrier is the ship, then removes the ship as
  `remove_ship` does. A ship with no number (id 0) takes none.
- `crates/rebellion-core/src/bombardment.rs` (`apply`) and
  `crates/rebellion-core/src/missions.rs` (the sabotage of a capital ship
  or a Death Star) use it.
- The scrap and the battle results keep `remove_ship`. Their squadrons
  still board another ship. That rule stays `hyp:` in
  `crates/rebellion-core/src/squadrons.rs`.

## Tests

- `a_destroyed_ships_docked_squadrons_are_destroyed_with_it`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2959 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed; the seed-42 golden is unchanged |
| `cargo mutants` | 5 tested: 5 caught |
| Release WASM SHA-256 | `7b5efadf8936c975f16d62cb8c4ac16f9ae414581ad96c8d4837fa2061d3393f` |
