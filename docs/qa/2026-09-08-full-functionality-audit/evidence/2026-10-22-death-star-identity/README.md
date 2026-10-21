# The Death Star's identity (F-025, W4)

Date: 2026-10-22. The reading is in `ghidra/notes/tactical-death-star-path.md`,
"The Death Star's identity".

## Finding

- Every CAPSHPSD record carries an object family at `+0x10`. It is `0x14`
  for every record but 136 (`0x88`, the Death Star), which has `0x18`.
- The original finds a fleet's Death Stars by that family
  (`FUN_005003d0`, families `0x18..0x1b`; `FUN_0040b0e0` ->
  `FUN_0040bcd0`; the bombardment's `0x223`; the smuggling count
  `FUN_00509140`).
- Family `0x34` is the Emperor (MJCHARSD `0x34000280`).

## Change

- `CapitalShipClass::is_death_star` drops its `family() == 0x34` clause.
  The record id 136 stands for family `0x18`, because the port's class
  keeps the record id rather than the family. Keeping the family itself
  would be a save change, so it is not done here.
- The Fleet and victory docs no longer call the Death Star family `0x34`.
- Test fixtures that built a Death Star from id `0x34000001` now use record
  136: `bombardment.rs` ×3, `integrator.rs`, `fleet_window.rs` and
  `mission_dialog.rs`. Their assertions are unchanged.
- `only_the_death_star_record_is_the_death_star` now asserts that family
  `0x34` ids (the Emperor `0x34000280`, `0x34000001`) are not the Death
  Star. It fails with the old clause.
- The shield's battle role was already ported in save v96
  (`TacticalDeathStar::attackable`, `FUN_005ba450`).

## Gates

- Campaigns (24 seeds, 1500 days, dual AI) against HEAD `17dce37e`: 24 of
  24 identical. No shipped class has family `0x34`.
- Workspace: 2554 passed, 0 failed. Replay goldens pass; the only failure
  is the long-standing
  `seeds::tests::fixed_fleet_tables_only_target_special_systems`.
- Mutants on the diff: 3 caught (`mutants.txt`).
- `provenance-scan check`: 0 uncited.
- Browser, Arch Chromium 155.0.8059.84:
  - Fleet window 58 of 58;
  - replay equivalence passed (no errors, 4 requests);
  - frame timing p99 0.9 / 1.7 ms, 0 frames over 50 ms.
- Release WASM: `039ba80acaee43a6f4f5642f22243a45ec35c32c91b961f526cba7e603eecc14`.
