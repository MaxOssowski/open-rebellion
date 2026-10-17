# Character injury and healing, timer 0x388 (F-038)

Date: 2026-10-17.

The port computed a character's injury in two places, a mission member's
evasion or capture and the uprising incident's code 3, and then dropped
it: the world kept no injury. The reading is in
`ghidra/notes/character-injury.md`.

## The original

- Character `+0x94` is the injury. The injury roll (`FUN_0053e990`, slot
  `+0x2e4`) uses the effective combat `+0x86`, and slot `+0x2f0`
  (`FUN_004ef6d0`) adds the result, clamped to `0x7fff`.
- The effective combat is `+0x62 + +0x62 × Force level (+0x8c) / 100 -
  injury`, at least 0 (`FUN_004eed90` -> `FUN_0053e900`). The Status
  window's Combat Rating shows it, and it shows "Injured" while `+0x94` is
  not 0.
- **Healing.** While a character is with its own side (`CanHeal`,
  `FUN_004eef90`), not en route, and injured, its Healing flag
  (`FUN_004ef030`) arms timer `0x388`. The timer fires every GNPRTB 2561/2562
  (1) days and heals GNPRTB 2563 (2) points with `FastHeal` (Force level 80+,
  GNPRTB 2560, `FUN_004eeff0`), else 2564 (1) (`FUN_004ef100`).
- The injury hook also clears the command, asks to resign from a mission,
  and sets `Damaged`. For the class Character an injury never kills;
  a subclass at `0x00665be0` dies of any injury (GNPRTB 2577).

## Port

- `Character::injury` (save v68), `Character::injure`,
  `Character::effective_combat`.
- The missions' and the uprising incident's injuries are stored (headless
  integrator, app, `apply_uprising_event`).
- Combat is read as the effective combat by `missions::member_skill`, by
  the uprising incident's injury roll, and by the Status window, which now
  also shows "Injured".
- `healing.rs`: `HealingState` (save v68) and `HealingSystem::advance`. The
  simulation runs it after the informants (step 9c) and records
  `character_health` with `cause: healing`; the app runs it too.
- port: the combat and Force level are the rolled templates (base plus
  half the variance). A character's container is its fleet, else its
  system, whose side is its holder.
- Not ported yet: the resign request, the command clearing, `Damaged`,
  death by injury for the `0x00665be0` subclass, and the Character Injured
  / Healed messages (`0x106`).

## Tests

- `world/mod.rs`: `effective_combat_adds_the_force_share_and_takes_the_injury`.
- `missions.rs`: `a_mission_reads_a_character_s_effective_combat`.
- `healing.rs`:
  - `an_injured_character_at_home_heals_a_point_a_day`;
  - `a_force_level_of_eighty_heals_two_points_a_day`, with the rolled
    Force level;
  - `the_timer_waits_its_record_s_minimum_between_healings` (GNPRTB 2562 =
    3);
  - `only_a_free_character_among_its_own_side_heals`: an enemy system, no
    location, captured, its own fleet and the other side's fleet.
- `integrator.rs`:
  `a_mission_injury_adds_to_the_character_and_healing_records_it`.
- `status_window.rs`:
  - the status test also checks "Injured" between Captured and a mission;
  - the Figure 3.46 test's Luke had combat 135 at Force 40. The figure
    shows the effective combat 135 at Force rank Trainee (20..79). The
    fixture now gives combat 90 at Force 50, one pair that the recovered
    formula turns into 135. In the data Luke starts at combat 50 and Force
    10, so the figure shows a developed Luke. The assertion is unchanged.
- `world/mod.rs`: `serde_backward_compat_missing_new_fields` lists the new
  required field `injury`, as it lists `recruited`.
- Neutralized, each failing its tests:
  - healing anywhere;
  - no `FastHeal`;
  - healing at once;
  - no Force share;
  - the injury ignored in the effective combat.

## Campaigns (`playtest-injury.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `47c282d5`:
- Healing is rare. Only seed 7 has a healing character: 41 days for one
  character. Most injured characters are captured, and a captive does not
  heal.
- The campaigns diverge between days 89 and 492, because combat missions
  now use the effective combat (the Force share and the injury). Mission
  successes per run, old -> new:

  | Seed | 1 | 2 | 3 | 7 | 13 | 42 | 47 |
  |---|---|---|---|---|---|---|---|
  | Successes | 414 -> 396 | 325 -> 335 | 392 -> 370 | 343 -> 323 | 462 -> 459 | 344 -> 334 | 393 -> 266 |

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`0dd3c72b84578ffae9b5577ffffcfb843c2f849e96e4022760bf0d4c19afc08b`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.7 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 2.5 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2287 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the injury field, the
  healing timer and the effective combat): initial `v1:5f3d9fa492b06b98`
  -> `v1:27f93b3cb81b6605`, final `v1:c02d4ee44749e195` ->
  `v1:29ab46197f1887c6`. Two recordings are identical, and only the
  fingerprints changed in the manifest. The replay equivalence check
  passes with 0 browser errors. Release WASM SHA-256
  `c998f907912a65e0ee18cb2b62e5172acc7aa78177ff150e4f7a200792a8beba`.
- `cargo mutants --in-diff`: 68 mutants, 51 caught, 8 unviable, 9 missed
  in the first run.
  - Seven were in the rolled-template arithmetic and the timer's period,
    which the tests' variance of 0 and period of 1 hid. The variance and
    period cases above catch them; a second run over the two files missed
    one more, `/ 2` -> `* 2` in the Force level, caught by the 60 + 30 / 2
    case (tried).
  - `injury > 0` -> `>= 0` in `injure` is equivalent: adding 0 changes
    nothing.
  - `convert_character` returning `Default::default()` (`lib.rs`, the DAT
    loader that gained the field): an old gap, untouched here beyond the
    new field.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
