---
title: "Manufacturing manager: orders, points, yards and completion"
description: "How a production manager queues products, counts build points from its yards, picks and completes the current product, and what each command does; with the port's differences"
category: ghidra
created: 2026-10-13
updated: 2026-10-13
tags: [manufacturing, production, manager, yards, refined-materials, build-queue]
---

# Manufacturing manager

Read from the typed corpus (`~/Games/rebexe-decompile/decompiled-typed`,
Ghidra 12.0.2 with the generated names and structures) for the GOG
`REBEXE.EXE`. Field and function names are the generated ones
(`ghidra/generated/setters.tsv`, `class-fields.tsv`). Yard cycles and
refined material are in `side-resource-monitors.md`; the windows are in
`manufacturing-build-selection.md`; delivery is in `build-delivery.md`.

## The manager

A system keeps one manager per production area, and a yard finds it through
its own area (`FUN_0052eb60`: yard slot `+0x204` into `FUN_00509670`). Class
`ManuMgr`, vtable `0x0065fbf8`, a `GameObj`.

| Field | Name | Meaning |
|---|---|---|
| `+0x54 -> +0x18` | | The queue: the keys of the products ordered here, in order. |
| `+0x58` | RemainingGameObjCount | Queue length (`FUN_0052a7d0`). |
| `+0x5c` | CompletedPointCount | Points done on the current product. |
| `+0x60` | OverflowPointCount | Points made beyond the current product's cost. |
| `+0x68` | RequiredPointCount | The current product's cost: its class refined material cost (`FUN_0052a8f0`, `FUN_0053b860`); 0 with no product. |
| `+0x6c` | TotalRequiredPointCount | The summed cost of everything queued (`FUN_0052a9c0`). |
| `+0x70` | Reserved | The Reserved command's flag. |
| | ProductKey, SeedKey, DeploymentKey, TargetKey, ProductName | The current product, its class, where it deploys, the area's destination, and its name. |

A product is a real object from the order on: `FUN_0052a140` creates each
unit with `FUN_004f7d50(class, side, ctx, created = 1, completed = 0)` and
sets its BuilderKey to the manager. It is Existing at once, so its upkeep
counts from then.

## Queue membership and the current product

Two events keep the queue in step with its products:

- `0x300` (Existing changed, handler `FUN_005140c0`): for an object that is
  not completed and has a builder, `FUN_00529cf0` on that manager.
- `0x308` (Completed changed, handler `FUN_005150c0`): `FUN_00529d70` on
  the builder.

Both start with `FUN_0052a710(product)`. A product belongs in the queue
while `FUN_00528890` accepts it: created, not completed, destroyed or en
route (`+0x50 & 0x1c == 0`), and built by this manager. `FUN_0052a710` adds
an accepted product that is missing and removes a listed one that is no
longer accepted, then recomputes TotalRequiredPointCount
(`FUN_0052a9c0`), RemainingGameObjCount (`FUN_0052a7d0`) and the current
product (`FUN_0052aac0`: the first accepted product in the queue, or none).

Setting ProductKey (notifier slot `+0x220`) sets SeedKey, DeploymentKey and
ProductName from the product (`FUN_0052a800`, `FUN_0052ac40`,
`FUN_0052ad30`); setting SeedKey (slot `+0x210`) sets RequiredPointCount to
the class cost.

## Points

Each yard cycle that ends adds one point (`FUN_00530950` ->
`FUN_0052a430`): to CompletedPointCount while it is below
RequiredPointCount, otherwise to OverflowPointCount. A cycle start costs
one refined material (`side-resource-monitors.md`).

After every point, and after every queue change, `FUN_00529dd0` sets how
many yards run. The manager's yards are those of its area and side that
are usable (`FUN_0052bde0`) and not Suspended. With

    needed = TotalRequiredPointCount - OverflowPointCount - CompletedPointCount

