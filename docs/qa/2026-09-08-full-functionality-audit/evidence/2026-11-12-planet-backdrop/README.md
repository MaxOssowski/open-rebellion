# The tactical planet backdrop's random framing (TAC-01-C008)

Date: 2026-11-12. The ledger JSON is not changed.

## Question

The planet decal's frame stands at `(2e, rand(0.2e) - 0.1e,
rand(0.2e) - 0.1e)` (`FUN_005d4d10`, see
`../2026-10-24-tactical-camera-backdrop/`). The port took both offsets at
the centre of their range (`port:`). In `production-umgul-backdrop` the
planet's edges were then up to 14 px off A0 TAC-01-C008. Which random
stream draws the offsets, and can the state of the game reproduce them?

## Source

Static reading of `REBEXE.EXE` (SHA-256
`b3fe3997cab9a6e96403d638875dcba25484e4d8601751afec748471ac0ed6ab`):
the typed decompile corpus (`~/Games/rebexe-decompile/decompiled-typed`)
and `objdump` of the call sites.

| Address | What it does |
| --- | --- |
| 0x005d52a7 to 0x005d52ab | `FUN_005f5650` builds a local stream object at `[esp+0x20]`: vtable `0x0066dcf0`, state -1. |
| 0x005d53b2 to 0x005d53bc | Planet y: `n = ftol(0.2e)`, `FUN_005f5700(n)` on that local object (`lea ecx,[esp+0x24]` after one push). |
| 0x005d53dc to 0x005d53e6 | Planet z: the same, the next draw. |
| 0x005d5412 | `SetPosition(frame, scene, 2e, y - 0.1e, z - 0.1e)`. The planet block runs only while Show Planet (`+0x680`) is on (0x005d5372). |
| 0x005d5543, 0x005d556d | Death Star y and z: `rand(ftol(0.4e)) - 0.2e`, the next two draws, only when a Death Star resource is passed (0x005d54f7). |
| 0x005d56ed | `FUN_005f56a0` destroys the local object. |

The stream:

- `FUN_005f5700(n)`: 0 when `n <= 1`. If the state is -1 it seeds first
  through `FUN_005f56b0(-1)`. Then `state = FUN_005f57a0(state)` and the
  result is `state / (0x7ffffffe / n)`: 0 to `n`, `n` itself rarely.
- `FUN_005f57a0`: `s * 0xa18e + (s / 0xcad4) * -0x7fffffff`, plus
  `0x7fffffff` if negative: the Park-Miller step with multiplier 41358.
- `FUN_005f56b0(-1)`: a seed out of range calls `FUN_005f5750`, then steps
  the stream 16 times.
- `FUN_005f5750`: `FUN_0061b120(0)` is the CRT `time(NULL)` (it reads
  `GetLocalTime`/`GetSystemTime`), `FUN_0061a300` is `srand`, and
  `FUN_0061a310` is the MSVC `rand` (`holdrand * 0x343fd + 0x269ec3`, bits
  16 to 30). The seed is `rand() * rand() + rand() * rand()`, redrawn if
  it is `0x7fffffff`.

## Answer

- The stream is local to `FUN_005d4d10` and seeded from the wall clock in
  seconds. It is not the game's random stream (`DAT_006b9054`, used by
  `FUN_0053e290`), and no battle, system or save field enters it.
- The framing is therefore random per battle and cannot be reproduced from
  the game state. It is fixed for the battle once the scene is built.
- Range: each planet offset is an integer draw `k` in `0..=ftol(0.2e)`
  minus `0.1e`, near-uniform (`n` itself has a chance of about 1 in 2^31).
  For extent 106: `k - 10.6`, `k` in 0 to 21. The Death Star's: `k` in
  `0..=ftol(0.4e)` minus `0.2e`.
- When Show Planet is off, the Death Star takes the stream's first two
  draws instead of the third and fourth.

## A0's planet is one of these draws

A fit of the port's projection (camera after 16 Rotate Left presses,
extent 106, the fixture's Umgul battle) over every pair `(k_y, k_z)` puts
A0's planet edges best at `(6, 5)`, offsets `(-4.6, -5.6)`: the five
measured edges within 1.6 px. The centred frame misses by 13.7 px. Extent
100 gives the same pair. A0's real second is unknown; the stream maps
about one second in 440 to that pair. The first one after 1,800,000,000 is
1,800,000,005 (`UMGUL_A0_BACKDROP_CLOCK`).

## Port

- `crates/rebellion-render/src/tactical_backdrop.rs` ports `FUN_005f57a0`,
  `FUN_005f5750`, `FUN_005f56b0` and `FUN_005f5700`, and draws the planet
  and Death Star offsets in the original's order (`BackdropOffsets`).
- `TacticalState::begin_battle` seeds it with the wall clock in seconds
  (`macroquad::miniquad::date::now`, the browser's `Date.now()`), as the
  original's `time(NULL)`. The planet, the Death Star decal and the Death
  Star beam's origin take the drawn offsets.
- The interface-test fixture pins the clock: `production-umgul-backdrop`
  gets `UMGUL_A0_BACKDROP_CLOCK`; every other scenario keeps both frames
  centred, so their pictures stay the same between runs.

Tests (`cargo test -p rebellion-render`):

- `tactical_backdrop.rs`:
  `the_backdrop_stream_steps_as_the_minimal_standard_generator`,
  `the_backdrop_seed_combines_four_crt_rand_values_after_srand`,
  `a_backdrop_draw_spans_zero_to_its_bound`,
  `the_planet_draws_first_and_the_death_star_follows`,
  `the_umgul_a0_clock_draws_the_planets_sixth_and_fifth_offsets`,
  `the_backdrop_framing_follows_the_clock_second`.
- `tactical_assets.rs`: `the_umgul_planet_framing_is_one_of_the_originals_draws`
  projects the planet disc and checks A0's five edges within 2 px, and the
  centred frame more than 9 px off.
- `tactical_view.rs`: the two backdrop placement tests now also move the
  frames by an offset.

With `BackdropOffsets::from_clock` forced back to the centre, four of the
new tests fail ("the drawn planet misses A0 by 13.664093 px").

`cargo mutants`:

- `tactical_backdrop.rs`: 41 mutants, 35 caught, 2 unviable, 1 timeout
  (`!=` to `==` in `clock_seed` loops for ever). The 3 missed (`step`'s
  `<` to `<=`, `wall_clock_seconds` to 0 or 1) are now caught by two added
  checks (`step(0) == 0`, the clock past 1,700,000,000): a rerun of those 8
  mutants caught 8.
- `tactical_view.rs`, filtered to the backdrop functions: every mutant of
  `planet_backdrop_source` and `death_star_backdrop_source` caught;
  `backdrop_offsets` unviable. Missed: `set_backdrop_clock_for_fixture`
  replaced with `()`, a fixture-only setter that the browser probe catches
  (the wall clock would move the planet off A0's edges). The filter also
  ran 31 unrelated, older drawing-parameter mutants, which no unit test
  draws.

Other checks: `cargo test -p rebellion-render` 1,500 passed with the two
added checks; `cargo test -p rebellion-app` (with and without
`interface-test-fixtures`) 158 passed; `cargo check --workspace` clean;
`provenance-scan check` 0 uncited items.

## Browser (Linux, Arch Chromium, 640 x 480)

`node .gate-run-linux.mjs --no-build --battle --scenario=production-umgul-backdrop`

The probe now requires each planet edge within 2 px of A0's.

| Measure | A0 | Before (run `2026-11-12T04-31-05-179Z-3870797`) | After (run `2026-11-12T11-52-25-172Z-844583`) |
| --- | --- | --- | --- |
| Left edge, row 30 | 334 | 327 | 334 |
| Left edge, row 40 | 339 | 334 | 339 |
| Left edge, row 60 | 352 | 352 | 352 |
| Bottom edge, column 420 | 107 | 97 | 107 |
| Bottom edge, column 450 | 116 | 102 | 116 |
| Holocube line pixels on A0's | | 1,052 of 1,148 | 1,052 of 1,148 |

Both sides passed (2 of 2). The Alliance has no A0 capture of this view;
its planet is out of view as before.

## Verdict for TAC-01-C008

Proposed: **pass, with a note.** The planet's framing is random per battle
in the original (wall-clock seed), so no game state fixes it and a given
capture's framing cannot be predicted. The port draws from the same
stream, range and order. A0's framing is one of the stream's draws, and
with that draw pinned the port's planet edges land on A0's pixels.
