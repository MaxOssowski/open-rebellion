# The informant incident runs on its timer (F-029)

Date: 2026-10-17.

The economy raised the informant incident whenever a held system was
short of regiments, and nothing followed from it but a telemetry record.
In the original it is a per-system timer, and it tells the other side
about the system. The reading is in `ghidra/notes/informant-incident.md`.

## The original

- `FUN_0050c820` arms timer `0x38e` while the system has a side. It is
  called from the system's setup (`FUN_0050e200`, record `+0x54`+`0x40`:
  GNPRTB 7703 (30) + rand(0..=7704 (970)) days) and from its side hook
  (`FUN_00510820`).
- `FUN_0050cbe0` fires the incident with `100 - support` percent: it runs
  `FUN_00559f90`, a failed `FUN_0053e2f0(holder's support)`. It then
  pulses `+0x88` bit 17, whose hook posts notification `0x153`
  (TEXTSTRA 29008-29009) and runs `FUN_0050d510`.
- `FUN_0050d510` draws INFORMTB at GNPRTB 7709 (1) + rand(0..=7710 (7))
  and passes the code to `FUN_0050d5a0` for the other side. That syncs a
  part of the system (fields, units, facilities, fleets or characters) to
  the side's view.

## Port

- `informant.rs`: `InformantState` (each held system's timer, save v66)
  and `InformantSystem::advance`.
- The headless simulation runs it after the uprisings. The integrator
  reveals the system in the fog it keeps when that side is the recipient,
  and records `informant_intel` with `cause: informants`.
- The app reveals the system in the recipient's fog and posts it the
  Informants message, with picture `0x3e8` (`FUN_00491060`).
- The economy no longer raises the informant flag; the field stays for the
  save layout.
- port: the port's fog knows systems, not their objects or fields, so
  every INFORMTB code reveals the system. The code is kept in the report.

## Tests

- `informant.rs`:
  - `a_held_system_s_timer_waits_thirty_to_a_thousand_days`, which also
    covers a neutral system and a lost side;
  - `informants_tell_the_other_side_with_a_hundred_less_support_percent`:
    a draw of 59 against support 60 gives nothing, 60 gives code 1, and the
    last row gives code 10;
  - `the_timer_repeats_from_the_day_it_fires`, with the redrawn delay
    (30 + 485);
  - `a_day_draws_within_its_roll_budget`.
- `integrator.rs`: `informants_reveal_the_system_to_the_side_they_tell`.
- Removed: `informant_incident_on_garrison_shortfall`, the test of the
  invented trigger.
- Neutralized, each failing its tests:
  - the chance reversed;
  - telling the holder;
  - keeping the timer of a system with no side;
  - a fixed reschedule.

## Campaigns (`playtest-informant.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `cbc32820`:

| Seed | Old incidents | Reports (recipient) | Codes |
|---|---|---|---|
| 1 | 15 | Alliance 2, Empire 2 | 7, 8, 9, 10 |
| 2 | 13 | Alliance 2 | 1, 2 |
| 3 | 16 | Alliance 2 | 3, 9 |
| 7 | 13 | Alliance 4, Empire 2 | 1, 4, 7, 8, 8, 10 |
| 13 | 5 | Empire 1 | 3 |
| 42 | 13 | Alliance 3, Empire 1 | 4, 7, 8, 9 |
| 47 | 16 | Alliance 2 | 1, 1 |

- The incident is now rare: a held system waits 30 to 1000 days, and well
  supported holders seldom leak.
- The campaigns diverge between days 3 and 156 (`cmpruns`), as the timers
  draw from the simulation's rolls.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`5a41d301b6bffbdbaebfff041ad993f5dd210acd62634367a900949c1238a99e`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.6 ms | 0.8 ms | 24.1 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 2.0 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2277 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the timer, its rolls and the
  save field): initial `v1:affc307d4c23c84d` -> `v1:5dc07c1880d3a9b6`,
  final `v1:21ecf34ac9e1f828` -> `v1:e212b88bcf82db21`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The
  replay equivalence check passes with 0 browser errors. Release WASM
  SHA-256 `c6f567d06565910a0344c5b1be05db16b1d83cc6cb68f1b8b28e99880171304f`.
- `cargo mutants --in-diff`: 41 mutants, 27 caught, 7 unviable, 7 missed
  in the first run.
  - Four were in `roll_budget`; `a_day_draws_within_its_roll_budget` now
    catches them.
  - Three are in the economy's resource incident (`evaluate_incident_flags`
    and the flag's rising edge). They were covered only by the removed
    informant test. That trigger has no source either and is the next part
    of F-029.
- `provenance-scan check`: 89 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
