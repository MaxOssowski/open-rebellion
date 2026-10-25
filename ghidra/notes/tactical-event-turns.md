---
description: "The tactical battle manager's 1000 ms event turns: +0x878/+0x87c/+0x8ac lead, FUN_005a96b0, the per-turn order (+0x80 commanders, +0x84 task forces, +0x88 units), the per-frame slot +0x98, and the inventory of queued events with posters, handlers and port status"
---

# Tactical event turns

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`), 2026-10-25,
from the objdump disassembly.

## The two managers

| Manager | Constructor | Vtable | Frame function |
|---|---|---|---|
| A | `0x005be160` | `0x66c7c8` | `0x005bea..` (slot) |
| B | `0x005bf830` | `0x66c880` | `0x005c05..` |

Both run the same turn machine (see `0x005bea6a` and `0x005c0521`).

## Fields

| Field | Meaning |
|---|---|
| `+0x878` | Turns begun. Starts at 0 and advances at each turn (`0x005bec3c`). |
| `+0x87c` | The stamp for a new event. Starts at `+0x8ac` (`0x005a877b`) and advances once per turn, after the turn's updates (`0x005bedf6`). |
| `+0x8ac` | The lead, 3 (`0x005be233`, `0x005bf929`; never written otherwise). |
| `+0x8a8` | The turn length, `0x3e8` = 1000 ms (`0x005bea9c`, `0x005c0545`). |
| `+0x89c` | `timeGetTime` at the last turn. |
| `+0x8b0` | Turn sub-state: 0 run events, 1 updates, 2/4 waiting, 5 next turn. |
| `+0x908` + 12·side | The side's event queue (sorted by stamp). |

## One turn

1. Once `now - +0x89c >= 1000` (capped below 2000 ms), `+0x878` advances
   (`0x005bec3c`).
2. Once `+0x878 >= +0x8ac`, `FUN_005a96b0` runs each side queue's events
   stamped `+0x878 - 1`. It drops older ones and keeps newer ones.
   `+0x9e8` becomes `+0x878 + +0x8ac`.
3. On the next frame (`+0x8b0` = 1, `+0x8b4` = 4), the updates run:
   - slot `+0x80` = `0x005a9d10` → `FUN_005c8780`, each side's commander,
     with `+0x87c`;
   - slot `+0x80` again for the other side;
   - slot `+0x84` = `0x005a9d40` (1000.0 ms): for each task force (vtable
     `0x66bee0` fighters, `0x66bf70` capital ships), slot `+0xc`
     (`0x005c9bd0`), its mover's `+0x40` (`0x005ea740`), then slot `+0x10`
     (`0x005c65e0`, the trench-run result's update `0x005d0430`).
     Correction (2026-10-25, s87): the order step, slot `+0x14`
     (`FUN_005c6610` / `FUN_005cc300`), is not here. The frame function
     calls it with the frame's milliseconds (`0x005a9ff8`), see below;
   - slot `+0x88` = `0x005a9de0` (1000.0 ms): the Death Star's
     `FUN_005b8800`, then for each unit in the fight:
     - `FUN_005b6320` recharge;
     - slot `+0x9c` (capital ship `FUN_005b0330`);
     - `FUN_005b8800`;
     - slot `+0x88` (`FUN_005b3a40` weapons).
4. Slot `+0x6c` sends the turn's events. Then `+0x87c` advances, and slot
   `+0x70` waits for the other side.

So an event posted during the updates of turn k is stamped k + 2 and runs
at turn k + 3. One posted between turns (from the interface) is stamped
k + 3 and runs at turn k + 4.

Per frame, slot `+0x98` = `0x005a9ec0` runs with the frame's
milliseconds, but only once `+0x878 >= +0x8ac` (`0x005bef1d`). It:
- runs each task force's order step, slot `+0x14` (`0x005a9fec`..
  `0x005aa034`);
- moves each unit in the fight (slot `+0x8`, `FUN_005b2f30`) and holds it
  within +-2/3 of the extent;
- runs the state-7 jump flights (`FUN_005a6b40`).

The visual objects (`0x0059f4d0`, `0x005a6150`) are animated only in the
`+0xa08` = 1 branch.

## Posting

| Slot | Function | Stamp |
|---|---|---|
| `+0x24` | `0x005ad430` | given |
| `+0x28` | `0x005ad4a0` | `+0x87c` |
| `+0x2c` | `0x005be290` | the event's own (built by `0x005ce500` / `0x005ce460`) |

The event object (vtable `0x66c928`, `0x220` bytes) holds:

| Offset | Field |
|---|---|
| `+0x20` | time (stamp) |
| `+0x28` | type byte |
| `+0x29` | first id |
| `+0x2d` | second id |
| `+0x31` | payload |

The dispatcher is `0x005a9780`, table `0x5a9bc4`, index type − 3.

## Inventory

| Type | Handler | Posted by | Meaning | Port |
|---|---|---|---|---|
| `0x03` | `0x005b2b60` | `0x005ba720` (+1), `FUN_005b50b0`, `FUN_005b54d0`, `FUN_005b5ad0` | destroy target (source id) | Death Star: queued (s83/s84); weapon kills: immediate |
| `0x0a` | `0x005caba0` -> task force slot `+0x48` (`FUN_005ca6d0`) | `SetOrder` `0x005ca530` (`0x005ca684`; task-force slot `+0x64` in vtables `0x66bee0` and `0x66bf70`) | installs the given order's executor (`+0x64`, `+0x60`); `+0x68` is set at once | player orders queued (s94); withdrawal (`FUN_005ad530`) immediate |
| `0x0b` | `0x005cac00` (`0x005cb1a0` rebuilds the mover `+0x54`) | `0x005cb160` (sets `+0x5c` at once; UI `0x005a2920`, `0x005a3850`, `0x005a4a30`) | formation / tactic | tactic stored only; the formation mover is not ported |
| `0x0e`, `0x0f`, `0x10`, `0x12`, `0x13` | ? | `FUN_005b3f10` (+1; visual `0x005a6ac0` at +2) | weapon fire / hit | immediate |
| `0x16` | ? | `FUN_005b5ad0` | ? | not read |
| `0x19` | ? | `FUN_005b8800` | Death Star | not read |
| `0x1a` | `FUN_005b8bd0` | `FUN_005cf920` | Death Star damage | not read |
| `0x1e`..`0x22` | `0x005b28d0`, `0x005b28a0`, `0x005b2870`, `0x005b2840`, `0x005b2900` | `FUN_005b50b0`, `FUN_005b54d0`, `FUN_005b5ad0` | subsystem hits | immediate |
| `0x23`..`0x27` | `0x005b29c0`, `0x005b2990`, `0x005b2960`, `0x005b2930`, `0x005b29f0` | `FUN_005b1490` | subsystem repairs | immediate |
| `0x28` | `0x005b2630` | `FUN_005b2140` | ? | not read |
| `0x29` | `0x005b26a0` | `FUN_005b05c0`, `FUN_005b18d0` | ? | not read |
| `0x2b` | none (default) | `FUN_005b05c0` | no-op | — |
| `0x2c` | `FUN_005b1e10` | `FUN_005b8400`, `FUN_005b84e0` | ? | not read |
| `0x2d` | `FUN_005b1e40` | `FUN_005c6610` (`0x005c6809`), `FUN_005cc300` (`0x005cc53c`) | jump starts (state 7) or destroy | queued (s84) |
| `0x2e` | `FUN_005b26d0` | after `FUN_005a6b40` returns 1 (`push 0x2e` at `0x005aa2e1` and `0x005cc760`, not yet checked) | jumped unit gone (state 9) | queued (s84) |
| `0x2f` | `FUN_005b2720` | `FUN_005cf920` (`0x005cfc74`) | left behind: destroy | immediate |
| `0x30` | `FUN_005cc1b0` | `FUN_005aded0` | order 3 given | immediate |
| `0x32` | `0x005cae40` | `FUN_005c81d0` | ? | not read |
| `0x3c` | `FUN_005caf20` | `FUN_005cfe80` | trench-run result update | immediate |
| `0x3d` | `0x005cabd0` | `FUN_005c9be0` | ? | not read |
| `0x44` | `0x005ba520` | `0x005ba720` | superlaser beam starts | queued (s83/s84) |
| `0x45` | `0x005ba5e0` | `FUN_005c8780` (`0x005c8827`) | Death Star committed (`+0x78`) | AI path not ported |

"?" means not yet read. `0x2b` falls to the table's default (`0x005a9b0d`).

## Orders (s94)

`SetOrder` (`0x005ca530`, slot `+0x64`) keeps the old order when the task
force is on the trench run (`+0x68` or `+0x64` is 2 or 6) for a remote side,
otherwise stores the new order in `+0x68` and releases the per-order objects
(jump table `0x5ca694`). It then posts `0xa` through `+0x28` with the task
force id (`+0x18`), the order and a local-side flag. The handler calls slot
`+0x48`, `FUN_005ca6d0`, which takes the order from the event, stores it in
`+0x64` and creates its executor (jump table `0x5cab6c`).

`FUN_005ad530` (withdrawal) also calls slot `+0x64`: with 2 for a fighter
group without hyperdrives, 3 for the others (`0x005ad60f`) and 3 for each
capital ship's task force (`0x005ad6b8`). A group on the trench run first
posts `0x3c` with state 8 (`0x005ad5e7`). The port still applies these at
once.
