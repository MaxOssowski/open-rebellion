# Bit 11 is DeathStarNearby, and the system hooks (F-037, step 3a)

Date: 2026-10-17.

Step 3 recomputes the system's derived fields when their inputs change,
as the original's hooks do. Reading the hooks showed that one of their
inputs, `+0x88` bit 11, had been misread. This first part ports that bit
and records the hook map; the cadence itself follows in step 3b.

## The hooks

Each System setter notifies a master slot (`ghidra/generated/setters.tsv`),
and the System vtable `0x0065e640` holds that slot's hook
(`class-slots.tsv`). The full table is in
`ghidra/notes/economy-cadence.md`, "The hooks". Two earlier readings were
wrong:
- `FUN_005109f0` (slot `+0x1d0`) is the support's hook
  (`System_SetLoyalty`, `+0x58`), not the regiments'. It also resets timer
  `0x386`'s minimum when the support changes, not when the regiments do.
- `FUN_00511740` (slot `+0x238`) is bit 11's hook.

A unit joining or leaving a system runs `FUN_00508660` (slot `+0xd0`).
For regiments it recomputes the holder and the troop surplus, but not the
smuggling percent or the production modifier.

## Bit 11

`System_SetDeathStarNearby` (`FUN_0050a890`) writes `+0x88` bit `0x800`.
Its only caller, `FUN_005178b0`, walks the sectors. For each sector it
looks for an active Death Star (`0x18..0x1c`, `+0x50` bit 0) whose system
lies in that sector (`FUN_00518690`), and sets the bit on every system of
the sector to the answer. The Death Star's side does not matter. The
object move code (`FUN_00514510`, `FUN_00514a60`) runs it.

The port had read bit 11 as "the holder's support is above GNPRTB 7732
(40)". The bit drives four rules:
- the Empire's regiments count twice in the smuggling percent
  (`FUN_00559b60`);
- they count twice in the uprising score (`FUN_00559ce0`);
- the Empire's garrison requirement is halved (`FUN_00559fe0`);
- support shifts that favour the Alliance or hurt the Empire are halved
  (`FUN_00559be0`).

## Port

- `economy::death_star_nearby(world, sys)` replaces `is_strong_support`.
  It looks at the fleets standing at the sector's systems when it is
  read, which is what the move hooks keep up to date.
- `SystemSummary::strong_support` is renamed `death_star_nearby`. Bincode
  stores the same bool in the same place, so the save stays v62.
- The smuggling percent and the garrison requirement now read the bit for
  the current day, not the summary from the previous day.
- `compute_system_summary` loses its now unused GNPRTB arguments.
- Notes corrected: `economy-cadence.md`, `system-control.md`,
  `timer-scheduler.md`, `uprising-incident.md` and
  `agent_docs/systems/uprising.md`. The evidence for step 2
  (`2026-10-17-support-timers`) says `FUN_005109f0` resets the `0x386`
  minimum "when the regiments change"; the support changing is right.

## Tests

- New: `a_death_star_sets_bit_11_on_every_system_of_its_sector_only`
  (`FUN_005178b0`). It covers another sector, a Death Star of either side,
  a destroyed one, and support playing no part.
- Five `uprising.rs` tests set bit 11 by giving the holder 50 to 60
  support. They now set it with a Death Star in the sector, keep their
  expected values, and are renamed:
  - `an_empire_near_a_death_star_loses_nothing_to_the_revolt_timer`;
  - `support_changes_that_favour_the_alliance_near_a_death_star_are_halved`.
    It also checks that 50 support alone no longer halves a gain.
  - `support_changes_that_hurt_the_empire_near_a_death_star_are_halved`;
  - `an_empire_near_a_death_star_counts_each_regiment_twice`;
  - `the_incite_support_loss_near_a_death_star_is_halved_only_for_the_empire`.
- `empire_garrison_is_halved_only_when_support_is_strong` is renamed
  `empire_garrison_is_halved_only_with_a_death_star_nearby`. Its assertions
  are unchanged.
- Neutralized, each failing its tests:
  - counting a Death Star in any sector;
  - counting a destroyed one;
  - restoring the support rule.

## Campaigns (`playtest-death-star-nearby.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `c245b4d6`.
No Death Star exists in these 600 days, so bit 11 is now always clear. The
old rule had set it on every held system whose holder had more than 40
support.

| Seed | Revolt shifts | Control changes | Systems changing 3+ times | Uprisings begun | Incidents |
|---|---|---|---|---|---|
| 1 | 13 -> 39 | 15 -> 17 | 0 -> 2 | 6 -> 4 | 27 -> 18 |
| 2 | 0 -> 23 | 13 -> 17 | 0 -> 2 | 4 -> 5 | 17 -> 8 |
| 3 | 38 -> 86 | 19 -> 42 | 1 -> 8 | 9 -> 15 | 59 -> 39 |
| 7 | 22 -> 69 | 14 -> 18 | 1 -> 1 | 8 -> 7 | 25 -> 34 |
| 13 | 15 -> 32 | 14 -> 20 | 0 -> 3 | 4 -> 4 | 22 -> 15 |
| 42 | 19 -> 75 | 17 -> 18 | 1 -> 2 | 9 -> 9 | 36 -> 34 |
| 47 | 24 -> 89 | 16 -> 22 | 0 -> 2 | 10 -> 11 | 52 -> 38 |

- Blockade shifts now apply: 19 × -1 in seed 7 and 8 × -1 in seed 47,
  where step 2 had halved them all to 0.
- Revolts wear down their holders without halving, and the Empire needs
  its full garrison. Systems therefore go neutral more often. For
  example, in seed 3 the Empire won Corsin by Diplomacy on day 52 and it
  revolted the next day. The revolt timer took a point every 30 days
  until day 389, when the system went neutral and the revolt ended. The
  Empire won it again on day 525 and lost it on day 557. These changes are
  weeks or months apart, not the same-day clashes of `system-control`.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`3a8a9cefe0d299269c6144e96a567f92e8b5ce25e9f394a12db32b62acc8d930`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.1 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 2.0 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2256 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
  - `telemetry_coverage_all_sys_constants_emit` first failed: "repair"
    emitted no event in its 1000-day seed-42 campaign. That campaign
    builds a Death Star on day 100, so bit 11 now applies there. The
    campaign diverged from day 152 and fought 2 space battles instead of
    20, so no ship needed repair.
  - The seven 600-day campaigns show no such drop: repairs went 20 -> 12,
    3 -> 3, 157 -> 165 and 19 -> 19 where they occurred.
  - The test already plants fixtures so that the uprising, the Death Star
    and betrayal fire. It now also halves one ship's hull in a fleet that
    starts at a shipyard. Its assertion is unchanged, and it passes.
- Seed-42 golden regenerated for this cause. The initial fingerprint
  `v1:0dfc4521e1bcaab5` is unchanged, and the first three checkpoints
  too. The final fingerprint goes `v1:07eb8ba0a9c5f765` ->
  `v1:cdf8f5db1e744a60`. Two recordings are identical, and only the
  fingerprints changed in the manifest. The replay equivalence check
  passes with 0 browser errors. Release WASM SHA-256
  `4e3b65b5b1f9408273f5447d0647d98ee16d383ce50efeff0765319344dcc4c6`.
- `cargo mutants --in-diff`: 36 mutants, 32 caught, 2 unviable, 2 missed.
  The two missed are `delta > 0` -> `>= 0` and `delta < 0` -> `<= 0` in
  `support_change`. Both are equivalent, since halving a change of 0 gives
  0.
- `provenance-scan check`: 90 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
