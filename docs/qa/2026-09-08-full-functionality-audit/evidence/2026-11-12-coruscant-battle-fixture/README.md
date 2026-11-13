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

## Third stage: the box the chase reads

This stage traces the value that the second stage left open. It is a static
trace only. No code changed, no WASM was built, and no browser gate ran. The
lead's next batch reruns the gate.

### The chain, from REBEXE

Sources: `~/Games/rebexe-decompile/decompiled-typed/FUN_<addr>.c` and
`objdump -d` of the GOG `REBEXE.EXE`.

1. `0x005d9b10` (`Class_0066d478_vtc`) calls `FUN_00595d00(+0x30)` for two
   ids. It looks up the first with `FUN_005c1080` (`FUN_005f5500`, an id
   map). That object is the followed hull.
2. At `0x005d9b79` it calls that object's vt `+0x2c`. For the hull class
   `0x0066d198` (built by `FUN_005d26c0`), that slot is `0x0066d1c4` ->
   `0x005d43b0` (`Class_0066d198_vt2c`). It returns `this + 0xcc`, with no
   copy and no scale.
3. `+0xcc` is written in one place: `FUN_005d26c0` at `0x005d2ae2`. That is
   `GetBox(this + 0xcc)` (vt `+0x40`) on the mesh builder at `+0x48`, the
   family's base + 1 member (`param_5 + 1`). The second `GetBox` at the end
   of that constructor is the same call on the same mesh. In
   `0x005d26c0..0x005d4400`, `objdump` finds no other store to
   `+0xcc..+0xe0` of the object. It finds only `lea 0xcc(%esi)` at
   `0x5d2ad7` and the getter at `0x5d43b0`.
4. The mesh builder comes from the hull cache's loader `0x005d5b10`
   (vtable `0x0066c990` + 4). It calls `Load` (vt `+0x2c`), then
   `Scale(3, 3, 3)` (vt `+0x34`, `DAT_0066d2b0`), before the box is read.
   So the box is the base + 1 vertex box times 3.
5. `0x005d9b80..0x005d9ba4`: e = `box[3] - box[0]` (the X span) unless
   `fcomps` sets C0 or C3. Then it takes `box[4] - box[1]` (the Y span).
   That is the larger of the two full spans. It is not a half span, a
   radius or a diagonal, and the Z span is not used. No constant multiplies
   it. e goes to `+0x34`, the eye is SetPosition(hull frame, -e, e, 2e)
   (vt `+0xf4`), and the eye looks along SetOrientation(hull frame, 0, 0,
   -1, 0, 1, 0) (vt `+0xf0`).

The port does the same:

- `decode_mesh_object` scales the vertex box by `ORIGINAL_HULL_MESH_SCALE`
  (3.0).
- `participant_box` takes family member 1.
- `original_chase_view` takes the larger of the X and Y spans.

For the Alliance Escort Carrier (base 2050), e = 3.855. The runtime
manifest's 2051 vertex box gives spans 3.855 x 0.617 x 5.304 after the
scale. **So the box is not where the 1.25 gap comes from.**

### Other places a 0.8 could come from, also checked

- **The hull frame's own scale.** `0x005d3500` (`Class_0066d198_vt1c`)
  builds the frame with `AddTransform(REPLACE)`. Its rows are:
  - the side axis `(-Oz, 0, Ox)`, normalised to `DAT_0066d17c` = 1.0
    (`DAT_0066d178` = -1.0, `DAT_0066d180` = 0.0);
  - `(0, 1, 0)`;
  - `-O`;
  - the position.

  A short `O` could only shorten the eye's Z offset. The fit shrank X and
  Y too, so it does not explain the fit.
- **The parent frame.** `FUN_005c1f10` passes the 3D window's `+0x138` to
  the hull's constructor as the parent of its frame. That frame is made by
  `CreateFrame(NULL, ...)` (`0x005d6ec0`), a scene root with no scale.
