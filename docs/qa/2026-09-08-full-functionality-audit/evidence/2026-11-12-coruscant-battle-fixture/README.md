# The Battle at Coruscant rebuilt for TAC-04-C014 (2026-11-12)

TAC-04-C014 (chase camera) was partial: its A0 is the Alliance's Battle at
Coruscant, and no fixture rebuilt that battle. This adds the fixture and a
browser case that compares the F1 frame with A0. The ledger JSON is not
changed.

## A0 used

| Capture | SHA-256 | What it shows |
| --- | --- | --- |
| `interface-parity/a0/TAC-04-C014/original` | `8f2c212e346124ac51f16c8893e82886c36a91f2cd3363d4943c2bc2d683ab0a` | Battle at Coruscant, paused. Navigation sets 1-4 on, Red Group escorting the carrier, then F1. The camera sits behind Task Force 1, and the red nav pyramids fill the foreground. |

The manifest (`new-game-alliance-small-2026-10-24-coruscant2`) gives the
sides. Fleet 2 has one Alliance Escort Carrier, one X-wing squadron and one
Y-wing squadron. It moves from Averam onto Imperial Coruscant. The manifest
names the Imperial defenders only in part: the Judicator in Fleet 3, plus
TIE squadrons (TAC-02-C004, TAC-05-C009 notes). No new capture was taken.

## The fixture (tactical fixture 43)

`crates/rebellion-app/src/tactical_test_fixture.rs`, codes 65835 (Alliance)
and 66091 (Empire):

- The system is Coruscant. Its own fleets stay. The defender is the
  Imperial fleet the galaxy placed there: one Imperial Star Destroyer and
  a TIE Fighter entry of 6 squadrons. port: this home fleet stands in for
  the day-2 save's Fleet 3, whose full roster is not recorded.
- The attacker is a new Alliance fleet: one Alliance Escort Carrier and
  X-wing and Y-wing entries of 1 squadron each (`coruscant_chase_fleets`).
- Coruscant is held by the Empire, and the Alliance arrived last.
- The battle enters through `tactical_flow::begin_player_battle`, as the
  other fixtures do. `main.rs` is not touched.

The battle the port builds:

- Layout (`FUN_005ab650`): 3 Alliance objects against 1 Imperial object,
  extent 109. The six TIE squadrons are listed but not active.
- Initial projection: the carrier at (30.4, 427.9) and the Star Destroyer
  at (368.9, 134.7). A0 TAC-05-C011 and TAC-04-C003 show the same battle
  before F1, with the carrier at the bottom left near (30, 428) and the
  Star Destroyer near (370, 132). This was judged by eye and is not
  asserted.

Tests:

- `code_43_enters_only_the_coruscant_chase_battle_on_either_side`. Before
  this change the decode test asserted that `0x1012b` decodes to nothing.
- `the_coruscant_chase_sends_an_escort_carrier_with_x_and_y_wings_against_coruscants_fleet`
  is ignored by default because it needs `data/base`. It was run with
  `--include-ignored`.

## The browser case: `production-coruscant-chase`

`tools/interface-parity/run.mjs --battle`, probe
`probeTacticalCoruscantChase`. On the paused battle the probe does what A0's
trace does, except Recover then Cancel and the two-point nav path:

1. Clicks Navigation Point Set 1 to 4.
2. Clicks the Red Group button (359, 10).
3. Right-clicks the carrier where the scene drew it. The app logs
   `[tactical_escort] event=assigned … fighter_members=1`.
4. Presses F1. The app logs `[tactical_camera] command=follow
   kind=task_force group=1`.
5. Rests the pointer at A0's (300, 465).

Requirements:

- The Alliance fleet is one Alliance Escort Carrier and two active
  squadrons.
- The layout and participant expectations listed above hold.
- The escort and follow logs are present.
- At 640 x 480 for the Alliance, the frame is compared with A0 outside the
  view (17, 30, 440 x 438), outside the text and outside A0's pointer. No
  pixel may differ, except one palette entry that reads (194, 158, 149) in
  A0 and (238, 142, 142) in the port. That entry covers 1 pixel in the
  group bar's ship icon and 1 in the X-wing portrait.
- The nav pyramids in the view are measured but not asserted.

