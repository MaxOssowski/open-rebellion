# The briefing droid and narration against A0 (2026-11-12)

This covers PRE-05-C003 (droid animation), PRE-05-C004 (narration) and
PRE-05-C005 (skip). It follows
`../2026-11-11-options-save-briefings/README.md`, where the Empire's droid
matched 0 of 98 A0 frames.

## The cause: the Empire's frames were stretched by one column

- The Empire's agent aperture is 107 by 133 (`FUN_0042adb0`: `0x6b` by
  `0x85`). Every Empire droid frame is 106 by 133: EMSPRITE and EMBRIEF
  BMPs and all their type-302 deltas. The Alliance's aperture and frames
  are both 67 by 116.
- The port drew each frame over the whole aperture. For the Empire that
  stretched 106 pixels to 107, so one column near the middle was doubled
  and the droid's right half moved one pixel. That is the "shading"
  difference: in A0 `e005` against the nearest port frame, 1,121 pixels
  differed, all right of the droid's middle.
- The game draws a frame at its own size. The clip player's
  `Class_0065a358_vtc` (`FUN_004729c0`) sizes its rectangle from the frame
  bitmap's width and height (`FUN_005fc0e0`, `FUN_005fc0f0`).
- Ruled out: the palette (every A0 and port colour is in the BMP's
  palette, unchanged), the delta decoding (`FUN_0041c7a0` adds each literal
  byte to the index, as the port does; all 2,738 EMBRIEF type-302
  resources parse to their declared size), and another frame source (see
  below).

### Proof offline

`a0_droid_clips.py` decodes every voiced clip of the briefing DLL as the
game does (`FUN_00404440`, `FUN_0041c7a0`). It draws each frame unscaled at
the aperture's corner and looks for the frame whose opaque pixels all
equal A0's box.

- Empire: 97 of 99 A0 frames match a clip frame exactly. The others are
  `e000` (the idle clip, before the first voice) and `e094` (the Message
  Index opening).
- Alliance: 66 of 86 match exactly. The rest are `b000` and `b121..b139`,
  after the briefing.

### The fix

`droid_frame_rect` in `crates/rebellion-render/src/advisor.rs` draws each
droid frame at its own size, scaled with the canvas, from the aperture's
corner. The test
`a_droid_frame_keeps_its_own_size_inside_a_wider_aperture` fails without
it (the rectangle is 107 wide, not 106).

## Narration timing from A0 (PRE-05-C004)

