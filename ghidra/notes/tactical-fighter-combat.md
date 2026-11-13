# Tactical fighter combat

This note records the fighter-combat contract recovered from the owned English
`REBEXE.EXE` with SHA-256
`b3fe3997cab9a6e96403d638875dcba25484e4d8601751afec748471ac0ed6ab`.
It covers fighter construction, live hull and shield state, weapon output,
range, target modifiers, torpedoes, maneuver defense, and strategic squadron
transport.

## Recovered functions

| Function | Role |
|---|---|
| `FUN_0040be90` | Transports FIGHTSD values into the tactical creation call |
| `FUN_005964c0` | Allocates and registers one tactical fighter object |
| `FUN_005b9c60` | Fighter constructor and exact field initialization |
| `FUN_005b9c00` | Live weapon recharge from current hull (vtable `+0x60`) |
| `FUN_005b9c30` | Derives live shield recharge from current hull |
| `FUN_005b49e0` | The AI's strength estimate per family (called from `FUN_005edf80`); not a fire path |
| `FUN_005b3f10` | Emits weapon and shield-gated torpedo events in source order |
| `FUN_005b7780` | Scores one concrete target, including maneuver ratios |
| `FUN_005b7a80` | Scores a target class without a concrete object |
| `FUN_005b23a0` | Produces torpedo event strength |
| `FUN_005b1650` | Returns target shield ratio |
| `FUN_005b5100` | Laser hit and overflow damage (event `0x11`, slot `+0xd8`) |
| `FUN_005b5750` | Torpedo hit and overflow damage (event `0x14`, slot `+0xd4`) |
| `FUN_005b5b60` | Ion hit: shields only; past them, engine stun and arc knockouts (event `0x12`, slot `+0xdc`; `evidence/2026-10-26-ion-hits/`) |
| `FUN_005b5f50` | Turbolaser hit and overflow damage (event `0x13`, slot `+0xe0`) |
| `FUN_005b5700` | Direct floating hull damage path |

## Object and DAT transport

`FUN_005b9c60` constructs one tactical object per strategic fighter squadron.
All eight records in the owned `FIGHTSD.DAT` use only their fore battery. Their
aft, port, and starboard weapon counts are zero. Open Rebellion therefore
expands an aggregate strategic roster count into individual tactical
squadrons, while preserving the aggregate roster slot and a squadron ordinal
for the return path.

The constructor applies these exact scalar values:

| State | Formula or value |
|---|---|
| Maximum and current hull | `craft_count * 8.0` |
| Sublight power | `FIGHTSD.sub_light_engine * 0.4` |
| Initial weapon energy | sum of fore laser, ion, and turbolaser counts |
| Initial weapon readiness | true when the sum is positive |
| Weapon recharge per second | `current_hull * 3.751 / 8.0` |
| Shield recharge per second | `current_hull * 0.025 / 8.0` |
| Effective maneuverability | `raw + 1`, clamped at 9 |

FIGHTSD laser, ion, turbolaser, and torpedo ranges are direct tactical ranges.
The owned data confirms squadron size 12 for every original class.

## Weapon events and target modifiers

A fighter group fires through the same paths as a capital ship. The
autonomous search (`FUN_005b3a40`, vtable `+0x88`, the fighter tail at
`0x5b3ba0`) keeps a laser or turbolaser candidate of

```text
FUN_005b1770(weapon_count * family_coefficient * target_modifier)
```

that is, scaled once by current hull over maximum hull (`+0x3cc / +0x3c8`).
A direct target (`FUN_005b6530`, vtable `+0xa8`) offers
`weapon_count * target_modifier * family_coefficient` without the hull
scale. `FUN_005b3f10` fires the kept strength.

`FUN_005b9c00` (`current_hull * 3.751 / 8`) sits in vtable slot `+0x60`,
the slot of a capital ship's weapon recharge `FUN_005b0400`; it is the
group's weapon recharge per second, never a strength factor. `FUN_005b49e0`
multiplies by slot `+0x60`, but it is only the AI's strength estimate
(`FUN_005edf80`). Until 2026-11-13 the port took it for the fire path and
fired each fighter shot about 45 times too strong at full strength.

The shared family coefficient is the exact float `0x3e888889`. Ion cannons do
not fire at fighters (the search offers only lasers and turbolasers). Lasers
retain a 1.0 fighter modifier and use the exact `0x3e2b020c` modifier against
capital ships. Turbolasers use 0.5 against fighters and 1.0 against capital
ships. The source order is ion, laser, turbolaser, then the conditional
torpedo event.

Torpedo strength is `FIGHTSD.torpedoes * 0x3f088889`. The value is a combat
rating rather than a decremented ammunition counter. A fighter can emit the
torpedo event only against a capital target whose current shield ratio is zero
and only within `FIGHTSD.torpedoes_range`.

## Fighter damage and maneuverability

The laser, turbolaser, and torpedo receivers compare the target and source
effective maneuverability. The target-to-source ratio is clamped to at least
0.1, then incoming float damage is divided by that ratio. A more maneuverable
target therefore receives less damage. Ion damage against fighters is excluded
by the target modifier before dispatch.

Damage first subtracts from the floating shield pool. Negative shield overflow
continues into the floating hull pool with a minimum of one hull point at the
crossing. Live craft count is the ceiling of current hull divided by eight.
Zero hull removes the tactical object.

## Strategic return boundary

The strategic world stores whole squadrons rather than partial craft. The
return path counts each tactical squadron with positive hull, including a
successfully recovered squadron, back into its original aggregate roster slot.
Destroyed tactical squadrons do not return. Partial craft damage remains
tactical-only because the current strategic schema has no partial-squadron
field.

## Remaining boundary

The implementation preserves the recovered construction, coefficients,
target modifiers, ranges, event order, float hull and shield state, and typed
target paths. The executable applies `FUN_005b1770` through its global random
stream to some fighter values. Exact global RNG sequencing remains open for the
consolidated battle gate. Fighter collision, formation, authored arrival
callbacks, and lossless original visual comparison also remain open.
