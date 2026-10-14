# Research AI: tree and target choice (F-040)

Date: 2026-10-14. `rebellion-playtest data/base --seed 42 --ticks 600
--dual-ai` (release build); `playtest-research-ai.jsonl` keeps the research
dispatches, results, level-ups and AI cancellations.

| Side | Research dispatched | Targets | Levels reached |
|---|---|---|---|
| Empire | 2 Ship Design Research | Coruscant (both) | Ship 1, 2, 3, 4, 5, 6 on days 74, 184, 253, 313, 411, 536 |
| Alliance | 3 Troop Training, 3 Facility Design | Vagran, Hoth | Troop 1 on day 470 |

Before this change (`evidence/2026-10-14-research-missions/`) the Empire
ran one ship research mission and reached Ship level 4. Now it keeps one
mission per character able to research the tree (`FUN_0041bc10`).

The three Facility Design missions of Talon Karrde ended the day after
each order with end code 3: the detection run exposed the team before
phase 4 and the record lets members resign (slot `+0x1dc`). That is the
mission system's detection, not the research AI; the AI then ordered him
again. No AI cancellation occurred in this run.

Gates: `cargo test --workspace` 1741 passed, 0 failed; the seed-42 replay
golden is unchanged (`v1:ee7a9a22f312f0e0` -> `v1:398db20957865176`) and
`scripts/check-replay-equivalence.py --skip-build --json` passes (native
and WASM agree, 0 browser errors). `cargo mutants --in-diff`:
rebellion-core 57 mutants, 53 caught, 2 unviable, 2 missed (`|` to `^` in
`Planner::able` over the disjoint bits `0x2` and `0x20000000`, equivalent);
rebellion-data 6, 5 caught, 1 unviable; rebellion-app 2, both in `main.rs`
(`main`, `apply_ai_actions`), which no native test reaches.

Hashes are in `SHA256SUMS`.
