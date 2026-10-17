# The smuggling percent replaces the support drift (F-037, step 1)

Date: 2026-10-17.

The economy's step 1 moved each held system's support every day by
`FUN_00559c40`. That function computes the system's SmugglingPercent
(`+0x6c`, setter `FUN_00509ec0`); the original never moves support with it
(`ghidra/notes/economy-cadence.md`). By the user's decision ("Etapami,
wiernie"), this step removes the drift and ports the field.

## The original

`FUN_0050b230(system)` takes the holder from `+0x24` bits 6-7 and passes
`FUN_00559c40`:
- the holder's support (`FUN_00507270`);
- `+0x88` bit 11;
- the capital ships in its fleets (`FUN_00509090`, weight GNPRTB 7686 = 10);
- its squadrons on the planet, not those aboard fleets (`FUN_00508fb0`,
  7687 = 5);
- its regiments (`FUN_00509020`, 7688 = 2), times GNPRTB 7680 (2) for the
  Empire with bit 11 (`FUN_00559b60`);
- the Death Stars in its fleets (`FUN_00509140`, CAPSHPSD family `0x18`).

With no Death Star and support at most GNPRTB 7732 (40), the result is a
base by support band (75 up to 20, 50 up to 30, else 25) less the weighted
forces, clamped to 0..100. It is negative for the Empire, and 0 for a
system no side holds. `FUN_00509ec0` stores it and notifies its views only
when it changes.

## Port

- `economy::smuggling_formula` (`FUN_00559c40`) and
  `economy::smuggling_percent` (`FUN_0050b230`).
- Step 1 stores the value in `SystemEconomy::smuggling_percent` and emits
  `EconomyEvent::SmugglingPercentChanged` on a change; the telemetry event
  `support_drift` becomes `smuggling_percent`.
- Removed: `calculate_support_drift`, `EconomyEvent::SupportDrifted` and
  its handling in the integrator and the app, and the unproduced
  `GameEffect::SupportDrifted`.
- The port had counted the squadrons aboard fleets, used the number of
  fleets in place of capital ships, gated on any own fleet in place of a
  Death Star, and raised each weight to at least 1. All four are fixed.
- port: the economy loop skips unpopulated systems, so one held only by
  regiments keeps 0. Bit 11 is still the port's `strong_support` guess.
- Not ported: the readers of the field, the discarded manufacturing roll
  and the Smuggling message `0x145`.

Save format: v60 -> v61 (`EconomyState` gains the field; bincode is
positional, no migration).

## Tests

- New in `economy.rs`:
  - `smuggling_takes_the_support_band_s_base_less_the_weighted_forces`;
  - `the_empire_smuggles_negative_and_counts_its_regiments_twice_with_bit_11`;
  - `a_death_star_in_the_holder_s_fleets_stops_smuggling`;
  - `smuggling_counts_the_holder_s_ships_planet_squadrons_and_regiments`;
  - `a_system_no_side_holds_smuggles_nothing`;
  - `the_economy_stores_the_smuggling_percent_and_leaves_the_support_alone`.
- New in `integrator.rs`: `a_smuggling_change_leaves_the_support_alone`.
- Removed with the drift: five `support_drift_*` tests in `economy.rs` and
  `support_drift_moves_both_sides_popularity` in `integrator.rs`. They
  asserted the invented support change.
- Neutralized, each failing its tests: counting the squadrons aboard
  fleets, dropping the Death Star gate, notifying every day, giving a
  neutral system a side, and moving support in the integrator.

## Campaigns (`playtest-smuggling.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `6646fa3b`:

| Seed | Control changes | Uprisings begun | Uprising incidents | Smuggling range |
|---|---|---|---|---|
| 1 | 18 -> 14 | 7 -> 6 | 40 -> 33 | -50..0 |
| 2 | 12 -> 13 | 6 -> 4 | 28 -> 17 | -19..0 |
| 3 | 17 -> 20 | 9 -> 9 | 59 -> 55 | -75..0 |
| 7 | 16 -> 15 | 7 -> 7 | 41 -> 36 | -19..0 |
| 13 | 13 -> 13 | 3 -> 4 | 20 -> 23 | -25..0 |
| 42 | 13 -> 15 | 9 -> 8 | 36 -> 36 | -50..0 |
| 47 | 10 -> 16 | 7 -> 10 | 31 -> 52 | -46..19 |

- The daily `support_drift` events (15 to 526 a run) are gone; the
  smuggling percent changes 3 to 23 times a run.
- One system changes hands 3 or more times (Sluis Van, seed 3), weeks
  apart: the Empire's regiments take it and the Alliance's support takes it
  back. Before, Sullust (seed 1) and Umgul (seed 13) did the same.
- Support now moves only through missions, uprisings and events, until
  step 2 ports the timers `0x384` and `0x386`.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`24cada7607a54a91510d8476d9226b2132264d4268ad6a10297aa12dffa95796`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.7 ms | 23.1 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 2.2 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2246 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the drift removed and the new
  save field): initial `v1:54adafc27a3c30ff` -> `v1:50660a69e8e81fc4`,
  final `v1:6f20d000b67b9f3a` -> `v1:8790fcc409096722`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The replay
  equivalence check passes with 0 browser errors. Release WASM SHA-256
  `6d86a185e4d61c5cdd2570f679d3b7bd072c0420186733ea477ca8c3591c1b4e`.
- `cargo mutants --in-diff`: 47 mutants, 41 caught, 3 unviable, 3 missed.
  - `*` -> `+` in the bit-11 doubling: 2 x 2 = 2 + 2 in the test. The test
    now uses 3 regiments and catches it (tried).
  - `+=` -> `-=` on the Death Star count: equivalent, since the formula only
    tests the count against 0.
  - Deleting `GameEffect::invert`'s `ShipRepaired` arm: existing code next
    to the edit, not changed here.
- `provenance-scan check`: 90 uncited items, 9 fewer.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
