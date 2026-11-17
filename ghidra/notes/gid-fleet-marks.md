# The GID fleet marks and whose fleets they count (CMD-02-C024)

Traced 2026-11-17 from `~/Games/rebexe-decompile/decompiled-typed` and the
REBEXE.EXE disassembly (objdump at `0x4f9d40`, `0x4fca30`, `0x4fcb50`; the
fleet vtable `0x0065d438` read from the image). Builds on `gid-markers.md`
(the mark tables), `per-side-object-views.md` (the three copies) and
`object-state-flags.md` (the `+0x50` bits and the child walk).

## The marks

`FUN_0042b330` cases `0x20`/`0x21` (Idle Fleets) and `0x22` (Fleets
Enroute), with `FUN_00426e70(value, side, 3, 2, 2, 1, 1)`:

| Player's count | Enemy's count | Mark |
|---|---|---|
| > 0 | > 0 | `0x27b0` (STRATEGY **10160**), value = sum |
| n > 0 | 0 | the player's table by n: 1 medium, 2 large, 3+ largest |
| 0 | n > 0 | the enemy's table by n |
| 0 | 0 | `0x21`: the holder's smallest (`FUN_004feea0(system, holder, 1)` is 0 here); `0x22`: the holder's smallest when the copy is explored (`+0x88` bit 1), else side 0, the unexplored mark |

Tables (`FUN_00426ee0`, smallest..largest): Alliance `0x27a5..0x27a2`
(10149..10146), Empire `0x27a6..0x27a9` (10150..10153), no side
`0x27aa..0x27ad` (10154..10157). So **10151** (`0x27a7`) is the Empire's
medium mark: one Imperial fleet. 10158 is the unexplored mark, 10166 the
headquarters burst under a mark. 10160 is the only special art in this
range; `0x27b0` is used nowhere else in `FUN_0042b330`.

## What `FUN_004feea0` counts

`FUN_004feea0(system, side, mode)` = `FUN_004f6c00(range 8..0x10, side,
mode)`: the iterator `FUN_00513090` over the system's **direct children**
(`FUN_00513120`, `FUN_005130d0`), accepting (`FUN_005131d0`) a child whose
family is `0x08..0x0f` (a fleet), whose `+0x24` bits 6..7 equal `side`
(`+0x14` = 1 always turns the side filter on), and whose `+0x50` passes
`FUN_004f6b90(mode)`: 1 usable (bit 0), 2 en route (bit 4).

It asks nothing about knowledge itself. The knowledge is in **which
system object** it walks: `FUN_0042b330` gets the system as
`FUN_004f2d10(view +0x9c, id)` (its own top, and its callers
`FUN_0042aaf0`, `FUN_0042abd0`; `FUN_0042ad10` uses `FUN_004f3220`), the
typed lookup into **the player's copy** (`per-side-object-views.md`). The
children of that copy are the player's copies of the fleets, placed and
flagged by the copy rules below.

## The player's own fleets

