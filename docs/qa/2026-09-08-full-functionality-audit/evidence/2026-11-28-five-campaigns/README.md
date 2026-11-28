# Five 5,000-day campaigns against the M1 thresholds (F-007, roadmap M1)

Date: 2026-11-28, commit `b7984cda`. This is a measurement, not a code
change. Roadmap M1 asks five reproducibly configured 5,000-day seeds for
transit ≤10%, fleet move orders ≤1.5× arrivals, and fleets ≤3× the start.

## Run

`rebellion-playtest data/base --dual-ai --ticks 5000 --seed N --output cN.jsonl`
for seeds 1 to 5; the starting fleets come from the same command with
`--ticks 1 --summary`. The metrics and the 250-day snapshots are in
`five-campaigns.json`. The JSONL logs are not committed.

- Fleet move orders: AI actions `MoveShips`, `SetFleetDestination` and
  `MoveShipsIntoSpare`. Arrivals: `fleet_arrived`.
- Transit: `in_transit / fleets` at each 250-day `campaign_snapshot`.
- Fleets: the most at a snapshot, against both sides' fleets on day 1.

| Seed | End | Fleets start / most (×) | Transit most / mean | Orders / arrivals | Space + ground | Systems | Busiest |
|---|---|---|---|---|---|---|---|
| 1 | day 5000 | 14 / 22 (1.6) | 44% / 24% | 754 / 1,079 (0.70) | 118 + 32 | 23 | Mon Calamari 22% |
| 2 | Empire wins day 2577 | 13 / 26 (2.0) | 64% / 40% | 757 / 1,191 (0.64) | 228 + 28 | 26 | Talus 21% |
| 3 | day 5000 | 12 / 21 (1.8) | 52% / 27% | 923 / 1,287 (0.72) | 229 + 61 | 33 | Uvena 26% |
| 4 | Empire wins day 4053 | 12 / 36 (3.0) | 54% / 17% | 455 / 666 (0.68) | 69 + 36 | 21 | Woostri 15% |
| 5 | day 5000 | 14 / 32 (2.3) | 70% / 40% | 1,244 / 2,271 (0.55) | 541 + 59 | 34 | Kabal 19% |

Both wins are the full Empire word `0x10100031` (headquarters, Luke and
Mon Mothma held).

## Against the thresholds

- **Orders ≤1.5× arrivals: passes** on all five (0.55 to 0.72).
- **Fleets ≤3× start: passes** on all five (1.6 to 3.0; seed 4 at the
  limit). The snapshots are 250 days apart, so a peak between them is not
  seen.
- **Transit ≤10%: fails** on all five (most 44% to 70%).
- Diagnostics: battles 105 to 600 (seed 5 above 400), 21 to 34 systems,
  busiest system 15% to 26% (all under 40%).

## Where the transit comes from

A temporary trace (not committed) at the four places that give a fleet
move (`ai_plan.rs` `move_fleet` and the disembark move, module `0xb`,
module 9) counted the moves of the five runs:

| Seed | Plan target (state `0xd`) | Regroup (state `0xc`) | Module `0xb` | Disembark | Base drops |
|---|---|---|---|---|---|
| 1 | 251 | 431 | 54 | 17 | 1,105 |
| 2 | 332 | 269 | 96 | 33 | 1,728 |
| 3 | 488 | 310 | 24 | 49 | 41 |
| 4 | 224 | 109 | 60 | 37 | 48 |
| 5 | 632 | 301 | 62 | 49 | 167 |

Most moves are the sector plans' own: a fleet bound to a role goes to
its target (`FUN_004b0f90`), and a held fleet regroups to the plan's base
(`FUN_004b0c90`). Fleets on the way are skipped (`FUN_004b04c0`, record
`+0x38` 2), so no fleet is turned round mid-route.

Seeds 1 and 2 also show fleets going back and forth between two
neighbouring systems (seed 1, Empire fleet `8v25`, Svivren and Corsin,
every 9 days from day 1845). The trace found the chain:

1. Planner 1 of the Empire (sector 2) finds no base in its own sector and
   takes the nearest sector's (`FUN_0041b2b0`): system 76, the
   headquarters of sector 8, which takes mark `0x40000`.
2. Planner 2 (sector 8) cannot take 76 (`FUN_0041a340` forbids
   `0x40000`), so it takes another own system by key `0x26`.
3. On its next cycle, slot `+0x34` (`FUN_004af8a0`) drops that base: the
   sector has a headquarters (sector `+0x24` `0x40000000`, set by
   `FUN_00432350` for an own or neutral system with `+0x28`
   `0x80000000`) and the base is not it. It chooses again in the same
   call; equal keys go by the coin.
4. State `0xc` sends the plan's fleet to the new base.

Steps 2 and 3 follow the decompile of `FUN_004af8a0` line for line (the
port's `choose_base`), and the bits of step 3 match `FUN_00432350`. No
orphan base mark was found: a per-step check in module 8 saw every
`0x40000` mark held by a planner's base.

## What if the base did not change (not a fix)

A temporary switch (not committed, commit `b58c216f`) kept a planner's base
while it stays the side's and unblockaded, even when it is not the sector's
headquarters, so step 3 above never drops it. The same five seeds, against
the same build without the switch:

| Seed | Transit most / mean now | With the base kept |
|---|---|---|
| 1 | 44% / 24% | 50% / 23% |
| 2 | 64% / 40% (Empire wins day 2577) | 59% / 35% (day 5000) |
| 3 | 52% / 27% | 43% / 18% |
| 4 | 54% / 17% (Empire wins day 4053) | 61% / 33% (day 5000) |
| 5 | 70% / 40% | 48% / 30% |

The loop is not where most of the transit comes from: without it the most
is still 43% to 61%. It comes from the plans' own moves (states `0xc` and
`0xd`). The orders-to-arrivals ratio stays 0.63 to 0.74.

## Conclusion

The orders and fleet-count thresholds pass. The transit threshold does
not, and its cause is the ported planners' movement, including a
base-choice loop that the original's own functions produce in this
state. Without telemetry from the original, a 10% guide (set for the
retired heuristic AI, 2026-09-10) cannot be shown to hold for the
original. The item stays open for the owner's decision on the transit
threshold; no rule was added to hold fleets still.
