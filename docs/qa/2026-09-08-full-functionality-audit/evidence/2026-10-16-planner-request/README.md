# The planner's request and module 6's planned moves: module 8, stage 5h (F-036)

Date: 2026-10-16.

Stage 5g ported kind 4's plan object, but no regiments reached its base:
the planner's request (slot `+0x0c`) and the moves that read it were not
ported. This stage ports the request and module 6's two moves that read
it. The reading is in `ghidra/notes/ai-production-modules.md` ("The move
cycle") and `ghidra/notes/ai-sector-plans.md` ("Ported (2026-10-16, save
v57): the planner's request").

## Port

- `ai_planner.rs`: `SectorPlanner::request` (`FUN_00476b70`, slot
  `+0x0c`). For kind 4 (`FUN_004b7100`) it is the base's `v[20]` less its
  `v[16]`, plus the target's `v[52]`, when both records exist; otherwise
  0. Kinds 1 to 3 have `0x6158b0` there, which gives 0.
- `ai_module1.rs`:
  - `Tallies::sector_invading`: controller `+4` bit `0x10`, set for a
    sector with `+0x2c` `0x1000000` (`FUN_0041b7e0`);
  - `Inputs::planners`: module 8's sector planners (controller `+0xd8`).
- `ai_module6.rs`, `planned_move`:
  - `FUN_004ddb70` (state 1) sends defensive regiments (class flag
    `0x2000`). It runs while bit `0x20` is set and uses sector `0x2000000`,
    base `0x20001`, source sector `0x1000`, source system `0x8001`, and the
    fallback `0x280000`/`0x18000`.
  - `FUN_004de130` (state 2) sends offensive regiments (`0x1000`). It runs
    while bit `0x10` is set and uses sector `0x1000000`, base `0x10001`,
    source sector `0x800`, source system `0x4001`, and the fallback
    `0x180000`/`0x10010000`.
  - Both send them to the base of the planner based there
    (`FUN_0042f310`). The count is the planner's request, at most the
    source's `v[19]`, and the lowest defense goes first.
- A correction of an earlier port: the source sector of all four module 6
  moves is taken by distance with order 1 (`0x4ddce1`, `0x4de85d`,
  `0x4ded0b`, `0x4dee02`). `FUN_004357b0` takes the last entry, so the
  source is the farthest sector. The port had order 2, which gave the
  nearest.

Save format: unchanged at v57.

port:
- Kinds 5 and 6's request slots (`FUN_004b17b0`, `FUN_004b4df0`) are not
  read, so those planners ask for nothing.
- Module `0xb`'s `FUN_004c9cb0` was read further but is not ported. It
  moves the regiments carried by the first fleet that has any to a
  planner's base.

## Tests that changed with this stage

- `ai_module6.rs`: `the_planned_moves_find_no_planner_and_clear_their_systems`
  is renamed `the_planned_moves_without_their_sector_bit_clear_their_systems`.
  Its assertions are unchanged. The planner now exists, so the reason
  nothing moves is the missing controller bit.

## Campaigns (`playtest-planner-request.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, compared with the
stage 5g build:
- Seed 47 has the first Planetary Assault in these campaigns:
  - days 6 and 7: module 6 moves 5 regiments to the kind 4 base, Svivren;
  - days 14 and 15: the plan loads them onto a fleet (4 + 1);
  - day 28: the fleet assaults Commenor. 5 attackers meet 0 defenders and
    all 5 land;
  - day 29: Commenor passes to the Empire.
- Seeds 42, 1, 3 and 47 send more regiments (35 → 36, 4 → 5, 8 → 12,
  12 → 24 `MoveRegiments`). Seed 13 sends the same regiments, but its
  other orders change. Seeds 2 and 7 issue the same orders as before.
- Planets changing hands: seed 1 goes from 7 to 8 and seed 3 from 12 to
  11. The other seeds are unchanged.
- The `telemetry_coverage` data test fails on its required `repair` events,
  in this build and in the stage 5g build alike.

## Browser

The run used the release build with `agent-browser` (headless Chromium,
640 by 480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.
Each frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.5 ms | 0.7 ms | 23.5 ms |
| Fast | 60 s | 3601 | 0.5 ms | 1.5 ms | 3.6 ms |

- No frame took over 50 ms.
- The AI's longest day took 18 ms (day 1).
- The Fast run reached day 152.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The Fleet window gate (`fleet-window.mjs`, Arch Chromium 153 through a
  temporary Linux manifest): all 46 cases pass.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2236 passed, 0 failed.
  - `ai_planner.rs`: kind 4's request and its 0 cases.
  - `ai_module6.rs`:
    - the short move: its order and cycle end, the `v[19]` and request
      limits, the fallback, no source, and a base with no planner, another
      base's planner or one asking for nothing;
    - the invading move: its order, its fallback, and the other move's
      bits or a missing controller bit;
    - the farthest source sector, for the planned move and for
      `FUN_004de6f0`;
    - `sector_invading`.
  - 15 cases fail with their change neutralized: the request, its terms,
    the tally, each of the invading move's bits and class, the short
    move's class and fallback, the controller bit, the surplus limit, the
    planner lookup, and both orders.
- `cargo test -p rebellion-data --tests -- --include-ignored`: the replay
  goldens pass unchanged. Two tests fail, both also failing before this
  stage: `fixed_fleet_tables_only_target_special_systems` (since stage 5c)
  and `telemetry_coverage` (above).
- Seed-42 golden: `v1:a8343f6c6630924e`, final `v1:e523a780152b284e`,
  both unchanged. The replay equivalence check passes with 0 browser
  errors. WASM SHA-256
  `e302b6ea25bc4535f6257c0a1d295f262ac6efe9def03353765b968bd05d51ee`.
- `cargo mutants --in-diff` (core): 40 mutants, 38 caught, 2 unviable.
- `provenance-scan check`: 102 uncited items, within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
