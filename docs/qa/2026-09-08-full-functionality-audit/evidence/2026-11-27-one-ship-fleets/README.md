# One-ship AI fleets come from traced planner orders (F-007, roadmap M1)

Roadmap M1 asked to "balance one-ship fleets in the AI": after the arrival
merge was retired (2026-10-05), seed 42's fleet count grew. This note traces
where the one-ship fleets come from at commit `8c1a94b8`.

## Run

`rebellion-playtest data/base --seed 42 --ticks 1500 --dual-ai`, with
temporary prints (not committed) in `emit_campaign_snapshot`, in
`AIState`'s module step, and at the four Create Fleet sites of
`ai_planner.rs`.

Fleets at the six snapshots: 10, 12, 10, 16, 25, 19 (5, 6, 11, 11, 17, 17
on 2026-10-05; most of the game has changed since). On day 1,250 the Empire
holds 12 one-ship fleets, 10 of them at a system with another Empire fleet;
the Alliance holds 4.

## Who creates them

Create Fleet (`0x270`) orders in 1,500 days, by module:

| Module | One ship | Two | Three |
|---|---|---|---|
| 8, Empire | 136 | 1 | 2 |
| 8, Alliance | 15 | 3 | 1 |
| 9 | 2 | 0 | 0 |

Module 8's orders, by the sector planner function that gave them:

| Function | Orders | What it does |
|---|---|---|
| `FUN_004782a0` (`acquire`) | 78 | takes free ships of the nearest own fleets for a short count |
| `FUN_00478aa0` (`giveback`) | 49 | gives one spare ship back, one at a time |
| `FUN_00478690` (`steal`) | 27 | takes held ships of a later plan's fleets |
| `FUN_00477a40` (`split_damaged`) | 4 | splits the damaged ships off a held fleet |

Each order is Create Fleet to `FUN_00479220`'s system
(`ghidra/notes/ai-sector-plans.md`, "The planner's step"). The ships a
planner takes arrive as separate fleets; the plan object's state `0xc`
(`FUN_004b0c90`, `ai_plan.rs` `regroup`) joins a held fleet to the nearest
other held fleet whose system has no enemy fleet, for every plan kind. A
ship given back stays a fleet of its own until a planner takes it; since
`720e36d8` module `0xb`'s `FUN_004c94c0` moves such idle fleets to a base.

## Conclusion

Every one-ship fleet in the run comes from a traced function, and the
original's only join of planner fleets (`FUN_004b0c90`) is ported. Joining
lone ships by another condition would be a rule the original does not
have, so the port does not add one. The roadmap item is closed as
following the original; the fleet-count threshold stays with the
five-seed campaign item.
