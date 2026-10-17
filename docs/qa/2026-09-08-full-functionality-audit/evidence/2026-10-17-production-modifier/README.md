# ProductionModifier: its value and its reach (F-037)

Date: 2026-10-17.

The port computed a "collection rate" that only telemetry read, and gave
200 to a system with no holder. Reading its hook showed the field is the
system's ProductionModifier (`+0x70`) and that it sets the pace of the
system's mines and refineries.

## The original

- `FUN_0050b2c0` stores 100 when the side bits name no side. Otherwise it
  stores `FUN_00559c10(FUN_00507270(side))`: GNPRTB 7763 (100) times 100
  (`DAT_00661a88`) over the holder's support, with support 0 counted as 1,
  in integers (`FUN_0053e170`, `FUN_0053e150`). The setter
  `FUN_00509f40` accepts any value of 0 or more.
- The field's hook (System slot `+0x1e8`, `FUN_00510b60`) runs
  `FUN_0050c2b0`. That walks the system's processing facilities
  (`FUN_0055b260`, families `0x2c..0x30`, mines and refineries) and gives
  each the system's value (`FUN_0050c560` -> `FUN_0055a7a0`, facility
  `+0x6c`). A facility joining the system gets it too (`FUN_00508660`).
  Shipyards are not in the walk.
- The facility's hook runs `FUN_0055abf0`, which writes the next period,
  `FUN_0055e2b0`: `(rate + extra) × modifier / 100`, into the cycle
  timer's record without cancelling the running timer (disassembly
  `0x55ac10`..`0x55ac68`). A change therefore applies from the next
  cycle.

At 100 support a mine's cycle takes its base length, at 50 twice that, and
at 10 ten times. A system with no holder runs at 100.

## Port

- `economy::production_modifier` is `FUN_0050b2c0` with `FUN_00559c10`. A
  system in revolt keeps its holder's side. The old `.max(1)` guards on
  GNPRTB 7763 and the result are gone.
- `economy::production_modifier_at` reads the stored value, or 100 where
  the economy has not run yet (port).
- `ProductionSystem::advance` takes a `modifier` per system. The
  simulation and the app pass the economy's value, and a mine's or
  refinery's cycle takes it when the cycle starts.
- The field keeps its name `collection_rate` and its f32 type, so the save
  stays v63.

## Tests

- `collection_rate_high_support`, `collection_rate_low_support` and
  `collection_rate_at_zero_support` now call `production_modifier`. Their
  values (125, 1000, 10000) are unchanged, and the first also checks the
  integer division (10000 / 70 = 142).
- New:
  - `a_system_without_a_holder_has_a_production_modifier_of_100`, which
    also covers a revolt and a system the economy has not run;
  - `a_mine_takes_its_system_s_production_modifier_when_a_cycle_starts`
    (`production_cycle.rs`). At modifier 200 the cycle takes 10 days. A
    change to 100 applies from the next cycle.
- Neutralized, each failing its tests:
  - a fixed modifier of 100 in the cycle;
  - 200 with no holder;
  - the simulation passing 100. The seed-42 replay catches this one,
    once its golden was regenerated.

## Campaigns (`playtest-production-modifier.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `95ce5276`:

| Seed | Builds completed | Control changes | Uprisings begun | Space battles |
|---|---|---|---|---|
| 1 | 468 -> 431 | 17 -> 25 | 4 -> 5 | 2 -> 2 |
| 2 | 406 -> 373 | 17 -> 17 | 5 -> 8 | 1 -> 4 |
| 3 | 487 -> 454 | 34 -> 29 | 13 -> 11 | 1 -> 1 |
| 7 | 383 -> 328 | 15 -> 20 | 8 -> 8 | 8 -> 9 |
| 13 | 601 -> 490 | 19 -> 10 | 6 -> 3 | 4 -> 12 |
| 42 | 454 -> 371 | 18 -> 26 | 11 -> 11 | 2 -> 2 |
| 47 | 451 -> 455 | 26 -> 17 | 14 -> 6 | 3 -> 4 |

Mines and refineries in systems below 100 support now run slower, so
fewer refined materials reach the yards. Builds fall by up to 19% (seed
13), and seed 47 is unchanged. The other counts move because the
campaigns diverge.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`504b95252bee3f827018ee959beda27e4eeeb9886b8b8de8080537efadf88f2e`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 24.9 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 2.1 ms |

- The resource bar after Fast reads 57 / 143 / 206; the previous step's
  capture read 36 / 184 / 206.
- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2261 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause. The initial fingerprint
  `v1:156fbf23b95e317e` and the first three checkpoints are unchanged. The
  final fingerprint goes `v1:653ff1b1bbff8904` -> `v1:99cb09fad4a57148`.
  Two recordings are identical, and only the fingerprints changed in the
  manifest. The replay equivalence check passes with 0 browser errors.
  Release WASM SHA-256
  `c7717dbd32915155c347965d2e59b28844412132ea89ad0a46f689b6c3420792`.
- `cargo mutants --in-diff`: 16 mutants, 14 caught, 2 unviable, 0 missed.
- `provenance-scan check`: 90 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
