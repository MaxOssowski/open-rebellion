# Timer 0x381 is the yards' own research (F-037, F-040)

Date: 2026-10-17.

The last open timer of F-037, `0x381`, was noted as summing "per-system
lists" into side `+0x90/+0x94/+0x98`. Those fields are the side's research
points (`ghidra/notes/mission-lifecycle.md`, "Research"), and their
setters run the research points hook. The timer is the research a side's
yards do on their own, which the port lacked: it researched only through
missions.

## The original

- `FUN_00532350` arms timer `0x381` when the side comes into play, on
  record `+0xd4`. Its minimum is GNPRTB 7169 (30) and its spread 7170
  (30) (disassembly `0x53237d`..`0x53239a`), so it fires every 30 to 60
  days, drawing a new delay each time.
- `FUN_00530460` walks the systems whose side bits match the side's. For
  each it takes the managers of areas 0, 2 and 1 (`FUN_00509670`) and
  counts, through `FUN_00528040(0)`, their yards that `FUN_0052bde0`
  accepts (the manager's area, usable `+0x50` bit 0, the manager's side)
  and that are not suspended (`+0x60` bit 0, set in uprising or blockade
  by `FUN_0050bbb0`). The decompile lost which manager each count belongs
  to; the disassembly (`0x5304f9`..`0x53057e`) gives area 0 to `+0x90`
  (ships), area 2 to `+0x94` (troops) and area 1 to `+0x98` (facilities).
- The setters (`FUN_0052f130`, `FUN_0052f1a0`, `FUN_0052f210`) store the
  new totals and run the points hook (`FUN_005326f0` and its siblings),
  which raises levels as research missions do.
- If a same-side system lacks any of the three managers, nothing is
  added.

## Port

- `ResearchState` gains each side's timer (`alliance_tally`,
  `empire_tally`; save v65), drawn on the first day.
- `ResearchSystem::yard_points` counts a side's yards of a tree's area in
  the systems it holds, leaving out blockaded or revolting systems.
- `ResearchSystem::advance_tally` fires the due timers, adds the points
  through `ResearchState::add_points` (the existing points hook), and
  draws the next delay.
- It runs after the production cycle in the headless simulation (step 13d)
  and in the app. The app posts a Research Report for each level reached,
  as for missions, and telemetry records `research_unlocked` with
  `cause: yards`.
- port: every yard is usable, and a system always has its managers. A
  tree that gains no points is left alone, since an unchanged setter runs
  no hook.

## Tests

- `research.rs`:
  - `a_side_s_yards_count_in_the_systems_it_holds_unless_suspended`;
  - `the_yards_research_every_thirty_to_sixty_days`, which covers the
    first delay, the fire, the level reached and the redrawn delay.
- `simulation.rs`: `a_shipyard_in_a_system_in_revolt_does_no_research`.
- `integrator.rs`:
  `a_level_the_yards_reach_is_recorded_as_research_unlocked`.
- Neutralized, each failing its tests:
  - any holder;
  - counting suspended yards;
  - any area;
  - a fixed reschedule;
  - the simulation's blocked test with `&&` (the one mutant the first run
    missed).

## Campaigns (`playtest-yard-research.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `a2807d36`:

| Seed | Levels from yards | First on day | Research levels in all (old -> new) | Builds |
|---|---|---|---|---|
| 1 | 5 | 81 | 23 -> 27 | 431 -> 388 |
| 2 | 7 | 157 | 15 -> 24 | 373 -> 410 |
| 3 | 14 | 91 | 18 -> 25 | 454 -> 444 |
| 7 | 8 | 77 | 21 -> 20 | 328 -> 309 |
| 13 | 8 | 132 | 27 -> 29 | 490 -> 472 |
| 42 | 10 | 146 | 15 -> 22 | 371 -> 398 |
| 47 | 13 | 134 | 16 -> 25 | 455 -> 457 |

- In most seeds both sides end at higher levels. In seed 7 the Alliance
  ends lower (Ship 4 -> 3, Troop 3 -> 1), as the campaign diverges.
- In four seeds the Alliance opens trees it had not reached before:
  Troop (seeds 1, 3, 47) and Facility (seeds 2, 3, 47).
- The other counts move because the campaigns diverge.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`ac04f48f9f75763be16170732dc76f67c20bae2e42e6fead2ca147d5ebaa62eb`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.9 ms | 22.7 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 2.3 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2273 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the timer and its rolls):
  initial `v1:3ce41483f255839d` -> `v1:affc307d4c23c84d`, final
  `v1:c23f38a8242bca8f` -> `v1:21ecf34ac9e1f828`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The
  replay equivalence check passes with 0 browser errors. Release WASM
  SHA-256 `7c30a748dd08cab8d3a1807526ef4c9ea312f49c87c6f209c30201dcbf9bbc1c`.
- `cargo mutants --in-diff`: 23 mutants, 20 caught, 2 unviable, 1 missed
  in the first run. The missed one, the simulation's `||`, is now caught by
  the simulation test (tried).
- `provenance-scan check`: 89 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