Masked text: the task-force button's "1" glyph, the panel's three lines,
and "X-wings: 1 12/12". The port's orders line reads "Escort Alliance
Escort Carrier" without A0's trailing " 1", which A0 itself cuts at the
panel's edge. This is the accepted text deviation.

## Results

| Region | Pixels checked | Differ |
| --- | --- | --- |
| Group bar (0, 0, 468 x 24) | 11,076 | 1 (the palette entry) |
| Right panel (468, 0, 172 x 480) | 75,000 | 1 (the palette entry) |

| Chase view: red nav pyramids (4-connected, at least 20 px) | A0 | Port |
| --- | --- | --- |
| Count | 6 | 9 |
| Total area (px) | 3,778 | 373 |
| Largest | 1,969 px, box (29, 380)-(96, 436) | 53 px, box (292, 194)-(300, 202) |

The 3D view does not match. In A0, F1 puts the camera behind the carrier,
and the pyramids nearest the camera are large in the foreground. In the
port, F1 only re-aims the orbit camera at the task force's centre
(`update_tactical_camera_follow` → `focus_target`). The carrier is
centred, the view keeps its old distance and direction, and the holocube
lines stay visible.

The original's chase (traced statically from `~/Games/rebexe-decompile`):

- `FUN_005c2990` default case (mode 0xb): builds class `0x66d478` through
  `FUN_005d9a70`, with step 0.5, front clip 0.02 and field 0.1.
- Its frame step, `Class_0066d478_vtc` (`0x005d9b10`):
  - It sets the camera frame's position in the followed object's own frame
    to (-e, e, 2e) (`+0xf4`), where e is the larger of the object's box
    width and height (`+0x2c` box).
  - With a second object, it takes the offset along the vector to that
    object (`_DAT_0066d458` threshold) and looks at it (`+0x90`).
  - Without a second object, it sets the orientation along the object's
    own -z with up +y (`+0xf0`).

The port does not implement this.

## Run

Linux, Arch Chromium, `--no-build --battle
--scenario=production-coruscant-chase`. The run id is in UTC. This scenario
runs only the 640 x 480 viewport, once per side.

- `2026-11-12T12-27-11-053Z-1313516`: 2 of 2 pass.
- WASM SHA-256: `7d731c4c8a841b4574176e628ed7b94bab75d53c6589fcb8391ec65d1a3683ee`.
- Port F1 frame (`coruscant-chase-f1.png`, Alliance): SHA-256
  `bcb9348a2d5fa4bf84c31c817df3fb3ba71f9c5d5f83a70a84969f63f4598574`.

Earlier runs in the same session failed on the generic expectations this
work then added for the new scenario: the layout of 3 against 1, 10
participants, families 2050/2560, and 2 camera states. The earlier runs
are `12-23-22-229Z`, `12-24-33-236Z`, `12-24-57-822Z` and `12-25-22-681Z`.
Run `12-25-43-386Z` passed before the escort step and the text masks were
added; it had 35 and 8 differing pixels, mostly the masked text.

## Checks

- `cargo check --workspace`: pass.
- `cargo test -p rebellion-app`: 159 passed, 0 failed (4 ignored). With
  `--include-ignored tactical_test_fixture`: 3 of 3 pass.
- `rustfmt --check` on the fixture file: the same 5 baseline diffs as HEAD,
  none in the new lines.
- `cargo mutants -p rebellion-app --file
  crates/rebellion-app/src/tactical_test_fixture.rs --re
  'coruscant_chase_fleets|decode'`: 62 mutants, 60 caught, 1 unviable, 1
  missed. The missed one is `&&` to `||` in the defender filter. It is
  equivalent on the original data, because Coruscant's only fleet is
  Imperial and has a hull.

## Verdict for TAC-04-C014 (first stage)

TAC-04-C014 stayed partial after the first stage. A0's battle was rebuilt,
and everything outside the 3D view matched A0. But the view F1 produced did
not match: the port's F1 was a look-at, not the original's chase camera
behind the group (`0x005d9b10`). The second stage below ports that camera.

## Second stage: the chase camera

### The port

`crates/rebellion-render/src/tactical_assets.rs`:

- `original_chase_view` is `Class_0066d478_vtc` (`0x005d9b10`) without a
  second object.
  - e is the larger of the X and Y spans of the object's box.
  - The eye is at (-e, e, 2e) in the hull's frame. That frame is the one
    `0x005d3500` builds: x is (-Oz, 0, Ox), y is +Y, z is -O.
  - The camera faces the hull's (0, 0, -1), its heading, with up +Y.
