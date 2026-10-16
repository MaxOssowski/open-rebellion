# The AI's plan object, states 7 to 0xe: module 8, stage 4c (F-036)

Date: 2026-10-16.

Kind 3's plan object (`FUN_004adb70`) now runs to its end: the step's
fleet takes a role and a target, moves there or regroups, and the roles
are settled at their targets. The reading is in
`ghidra/notes/ai-sector-plans.md` ("The plan object's step"); the port is
in its "Ported (2026-10-16, save v54)" section.

## Port (save v54)

- `crates/rebellion-core/src/ai_plan.rs`:
  - 1 `FUN_004af6d0`: each held fleet with a role marks its target system
    (`0x1000000`, `0x100000`, `0x200000`, `0x400000` by role);
  - 7 kind 3's `FUN_004b4560`: a fleet with attack takes the first of the
    sector's systems with an enemy fleet and no attacker bound there (key
    `0x27`) that it prevails at (`FUN_004ae150`): mark `0x100000`, target
    `+0x2c`, role `0x10000000`;
  - 8 and 10: slots `0x6158b0`, which return 0;
  - 9 kind 3's `FUN_004b4730`: a fleet with bombardment takes the first of
    the sector's systems of the opponent with no role bound there, merged
    by key 8 and key `0x10`, that `FUN_004ae320` passes: mark `0x400000`,
    role `0x40000000`;
  - `0xc` `FUN_004b0c90`: with more than one held fleet, join the nearest
    other one whose system has no enemy fleet (`FUN_0041ae20`,
    `AIAction::JoinFleet`, new); else kind 3's slot `+0x54`
    (`FUN_004b4a20`: the base, the headquarters, a yard, or the nearest
    safe system when standing at the opponent's);
  - `0xd` `FUN_004b0f90`: move the fleet to its target;
  - `0xe` `FUN_004b1070`: an attacker at its target keeps its role while
    it prevails; every other role and target goes; the walk's end ends
    the step, and the planner's state 5 goes to 6.
- `FleetRecord::target` (`+0x2c`) is kept in `FleetHold` across refreshes.
- `AIAction::JoinFleet` is validated (`FUN_00578d00`:
  `fleet_join::validate_join`) and carried out by the integrator and the
  app (`fleet_join::join_fleet`).
- The fleet heuristics no longer stand in for kind 3 (`plan_done` ends
  only the other kinds' cycles), and they leave a fleet any planner holds
  to its plan (port:).

port:
- The bombardments `FUN_004b1070` gives at a `0x40000000` fleet's target
  (`FUN_004b1330`: Destroy System `0x223`, or `FUN_004ae320`'s Target
  Military Facilities `0x220` and General Bombardment `0x222`) are not
  given: the Fleet menu's bombardment orders are not ported and the
  bombardment function is unrecovered (F-032). The fleet loses its role
  and target as after the order; with no order the walk always ends, so
  its cursor `+0x30` stays clear.
- The port's move refuses a fleet's own system, which the original's
  accepts and does nothing with (`FUN_00578d00`); no order is given there.
- hyp: controller `+0x168` reads as 0 (`FUN_004ae320`'s shields).

Tests that changed with this stage:
- `a_cycle_runs_state_by_state_and_reports`: the plan object's step now
  ends at `0xe` (`*out`) and the planner goes on to state 6 instead of
  handing over to the stand-in; its assertions on the cycle and the
  report are unchanged.
- `the_plan_object_steps_...` (renamed from `..._then_stands_in`): it
  follows the states to `0xe` and the step's end (`FUN_004adb70`).
- Five scheduler tests (`a_pass_issues_one_order...`,
  `a_module_with_orders_left...`, `the_day_s_tenth_order...`,
  `equal_priorities...`, `a_refused_module_eight_order...`) used a kind 3
  planner only so that module 8 would issue the stand-in's fleet orders.
  They now set up a kind 4 planner (posture 1 kept, so the strategy does
  not name the sector kind 3 again), which the stand-in still serves; the
  first two again carry their assertions as of stage 3 (`40e515ce`),
  before stage 4a let a kind 3 cycle go on after the stand-in.

## Campaign (`playtest-plan-roles.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`, with a
temporary trace (removed): the Alliance's planner holds one fleet at its
base, the headquarters, in a sector with no enemy fleet and no system of
the opponent. States 7 and 9 find no target and `0xc` gives the base it
stands at, so the plan gives no order. Without the stand-in for kind 3
the heuristics' fleet moves fall from 28 to 15, space battles from 12 to
5 and bombardments from 4 to 0. In seed 3 the Alliance's planners send a
fleet home twice (days 100, 163) and bind a bombarder, which reaches its
target on day 519 and there, without the bombardment, loses its role and
takes it again each cycle. Seeds 1, 2, 7, 47 and 99 give no plan order.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3745 | 0.5 ms | 0.7 ms | 2.1 ms |
| Fast | 60 s | 3746 | 0.5 ms | 0.8 ms | 3.3 ms |

No frame took over 50 ms. The AI's longest day took 20 ms (day 1 at Very
Slow, 600 passes). The Fast run reached day 151 (`2-fast.png`). The
console has 151 `[ai]` lines and 0 errors; `network.txt` lists 4
requests, all HTTP 200. Native GUI acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2186 passed, 0 failed.
  - `ai_plan`: 62 tests (new: `FUN_004ae150`'s branches by table, the
    bombardment codes, states 7, 9, `0xc`, `0xd`, `0xe`, slot `+0x54`,
    the role marks, the merged lists, the ties by coin in every list, the
    target across refreshes); `ai.rs`: Join Fleet's validation, the
    heuristics leaving held fleets; the integrator: Join Fleet carried
    out in the system and elsewhere.
  - Each new behaviour fails with its change neutralized (56 cases).
- Seed-42 golden regenerated for save v54 and the plan objects' roles
  (the stand-in no longer runs for kind 3): initial
  `v1:a9d12433c6c44f1e` -> `v1:dbb908594619dd4b`, final
  `v1:a5c61ad2824e809c` -> `v1:58f498ebb709e88b`; two recordings
  identical. The replay equivalence check passes with 0 browser errors.
  WASM SHA-256
  `4b993de411d1cf8c48ae35de6af0b5b3bde1465da9b48d4d8e4573dc8c108263`.
- `cargo mutants --in-diff`: core 177 mutants, the first run missed 21;
  after the tie, threshold and sum tests three remain, all equivalent:
  - `regroup`'s `fleets.len() > 1` to `>=`: with one held fleet the list
    holds only the step's fleet, which is skipped, and draws nothing;
  - `settle_roles`' `flags & ROLES == 0` to `|`/`^`: a fleet without a
    role has no target, so taking them away changes nothing.
  Data: 4 mutants, all caught.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
