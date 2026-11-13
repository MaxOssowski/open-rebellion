# Tactical impact-effect path

The saved `REBEXE.EXE` Ghidra project establishes the target-attached tactical
effect dispatcher and its frame lifecycle.

## Event and resource mapping

`FUN_005a7500` registers the ship-hit messages in this order:

| Message | Dispatcher code |
|---|---:|
| `SHIP_TAKE_LASER_HIT` | 1 |
| `SHIP_TAKE_ION_HIT` | 2 |
| `SHIP_TAKE_TURBO_HIT` | 3 |
| `SHIP_TAKE_TORPEDO_HIT` | 4 |

`FUN_005d3e90` combines that code with the post-hit stage:

| Stage | Weapon branch | State bit | Type-303 base | Frames | Draw size |
|---|---|---:|---:|---:|---:|
| Hit | ion | `0x04` | 3180 | 6 | 32 by 32 |
| Hit | other | `0x08` | 3060 | 6 | 32 by 32 |
| Damage | ion | `0x10` | 3240 | 16 | 64 by 32 |
| Damage | turbolaser | `0x40` | 3300 | 7 | 32 by 32 |
| Damage | other | `0x20` | 3120 | 7 | 32 by 32 |
| Destroyed | all | `0x80` | 3360 | 16 | 64 by 64 |

The dispatcher rejects an incoming state whose bit value is not greater than
the active state. States in `0x04`/`0x08` are mutually exclusive, as are states
in `0x10`/`0x20`/`0x40`. Destruction can replace a lower state.

## Timing and rendering boundary

`FUN_005d39a0` preloads the exact bases, frame counts, and draw sizes.
`FUN_005d41a0` keeps a counter at `+0x140` and adds one to it on every call
(`0x5d437c`), whether or not the image changes. It swaps the image to
`base + counter` on the first call and then only on a call that finds the
`+0xf0` timer past `0.1` s (`0x5d422d`, the float at `0x0066d154`), and
resets the timer there. The call that finds the counter at the stated count
removes the effect (`0x5d4301`). With the destroyed bit (`0x80`) set, the call
at counter 11 turns off both field loops (vtable `0x24` with 1 and 2) and calls
vtable 4, `FUN_005d3180`, whose vtable `0x30` (`FUN_005d3420`) deletes the
hull's mesh visual; the fireball runs on to 16.

The counter therefore counts scene renders, not tenths of a second.
`FUN_005c25b0` (scene vtable `0x24`) calls `FUN_005c2c80`, which calls
`FUN_005d41a0` for every effect. `FUN_0059a530` (the tactical idle step from
`FUN_0040a050`, called on every pass of the `PeekMessageA` loop in
`FUN_00413560`) busy-waits in `FUN_005c1490`/`FUN_005c1cf0` until the scene
timer at `+0x660`, reset at the start of each render, passes `0.02` s (the
float at `0x0066c954`). At that cap a call comes just over 20 ms after the
last, the fifth call after a swap is the first past 0.1 s, and the destroyed
sequence shows 3360, 3365, 3370 and 3375 over 16 renders (about 0.32 s). A
machine slower than 25 ms a render swaps after fewer calls and shows more of
the frames; the effect still lasts 16 renders. The renders, and so the
effects, also run while the battle is paused: `FUN_0059a530` calls
`FUN_005c1470` whether or not it stepped the battle.

The flight path is a different retained-mode object. Vtable slot 8 at
`FUN_005d3de0` constructs it through `FUN_005ee590`; it is not one of these
type-303 frames. Open Rebellion therefore retains its bounded procedural beam
until that geometry path is recovered. The two looped families at 3520 and
3620 are also left open because their shield, tractor, or gravity event
semantics are not yet authoritative.

This note records conclusions only. The saved Ghidra database, generated
decompilation output, and proprietary resource bytes remain untracked.
