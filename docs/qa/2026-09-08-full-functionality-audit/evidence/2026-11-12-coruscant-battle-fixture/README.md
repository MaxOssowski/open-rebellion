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

## Verdict for TAC-04-C014

TAC-04-C014 stays partial. A0's battle is now rebuilt, and everything
outside the 3D view matches A0. But the view F1 produces does not match:
the port's F1 is a look-at, not the original's chase camera behind the
group (`0x005d9b10`). To pass, the cell needs that camera ported, and then
the pyramid measures above asserted against A0's.