The player owns them, so their copies are always live (`FUN_004f9a60`:
the owner's copy is state 1) and every setter's sync reaches them.

- A move (`FUN_00556390`) sets the in-transit bit 5 (`FUN_004f7640`), which
  recomputes bit 4 (`FUN_004fb760` -> `FUN_004f8240`), then changes the
  container to the destination at once (slot `+0xa8`). The live copy
  follows (`FUN_004f6fd0` -> `FUN_004f9d40` on copies 1 and 2, the live
  branch: `FUN_004fc600` into the side's copy of the new container).
- So Fleets Enroute counts every own fleet in hyperspace at its
  destination from the first refresh. A0 (the user's runs): the Empire's
  own fleet en route draws 10151 at once.
- Idle Fleets counts own usable fleets in orbit: bit 0 (`0x004f7b80`:
  existing, completed, not en route; the fleet override `FUN_004fe540`
  needs a completed member not in transit). A fleet in hyperspace is a
  child of its destination with bit 0 clear, so it counts at neither end.

## The enemy's fleets

The player's copy of an enemy fleet is live only while the fleet is
complete, not en route (bit 4 clear) and its seen bit for the player is
set (`FUN_004f9a60`). Its seen bit (`FUN_004f7bf0`, fleet vtable `+0x8c`
/ `+0x90`) is the container's, and 0 while bit 5 (in transit) is set. A
system's seen bit (`FUN_0050ac80`) is set while the side holds it or has a
usable fleet there. Liveness is recomputed only on an owner change,
`FUN_004fb310` (the fleet's usable handler, vtable `+0x110` `FUN_004ff5e0`,
also reaches it), and a seen bit change (`FUN_004fb860` / `FUN_004fb920`).

**Departure.** `FUN_00556390`:
1. `FUN_004f7640` sets bit 5 and syncs live copies; its handler (fleet
   vtable `+0x124` = `FUN_004fb760`) runs, in this order: `+0x8c`, `+0x90`
   (the seen bits drop to 0 for bit 5 -> `FUN_004fb860` ->
   `FUN_004f9a60`: the player's copy **freezes now**, with bit 4 still
   clear), then `FUN_004f8240` (bit 4 set on the master; the frozen copy
   does not take it).
2. The container change: `FUN_004f9d40` on the frozen copy (disassembly
   `0x4f9e4b..0x4f9eca`): its state is not the context's, so it asks the
   copy's **old** container (the origin's copy). If that is live for the
   player, `FUN_004fca30` -> `FUN_004fcb50` moves the copy into the side's
   `0xf2` object (`FUN_0053f030({0xf2, 0xf3}, side, 1)`), the "Location
   Unknown" container (`personnel-finder.md`). Else the copy stays where
   it froze.

So an enemy fleet's copy never sits in the destination with bit 4 set
after a move: the player's copy of the destination does not hold it
while it travels, and **an enemy fleet in hyperspace is not counted** by
Fleets Enroute.

**Arrival.** Bit 5 clears: `FUN_004fb760` recomputes the seen bits first;
where the player sees the destination the seen bit rises, but
`FUN_004f9a60` still finds bit 4 set and keeps the copy frozen. Then
`FUN_004f8240` clears bit 4, `FUN_004fb6a0` (`+0x120`) runs the fleet's
usable recompute (`+0x84`), usable rises, `Fleet_vt110` -> `FUN_004fb310`
-> `FUN_004f9a60` makes the copy live and rebuilds it (vtable `+0x24`,
`FUN_004f8340`). From then on Idle Fleets counts it. Where the player
does not see the destination, the copy stays frozen where it was.

**Frozen systems.** Idle Fleets at a system the player does not see
counts the enemy fleets its frozen copy holds: those seen there last,
usable then, whether or not they are still there.

**The intelligence pushes** (`Event319_Handle` `FUN_0050d5a0`) sync a
side's copies "live or frozen" (context state 3) and walk the system's
fleets with mode 4 (`FUN_004ffe70(system, 4)`, all of them). Levels 5, 6,
7, 8 and 10 include the fleet family `0x08..0x0f`; children are whole-
synced (`FUN_004f7e30` -> `FUN_004f9510` -> copy vtable `+0x24`). A push at
a system while an enemy fleet is in hyperspace towards it (a child of the
master with bit 4) is the one path traced that puts an enemy copy with
bit 4 into the player's copy of the system; hyp: the whole-sync places it
there (the container part of vtable `+0x24` is not read). Such a copy
stays frozen with bit 4 until the player sees the system again (or a later
push), because at the fleet's arrival the seen bit does not rise there.
Pushes that carry fleets: Espionage (10), Reconnaissance (8), an
encounter (6, both sides), the Final Battle (6), a mission's end at a
system target (6, its side), an informant incident (its level). A
bombardment (11) carries no fleet family.

## When 10160 is drawn

- **Idle Fleets** (`0x21`, Alt+3): the player has a usable fleet in orbit
  at the system and the player's copy holds a usable enemy fleet there.
  Since the player's fleet makes the system seen, that is the live
  enemy fleet in orbit there: both sides' fleets in one system, outside
  hyperspace (before a battle is fought, after a battle neither side
  left, or when a fleet arrives and no battle starts). A frozen copy
  cannot pair with a live own fleet (the own fleet makes the copy live).
- **Fleets Enroute** (`0x22`, Alt+4): an own fleet in hyperspace to the
  system and an enemy copy there with bit 4, which only an intelligence
  push during the enemy's transit makes. Rare: the A0 runs over ~150 days
  never drew it, and only own fleets marked en route (10151 for the
  Empire), as this trace predicts.
- `0x20` (the briefing's fleets display, `FUN_004c0fc0` code 5) is Idle
  Fleets.

## The port (2026-11-17)

`rebellion-render` `gid_fleets` counted the live world: every enemy fleet
with a move order to the system (Fleets Enroute), and every fleet in the
system's list, including the ones in hyperspace that had left it (Idle
Fleets), at any system, seen or not. So it drew enemy marks and 10160 the
original cannot. Now:
- the player's own fleets: en route by move order, idle in orbit and not
  in transit (a staged spare is in no system's list);
- the enemy's: none en route; idle the live fleets where the player sees
  the system (`object_views::system_seen`, or `system_sight` Live), else
  the frozen copy's fleets (`SystemDetails`), else none;
- the debug "see everything" mode counts every live fleet.

port, not kept: the pushes copy no enemy fleet in hyperspace
(`SystemExtras::of` keeps fleets in orbit only), so the port never draws
an enemy en route mark or 10160 on Fleets Enroute. The holder of the
`(0, 0)` case is the live system's, not the copy's.

## Open

- The container part of a copy's whole-sync (vtable `+0x24`) during a
  push; whether `FUN_004f7e30`'s second loop walks the copy's own children
  (and evicts those whose master left) is inferred from `FUN_004fca30`.
- Whether a fleet with `HyperdriveActive` (`+0x50` `0x800`) skips bit 5 on
  a move (`FUN_00556390` tests it), which would carry a live enemy copy
  into the destination; no case of it is traced.
