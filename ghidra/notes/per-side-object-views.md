# Per-side object views (OBJ-02-C010)

Traced 2026-11-14 from `~/Games/rebexe-decompile/decompiled-typed` and the
REBEXE.EXE disassembly (objdump at `0x4f9ba0`).

## Three copies of every object

Every game object belongs to a group that holds three copies
(`FUN_005844e0(side)`: group `+0x24 + side * 4`, side 0..2):

- copy 0 is the true object (master);
- copy 1 is the Alliance's view, copy 2 the Empire's view.

`FUN_00539fd0(side)` reaches a sibling copy through the object's group
pointer `+0x20`. Each copy's flags word `+0x24` holds:

- bits 0..3: the copy's sync state (`FUN_00540630` sets it,
  `FUN_005404d0` tests it): 1 live, 2 frozen, 3 any;
- bits 4..5: which copy this is (`FUN_00540650`; `FUN_0053a000` is "master",
  `FUN_0053a010` "a side's copy");
- bits 6..7: the owning side (`FUN_00540670`).

(`entity-system.md` calls bits 4..5 "difficulty": that reading is wrong.)

## How a change reaches a side's copy

A master setter changes the master, then calls the matching sync on copy 1
and copy 2 (for example `FUN_004f7720`, `FUN_004f6e60`,
`Character_SetFastHeal` `FUN_004ee8e0`). It passes an update context
(class `0x0065d42c`, `FUN_004fd400` defaults: `+8` = 1, `+0xc` = 1,
`+0x10` = 1). The copy's sync (`FUN_004f9ba0`, `FUN_004f9ca0`) applies the
change only when:

- `FUN_005404d0(context +8)` holds on the copy: its state equals the
  context's, or either is 3. A frozen copy (2) fails against the default
  context (1), so **a frozen copy keeps its old values**;
- `FUN_004fd600(context, copy side)` (`+0xc` Alliance, `+0x10` Empire) is
  set. `FUN_004fd620` clears one of them only in `FUN_004fc840` (the
  object's own side) and sets both in three mission-key paths.

## When a side's copy is live (`FUN_004f9a60`)

For copy 1 and copy 2 of an object (`this` = master):

- live (1) when the master's owner (`+0x24` bits 6..7) is that side;
- live (1) when master `+0x50` has the side's seen bit (bit 7 `0x80`
  Alliance, bit 8 `0x100` Empire) **and** bit 2 `0x4` (completed) **and
  not** bit 4 `0x10` (en route);
- frozen (2) otherwise.

On a change from frozen to live it rebuilds the copy from the master: copy
vtable `+0x24` with a context whose `+8` is 1, then `FUN_004f8340`.

Callers: `FUN_004fae40` (owner change, then children `+0xb0`),
`FUN_004fb310` (container change), `FUN_004fb860` / `FUN_004fb920` (the
Alliance / Empire seen bit changed: self, then each child's vtable `+0x8c`
/ `+0x90`).

## The seen bits

- `FUN_004f7720(value)` writes `+0x50` `0x80` (Alliance), `FUN_004f7790`
  `0x100` (Empire); both sync the copies and run vtable `+0x12c` / `+0x130`.
- A system (`FUN_0050ac80`): Alliance seen = an Alliance fleet in the
  system (`FUN_004ffef0(system, 1, 1)`, child types `0x08..0x0f`) or the
  system is the Alliance's (`+0x24 & 0xc0 == 0x40`). Empire the same with
  side 2 / `0x80`. Run on a fleet entering or leaving (`System_vtc8`
  `FUN_00508660`, child type `0x08..0x0f`) and on an owner change
  (`System_vtf0` `FUN_00510820`).
- Any other object (vtable `+0x8c` default `FUN_004f7bf0`, 80 vtables): the
  container's (`FUN_004f6b50`) Alliance bit, when the object's `+0x50`
  bit `0x20` is clear and its vtable `+0x4c` holds; else 0. Empire the
  same through `+0x90`.
