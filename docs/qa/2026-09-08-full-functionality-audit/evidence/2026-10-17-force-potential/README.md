# Force potential, awareness and the Force base (F-038, stage 1)

Date: 2026-10-17.

The port rolled every character's Force template (`jedi_level`) into a
Force level at game start, so every minor character fought with a Force
share of 10 to 50. It also made a character with a passed
`jedi_probability` roll Force aware (`force_tier = Aware`). The reading
is in `ghidra/notes/force-awareness.md`.

## The original

- At creation (slot `+0x114`, `FUN_004f11f0`), after the skills,
  `ForceAware` (`+0xac` bit `0x10`) is the class `is_known_jedi`.
  `ForcePotential` (bit `0x20`) is a `FUN_0053e2f0` roll on the class
  `jedi_probability`: rand(0..=99) < p.
- `FUN_004eee60` gives the Force base `+0x8e` only to a character with
  both flags and no base yet: class base `+0xa0` plus rand(0..=variance
  `+0xa4`) (`FUN_0053e290`), clamped to `0..=0x7fff` (`FUN_004ee150`). It
  runs at creation and from both flags' hooks (slots `+0x358`, `+0x35c`,
  `FUN_004f1b30`).
- The Force level `+0x8c` is `+0x8e + +0x90` (`FUN_004eee40`, the base's
  hook, slot `+0x320`). Its own hook (slot `+0x31c`, `FUN_004f16a0`)
  recomputes the effective diplomacy, espionage and combat and the
  `FastHeal` and `DiscoveringForceUser` flags.
- The Status window's Force Ranking reads `+0x8c` (`FUN_004486f0`).

So at the start only Luke (Force 50), Vader (150) and the Emperor (120)
have a Force level. Leia has the potential (100 percent) but is not
aware, so her Force stays 0 until someone discovers it (timer `0x389`,
stage 2). The minor characters roll the potential at 10 or 20 percent and
stay unaware.

## Port

- `Character::force_potential`, `force_aware` and `force_base`
  (save v69). `force_level()` is the base; `+0x90` is not read yet and
  stays 0 (tagged `port:`).
- `force_base_due()` and `roll_force_base()` are `FUN_004eee60`'s guard and
  roll.
- The DAT loader sets `force_aware` from `is_known_jedi`. Seeding rolls
  `force_potential` for every character and, when due, the Force base.
  The class template `jedi_level` is no longer rolled in place; it stays
  the template the later rolls read.
- The effective combat, the healing timer's `FastHeal` and the Status
  window's Force Ranking read `force_level()`. The story events'
  `JediLevel` skill change now moves the Force base.
- Potential no longer makes a character Force aware (`force_tier`).
  `force_tier` and the rest of `jedi.rs` are unchanged. Removing that link
  changes nothing in the seven campaigns (variant `var` below).
- Not in this stage: the Force bonus on diplomacy and espionage
  (`FUN_004eecb0`, `FUN_004eecf0`), timer `0x389`, and `+0x90`.

## Tests

- `seeds.rs`:
  - `game_start_rolls_force_potential_and_a_force_base_only_for_the_aware`.
    It replaces `game_start_makes_characters_force_aware_by_their_jedi_probability`,
    whose expectation (potential makes a character aware) `FUN_004f11f0`
    contradicts: the two flags are set separately.
  - `a_zero_jedi_probability_never_gives_force_potential`.
  - `the_force_base_roll_draws_across_the_class_variance`.
- `world/mod.rs`:
  - `only_a_potential_and_aware_character_without_a_force_base_rolls_one`;
  - `the_force_base_roll_adds_the_draw_to_the_class_base_up_to_0x7fff`;
  - `effective_combat_adds_the_force_share_and_takes_the_injury` now sets
    the Force base, and adds a case where the class template alone gives
    no Force share.
- `healing.rs`: `a_force_level_of_eighty_heals_two_points_a_day` sets the
  Force base. Its two rolled-template cases are replaced by one where a
  template of 90 with no base heals slowly.
- `integrator.rs`: `a_story_jedi_level_change_moves_the_force_base`.
- Fixtures that gave a Force level through `jedi_level` now give it through
  `force_base`, with unchanged assertions:
  - `missions.rs`: `a_mission_reads_a_character_s_effective_combat`;
  - `status_window.rs`: the Figure 3.46 Luke is now potential and aware
    with base 50. MJCHARSD 578 gives Luke combat 90 and Force 50, the very
    pair the fixture uses. The character-injury README said Luke starts
    at combat 50 and Force 10; those are Leia's (577).
- Shown failing without the change:
  - seeding without the potential roll fails both seeding tests;
  - reading the template as the Force level fails the effective combat
    and healing tests.

## Campaigns (`playtest-force.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `dd2b360c`.
Every seed differs from tick 1, because seeding now draws the potential
for every character and the Force base only for the aware.

| Seed | 1 | 2 | 3 | 7 | 13 | 42 | 47 |
|---|---|---|---|---|---|---|---|
| Space battles, old -> new | 1 -> 8 | 5 -> 28 | 2 -> 20 | 6 -> 6 | 5 -> 6 | 2 -> 17 | 7 -> 24 |
| Mission successes, old -> new | 396 -> 332 | 335 -> 324 | 370 -> 363 | 323 -> 372 | 459 -> 350 | 334 -> 380 | 266 -> 379 |

- Two variants test the cause.
  - `var` also sets `force_tier = Aware` on potential, as before. It plays
    exactly the same games as the change.
  - `varb` rolls a Force base for every character, as before. It gives 66
    space battles against 109 for the change and 28 before.
- The battle count reacts strongly to small changes. The Force share of
  the minor characters (gone now) moves the missions, and through them
  the war.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`e528383dffa595739265225dd8ff06bde672c13c2693ca2ba7860277e83290df`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 23.5 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.7 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2293 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the Force flags and base,
  save v69): initial `v1:27f93b3cb81b6605` -> `v1:9c1efa24f4e2cd19`,
  final `v1:29ab46197f1887c6` -> `v1:aa49f78d3f06cc2f`.
  - Two recordings are identical, and only the fingerprints changed in the
    manifest.
  - The replay equivalence check passes with 0 browser errors.
  - Release WASM SHA-256
    `d699358ae9e8453f9f398e9af77150da14f35a9ebf7a576e26e5d7dbc70404e3`.
- `cargo mutants --in-diff`: 24 mutants.
  - The first run missed 8: the guard and roll had only data-crate tests,
    plus the `<` of the potential roll and the integrator's action.
  - With the tests above, the second run caught 22 with 2 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the changed
  lines.

Hashes are in `SHA256SUMS`.
