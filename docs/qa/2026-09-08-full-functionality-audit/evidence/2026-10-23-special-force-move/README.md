# A special force's Move and Confirmed Move (P08)

Date: 2026-10-23. The reading is in `ghidra/notes/move-order.md` and
`character-retire.md`.

## Original

The special-force class lists the unit orders (`FUN_00536bc0`):
- Move `0x201` and Confirmed Move `0x202`, which expand to the Move
  mission `0x241`, as a character's do;
- the trip's days come from `FUN_004f4850`, at the default speed;
  `FUN_00548370` speeds up only Han Solo;
- `FUN_0049a350` gives the confirmation and the blockade warning for a
  first member that is not a fleet.

## Change

- `TargetOrder::SpecialForceMove`.
- `OrderGates::special_force_move`: the player's unit.
- `PanelAction::MoveSpecialForce`: the Move mission with the unit as its
  team.
- `missions::member_move_days`, which `move_days` now calls.
- The confirmation lists the unit's class name and days. Its checkmark
  carries the unit (`MoveConfirmationAction::Confirm::special_forces`).
- Before this change the port drew a unit's Move rows disabled.

port: the port's fleets carry no special forces, so a release on a fleet
sends the unit to the fleet's system.

## Gates

- Tests: 2 new (a unit's trip days; its menu rows and gate). Workspace
  2677 passed, 0 failed.
- Mutants: 19 of 20 caught at first. The miss, `&&` to `||` on the
  unit's Confirmed Move, is now caught: the test checks that the unit's
  gate opens no character's Confirmed Move. Then 10 of 10 on the menu
  file.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- No browser probe walks a unit's Move yet.
- Replay equivalence passed.
- Release WASM `523ff9220f2ce471a3aa1ce9f2466eb032e73ad21f7aac7b96a3884d00ec2c29`.
