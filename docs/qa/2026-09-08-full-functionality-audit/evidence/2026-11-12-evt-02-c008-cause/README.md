# EVT-02-C008: what disables Retreat at Sluis Van (2026-11-12)

The cell's note ends: the Alliance's Fleet 55 at Sluis Van (Freedom,
X-wing, A-wings) has no gravity well in the forces list, so the cause of
the disabled Retreat (`FUN_00516730`: a unit that can move, an autoroute
search) has not been compared.

**Verdict.** The captures cannot settle the cause, and no port change is
justified. The trace below narrows it to two candidates. The cell stays
partial. The ledger is not edited here.

## The rule, traced again

`FUN_004865b0` enables Retreat only when `FUN_004fcf50` returns non-zero
and its outputs are 0, 1 and 1. The call chain is `FUN_004fcf50` ->
`FUN_0051dce0` -> `FUN_00516730` on the battle's system (`+0x8c`). This
matches `ghidra/notes/abstract-battle.md`.

1. **Gravity well (output 1 must be 0).** For each enemy fleet at the
   system (`FUN_004ffef0`) and each of its capital ships (`FUN_00502e30`),
   the rule reads slot `+0x254` less slot `+0x280`. That is CAPSHPSD
   `gravity_well_projector` less 0.
2. **A unit that can move (output 2 must be 1).** The rule takes the first
   of the viewer's fleets that has a ship whose slot `+0x34` is non-zero.
   That slot is `FUN_00500820`, read as 0 unless the ship is Existing.
   Without such a fleet, it takes the first of the viewer's squadrons on
   the planet (`FUN_00503a50`) that can move. The unit kept is the **fleet**
   (`piVar4 = FUN_0052bed0(local_4c)`), not the ship.
3. **A destination (output 3 must be 1).** The call is
   `FUN_005545e0(autoroute, unit, unit's system, null +0x70 key, 0, 0,
   &dest, &status)`. With `param_5 = 0`, step 1 (the current container) is
   skipped. The search must return non-zero, `status[1]` must be -1 (a
   candidate was accepted), and the destination must be a system
   (`0x90..0x97`).

