# The support timers 0x384 and 0x386 (F-037, step 2)

Date: 2026-10-17.

After step 1 removed the invented daily drift, support moved only through
missions, uprising incidents and events. The original also moves it with
two per-system timers (`ghidra/notes/economy-cadence.md`). This step
ports them.

## The original

- **Arming.** `FUN_0053fa60` stores a flag's new value in the timer
  record's armed bit. When that changes the record, the old timer is
  cancelled and, if armed, a new one is scheduled. A fired timer
  reschedules with the same delay (`FUN_005862a0`).
- **Delays.** The system's setup `FUN_0050e200` sets each record's minimum.
  The decompile drops the record pointers, so they were read from the
  disassembly (`0x50e312`..`0x50e39b`):

  | Record | Timer | Minimum | Spread |
  |---|---|---|---|
  | `+0x54`+`0x18` | `0x383` | GNPRTB 7699 (20) | 7700 (20) |
  | `+0x54`+`0x20` | `0x384` | 7698 (30) | — |
  | `+0x54`+`0x28` | `0x385` | 7692 (1000) | — |
  | `+0x54`+`0x30` | `0x386` | 7694 (30) | — |
  | `+0x54`+`0x38` | `0x38d` | 7701 (30) | 7702 (70) |
  | `+0x54`+`0x40` | `0x38e` | 7703 (30) | 7704 (970) |

  The `0x383` and `0x385` records were confirmed at their arming sites
  (`0x5116e6`, `0x511685`).
- **`0x384`**, armed with `0x38d` by the uprising hook `FUN_00510f20`:
  in revolt, `FUN_0050caa0` shifts the holder's support by GNPRTB 7697 (-1)
  through `FUN_0050c9f0`.
- **`0x386`**, armed by the blockade hook `FUN_00511300`: while blockaded,
  `FUN_0050cb20` takes the side of the first fleet at the system
  (`FUN_00509890`). That side's support shifts by GNPRTB 7693 (1) when it
  has more support than the other side (`FUN_00509980` against
  `FUN_00509930`, which names neither on a tie), else by 7695 (-1).
  `FUN_005109f0` resets the minimum to 7694 or 7696 (both 30) when the
  regiments change.
- `FUN_0050c9f0` needs a populated system and divides a shift that favours
  the Alliance or hurts the Empire by GNPRTB 7681 (2) under `+0x88` bit 11
  (`FUN_00559be0`); the port's `uprising::support_change` already does this.

## Port

- `uprising.rs`: `ActiveUprising::next_support_tick`, and
  `UprisingEvent::SupportEroded` with the holder's change.
- `blockade.rs`: `BlockadeState`'s support timers are set when a blockade
  starts and dropped when it ends or the system is cleared.
  `BlockadeEvent::SupportShifted` carries the blockader's change.
- `uprising::shift_support` applies both changes, in the headless
  integrator (telemetry `uprising_support`, `blockade_support`) and in the
  app. As in the original, neither change posts a message.
- A change of 0 posts no event.
- port: the spread draw over `0..=0` takes no roll. `FUN_005109f0`'s reset
  to 7696 is not modelled; it has the same value.
- hyp: a spread `FUN_0050e200` does not set is 0.
- `0x385` is not ported: its change, GNPRTB 7691, is 0.

Save format: v61 -> v62 (`UprisingState` and `BlockadeState` gain the
timers; no migration).

## Tests

- New in `uprising.rs`:
  - `a_holder_in_revolt_loses_a_support_point_every_thirty_days`;
  - `a_strongly_supported_empire_loses_nothing_to_the_revolt_timer`;
  - `the_revolt_timer_takes_the_point_from_the_world`.
- New in `blockade.rs`:
  - `a_popular_blockader_gains_a_support_point_every_thirty_days`;
  - `an_unpopular_or_tied_blockader_loses_a_support_point`;
  - `lifting_the_blockade_cancels_its_support_timer`;
  - `clearing_a_destroyed_system_drops_its_blockade_and_support_timer`;
  - `every_blockade_event_reports_its_tick`.
- New in `integrator.rs`:
  `a_blockade_support_shift_moves_the_blockader_s_support`.
- Changed: `the_incident_timer_waits_thirty_to_one_hundred_ticks_and_repeats`
  counted one event on day 40. `FUN_00510f20` arms `0x384` on the same day
  as `0x38d`, so the 30-day support timer also fires on day 40. The test
  now checks both events by kind, and its incident checks are unchanged.
- `save.rs`'s round-trip test also checks the new revolt timer.
- Neutralized, each failing its tests:
  - always taking the popular shift;
  - counting a tie as popular;
  - keeping the timer after the blockade ends;
  - no revolt timer;
  - skipping `FUN_00559be0`;
  - the integrator not applying the shift.

## Campaigns (`playtest-support-timers.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `44d3d8e4`:

| Seed | Revolt shifts | Blockade shifts | Control changes | Uprisings begun | Incidents |
|---|---|---|---|---|---|
| 1 | 13 × -1 | 0 | 14 -> 15 | 6 -> 6 | 33 -> 27 |
| 2 | 0 | 0 | 13 -> 13 | 4 -> 4 | 17 -> 17 |
| 3 | 38 × -1 | 0 | 20 -> 19 | 9 -> 9 | 55 -> 59 |
| 7 | 22 × -1 | 0 | 15 -> 14 | 7 -> 8 | 36 -> 25 |
| 13 | 15 × -1 | 0 | 13 -> 14 | 4 -> 4 | 23 -> 22 |
| 42 | 19 × -1 | 0 | 15 -> 17 | 8 -> 9 | 36 -> 36 |
| 47 | 24 × -1 | 0 | 16 -> 16 | 10 -> 10 | 52 -> 52 |

- A revolt now wears down its holder. At Bothawui (seed 7) the Alliance
  lost a point every 30 days from day 40. On day 191 its support fell
  below 60, the system went neutral (`FUN_0050b610`), and the revolt
  ended.
- Seed 2's four revolts are all in Empire systems where the Empire has
  strong support, so FUN_00559be0 halves each -1 to 0.
- Blockade shifts: most blockades last 1 to 7 days. Two long ones
  (Bothawui in seed 7, 180 days; Selonia in seed 47, 346 days) fired the
  timer every 30 days (seen with a temporary print). Each time the Empire
  was the unpopular blockader of an Alliance system with strong support,
  so the -1 was halved to 0. Bit 11 is still the port's `strong_support`
  guess; this is recorded as open in `economy-cadence.md`.
- Systems changing hands 3 or more times: Sluis Van (seed 3), as before;
  Bothawui (seed 7) and Drall (seed 42), months apart.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`aa61b76e14378f2b45ac327b07e8c47af8ca4b32f0d909edf512be8c47a963b8`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 22.7 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 2.4 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2255 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the timers and the save
  fields): initial `v1:50660a69e8e81fc4` -> `v1:0dfc4521e1bcaab5`, final
  `v1:8790fcc409096722` -> `v1:07eb8ba0a9c5f765`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The
  replay equivalence check passes with 0 browser errors. Release WASM
  SHA-256 `d65f4db00b1f07729866efebc607e2b1d3e85577764f576445f395b6ccf4a86f`.
- `cargo mutants --in-diff`: 35 mutants, 28 caught, 3 unviable, 4 missed.
  - `BlockadeState::clear_blockade` and `BlockadeEvent::tick`: old code
    next to the edit, now covered by two new tests.
  - `main` of the app: not testable.
- `provenance-scan check`: 90 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines. Step 1 left one line in `economy.rs` formatted for
  edition 2024; it is fixed here.

Hashes are in `SHA256SUMS`.
