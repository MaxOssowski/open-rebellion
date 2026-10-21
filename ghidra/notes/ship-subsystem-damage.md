# A capital ship's subsystem damage (read 2026-10-21)

A capital ship (vtable `0x0065d650`) keeps six 4-bit hit counters in `+0x64`.
Each has a setter that checks its range (`FUN_0053fc90`, message
"Invalid ...CHAllocated value") and notifies through a slot:

| Bits | Counter | Setter | Reader | Tactical field |
|---|---|---|---|---|
| 0..3 | ShieldRechargeRateCHAllocated | `FUN_00501510` (slot `+0x2a0`) | `FUN_0040d6b0` | `+0x370` shield generators |
| 4..7 | WeaponRechargeRateCHAllocated | `FUN_005015a0` | `FUN_0040d6c0` | `+0x37c` weapons |
| 8..11 | TractorBeamPowerCHAllocated | `FUN_00501640` | `FUN_0040d6d0` | `+0x378` tractor beam |
| 12..15 | SpeedCHAllocated | `FUN_005016e0` | `FUN_0040d6e0` | `+0x36c` sublight engines |
| 16..19 | PrimaryHyperdriveModifier | `FUN_00501780` | `FUN_0040d6f0` | `+0x374` hyperdrive (with the next) |
| 20..23 | BackupHyperdriveModifier | `FUN_00501820` | `FUN_0040d700` | |

## Into and out of a battle

- **Into** (`FUN_0040b0e0` -> `FUN_0040b8a0` -> the tactical constructor
  `FUN_005b05c0`): each counter becomes the unit's hit count
  (`param_1[0xdb..0xdf]` = `+0x36c..+0x37c`). The hyperdrive's count is
  primary + backup. The hyperdrive component count `+0x3ec` is the number of
  nonzero CAPSHPSD `hyperdrive` and `hyperdrive_if_damaged`
  (`tactical-subsystem-damage-path.md`).
- **The record** (`FUN_005b0280`, read by `FUN_00596870` as 11 words): `[0]`
  id `+0xec`, `[2]` `+0x128`, `[3]` `+0x12c`, `[4]` hull `+0x3cc` (0 in
  state `0xb`, at least 1.0 otherwise, `DAT_0066c488`), `[5]` `+0x36c`,
  `[6]` `+0x370`, `[7]` `+0x374`, `[8]` `+0x378`, `[9]` `+0x37c`, `[10]`
  the state `+0x170`.
- **Out** (`FUN_0040c780`'s last walk -> `FUN_0040cfc0` per capital ship
  `0x14..0x17`; `FUN_0040d170` per Death Star; `FUN_0040d510` per fleet
  squadron):
  1. The hyperdrive hits `[7]` go first to the primary, up to 1 if the class
     has a `hyperdrive` (`FUN_00500680`: slot `+0x1ec` != 0). The rest go to
     the backup, up to 1 if it has a `hyperdrive_if_damaged` (`FUN_00500690`,
     slot `+0x1f0`). Hits left over stop the write.
  2. Hull: `[4]` (`__ftol`) less (full hull slot `+0x248` less current, slot
     `+0x274`) adjusts the damage: more damage through `FUN_00501ae0(n, 0xc)`,
     less through `FUN_00501b70`.
  3. The counters: shield `[6]`, weapon `[9]`, tractor `[8]`, speed `[5]`,
     primary, backup.

## Speed

`FUN_00500820` (slot `+0x34`), with object `+0x50` bit `0x40`:

- speed = class `hyperdrive` (slot `+0x1ec`, CAPSHPSD `+0x6c`) less
  `FUN_005011f0`;
- `FUN_005011f0` = primary counter x `hyperdrive` / 1 (`FUN_0053e170(h,
  n, 1)` = n x h / 1), so a hit primary gives 0;
- if 0: class `hyperdrive_if_damaged` (`+0x70`) less `FUN_00501230`, the
  same over the backup counter;
- if still 0 and `+0x40` byte 1 is 0: the default (`FUN_004f63f0`, GNPRTB
  1).

So a ship whose primary hyperdrive was hit in battle travels at its backup
rating, and with both hit at the default speed.

## Repair

Each repair fire (`ship-repair.md`, `FUN_00501c50`) lowers one damaged
counter by one, chosen by a draw (`FUN_0053e290`), after mending a tenth of
the hull.

## The port (before)

- `ShipInstance` keeps only `hull_current`. A battle's subsystem hits are
  lost, every ship reads as undamaged (`movement::capital_ship_speed`), and a
  battle starts with no hits.
- The AI's records leave out weapon and shield recharge damage
  (`ai_records::ship_record`, "port:").
