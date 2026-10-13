# Overnight log

## Summary (2026-10-14, main checkout, branch `overnight/2026-10-14`)

Two behavior fixes and three documentation changes are committed; two items
are blocked on decisions only you can make. Nothing was pushed and `main` is
untouched. Gates at the branch head match the baseline: build ok, all
non-ignored tests pass (core 737, up 3), clippy warning counts unchanged, the
105 pre-existing `cargo fmt` diffs unchanged (touched files add none), the
provenance check passes, and `./scripts/build-wasm.sh` succeeds.

**A second session ran this same prompt in parallel.** It took backlog items 7
to 12 on branch `overnight/2026-10-14-b` in `.claude/worktrees/overnight-b`
and reports in `OVERNIGHT_LOG_B.md` there. This summary covers items 1 to 6
and 14 only. The two branches both start at `03cb5d1d` and are not merged.

### Features completed

| Item | Commit | What |
|---|---|---|
| 1 | `fb89f98a` | Betrayal reads its table with the original step lookup (F-028) |
| 1 follow-up | `90324942` | Reworded my own comment so the betrayal rule stays counted as uncited |
| 2 | `d6adacfe` | Captive escape reads ESCAPETB with the step lookup; the unused interpolating lookup is removed (F-028) |
| 4 | `1151da6f` | Provenance tags for the support bands (`hyp:`) and the fog model (`port:`); baseline 113 to 108 (F-031) |
| 5 | `ea49ca22` | Agent docs no longer call `FUN_004927c0` the master tick, `0x1f0` a daily AI trigger, or `FUN_00508250` per-tick (F-039) |
| 14 | `8d370254` | Audit notes for two rules it lists as missing but the code has (F-034, F-037) |
| build | `95ec4717` | Rebuilt the tracked `web/open-rebellion.wasm` (unoptimized: `wasm-opt` is not installed here) |

### Features blocked

- **3, KDY production modifier (F-023).** Removing it drops a positional field
  from the save format and needs two tests deleted.
- **6, Emperor's 1.5x bonus (F-024).** Removing it needs the tests that assert
  it deleted or rewritten.
- **13, injury timer (F-038).** Not attempted by either session: save-format
  bump.

### Things that did not go to plan

- **The seed-42 replay golden cannot be checked on this tree.** The test fails
  before any change of mine, and still fails with a 200-system galaxy forced.
  Items 1 and 2 change betrayal and escape odds, so they almost certainly move
  the golden, but I could not measure or regenerate it. F-028 therefore stays
  "partially remediated".
- **Three other data-dependent tests fail at baseline** (fleet seeding, repair
  telemetry, tactical asset manifest). Details under "Baseline".
- **This session finished its share in about 20 minutes.** I looked for more
  work and found only the item 14 docs sync; see the next section for why.

### Assumptions about original-game behavior to verify

No `// ASSUMPTION:` comment was added: both code changes follow
`FUN_00595090`, already ported and tested as `MstbTable::step_lookup`. What I
tagged or noticed instead:

- `SupportTier::from_support_int` is now marked `hyp:`. Its 20/30/40/60 edges
  are a guess about when the original announces a support change.
- Betrayal still reads **UPRIS1TB** as a loyalty-to-percent table. The
  uprising code reads the same table as incident codes. The audit says the
  original runs betrayal only in mission phase 9, so the standalone
  `BetrayalSystem` is probably invented as a whole.
- Escape still uses loyalty alone and rolls **every tick for every captive**.
  From the saved decompiles (`FUN_00561990`, `FUN_0055e7e0`) the original
  input is: captive slot `+0x1f0` plus slot `+0x1e0` (espionage), minus the
  average slot `+0x1f0` of the characters (types `0x30..0x3c`) at the
  captive's container whose slot `+0x1d4` returns 0, minus the count of
  regiments (`0x10..0x14`) there. Slot `+0x1f0` is combat if the skill slots
  run in field order, which I did not confirm. Not ported, because three
  things are unrecovered: what triggers `FUN_0057d670` (the cadence), what
  slot `+0x1d4` filters, and whose side `+0x24` bits 6-7 select. The port
  also has no location for a captive.
- `ai-parity-tracker.md` calls `FUN_00508250` 18 dispatch validators while
  `economy.rs` calls it an 18-function state rebuild. One of them is wrong.

### Suggested next items

1. Decide items 3, 6, and 13 (save-format bump and test deletions).
2. Find out why the seed-42 fixture loads 100 systems and why its initial
   fingerprint no longer matches, then regenerate the golden once, naming
   items 1 and 2 among the causes.
3. Recover the three open points of the escape roll above; with them it is a
   small port and closes an F-034 bullet.
4. The 18 `GNPRTB_*` constants in `economy.rs` count as uncited only because
   their notes are trailing comments. Check each parameter's meaning, then
   move the notes above the constants.
5. Review and merge `overnight/2026-10-14-b`. Expect conflicts in
   `OVERNIGHT_BACKLOG.md` and possibly the audit JSON and Markdown.

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

