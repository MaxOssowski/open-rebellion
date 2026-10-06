---
title: "Manufacturing and Build Selection windows"
description: "Window type 9 and its 210 by 261 Build Selection child: pages, producer bands, resource IDs, controls, commands, quantities, costs, and completion/deployment fields"
---

# Manufacturing and Build Selection windows

Recovered 2026-10-06 from `REBEXE.EXE` with Homebrew Ghidra 12.1.4 in
headless, read-only mode on an isolated copy of the project. The trace was
cross-checked against the owned `STRATEGY.DLL` resources and the preserved
manual's Manufacturing and Production and Build Selection figures. No analysis
or lock files were written to the repository project.

This note separates facts recovered directly from code and resources from the
remaining inferences. See also [production-destination.md](production-destination.md)
for destination state and [build-delivery.md](build-delivery.md) for completion,
transit, and arrival behavior.

## Manufacturing window

`FUN_00452fc0` allocates `0x26c` bytes and constructs window type 9 through
`FUN_0045aac0`. The window is 226 by 304. Its content rectangle is
`(8,57)-(230,282)`, and it starts on page `0x67`.

`FUN_004568a0` selects these pages:

| Page | Object family | Recovered purpose |
|---:|---:|---|
| `0x67` | none | Manufacturing overview |
| `0x68` | `0x28` | Ship construction |
| `0x69` | `0x29` | Troop and special-forces training |
| `0x6a` | `0x2a` | Facility construction |
| `0x6b` | `0x2d` | Refinery facilities |
| `0x6c` | `0x2c` | Mine facilities |

The family meanings are corroborated by `FUN_00537ff0`, which maps production
manager type 0 to `0x28`, type 1 to `0x2a`, and type 2 to `0x29`, and by the
manual's three overview labels.

### Overview producer bands

`FUN_00458480` installs three selectable producer regions:

| Area | Rectangle |
|---|---|
| Ship Construction | `(55,57)-(221,136)` |
| Troops in Training | `(55,138)-(221,217)` |
| Facilities Under Construction | `(55,219)-(221,298)` |

The third source rectangle extends below the nominal content rectangle; the
port should preserve the source hit geometry until an original-runtime probe
proves clipping behavior. `FUN_00457690` draws the active or inactive shell,
the current page label, and overview capacity values at x=6 and y=119, 200,
and 280. Its producer counts and capacities come from `FUN_0052c8c0`,
`FUN_0052c5a0`, and `FUN_0052c270`.

Facility rows use 69 by 40 miniatures. `FUN_00454160` selects these page-label
or facility resource IDs:

| Family/page | Decimal | Hex |
|---|---:|---:|
| `0x2c` / `0x6c` | 10321 | `0x2851` |
| `0x2d` / `0x6b` | 10324 | `0x2854` |
| `0x28` / `0x68` | 10327 | `0x2857` |
| `0x29` / `0x69` | 10330 | `0x285a` |
| `0x2a` / `0x6a` | 10333 | `0x285d` |

`FUN_00458fe0` selects four-state item art from these bases:

| Type code | Families `0x28..0x2a` | Other facility families |
|---:|---:|---:|
| 1 | 9006 (`0x232e`) | 9001 (`0x2329`) |
| 2 | 9014 (`0x2336`) | 9030 (`0x2346`) |
| 3 | 9022 (`0x233e`) | not observed |
| 4 | 9010 (`0x2332`) | not observed |
| 5 | 9018 (`0x233a`) | not observed |
| 6 | 9026 (`0x2342`) | not observed |

The state offset is 0 through 3. The exact meaning of each state offset still
needs an original-runtime capture or a complete paint-path trace.

`FUN_004534f0` owns page selection, overview producer selection, list/item
selection, dragging, and release. Command `0x70` selects a page. The overview
routes one of its three producer regions to its corresponding production page;
non-overview pages route the pointer through the item list.

## Build Selection window

`FUN_0041d640` calls `FUN_00437df0`, which allocates `0x154` bytes and invokes
`FUN_00437880` with width `0xd2` (210), height `0x105` (261), and background
resource 10800 (`0x2a30`). The owned 210 by 261 STRATEGY bitmap matches the
manual's Build Selection figure. The class vtable is `PTR_FUN_00658db8`.

### List construction

`FUN_00437880` loads GOKRES module 10, initializes a candidate list with
`FUN_0052d720`, and filters it for the selected production manager through
`FUN_00537ff0`. For each buildable class it:

1. reads the class miniature resource from `class + 0x30 & 0xfff`;
2. centers the miniature in a 195 by 63 buffer;
3. attaches the class name; and
4. appends it to the drop-down list.

`FUN_00537ff0` scans production-manager families `0xa0..0xaf`, observes their
current producer state, and maps producer type 0 to ship classes (`0x28`),
type 1 to facility classes (`0x2a`), and type 2 to regiment/special-force
classes (`0x29`). It calls `FUN_0052e580` to add the eligible classes. This is
the recovered per-producer build-list boundary; it is not one global list.

