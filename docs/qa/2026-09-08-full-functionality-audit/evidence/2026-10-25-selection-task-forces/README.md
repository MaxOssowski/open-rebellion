# Orders turn the selection into task forces (P24)

Date: 2026-10-25. This settles how the original applies an order or a
tactic to what the player has selected. The trace is in
`ghidra/notes/tactical-collision-formation.md`, "Orders and tactics from the
selection".

## The original

The Maneuvers commit (`FUN_005dd3a0`) sends the order and the tactic to the
current input state (`DAT_006bc464`, slots `+0x20` and `+0x28`). There
are three states:

| State | Order and tactic |
|---|---|
| a task force chosen with its group button (`0x66bc48`) | the force takes them (`0x005a3630`, `0x005a3740`) |
| one unit clicked (`0x66bce8`) | `FUN_005aded0` first gives the unit a force of its own; the order plays the generic assignment line `0x77`/`0xf5` (`0x005a1a60`, `0x005a1ac0`) |
| several units (`0x66bc98`) | see below |

With several units (`0x005a2c70`, `0x005a2a40`):

- `0x0059f610` looks for the first free group slot of eight, before any
  unit leaves its force.
- With a free slot, `0x005ade70` gathers the units into one new force and
  `0x005a03f0` stores it in that slot. The task-force state follows
  (`0x005a33d0`).
- Without one, each unit gets a force of its own, and `0x84`/`0x102`
  plays.

A force of its own holds no group slot. `0x005a0300` then finds none, and
the voices take each family's generic line (`FUN_005a5cf0`: `0x55`/`0xcf`
for a kill, `0x62`/`0xdc` for a loss).

The ship's two force fields are one force:

- `+0x178` is set at once (`0x005cb770`);
- `+0x174` is set on the force's event `0x30` (`0x005cc2a3`).

The manual agrees: orders go "to your capital ships individually or by
task force" (p. 147).

## The port

- `TACTICAL_UNGROUPED_TASK_FORCE` (`u8::MAX`) marks a ship's force of its
  own. Every such force holds that one ship.
- `BattleSession::task_force_selection` records a task force chosen whole.
  - The group buttons and keys set it (`select_task_force`).
  - A click on a ship or on empty space clears it.
  - Stepping to the next or previous ship of the force keeps it.
- `assign_selected_command`:
  - a chosen force: its members still in the fight take the order;
  - otherwise `form_selected_task_force` runs first. One unit goes into
    a force of its own. Several form a force in the first free slot (and
    that force becomes the selection), or each goes into one of its own.
- `FUN_005ad530`'s withdrawal now also puts each ship into a force of its
  own (`FUN_005aded0`).
- Voices:
  - the capital families take their generic line for a force of its own;
  - one unit's order plays the generic assignment line;
  - several units with no free slot play `0x84`.
  - `hyp:` the maneuver-complete and orders-complete lines of such forces
    use the generic line too, as the kill and loss lines do.
- The group bar and "selected task force" ignore forces of their own. The
  abstract strength count (`tactical_side_withdraws`) treats each as its
  own task force (`AbstractGroup::own_force`).

## Tests

- `maneuver_confirm_commits_and_cancel_preserves_selected_capital_orders`
  committed a maneuver for one clicked ship and expected task force 1's
  formation line. On this evidence it now expects a force of its own and
  the generic line `0x77`.
- New:
  - `a_chosen_task_force_takes_the_maneuver_as_a_force` (the formation
    line stays for a chosen force);
  - `selected_units_form_a_task_force_in_the_first_free_slot` (and, with
    none free, forces of their own and `0x84`);
  - `a_force_of_its_own_holds_no_group_slot`;
  - `a_chosen_task_force_orders_only_its_members_in_the_fight`;
  - `a_single_unit_s_order_voice_ignores_unselected_ships`;
  - `a_force_s_completion_waits_only_for_its_own_members`;
  - core: `a_ship_outside_the_group_slots_is_a_force_of_its_own`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2801 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed |
| Replay equivalence | passed |
| `cargo mutants` | 99 tested: 82 caught, 11 unviable, 6 missed, one of them then caught |
| Release WASM SHA-256 | `9cad204be03fcf53775e03821e2833c7e990f09a0ed056e083970a1d6da12912` |

The missed mutants:

- `capital_group_offset` with `group <= 7`: the same as `< 7` (7 maps to
  7 either way).
- `< 8` to `<= 8` in the group bar and `assign_selected_to_task_force`: a
  `task_force` is 0..7 or `u8::MAX`, never 8.
- `refresh_original_attack_targets`, the enemy test of the completion
  check: the new test's enemy did not share force 0. With it pinned there,
  the mutant fails the test (run by hand).
- `handle_combat_input` and `handle_placement_input` emptied: these are
  the mouse handlers. The browser matrix covers them.
