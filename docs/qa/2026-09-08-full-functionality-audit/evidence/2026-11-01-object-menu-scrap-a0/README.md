# Scrap from the object menu (2026-11-01)

## A0

The original's own-unit menus list Scrap below Encyclopedia and Status and
enable it for the player's regiment (`11-troop-menu.png`), fighter squadron
(`20-fighter-menu.png`), battery (`22-battery-menu.png`) and facility
(`28-fac-menu.png`). On an enemy object it is greyed (`02-menu.png`). The
captures stay in `.artifacts/a0-vm/status/`; only their hashes are here
(`a0-sha256.txt`).

The order is 0x200 (FUN_00487cc0). FUN_0049a350 case 0x200 asks with RCDATA
0x7050 "Are you sure you want to scrap the following units?" and one 0x7054
"\n{1:1}" line per unit (FUN_0049a880), over picture 0x408/0x409.
See `ghidra/notes/scrap-order.md`.

## Change

- `object_menu`: the Scrap row is enabled when the object maps to a scrap
  target and `rebellion_core::scrap::check` allows it.
- `move_confirmation`: a Scrap confirmation with the 0x7050 text, the unit
  names and pictures 1032/1033; the checkmark returns
  `MoveConfirmationAction::Scrap`.
- The app runs `scrap::scrap_team` on the checkmark and drops cargo left
  without transport; X leaves the units alone.
- `scrap::target_name` names a ship by its own name, otherwise by class.

## Result (Linux Chromium)

`fleet-window.mjs --only=alliance/scrap --only=empire/scrap` passes
(`gate-fleet-window-scrap.txt`): the regiment's menu shows Scrap at row 4,
enabled; X keeps the regiment; the checkmark removes it.

With the row forced off the same gate fails with `scrap_enabled:false`
(`gate-fleet-window-scrap-row-off.txt`). The full gate suite passes.

## Mutants

- `object_menu.rs`: 42 tested, 40 caught, 2 unviable.
- `move_confirmation.rs`: 17 tested, 16 caught, 1 unviable.
- `scrap.rs --re target_name`: 5 tested, 5 caught.

## Open

No pixel comparison of the menus against the A0 captures; the browser case
covers only the regiment. The OBJ-14 cells stay partial.

## A ship's Scrap (2026-11-02, OBJ-06 C013)

**A0.** `.artifacts/a0-vm/personnel-states/21-own-ship-menu-scrap` (Alliance, DS_flip_allian, day 185) shows the menu of the player's ship Swift, in Fleet 4 at Mon Calamari. Its rows are Move, Confirmed Move, Create Fleet, Rename, Encyclopedia, Status and Scrap, and every row is white.

**The browser.** The new `ship-scrap` case of `fleet-window.mjs` uses the fleet-join fixture. It right-clicks the player's first ship in orbit and asserts the same seven rows, all enabled. Then it chooses Scrap, sees the confirmation and presses the checkmark. It waits for the log line `command=0x200 destination=scrap targets=[Ship … result=Ok`.

Run `fleet-window-2026-11-02T22-54-31-639Z-1256992` (Arch Chromium, both sides, test WASM `283d1246…`): pass.
