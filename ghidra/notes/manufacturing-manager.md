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

Its creation numbers it: event `0x300` reaches `FUN_00517760`, which names
an unnamed capital ship "{class} {N}" with N from its side's per-class
counter (`FUN_005302c0`). The counter never goes down, so a product
scrapped before it completes burns its number. A0
`2026-11-13-band-title-timing/41`, `42`: Drall's "Alliance Escort Carrier
4", replaced by a one-unit Confirm on day 278, is gone; the new order's
ship in Fleet 8 is "Alliance Escort Carrier 5". With several units the
first takes the next number and the others the ones after it
(`2026-11-11-icon-menu-topic/66`: "Carrack Light Cruiser 2 / Building: 4"
with Carrack 1 in play).

## The band's product line

`FUN_00457b40` writes the band's product line, and only when the band is
built: by `FUN_00458080` when the window is created and by `FUN_00457c90`'s
full rebuild. A rename (`FUN_00517760`, or Rename `0x215`) does not
rewrite it. Inference fitting every A0: after a Confirm of exactly one unit
the window rebuilt the band before the product was numbered, so the line is
the bare class name and stays so until the next rebuild (A0
`2026-11-13-band-title-timing/03`, `10..39`: "Alliance Escort Carrier" for
23 days with the window open; `42`: "Alliance Escort Carrier 5" after the
window is closed and opened; `2026-11-13-build-confirm/16` bare). With
several units the line names the first unit with its number at once (A0
66). What triggers `FUN_00457c90` is not traced; the port rebuilds when the
band's units left or current product change.

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

`rebellion-core/src/manufacturing.rs` keeps a queue per system and area
with the two point counts; `production_cycle::advance_yards` runs the yards
that feed it.

| Original | Port |
|---|---|
| Build replaces the queue; Stop empties it | Same (`build_selection::confirm_order`, `build_units`, `stop`). The scrapped products refund nothing (cause `0x16`, see Open 3) and the new current product starts from no points. |
| A product is named at its order from its side's counter | Same (`QueueItem::ordered`, `GameWorld::take_ship_name`, save v116); the name rides the queue item, its delivery and its completion to the ship. The Death Star stays unnamed. |
| The band's product line is written when the window builds the band | `SystemWindowState::shown_band` keeps each open window's line until the band's units left or current product change, or the window is opened anew; a one-unit Confirm leaves the bare class name (`confirmed_build`). |
| A unit costs its class refined cost in points | Same (`QueueItem::cost`, `ProductionQueue::add_point`). |
| A point goes to the current product, or to the overflow once its cost is met | Same. |
| Completion at non-zero points that reach the cost; the next product starts with `min(cost, overflow)` | Same, chained. |
| An emptied queue clears both counts | Same. |
| One refined material per yard cycle start; a yard without one queues and is served first come first served | Same (`SideStock::refined_requests`), with the cycle counter check. |
| A yard's cycle is its class rate | Same; a yard whose class has no rate is left out. |
| No more yards run than points are needed; wake fastest first, stop slowest first and state 1 before 2 before 3 | Same (`YardDay::regulate`). |
| Each event acts as it happens | The managers are walked once a day. Every cycle of a manager that ends on a day adds its point before the yards are set, so an order ends on Build Selection's day (`FUN_00528d30`); the original's order among timers of one day is not traced. A yard woken by an order counts its cycle from the day before, when the order was given. |
| Suspended yards (system in uprising or blockaded) do not count | They are put to state 4 and lose the cycle they were in (`production_cycle::suspended_systems`). Whether a suspended yard's running cycle still ends in the original is not traced. |
| A manager with no yard scraps its queue | Same (`production_cycle::scrap_yardless_queues`); the queued units are dropped, not destroyed as objects. |
| Build is refused without a usable yard of the area | `build_selection::order_items` gives no order, and the AI orders only what a system's own yards build (`ai.rs`). |
| Products exist, and cost upkeep, from the order | Upkeep counts queued units (`side_resources.rs`); the objects are created at completion, with the name their order gave them. |
| A yard stopped in a cycle | Its refined material is spent; whether the original returns it is not traced. |
| No order to remove or reorder one unit | `cancel` and `prioritize` remain as port helpers with no caller; they leave the points as they are. |

## Open

1. Which of `0x210..0x216` calls which manager slot, from the order events'
   fire functions.
2. The validations at slots `+0x1c8`, `+0x1d0` and `+0x1dc`.
3. Settled 2026-11-13: scrapping with cause `0x16` refunds nothing for
   a queued product. `FUN_00530270` refunds half the class refined cost
   only for causes `0x14..0x16` of a completed object (`+0x50` bit 2),
   and a queued product never is (`scrap-order.md`). The refined
   material its yard cycles took stays spent.
4. What triggers the band's full rebuild `FUN_00457c90` (see "The band's
   product line").
