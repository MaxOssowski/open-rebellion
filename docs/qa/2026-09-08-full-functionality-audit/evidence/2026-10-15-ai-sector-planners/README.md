# The AI's sector planners: module 8, stage 2 (F-036)

Date: 2026-10-15.

Stage 1 (`../2026-10-15-ai-strategy-posture/`) ported the strategy that
plans sectors. This stage ports module 8's planner lifecycle; the plan
objects that give the planners' orders come in later stages. The reading
is in `ghidra/notes/ai-sector-plans.md` ("Module 8").

## Port (save v50)

- `crates/rebellion-core/src/ai_module8.rs`: `Module8` (`+0x3c..+0x44`
  and the planner list, controller `+0xd8`) and `SectorPlanner`. The
  step `FUN_004cf630`:
  - state 1 (`FUN_004cf700`): a planner is added (`FUN_0042f350`) while a
    planned sector lacks one (controller `+0x204 < 0`) and every planner
    has `+0x60` `0x4000000`; one holding no plan is dropped
    (`FUN_0042f3d0`, `FUN_00477190`) when `+0x204 > 0`;
  - state 2 (`FUN_004cf760`): the planners in turn;
  - state 3 (`FUN_004cf7c0`): a planner keeps its sector, else claims the
    planned sector nearest it (`FUN_00419b80`), else one by a random key
    (`FUN_0041a1d0`, `FUN_0041cd80(0x14)`, the key draws interleaved with
    the tie coins), through `FUN_004768b0`, and binds the plan's kind
    (`FUN_004773f0`);
  - state 4: the planner's step.
- The refresh writes each planner's sector `+0x38`, so the sector record
  bits and controller `+0x204` follow.
- The strategy's review (`FUN_004e5560`) now reads the planners: each
  takes its plan's id, and the review grows or shrinks the list from
  their `+0x60` bits; a posture change clears their `0x8000000`
  (`FUN_004e54f0`).
- `Planner::random` is `FUN_0041cd80(n)`.

port:
- The plan objects (`FUN_00477630`, six kinds) and the planner's step
  (`FUN_004774c0`) are not ported. In state 4 a planner holding a plan
  runs the port's fleet heuristics in their place, decided once a cycle
  and issued one order a step (priority raised while orders remain).
  Without a planner module 8 orders nothing.
- No planner gains `+0x60` bits yet, so module 8 keeps one planner per
  side and the strategy one plan.
- Four scheduler tests used module 8 as a module that orders at once;
  module 8 now needs a planner holding a plan (`FUN_004cf630`), so they
  start with one (`with_sector_planner`, `fleets_mid_cycle`). Their
  assertions are unchanged.

## Campaign (`playtest-planners.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`, with a
temporary trace in `AISystem::module8_step` (removed):
- Each side adds its planner on day 3 and claims its planned sector the
  same day: the Alliance a kind 4 sector, then on day 16 a kind 3 sector;
  the Empire a kind 4 sector, rebinding between kinds 4 and 2 as the
  strategy changes the sector's plan (days 122, 123, 142, 143, 364).
- The planners' step decided the fleet heuristics' orders 2,263 times for
  the Alliance and 2,112 for the Empire.
- The run's events differ from stage 1's, as the strategy's and
  planners' draws change the AI's generator stream: 535 AI actions
  against 483, 220 missions dispatched against 181. Stage 1's run also
  logged 806 economy overcap reports (`economy_tick`) against 1 here.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3744 | 0.5 ms | 0.7 ms | 2.0 ms |
| Fast | 60 s | 3744 | 0.4 ms | 0.7 ms | 49 ms |

No frame took over 50 ms. The longest Fast frame (49 ms) is one frame;
the run ends on the battle screen at Bothawui (`2-fast.png`), as in the
earlier stages. The AI's longest day took 17 ms (day 1 at Very Slow, 600
passes; `console.txt`). The console has 32 `[ai]` lines and 0 errors;
`network.txt` lists 4 requests, all HTTP 200. Native GUI acceptance was
not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2105 passed, 0 failed.
  - `ai_module8`: 10 tests; `ai_strategy`: the review with planners and
    the posture change; `ai.rs`: the third pass adds and binds a planner,
    the refresh's `+0x38`, a refused module 8 order; `mission_planning`:
    the ranged draw.
  - Each new test fails with its change neutralized.
- Seed-42 golden regenerated for save v50 and the planners' draws:
  initial `v1:cca1045386d40ca4` -> `v1:6cc71d2663560368`, final
  `v1:feef56582a180eed` -> `v1:951c79bcafd01fcf`. The replay equivalence
  check (`replay-equivalence.json`) passes with 0 browser errors. WASM
  SHA-256 `7b54a89996be045f2af597e48e4dfc3aab4d1516f72f3d025d1198f027a78151`.
- `cargo mutants --in-diff` (`ai_module8.rs` whole, the changed lines of
  `ai.rs`, `ai_strategy.rs`, `mission_planning.rs`): the first run, 121
  mutants, missed 13 (tie coins at 4 and 5, one bit of the review, two
  planners in state 4, the ranged draw); after their tests the rerun
  caught all 118 viable mutants (3 unviable).
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
