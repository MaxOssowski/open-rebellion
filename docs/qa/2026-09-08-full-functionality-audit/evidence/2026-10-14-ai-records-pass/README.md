# The AI's records in each pass (F-036)

Date: 2026-10-14.

The AI player's tick (`FUN_00485b40`) refreshes the side's records in
phase 4 (`FUN_00418500`) before the modules step
(`ghidra/notes/ai-mission-planning.md`, "Cadence"). The port now does
the same in each `AISystem::pass`:

- `AISystem::records` builds `ai_records::Galaxy`, that is the class,
  fleet, system and sector records. Its inputs:
  - the world, economy, uprisings, manufacturing, research, movement and
    troop transport;
  - the people aboard, read from the planner's records (`Planner::flags`,
    `FUN_00401d20`);
  - the yard marks the side keeps (`AIState::yard_marks`);
  - side `+0xa4`, the count of MNCHARSD classes, which the port takes from
    the minor characters (`hyp:`, one per record).
- `ModuleInputs::galaxy` hands the records to the modules. No module reads
  them yet; module 1 is the first, so the simulation is unchanged.
- `AISystem::pass` takes the economy and troop transport states. Both
  callers pass them: `simulation.rs` and the interactive loop.

port: the original refreshes a record only when its dirty flag is set,
but the port rebuilds every record each pass. On the seed-42 galaxy a
native release build takes about 29 µs per build (200 builds timed
once).

## Browser

The release build ran in `agent-browser` (headless Chromium, 640 by 480)
as a new Empire campaign:

| Speed | Time | Frames | Median frame | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3584 | 0.5 ms | 0.9 ms | 2.6 ms |
| Fast | 15 s | 904 | 0.5 ms | 1.1 ms | 1.6 ms |

Notes on the run:
- Very Slow ran day 1 at `passes=600` (10 orders, 7.0 ms in all); Fast
  ran days 2 to 32 at `passes=4`.
- The console (`console.txt`) has 32 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Screenshots: `1-veryslow.png` (the galaxy at day 1) and `2-fast.png`
  (the Bothawui battle window).

## Gates

- `cargo test --workspace --no-fail-fast`: 1836 passed, 0 failed. New
  tests:
  - `phase_four_builds_the_side_s_records_with_its_marks_and_minor_classes`
  - `a_side_s_minor_classes_are_its_minor_characters`
- Seed-42 golden unchanged (`v1:6685c6f382ede031`). The replay
  equivalence check (`replay-equivalence.json`) passes on nine
  checkpoints, with 0 browser errors. WASM SHA-256
  `eb1d31f4a70cb7b1007f6544a93d0967a625555e99a4773ba73b305def1f2cfa`.
- `cargo mutants --in-diff`: 9 mutants, 7 caught, 2 unviable, none
  missed.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed files. rustfmt: no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
