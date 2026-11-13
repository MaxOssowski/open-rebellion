# The destroyed hull's fireball: frames, timing and A0 (2026-11-13)

This is for TAC-03-C004 (tactical, a destroyed ship). It adds to
`2026-11-11-hq-and-tactical-states` and `2026-11-12-tactical-panels`. The
ledger JSON is not changed.

## Inputs

All are local and untracked. Only paths and hashes are given.

| Input | SHA-256 |
| --- | --- |
| `REBEXE.EXE` (Heroic install, disassembled with `objdump`) | `b3fe3997cab9a6e96403d638875dcba25484e4d8601751afec748471ac0ed6ab` |
| `data/base/TACTICAL.DLL` | `db12cdcfb6c85cec572a425307d2ad617bbc850027657424c3cc1f61c6ec275a` |
| `data/base/ui/tactical-dll/TACTICAL3D/runtime/manifest.json` (the decoded 3360..3375 and palette 5531) | `c14f34ae8f1c08a66d980e889e92a27019aaf0e4d930e06130000e5302134c23` |
| A0 `.artifacts/interface-parity/a0/TAC-03-C004/original.png` (Windows 10 VM, 2026-10-24, Battle at Mon Calamari, Medium Transport 2 destroyed, the 17th screendump of a 3 s series) | `605e464ff7aefb4cdb77892b73b6d24910293e12a923d938270f424fd1e13a1f` |

## What the original does

Functions are in `~/Games/rebexe-decompile/decompiled-typed/`; the
instruction addresses were read from `REBEXE.EXE` with `objdump`. The
corrected account is in `ghidra/notes/tactical-impact-effect-path.md`.

- `FUN_005d3e90` (vtable 0x28 of class 0x66d198) starts the destroyed
  sequence: state bit 0x80, base 3360 (0xd20), count 16, counter `+0x140` at 0.
- `FUN_005d41a0` runs once per effect for every scene render
  (`FUN_005c25b0` calls `FUN_005c2c80`, which calls it). On each call:
  - with the counter below the count, it swaps the image to
    `base + counter` if the counter is 0 or the `+0xf0` timer is past
    0.1 s (`0x5d422d`, float `0x0066d154`), and then resets the timer;
  - with the counter at the count, it removes the effect (`0x5d4301`);
  - with bit 0x80 set and the counter at 11, it turns off both field
    loops and calls vtable 4 (`FUN_005d3180`), whose vtable 0x30
    (`FUN_005d3420`) deletes the hull's mesh visual (`0x5d434f..0x5d4379`);
  - it adds one to the counter in every case (`0x5d437c`). A call that
    does not swap still counts.
- The render rate: `FUN_00413560`'s `PeekMessageA` loop calls the idle step
  on every pass; for a battle that is `FUN_0059a530`, which busy-waits in
  `FUN_005c1490`/`FUN_005c1cf0` until the scene timer (`+0x660`, reset at
  the start of `FUN_005c25b0`) is past 0.02 s (float `0x0066c954`), then
  renders through `FUN_005c1470`. So renders come at most 50 times a
  second. The render, and with it `FUN_005d41a0`, runs whether or not the
  battle is paused.

So the counter counts renders. A frame number on screen depends on the
render period P:

| P | Swap every | Destroyed frames shown | Hull goes | Lasts |
| --- | --- | --- | --- | --- |
| 20 to 25 ms (the cap) | 5 renders | 3360, 3365, 3370, 3375 | render 12 | 16 renders, 0.32 to 0.4 s |
| 33 to 50 ms | 3 renders | 3360, 3363, 3366, 3369, 3372, 3375 | render 12 | 0.53 to 0.8 s |
| over 100 ms | every render | all 16 | render 12 | over 1.6 s |

The port showed every frame for 0.1 s each (1.6 s in all) and took the
hull away at once. That holds only for a render period over 100 ms.

## Which frame A0 shows

`tools/interface-parity/fireball-frame-a0.py` decodes 3360..3375 through
palette 5531 (first index transparent, as `tactical_assets.rs` does), draws
each as a square decal with nearest sampling, estimates its size from the
frame's opaque box against A0's fire box, and searches size and position
within a few pixels. A pixel is fire when its colour is one of the frames'
ten palette colours. The window is x 130..239, y 260..349.

A0's fire: 705 pixels, box x 163..201, y 290..317, eight colours.

| Frame | Size from width / height | Best decal | Same colour | Other fire colour | Fire only in A0 | Fire only in the frame |
| --- | --- | --- | --- | --- | --- | --- |
| 3367 | 52.5 / 73.1 | 49 px | 74 | 309 | 322 | 5 |
| 3368 | 53.1 / 66.4 | 59 px | 248 | 397 | 60 | 119 |
| **3369** | **50.9 / 51.9** | **51 px at (182.9, 304.2)** | **526** | **168** | **11** | **7** |
| 3370 | 47.5 / 47.2 | 45 px | 290 | 352 | 63 | 44 |
| 3371 | 47.1 / 49.1 | 46 px | 104 | 541 | 60 | 47 |

