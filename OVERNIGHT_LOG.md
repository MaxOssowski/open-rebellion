# Overnight log

## Session notes (2026-10-14)

### Decisions

- **Two sessions ran this prompt at the same time.** The other one created the
  branch and the `wip: pre-overnight snapshot` commit (`03cb5d1d`), then moved
  to its own worktree (`.claude/worktrees/overnight-b`, branch
  `overnight/2026-10-14-b`, log `OVERNIGHT_LOG_B.md`) for backlog items 7 to 12.
  This session keeps the main checkout and items 1 to 6. Item 13 is left for
  you (save-format bump). The two branches are not merged; that is yours to do.
- **Prompt over CLAUDE.md where they conflict:** branch-only, no push.
- **`cargo fmt` is run per touched file** (`rustfmt --check <file>`), because
  the workspace already has 105 formatting diffs and a workspace-wide format
  would rewrite unrelated code.
- **WASM gate per feature is `cargo check -p rebellion-app --target
  wasm32-unknown-unknown`** (the command `agent_docs/deterministic-replay.md`
  lists). The full `./scripts/build-wasm.sh` rewrites the tracked
  `web/open-rebellion.wasm`, so it runs once at the end.
- **CLAUDE.md gates are applied too:** failing-without tests, `cargo mutants`
  on touched files, and `provenance-scan -- check` for simulation code.

### Baseline at `03cb5d1d`

| Gate | Result |
|---|---|
| `cargo build --workspace` | ok |
| `cargo test --workspace` | ok: core 734, render 733, data 125 (13 ignored), app 72 |
| `cargo clippy --workspace --all-targets` | exit 0 with existing warnings: core 5 (8 with tests), render 12 (24), app 6 |
| `cargo fmt --all -- --check` | **fails already**: 105 diffs (status_rows.rs 32, main.rs 13, world/mod.rs 12, movement.rs 12, ...) |
| `provenance-scan -- check` | ok, 109 uncited |
| `cargo test --workspace -- --ignored` (needs `data/base`) | **4 tests fail already**, listed below |

Ignored tests that fail before any change of mine:

- `rebellion-data --test replay_manifest original_campaign_replay_matches_after_save_reload`:
  "seed-42 fixture requires 200 systems, loaded 100". Forcing a 200-system
  galaxy in a throwaway experiment still fails: "initial fingerprint does not
  match the reviewed golden". **The seed-42 golden does not reproduce on this
  tree, so no item tonight can confirm or regenerate a golden change.** I did
  not regenerate it.
- `seeds::tests::fixed_fleet_tables_only_target_special_systems`: Empire fleets
  at 4 systems, expected at most 3.
- `telemetry_coverage_all_sys_constants_emit`: `repair` emits no events.
- `tactical_assets::tests::owned_native_complete_corpus_installs_with_exact_counts`:
  `data/base/ui/tactical-dll/TACTICAL3D/runtime/manifest.json` is missing.
- The `--ignored` doctests (14) are illustrative snippets and fail to compile.

## Feature entries

### 1. Betrayal reads its table with the step lookup (F-028) — done

- `betrayal.rs` now calls `MstbTable::step_lookup` (`FUN_00595090`).
- Tests: `betrayal_chance_steps_between_table_rows_instead_of_interpolating`
  (shown failing with the old call), and
  `a_roll_equal_to_the_betrayal_chance_does_not_betray`, which closes the one
  surviving mutant (`<` to `<=` on the roll). Mutants: 12 caught, 1 unviable,
  0 missed after that test.
- Golden: not measurable, see baseline.
- Worth knowing, not changed: both callers pass **UPRIS1TB** as the "loyalty
  table" (`simulation.rs:671`, `main.rs:2743`). The uprising code reads that
  same table as incident codes, not percentages, so the betrayal rule as a
  whole has no recovered source. The step lookup only fixes how the table is
  read.

## Earlier attempt (2026-10-14, read-only session)

