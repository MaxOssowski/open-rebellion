# Order voices follow the force's route and targets (P24)

Date: 2026-10-27. Before this stage, three order voices were tagged `hyp:`.
The port played them on every click without Ctrl, and on a Ctrl click
whenever the point or target was new. The original also asks two of the
force's lists, a test the port had not traced. The same trace showed that
several selected ships given a point form a task force, which the port did
not do.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`, outside
the repository), with objdump for the lists and the vtable slots.

## The original

### The force's two lists

- `FUN_005cb950`, which sets a target, clears `+0x98` and adds the target.
  So `+0x98` is the force's target list.
- `FUN_005cb920`, which sets a route, clears `+0x80` and adds the point.
  So `+0x80` is its route.
- In the force's constructor (`FUN_005c9db0`):
  - `+0x80` has class `0x66bdb8`, whose slot `+0xc` is `0x5f6590`;
  - `+0x98` has class `0x66cda0`, whose slot `+0x10` is `0x5cbca0`.
- Both slots jump through `0x568ee0` to `FUN_005f5060`, which returns the
  list's first item, or 0 when the list is empty. It removes nothing.
- So "slot == 0" means the list is empty.

### The voices

| State and slot | Click | The voice plays when |
|---|---|---|
| Fighter group `0x66bbf8` `+0xc` (`0x005a4bd0`), attack | no Ctrl | the route is empty |
| the same | Ctrl, a new target | the targets and the route are empty |
| Fighter group `+0x10` (`0x005a4e50`), route | no Ctrl | always |
| the same | Ctrl, a new point | the route is empty |
| One unit `0x66bce8` `+0x10` (`0x005a1f30`), task force `0x66bc48` `+0x10` (`0x005a3f00`), route | no Ctrl | always |
| the same | Ctrl, a new point | the route is empty |

### Several units given a point

`0x66bc98`'s slot `+0x10` (`0x005a3020`) takes no Ctrl argument. Given a
point:
- `FUN_0059f610` finds the first free group slot. If there is one:
  - the slot's line plays (`0x22` or `0x9c`, plus the slot);
  - the point goes to the new force (`FUN_005c5ca0`);
  - the task-force state follows (`FUN_005a34e0`).
- With no free slot:
  - `0x84` or `0x102` plays;
  - each ship takes a force of its own (`FUN_005aded0`) and the point
    (`FUN_005a8f20`).

## The port

In `crates/rebellion-render/src/tactical_view.rs`:
- `assign_selected_fighter_group_target` voices only with an empty route,
  and on a Ctrl click on a new target only with no target held.
- `assign_selected_fighter_group_route` and `queue_capital_route_voice`
  voice a Ctrl point only on an empty route.
- `route_selected_ships_as_a_force` runs `form_selected_task_force` for
  several selected ships. It plays the new slot's line, or `0x84`/`0x102`
  when no slot is free, and the point becomes the whole route.

## Tests

- `several_ships_given_a_point_form_a_task_force` is new.
- These now encode the traced conditions where they had asserted the old
  rule:
  - `a_ctrl_right_click_on_a_held_target_drops_it`;
  - `a_fighter_group_takes_navigation_points_as_its_route`;
  - `a_capital_route_plays_its_task_forces_line`.
- The first two fail with the old code.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2996 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` (diff) | 36 tested: 33 caught, 3 missed. The three sat in `route_selected_ships_as_a_force`'s filter; a case with a lost ship and an enemy also selected now catches them |
| Release WASM SHA-256 | `2f3e30ef00edd6222c719de6b43eef35b7c6be80711611b88bbbe4a1f2e60da4` |
