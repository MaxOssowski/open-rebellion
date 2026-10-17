# The Force share of diplomacy and espionage (F-038, Force stage 1b)

Date: 2026-10-18.

The port gave the Force share only to combat. A mission read a
character's diplomacy and espionage as the plain skill. The reading is
in `ghidra/notes/force-awareness.md`.

## The original

- The Force level's hook (slot `+0x31c`, `FUN_004f16a0`) recomputes the
  effective diplomacy `+0x7c` (`FUN_004eecb0`, from `+0x58`) and espionage
  `+0x7e` (`FUN_004eecf0`, from `+0x5a`) as skill + skill × Force level /
  100 (`FUN_0053e940`). The setters `FUN_004edcb0` and `FUN_004edd30` clamp
  to `0..=0x7fff` and notify `CharacterEnhancedDiplomacyNotif` and
  `CharacterEnhancedEspionageNotif` (slots `+0x374`, `+0x378`).
- Slots `+0x1dc` (`FUN_004edbf0`) and `+0x1e0` (`FUN_004edc00`) return
  them. The Diplomacy mission's chance reads the member's slot `+0x1dc`
  (`FUN_00573ff0`), Espionage's reads `+0x1e0` (`FUN_00573090`), and the
  captive escape reads `+0x1e0` (`captive-escape.md`). The Status window's
  Diplomacy and Espionage Ratings show them (`FUN_004486f0`).
- Leadership's share reads `+0x92` (`FUN_004eede0`), not the Force; it is
  not read yet.
- Check against the manual: MJCHARSD 578 gives Luke diplomacy 50,
  espionage 50, combat 90 and Force 50. With the shares these are 75, 75
  and 135, the values of the manual's Figure 3.46 (p. 101).

## Port

- `Character::effective_skill(skill)`: diplomacy and espionage with the
  Force share, combat through `effective_combat`, the others the skill
  itself (port: base plus half an unrolled variance, as before).
- `missions::member_skill` reads `effective_skill` for every skill, so
  missions, mission detection, the captive escape and the AI's mission
  planning all see the effective values.
- The Status window's Diplomacy and Espionage Ratings show them.
- Only Luke, Vader and the Emperor have a Force level at the start (stage
  1), so only their diplomacy and espionage change.
- No save change: the effective values are computed, not stored.

## Tests

- `world/mod.rs`: `diplomacy_and_espionage_gain_the_force_share` (Luke's
  data and Figure 3.46, leadership without a share, the clamp, combat).
- `missions.rs`: `a_mission_reads_a_character_s_effective_skills` (renamed
  from `..._effective_combat`). Its diplomacy expectation changes from 65
  to 97 (65 + 65 × 50 / 100). The old value asserted that diplomacy skips
  the Force share. `FUN_00573ff0` reads slot `+0x1dc`, the effective value,
  so that expectation was wrong. A leadership case keeps the plain value.
- `status_window.rs`: the Figure 3.46 fixture now carries Luke's own data,
  diplomacy and espionage 50 (it had 75, the figure's values). The
  assertion of 75/75/135 is unchanged.
- Shown failing without the change: dropping the share fails all three
  tests.

## Campaigns (`playtest-force-share.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `137f2480`.
Every seed differs from day 7 to 9, the first missions of a Force user.

| Seed | 1 | 2 | 3 | 7 | 13 | 42 | 47 |
|---|---|---|---|---|---|---|---|
| Diplomacy successes, old -> new | 67 -> 45 | 57 -> 50 | 58 -> 85 | 61 -> 55 | 61 -> 58 | 86 -> 71 | 72 -> 55 |
| Espionage successes, old -> new | 40 -> 40 | 50 -> 42 | 49 -> 34 | 45 -> 54 | 49 -> 43 | 39 -> 56 | 46 -> 58 |

The games diverge after the first change, so the counts move both ways.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`f7f90997b3fd64a8b26eb91f360abb6a1a5d5f4f80d001437dee4588fc51f4b7`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 26.1 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 3.9 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The Status window was not opened in the browser; its rows are covered
  by the unit test above.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2294 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the Force share of the
  mission skills):
  - the initial fingerprint is unchanged (`v1:9c1efa24f4e2cd19`); the first
    four checkpoints match;
  - final `v1:aa49f78d3f06cc2f` -> `v1:5531248a4edc88bb`;
  - two recordings are identical, and only the fingerprints changed in the
    manifest;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `d2da35161dc679ca2c2240fced336275f164b377ad989ce273faef2016b6af76`.
- `cargo mutants --in-diff`: 20 mutants, 19 caught, 1 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the changed
  lines.

Hashes are in `SHA256SUMS`.