- **The projection.** The chase sets front 0.1 (`0x005d9b6a`) and field
  `+0x38` = 0.02 (`FUN_005d9a70`). The free camera (`0x005d9640`) sets
  field `+0x48` and then front 1.0. The ratio is the same. The chase's vt
  `+0x10` (`0x005d9d80`) changes `+0x38` only on a zoom step: x0.9, at
  least 0.005 (`DAT_0066d438`), or x1.1, at most 0.2 (`DAT_0066d43c`).

### What is still open

Two things in `0x005d9b10` are still not traced. Either could explain why
the original's eye sits closer:

1. **Which hull is followed.** `FUN_005aafc0` returns the group's `+0x6c`
   object. If the group's commander passes the `FUN_005a5fb0` 1..8 check,
   it returns that commander's flagship (`FUN_00583c40` `+0xec`) instead.
   The port takes the group's first live hull. A hull from another family
   gives another e.
2. **The second object** (the group's vt `+0x44`, read into `piVar4`). When
   it exists, the eye is (e·x/d, e·y/d, 2e). (x, y) is the second object's
   position in the hull's frame, from `GetPosition` (vt `+0x60`). d is the
   length of (x, y), and the threshold is `DAT_0066d458` = 0.001. The eye
   is then e·√5 from the hull, not e·√6, and it calls `LookAt` on that
   object (vt `+0x90`). The group class behind vt `+0x44` was not found in
   this pass.

The fitted 0.8 is still not put into the code.

### The two leads, traced (same day, 15:20-15:30)

This is a static trace again. No code changed, and nothing ran in a
browser.

**1. Which hull F1 follows: the carrier, in every branch.**

The disassembly of `FUN_005aafc0` (`0x5aafc0..0x5ab0d3`) corrects the
decompile:

- The F-key index i (0..7) reads slot i of the tactical manager
  `DAT_006bc468` (`+0x164 + 4i`, `FUN_0059f5d0`). That gives the group,
  through `FUN_005a6460`.
- `FUN_005a5fb0` is `jmp [vt+8]` on the current input state
  `DAT_006bc464`. The states return:
  - one unit (`0x66bce8`, `0x005a5920`): 0xe;
  - several units (`0x66bc98`, `0x005a5980`): 0x10;
  - a task force (`0x66bc48`, `0x005a3a10`): slot `+0x54` + 1;
  - a fighter group (`0x66bbf8`, `0x005a48c0`): `+0x54` + 9;
  - the idle state (`0x66be30`, `0x005a5b60`): 0.
- Only a selected task force returns 1..8. Then the followed id is
  `[manager+4] + 0xd4 + 0x18`. It is not a commander's flagship:
  `FUN_00583c40` is only a `+4` getter, called on the manager.
  `manager+4` is the selected unit. When a task force is selected,
  `FUN_005a33d0` sets it to the unit whose id is the group's `+0x70`, the
  id passed to the group's constructor `FUN_005c9db0`.
- Otherwise the followed id is the group's `+0x6c`:
  - `Class_0066cd28_vt28` (`0x005cb3e0`) sets it to the first member
    added;
  - `vt8` (`0x005ca1a0`) sets it outright;
  - `vt30` (`0x005cb510`), when that member leaves, moves it to the next
    member in the `+0x1c` list.
- A0's trace selects Red Group (a fighter-group state, 9..12) before F1.
  So A0 takes the `+0x6c` branch.
- Task Force 1 has one capital member, the Escort Carrier. Both branches
  name that one hull. The port's "first live hull" picks the same one.

This lead does not explain the 1.25.

**2. The second object: the force's attack target.**

- Both capital force vtables (`0x66cd28` and `0x66bf70`) have vt `+0x44` =
  `0x005cb990`. It returns:
  - the first entry of `+0x74` (its `+0x18` id) if there is one;
  - else the id of the first node of `+0x8c`;
  - else 0.
- `Class_0066cd28_vt20` (`0x005cb430`) adds a fighter group to `+0x74`
  (`lea 0x74(%esi)` at `0x5cb445`, `FUN_005c5ca0`).
- `vt2c` (`0x005cb460`) adds a capital ship to `+0x8c` (`lea 0x8c(%esi)`
  at `0x5cb475`, `FUN_005d1ff0`).
- So the second object is the force's current attack target: a fighter
  group first, then a capital ship.
- SetOrder (`Class_0066cd28_vt48`, `0x005ca6d0`) clears `+0x74` for every
  order. It also clears `+0x8c` for orders 1-6.
- The constructor `FUN_005c9db0` leaves the order (`+0x64`) and its
  executor (`+0x60`) at 0, and does not add to either list.

A0's trace gives Task Force 1 no order. Only Red Group gets one: Escort,
on the carrier. So unless the original targets for a force with no order,
which this pass did not find, `piVar4` is 0. `0x005d9b10` then takes the
branch the port runs: SetPosition(-e, e, 2e), then SetOrientation along
the nose.

If a target did exist, the eye would be (e·x/d, e·y/d, 2e), at e·√5. That
is 0.913 times the port's e·√6 distance, not the fitted 0.80. LookAt would
also turn the camera off the nose, while the fit matched the nose
direction to a mean of 1.3 px. So this branch does not fit the capture
either.

**Also confirmed:** the frame calls are the `IDirect3DRMFrame` (v1/v2)
slots:

- SetOrientation `+0xf0`, SetPosition `+0xf4`;
- LookAt `+0x90`, GetPosition `+0x60`, GetOrientation `+0x7c`;
- Move `+0x94` on the scene.

`Frame3` would put LookAt at `+0x8c`.

**Result.** Static tracing now accounts for every input to the chase eye:

- the followed hull (the carrier);
- e (the base + 1 box times 3, larger X/Y span, 3.855);
- the branch (no target, so (-e, e, 2e));
- the hull frame (unit axes);
- the parent frame (an unscaled root);
- front and field (0.1 and 0.02).

All of them match the port, and none explains the 1.25. What is left needs
evidence at runtime:

- a read of the chase's `+0x34` and the camera frame's position in the
  original (Windows VM debugger or a Frida/DLL hook on `0x005d9ba4`);
- or a check of how `d3drm.dll` turns front and field into a projection.
  The free camera's front of 1.0 hides any dependence on front. The chase
  is the first view with front 0.1.

The second one is a hypothesis only. Nothing here changes the code.

### Verdict

TAC-04-C014 stays partial. The box question from the second stage is
settled: the port reads the same value as the original. Both open items
are now traced. Neither explains the gap: F1 follows the carrier, and with
no attack target the eye is (-e, e, 2e). The cell needs a runtime read in
the original of the chase's `+0x34` and the camera's position. The lead's
batch reruns the gate on the unchanged code.

## Batch b200

Full gate batch b200 (Linux, Arch Chromium): `cargo check`, workspace
3641 and fixture 163 tests pass; 29 of 31 gates and 175 of 176 battle
cases pass on the first run. The two failures were in the gates:
- `game-options` compared the Yes tip's border pixel for pixel. The port
  draws tips within five pixels of A0's widths (`win_tooltip.rs`), so
  this tip is 26 px wide where A0's is 24. Commit 1c0ef350 checks its
  corner and height exactly and its width within that tolerance. Rerun:
  `game-options-2026-11-12T15-40-26-930Z`, pass.
- `production-coruscant-chase` (Alliance, 1280 x 800) mapped the
  carrier's logged canvas position through the letterbox a second time,
  so the right-click missed. Commit c4f02d3f fixes the mapping. Rerun of
  all four cases: pass (`g-chase-rerun4`).
Batch b201 reruns everything with both fixes.

## Batch b201

Full gate batch b201 (Linux, Arch Chromium), with the Alt-accelerator fix
71bf9394 and the gate fixes 1c0ef350 and c4f02d3f: check, workspace 3643
and fixture 163 tests pass, 31 of 31 gates and 176 of 176 battle cases
pass. The Alt case under the Battle Summary (Alt+G/H refused, Alt+2/Alt+1
switch the GID display) passes in production-battle-last-ship-lost.

## Fourth stage: A0's battle extent (2026-11-13)

`ghidra/notes/tactical-chase-camera-distance.md` showed that the chase is
not the cause of the gap. A0's battle has extent 136, the fixture's had
109. Positions scale with the extent; mesh sizes, and with them the chase
offset, do not.

### What the extent counts

`FUN_005ab650` walks every tactical unit (`+0x9a0`, ships and squadrons
alike). It counts a side's unit unless the unit is in state 1
(`0x5ab94b`, `0x5abafc` -> `FUN_005b69f0`: `+0x170 == 1`). It adds 3.0
(`DAT_0066c2a0` = -3.0, subtracted) each time a side's count passes the
other's (`0x5abb11`..`0x5abb4b`).

State 1 at placement is a squadron inside its carrier's hangar
(`0x40bf0b` -> `FUN_005964c0` -> `FUN_005b69d0`, aboard a ship without a
working hyperdrive). A squadron on a planet starts deployed and counts.
The port's count (`assign_original_world_positions`) already follows this
rule: living ships, plus squadrons that are not in a hangar. Only its doc
comment changed.

A0's save holds the Judicator with its TIE squadrons aboard, plus 11
Coruscant planet TIE squadrons (manifest TAC-02-C004, TAC-05-C009).
- With the hull, that is 12 Imperial units: 100 + 3 x 12 = 136.
- The fixture's ISD has 6 TIE squadrons aboard, so it counted 1.

### The fixture

`coruscant_chase_fleets` adds 11 TIE Fighter squadrons to the home fleet,
with no carrier.
- port: the port's battle takes no planet squadrons. Unboarded fleet
  squadrons start deployed, as the planet's do.
- The new test
  `the_coruscant_chase_battle_counts_twelve_imperial_objects_as_a0_s_extent_shows`
  (ignored, needs `data/base`) asserts 3 against 12, extent 136, lane 68,
  and 11 deployed TIE Fighters.
- Without the change it fails with `(3, 1)` against `(3, 12)` (shown).

### The gate

`production-coruscant-chase` now expects:
- the layout 3 against 12, extent 136, lanes +/-68 and +/-48;
- 21 participants: the 6 aboard are inactive, and the 11 deployed sit at
  z 48.

The chase check is tightened:
- every A0 pyramid of at least 100 px within 3 px of the port's (was
  40 px);
