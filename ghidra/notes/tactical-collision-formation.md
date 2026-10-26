# Tactical collision and automatic grouping

This note records the collision envelope, overlap response, fighter-group
assignment, and bounded formation facts recovered from the owned English
`REBEXE.EXE` with SHA-256
`b3fe3997cab9a6e96403d638875dcba25484e4d8601751afec748471ac0ed6ab`.

## Recovered functions

| Function | Role |
|---|---|
| `FUN_005ab0e0` | Installs one mesh-derived collision envelope |
| `FUN_005b2e60` | Tests two tactical objects for strict overlap |
| `FUN_005b2f30` | Advances a capital object and restores its prior position on overlap |
| `FUN_005b8f70` | Fighter movement path using the same spatial query |
| `FUN_005ae330` | Maintains one colored fighter-group list |
| `FUN_005ae460` | Assigns and rebalances red, blue, green, and gold groups |
| `FUN_005c81d0` | Derives the formation leader count |
| `FUN_005c8300` | Produces the alternating vertical leader slots |
| `FUN_005c83c0` | Divides a source object list across formation leaders |
| `FUN_005cbbc0` | Requests a retained follower position from a formation visual |
| `FUN_005eb040` | Returns an existing follower offset or derives one from the anchor |
| `FUN_005ea250` | Retains the largest member footprint multiplied by 1.2 |
| `FUN_005ec400` | Produces the alternating lateral follower slots |

## Collision contract

The close/base mesh bounds produce two values:

```text
vertical_radius = max((max_y - min_y) * 0.5, 0.5)
planar_diameter = max(hypot(max_x - min_x, max_z - min_z), 0.5)
```

Two objects overlap only when both source comparisons are strict:

```text
abs(first_y - second_y) < first_vertical_radius + second_vertical_radius
xz_distance_squared < ((first_planar_diameter + second_planar_diameter) * 0.5)^2
```

Touching either boundary is not an overlap. After a moving object is
integrated, the source spatial query checks the new position. A collision
restores the previous position while retaining the velocity computed for that
step. The envelope comes from the close/base family and does not change with
visual LOD.

## Fighter groups

`FUN_005ae460` assigns original ordinals 29 through 32 and 33 through 36 to
red, blue, green, and gold independently for each faction. When a color is
empty, the first largest donor wins ties. The first half of that donor moves
only when the donor contains more than seven squadrons.

Capital objects are divided into three lists before task-force formation:

1. armed without laser cannons;
2. laser-armed;
3. unarmed.

The first list may use slots 0 through 3. The second starts after the first
list and may use four slots. The third consumes the remaining slots through 7.
Within each list, `floor(sqrt(n))` leaders are selected, capped by available
slots. Every `floor(n / leader_count)`th member becomes the next leader and
intervening members attach to the most recent leader.

Before that division, `FUN_005c81d0` inserts every member into an ordered list
with this key:

```text
key = -(maximum_weapon_range * 10000 + available_sublight_power)
```

The exact float at `0x0066cbb0` is `-10000.0`. `FUN_005b18d0` returns the
maximum installed ion, laser, or turbolaser range and capital vtable slot
`+0x90` resolves to `FUN_005b17f0`, the live sublight-power calculation.
Ascending key order therefore selects greater maximum range first, then
greater available sublight power. Stable source-list order resolves exact
ties.

## Formation boundary

For a source list of `n` objects, the leader count is `floor(sqrt(n))`, with
one leader for fewer than two objects and the source slot cap applied. Leader
Y slots alternate `0, +8, -8, +16, -16, +24, -24, +32`, with X fixed at zero.
The formation visual stores follower offsets relative to its anchor.
`FUN_005ea250` retains the greatest member visual footprint multiplied by the
exact float `1.2`. `FUN_005ec400` truncates that spacing toward zero and emits
the following lateral sequence relative to the leader's forward vector:

