# Fighter groups follow navigation points (P24)

Date: 2026-10-27. This follows `../2026-10-27-fighter-group-targets/`.
The manual says: "To tell a ship or task force where to go, select the
ship or task force, then right-click on the destination navigation point
... right-click on each point in turn while holding down the CTRL key".
Its note names fighter groups too. The port gave routes only to capital
ships. A Ctrl right click on a point already on the route did nothing,
and no route played a voice.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

### A right click on a navigation point, slot `+0x10`

| State | Slot `+0x10` | Line without Ctrl |
|---|---|---|
| one ship `0x66bce8` | `0x005a1f30` | `0x21`/`0x9b`, the generic line |
| task force `0x66bc48` | `0x005a3f00` | `0x22`/`0x9c` plus the slot (`FUN_005a0300`) |
| fighter group `0x66bbf8` | `0x005a4e50` | `0x2a`/`0xa4` plus the slot (`FUN_005a02c0`) |

Each slot works the same way:
- **Without Ctrl:** the line, order 0 (the force's slot `+100`), and the
  point as the whole route (`FUN_005a8f70`, event `0x3a`).
- **With Ctrl:**
  - a point already on the route (`FUN_005ca330`) is dropped
    (`FUN_005a8ed0`);
  - any other point is added (`FUN_005a8f20`), with the line while the
    force's `+0x80` slot `+0xc` gives 0.

### The fighter force's route, `FUN_005c6610`

The order step of a fighter force (class `0x66bee0`, slot `+0x14`) works
the route `+0x74` before its targets:
- While the route has a point, the mover flies to it (slot `+0xc`,
  radius 1.0, `0x3f800000`) and the step ends there.
- Within that radius:
  - the point becomes the mover's anchor (`+0x3c`);
  - the point leaves the route (`FUN_005c6200`);
  - when the route is empty and the force is the player's, the group's
    "maneuver complete" line plays: `0x37`/`0xb1` plus its slot. Further
    tests of `+0x60` and `+0x98` guard it.
- Only with no route left does the step turn to the targets (`+0x8c`).

## The port

`crates/rebellion-core/src/tactical.rs`:

- `TacticalFighter.navigation_route` holds the force's route on each
  member.
- `BattleSession::advance_fighter_route` is the route branch:
  `step_fighter_anchor` with `ORIGINAL_FIGHTER_ROUTE_RADIUS` (1.0), the
  anchor and the pop on arrival, and the line when the player's route
  ends. Meanwhile the members fly to their places in the frame.

`crates/rebellion-render/src/tactical_view.rs`:

- `assign_selected_fighter_group_route` holds a fighter group's right
  click on a point.
- `queue_capital_route_voice` plays the capital lines.
- A Ctrl right click on a point already on a ship's route now drops it.
- The navigation display also marks the selected fighter group's points.

Not exact:
- hyp: the further tests on the lines (the force's `+0x80` on a Ctrl
  click, `+0x60` and `+0x98` on arrival) are not traced. The lines play
  without them.
- hyp: several selected ships (the several-unit state's slot, not read)
  use the generic line.
- port: as for the ships, the route is set at once, not through event
  `0x3a`.

## Tests

- `a_fighter_route_comes_before_its_target`
- `a_fighter_force_takes_its_route_point_and_then_its_target`
- `a_fighter_group_takes_navigation_points_as_its_route`
- `a_capital_route_plays_its_task_forces_line`
- `a_ctrl_route_click_drops_a_held_point_and_adds_another`
- `the_display_marks_only_our_selected_routes`
- `a_fighter_group_route_skips_lost_squadrons_and_other_groups`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2943 passed, 0 failed; 2946 after the mutant tests |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | first run 32 tested: 9 missed. After `toggle_route_point`, `selected_route_points` and the three tests: 38 tested, 35 caught, 1 unviable, 2 missed |
| Release WASM SHA-256 | `946238613ed598e1362cd312f0b034d26d8ae091dd45b5212783fcd2bcec81ce` |

The two missed mutants replace `draw_tactical_navigation_points` and
`handle_combat_input` with `()`. Both draw or read the mouse through a
live macroquad window, which unit tests do not have. Their rules sit in
`selected_route_points` and `toggle_route_point`, and tests catch both.

The browser matrix ran before those two functions were pulled out of the
drawing and mouse code. Their filters are the same.
