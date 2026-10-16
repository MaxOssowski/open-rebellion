# The AI's kind 2 plan object: module 8, stage 5c (F-036)

Date: 2026-10-16.

A sector planner of kind 2 now runs its own plan object, which sweeps its
sector: it binds attackers as kind 1 does, sends a lone fleet to each
system of the opponent once, and marks the sector swept when none is left.
The reading is in `ghidra/notes/ai-sector-plans.md` ("Kinds 1 and 2's plan
objects"); the port is its "Ported (2026-10-16, save v56): kind 2's plan
object" section.

## Port (save v56)

- `ai_planner.rs`:
  - `ported_kind` covers kinds 1 to 3;
  - slot `+0x50`, `FUN_004b9fe0`: 1.5 times (`* 3 / 2`) for 1.2; `+0x38`
    adds the sector's `+0x13c`; `+0x40` takes the floor controller
    `+0x16c`;
  - the report, `FUN_004b9310` (`sweep`, `sweep_left`): `0x20000` with the
    sector's `+0x2c` `0x40000`, or while a system of the opponent with no
    enemy fleet or role there (`FUN_0041a3a0`: `+0x30` `0x20` forbidding
    `0x700201`) is not on the visited list; with none, the sector record
    takes `0x20000000`.
- `ai_plan.rs`:
  - slot `+0x3c`, `FUN_004b9c90`: kind 1's two lists; the target joins the
    visited list (`FUN_004f57b0` on `+0x5c`);
  - slot `+0x44` is `0x6158b0`: no bombarder;
  - slot `+0x54`, `FUN_004ba0d0`: a lone held fleet without regiments,
    not en route, with `values[2]` and `values[3]` at least controller
    `+0x164`/`+0x168`, goes to the first such system not visited, which
    joins the list; else kind 3's chain.
- `SectorPlanner::visited` (plan object `+0x5c`) goes with the plan
  object (`FUN_004773f0`, `FUN_00477190`).
- `AIState::swept` keeps the sectors' `+0x2c` `0x20000000` across the
  port's rebuilt records (the original's refresh keeps `0x28000000`);
  `ai_strategy::mark_swept` applies it, and naming the plans clears it for
  the old and new plans' sectors (`FUN_004317b0`, `FUN_00431790`).

hyp: controller `+0x16c` has no writer found and reads as 0.

## Campaigns (`playtest-kind2.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai` over seeds 42, 1, 2,
3, 7, 13 and 47, against the stage 5b build: every seed's counts move, as
the kind 2 planners now take ships and create fleets instead of handing
over to the heuristics. Seed 42: AI orders 777 -> 699, Create Fleet 1 -> 3,
fleet moves 15 -> 9, space battles 5 -> 4. A temporary trace (removed):
kind 2 attackers bind in seeds 42, 1, 2, 3, 7 and 47; seed 3 sends a lone
fleet to three systems from day 463; seeds 1 and 2 mark their sector
swept.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign, 60 s at Very Slow then 60 s at Fast, each frame's work
timed by wrapping the game's `requestAnimationFrame` callbacks:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.6 ms | 0.8 ms | 22.6 ms |
| Fast | 60 s | 3601 | 0.6 ms | 1.5 ms | 3.6 ms |

No frame took over 50 ms. The AI's longest day took 17 ms (day 1). The
Fast run reached day 152 (`2-fast.png`). The console has 152 `[ai]` lines
and 0 errors; `network.txt` lists 4 requests, all HTTP 200. Native GUI
acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2218 passed, 0 failed.
  - `ai_planner.rs`: kind 2's wants and floors; its report with a system
    left, with none (the sector swept, once), with an enemy fleet or a role
    there, and with `0x40000`; kind 4 still handed over (this assertion
    used kind 2 in stage 5b, now ported).
  - `ai_plan.rs`: the attack target joining the visited list (not for
    kind 1), no bombarder; the lone fleet's sweep in order, the coin on
    equal ages, neutral systems left out, and each condition that stops it.
  - `ai_module8.rs`: the visited list goes with a new plan object and a
    reset, and stays with the same one. `ai_strategy.rs`: naming clears
    the swept bit of the plans' sectors only. `ai.rs`: the refreshed
    records carry it.
  - Each new behaviour fails with its change neutralized (15 cases).
- `cargo test -p rebellion-data --tests -- --include-ignored`: the replay
  goldens pass; `seeds::tests::fixed_fleet_tables_only_target_special_systems`
  fails, as it does on the stage 5b commit without this change.
- Seed-42 golden regenerated for save v56 and the kind 2 plans:
  `v1:b160883b9ed0c807` -> `v1:fb57b4045cb49e83`, final
  `v1:f5933994514e98bc` -> `v1:d62b35fe73dd4203`; two recordings
  identical. The replay equivalence check passes with 0 browser errors.
  WASM SHA-256
  `834ec6f703a8b2dc1013451b51c048868206b511e0f9f82bf2b2f998e066117d`.
- `cargo mutants --in-diff` (core and data): 103 mutants, 5 missed (the
  kind check on the visited list, a coincidence of `3 * 3 / 2` with
  `3 + 3 / 2` in the test's values, the sweep's coin); after the new
  assertions 56 of 56 in those functions caught.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