it wakes inactive yards, fastest class rate first (`FUN_0053aac0`), while
fewer than `needed` are in states 1..3, and puts running yards to state 4,
slowest first and state 1 before 2 before 3 (`FUN_0053ab10`), while more
are. With no yard at all it scraps the queue (`FUN_0052a2c0`: each queued
object's slot `+0xac` with cause `0x16`).

## Completion

The CompletedPointCount notifier (slot `+0x208`, `FUN_0052b960`) completes
the current product when CompletedPointCount is non-zero and at least
RequiredPointCount. `FUN_0052bee0` sets the product Constructed
(`FUN_004f7aa0`), EnrouteActive when its deployment location is not where
it stands (`FUN_004f7640`), and Completed (`FUN_004f74f0`).

Completed raises `0x308`, whose handler `FUN_00529d70` removes the product
from the queue, picks the next one, and gives it a head start:

    CompletedPointCount = min(RequiredPointCount, OverflowPointCount)
    OverflowPointCount -= CompletedPointCount

When the queue empties, the RemainingGameObjCount notifier (slot `+0x204`,
`FUN_0052b8c0`) clears TargetKey, CompletedPointCount and
OverflowPointCount, so unused overflow is lost.

## Commands

The manager's slots `+0x1f0..+0x200` carry the window's commands. Which
command reaches which slot is read from what each slot does (inference);
the `0x210..0x216` event handlers are not settled in `events.tsv`.

| Slot | Does | Command |
|---|---|---|
| `+0x1f0` `FUN_00529ab0` | Scraps the queue (`FUN_0052a2c0`), then creates the ordered units (`FUN_0052a140`). | Build (`0x210..0x212`) |
| `+0x1f8` `FUN_00529ba0` | After validation (`+0x1d0`), scraps the queue. | Stop (`0x213`) |
| `+0x200` `FUN_00529cd0` | `FUN_0052a380`: sets TargetKey and calls each queued object's slot `+0xa8` with it. | Destination (`0x214`) |
| `+0x1f4` `FUN_00529b00` | After validation (`+0x1c8`), sets ProductName (`FUN_0052a240`). | Rename (`0x215`) |
| `+0x1fc` `FUN_00529c30` | After validation (`+0x1dc`), sets Reserved (`FUN_0052a360`). | Reserved (`0x216`) |

Build validation (slot `+0x1bc`, `FUN_00527d00`) fails with the pair
`(0xa0, 3)` when the manager has no usable, unsuspended yard.

A manager that changes side (slot `+0xf0`, `FUN_0052b790`) or stops
existing (slot `+0x128`, `FUN_0052b7d0`) scraps its queue and clears
Reserved; one that stops existing also takes side 3.

## The port

`rebellion-core/src/manufacturing.rs` keeps a queue per system and area and
counts days per unit, computed at the order from the yards' rates
(`completion_day`, `unit_build_days`).

| Original | Port |
|---|---|
| Build replaces the queue; Stop empties it | Same (`build`, `stop`). |
| A unit costs its class refined cost in points | Same total, as days: `unit_build_days` spreads `k x cost` over the yards. |
| One refined material per yard cycle start; a yard without one waits | Approximated per day and per area (`production_cycle::supply_yards`). |
| Overflow carries to the next product; lost when the queue empties | Within one order the cumulative day count has the same effect; nothing carries past an order, which is also the original's case once the queue empties. |
| No more yards run than points are needed | Every yard counts to the end, so the last points of an order can finish up to a cycle early. |
| Suspended yards (system in uprising or blockaded) do not count | Same: both stop the system's queues (`production_cycle::suspended_systems`). |
| A manager with no yard scraps its queue | Same (`production_cycle::scrap_yardless_queues`); the queued units are dropped, not destroyed as objects. |
| Build is refused without a usable yard of the area | The AI orders only what a system's own yards build (`ai.rs`); before, any manufacturing facility let it order ships. |
| Days are fixed at the order | Same; a yard gained or lost mid-build does not change them, where the original's point rate follows the yards. |
| Products exist, and cost upkeep, from the order | Upkeep counts queued units (`side_resources.rs`); the objects are created at completion. |

## Open

1. Which of `0x210..0x216` calls which manager slot, from the order events'
   fire functions.
2. The validations at slots `+0x1c8`, `+0x1d0` and `+0x1dc`.
3. What scrapping with cause `0x16` refunds, if anything, beyond the
   destruction refund in `side-resource-monitors.md`.
