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

## Round 7: the first voice's wait and the skip (2026-11-12)

### Who sets bit 5 of `+0x28`

The clip's sound is a six-word block at `+0x18` (`FUN_00403f70` zeroes
`+0x18..+0x2c`, then starts it through `FUN_00610c30` → `FUN_006115f0`).
`+0x28` is that block's `+0x10`, the sound layer's flags:

- `FUN_00611580` (the play) sets `0x20` when the buffer starts.
- `FUN_00611780`, which `FUN_00403fe0` calls through `FUN_00610c80`, looks
  the WAVE up. If it is loaded, `FUN_00612890` reads the buffer's status
  (`FUN_00613c80`) and writes its playing bit to bit 5. If not, it clears
  `0x30` and returns `0x10`, so `FUN_00403fe0` returns 0.

So bit 5 means "the voice is playing". No clip-specific writer exists: every
voiced clip waits while its voice plays, the briefing's first too. The
port's `hyp:` is replaced with these citations. One case the port lacked:
a voice that did not play holds nothing (`FUN_00611780` returns `0x10`).
`ClipPlayer::voice_started` now takes whether the voice played. The test
`a_clip_whose_voice_did_not_play_ends_the_tick_its_last_frame_is_added`
fails without that (the clip held for the WAVE's length).

The clip loader (`FUN_00404440`) decodes every frame, then starts the voice.
The player (`FUN_004729c0`) loads the next clip in the step the last one
ends, and the new clip's first step is the next period. That holds for every
clip, so nothing here makes the first voice start before its first frame.

### The first gap is not special

`a0_droid_clips.py` now keeps only the briefing's own clips. ALBRIEF 1220's
first 76 frames are also 1156's, and 1225's first 173 are 1162's, at the
same index. Neither 1156 nor 1162 plays in message `0x29`, so those A0
frames date 1220 (9.93 s, spread 0.05) and 1225 (11.77 s, spread 0.05).
The Empire's dates are unchanged.

The Alliance's "first gap" (1155 to 1221) spanned two voice-limited clips.
Split, and against each clip's longer limit (frames at 66 ms, or voice):

| Side | Clip | Frames | Voice | A0 gap | A0 gap minus the limit |
|---|---|---|---|---|---|
| Alliance | 1155 | 9.04 s | 9.43 s | 9.15 s | -0.28 s |
| Alliance | 1220 | 5.02 s | 5.06 s | 5.06 s | 0.00 s |
| Empire | 1147 | 11.02 s | 11.57 s | 11.43 s | -0.14 s |
| Empire | 1223 | 7.72 s | 7.80 s | 7.80 s | 0.00 s |
| Empire | 1226 | 16.37 s | 16.56 s | 16.45 s | -0.11 s |
| Empire | 1155 | 26.27 s | 26.52 s | 26.39 s | -0.13 s |
| Alliance | 1223 | 10.82 s | 11.00 s | 10.85 s | -0.15 s |

In A0, a voice-limited clip's next voice starts up to about 2 ticks before
the voice's nominal end, while a frame-limited clip's starts 1 to 2 ticks
after its frames (an action between them). The Empire's first clip is like
its others. The Alliance's 1155 is 2 ticks shorter again, inside the A0
dating spread (0.05 s per voice, two voices per gap) plus one sample. So
the first clip waits like the others. Nothing in `FUN_00403fe0`'s chain
shortens the wait. The voice ending a tick or two early in A0 is therefore
probably the buffer status that Windows 10's DirectSound reports, which is
not observed. The port is not changed for it.

`briefing.mjs` now asserts every gap, the first included, with the same
0.3 s plus 2 % tolerance. The Alliance's first series has 6 gaps and its
second 6 (was 5 and 5).

### Skip (PRE-05-C005): the trace

- Action `0xc` (`FUN_004c30c0` / `FUN_004c0fc0` → `FUN_0041d9d0`) calls
  `SetWindowsHookExA(WH_KEYBOARD, LAB_0041d8f0, NULL, 0)` and
  `SetWindowsHookExA(WH_MOUSE, FUN_0041d950, NULL, 0)` (the disassembly
  pushes 0 for both `hMod` and `dwThreadId`), clears `DAT_006b14b4` and
  sets `DAT_006b14bc`.
- Mouse hook `FUN_0041d950`: a left or right press (`0x201`, `0x204`)
  sets advice bit `0x2000` (`FUN_0043a200`); presses and releases
  (`0x201`, `0x202`, `0x204`, `0x205`) are swallowed. Other messages,
  such as the middle button, wheel and moves, pass on.
- Keyboard hook `LAB_0041d8f0` (disassembled 2026-11-12): while
  `DAT_006b14b4` is 0 it swallows every key. On Escape (`0x1b`) with
  lParam bit 31 set (the release), it sets `0x2000`. Once `DAT_006b14b4`
  is set it passes keys on.
- The side's update answers `0x2000` (`FUN_004c2dd0` / `FUN_004c0d00`).
  It schedules the short form `0x28` / `0x49` for 40000 days and calls
  `FUN_0041da30`, which flushes both players once (`FUN_0042d620` →
  `FUN_00472ba0`, guarded by `DAT_006b14b4`). The flush stops the current
  op-4 clip, which cuts its voice (`FUN_00404360` with `+0x64`). So a skip
  ends the whole briefing, not the current clip. The short form then plays:
  action `0xb` (Alliance: `FUN_0041d8c0(0x11, 0)` and `FUN_0041dbe0`), one clip
  (ALBRIEF 1165 / EMBRIEF 1157), and action `0xd`. Action `0xd` opens the
  Message Index on the Advice Messages (`FUN_0041d770(1, 0x82)`, unless
  bit `0x8000`) and removes both hooks (`FUN_0041da80`).

The port matches this: `advisor_schedule.rs` `holds_input` (mouse until
`0xd`), `holds_keys` (keys until the flush), and `request_skip`;
`main.rs` asks on a left or right press, or on Escape's release. The gate's
`skip` case asserts actions `0xc, 0xb, 0xd`, voices first + short, and one
cut voice. One difference remains: the original's hook swallows the
clicks for every window, but the port blocks only the galaxy map
(`map_state.input_held`), so an egui window or menu can still take the click
that skips. It is not changed here, because of the next point.

**Open: on Windows NT the hooks may not install.** `SetWindowsHookExA`
with `dwThreadId` 0 (a global hook) and `hMod` NULL fails on NT-family
Windows with `ERROR_HOOK_NEEDS_HMOD` (1428), and Wine returns the same.
Only `WH_*_LL` hooks are exempt. If so, on the A0 VM (Windows 10) neither a
click nor Escape skips the briefing, and clicks are not swallowed. The
handles in `DAT_006b14ac` and `DAT_006b14b0` would be 0. This is from the
API contract and was not observed. Windows 95/98 behaviour is not
established either.

The A0 captures that would settle it:
1. Windows 10 VM, new Alliance game with ALBRIEF.DLL restored. Take QMP
   screenshots every 0.5 s from the first voice. Left-click once in the
   galaxy view about 3 s into 1155, and keep going 20 s. Record audio with
   `-audiodev wav` and save the QMP clock. A skip shows the droid switching
   to clip 1165's frames, then the Message Index at about 1 to 2 s plus
   1165's length. No skip shows 1220's frames at about 9.9 s.
2. The same with Escape pressed and released instead of a click, and the
   Empire once (EMBRIEF 1157).
3. Optional: the same on Windows 98 (or 95), for the hooks as designed.

### Proposed status (Round 7)

- PRE-05-C004: **partial**. Every voice gap, the first included, matches
  A0 in the gate. The voices' sound is not compared, because A0 has no
  audio.
- PRE-05-C005: **partial**. The port follows the traced hooks, but there is
  no A0 skip, and on Windows 10 the hooks may not install at all.

### Round 7 run (Linux, Chromium, headless)

`briefing-2026-11-12T06-51-05-575Z`, on this tree's build
(`web/open-rebellion.wasm` sha256 `1676a333…0501a2ca2f`, load average
2.4 at the start): **all four cases pass.**

- Alliance: droid 83 of 85 exact (2,512 crops; misses `b104` 435 and
  `b121` 543). All 12 gaps are asserted. The first, 1155 to 1220, is
  9.57 s against A0's 9.15 s, an excess of 0.42 s on a 0.48 s tolerance.
  1220 to 1221 is 5.23 s against 5.06 s. The rest are 0.01 to 0.35 s over
  A0.
- Empire: droid 96 of 98 exact (2,626 crops; misses `e063` 416 and `e094`
  539). All 15 gaps are asserted. The first, 1147 to 1220, is 11.72 s
  against 11.43 s, 0.29 s on a 0.53 s tolerance.
- Skip: both sides give actions `0xc, 0xb, 0xd` and cut the first voice.

The port's gaps run longer than A0's by 0.0 to 0.42 s. That fits the
port's 67 ms tick against A0's fitted 66 ms, plus a voice-limited clip's
tick or two. The Alliance's first gap has the least margin, 0.06 s. On a
loaded machine it is the first assertion likely to fail.

Tests:
- `cargo test -p rebellion-render`: 1,489 passed.
- Mutants (`--re voice_started`): 2 mutants, 2 caught.
- The `played` branch is not among the mutants cargo-mutants generates.
  Reverting it by hand fails the new test.

## Batch b197

The full gate batch b197 (2026-11-12 08:23-09:45) passed: 30 gates, the workspace (3617 tests) and fixture tests (158), and 172 tactical battle checks in 43 scenarios, 0 failed. The ledger notes above were applied after it.
