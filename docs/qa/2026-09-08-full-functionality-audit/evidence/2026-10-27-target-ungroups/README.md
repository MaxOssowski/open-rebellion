# A target given to one ship takes it out of its task force (P24)

Date: 2026-10-27. `ghidra/notes/tactical-collision-formation.md` left open
whether a single unit's order takes it out of its numbered group. It said
this "needs a run of the original".

## The run

The original ran in the Windows VM, scenario `save_res1`, "Battle at
Yavin" (see `../2026-10-27-enemy-panel-a0/`):

1. "Victory Destroyer 1" was selected alone; its panel read
   "Task Force # 1", and the top bar showed groups 1 and 2.
2. A right click on the enemy "Haste" gave it a target, the white box.
3. The panel now read "Task Force Unassigned", and group 1's button had
   gone from the top bar.

The captures are in `.artifacts/a0-vm/2026-10-27-controls/`:

| Capture | SHA-256 |
|---|---|
| `own-selected-victory-destroyer.png` (before) | `ecb56e4f9e47201ff1f2fccefd353091e9525329af31a9380b0b1cd4399dfec4` |
| `right-click-target-box.png` (after the right click) | `43f70d81ec362f9c01cb74a246108e6af1424b8c922d9e39ef72a187aeb87ae8` |
| `after-target-task-force-unassigned.png` | `43f70d81ec362f9c01cb74a246108e6af1424b8c922d9e39ef72a187aeb87ae8` |

The last two captures are the same frame. A Ctrl click on the enemy came
between them and changed nothing; Ctrl did not reach the game through
QMP, so the Ctrl clicks remain untested in the original.

This agrees with the one-unit state's slot `+0xc` (`0x005a1af0`). For an
enemy unit it gets the unit's force through `FUN_005a0c50`, and when that
gives 0 it gives the unit a force of its own (`FUN_005aded0`) before the
target (`FUN_005a8fc0`).

## The port

`assign_selected_focus_target` in `crates/rebellion-render/src/tactical_view.rs`:
- when one ship of ours is selected and no task force is, it first gives
  that ship no group (`form_selected_task_force` with one member). The
  one-unit orders already did this.
- Space reaches the same function, as the original's Space reaches slot
  `+0xc`.
- A selected task force keeps its group.

## Tests

- `a_target_takes_a_single_ship_out_of_its_task_force`

## Gates

This stage was gated and committed together with its sibling
(`../2026-10-27-target-ungroups/`, `../2026-10-27-nav-point-selection/`).

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2970 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 14 tested: 6 caught, 3 unviable, 5 missed |
| Release WASM SHA-256 | `c4b24215eb059730952c8d56b463bad14c77db1f9d8acd6c5e3bd1d49c4672ac` |

The five missed mutants are all in `draw_tactical_navigation_points` and
`handle_combat_input`, which draw or read the mouse through a live
macroquad window:
- either function replaced with `()`;
- the selected-point comparison in the drawing;
- the button test (`&&` against `||`);
- the `!control` filter of the click.

The rules themselves are in `navigation_point_color`,
`select_navigation_point` and `assign_selected_focus_target`, and tests
catch them.
