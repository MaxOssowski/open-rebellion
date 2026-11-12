# The battle view's viewport, a side panel's tip and the fireball (2026-11-12)

This adds to TAC-01-C008 (planet backdrop), CMD-11-C030 (tactical battle
panels) and TAC-03-C004 (destroyed). The ledger JSON is not changed.

## A0 used

All captures are in the main checkout's `.artifacts/` (git-ignored). No new
capture was taken for this work.

| Capture | SHA-256 | What it shows |
| --- | --- | --- |
| `interface-parity/a0/TAC-01-C008/original` | `d0e829ffa18b40ce1ee72d4b9fcd70fefb4255507ae52d2892a722122f6879be` | The Empire's Battle at Umgul, paused, after Rotate Left 16 times |
| `interface-parity/a0/TAC-01-C002/original` | `bdd29a6005199b1f5dec12c4ae07bd177d2bf434e9e6a2e875e7528c70911033` | The Empire's first battle frame |
| `interface-parity/a0/TAC-03-C012/original` | `1470e4e8898c376b2a890e03ebc87a9156556db5b9f249cdc06c20b506c32da5` | The Alliance's paused battle at Mon Calamari, Medium Transport 2 selected, the "Missions" tip on the capital-ship panel |
| `interface-parity/a0/TAC-03-C004/original` | `605e464ff7aefb4cdb77892b73b6d24910293e12a923d938270f424fd1e13a1f` | The same battle after the transport died: a fireball where it was |

All four are Wine captures.

## TAC-01-C008: the port projected into the wrong rect

The holocube's lines sat one pixel right and one or two pixels up of A0's.
The cause is the rect the port projected into.

- The port projected the battle into the black aperture of bitmap 1000:
  (16, 28), 444 by 439 (`BATTLE_APERTURE`).
- In A0 TAC-01-C002 and TAC-01-C008, the black rect the original draws is
  (17, 30) to (456, 467): 440 by 438.
- `FUN_005c1d30` creates the scene's viewport through `FUN_005d9490` and
  `FUN_005d9eb0`. It passes x, y, width and height as 0, so
  `CreateViewport` covers the whole device. The device is 440 by 438, the
  size of the star surface, resource 5030.
- So the centre was half a pixel to two pixels off, and the scale used 439
  rows instead of 438.

A fit of the port's lines onto A0's, before the fix, wanted a shift of
(-1, +1.5) and a scale of 0.9975 (= 219 / 219.5). With the fix the best fit
is no shift and no scale.

The fix: `tactical_scene_viewport` (`tactical_assets.rs`) turns the
aperture into the device rect, (x + 1, y + 2, 440, 438) times the canvas
scale. Every 3D entry point of `TacticalAssetRenderer` uses it: the
planet, the decals, the hulls, the fighters, effects, fields,
projectiles, the Death Star beam, the navigation markers and
`project_source_position`. The holocube clips its lines to the same rect.
The star bitmap keeps its own place.

New tests (`tactical_assets.rs`):

- `the_scene_viewport_is_the_original_devices_440_by_438_rect`;
- `the_holocube_edge_after_sixteen_turns_lands_on_the_original_pixels`.
  It projects the near vertical edge and the bottom edge after 16 Rotate
  Left presses. Each must fall on the pixel A0 lights, within a quarter
  pixel. Without the fix it fails: row 136 is at x 95.7, A0 lights 94.

Browser (`production-umgul-backdrop`, Empire, 640 x 480):

| Measure | Before (run `2026-11-12T02-19-40-739Z-3454123`) | After (run `2026-11-12T04-31-05-179Z-3870797`) |
| --- | --- | --- |
| Port line pixels on A0's line pixels | 120 of 1,153 | 1,052 of 1,148 |
| Port line pixels within 1 px of A0's | 728 of 1,153 | 1,148 of 1,148 |
| Corner (first 3-wide row), A0 (106, 298) | (108, 297) | (105, 298) |

The probe now requires the corner within 1 pixel, at least 90% of the
port's line pixels on A0's, and all within 1 pixel. The rest differ by
one pixel where the original's line rule and GL's differ.

The planet's framing is still open. Its frame takes random offsets in y
and z (`FUN_005d4d10`); the port takes their centre (port:). Its left
edge on row 40 is at 334 against A0's 339; its bottom edge at column 420
is at 97 against A0's 107.

## CMD-11-C030: a side panel's tip

A0 TAC-03-C012 shows the tip "Missions" under the capital-ship panel's
Missions button. The pointer is at (562, 255). The box is at (562, 276),
52 by 19: the pointer plus (0, 21), the shared tip rule.

New scenario `production-capital-panel-missions-tip` in
`run.mjs --battle` (appended at the end of the catalog). It reuses the
fixture codes of `production-hull-scale-a0` (65829 and 66085). That is A0
TAC-01-C003's battle: one Medium Transport against one Victory Destroyer
at Mon Calamari, paused, as in TAC-03-C012. No Rust fixture was added.