- Always seen (`FUN_004f7720(1)` / `FUN_004f7790(1)`): class `0x00662158`
  (`FUN_00556f10`, `FUN_00556f20`) and the managers `0x00661c08` /
  `0x006630c0` (`FUN_005620a0`, `FUN_005620c0`).

So a side sees an object live while the object is its own, or while it is
complete, not en route, and inside a system where the side has a fleet or
which the side holds. Anywhere else the side's copy stays as it was.

## Intelligence pushes (`Event319_Handle` `FUN_0050d5a0`)

`Event319_Handle(level, side, context)` copies parts of a system's master
into the side's copy, live or frozen (context `+8` = 1, `+0x14..+0x18` =
1, 3):

"Fields" = explored, populated (`System_SyncExplored`, `System_SyncPopulated`)
and owner (`FUN_004f9ca0`). "Whole" = the copy's vtable `+0x24` from the
master. Type ranges are the child object types pushed (`FUN_00520580` into
three range lists: the system's children, then its fleets' children and
their children; which range lands in which list is not traced).

| Level | System | Child type ranges |
|---|---|---|
| 1 | fields; control kind, control data, loyalty | none |
| 2 | fields; energy, raw material | none |
| 3 | fields | `0x10..0x1f`, `0x22..0x27` |
| 4 | fields | `0xa0..0xaf`, `0x28..0x2f` |
| 5 | fields | `0x08..0x0f`, `0x01..0xfe` |
| 6 | fields; control, loyalty, uprising (`FUN_0050f840`, `FUN_0050eea0`) | `0x08..0x0f`, `0x01..0xfe` |
| 7 | fields | `0x08..0x0f`, `0x01..0xfe` twice |
| 8 | whole | `0x20..0x2f`, `0x10..0x1f`, `0xa0..0xaf`, `0x08..0x0f`, `0x01..0xfe`, `0x10..0x1f` |
| 9 | fields | `0x30..0x3f` |
| 10 | whole | `0x01..0xfe` three times |
| 11 | whole | `0x10..0x3f`, `0xa0..0xaf` |
| 12 | fields | `0x20..0x2f` |

Then it walks the system's fleets (`FUN_004ffe70(system, 4)`) and pushes
their ships and units the same way. Child pushes go through
`FUN_004f7e30`.

Callers: Espionage 10 (`FUN_00573610`, `FUN_00573890`), Reconnaissance 8
(`FUN_0056bee0`), 10 for the Empire (`FUN_0051b990`), 6 for both sides
(`FUN_0054b040`, `FUN_0055ff60`), 6 for the mission's side
(`FUN_00592c80`), 11 (`FUN_005650f0`), 12 and 4 (`FUN_00566eb0`), 5
(`FUN_00556c70`), a level table (`FUN_0050d510`). Their triggers are
untraced.

## Who reads the copies

The windows look objects up in the player's side's copy: `FUN_004f2d10(side,
id)` -> `FUN_0053ef00` -> `FUN_0053efd0(id, side)` -> the group's copy. The
side comes from the window's `+0x114 -> +0x9c` (`FUN_0044aa20`,
`FUN_00445c60`, `FUN_00442d70`, 85 typed lookups `FUN_004f2d10..FUN_004f3f10`).
So every window shows the side's copy: live where the side sees the object,
frozen elsewhere. A0 OBJ-02-C010 (Chandrila, regiment frozen at "Enroute,
ETA Day 171" on day 576) fits: the regiment's Alliance copy froze while it
was en route and never became live again.

## The port

The port has one object per id. `opposing_contents_visible` (render
`system_window.rs`) shows the true objects when the player holds the system
or its fog has ever revealed it; the fog never forgets. Untraced here:
vtable `+0x4c` of `FUN_004f7bf0`, `+0x50` bit `0x20`, and the triggers of
the level 1..12 pushes.