A0 has no sound. But a clip starts its voice with its first frame
(`FUN_00404440` -> `FUN_00403f70`), and each A0 frame shows one numbered
frame of one clip. So frame `k` of a series taken every `P` seconds that
shows frame `f` dates that voice's start to `k * P - f` ticks. The tick
fitted from the samples is 66 ms (the panel's `0x43` ms). The starts agree
within 0.08 s across all frames of a clip.

This also settles the port's `hyp:` that a clip waits for its voice. EMBRIEF
1152's clip has 134 frames (9.0 s) and a 10.6 s voice. In A0 the next voice
starts 10.6 s later.

`briefing.mjs` `full` now records when each voice starts. It checks each
gap between two voices against A0's, within 0.3 s plus 2 %. A0's sample
period is only nominal.

### Open: the first voice

The gap after the first voice is recorded but not asserted. On both sides
the first clip's voice outlasts its frames:

| Side | First clip | Frames | Voice | Next clip in A0 starts |
|---|---|---|---|---|
| Alliance | ALBRIEF 1155 | 137 | 9.43 s (143 ticks) | about 139 ticks after its first frame |
| Empire | EMBRIEF 1147 | 167 | 11.57 s (175 ticks) | about 173 ticks after its first frame |

A later clip waits for its voice plus about 2.6 ticks (EMBRIEF 1152). So
the first clip ends 4 to 6 ticks (0.3 to 0.4 s) early in A0. The port waits
the whole voice. Its gap from 1155 to 1221 was 14.79 s and 14.81 s in two
runs, against A0's 14.21 s.

Two causes fit, and neither is traced: the first voice starts before its
clip's first frame shows, or the first clip does not wait for its voice.
`FUN_00403fe0` makes a clip wait while its WAVE (`+0x18`) plays and flag
bit 5 of `+0x28` is set; who sets that bit is not read. An A0 audio
recording with a clock would settle it.

## The gate

`tools/interface-parity/briefing.mjs`:
- `full` asserts the droid check for both sides now. At least 0.9 of A0's
  frames must match a sampled port frame exactly, and every miss must be
  within 600 pixels.
- `full` checks the narration gaps (Empire 15 gaps; Alliance 10, inside
  its two A0 series).
- The droid's sampled crops are decoded after the briefing, not in the
  sampling loop.

## Runs (Linux, Chromium, headless)

All on the interface-test build of the fixed tree, `web/open-rebellion.wasm`
sha256 `53a3cbd1…7473f4e3`. The run folders are under
`.artifacts/interface-parity/` of the agent worktree.

- `briefing-2026-11-11T22-48-55-354Z`: no result. The worktree had no
  `web/data/video`, so the intro's media-failure box blocked the menu.
- `briefing-2026-11-11T22-55-29-464Z`: the Empire passed both cases.
  Its droid matched 91 of 98 A0 frames exactly; the 7 misses were 205 to
  539 pixels. The Alliance's `full` failed: 66 of 85 exact. The machine
  was loaded (load average 13 to 19, other agents' gates), and the gate
  sampled only 1,900 distinct droid crops, against 2,502 on 2026-11-11.
  The sampling loop then decoded each PNG, so the gate now decodes them
  after the briefing.
- `briefing-2026-11-11T23-06-25-280Z` (Alliance `full` only, before that
  change): failed on the first voice's gap, 14.81 s against A0's 14.21 s.
  See "Open: the first voice".
- `briefing-2026-11-11T23-11-43-535Z`: **all four cases pass.**
  - Alliance: 83 of 85 A0 frames exact (2,480 crops). Misses: `b104`
    (435 pixels) and `b121` (543, after `0xd`).
  - Empire: 95 of 98 exact (2,608 crops). Misses: `e009` (432), `e063`
    (416) and `e094` (539, the index opening).
  - Narration: the 9 asserted Alliance gaps differ from A0 by 0.00 to
    0.33 s, the 14 Empire gaps by 0.00 to 0.37 s. The port's gaps are
    1 to 2 % longer than A0's. The first voice's gap is 14.80 s against
    14.21 s (Alliance) and 11.74 s against 11.43 s (Empire), recorded.
  - Skip: both sides give actions `0xc, 0xb, 0xd` and cut the first voice,
    as before.

## Tests

- `cargo test -p rebellion-render`: 1,472 passed, 0 failed.
- `cargo run -q -p provenance-scan -- check`: ok, 0 uncited items.
- `cargo mutants --in-place -p rebellion-render --file
  crates/rebellion-render/src/advisor.rs --re droid_frame_rect`: 7
  mutants, 5 caught, 2 unviable, none missed. The `scale` line in
  `draw_advisor` has no unit test (it draws through egui); the browser
  gate covers it.
- A0 frames used: `.artifacts/a0-vm/briefing-alliance/b000..b045,
  b100..b139` and `.artifacts/a0-vm/briefing-empire/e000..e098` (hashes in
  `../2026-11-05-briefing-a0/a0-sha256.txt`).

## Skip (PRE-05-C005)

No A0 capture shows a skip. The A0 folders hold only full briefings
(`briefing-alliance`, `briefing-empire`). The skip stays as in
`../2026-11-11-options-save-briefings/README.md`. It needs A0 frames every
0.5 s of a new game with a click during the first voice, and audio.

## Proposed status

- PRE-05-C003: **pass**. Both sides' droid frames match A0 in the gate.
- PRE-05-C004: **partial**. The voices' order and timing match A0 within
  0.3 s plus 2 %. Their sound is not compared, as A0 has no audio.
- PRE-05-C005: **partial**. No A0 evidence of a skip.

## Batch b194

Full gate batch b194 (2026-11-12 02:32) is green: 30 gates, check, workspace
(3598) and fixture (156) tests with 0 failures, 160 battle PASS (40
scenarios). The briefing gate, run afterwards on the main checkout
(`briefing-2026-11-12T01-32-46-611Z`), passes all four cases. Passed:
PRE-05-C003, OBJ-01-C008, EVT-02-C005, TAC-07-C004.