Every candidate step walks one row of the table at `+0x68`.
`FUN_0058b060` builds one row per system, listing every system with its
distance and sorted ascending, so the origin itself comes first at
distance 0. Step 4 (`FUN_00554f80`, the nearest system that holds a fleet
of the unit's side) therefore always offers the battle's own system when
the unit is a fleet.

Each candidate goes through `FUN_005571c0`. Its unit checks
(`FUN_00553410`, through the fleet's slot `+0x74` `FUN_004fe0d0` and the
capital ship's `FUN_00500ac0`) refuse:

- an object of another side (`0x24`, `0x28`);
- an unfinished object (`0x20`, `+0x50` bit 2 clear). A ship always gets
  this check. A fleet gets it only when it is not a system spare
  (`(+0x58 & 6) == 0`);
- a destroyed object (`0x22`);
- an object en route (`0x21`).

Conclusion: output 3 turns on the unit, not on the map. Whenever the viewer
has a fleet at the battle, the map always offers candidates.

## What the A0 captures show

`.artifacts/a0-vm/sluis-van-768/` (Empire viewer, day 768, Retreat drawn
disabled with 10973):

- **Freedom is a Mon Calamari Cruiser.** In `02-alliance`, the row's mini
  at (208, 172) matches GOKRES 18240 (`MINI_SHIP_MC80_LIBERTY_CRUISER`, the
  alert's mini for CAPSHPSD 64) in all 581 of its opaque pixels. The other
  minis tried (18242, 18243, 18250, the CC-7700's) match 82 pixels at
  most. "Freedom" is a ship
  name (TEXTSTRA 21185, in the bank from 21184 "Liberty" that the AI's
  Rename module uses, `ai-mission-planning.md` module `0xe`). In
  CAPSHPSD, class 64's `gravity_well_projector` is 0.
- **The Alliance list continues past the visible rows.** The scroll bar's
  thumb covers y 101..255, 155 of the 217-pixel track. The Empire list's
  thumb in `03-empire` covers 206 pixels, and the System Assets thumb in
  `04-assets` covers 104. With the port's view of 234 and rows of 40
  (`ALERT_LIST_HEIGHT`, `ALERT_LIST_ROW`), that gives about 328, 247 and
  488 pixels of content. The Alliance list is about two rows longer than
  the Empire's, and those rows are not shown. One CAPSHPSD Alliance class
  projects a gravity well: class 74, the CC-7700 Frigate, with
  `gravity_well_projector` 4 (dat-dumper on `CAPSHPSD.DAT`). A CC-7700 in
  the hidden rows would explain the capture.
- **Fleet 2** (Victory Destroyer 1, TIE Fighter, Imperial Fleet Regiment,
  Jerjerrod) is the Empire's fleet at its own Sluis Van, which carries an
  uprising mark (`07-sectors`). A ship order places its products in the
  system's bit-1 spare fleet as soon as the order is given, unfinished
  (`ship-deployment-fleet.md`), and a numbered fleet stays that spare.
  If Fleet 2 held an unfinished product, `FUN_00553410` would refuse every
  candidate with `0x20`. An unfinished Victory Destroyer would also have
  speed 0 (`FUN_00500820` requires Existing), which fails output 2. The
  captures do not show whether anything in Fleet 2 was being built.

The other A0 cases do not separate the candidates either:

- **Umgul, Alliance game, day 718** (disabled): the Empire's Gorgon in
  `battle-clean/22` is a Star Galleon Frigate. Its mini matches 18316 in
  all 1045 opaque pixels. CAPSHPSD 140 has no well. That list's thumb
  (98..274, 177 of 217) also leaves about two rows hidden.
- **Chandrila, Empire game, day 689** (enabled), has the same alert text
  as Sluis Van ("The Alliance fleet is threatening ..."). It has no
  captured forces lists.

## The port

`abstract_battle::retreat_offered` checks output 1 and output 2 as traced
above. For output 3 it uses `missions::nearest_place`, which leaves out
the battle's system and ignores the unit. Its 5 rules always find some
system in a real galaxy, as the original's search does whenever the
viewer has a fleet there. In the Sluis Van state as captured, the port
would offer Retreat unless the hidden rows hold a CC-7700.

The port differs from the original in two traced ways:

- the original's rows include the battle's system;
- the original refuses an unfinished, destroyed or en-route unit.

Neither is changed here:

- **The unfinished-unit refusal** has no counterpart in the port. Port
  fleets hold only finished ships; products wait in the manufacturing
  queue. Modelling unfinished products inside the spare fleet would change
  the architecture, and that needs the user's approval.
- **Including the battle's system** would change the existing assertion
  "With no other system, the search finds nowhere to go"
  (`retreat_is_offered_unless_an_interdictor_or_nowhere_to_go_bars_it`).
  That would need proof that `FUN_005571c0` accepts a route to the
  unit's own system. The decompile does not give that proof:
  - any refusal on its paths returns 0, and `FUN_00554e80` then stops the
    whole search;
  - the leg builder's `0x27` refusal (`FUN_00550700`, slot `+0x78`
    capacity) is unread for a system destination.

  Per CLAUDE.md, an expectation changes only with evidence, so the
  assertion stays.

## What would settle it

Capture the next Sluis Van-like alert with each forces list scrolled to
its end. If the enemy's list shows a CC-7700 Frigate or an Interdictor
Cruiser (CAPSHPSD 130, `gravity_well_projector` 4), the rule is compared
directly. Otherwise, open the viewer's fleet before the battle: an
unfinished member (the shipyard's Building band naming that fleet)
points to the `0x20` refusal.

## Hashes (sha256)

| File | sha256 |
|---|---|
| `sluis-van-768/01-alert.png` | `e35c2b5302d1b354fa67da335683336c9eed7f4d01f7b0357e20927845d4889e` |
| `sluis-van-768/02-alliance.png` | `5cc32031e21fbcbbff7490d868cf328214838908b2ca9907f6a8ff8332f0ce56` |
| `sluis-van-768/03-empire.png` | `0c84281d58da514ee1dad4834d7aa98d939046b0c716c0ded0a48f5fb8dcc234` |
| `sluis-van-768/04-assets.png` | `53079d57f48a59a90a462a0f37d9a6fef2a6b0e03fac24fa310ffce15add178c` |
| `battle-clean/21-umgul-alliance-forces.png` | `29986db2b470a676b88431d94f152d90be9a11b308b818b5bd641f2fcb79dd05` |
| `battle-clean/22-umgul-imperial-forces.png` | `c7ac78e4d2b695b6eedb68a92ddf5000b1317a12d1297399c5c9e37884783a9f` |
| GOKRES 18240 (`web/data/ui/gokres-dll/BMP/18240.bmp`) | `f2ebf6024e5762a9332dc9499cf251a0ae200a7f990ca9eb04736de3a416e256` |
| GOKRES 18316 | `06d9df7f8d70470fe0b8653ff27d0d389a8ffd5ebc1526efd1cb3852378adacc` |
| `data/base/CAPSHPSD.DAT` | `36fdaaeef4057d70e10a1593950cee5b197ce38d7d1b6a06201a74c93bf44515` |

## Read

Typed decompile: `FUN_00516730`, `FUN_004fcf50`, `FUN_0051dce0`,
`FUN_004865b0`, `FUN_00500820`, `FUN_005545e0`, `FUN_00554e80`,
`FUN_00554150`, `FUN_0058afd0`, `FUN_0058b060`, `FUN_005571c0`,
`FUN_005511d0`, `FUN_005513a0`, `FUN_00552ff0`, `FUN_00552d10`,
`FUN_00550700` (in part), `FUN_00553410`, `FUN_004fe0d0`,
`FUN_00500ac0`, `FUN_00506e60`, `FUN_004d29b0`.

No code changed, so no tests, mutants or provenance scan ran, and no
browser gate covers this case.

## Batch b199

The full gate batch b199 (2026-11-12 13:25-15:05) passed 28 of 30 gates, the workspace (3631 tests) and fixture tests (159), and 172 tactical battle checks, 0 failed. fleet-window (move-and-carry: the regiment did not board in time) and status-window (fighter: Status did not open in time) failed under a load average near 24 from three agents' builds; rerun alone they pass (fleet-window-2026-11-12T14-08-10-312Z-2322992 2/2, status-window-2026-11-12T14-08-44-749Z-2323466 whole gate).