- each red one within 10 % of A0's area.

The pyramid finder keeps port components down to 10 px. A pyramid half
behind the carrier fell under 20 px in the port.

Results, run `2026-11-13T22-04-37-221Z-447138` (Linux, Arch Chromium 155),
2 of 2 pass:

- Chase: 10 pyramids, worst 1.5 px. Red areas are 0.95 to 1.0 times A0's
  (the largest is 1,948 px against 1,969). With extent 109 they were
  about 1.5 times A0's, and up to 34.5 px off.
- Free camera (TAC-04-C001): 34 pyramids, worst 0.6 px. Areas are now
  mostly equal to A0's to the pixel. The "1.2 times" size difference of
  the second stage was the same extent gap.
- Outside the view: 1 and 1 pixels, the known palette entry.

An earlier run (`22-03-26-530Z`) passed the Alliance case and failed the
Empire's on the 60-frame timing sample ("paused tactical render stalled").
The rerun passed.

Checks:
- `cargo check --workspace` passes.
- `cargo test -p rebellion-app`: 190 passed. With `--include-ignored
  tactical_test_fixture`: 6 of 6 pass.
- `provenance-scan check`: 0 uncited items.
- `rustfmt --check` on the fixture: the same 7 baseline diffs as HEAD.

`cargo mutants --in-place` (`coruscant_chase_fleets`, `--include-ignored
coruscant`): 11 mutants, 9 caught, 2 missed. Both missed ones are
equivalent on the original data:
- `&&` to `||` in the defender filter (as before). Coruscant's only fleet
  is Imperial and has a hull.
- `&&` to `||` in the TIE class filter. The first Imperial fighter class
  is the TIE Fighter.
