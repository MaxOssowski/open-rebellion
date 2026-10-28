# The Fleet, System, Defenses and Missions windows move by their surface (P06)

Date: 2026-10-28. The user reported that the Fleet and planet windows could
not be dragged, while the original's can. The reading is in
`ghidra/notes/fleet-window.md`, "Moving the window".

## The original

- `FUN_00606650` gives a left press on the window itself (`0x201`) to
  `FUN_00606ee0`, which holds the window when `+0xb8` has bit 4.
- The base dialog `FUN_004ac120` leaves `+0xb8` at 0. Four planet windows
  then set bit 4:
  - the Fleet window, `FUN_004a2630` (`0x4a26c4`);
  - the System window, type 9, `FUN_00452fc0` (`0x4530da`);
  - System Defenses, type 10, `FUN_004a7790` (`0x4a77f9`);
  - Missions, type 11, `FUN_0049f130` (`0x49f1cf`).

  The GID legend (`FUN_00452090`) does too; it was ported on 2026-10-22.
  The sector window keeps its own press (`FUN_004593e0` case `0x201`) and
  does not move.
- Buttons, lists and tabs are child windows, so a press on one goes to
  that control.
- While the button is held, the window follows the pointer with no bounds
  (`FUN_006071e0`; `FUN_00606380` sets `+0xf4` to 1). On release,
  `FUN_00607010` moves it back inside the galaxy view's rect
  (`FUN_006071a0`). That is the same rect the window opened in.
- Manual p. 63, Fig. 3.1: "Use the bar at the top of the window to move
  the window around on the screen. Drag the window by grabbing it by the
  bar". The title bar is part of the window's own surface, so this matches
  the code.

## The port

`crates/rebellion-render/src/window_move.rs`:
- `held_travel` measures how far the pointer has moved since the press on
  the window's background. The background now senses drags. The move does
  not start when the press began on a control. Each window passes its
  control rects:
  - its title buttons;
  - its lists;
  - its tabs;
  - the System window's bands.

  Once a move is under way, the controls are no longer checked: the moved
  window would put another control under the press point.
- `follow` moves the window from where the press found it. On the drop it
  applies `clamp_window_to_galaxy`, the bounds the window opened in.

The Fleet, System, Defenses and Missions windows keep that anchor. A press
that starts a move brings the window to the front, as a click does.

## Tests

- `window_move::tests::a_held_window_follows_the_pointer_and_drops_inside_its_bounds`
- `the_window_moves_by_its_surface_and_drops_inside_the_galaxy_view` in
  `fleet_window`, `system_window`, `defenses_window` and `missions_window`:
  - a press on the title strip moves the window;
  - a move past the galaxy view's left edge holds it there, and the drop
    puts it on the edge;
  - a press on a list or band moves nothing.

  The System window's test also checks that hovering takes no focus.

The Fleet test fails with the background back on `Sense::click()`, at
`(20, 30)` against `(50, 50)`. The System test failed before the anchor
rule above, at `(55, 80)` against `(30, 80)`.

## Browser

Arch Chromium 155.0.8059.84, not the pinned macOS build, run through a
temporary Linux manifest. Interface test WASM SHA-256
`d011d9f3886a35c7eb58e3ef1f9438dd2a7389a802b8f6e355360781c4edc13e`.

- `fleet-window.mjs` has a new case, `move`, for both factions:
  - a drag on the title strip moves the window;
  - a drag past the left edge drops it at the galaxy view's x (55 for the
    Alliance, 120 for the Empire);
  - a drag that starts on the left list leaves it in place.
- Captures (ignored), `.artifacts/interface-parity/fleet-window-2026-10-28T14-07-43-679Z-2622709/`:

  | Capture | SHA-256 |
  |---|---|
  | `alliance-move/moved.png` | `624ef8140fe4ca28f7aa3b76a84cd16b2e045877203a3c832d42b20e96fa24e1` |
  | `alliance-move/dropped.png` | `a9e039247381f7a37fddc894ebe5365a095f62467def21fe6e804bde33cba2e0` |
  | `empire-move/moved.png` | `1a099103d34714d7471e588cd9cf65bc1cd0b0541c0113bea95691d46074800b` |
  | `empire-move/dropped.png` | `1a9c656e4c9cbe4e9d1a3d08172b35ef94885c892901923ff7074565a89260dc` |

- Full `fleet-window.mjs`: 60 of 60 cases passed. Run
  `fleet-window-2026-10-28T14-11-32-550Z-2679651`.
- `sector-quadrants.mjs` (the Defenses and Missions windows): 12 of 12
  passed. Run `sector-quadrants-2026-10-28T14-14-46-593Z-2691704`.
- `build-selection.mjs` fails on 6 of 28,646 pixels of `opened`. HEAD
  `16cb3c95`, built in a separate worktree without this change, fails the
  same way, so the failure predates this change.

## Gates

- Workspace: 3021 tests passed (5 new). Provenance scan: no uncited
  items.
- Mutants (`rebellion-render`, the diff):
  - The first run missed `&&` to `||` in the System window's focus test,
    which would focus the window every frame. The test now checks that
    hovering takes no focus.
  - Second run: 37 mutants, 30 caught, 7 unviable.
- rustfmt: the new file is formatted. The other four files have only their
  baseline differences.
- Release WASM SHA-256 `81c9f83d467f08f9318109e530127d4c863dbcb95eb4d5b83fc23fb0d9004a8b`.
