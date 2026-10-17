# Luke and Leia gain Force by evading, and mission injuries land (F-038)

Date: 2026-10-18.

The previous step read how the Force grows (`ghidra/notes/force-awareness.md`,
"Force growth"). This step ports the smallest part. A defect from the
injury step came to light along the way.

## The original

- When a mission member evades capture (the RLEVADTB roll, `FUN_005349e0`),
  slot `+0x208` runs. For every character, the base `FUN_004ef450` asks to
  resign and rolls the injury (slot `+0x2e4`).
- Luke's class (`0x00665fd8`) and Leia's (`0x006663e0`) override it
  (`FUN_0056f8b0`, `FUN_0056ffa0`). They run the base, then add GNPRTB 3081
  (Luke, 1) or 3082 (Leia, 1) to `ForceExperience` `+0x8e`
  (`FUN_004ee150`, clamped to `0..=0x7fff`).
- Being captured (slot `+0x20c`) has no override, so no gain.
- The bounty hunters (`FUN_00575de0`) and the Luke-Vader encounter
  (`FUN_00560200`) use the same roll. Neither is ported.

## Port

- `MissionEffect::ForceExperienceGained`. The mission detection's evade
  pushes it for Luke (`0x242`) and Leia (`0x241`) after the injury roll.
- `Character::add_force_experience`.
- `missions::apply_advance_effects` applies it, and now also
  `CharacterInjured`.
- Leia gains Force before she is aware of it, as in the original. Her
  `+0x8e` is then no longer 0, so the awareness roll (`FUN_004eee60`) will
  not replace it.
- The headless simulation records `character_health` (cause
  `mission_injury`) and `force_experience` for these effects. The app
  posts Character Injured for a detection injury.

## Defect fixed: mission injuries were dropped

The injury step (2026-10-17, `evidence/2026-10-17-character-injury/`)
applied `CharacterInjured` only when a mission result carried it. The
detection that rolls injuries puts its effects in the advance, and
`apply_advance_effects` ignored them. No mission injury ever reached a
character, and the Injured message for one never showed.

Now the advance applies them. Over seven 600-day campaigns, 19 mission
injuries land, and three seeds gain healing days (122, 17 and 35). Before,
none of these seeds had either. The injury step's own tests called the
result path directly, so they passed.

## Tests

- `mission_detection.rs`:
  - `luke_and_leia_gain_force_experience_when_they_evade`: Luke, Leia and a
    plain character; the gain follows the injury;
  - `a_luke_captured_gains_no_force_experience`.
- `missions.rs`: `detection_effects_injure_and_raise_the_force_experience`
  (and the clamp).
- `integrator.rs`: `the_detection_s_injuries_and_force_gains_are_recorded`.
- `main.rs`: `a_detection_injury_posts_character_injured`.
- Shown failing without the change: no gain on the evade, or no injury
  arm in `apply_advance_effects`.

## Campaigns (`playtest-evade-force.json`)

`rebellion-playtest data/base --dual-ai`, against `fc5c6602`.
- Seeds 2 and 3 play the same 600 days; the others diverge from day 98 to
  431, at the first landed injury or Force gain.
- Leia evades and gains Force:
  - 600 days: 3 times on seed 1, 5 on seed 7, 2 on seed 42 and 6 on seed
    47;
  - 3000 days: 11 times on seed 1 (Force 11).
- Luke is never detected in these runs.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`cc09e9ce58cf50d52e2e4071a02d2dcb8398248d3d5531c15bdc938e4762e28b`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 25.6 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 3.8 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2310 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- The seed-42 golden is unchanged (initial `v1:58473b5bf4582374`, final
  `v1:461650933b75014c`). Its 600-day replay deals no mission injury and
  no evade by Luke or Leia. Two recordings are identical, and the replay
  equivalence check passes with 0 browser errors.
- Release WASM SHA-256
  `56de42c34b40cdc3e1bc4337484691161a928cddaa700b35556d16a421f29dff`.
- `cargo mutants --in-diff`:
  - first run: 27 mutants, 4 missed. Two were the `injury > 0` filters,
    now caught by tests that put the zero injury first.
  - second run over the app and integrator: 15 mutants, 12 caught, 1
    unviable, 2 missed. Those are `main` and `apply_mission_result`; the
    latter opens an audio device, so the app's tests do not call it.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the changed
  lines.

Hashes are in `SHA256SUMS`.