### 2. Escape reads ESCAPETB with the step lookup (F-028) — done

- `MissionSystem::check_escapes` now calls `step_lookup`.
- `MstbTable::lookup` (the interpolating one) had no caller left in `crates/`
  or `tools/` and no test of its own, so it is removed. The module docs in
  `missions.rs`, `world/mod.rs`, and `agent_docs/simulation.md` no longer
  describe a piecewise-linear lookup.
- Test: `escape_chance_steps_between_table_rows_instead_of_interpolating`
  (shown failing with the old call; also pins the roll-equals-chance boundary,
  which closes the one surviving mutant). Mutants on `check_escapes`: 8 caught,
  1 unviable, 0 missed after the boundary assertion.
- Not changed: the escape rule itself (loyalty as the ESCAPETB input, one roll
  per captive per call) has no cited source in the code.
- F-028's audit status is left as "partially remediated": its acceptance asks
  for a named golden change, which this tree cannot produce (see baseline).

### 3. Drop the KDY "production modifier" (F-023) — blocked, nothing changed

- `SystemEconomy.production_modifier` is a positional bincode field in the
  save, so removing it breaks the save format.
- `kdy_production_modifier_reduces_with_ships` and
  `kdy_production_modifier_floors_at_zero` assert the invented value. Any
  version of this change (remove the field, or keep it and stop computing it)
  needs those two tests deleted.
- Both are yours to approve. The survey that wrote the backlog missed this.

### 4. Provenance tags for `SupportTier` and `fog.rs` (F-031) — done

- `SupportTier::from_support_int` is tagged `hyp:` (band edges are hard-coded
  stock threshold values; no recovered function announces a support change at
  them). `FogSystem::advance` is tagged `port:` (the visibility model is ours).
  Comments only, no behavior change.
- `scripts/provenance-baseline.json`: 113 entries to 108. Two are these tags;
  three (`ai.rs evaluate_production`, `integrator.rs
  apply_build_completion_inner`, `simulation.rs run_simulation_tick`) were
  already cleared by earlier commits and only dropped out when the baseline was
  rewritten.
- A mistake of mine, fixed in its own commit: the comment I added for item 1
  named `FUN_00595090` inside `BetrayalSystem::advance`, and the scanner then
  counted the whole betrayal rule as cited. The comment now points at
  `MstbTable::step_lookup`, and betrayal stays in the baseline.
- Not done, for you to decide: 18 `GNPRTB_*` constants in `economy.rs` are
  "uncited" only because their notes are trailing comments the scanner does not
  read. Moving the notes above each constant would clear them mechanically; I
  left them because that would shrink the count without anyone checking the
  parameter meanings (two of them are the F-023 pair).
- The audit JSON and Markdown for F-031 are untouched; the finding stays open
  and its counts (135 items) were already stale.

### 5. Correct the false claims in the agent docs (F-039) — done (docs only)

- `agent_docs/ghidra-re.md` and `agent_docs/systems/ai-parity-tracker.md` no
  longer call `FUN_004927c0` master turn processing or `0x1f0` a daily AI
  trigger. The tracker row is marked REFUTED and the "evaluation frequency"
  rows now say the original cadence is not recovered (F-036).
- `FUN_00508250` is described as a one-time setup pass in those two files, in
  `agent_docs/simulation.md`, `agent_docs/architecture.md`, and the
  `economy.rs` module comment (which now says the port runs it every tick
  while the original does not).
- `ghidra/notes/timer-scheduler.md` notes the correction.
- Audit JSON and Markdown: F-039 moves to "partially remediated" (the
  community-address citations and `entity-system.md:383` remain); F-028 records
  items 1 and 2 and stays "partially remediated" because the golden change is
  unnamed.
- Left alone: `ai-parity-tracker.md` still describes `FUN_00508250` as "18
  AND-chained dispatch validators" while `economy.rs` calls the same function
  an "18-function strategic state rebuild". Both cannot be right; the timer
  note only settles when it runs, so I did not pick one.

### 6. Remove the Emperor's 1.5x battle bonus (F-024) — blocked, nothing changed

- Removing the bonus means deleting or rewriting the tests that assert it,
  which the overnight rules forbid without your sign-off.

### 14. Record two audit bullets that are already ported (F-034, F-037) — done (docs only)

- Looking for more work after items 1 to 6, I found the audit lists two rules
  as missing that the code already has: the `FUN_00559be0` support divisor
  with the `FUN_0050c9f0` clamp (`uprising.rs` `support_change`,
  `apply_support_change`) and the `FUN_00520cd0` leadership average
  (`uprising_mission_terms`). Both have tests citing their functions.
- The audit JSON and Markdown get a dated re-check note under F-037 and F-034.
  Statuses are unchanged; I did not re-check their other bullets.
- I found no further item that is small, has its evidence in the repo, and
  needs neither a save-format change, a deleted test, a new decompile, nor a
  GUI pass. See "Suggested next items" in the summary.

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