Kept for the record; superseded by the entries above.

**No features were implemented and nothing was committed.** The session's
permission mode only allowed read-only commands. Every command that writes was
refused with "requires approval", and nobody was there to approve:

| Needed for | Command tried | Result |
|---|---|---|
| Setup step 2 | `git switch -c overnight/2026-10-14` | refused |
| Setup step 2 | `git checkout -b overnight/2026-10-14` | refused |
| Setup step 2 | `git branch overnight/2026-10-14` | refused |
| Setup step 1 | `git add --dry-run`, `git commit --dry-run` | refused |
| Setup step 4 | `cargo --version` (plain, and with the CLAUDE.md `env PATH=...` form) | refused |

Without a branch, a commit, or a build, the per-feature loop cannot run, so I
stopped at a survey rather than edit source on `main` with no way to verify it.

### State of your tree

- Still on `main` at `fc446151`. No branch was created, nothing staged, nothing
  committed, nothing pushed.
- Your uncommitted change to `web/open-rebellion.wasm` and the untracked
  `NIGHT_PROMPT.md` are untouched. **The "wip: pre-overnight snapshot" commit
  was not made**, so that wasm change is still only in the working tree.
- Two new untracked files: `OVERNIGHT_BACKLOG.md` and this log.
- No baseline was recorded: `cargo build`, `cargo test`, `cargo clippy`, and
  the WASM build never ran.

### Features completed

None.

### Features blocked

All of them, for the reason above. The backlog has 13 ordered items, all
`[todo]`.

### To unblock the next run

Allow these in the session's permissions (or run in a mode that permits them):
`git switch`, `git add`, `git commit`, `cargo` (build, test, clippy, fmt, run,
mutants), and `./scripts/build-wasm.sh`.

### Decisions I would have made, for you to confirm

- **Branch before snapshot.** The prompt says to commit the snapshot and then
  branch, but also never to modify `main`. I intended to create
  `overnight/2026-10-14` first and put the snapshot commit there.
- **Prompt over CLAUDE.md where they conflict.** CLAUDE.md says to work on
  `main` and push after each feature; the overnight prompt says branch-only and
  never push. I intended to follow the prompt.
- **CLAUDE.md's Cargo PATH is macOS-specific** (`/opt/homebrew/bin`,
  `/Users/tomdimino/.cargo/bin`). On this Linux machine it will not find a
  rustup-installed cargo; the next run should use a local equivalent.
- **The WASM build overwrites the tracked `web/open-rebellion.wasm`.** It must
  not run until your current wasm change is snapshotted.
- **CLAUDE.md gates beyond the prompt's list** would also apply per feature:
  `cargo mutants` on touched files, `provenance-scan -- check` for simulation
  code, failing-without tests, and source citations. CLAUDE.md also says
  workspace fmt and strict Clippy have known baseline failures, so "no new
  warnings" has to be measured against a recorded baseline.

### Assumptions about original-game behavior to verify

None were introduced; no code changed. Two backlog items would need your
decision before anyone implements them: item 6 (removing the Emperor's 1.5x
bonus means rewriting tests that assert it) and item 13 (the injury timer is a
save-format bump).

### Suggested next items

Backlog items 1 to 5 are the safest start: two table-lookup switches with the
lookup already ported, one dead modifier, provenance tags, and a docs-only
correction. Item 11 may close an audit finding with no code at all.

### Survey findings worth knowing

- Checked against current code, these audit findings still stand: F-023
  (`economy.rs:53`), F-024 (`combat.rs:237-261`), F-025 (`combat.rs:347`,
  `:1346`), F-028 (two interpolating callers left: `betrayal.rs:115`,
  `missions.rs:2533`), F-029 (`economy.rs:1018`).
- F-005 looks stale: `main.rs:2993` sets `GameMode::VictoryModal`.
- `.gitignore` coverage of game data was not audited beyond confirming the file
  is tracked; do that before the first commit of any run that touches `data/`.
