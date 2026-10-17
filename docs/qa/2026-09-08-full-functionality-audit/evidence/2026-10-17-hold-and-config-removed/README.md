# The fleet hold and three AI config fields removed

Date: 2026-10-17.

The user asked for both. They were left unused when the stand-in fleet
heuristics (`522555db`) and the automatic landing (`1215eca6`) went.

## Removed

- `TroopTransportState`:
  - the `held` list of fleets and `is_held`, `landing_count` and
    `release`;
  - `load`, which was `embark` plus the hold. Every caller now uses
    `embark`.
  The hold only kept the automatic landing off a fleet that loaded by
  order (`ghidra/notes/regiment-unload.md`, `fleet-window.md`).
- `apply_fleet_arrival` (`movement.rs`) no longer releases a hold.
- `AiConfig`'s `alliance_deploy_budget`, `empire_deploy_budget` and
  `ds_retreat_strength_ratio`. Only the removed fleet heuristics read them.
- The Fleet window fixture's report loses `held`, and `fleet-window.mjs`
  no longer checks it.

Still unused and kept, pending the user's decision: `AiConfig`'s
`weight_weakness`, `weight_proximity`, `weight_deconfliction`,
`weight_freshness` (the removed attack scoring) and `max_attack_fronts`.
Only tests that change a config value read `max_attack_fronts`.

Save format: v59 (`TroopTransportState` and `GameConfig` lose fields). A
config file that still names the three fields loads, because `AiConfig`
is `#[serde(default)]` without `deny_unknown_fields`.

## Tests that changed

- Removed, because they tested the hold alone:
  `embarking_for_a_departure_does_not_hold_the_cargo`,
  `releasing_a_hold_lets_the_cargo_land`,
  `landing_or_losing_the_cargo_ends_the_hold`.
- Renamed, with their hold assertions removed or replaced by the cargo's:
  - `a_loaded_regiment_is_held_aboard_at_the_system_it_loaded_at` →
    `an_embarked_regiment_leaves_the_planet_for_the_fleet`;
  - `a_held_fleet_absorbed_into_another_passes_on_no_hold` →
    `a_fleet_absorbed_into_another_passes_on_its_cargo`;
  - `unloading_one_regiment_leaves_the_rest_aboard_and_held` →
    `unloading_one_regiment_leaves_the_rest_aboard`;
  - `a_regiment_boarding_from_its_planet_is_held_aboard` →
    `a_regiment_boarding_from_its_planet_goes_aboard`;
  - `a_held_fleet_arriving_elsewhere_releases_its_cargo_to_land` →
    `a_fleet_arriving_elsewhere_keeps_its_cargo_aboard` (`movement.rs`);
  - in `simulation.rs`, the two cases of
    `a_regiment_loaded_by_order_stays_aboard_at_the_system_it_loaded_at`
    were the same once the hold went. They are now
    `a_carried_regiment_stays_aboard_at_its_system`.
- Hold assertions dropped from: `fleet_join.rs` (create fleet), the
  fixture's loading tests, and three `troop_transport.rs` tests.
- New: `unloading_the_last_regiment_leaves_no_cargo_behind`. It catches
  the one mutant that survived, `forget_if_empty` emptied, and fails with
  that change neutralized.

## Campaigns

`rebellion-playtest data/base --ticks 600 --dual-ai` for seeds 1, 2, 3, 7,
13, 42 and 47, compared with `1215eca6`: every event stream is identical
apart from timing (`playtest-identical.txt`).

## Browser

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast. Each
frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.6 ms | 0.8 ms | 23.7 ms |
| Fast | 60 s | 3601 | 0.6 ms | 1.5 ms | 3.1 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The Fleet window gate passes all 48 cases (interface WASM
  `efd35a0e1725a8769f785aa9f686296cef238e6fb13a526e2401a689deb96926`).
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2244 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for save v59:
  - `v1:51f96ff9faae705b` -> `v1:e47089c0adbf9615`;
  - final `v1:3bb9a6063ff07d84` -> `v1:9234ac6ef0f26a8a`;
  - config fingerprint `4bfbb9c15c2d88a1` -> `b7c45d4690613fe6`.
  Two recordings are identical, and only the fingerprints changed in the
  manifest. The replay equivalence check passes with 0 browser errors.
  Release WASM SHA-256
  `6a523af62ddc81bb11794a5284646b4c8e88760db50dd6cf057b1023d2b9a271`.
- `cargo mutants --in-diff`: 14 mutants, 11 caught, 2 unviable, 1 missed.
  The miss is `forget_if_empty`, which the new test now catches.
- `provenance-scan check`: 100 uncited items, within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
