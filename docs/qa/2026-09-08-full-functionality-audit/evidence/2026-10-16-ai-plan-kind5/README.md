# The AI's kind 5 plan object: module 8, stages 6a and 6b (F-036)

Date: 2026-10-16.

A sector planner of kind 5 consolidates the side's own sector. Stage 6a
read its plan object (`ghidra/notes/ai-sector-plans.md`, "Kind 5's plan
object", commit `2abcb933`). This stage ports it. Until now the port's
fleet heuristics stood in for it.

## Port (save v57, unchanged)

- `ai_planner.rs`:
  - `ported_kind` now covers kinds 1 to 5;
  - the request (`FUN_004b17b0`): the base's `v[20]` less its `v[16]`,
    less the sector's own systems (`+0x78`), plus 10. It is 0 without the
    sector record or the base record;
  - the wants (`FUN_004b3300`): 1.5 times the threat, 1.2 times the
    troops, at least one regiment, and kind 4's tail;
  - the report (`FUN_004b1760`): `0x20000` while the sector's `+0x2c`
    lacks 2 (not wholly the side's) or has 8 (an opponent's system).
- `ai_plan.rs`:
  - the base (`FUN_004b24b0`) is kind 4's `invasion_base` with kind 5's
    marks. It takes `0x60000` and drops `0x10000`, keeps a base with
    `0x20000`, and releases `0x70000`;
  - boarding (`FUN_004b28c0`) needs the sector record. It takes the best
    defense (slot `+0x1e0`) while the sector holds no opponent system,
    else the best attack;
  - the occupier (`occupy_target`, `FUN_004b2f40`, state 10). A fleet
    carrying regiments takes the first neutral system (`+0x30` 2,
    forbidding `0x100208`; key `0xf`, order 1) that passes
    `fleet_occupies` (`FUN_004ae440`). The system takes `0x1000000` and
    the fleet takes role `0x80000000`;
  - state `0xe` with that role: while `FUN_004ae440` holds, the role stays
    and `land_one` (`FUN_004b1440`) moves one carried regiment onto the
    target. Otherwise the role goes;
  - the bombarder (`FUN_004b3110`) targets the opponent's held systems
    (`+0x30` `0x24`), with keys 8 and `0x21`;
  - the retreat (`FUN_004b3400`): a lone fleet without a base goes to a
    random system of the sector that is not yet explored (`+0x2c`
    `0x8000000`). This corrects the reading: that bit is the explored bit,
    not a mark.
- `troop_transport.rs`: the planet acceptance (`FUN_005073d0`) now lets a
  regiment land on a planet that is explored, unpopulated and neutral, and
  not blockaded. The occupier relies on this landing. The player's Move
  of a regiment gets it too.

port:
- The simulation's automatic landing of carried regiments (step 3b) still
  lands them only on their side's planets.
- The mission reports (slots `+0x24..+0x30`) are not read.

## Tests that changed with this stage

Kind 5 is now ported, so tests that used it only because the stand-in
served it now use kind 6. Their assertions are unchanged. The same was
done for kind 4 in stage 5g.
- `ai.rs`: `with_stand_in_planner` and `fleets_mid_cycle`, used by five
  scheduler tests, and `a_refused_module_eight_order_ends_its_cycle_and_its_orders`.
- `ai_planner.rs`: `a_cycle_runs_state_by_state_and_reports`,
  `the_report_reads_what_was_asked_and_held`,
  `kind_three_wants_more_than_its_sector_s_threat` and
  `kind_one_runs_its_own_cycle_with_kind_three_s_requests`.

## Campaigns (`playtest-kind5.json`)

The comparison is with the stage 6a build. A temporary trace (removed)
counted the planners' kinds.
- Over 600 days, seeds 1, 2, 3, 7, 13, 42 and 47 name no kind 5 plan, and
  their event streams are identical apart from timing.
- Over 1000 days, only seeds 3 and 26 of seeds 1 to 30 name one.
  - Seed 3 names it on day 730 and seed 26 on day 647. From then on its
    own cycle replaces the stand-in's.
  - In seed 26 it boards a regiment.
  - No occupation happens in these campaigns.

## Browser

The run used the release build with `agent-browser` (headless Chromium,
640 by 480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.
Each frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3599 | 0.5 ms | 0.7 ms | 24.0 ms |
| Fast | 60 s | 3601 | 0.6 ms | 1.5 ms | 3.4 ms |

- No frame took over 50 ms.
- The AI's longest day took 19 ms.
- The Fast run reached day 152.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The Fleet window gate (`fleet-window.mjs`, Arch Chromium 153 through a
  temporary Linux manifest): all 46 cases pass.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2249 passed, 0 failed.
  - `troop_transport.rs`: the neutral planet, `Controlled(Neutral)`, and
    the refusals when unexplored, populated or blockaded.
  - `ai_planner.rs`: the request, the wants with their floors and flags,
    and the report.
  - `ai_plan.rs`:
    - the base's marks and the step's state 2;
    - boarding by defense or attack, with the state 3 dispatch;
    - the occupier: its keys and coin, the forbidden bits, and no
      regiments or another kind;
    - the landing and the role kept or lost;
    - the bombarder's lists;
    - the lone fleet.
  - 36 cases were neutralized; 34 fail. The two that pass are
    equivalent:
    - the population test in the planet acceptance, since the group check
      (`FUN_0053d430`) already refuses a populated planet of another side;
    - the occupier's own `values[12]` test, which `fleet_occupies` repeats.
- `cargo test -p rebellion-data --tests -- --include-ignored`: the replay
  goldens pass unchanged. Two tests fail, as they did before this stage:
  `fixed_fleet_tables_only_target_special_systems` and
  `telemetry_coverage`.
- Seed-42 golden: `v1:a8343f6c6630924e`, final `v1:e523a780152b284e`,
  both unchanged. The replay equivalence check passes with 0 browser
  errors. WASM SHA-256
  `0a9ad9a189736b9c391ed1578a9f5f80c7c49c9bbf6878150d1f5bc66a037e97`.
- `cargo mutants --in-diff` (core):
  - First run: 130 mutants, 117 caught, 5 unviable, 8 missed.
  - Second run, after new cases: 56 mutants on the affected functions,
    2 missed. Both are `&` to `|` or `^` on the occupier's role test in
    `settle_role`. They are equivalent, because only that role bit is left
    at that point.
- `provenance-scan check`: 102 uncited items, within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
