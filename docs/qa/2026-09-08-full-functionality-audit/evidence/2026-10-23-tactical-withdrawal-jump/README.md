# The player's withdrawal: exit flight and jumps (P24)

Date: 2026-10-23.

## Source

- `FUN_005af5f0` builds the battle's two commanders with a fixed vector at
  `+0x48`: (0, 0, 1) for side 0, the Alliance (vtable `0x66c428`, slot
  `+0x04` `FUN_005a5b60`), and (0, 0, -1) for side 1, the Empire
  (`0x66c3f8`, `FUN_005a66f0`).
- `FUN_005cc300`, a capital-ship task force's step (slot `+0x14`), on
  order 3:
  - It heads the force for that vector times -1.0 (`_DAT_0066cff8`) and
    half (`_DAT_0066cffc`) the battle extent `+0x8cc` (disassembly
    `0x5cc3b1`..`0x5cc3e4`).
  - After the side's time (`FUN_005a8ad0` not above 0.0), a unit with a
    working hyperdrive, still in the fight, starts its jump (state 6,
    event `0x2d`) when `-((exit_z - z) * side_z) <= 8.0` (`_DAT_0066cff0`,
    `0x5cc46f`..`0x5cc487`).
- The jump itself:
  - Event `0x2d` (`FUN_005b1e40`) gives a unit with a working hyperdrive
    state 7; one without returns to the fight (`FUN_005b2720`).
  - `FUN_005a6b40` runs the jump until its clock reaches 4000 ms
    (`_DAT_0066c24c`). Event `0x2e` then removes the unit (`FUN_005a90c0`,
    state 9).
- `FUN_005c8780` keeps the side in state 3 while any of these jumps is
  under way.

## Change

- `BattleSession::withdrawal_exit_z`: each side's exit line.
- `begin_player_withdrawal`: a withdrawing ship drops its order and
  attack (`FUN_005a8d30`).
- `advance_original_tactical_movement`: each step, a withdrawing ship gets
  a waypoint on its exit line (same x and y) and flies there. Before, it
  stood still.
- `advance_withdrawal`: when the time ends, the player's fighter groups
  with a hyperdrive leave, as before (s52). Each withdrawing ship then
  starts its jump once within 8.0 of its line and leaves 4000 ms later.
  A ship that lost its hyperdrive returns to the fight. The time stays 0
  until the last jump ends. Before, every withdrawing ship left together
  the moment the time ended.
- `TacticalShip::retreat_progress` is the jump's clock (it was unused).
- Not ported, and recorded in `ghidra/notes/abstract-battle.md`:
  - the jump's drawn flight (four unread factors);
  - the exit flight of a ship without a hyperdrive, which the port keeps
    fighting;
  - the fighter groups' flight (`FUN_005c6610`);
  - the AI side's own withdrawal (`FUN_005c8780` runs `FUN_005ad530` for
    it; the port's AI side still leaves at once).

## Gates

- Workspace: 2714 passed, 0 failed. Three tests that assumed everyone
  left the moment the time ended changed with this evidence:
  - The core time test and the core carrier test place the carrier on
    its line and step through the 4000 ms jump.
  - The render carrier test also gives the enemy a large hull and the
    enemy group no weapons, so the battle outlasts the jump. A group bound
    for the carrier that stays may now dock on it during the jump
    (`FUN_005cf980`).
- New tests:
  - `each_side_withdraws_to_its_own_edge`;
  - `a_withdrawing_ship_drops_its_attack_and_heads_for_its_exit`;
  - `after_the_time_a_ship_jumps_only_near_its_exit`;
  - `only_the_withdrawing_units_leave`.
- Mutants on the touched core and render code:
  - The first run tested 52 and missed 6. New tests closed 5 of them.
  - One remains, `* side_z` replaced by `/ side_z`. It is equivalent
    because `side_z` is always 1 or -1.
- Provenance scan: 0 uncited items. The new constant names
  `FUN_005cc300`.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `b64232d2651bd8ce832c5f38eaaffce68964d44aebd85235003f2af8db1f3d94`.
  The tests and one doc comment changed after this build; the code it
  builds did not.
