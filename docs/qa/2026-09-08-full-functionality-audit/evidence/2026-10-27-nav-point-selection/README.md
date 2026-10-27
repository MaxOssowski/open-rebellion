# A left click selects a navigation point (P24)

Date: 2026-10-27. The original has a selection state for a navigation
point (`0x66bb58`). The port had none: a left click on a point fell
through to empty space and only cleared the selection.

## The original

- **Statics.** A left click without Ctrl first posts `FUN_00595b70(0, 0)`,
  which clears the selection (`FUN_005c2060`). It then posts the point.
  The empty state's slot `+0x10` (`0x005a0e10`) builds the point state
  (`FUN_005a0e70`, marker `FUN_005c5ca0`). That state's slot `+4`
  (`0x005a55f0`) gives the camera target button the point.
- **The A0 VM run of 2026-10-27, "Battle at Yavin".**
  - The set 1 button shows set 1's points in dark red, (148, 6, 0).
  - A left click on a point turns it purple, (128, 0, 128).
  - The unit selection ends, and the panel goes back to the side's
    emblem.

| Capture (`.artifacts/a0-vm/2026-10-27-controls/`) | SHA-256 |
|---|---|
| `nav-set-1-shown.png` | `4c72a8a485b3c6f9304f293e2a5e8a4edf867e16eaf00a327d2e1020c88c7835` |
| `nav-point-left-click.png` | `55b31e32259c13791387e57dac82a98262e192a80e7cf809029c88306ea20004` |

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- `TacticalState.selected_navigation_point` holds the selected point.
- A left click without Ctrl on a point, with no ship under the pointer,
  runs `select_navigation_point`. That ends the unit selection and
  selects the point.
- Any other left click drops the selected point, and so do the group
  buttons (keys 1 to 8 included).
- The selected point is drawn purple, (128, 0, 128).
- The camera target button (and Enter) looks at the selected point when
  no unit is selected (`TACTICAL_NAVIGATION_CAMERA_ID`). The point does
  not move, so it is not tracked.

Not exact:
- port: a ship under the pointer is taken before a point, because the
  port has no pick depth. The original takes the first pickable visual
  that `FUN_005c2d20` finds.
- port: the sets keep the port's colours; only the selected point uses
  the original's purple.

## Tests

- `a_selected_navigation_point_ends_the_unit_selection_and_aims_the_camera`
- `the_selected_navigation_point_is_purple`

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