```text
0, +spacing, -spacing, +2*spacing, -2*spacing, ...
```

`FUN_005cbbc0` adds the returned X/Y/Z offset to each follower's current world
position. `FUN_005eb040` reuses an existing retained offset when present and
otherwise derives the delta from the formation anchor and member record.
`FUN_005ec400` also proves that the lateral offset rotates with the formation
visual's current forward vector rather than remaining fixed on world X.

Open Rebellion now implements the mesh envelope, strict overlap predicate,
post-integration restoration, exact colored-group assignment, the three
capital weapon categories, exact capability ordering, square-root leader
counts, stable follower task-force ownership, leader slots, largest-footprint
spacing, and the alternating follower geometry. The decoded close-mesh report
refreshes geometry while the initial battle remains paused. Movement now
reapplies the retained, forward-relative offsets after every stable-order
integration and collision pass. Lossless original-runtime comparison remains
open, so this functional contract is not an A0 visual claim.

## Tactic movers (2026-10-25)

`0x005cb160` (a task force's tactic) stores `+0x5c` at once and posts
event `0xb`. Its handler `0x005cb1a0` deletes the old mover `+0x54` and
builds a new one from the tactic and the lead unit `+0x6c`:

| Tactic | Class (constructor, size) | `+0x58` | Slot function (vtable `+0x30`) |
|---|---|---|---|
| 1 | `0x005eb860`, `0x140` (vtable `0x66d9a0`) | 1 | `0x005eb8d0` (`0x66d9d0`) |
| 3, lead unit of type 1 | `0x005c72b0`, `0x144` | 3 | not read |
| any other (2) | `0x005ec310`, `0x140` (vtable `0x66da08`) | 2 | `0x005ec400` (`0x66da38`) |

It then adds the lead unit and every other member of the force's list
`+0x1c` through slot `+0x1c` (`0x005ea250`).

`0x005ea250` (add a member):

- increments the count `+0x12c` before asking for the member's slot, so
  the lead unit is index 1;
- keeps the largest member footprint times 1.2 in `+0x13c`;
- for index 1, stores the lead unit's position (`+0x30`..`+0x3c` of a
  unit is its position, w = -1) as the formation anchor
  `+0x3c`..`+0x48` and its id in `+0x100`;
- stores the slot vector in the member record's `+0x50`..`+0x5c` and
  adds the record to the list `+0xe0`.

`0x005eb040` (slot `+0x24`) returns, for a member of `+0xe0`, the move
`anchor - slot - position` (zero within `DAT_0066d850`). Only
`FUN_005cbbc0` uses it, once, from the setup at `0x005c85fd`: each
member's position becomes `anchor - slot`.

`0x005eb8d0`, the tactic-1 slot, was run in an emulator (unicorn, mapped
image, `this+0x14` = 1, `DAT_006bd88c` = 1) for indices 0..45. Index `k`
gives a world-axis direction (it reads no orientation):

| `k mod 22` | direction |
|---|---|
| 0 | (√½, -√½, 0) |
| 1 | (0, 0, 1) |
| 2 | (0, 0, -1) |
| 3 | (1, 0, 0) |
| 4 | (-1, 0, 0) |
| 5 | (0, 1, 0) |
| 6 | (0, -1, 0) |
| 7 | (1, 1, 1)/√3 |
| 8 | (-1, -1, -1)/√3 |
| 9 | (-1, 1, 1)/√3 |
| 10 | (1, -1, -1)/√3 |
| 11 | (1, -1, 1)/√3 |
| 12 | (-1, 1, -1)/√3 |
| 13 | (-1, -1, 1)/√3 |
| 14 | (1, 1, -1)/√3 |
| 15 | (0, -√½, -√½) |
| 16 | (0, -√½, √½) |
| 17 | (-√½, √½, 0) |
| 18 | (√½, √½, 0) |
| 19 | (0, -√½, -√½): the same as 15; table `0x5ebc30` sends both to `0x005ebb04` |
| 20 | (0, √½, √½) |
| 21 | (-√½, -√½, 0) |

