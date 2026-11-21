# Cleanup loop

A repeatable procedure for improving code quality without changing what the
game does. Run it in iterations, by hand, with `/loop`, as a workflow of
agents, or as a scheduled agent. Each iteration does one small, provable
cleanup.

## The rule

A cleanup changes **no behaviour**. The proof is that every check below
passes **without changing any expectation**:

- the seed-42 replay golden is unchanged
  (`cargo test --release -p rebellion-data --test replay_manifest -- --include-ignored`);
- the multi-seed sweep matches natively and in the browser
  (`replay-gate data/base --sweep`, `scripts/check-replay-equivalence.py --sweep`);
- `cargo test --workspace`, the app tests with `--features interface-test-fixtures`,
  and `cargo run -q -p provenance-scan -- check` pass;
- the full gate batch passes (all gates, 180 battle cases);
- the number of surviving mutants on the touched files does not grow.

If any golden, fixture expectation or gate assertion would have to change,
the change is **not a cleanup**. Revert it and record it as a separate item
that needs its own decision. Never regenerate a golden inside the loop.

## What counts as overhead here

This project is a hand-written reimplementation traced from the original
executable, not a mechanical recompilation. Its overhead is mostly:

- very large files, above all `crates/rebellion-app/src/main.rs` (panel
  actions, presentation hooks, fixtures; roadmap M2);
- dead or half-dead code left by removed heuristics and older models;
- duplicated helpers across AI modules and fixtures;
- the growing chain of save-format migrations;
- the audited rustfmt and Clippy baseline failures (CLAUDE.md).

Keep what is deliberate: citations of original functions, structures that
mirror the original's states, `port:`/`hyp:` tags, tests.

## One iteration

1. **Measure.** `python3 scripts/cleanup-metrics.py` writes
   `target/cleanup/metrics-<stamp>.json` and prints the largest files, the
   longest functions, duplicate 12-line windows, Clippy and dead-code
   warning counts, tags and the WASM size. Optional tools are used when
   installed: `cargo-machete` (unused dependencies), `cargo-llvm-cov`
   (code no test reaches). Use `--skip-clippy` for a quick run.
2. **Pick one target** from the top of the report, small enough for one
   atomic commit, for example:
   - delete a function, field or module nothing uses;
   - move one group of panel actions out of `main.rs` into its own module;
   - merge two copies of the same helper into one;
   - fold old save migrations into one transitional module;
   - fix one Clippy lint class in one crate.
3. **Change without changing behaviour.** Keep the edit mechanical. Do not
   mix a cleanup with a fix.
4. **Prove it** with every check in "The rule". Run the cheap checks first
   (tests, replay golden); run the gate batch once per few iterations.
5. **Commit** atomically (`refactor(...)`, `chore(...)` or `style(...)`),
   explicit staging, then push after a green batch.
6. **Record** the metric change in the iteration log
   (`target/cleanup/log.md`, untracked): target, commit, before/after.
7. Go back to step 1.

## When the loop stops

- No target above the thresholds: no non-test function over 150 lines, no
  dead-code warning, no duplicate window, `main.rs` below the agreed size.
- K iterations in a row (default 3) moved the metrics by less than 1%.
- A target would need a behaviour change: stop that target and report it.

## Ways to run it

- **By hand or with `/loop`:** one iteration per round, batch every few
  rounds.
- **Workflow of agents:** one measures and picks independent targets, a few
  work in separate worktrees, one merges and runs a single batch.
- **Scheduled agent (`/schedule`):** a few iterations a week, results left
  as commits for review.
- For a single pass over recent changes, the `/simplify` skill and
  `/code-review` also help.

## Suggested first targets

1. Dead code: `AdvanceTicks`, leftovers of removed heuristics, unused fields.
2. Split the panel actions out of `main.rs` (roadmap M2).
3. Merge duplicated AI helpers.
4. Gather the old save migrations (v116..v137) into one module.
5. Shrink the rustfmt and Clippy baselines step by step.
