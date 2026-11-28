# The AI and the Standard victory targets (F-007)

Date: 2026-11-28, commit `5fdb9714`. The audit asks that the AI reach all
Standard victory targets, and that the Death Star's part be checked in a
permitted fixture.

## Each target is reached by the AI

`rebellion-playtest data/base --dual-ai --ticks 5000 --seed N`, seeds 1 to
25. `victory-bits-25-seeds.json` lists each seed's changes of the
victory conditions word (`FUN_005180e0`, `crates/rebellion-core/src/victory.rs`)
and the bits seen at least once.

| Target (word bit) | Side | Seeds where the AI reached it |
|---|---|---|
| Alliance headquarters gone (`0x1`) | Empire | 9 |
| Luke Skywalker captive (`0x10`) | Empire | 21 |
| Mon Mothma captive (`0x20`) | Empire | 11 |
| Coruscant the Alliance's (`0x100`) | Alliance | 4 (8, 15, 19, 20) |
| Darth Vader captive (`0x1000`) | Alliance | 2 (15, 21) |
| The Emperor captive (`0x2000`) | Alliance | 5 (8, 15, 19, 20, 21) |
| Empire wins (`0x100000`) | Empire | 6 |
| Alliance wins (`0x1000000`) | Alliance | 0 |

The Empire won 6 of 25 games with the full word `0x10100031`. The
Alliance AI reached each of its three targets but never all three at
once. Its closest game was seed 15: it held Coruscant from day 2420,
captured Vader on day 4006, lost him to an escape on day 4125
(`character_escaped`), and then captured the Emperor on day 4620.

## The Death Star

**No Death Star is built in the 25 runs.** Module `0xb` takes the Death
Star role (`FUN_004ca5e0`: for the Empire with two shipyard sites or more,
no Death Star in its fleets, and no request of the sector planners). A
temporary trace (not committed, seed 2, 3,000 days) showed that the role
comes up, but no class fits the budget. The Death Star (CAPSHPSD record
136, maintenance 600) is on the shipyards' list, while the ship budget
(`FUN_004cc220`) ran from 32 to 241. For example, capacity 6,600 gives
30% × 90% × 6,600 = 1,782, less the capital ships' upkeep of 1,541.

- The budget formula follows the decompile of `FUN_004caad0` and
  `FUN_004cc220`.
- The percent `+0x24` is the constant 30 (`FUN_004c8820` sets
  `param_1[9] = 0x1e`; no other writer was found).
- The upkeep sum follows `FUN_0041ba80` (fleet record `+0x38` = the
  ships' `values[11]`).

The input still in question is the side's maintenance capacity (`+0x58`).
Its model already disagrees with the original's opening values (see
`ghidra/notes/side-resource-monitors.md`, "Day-0 maintenance survey" and
"Parameter scan").

Before `cc3f1ffb` (2026-11-27, the planners' role requests), the same 25
seeds built 4 Death Stars. Module `0xb` then often took no role and drew
among all classes. That is the state the earlier Death Star wins in the
notes come from.

**A Death Star given by a fixture is scrapped.** A temporary test (not
committed) gave the Empire's largest fleet a Death Star on day 0, on five
full 200-system campaigns. Each time:

1. The Death Star split off on day 2 and went home to Coruscant (module
   `0xb`, `FUN_004c94c0`).
2. Within 12 to 38 days it was scrapped. The fixture's 600 upkeep pushed
   the ship budget below 0 (`FUN_004caad0`).
3. `FUN_004cab20` scraps a ship of a free fleet (`5` forbidding `0x402`),
   and the Death Star's fleet was the free one.

The trace also found that the port scrapped any living ship of that
fleet, where `FUN_0041ac80` admits only own ships no planner holds and
not travelling or inactive (`0x24000` forbidding `0x2800`). That is fixed
in `5fdb9714`. The Death Star alone at Coruscant still passes that mask,
so the fix does not save it.

A fixture in which the Empire can afford a Death Star (its upkeep within
the budget) is still to be written. Without one, the Death Star's part
(the planners' Destroy System `0x223`, `FUN_004b1330`) is covered only by
the unit test in `ai_plan.rs`.

## Conclusion

The AI reaches every Standard target. The Empire wins 6 of 25 games. The
Alliance has no win; seed 15 came closest. The Death Star part stays
open, for two reasons:

- the side's maintenance capacity, which decides whether the Empire can
  afford a Death Star, is the open economy question above;
- no affordable-Death-Star fixture exists yet.