The direction is normalized (`0x005b50b0`). Its length is
`(floor(k / 22) * 0.25 + 1) * DAT_006bd88c` (`DAT_0066d8a8` = 0.25,
`DAT_0066d878` = -1.0). `DAT_006bd88c` is 12.5: the initializer
`0x005e9db0` (from `0x6a7754`) multiplies 5.0 (`DAT_0066d840`) by 2.5
(`DAT_0066d85c`).

The tactic's callers:

- `0x005a2920` gives each selected unit its own task force
  (`FUN_005aded0`) before setting the tactic.
- `0x005a3850` sets the tactic of a whole force and plays the formation
  voice (`0x6f` or `0xed` plus the group).
- `0x005a4a30` finds the force by id (`+0x58`).

Still open:

- which caller the Maneuvers panel's commit reaches;
- the formation's own step (`0x005eb1a0`, slot `+0x28` and the others):
  read and ported 2026-10-26, see
  `docs/qa/2026-09-08-full-functionality-audit/evidence/2026-10-26-formation-movers/`;
- the tactic-3 fighter class `0x005c72b0`.

The port still places every force by the Stand Off slots, whatever its
tactic.

## Orders and tactics from the selection (2026-10-25)

The Maneuvers commit (`FUN_005dd3a0`) calls `FUN_00595c00(order)` and
`FUN_00595c20(tactic)`: 1 when button `0xdd20` is down, else 2. Both go to
the current input state `DAT_006bc464` (`0x0059eec0`, `0x0059eef0`):
order to slot `+0x20`, tactic to slot `+0x28`. The returned state
replaces it (`0x0059f350`).

| State (vtable) | Order `+0x20` | `+0x24` | Tactic `+0x28` |
|---|---|---|---|
| one unit (`0x66bce8`, ctor in `0x005a1550`) | `0x005a1a60` | none (`0x0048b440`) | `0x005a1ac0` |
| several units (`0x66bc98`, ctor `0x005a2280`) | `0x005a2c70` | `0x005a2920` | `0x005a2a40` |
| a task force (`0x66bc48`) | `0x005a3630` | `0x005a3850` | `0x005a3740` |

- One unit: `FUN_005aded0` (a force of its own), then `SetOrder` (slot
  `+0x64`, voice `0x77`/`0xf5`) or `0x005c9be0`.
- Several units:
  - `0x0059f610` finds the first free group slot of 8 (`+0x164`).
  - With one, `0x005ade70` gathers the selection into one new force
    (`FUN_005aded0` for the first, `0x005ade50` for the others), and
    `0x005a03f0` stores the force in that slot.
  - Without one, voice `0x84`/`0x102`, and each unit gets a force of its
    own.
- `0x005c9be0` (SetTactic) stores `+0x38` and posts `0x3d`. Its handler
  (`0x005c9c50`):
  - stores `+0x3c`;
  - builds the tactic object `+0x40`: class `0x66ccb0` (type 1) for
    tactic 1, `0x66ccd0` (type 2) otherwise;
  - for a capital force (type 3, `0x005a6730`), rebuilds the mover
    `0x005cb1a0` with formation 1 or 2.
- A fighter group is type 4 (`0x005a6700`).

The destruction voice (`0x005a5cf0`) names the group slot of the
source's force (`0x56`/`0xd0` + slot), or uses `0x55`/`0xcf` when the
force has none.

A capital force (`0x66bf70`) keeps two member lists:

- `+0x1c`, through `0x005cb3e0`: also the lead unit `+0x6c` and the
  mover. It sets the ship's `+0x174`.
- `+0x44`, through slot `+0x4c` (`0x005cb770`) and `+0x54`
  (`0x005cb870`). It sets `+0x178`; `FUN_005aded0` and `0x005ade20` use
  this one.

