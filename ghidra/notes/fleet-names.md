---
title: "Fleet Names: Fleet 1, Fleet 2"
description: "How a fleet gets its default name: FUN_00517760 numbers a fleet still bearing the record's name from its side's counter (FUN_005302c0), which never goes down"
category: "ghidra"
created: 2026-10-06
updated: 2026-10-06
tags: [fleets, names, counters]
---

# Fleet names

Recovered 2026-10-06 with Ghidra 12.1.3 headless (read-only project). The
manual agrees: "The default names of Fleet 1, Fleet 2, etc., are not very
meaningful" (p. 123).

## The name

An object's name is its own `+0x34` string, else its record's `+0x34`, else
empty (`FUN_004f62d0`, `FUN_004f6270`). Side setup (`FUN_004f9190`, from
`FUN_004fef10`) starts a new fleet with the record's name, TEXTSTRA 11523,
"Fleet". `FUN_004f6e60` sets `+0x34` and tells both sides' views.

## The number

`FUN_00517760` runs for an existing object (`+0x50` bit 6) whose class
answers slot `+0x48`. When its name still equals the record's
(`FUN_005f3390`), it:

1. takes its side's counter object (`FUN_00518750`: side `+0x24 >> 6 & 3`;
   `FUN_00506ea0`: `DAT_006b2bb0 + 0xc4` for side 1, `+0xc8` for side 2);
2. asks `FUN_005302c0` for the next number: a list keyed by the record
   (`FUN_00402d80`), whose node `[8]` it adds one to and returns; a missing
   node is made by `FUN_004f42a0` (its constructor `FUN_00540a50` is unread;
   the manual's first fleet is Fleet 1). Nothing lowers the count, so a
   number is never given twice, even after its fleet is gone;
3. writes record name + " " (`0x006a7f40`) + the number (`%ld`) through
   `FUN_004f6e60`.

A renamed fleet no longer bears the record's name, so it is never numbered
again.

## When it runs

`FUN_005140c0` (an object's load) and `FUN_0051b7f0` (a pass over every
object, `FUN_005136d0` state `0x17`) call it. Where a fleet made in play
(Create Fleet, a completed ship) is first numbered is untraced; next to read
is the fleet's slot `+0x114` handler `FUN_004fc290`, reached from
`FUN_004f7480` when a spare fleet becomes real (`FUN_004fe630`).

## Ported

- `GameWorld::fleet_names` (`rebellion-core` `world`): each fleet's name and
  each side's last number; saved (v27).
- `GameWorld::insert_fleet` numbers every fleet the game makes: seeding,
  completed ships and fighters (`integrator.rs`), and Create Fleet's new
  fleet (`fleet_join.rs`). hyp: as it is made, in creation order.
- `fleet_label` (`system_window.rs`) reads the name, so the System window's
  Fleets tab, the Fleet window, the Fleet Finder and the move confirmation
  all show it. A fleet with no number shows "Fleet" (`FUN_004f62d0`).
- port: Rename (`0x203`) is not ported.

## Supporting decompiles

`FUN_00517760`, `FUN_00518750`, `FUN_00506ea0`, `FUN_005302c0`,
`FUN_004f62d0`, `FUN_004f6e60`, `FUN_004f9190`, `FUN_004fef10`,
`FUN_005140c0`, `FUN_0051b7f0`, `FUN_005136d0`, `FUN_004fe630`,
`FUN_004f7480`, `FUN_004f7d50`.
