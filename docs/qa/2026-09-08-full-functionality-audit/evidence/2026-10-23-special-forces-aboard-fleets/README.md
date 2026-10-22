# Special forces aboard fleets (P08)

Date: 2026-10-23. Save format v105, approved for the overnight run.

## The original

- A fleet's ships carry characters and special forces
  (`FUN_00536da0(ship, 3)`, the Fleet window's Personnel tab).
- A Move (`0x201` -> `0x241`) released on a fleet lands its member in that
  container (`FUN_004f7640` into `+0x74`), for a special force as for a
  character.
- When the container is destroyed, `FUN_00556710` calls the unit's slot
  `+0xb4`, `FUN_00503e20(0)`: a unit on no mission and not destroyed is
  set Autorouting (`FUN_004f78e0(1)`). It is not destroyed.
- The Personnel Finder's SpecForces view (`FUN_00464e10`) lists a system
  with units directly on it, then each fleet of the side there whose ships
  carry units, as a row of its own; `FUN_00465540` counts a fleet's units
  over its ships. This corrects the older note, which said fleets were not
  separate rows.

## The port

- `Fleet::special_forces` (save v105); a unit aboard is in no system's list.
- A special force's Move and Confirmed Move onto a fleet board it
  (`PanelAction::MoveSpecialForce { fleet }`, the mission's target fleet,
  `MissionEffect::MemberBoarded`). Its place follows the fleet.
- A merged fleet takes the units aboard; a fleet with units aboard does not
  disband; a destroyed fleet's units stand at its system. port: the
  autoroute away from another side's place (`FUN_0054c7d0`) is not ported
  for special forces.
- Fleet window: the Personnel tab lists the units after the characters
  (hyp: the order), lights for a unit alone, and a unit's right-click menu
  is a special force's (Move, Confirmed Move, Retire, Mission). port: a
  unit's drag is not ported (nor from the Defenses window).
- Personnel Finder: a fleet with units is its own row; Display opens the
  Fleet window.
- Special Force Status: "Attached:" names the fleet it is aboard.

## Golden

The seed-42 golden is regenerated for the save format change (v105): only
the state fingerprints differ, the commands and structure are unchanged,
and two runs agree. Initial `v1:06cd3db9594663cc`, final
`v1:d66014ea2b71379b`.

## Gates

- Workspace 2681 passed, 0 failed.
- Mutants on the touched logic: 49 tested, 40 caught, 8 unviable, 1 missed
  (`board_fleet`'s `&&` -> `||`), closed by a gone-fleet case in
  `a_special_force_boards_a_fleet_and_goes_where_it_goes`, which fails
  under that mutant.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed with the new golden.
- Release WASM `595cce41811180f501fe1d1d7d189c05ddaf9007c5b1ddaa7b316e4b3976820c`.