The withdrawal leaves the `+0x174` force first (`0x005ad652`..`0x005ad6a1`,
clearing its group slot when local) and then calls `FUN_005aded0`.

Not settled: which of `+0x174` and `+0x178` is the numbered group the
interface shows, and so whether a single unit's order takes it out of its
numbered group. This needs a run of the original.

## Closing on a target (2026-10-25)

Source: `~/Games/rebexe-decompile/decompiled-typed/FUN_005cc300.c`,
`FUN_005edf80.c`, `FUN_005b49e0.c`, `FUN_005b7e50.c` (corpus outside the
repository).

A capital force's order step (`FUN_005cc300`, slot `+0x14`, every frame
from turn 3) handles a force with targets in its list `+0x8c`, unless:

- the order is 3 (withdrawal, earlier branch);
- the navigation route `+0x74` is still under way;
- the order is 11 (Hold Position, `0x005cc339`).

It then works as follows:

1. `d` is the first target's position less the formation anchor (mover
   `+0x3c`), and `r = |d|`. When `d` is zero, it is replaced by the
   side's vector times -5.0 (`DAT_0066cfec`).
2. The tactic object (`+0x40`, slot `+0x10`) gives a band (min, max).
3. When `r + 2.0` (`DAT_0066d008` = -2.0) lies within [min, max], the
   mover stops (`+0x78`..`+0x84` cleared).
4. Otherwise, for Stand Off (`+0x3c` = 2) closer than min:
   - unless the target force's own first target passes
     `Class_0066c390_vt14`, the force backs off along the side's vector
     times `min * -1.1` (`DAT_0066d00c`);
   - an AI side with more than 7 members switches to Surround
     (`0x005c9be0(1)`).
5. The direction is made unit, then scaled:
   - Surround: by the object's slot `+0xc`, 5.0 (`DAT_0066cc94`);
   - Stand Off: by `min + k * (max - min)`, with k = 0.5 for fewer than
     3 members, 0.75 (`DAT_0066d014`) for fewer than 6, else 0.9
     (`DAT_0066d010`); at least 2.0 (`DAT_0066d018`).
6. The mover heads for `anchor + step` with radius 2.0 (slot `+0xc`).

The force's slot `+0xc` (`0x005c9bd0`) has the tactic object recompute
its band (its slot `+4`) once a turn. The objects are:

- Surround (`0x66ccb0`, `0x005edf00`): min 0, max 5.0 × 0.5 = 2.5
  (`DAT_0066daa8`, `DAT_0066daac`).
- Stand Off (`0x66ccd0`, `0x005edf80`):
  1. The band list is a piecewise sum over distance. Each force member's
     weapons against the first target add `+value` on [0, range)
     (`FUN_005b49e0` with 0). The target's weapons against the force's
     first member add `-value` (with 1). Overlaps are split and summed
     (`FUN_005b7e50`).
  2. A weapon's value is its arc's count × the slot `+0x60` factor ×
     damage × the target class modifier. The arc is chosen by slot
     `+0x84`; the ranges come from slots `+0x50`, `+0x54` and `+0x58`.
  3. `reach` is the largest `hi - target max range` over the bands
     beyond the target's reach.
  4. The best band is the one of highest value whose `hi` is within the
     force's own greatest range.
  5. The band (min, max) is that band, with `+0xc` = `reach` when reach is
     above 0, else `lo + (hi - lo) / 3` (`DAT_0066dab8`).
  6. An unarmed force uses min = `+0xc` = max(target range × 1.2, 10)
     and max 1e6.
  7. A max of 0 becomes 10, and min is at least 5.0.

Ported in s101 (`update_engagement_bands`, `advance_engagement_approach`;
`docs/qa/.../2026-10-25-closing-on-targets/`). Not ported: the AI's switch
to Surround above 7 members. Fighter groups (`FUN_005c6610`) since s102.