### Layout and fields

`FUN_00437f80` creates these fields:

| Field | Rectangle or origin | Notes |
|---|---|---|
| Selected item | `(6,22)`, 195 by 63 | Resource 10650 (`0x299a`) |
| First cost | `(36,110)`, 64 by 23 | Plain text/value field |
| Second cost | `(138,110)`, 64 by 23 | Plain text/value field |
| Best completion time | `(140,145)`, 60 by 15 | Updated by `FUN_00438f30` |
| Best deployment time | `(140,165)`, 60 by 15 | Updated by `FUN_00438f30` |
| Number to build | `(141,196)`, 45 by 17 | Starts at 1; input limit `0x19` |

`FUN_00439160` recomputes availability, costs, completion, and deployment
whenever the selected class or quantity changes. `FUN_00538220` multiplies the
two class costs by the selected quantity, evaluates the available producers,
and derives the best completion and deployment values. `FUN_00438dd0` caps
displayed costs at 9999; `FUN_00438f30` applies the same display cap to the two
time values. Invalid values use text resource `0x3816`; the time format uses
`0x3815`.

The code confirms that completion and deployment are separate computed values.
It does not support treating the display as one generic ETA or replacing it
with a single destination combo.

### Controls and commands

`FUN_00438620` creates the source controls:

| Control | Origin | Rest/pressed resources | Command | Help/string |
|---|---|---|---:|---:|
| Close | source chrome | 10108/10109 (`0x277c/0x277d`) | 100 (`0x64`) | 6403 (`0x1903`) |
| Drop-down | `(79,90)` | 10606/10607 (`0x296e/0x296f`) | 108 (`0x6c`) | 6406 (`0x1906`) |
| Confirm | `(73,224)` | 10594/10595 (`0x2962/0x2963`) | 101 (`0x65`) | 6409 (`0x1909`) |
| Cancel | `(141,224)` | 10596/10597 (`0x2964/0x2965`) | 102 (`0x66`) | 6410 (`0x190a`) |
| Encyclopedia | `(5,224)` | 10592/10593 (`0x2960/0x2961`) | 103 (`0x67`) | 6411 (`0x190b`) |
| Quantity up | `(189,196)` | 10610/10611 (`0x2972/0x2973`) | 105 (`0x69`) | 6407 (`0x1907`) |
| Quantity down | `(189,205)` | 10612/10613 (`0x2974/0x2975`) | 106 (`0x6a`) | 6408 (`0x1908`) |

The two quantity controls repeat after 500 ms. `FUN_00438500` applies top trim
resource 10801 for side 1 and 10802 for the other side. It also creates label
resources `0x3810..0x3813`; their exact English strings remain to be bound.

`FUN_00438800` dispatches the commands:

- Close and Cancel discard the pending selection and close.
- Confirm calls `FUN_00438980` and closes.
- Encyclopedia opens the selected class through `FUN_0041d6b0`.
- Quantity up and down change the count.
- Drop-down opens the buildable-class list.

`FUN_00438b60` handles Enter as Confirm, Escape as Close, keypad `+` as
increment, and keypad `-` as decrement. `FUN_00438c30` and `FUN_00438c60`
clamp the quantity to 1 through 255; `FUN_00438c90` writes it to the edit.

Confirm copies the selected class key and quantity into the pending order,
updates the remembered selection for its producer category, and calls
`FUN_0041ce20(target, 0)`. `FUN_00439160` disables Confirm when
`FUN_00538220` reports that the order is not currently valid.

## Port contract

An authentic replacement for the current manufacturing panel must preserve:

- a type-9 modeless window with overview plus the five recovered pages;
- three independently selectable production-manager bands on the overview;
- producer-specific buildable lists rather than a global catalog;
- a separate bitmap-backed 210 by 261 Build Selection child window;
- the exact control geometry, source resources, commands, quantity range, and
  keyboard routes above;
- two distinct costs and two distinct best-time values;
- per-production-area Destination behavior and absolute completion/deployment
  day presentation where required by the surrounding runtime contract; and
- source-disabled Confirm behavior for an invalid order.

The current System window's Personnel, Fleets, Defenses, and Troops tabs are
not part of native window type 9. They must not be used as substitutes for
this family in parity mode.

## Open questions

- Bind the exact TEXTSTRA strings and label purposes for `0x3810..0x3816`.
- Recover the active and inactive window-shell resource IDs stored at object
  offsets `+0x180` and `+0x184`.
- Trace the complete scroll and selection behavior of the drop-down list.
- Name every producer-manager field used by availability and time calculation.
- Recover the exact disabled reasons and any user-facing rejection message.
- Capture every Manufacturing and Build Selection state from an owned English
  640 by 480 installation for A0 comparison.

