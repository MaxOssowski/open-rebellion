# Attack Death Star needs more Alliance fighters and one group at a time

Date: 2026-10-20. A follow-up to F-017 (the tactical Death Star), on the
user's go-ahead.

## The original

The mission control's Attack Death Star check is `FUN_005ad930`.
`FUN_005afcc0` and `FUN_005d14d0` in the tactical AI share its parts. It
requires all of the following:

- an operational Death Star with no shield (`FUN_005ba450`, ported
  earlier today);
- no trench run under way. `FUN_005d0ae0` returns `DAT_006bc978`, which
  the result object's constructor `FUN_005d0820` sets and its destructor
  `FUN_005d0950` clears;
- fewer Imperial than Alliance fighter groups on the field. This counts
  the manager's object list `+0x988` for class 1 (slot `+0x20`, a fighter
  group), split by side `+0x168`. Side 0 is the Alliance: `FUN_005a2920`
  queues voice event `0x84` for it, which the port's voice table already
  gives to the Alliance;
- none of the player's groups with order or pending order 6
  (`+0x68`/`+0x64`).

The manual (p. 151) agrees: "there are more Alliance than Imperial
fighters currently launched. Only one fighter group at a time may attempt
the Death Star mission." The notes are in `tactical-death-star-path.md`.

## Port

- `tactical_view::attack_death_star_enabled` adds the last three rules.
- port: the run counts as under way from state 5's timer to its result
  (`trench_run_remaining_ms`).
- The previous stage's shield test placed no fighters. Under this rule it
  now adds one Alliance group, an expectation changed with the source
  above.

## Tests

- `attack_death_star_needs_more_alliance_fighters_and_no_run_under_way`
  covers:
  - an equal count;
  - one more Alliance group;
  - a lost group;
  - a run under way;
  - the player's group already on the order;
  - an enemy group on the order.

  It fails without the rule (`fail-without.txt`).
- `cargo test --workspace`: 2489 passed, 0 failed.

## Browser gate of the tactical Death Star: blocked

The four tactical Death Star scenarios of `run.mjs --battle` (presentation,
laser journey, trench-run success and failure) failed in all 16 cases. All
failed at fixture setup, before any check of this rule
(`tactical-fixture-blocked.json`):

- the error is "fixture system is not empty and intact";
- `tactical_test_fixture.rs` looks for SYSTEMSD index 100 (Alliance) or
  101 (Empire);
- SYSTEMSD holds indices 100..299, but the default galaxy loads 100
  systems with indices 120..289, so neither exists;
- this stage's commits do not touch the world loading, so the failure is
  older.

It is recorded as a candidate. The rule rests on the unit test.

## The environment changed during this stage

At 19:23 a system update did two things:

- it reinstalled the stable Rust toolchain (rustc 1.99.1) without the
  `wasm32-unknown-unknown` target. `rustup target add` restored it;
- it moved Chromium from 153 to 154. The Linux browser manifest for the
  interface gates now names 154.0.8037.97.

One gate run (`f12`) failed on both and was discarded. Everything below
is from `f13` and its window rerun.

## Gates (`f13`)

- The replay equivalence check passes with 0 browser errors (seed-42
  `v1:82332ef108ec7e21` -> `v1:49a3738dca7ca496`, unchanged; release WASM
  SHA-256
  `04a4cf23281bc36d142f2a00ea4daf2bcbe195cc1e677a33a332c372dd240e49`,
  built by rustc 1.99.1).
- The Fleet window suite passes all 50 cases on Chromium 154.
- The Status window gate passes on Chromium 154.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.7 ms |
| Fast | 3601 | 0.5 ms | 1.7 ms | 5.0 ms |

- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors; 4 requests, all HTTP 200.
- 24 dual-AI campaigns are identical to the stage before.
- `cargo mutants --in-diff`: 17 mutants, 17 caught.
- `provenance-scan check`: 53 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
