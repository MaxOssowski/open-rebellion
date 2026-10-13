# Overnight log

## Summary (2026-10-14)

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