- `OriginalTacticalCamera` holds the chase. While a chase runs, `pose`
  uses it with front 0.1 and field 0.02 (`0x005d9b10` and
  `FUN_005d9a70`). That is the free camera's angle (field 0.2 at front
  1.0).
- A memorized view never holds the chase (`camera_state`). Command 8 stores
  the free camera, and commands 7 and 10 destroy the chase (`+0x644`).
- `participant_box` gives the box: the same base + 1 member box as the
  collision envelope (`FUN_005d26c0`, `FUN_005ab0e0`).

`crates/rebellion-render/src/tactical_view.rs`, in
`update_tactical_camera_follow`:

- F1-F8 run the chase behind the task force's lead hull.
- port: the lead is the group's first live hull. `FUN_005aafc0` returns
  the group's `+0x6c` object, or a commander's flagship (`FUN_00583c40`),
  which is not traced.
- port: the group's second object (vt `+0x44`) is not traced. The camera
  always takes the no-target branch, which is SetPosition (-e, e, 2e) and
  SetOrientation. The target branch, which offsets toward the second object
  and calls LookAt, is not ported.
- F9-F12 keep the former look-at, because fighter sprites have no mesh box.
- Leaving the follow (a camera control, recall, or a lost group) clears the
  chase. The free camera returns as it was before F1. The old F1 overwrote
  the free camera's target; this one leaves it alone.

### Tests

- `the_chase_camera_sits_behind_and_above_its_hull_and_looks_along_its_nose`
  covers five headings and boxes: e from X or from Y, the Z span ignored,
  a climbing hull's normalised side axis, and a vertical heading.
- `a_chase_replaces_the_orbit_view_until_it_is_cleared` covers front, field,
  the unchanged back plane, pose and target, and a memorized view staying
  free.
- `following_a_task_force_runs_the_chase_camera_behind_its_lead_hull`
  (`tactical_view.rs`):
  - F2 passes over a dead, a retreating, an enemy and another task
    force's hull to reach the lead. The chase id is 10_002.
  - F9 keeps the look-at, and leaving the follow clears the chase.
  - With the chase branch disabled, it fails with "F2 runs the chase
    camera" (shown).
- The other two do not compile without the change.

### Browser

Final runs on WASM `b26d1c6b2e1f3f56eda6c90ee244bd750ddeeae4955d918abbdb9171967e9b9a`,
built from the committed code:

- `production-coruscant-chase`, run `2026-11-12T13-37-10-859Z-2048409`:
  2 of 2 pass. The numbers below are from this run. They are identical to
  the earlier run `13-10-23-272Z-1652659`, made before the follow function
  was restructured.
  - The F1 frame (`coruscant-chase-f1.png`): SHA-256
    `9109c5797bba981eb0aba7444d9c0b7a836568814f4ca1b41f8ecfec1b835ce2`.
  - The free-camera frame (`coruscant-navigation-sets.png`): SHA-256
    `239fcfac5a9e686311647d3d66cda703c1b818adb0ba1eace15dbad446ab4e85`.
- `production-navigation-camera-presentation`,
  `2026-11-12T13-37-25-571Z-2051450`: 2 of 2 pass.
  - Its first run under the chase (`13-09-00-197Z`) failed one count. It
    logged 3 fighter scenes where 1 was expected. The chase brings the
    camera close, so the squadrons' sprites change detail (`0x005d4af0`).
    The journey's manual exit then changes them back.
  - The expectation is now 3 for that scenario and 2 for the Coruscant
    chase. The logs show Indicator, then Close, then Indicator.
  - Its own chase assertions (aperture changed, manual exit changed it
    again, follow and follow_exit logs) passed unchanged.
- `production-group-presentation` (F12, fighter group),
  `13-37-41-761Z-2054484`: 2 of 2 pass.
- `production-selected-navigation-presentation`, `13-37-58-361Z-2057791`:
  2 of 2 pass.
