# Research planners (F-040)

Date: 2026-10-14. The port's research AI simplifications were checked
against REBEXE.EXE (`ghidra/notes/ai-mission-planning.md`, "The port's
simplifications checked") and fixed with persistent research planners
(save v37).

## Headless campaign

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai` (release
build); `playtest-research-planners.jsonl` keeps the research dispatches,
results, level-ups and AI aborts.

| Side | Research dispatched | Designers | Levels reached |
|---|---|---|---|
| Empire | 3 Ship Design, 2 Facility Design, all to Coruscant | Bevel Lemelisk, Thrawn, Klev, Orlok, Villar | Ship 1 to 7 (days 73, 124, 164, 194, 272, 382, 510); Facility 1 to 3 (days 379, 502, 546) |
| Alliance | none | — | — |

Each Empire planner holds its own designer, so five research missions run
side by side. The Alliance had research sites for troops (1) and
facilities (2) and three able characters per tree, but its candidates were
not yet recruited (record flag `0x1`, forbidden by the research queries),
so no planner found a member. No abort occurred.

The run differs from `evidence/2026-10-14-research-ai/` from the first day
because the AI now holds designers, breaks ties by coin flips and passes
over candidates at blockaded systems, so the two campaigns are not
comparable turn by turn.

Gates: `cargo test --workspace` 1750 passed, 0 failed. `cargo mutants
--in-diff`: rebellion-core 102 mutants, 91 caught, 6 timeouts (endless
planner loops, so caught), 5 unviable, 0 missed; rebellion-data 6, 5
caught, 1 unviable; rebellion-app 2, both in `main.rs` (`main`,
`apply_ai_actions`), which no native test reaches. The seed-42 golden
is regenerated for save v37 and the AI changes:
`v1:a6ee248d7d0194dc` -> `v1:68580f6dd8f9f6b0`;
`scripts/check-replay-equivalence.py --skip-build --json` passes (native
and WASM agree on all nine checkpoints, 0 browser errors).

Hashes are in `SHA256SUMS`.
