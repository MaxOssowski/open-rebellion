# Withdrawing fighter groups fly to their edge and jump (P24)

Date: 2026-10-23.

## Source

`FUN_005c6610` is a fighter task force's step (slot `+0x14`, vtable
`0x66bee0`). On order 3, which `FUN_005ad530` gives a group with a
hyperdrive, it:

1. Heads the force for its side's exit line. The line comes from the
   tactical commander's vector (`FUN_005be160`) times -1.0
   (`_DAT_0066cb10`) and half (`_DAT_0066cb14`) the extent `+0x8cc`.
2. Waits while the side's time (`FUN_005a8ad0`) is above 0.0
   (`_DAT_0066cb18`).
3. Then starts the jump of each member that has a working hyperdrive
   (`FUN_005b1b70`) and is within 8.0 (`_DAT_0066cb0c`) of the line:
   state 6, event `0x2d`.
4. Each jump lasts 4000 ms (`FUN_005a6b40`), and event `0x2e` then
   removes the member.

A member that cannot jump returns to the fight (`FUN_005b2720`).

## Change

- `TacticalFighter::retreating` and `retreat_progress` (the jump's
  clock) are new fields.
- `BattleSession::withdraw_fighters` marks a group with a hyperdrive as
  withdrawing.
- `advance_withdrawing_fighters`:
  - flies the group toward its line along z, straight at its sublight
    rating per second, as on its way to a carrier (port);
  - after the time ends, starts its jump within 8.0 and removes it,
    craft kept, 4000 ms later;
  - returns a group without engine power to the fight.
- The side's withdrawal ends when no withdrawing ship or fighter group
  is left.
- Before (s52): every such group left at the very moment the time ended,
  wherever it was.

## Gates

- Workspace: 2720 passed, 0 failed.
- Three s52 tests assumed the instant departure. Their expectations
  changed on this evidence:
  - the core player test and the core AI-fighter test;
  - the render test, where the enemy group also gets a large hull and no
    weapons, so the battle outlasts the jump.
- New tests:
  - `a_withdrawing_fighter_group_flies_to_its_edge_before_it_jumps`;
  - `withdrawing_fighters_head_for_their_own_edge`.
- Mutants:
  - The first run tested 46 and missed 6 (direction, the Empire's side,
    the 8.0 test). The new edge test closes 5 of them.
  - One remains: `* side_z` replaced by `/ side_z`, which is equivalent
    because `side_z` is always 1 or -1.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `e960805c4c70f64c413b4f6ed01da0d1a0bfab4090eaf4209801b7b506413b4e`
  (the tests changed after it; the code it builds did not).
