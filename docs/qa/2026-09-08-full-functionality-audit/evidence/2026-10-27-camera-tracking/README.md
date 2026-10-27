# The camera keeps looking at its target (P24)

Date: 2026-10-27. This follows `../2026-10-27-camera-target/`. The port
looked at the camera target's position only at the moment the button was
pressed, and Enter with nothing selected kept the old target.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

The free camera is class `0x66d418`.

- **Command 9 (`0x005d97c0`).** It sets the target `+0x44` to the
  selection's object (`FUN_00595be0`) and clears the cached frame `+0x40`.
  With nothing selected, the object is 0.
- **The frame step (slot `+0xc`, `0x005d9640`).** It rebuilds the view
  each frame:
  - the yaw and the pitch (`AddRotation`, `+0x44`, combine 0 = replace);
  - the distance (`AddTranslation`, `+0x3c`);
  - the field.
  - Then, while `+0x44` is set, it caches the object's frame
    (`FUN_005c1080`, `+0x2c`) and calls the camera frame's `LookAt`
    (`+0x90`) on it. The view therefore follows the object as it moves.
- **What keeps the target.** The other commands change only the angles,
  the steps and the field, so turns and zooms keep it. Command 9 with
  nothing selected drops it: the replaced rotation looks at the origin
  again.
- **A memorized view.** It stores `+0x44` with the angles
  (`0x005d9900`), so recall (command 7, `FUN_005d95d0`) brings the target
  back.

## The port

- `crates/rebellion-render/src/tactical_assets.rs`:
  - `OriginalTacticalCamera::clear_target` drops the target and looks at
    the origin;
  - `target_object_id` exposes the target.
- `crates/rebellion-render/src/tactical_view.rs`:
  - `track_tactical_camera_target` runs each frame unless a chase camera
    runs. It moves the camera's target point to the object's current
    position (`tracked_target_position`: ships from id 1, squadrons from
    1001).
  - It looks at the drawn point: the step's pose blended by
    `step_accumulator`, as the scene draws the unit (`presented_pose`).
  - The camera target button and Enter clear the target when nothing is
    selected.
  - `apply_original_collision_envelopes` now runs before the camera
    update, so the camera looks at the participants' settled positions
    in the same frame, and the starfield and the scene share one camera.
- The memorize and recall buttons already stored and restored the target
  id with the camera state, so a recalled view tracks again.

Not exact: port: a destroyed target keeps its last point, because how the
original's cached frame behaves after a destruction is not traced.

## Tests

- `the_camera_target_tracks_its_object_by_id`
- `the_camera_follows_its_targets_drawn_position`
- `camera_command_nine_without_a_selection_drops_the_target` (in
  `tactical_assets.rs`)

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2962 passed, 0 failed (render 995 after the last tests) |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | first run 144 passed, 4 failed. Second run, after the fixes below, 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | first run 21 tested: 11 missed. Last run 24 tested, 17 caught, 4 unviable, 3 missed; then 3 tested on `track_tactical_camera_target`: 3 caught |
| Release WASM SHA-256 | `999919ff0271540dc6db3e869208664fb990c60f96ec612d21c3c8c3e0ba0c1f` |

The first browser run failed in two places:
- **`production-effect-presentation`** logged two effect scenes instead of
  one. The fixture aims the camera at ship 3 before the collision
  envelopes settle that ship from x = -5 to x = -9 in the first frame.
  Since the envelopes now run before the camera update, the camera looks
  at x = -9 from the first frame.
- **`production-death-star-presentation`** logged 709 camera states where
  one was expected. Its camera target is a squadron on the moving trench
  run, and the camera follows it as the original's per-frame `LookAt`
  does. The browser check now requires at least one camera state, all
  with one target id, instead of exactly one.

Three mutants survive:
- `index - 1001` against `index / 1001` in the squadron pose lookup is
  caught after the test moved to id 1003.
- `index < 1001` against `<=` in `tracked_target_position` is equivalent.
  Id 1001 is a squadron and is handled before that line.
- Deleting `!` before `update_tactical_camera_follow` in
  `draw_tactical_view` needs the live draw loop. The same filter is in
  `track_tactical_camera_target`, and tests catch it.
