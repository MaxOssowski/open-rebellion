# The AI's kind 1 plan object: module 8, stage 5b (F-036)

Date: 2026-10-16.

A sector planner of kind 1 now runs its own plan object instead of the
port's fleet heuristics. The reading is in `ghidra/notes/ai-sector-plans.md`
("Kinds 1 and 2's plan objects", stage 5a, commit `4934bfb1`); the port is
its "Ported (2026-10-16, save v55): kind 1's plan object" section.

## Port

- `ai_planner.rs`:
  - `ported_kind`: kinds 1 and 3 step their own cycle; the other kinds
    still hand over to the heuristics;
  - slot `+0x50`, `FUN_004bb510`: as kind 3's, but `+0x34` (1.2 x the
    sector's `+0x190`) has no controller floor, and `+0x48` takes 1.1 x
    the sector's `+0x144` (floor controller `+0x174`);
  - the mission slot (`FUN_004b7150`) and the movement slot
    (`FUN_004b9290`, the same code as kind 3's `FUN_004b1800`) are shared;
  - the report (`FUN_004ba5e0`) sets `0x20000` from the sector's `+0x2c`
    `0x80000` (kind 3: 4).
- `ai_plan.rs`:
  - slot `+0x3c`, `FUN_004bafe0`: after kind 3's list, a fleet with
    `values[3]` above 0 tries the opponent's systems with `+0x2c`
    `0x400000` (key `0x1b`, order 1), each through `FUN_004ae150`
    (`attack_from`);
  - slot `+0x44`, `FUN_004bb300`: the bombarder merges three lists, key 8
    (order 2), `0x21` and 0 (order 1);
  - slot `+0x54`, `FUN_004bb630`: a lone held fleet, without regiments
    aboard and not en route (record `+0x38` `0xa00`), first goes to the
    sector's system not the side's, without an enemy fleet or a role there
    (`+0x30` forbidding `0x700201`), seen longest ago; then kind 3's chain.
- `ai_records.rs`: `Galaxy::system_query_by_age` is `FUN_0041a3a0`, ranked
  by `FUN_00433de0` (0 for an own system, else today less record `+0x44`,
  which `AIState::seen` already keeps).
- `PlannerIo` and `Inputs8` carry the day and `AIState::seen`.

hyp: controller `+0x174` has no writer found and reads as 0, like `+0x164`
and `+0x168`.

No save change: save v55. The seed-42 golden is unchanged.

## Campaigns (`playtest-kind1.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`: seeds 42 and 47 give
every event and AI action count of stage 4f. A temporary trace (removed)
over seeds 42, 1, 2, 3, 7, 11, 13, 47 and 99 found kind 1 planners only in
seed 47: 713 cycles, each holding no fleet, so the patrol and the second
attack list never came up. Kind 2 planners run in seven of the nine seeds
(stage 5c).

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign, 60 s at Very Slow then 60 s at Fast, each frame's work
timed by wrapping the game's `requestAnimationFrame` callbacks:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3599 | 0.5 ms | 0.7 ms | 27.7 ms |
| Fast | 60 s | 3601 | 0.5 ms | 1.4 ms | 3.5 ms |

No frame took over 50 ms. The AI's longest day took 22 ms (day 1). The
Fast run reached day 152 (`2-fast.png`). The console has 152 `[ai]` lines
and 0 errors; `network.txt` lists 4 requests, all HTTP 200. Native GUI
acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2213 passed, 0 failed.
  - `ai_planner.rs`: kind 1's wants (no floor on `+0x34`, `+0x48` with its
    floor), its own cycle, the shared requests, the report's bit, kind 2
    still handed over.
  - `ai_plan.rs`: the second attack list (and kind 3, no `values[3]`, no
    `0x400000`, too strong: none); the three bombardment lists against
    kind 3's two; the lone fleet's patrol by age, the coin on equal ages,
    and every condition that stops it.
  - Each new behaviour fails with its change neutralized (7 cases).
- `cargo mutants --in-diff` (core): 78 mutants, 3 missed (the patrol's
  coin), then caught by the equal-age case (12 of 12 in `home`).
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy (pedantic) and rustfmt: no findings in the changed lines.
- The replay equivalence check passes with 0 browser errors: golden
  `v1:b160883b9ed0c807`, final `v1:f5933994514e98bc`, unchanged. WASM
  SHA-256 `3e0c76fda9bb211a668b36db0f65fc32e51e4834d94bce00d2b233cc3689b9fc`.

Hashes are in `SHA256SUMS`.
