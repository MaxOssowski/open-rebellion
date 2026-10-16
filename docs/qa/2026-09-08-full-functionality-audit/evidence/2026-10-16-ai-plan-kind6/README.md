# The AI's kind 6 plan object: module 8, stages 6c and 6d (F-036)

Date: 2026-10-16.

A sector planner of kind 6 conquers a sector held by the opponent. Stage
6c read its plan object (`ghidra/notes/ai-sector-plans.md`, "Kind 6's
plan object", commit `ed89f0ee`). This stage ports it. With it, every plan
kind that `FUN_00477630` makes (1 to 6) runs its own plan object.

## Port (save v57, unchanged)

- `ai_planner.rs`:
  - `ported_kind` covers kinds 1 to 6. The port's stand-in now serves only
    a kind outside 1 to 6, which the original never makes;
  - the request (`FUN_004b4df0`): the sector's `+0xa0` (the summed
    garrison need of the opponent's systems) plus the base's `v[20]`
    less its `v[16]`;
  - the wants (`FUN_004b6a60`): 1.5 times `+0x190`, `+0x194` with
    `+0x13c`, and `+0x198` with `+0x138`, and `+0x3c` at least `+0x134`
    (the largest garrison need). Each term is floored by controller
    `+0x164..+0x170`, which read as 0. Then kind 4's tail;
  - movement 4 as kind 4 (`FUN_004b72d0`);
  - the report (`FUN_004b4d90`): `0x20000` with the sector's `+0x2c` 8
    and `0x400`, and `0x400000` or planner `+0x60` `0x200000`.
- `ai_plan.rs`:
  - the base (`FUN_004b5b20`, `CONQUEST_MARKS`) takes
    `& 0xfffdffff | 0x50000`, keeps a base with `0x10000`, and releases
    `0x70000`;
  - boarding is kind 4's code (`FUN_004b5f30`);
  - the attacker is kind 3's code (`FUN_004b6340`);
  - `assault_sector` (`FUN_004b6510`, state 8). A fleet carrying
    regiments, with the sector record, takes the first opponent system
    (`+0x30` 4, forbidding `0x300200`) that passes `fleet_assaults`:
    - by key `0x32` (`v[50]`, order 1) when the sector's `+0x19c` is
      above 0;
    - else from the merged lists by key `0x21` (order 1) and, for `+0x30`
      `0x24`, key `0x14` (order 2). These lists are built only when the
      first found nothing, so they draw no coin otherwise.
    The system takes `0x200000`, and the fleet takes the role
    `0x20000000`, which state `0xe` settles as for kind 4;
  - the bombarder is kind 5's code (`FUN_004b6870`);
  - the retreat (`FUN_004b6b70`): a lone fleet without a base, whose
    `values[2]` and `values[3]` reach the controller floors, goes to a
    random system of the sector without `+0x30` `0x700201`, explored or
    not.

port:
- Planner `+0x60` `0x200000`, which the report reads, has no writer found.
- The mission reports (slots `+0x24..+0x30`) are not read.

## Tests that changed with this stage

- `ai.rs` and `ai_planner.rs`: the stand-in tests (the same set as stage
  6b) now use kind 7, a kind the original never makes, instead of kind 6.
  Their assertions are unchanged.
- `ai_module8.rs`: `a_planner_holding_a_plan_reaches_its_plan_step` used a
  kind 6 plan and expected the stand-in's `Step8::Plan`. Kind 6 now runs
  its own cycle. The test now expects that cycle's first step
  (`FUN_004774c0`'s default: `Pending`, state 1, the lists to resync).
  The test's purpose, reaching state 4, is unchanged.

## Campaigns (`playtest-kind6.json`)

The comparison is with the stage 6c build.
- Over 600 days:
  - seeds 1, 2, 7, 13 and 42 are identical apart from timing;
  - seeds 3 and 47 run a kind 6 plan from days 421 and 522. It boards
    more regiments (1 to 3 and 2 to 5 `BoardRegiments`). The planets
    changing hands stay at 11 and 8.
- Over 1000 days:
  - in seed 47 the kind 6 plan assaults Triton on day 770: 15 regiments
    against 9 defenders, all 9 defenders fall, and 6 land. The planets
    changing hands go from 8 to 10;
  - seed 3 has no assault.
- A temporary trace (removed) gave the planner kind of each Assault order
  in seed 47:
  - day 770: kind 6;
  - day 28: kind 3 by then. That planner had bound the assault role while
    it was kind 4, the only kind that bound it at stage 5h.
  This corrects stage 5h's account, which said a kind 4 plan ordered the
  day 28 assault.

## Browser

The run used the release build with `agent-browser` (headless Chromium,
640 by 480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.
Each frame's work time was measured:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3600 | 0.5 ms | 0.7 ms | 23.6 ms |
| Fast | 60 s | 3601 | 0.5 ms | 1.5 ms | 3.2 ms |

- No frame took over 50 ms.
- The AI's longest day took 18 ms.
- The Fast run reached day 152.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The Fleet window gate (`fleet-window.mjs`, Arch Chromium 153 through a
  temporary Linux manifest): all 46 cases pass.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2256 passed, 0 failed.
  - `ai_planner.rs`: the request (and kind 5's beside it), the wants with
    their floors and flags, movement 4, and the report.
  - `ai_plan.rs`:
    - the base's marks and state 2;
    - boarding by attack in state 3;
    - the sector assault: by `v[50]`, by the merged keys, ties by the
      coin in either phase, the refusals and marks, no regiments, no
      sector record, another kind, and the lazy second lists;
    - the bombarder as kind 5's;
    - the lone strong fleet.
  - All 25 neutralized changes fail.
- `cargo test -p rebellion-data --tests -- --include-ignored`: the replay
  goldens pass unchanged. Two tests fail, as they did before this stage:
  `fixed_fleet_tables_only_target_special_systems` and
  `telemetry_coverage`.
- Seed-42 golden: `v1:a8343f6c6630924e`, final `v1:e523a780152b284e`,
  both unchanged. The replay equivalence check passes with 0 browser
  errors. WASM SHA-256
  `9c61a16176071302ded453552addc6450aff5d51b09deb2e3423eca8137232f6`.
- `cargo mutants --in-diff` (core):
  - First run: 108 mutants, 94 caught, 3 unviable, 11 missed. The misses
    were coin ties and the wants' flags.
  - Second run, after new cases: 47 mutants on the affected functions,
    all caught.
- `provenance-scan check`: 102 uncited items, within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
