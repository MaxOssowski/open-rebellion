# Force discovery, timer 0x389 (F-038, Force stage 2)

Date: 2026-10-18.

The port had no Force discovery. Characters with the Force potential
stayed unaware for the whole game, unless the port's own story events
changed them. The reading is in `ghidra/notes/force-awareness.md`.

## The original

- `DiscoveringForceUser` (`+0xac` bit `0x80`, `FUN_004ef070`) holds for a
  character whose class trains Jedi (`+0xb8`), with Force level at least
  GNPRTB 2580 (80), usable, not on a mission and not captured.
- Its hook (`FUN_004f1b70`) arms timer `0x389` on record `+0x54`+`0x20`.
  `FUN_004ef750` fills that record with GNPRTB 2582 (1) + rand(0..=2583
  (0)) (disassembly `0x4ef7aa..0x4ef7c8`), so the timer fires every day.
- Each fire (`FUN_00560a30`) walks the characters of the trainer's side in
  its container. One that is free, not on a mission, not the trainer, with
  the potential and not aware becomes aware and records the trainer
  (`+0xa4`). Leia (`0x241`) is found only by Luke (`0x242`) once he knows
  his heritage (`+0xb0` bit 0, `KnowsHeritage`); she then learns hers.
- Awareness's hook rolls the Force base (`FUN_004eee60`).
- The discoverer field sends notification `0x362` (Future Jedi
  Discovered, or Leia Uses Force). Not ported in this stage.

## Two corrections

- The note had given timer `0x389` GNPRTB 2584/2585 (100 to 1100 days).
  The disassembly puts those in record `+0x28`, the captive escape timer
  `0x38a`; `0x389` uses 2582/2583, every day.
- MJCHARSD 640 (`0x280`, family `0x34`) is the Emperor, Force 150, not a
  trainer; 641 (`0x281`, family `0x35`) is Vader, Force 120, a trainer
  (TEXTSTRA names; `0x34000280` and `0x35000281` in the original's
  message code). The note and the stage-1 READMEs had swapped them. The
  trainer who discovers at the start is Vader, not the Emperor.

## Port

- `force_discovery.rs`:
  - `discovering_force_user`;
  - `ForceDiscoveryState`, one timer per trainer (save v70);
  - `ForceDiscoverySystem::advance` and `apply`.
- The container is the trainer's fleet, else the characters at its
  system that are in no fleet (port:, as `uprising::usable_at`).
- Luke's `KnowsHeritage` is the port's `heritage_known`, which only the
  port's story events set today.
- `Character::force_discoverer` keeps `+0xa4` as a record id.
- The headless simulation runs it after healing (step 9d) and records
  `force_discovered`; the app runs it too.

## Tests

`force_discovery.rs`:
- `a_trainer_with_force_eighty_finds_a_latent_character_beside_it_the_next_day`;
- `a_trainer_below_eighty_or_busy_discovers_nobody`;
- `only_free_latent_unaware_characters_of_its_side_beside_it_are_found`;
- `a_trainer_in_a_fleet_finds_its_shipmates_only`;
- `leia_is_found_only_by_luke_once_he_knows_his_heritage`;
- `the_timer_waits_its_record_s_minimum_and_spread`;
- `the_roll_budget_covers_two_delays_and_every_character_per_trainer`.

`integrator.rs`: `a_force_discovery_makes_the_character_aware_and_records_it`.

Shown failing without the change: with no character found, five of the
module's tests fail.

## Campaigns (`playtest-force-discovery.json`)

`rebellion-playtest data/base --dual-ai`, against `38c14fbf`.
- Seven 600-day seeds and five 3000-day seeds: no discovery.
- On the real data Vader qualifies from day 1 (Force 120, trainer, free at
  Coruscant). The Empire's latent characters start unrecruited, and once
  recruited the AI sends them on missions, so none stands free beside
  him. Luke stays at Force 50, as the Force growth `+0x90` is not ported.
- The campaigns still diverge from day 25 to 121, because the timer's
  rolls are drawn each day.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`b99c59c734e9dba6b8c293e8e7bb2144f1c3acee396cbab859bf0f51796a7157`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 24.6 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.4 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2302 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (timer `0x389`, save v70):
  - initial `v1:9c1efa24f4e2cd19` -> `v1:58473b5bf4582374`;
  - final `v1:5531248a4edc88bb` -> `v1:461650933b75014c`;
  - two recordings are identical, and only the fingerprints changed in the
    manifest;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `8270910fa876796534193a9a06a8d200c5fc3ad9efac5c6a6711debffd53a5c4`.
- `cargo mutants --in-diff`:
  - first run: 74 mutants, 3 missed. The roll budget's `+` and the
    integrator's record are now caught by the tests above.
    `convert_character` returning a default is the DAT loader's old gap,
    untouched here beyond the new field.
  - after the Clippy fixes, the module and integrator again: 67 mutants,
    66 caught, 1 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the changed
  lines.

Hashes are in `SHA256SUMS`.
