# Ctrl clicks and empty clicks follow the original mouse handler (P24)

Date: 2026-10-27. This follows `../2026-10-27-fighter-group-routes/`. The
manual says: "To add a ship to an existing task force, select the task
force, hold down the CTRL key and then click on the ship to add". The
port only toggled the ship in a plain multi-selection. A trace of the
battle window's mouse handler found three more differences:
- a Ctrl click on an enemy ship replaced our selection, where the
  original adds a target (the s139 rule was wrong here);
- a Ctrl right-click on our own ship gave Escort, where the original
  changes the selection;
- a right click on empty space cleared the targets, where the original
  does nothing.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

### The battle window, `FUN_005c2060` (`Class_0066c960_vt14`)

- **`WM_LBUTTONDOWN` (`0x201`):** the unit under the pointer comes from
  `FUN_005c2d20`, probed at the point and four diagonal neighbours.
  - No unit: nothing with Ctrl (`MK_CONTROL`, 8); otherwise
    `FUN_00595b70(0, 0)`, which clears the selection.
  - A unit with Ctrl: `FUN_00595b70(unit, 1)`.
  - A unit without Ctrl: the frame is cleared, then
    `FUN_00595b70(0, 0)` and `FUN_00595b70(unit, 0)`.
- **`WM_RBUTTONDOWN` (`0x204`):** the same probe, with eight more points
  2 pixels out.
  - No unit: nothing.
  - A unit: `FUN_00595b70(unit, Ctrl)`.

`FUN_005a04f0` passes each posted unit to the selection state's slot
`+0xc` together with the Ctrl flag, so a Ctrl left click and a Ctrl
right click on a unit are the same. Unit 0 without Ctrl goes to slot
`+0x18`, the empty click.

### Slot `+0xc` with Ctrl, on a capital ship

| State | Enemy ship | Our ship |
|---|---|---|
| one unit `0x66bce8` (`0x005a1af0`) | target added or dropped | joins: several units (`FUN_005a2280`) |
| several units `0x66bc98` (`0x005a24d0`) | target added or dropped | joins or leaves the selection |
| task force `0x66bc48` (`0x005a3aa0`) | target added or dropped | see below |
| fighter group `0x66bbf8` (`0x005a4bd0`) | target added or dropped | escort dropped when held (`FUN_005ca310`, order 0), else Escort |
| enemy `0x66bba8`, none `0x66be30` | a new selection by side | a new selection by side |

For our ship in the task-force state, the cases are:
- the force escorts that ship (`FUN_005ca310`): the escort is dropped and
  the order becomes 0;
- the ship is not in the force (`FUN_005ca2f0`): it joins
  (`FUN_005ade50`);
- the ship is a member: it leaves (`FUN_005a8d30`). When the force has no
  member left, the selection ends (`FUN_005a0c80`).

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- `ctrl_click_ship` holds the table. A Ctrl left click and a Ctrl right
  click on a ship both reach it.
- A Ctrl left click on empty space keeps the selection.
- A Ctrl left click on a navigation point runs the same code as a Ctrl
  right click, `click_navigation_point`: it drops a point already on the
  route and adds any other one. Before, it could only drop points. The
  empty state's slot `+0x10` (`0x005a0e10`) and the router show the two
  buttons reach the same slot.
- A right click on empty space no longer clears the selection's targets.
- port: a ship that leaves a task force gets no group (it is ungrouped).

Not ported:
- **A click on a squadron in the 3D view.** `FUN_005c2d20` picks through
  the D3DRM viewport (`+0x70`) the first visual whose object is marked
  pickable (`+0x34`). `FUN_005a04f0` then sends every unit object (type
  1, squadrons included) to the unit states above. Whether a squadron's
  object is pickable is not traced. The port selects squadrons only
  through the group buttons.
- **A left click on a navigation point.** It builds the point's own
  state `0x66bb58` (`FUN_005a0e70`, marker `FUN_005c5ca0`).

## Tests

- `a_ctrl_click_on_an_enemy_adds_and_drops_a_target_of_our_ships`
- `a_ctrl_click_on_an_enemy_targets_it_for_the_fighter_group`
- `a_ctrl_click_on_an_enemy_without_our_selection_selects_it`
- `a_ctrl_click_with_a_task_force_moves_ships_in_and_out_of_it`
- `a_ctrl_click_with_a_task_force_drops_its_escort_of_the_ship`
- `a_ctrl_click_with_a_fighter_group_toggles_its_escort`
- `a_ctrl_click_on_our_ship_toggles_it_in_the_selection`
- `a_navigation_click_routes_only_our_selected_ships`
- `a_navigation_click_with_a_fighter_group_routes_the_group`
- `a_ctrl_click_escort_counts_only_the_selection`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2956 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | first run 144 passed, 4 failed (`production-group-presentation`) |
| Browser rerun | group, navigation-camera and selected-navigation scenarios: 12 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 50 tested: 43 caught, 7 missed. After the sharper tests, all 32 mutants of `ctrl_click_ship` were caught |
| Release WASM SHA-256 | `b84bbb1a58e7a25e061c285bf6e962dfa92f0c846842194459f57d409eab21c4` |

The first browser run failed because a Ctrl click on a ship logged only
`ctrl_click`. The probe waits for the selection line. A Ctrl click that
changes the selection now also logs `[tactical_3d] selection
object_id=...`, as a plain click does. The selection itself was already
right.

Two mutants survive, both in `handle_combat_input`:
- replacing the function with `()`;
- deleting `!` in `!control` for a click on empty space.

Both need live mouse input. The click rules sit in `ctrl_click_ship` and
`click_navigation_point`, and tests catch both.
