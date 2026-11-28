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

## An affordable Death Star (fixture)

A temporary test (not committed; `death-star-fixture.json` holds the
results) loads the full 200-system campaign of seeds 1 to 5 and gives the
Empire 100 more mine and refinery pairs, spread over its own systems. The
capacity is 50 per pair (the smaller of the mines' and refineries' sums,
`FUN_005323c0`), so this adds 5,000, and the ship budget rises by about
1,350. Both sides are run by the AI for 3,000 days.

**With a Death Star given on day 0, the planners use it.** It is never
scrapped in the five runs. The sector planners bind it to the bombard
role and give Destroy System `0x223` (`FUN_004b1330`); the Death Star
fires (`death_star_fired`) and the system is destroyed:

| Seed | Death Star | Destroy System orders | Systems destroyed |
|---|---|---|---|
| 1 | lost on day 2301 | 0 | none |
| 2 | stands | 2 | Commenor, Corellia |
| 3 | stands | 0 | none |
| 4 | stands | 1 | Caprionril |
| 5 | stands | 2 | Storthus, Corellia |

Each order destroyed its system. The same runs give 184 to 1,549 ordinary
bombardments (`0x220`) and some `0x222` orders.

**With the capacity alone, the Empire still builds no Death Star in 3,000
days.** A temporary trace (not committed) followed module `0xb`:

- `FUN_004ca5e0` takes the Death Star role only as a fallback, when no
  sector planner requests a role. Since the planners' requests
  (`cc3f1ffb`), they nearly always do. In the five runs the role came up
  once (seed 1, day 1).
- That time the budget was 1,526 and the Death Star (upkeep 600) fit.
  Module `0xb` ordered it at Caprionril (Build `0x211`, 584 points), and
  the order passed the room and maintenance checks.
- The build gets one point per yard cycle (`FUN_0052a430`), about 0.25 a
  day there: 68 of 584 points by day 275. The Alliance took Caprionril on
  day 289, and the queue went with it.

So the Death Star's part works once the Empire has one and can keep it:
module `0xb` keeps it, and the planners destroy systems with it. Building
one is rare: the role is a fallback, and its 584 points take years at one
yard.

## Conclusion

The AI reaches every Standard target. The Empire wins 6 of 25 games. The
Alliance has no win; seed 15 came closest.

In the affordable fixture, the Death Star destroys 5 systems over seeds
2, 4 and 5. Two questions stay open:

- the side's maintenance capacity, which decides whether the Empire can
  afford a Death Star in an ordinary game, is the open economy question
  above;
- whether the original also builds the Death Star this rarely cannot be
  shown without telemetry from the original.
