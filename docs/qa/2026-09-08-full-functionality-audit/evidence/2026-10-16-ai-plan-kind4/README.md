# The AI's kind 4 plan object: module 8, stage 5g (F-036)

Date: 2026-10-16.

A sector planner of kind 4, which invades an enemy yard system, now runs
its own plan object instead of the port's fleet heuristics. The reading is
in `ghidra/notes/ai-sector-plans.md` ("Kind 4's plan object", stage 5d).
This stage corrects one point of that reading: the target is chosen anew
each cycle, not kept. The port is described in the note's "Ported
(2026-10-16, save v57): kind 4's plan object" section.

## Port (save v57)

- `ai_planner.rs`:
  - `ported_kind` now covers kinds 1 to 4;
  - `choose_target` (`FUN_004b8f80`): the first of the opponent's systems,
    in pick order by the yard key and `v[0x14]`, whose record has the yard
    key above 0. It is stored in `SectorPlanner::plan_target` (`+0x5c`,
    saved);
  - `want_invasion` (`FUN_004b8b70`): the wanted counts, from 1.5 times the
    threat, the target's shields and the target's garrison need;
  - movement 4 (`FUN_004b72d0`);
  - the report's `0x20000` while the target is not the side's
    (`FUN_004b70a0`).
- `ai_plan.rs`:
  - `invasion_base` (`FUN_004b8070`: marks `0x50000`, key `0x13`);
  - `board` (`FUN_004b8470`): the base's strongest regiments, by TROOPSD
    attack above 0, onto a held fleet there. This gives the new
    `AIAction::BoardRegiments`, validated and carried out through the
    regiment move with the fleet as the leg;
  - the attacker tries the target first (`FUN_004b8880`);
  - `assault_target` (`FUN_004b8ab0`, role `0x20000000`), with
    `fleet_assaults` (`FUN_004ae240`) as the test;
  - no bombarder (slot `+0x44` is `0x6158b0`);
  - the retreat skips the blockade tests (`FUN_004b8ca0`).
- State `0xe` settles the assault role as `FUN_004b1070` does:
  - Destroy System, when the fleet has a Death Star (`0x10`) and the
    system's `v[50]` is above 0;
  - else Planetary Assault `0x234`, while `FUN_004ae240` holds;
  - else `FUN_004ae320`'s bombardment.
  The role stays after an order and goes only when no order is given.

port:
- The planner's request (slot `+0x0c`, `FUN_004b7100`) is not ported. So
  modules 6 and `0xb` do not move regiments to the base, and slot `+0x4c`
  (`FUN_004b8b30`) is not ported either.
- hyp: the base's walk gives the side's regiments on the planet in its
  order. Without a target, `+0x3c` takes the `+0x40` value.

## Tests that changed with this stage

- Five scheduler tests in `ai.rs` (`a_pass_issues_one_order...`,
  `a_module_with_orders_left...`, `the_day_s_tenth_order...`,
  `equal_priorities...`, `a_refused_module_eight_order...`) used a kind 4
  planner only because the stand-in served it. That kind is now ported, so
  they use kind 5. Their assertions are unchanged.
- `ai_planner.rs`: `kind_one_runs_its_own_cycle...` checked that kind 4 is
  handed over. It now checks kind 5.

## Campaigns (`playtest-kind4.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, compared with the
stage 5f build: every seed's counts move. The kind 4 planners now take
ships and give their own orders.

A temporary trace (removed) shows how far the plan gets:
- Kind 4 planners name a target in seeds 42, 3 and 47, and board
  regiments there (2, 3 and 3 `BoardRegiments`).
- No fleet takes the assault role. The fleets carry at most 3 regiments,
  while the targets need 4 or 5 (`v[52]`). Without the planner's request
  (`+0x0c`) no regiments are moved to the base.
- So no Planetary Assault happens yet, and no `combat_ground` event either.
- The number of planets changing hands stays as in stage 5f, except seed 3
  (11 → 12).

## Browser

The run used the release build with `agent-browser` (headless Chromium,
640 by 480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.
Each frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.5 ms | 0.7 ms | 23.0 ms |
| Fast | 60 s | 3601 | 0.5 ms | 1.4 ms | 3.2 ms |

- No frame took over 50 ms.
- The AI's longest day took 18 ms (day 1).
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The Fleet window gate (`fleet-window.mjs`, Arch Chromium 153 through a
  temporary Linux manifest): all 46 cases pass.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2231 passed, 0 failed.
  - `ai_planner.rs`: kind 4's target by yard kind, the coin on ties, no
    target, and the sector missing; the wants and their floors; movement
    4; the report.
  - `ai_plan.rs`:
    - the base, its marks, the coin, and a lost base;
    - boarding: the strongest regiment first, attack above 0, the room and
      `v[19]` limits, and no fleet there;
    - the step's states 2 and 3;
    - the attacker trying the target first, also past an attacker's mark;
    - the assault binding and its refusals; no bombarder;
    - the retreat;
    - `FUN_004ae240`'s table;
    - the assault role at its target: assault, the bombardment fallback,
      Destroy System and its edges, and the role kept or lost.
  - `ai.rs`: boarding's validation. The integrator: a boarding order
    carried out.
  - Each new behaviour fails with its change neutralized (17 cases).
- Seed-42 golden regenerated for save v57 and the kind 4 plans:
  `v1:fb57b4045cb49e83` -> `v1:a8343f6c6630924e`, final
  `v1:d62b35fe73dd4203` -> `v1:e523a780152b284e`. Two recordings are
  identical. The replay equivalence check passes with 0 browser errors.
  WASM SHA-256
  `bb4c408ce9f2714a3bfe6b27ae2365bd4bacc5db45335ccdb6709d6393827b73`.
- `cargo mutants --in-diff` (core and data):
  - First run: 183 mutants, 19 missed. The misses came from coin ties, edge
    values, and two coincidences in the test's numbers.
  - Second run, on the affected functions after the new cases: 97 mutants,
    4 missed. The attack test then catches three of them.
  - The one left is equivalent: `assault_target`'s `||` before
    `fleet_assaults`, because a target without a record fails
    `fleet_assaults` anyway.
- `provenance-scan check`: 102 uncited items, within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