- Checks: `cargo test -p rebellion-render` 1496 passed, 0 failed.
  `cargo check --workspace` passes. `provenance-scan check` reports 0
  uncited items. `rustfmt --check` gives the same diff count as HEAD on
  both files.
  - One cargo-mutants baseline, which ran the whole crate, failed on
    `status_window::tests::the_marks_are_painted_under_the_picture_at_its_origin`.
    That test is untouched and passed in three later runs, so it is flaky.
    The mutants runs below are filtered to the `tactical` tests.
- `cargo mutants -p rebellion-render` on both files, `--re 'in
  update_tactical_camera_follow$|in original_chase_view$'`, run on the
  final tree:
  - 188 mutants, 76 caught and 112 missed.
  - All 69 mutants of the two functions are caught.
  - The 112 missed are all field deletions of `Camera3D`,
    `DrawTextureParams` and `PipelineParams` literals in drawing functions
    outside the regex. They need a live draw loop and are untouched by this
    work.
  - Earlier rounds found mutants that had survived: the box span, the
    side-axis normalisation, the back plane, the lead filter, the group id,
    and the follow's centres. Tests were added for each. The old task-force
    arm, which the first version left unreachable, was folded into one
    member filter.

The A0 compared:

| Capture | SHA-256 | What it shows |
| --- | --- | --- |
| `interface-parity/a0/TAC-04-C001/original` | `f3a794effebeb3111af7b7779e5609f36747bf12a841f8d7642e4a09292289af` | The same battle, free camera, the four navigation sets on |
| `interface-parity/a0/TAC-04-C014/original` | `8f2c212e346124ac51f16c8893e82886c36a91f2cd3363d4943c2bc2d683ab0a` | F1 |

What the probe asserts, now that the chase is ported:

- **Free camera against TAC-04-C001.** Every A0 pyramid of at least 20
  px, red or grey-white, has a port pyramid of its colour within 2 px.
  Result: 34 pyramids, worst 1.0 px. The port draws the pyramids larger,
  about 1.45 times A0's area (1.2 times linear, for example 54 px against
  35 px). So areas are not compared. This size difference predates this
  work.
- **Chase against TAC-04-C014.** Every A0 pyramid of at least 100 px has a
  port pyramid of its colour within 40 px. This tolerance is coarse. It
  confirms the chase view and its direction, not its exact distance.
  Result: 10 pyramids, worst 34.5 px. The others are 15 to 24.4 px off.
  Before the port, the largest red pyramid was 53 px against A0's 1,969
  px. Now it is 2,293 px.
- Outside the view, nothing changed: 1 and 1 pixels differ, both the known
  palette entry.

### The remaining gap: the camera's distance

The F1 view now shows what A0 shows: the carrier ahead, the red line of
pyramids running down to the bottom left, the four grey-white pyramids
above, and the X-wing and Y-wing at A0's (85, 400) and (282, 400). But the
port's pyramids sit 15-35 px from A0's.

Method of the fit:

- A temporary log gave the port's navigation-point positions and the
  carrier's box. The box spans are x 3.855 and y 0.617, so e = 3.855
  (family 2051).
- A projection model reproduces the port's own chase frame.
- Nine A0 pyramid centroids were fitted with three separate multipliers
  for the eye's (-e, e, 2e) offset. Field and front were kept.

Results:

- The best fit is (0.80, 0.80, 1.60) x e, a uniform 0.80 (grid step 0.05).
  Mean error 1.3 px, worst 4.1 px. With the port's e, the mean error is
  22.6 px.
- The original's e is therefore about 3.08. No member of the carrier
  family gives that. The spans are 3.855 for 2050 and 2051, and 3.412 for
  2052.
- So the box `0x005d9b10` reads (the followed object's vt `+0x2c`) is not
  the mesh box the port uses, or that object carries a scale the port does
  not apply. This is not traced.

The fitted factor is not put into the code. It is a fit to the one capture
the probe compares with, so it could not count as evidence.

## Verdict for TAC-04-C014

TAC-04-C014 is still partial, closer than before.

What matches A0:

- The battle, the free view (34 pyramids within 1 px of TAC-04-C001), and
  everything outside the view.
- The chase's direction and composition.

What does not:

- The chase camera's distance from the hull is about 1.25 times A0's.
  Pyramids land 15-35 px off.

To pass, the cell needs the box `0x005d9b10` reads traced (the tactical
object's vt `+0x2c`). The chase test should then tighten from 40 px to a
few pixels.