The probe:

1. clicks Task Force 1 (72, 10), so the capital-ship panel opens;
2. moves the pointer away, then to A0's (562, 255), and waits 1 s;
3. finds the tip's box (win_tooltip FILL and BORDER);
4. at 640 x 480, requires the box's place and height equal to A0's, and
   its width within 12% (the accepted Liberation Sans deviation);
5. when the widths agree, compares the frame pixel for pixel; one pixel
   may differ (A0's pointer covers it).

Result, both sides: the box is at (562, 276), 52 by 19, exactly as A0's.
The frame's 138 pixels: 1 differs, under A0's pointer. Inside the box,
the glyphs differ (Liberation Sans for GDI's Arial). The Empire has no A0
capture. Its panel has the same layout, so its box is held to the same
place. Run `2026-11-12T04-32-22-934Z-3887615` (before step 5 was added),
then the full run below.

## TAC-03-C004: which frame is A0's fireball

A0's fireball is about 36 by 23 pixels near (183, 304). It holds seven
colours: the five listed in `2026-11-11-hq-and-tactical-states` and two
dark reds, (94, 8, 0) and (148, 6, 0).

I decoded the destroyed sequence, TACTICAL 3360..3375, through the battle
palette (5531; the fire colours are the same in every 5530+ palette).
Each frame was drawn as a square decal of every size from 34 to 70 pixels,
on a grid of centres near A0's, with nearest sampling. I counted the
pixels where the port's frame and A0 both show fire in the same colour.

| Frame | Best decal size | Same colour |
| --- | --- | --- |
| 3369 (frame 9) | 51 px at (183, 304) | 485 of 725 (67%) |
| 3370 (frame 10) | 47 px | 299 of 779 (38%) |
| 3368 (frame 8) | 59 px | 240 of 828 (29%) |

So A0 most likely shows frame 9, not frame 8 where
`production-detail-escort-presentation` holds the sequence. This is a
hold point chosen by a fixture, not a fault in the port. No port change was
made. The decal size depends on the camera, and A0's camera path is not
known, so the size is not compared in the browser.

## Proposed status

**TAC-01-C008: partial.** The holocube's lines now land on A0's pixels
(1,052 of 1,148 exact, all within 1 px). The planet's framing still
depends on random offsets the port takes at their centre.

**CMD-11-C030: pass, with a note.** A side panel's tip now has a browser
check against A0: its place, height, width and frame match. Not checked
against A0: the other side-panel controls' tips (no capture shows them);
the unit tests of `command_tooltip` cover their strings.

**TAC-03-C004: partial.** A0's fireball best matches frame 3369. Its
size and the time from the hull's death to that frame are not compared.

## Gate runs (Linux, Arch Chromium)

Run ids are UTC.

- `2026-11-12T04-31-05-179Z-3870797`: `--scenario=production-umgul-backdrop`, 2 of 2
  (before the new line checks).
- `2026-11-12T04-32-22-934Z-3887615`: `--scenario=production-capital-panel-missions-tip`, 2 of 2.
- `2026-11-12T04-44-00-154Z-4094531`: `run.mjs --battle --all`, all 172
  executions pass (43 scenarios, both sides, both viewports). It holds the
  final probes: Umgul's lines 1,052 of 1,148 on A0's, 1,148 within 1 px;
  the Missions tip's frame 1 of 138 differs on both sides.

An earlier full run, `2026-11-12T04-33-37-374Z-3907282`, ran while
`cargo mutants` compiled on the same machine. Four cases failed on timing
("paused tactical render stalled", a missed camera frame, a hover hash). It
was stopped and run again alone.

## Checks

- `cargo test -p rebellion-render`: pass (1,484).
- `cargo run -q -p provenance-scan -- check`: 0 uncited items.
- Mutants:
  - `cargo mutants --in-place -p rebellion-render --file
    crates/rebellion-render/src/tactical_assets.rs --re
    'tactical_scene_viewport|project_in_scene_viewport|project_source_position'`
    caught every mutant of `project_source_position` and
    `project_in_scene_viewport` that it reached. The tool also queued
    about 170 field-deletion mutants of other functions, which `--re` does
    not filter, so the run was stopped.
  - The 18 mutants of `tactical_scene_viewport` (return values and each
    operator) were applied by a script and run against the
    `tactical_assets` tests: all 18 caught.
- The WASM for these runs was built from this tree
  (`scripts/build-interface-test-wasm.sh`).

## Batch b197

The full gate batch b197 (2026-11-12 08:23-09:45) passed: 30 gates, the workspace (3617 tests) and fixture tests (158), and 172 tactical battle checks in 43 scenarios, 0 failed. The ledger notes above were applied after it.
