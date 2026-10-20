# The Death Star has no separate construction timer (F-017, save v94)

Date: 2026-10-20. Stage DS2 of the "Gwiazda Śmierci" area, on the user's
go-ahead.

## The original

- The Death Star is CAPSHPSD record 136 (TEXTSTRA 10120 "Death Star"):
  - an Empire capital ship with refined cost 584, maintenance 600,
    research order 0 and hull 25,000;
  - it lists production families 40/41, like the other capital ships.
- A shipyard builds it as it builds any ship.
- The 2026-09-26 search found no Death Star construction notifier in the
  binary (F-017).
- The manual (p. 124, "The Death Star") says it "takes immense resources
  and a long time to deploy". The record's cost and the ordinary build
  time give both.

## What the port did before

`DeathStarState::under_construction`, `start_construction` and
`clear_construction` counted down an uncited 1,825 ticks
(`DEATH_STAR_CONSTRUCTION_TICKS`) and emitted `ConstructionCompleted`.
Several things reacted to that event:
- the integrator's `death_star_construction` telemetry;
- the app's "Death Star construction complete" message;
- the Death Star panel's progress view, and its "the Empire is
  constructing" view.

Nothing outside one telemetry fixture started that timer. The real build
already went through manufacturing, and `apply_build_completion_inner`
marks the new fleet (test `a_completed_death_star_can_fire_from_its_new_fleet`).

## Port

- All of the above is removed. The Death Star is built and completed only
  through manufacturing.
- `cleanup_destroyed_system` and the app's `destroy_planet` no longer take
  the Death Star state, which only cleared that timer.
- `scripts/eval_parity.py` no longer requires `death_star_construction`.
- The docs (`agent_docs/systems/death-star.md`, `simulation.md`,
  `save-load.md`) and the coverage map follow.

## Save v94

- `DeathStarState` loses a field, so the bincode layout changes:
  `SAVE_VERSION` 93 -> 94, and older saves are rejected (the no-migration
  rule).
- The seed-42 golden is regenerated for this cause:
  - initial `v1:57cfc0d727199c15` -> `v1:f47443f73b78e129`;
  - final `v1:4dee6ff02fdbe5bc` -> `v1:caadbc1bab82cbda`;
  - two recordings are identical, and only the fingerprints changed.

## Tests

- The three tests of the removed timer go with it. They are not weakened:
  the behaviour they asserted has no source.
- The build path stays covered by
  `a_completed_death_star_can_fire_from_its_new_fleet`.
- `cargo test --workspace`: 2494 passed, 0 failed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all 24
are identical to the stage before. No campaign ever started the timer.

## Gates

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `0abddc5d891a8612cbfe9d1152867664387a4dfb30965521746c6fe36eb10521`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.8 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.6 ms |

- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors; 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 4 mutants, 2 caught, 1 unviable, 1 missed.
  - The missed one replaces the panel's `draw_alliance_view` with `()`.
  - That is egui drawing code with no test harness; this stage only
    deleted its "under construction" branch.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