The other eleven frames do worse (full output: run the script). Only
3369's opaque box has A0's proportions, and its outline matches to 18
pixels (11 + 7). Of its 168 pixels of another colour, the largest group is
50 pixels of the frame's (255, 128, 0) shown in A0 as (255, 85, 0); A0 has
no (255, 128, 0) at all, so the 8-bit display seems to map it there. This
is not settled.

A0 shows frame 9 (3369). Frame 8 (3368, where the fixture held the
sequence) cannot be the frame. 3369 is not shown at the render cap; it is
shown when the VM rendered at 33 to 50 ms a frame (or slower than 100 ms,
or on a mix of periods that sums to it). The VM has no 3D acceleration,
which fits.

## Port change

- `TacticalImpactEffect` (`rebellion-core/src/tactical.rs`) keeps the
  counter (`frame`), the counter at the last swap (`shown_frame`) and the
  calls since it. `original_render_call` is one `FUN_005d41a0` call.
  `advance_presentational_effects` makes one call per
  `ORIGINAL_TACTICAL_RENDER_SECONDS` (0.02 s) of elapsed time, the cap.
  `ORIGINAL_TACTICAL_EFFECT_SWAP_CALLS` (5) is the `port:` choice that
  the original runs at its cap.
- The scene keeps a destroyed hull until the call at counter 11
  (`hull_in_scene`, `tactical_view.rs`).
- The detail fixture (`production-detail-escort-presentation`) now holds
  A0's frame: 3369 shown, counter 12, hull gone. `run.mjs` expects 3369.
  The six-effect fixture keeps its frames.

New tests (`tactical_view.rs`), each failing before the change:

- `a_destroyed_hull_shows_every_fifth_fireball_frame_at_the_render_cap`
- `a_destroyed_hull_stays_under_its_fireball_until_the_eleventh_render_call`
- `a_laser_hit_holds_its_first_frame_then_shows_its_last_for_one_render`
- `impact_frames_wait_for_a_whole_render_period`, which replaces
  `original_effect_frames_advance_at_ten_hertz_and_expire_exactly`. That
  test cited `FUN_005d3cc0` (the field loop, which does step inside its
  timer); `FUN_005d41a0` counts on every call (`0x5d437c`), so its
  expectation was wrong.
- `the_scene_keeps_a_dead_hull_only_while_its_fireball_has_it` (added with
  the helper).

Before the change the first four failed, for example the destroyed
sequence came out as 3360 x5, 3361 x5, 3362 x5, 3363 x3 instead of
3360 x5, 3365 x5, 3370 x5, 3375, gone.

## Still open

- The render rate. The port runs the counter at the cap. A0's machine was
  slower, so live play in the port never shows A0's 3369. To fix the rate
  from A0, one capture series is needed: screendumps as fast as QMP gives
  them (at least 10 a second) over the second after a hull dies in the VM's
  battle, and the frame count of a fixed camera move (or any other render
  counter) over a timed span. Either gives the VM's render period.
- Pause: the original's renders, and so its effects, run while the battle
  is paused. The port advances them only when it is not paused; its
  fixtures rely on that to hold a frame. Not changed.
- The colour shift (255, 128, 0) to (255, 85, 0) is not explained.
- The fireball's size depends on the camera; A0's camera path is not known,
  so the size is not compared in the browser.

## Checks

- `cargo test --workspace`: pass (rebellion-core 1,746, rebellion-render
  1,588); `cargo check -p rebellion-app --features interface-test-fixtures`: pass.
- `cargo run -q -p provenance-scan -- check`: 0 uncited items.
- Mutants (`--in-place`):
  - `tactical.rs`, `--re 'TacticalImpactEffect::|advance_presentational_effects'`,
    tests of rebellion-core and rebellion-render: 46 mutants, 42 caught,
    2 timeouts (the field loop's `-=` turned into an endless loop), 2 missed.
    Both missed ones are on lines this change does not touch: the
    `!elapsed.is_finite() || elapsed <= 0.0` guard (`||` to `&&`; no test
    passes a non-finite time) and the projectile's `elapsed < duration`
    (`<` to `<=`). Every mutant of the new code was caught.
  - `tactical_view.rs`, `--re hull_in_scene`: all 7 mutants of
    `hull_in_scene` caught. The tool also ran 34 field-deletion mutants of
    drawing functions this change does not touch (30 missed), which `--re`
    does not filter.
- Gates (Linux, Arch Chromium 155, not the pinned build; the test WASM was
  built from this tree, data copied into the worktree):
  - `2026-11-13T21-53-25-405Z-270254`: `run.mjs --battle
    --scenario=production-detail-escort-presentation`, 2 of 2. The fixture
    submits 3369 at the retired hull's place; the hull is not drawn. The
    colour check finds all five of A0's colours in the port's fireball
    (box 44 x 29 at the fixture's camera); the port adds (255, 128, 0).
  - `2026-11-13T21-53-37-970Z-270697`: `--scenario=production-effect-presentation`,
    2 of 2 (the six-effect fixture, frames unchanged).
