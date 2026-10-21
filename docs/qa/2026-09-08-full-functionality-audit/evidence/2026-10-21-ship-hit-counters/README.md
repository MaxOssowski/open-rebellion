# A capital ship's hit counters (F-032, D2, save v103)

Date: 2026-10-21. The reading is in `ghidra/notes/ship-subsystem-damage.md`
(commit `4e19d7b5`).

## Change

- `ShipInstance::subsystem_hits` keeps the ship's `+0x64` word, with six
  4-bit hit counters (`world::Subsystem`): shield, weapon, tractor, speed,
  primary and backup hyperdrive.
  - It replaces `shield_weapon_packed`, a byte with only the first two
    counters, which only the bombardment's shield read.
  - Save v103.
- A battle starts from the counters (`BattleSession::new_for_fleets`,
  `FUN_0040b8a0` -> `FUN_005b05c0`); the hyperdrive's count is primary plus
  backup.
- It writes them back with the hull (`ShipDamageEvent::hits_after`,
  `TacticalShip::strategic_hits`, `FUN_0040cfc0`), in the abstract battle
  and after a tactical one. The hyperdrive's hits fill the primary, then
  the backup, one each.
- `movement::ship_speed` is `FUN_00500820`: a hit primary leaves the
  `hyperdrive_if_damaged` rating, and both hit leave the GNPRTB 1 default.
  Fleets (`fleet_speed`) and the AI's ship records use it.
- Repair does not yet lower the counters (D3).

## Golden

The seed-42 replay golden is regenerated for save v103 (fingerprints only):
`v1:0c1f7f5edba92404` -> `v1:c117ed614a9dd901`.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against HEAD `4e19d7b5`. 20 campaigns change (`campaign-totals.txt`):

- battles 2080 -> 2146;
- Alliance wins 782 -> 733;
- Empire wins 1298 -> 1413;
- capital ships lost: Alliance 18 -> 13, Empire 9 -> 8.

The causes are the counters: a battle now starts from earlier hits, and a
ship with a hit hyperdrive travels slower.

## Gates

- 5 new tests (the counters' bits, the speed rule, the battle round trip
  with the hyperdrive sum, the hyperdrive split, the write-back) and one
  for the changed-ship rule. With the changes neutralized, 3 fail.
- Workspace 2563 passed, 0 failed; replay goldens pass.
- Mutants: 2 survivors remain, both equivalent.
  - `|` -> `^` in `set_hits` acts on masked-out bits.
  - `> 0` -> `>= 0` on a ship's speed in the AI records: `ship_speed`
    never returns 0, because GNPRTB 0 reads as the shipped 100.
- `provenance-scan`: 0 uncited after `Subsystem::shift` was cited.
- Browser, Arch Chromium 155.0.8059.84: Fleet window 58 of 58; replay
  equivalence passed; frame timing p99 0.8 / 1.6 ms, 0 frames over 50 ms.
- Release WASM: `b6b3f5b37c98d31b220f2a80d98fe8858d2ae9f90c1b0745ac1a5540e57ee471`.
  It was rebuilt after the `ship_changed` refactor. The campaigns rerun
  after it are identical, 24 of 24.
