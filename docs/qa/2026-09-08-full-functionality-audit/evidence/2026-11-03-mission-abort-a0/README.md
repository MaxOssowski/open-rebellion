# A mission icon's Abort in A0 (2026-11-03, open)

This is for surface ledger cell OBJ-12 C007, "aborted". The cell stays partial.

A0 ran the Alliance save at day 308–309 (Windows 10 VM). The captures are `.artifacts/a0-vm/personnel-states/37-*` (ignored); the hashes are in `a0-sha256.txt`.

- **`37-sullust-mission-menu-abort-grey`.** The menu of the mission icon over Sullust offers Encyclopedia, Status (grey) and Abort (grey).
- **`37-mon-calamari-mission-menu-abort-highlighted`.** Mon Calamari's mission icon menu offers Abort enabled, and it is highlighted under the pointer.
- **`37-after-abort-no-question`.** After Abort is chosen, the menu closes and no question opens.
- **`37-mon-calamari-missions-window`.** The icon still opens "Mission at Mon Calamari", with two Facility Design Research missions and Adar Tallon.

## Reading

`FUN_00487cc0` asks for order `0x250` (case `0x250` returns 1), but only after the order's own check, `(*order + 0x1c)`, passes. So A0 most likely refused this Abort without a word.

The port enables Abort only when every mission named passes `FUN_00521600` (`MissionState::abort_allowed`, phase 8). If these missions are not in phase 8, the port would grey the item where A0 shows it enabled.

## Open

- What enables the Abort row: the order's `+0x18` in its base class (`FUN_0051fa20`'s vtable) is untraced.
- What A0 does after a refused Abort.
