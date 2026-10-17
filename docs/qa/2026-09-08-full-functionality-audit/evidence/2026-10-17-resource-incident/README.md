# The resource incident runs on its galaxy timer (F-029)

Date: 2026-10-17.

The port had two invented sources for notification `0x155`:
- the economy raised a "resource" incident when a system's facilities
  exceeded its energy or raw materials;
- it posted "New resources discovered" whenever a system's raw material
  output rose (`ResourceDiscovered`).

In the original, `0x155` is the resource incident, raised by a galaxy
timer. The reading is in `ghidra/notes/uprising-incident.md`, "Other
incidents".

## The original

- Event `0x390` (`FUN_00556fa0`) fires every GNPRTB 7719 (1) +
  rand(0..=7720 (499)) days.
- `FUN_00556be0` picks one existing system at random.
- `FUN_0050cc70` notes whether the system's side has a mine
  (`FUN_0052d690`) and any facility (`FUN_00539df0`, `0x20..0x30`).
- `FUN_00559ee0` draws RESRCTB (table `0x2b`) at rand(0..=99). Its rows
  0/25/50/75 give codes 1 to 4:
  1. with a mine, raw materials above GNPRTB 7711 (0) fall by one;
  2. with a facility, energy above 7713 (0) falls by one;
  3. raw materials below 7712 (15) and below the energy rise by one;
  4. energy below 7714 (15) rises by one.
- Only a change pulses bit 19 and stores the values. Notification `0x155`
  is New Resources or Resources Depleted (`FUN_0048e940`, picture `0x3ea`,
  TEXTSTRA `0x7194`..`0x719a`).

## Port

- `uprising.rs`, next to the disaster timer:
  - `UprisingState::next_resource_tick` (save v67);
  - `resolve_resource`;
  - `UprisingEvent::ResourceChanged`, which `apply_uprising_event` stores.
- The roll budget counts the timer's first delay, or its pick, table draw
  and next delay.
- The app posts the message with the original's wording; the integrator
  records `resource_discovery` with the code.
- Removed from the economy:
  - `ResourceDiscovered` and its arming field;
  - the overcap incident and `IncidentTriggered`;
  - `evaluate_incident_flags`, and `IncidentFlags`, now always empty
    (save v67).
- `EnergyOvercapped` and `RawMaterialOvercapped` (`FUN_0050ad60`) stay.

## Tests

- `uprising.rs`:
  - `each_resrctb_quarter_moves_raw_materials_or_energy_by_one`;
  - `a_resource_incident_that_changes_nothing_posts_nothing`: no mine or
    facility of the side, the other side's mine, and each floor and
    ceiling;
  - `the_resource_timer_waits_one_to_five_hundred_ticks_and_counts_its_rolls`;
  - `a_resource_change_sets_the_system_s_values`.
- The revolt test fixture `in_revolt` sets the resource timer to never,
  as it already did the disaster's. The roll budget test therefore keeps
  its counts.
- Removed: `k3_resource_discovery_fires_on_positive_delta_not_on_seed`, the
  test of the invented discovery.
- Neutralized, each failing its tests:
  - any side's mine;
  - no raw ceiling;
  - no energy check on raw;
  - posting an unchanged system;
  - no resource timer.

## Campaigns (`playtest-resource.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `1cc835d7`:

| Seed | Old overcap incidents | Old discoveries | Resource changes (by code) |
|---|---|---|---|
| 1 | 3 | 16 | 1: 1, 2: 1 |
| 2 | 3 | 14 | 4: 1 |
| 3 | 4 | 27 | 2: 1 |
| 7 | 1 | 9 | 4: 3 |
| 13 | 3 | 10 | 2: 1, 4: 1 |
| 42 | 2 | 14 | none |
| 47 | 3 | 12 | 3: 1, 4: 2 |

- The timer fires every 250 days on average, and many draws change
  nothing.
- The campaigns diverge between days 40 and 87.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`012c695aa0d825cbde675caed3119937491c07374f72d6301ddacf9190520449`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.7 ms | 23.8 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 2.1 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2280 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the timer, its rolls and the
  save layout): initial `v1:5dc07c1880d3a9b6` -> `v1:5f3d9fa492b06b98`,
  final `v1:e212b88bcf82db21` -> `v1:c02d4ee44749e195`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The
  replay equivalence check passes with 0 browser errors. Release WASM
  SHA-256 `7c6017c181715331ffa7ea56edd06fe58db9b449a789d83f5a16cfdf50f6885a`.
- `cargo mutants --in-diff`: 62 mutants, 58 caught, 3 unviable, 1 missed.
  The missed one, `+=` -> `*=` on the next delay, is now caught by the
  timer test (tried).
- `provenance-scan check`: 88 uncited items, one fewer.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
