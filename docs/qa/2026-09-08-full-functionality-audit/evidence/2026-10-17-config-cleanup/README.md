# The remaining unused AI config fields removed

Date: 2026-10-17.

The user asked for the unused config fields to go. After `c4b8cfa4`,
seven fields of `AiConfig` were read by no code. The removed attack
scoring (`score_attack_target`, `evaluate_fleet_deployment`) had been
their only reader:
- `max_attack_fronts`;
- `battle_cooldown_ticks`;
- `proximity_divisor`;
- `weight_weakness`, `weight_proximity`, `weight_deconfliction` and
  `weight_freshness`.

## Change

- `rebellion-core/src/tuning.rs`: the seven fields are gone.
- `configs/autoresearch/default.json`: the seven keys are gone. The other
  config files did not name them. A file that still names them loads,
  because `AiConfig` is `#[serde(default)]` without
  `deny_unknown_fields`.
- The save and replay tests changed `max_attack_fronts` only to see a
  config change reach the fingerprint and the save. They now change
  `troop_garrison_min`. Their assertions are the same.
- Save format: v60 (`GameConfig` loses fields).

## Campaigns

`rebellion-playtest data/base --ticks 600 --dual-ai` for seeds 1, 2, 3, 7,
13, 42 and 47, compared with `73e0b054`: every event stream is identical
apart from timing (`playtest-identical.txt`).

## Gates

- `cargo test --workspace`: 2245 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for save v60:
  - `v1:e47089c0adbf9615` -> `v1:54adafc27a3c30ff`;
  - final `v1:ef80035d2be8099a` -> `v1:6f20d000b67b9f3a`;
  - the config fingerprint changes too.
  Two recordings are identical, and only the fingerprints changed in the
  manifest. The replay equivalence check passes with 0 browser errors.
- Release WASM SHA-256
  `bee12e7c755eff6767ca44f09f1cc82ef089e10288baec77948412afbca092f3`.
  The Fleet window gate passes all 48 cases (interface WASM
  `00a3aa5f7352d24a8028de2cbedd72abd4acf1818f441c6052e4929b0a6c0ef8`).
- Browser (release build, `agent-browser`, 640 by 480, a new Empire
  campaign):
  - 60 s at Very Slow: 3600 frames, median 0.5 ms, longest 22.4 ms;
  - 60 s at Fast: 3601 frames, median 0.6 ms, longest 2.1 ms;
  - no frame over 50 ms, 0 console errors, 4 requests all HTTP 200.
- `cargo mutants --in-diff`: no mutants, since the diff only removes.
- `provenance-scan check`: 99 uncited items, within the baseline.
- rustfmt: no findings in the changed lines.
- Native GUI acceptance was not run.

Hashes are in `SHA256SUMS`.
