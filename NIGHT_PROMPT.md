
# Overnight autonomous session: Open Rebellion

You are working unattended overnight on my fork of Open Rebellion, a Rust/WASM
reimplementation of Star Wars: Rebellion (1998). Nobody will answer questions
until morning. Never stop to ask. Decide, document the decision, and keep going.

## Setup (do this first)
1. Run `git status`. If the tree is dirty, commit the current state as
   "wip: pre-overnight snapshot" so nothing of mine is lost.
2. Create and switch to a branch: `overnight/<YYYY-MM-DD>`. Do ALL work there.
3. Read README, CLAUDE.md (if present), the Cargo workspace layout, and any
   docs/notes on file formats and game mechanics. Build a mental map before coding.
4. Confirm the baseline builds and tests pass: `cargo build`, `cargo test`,
   `cargo clippy`, plus the WASM build the project uses. Record the baseline
   result (including failures that already existed) in the log.

## Backlog
- If `OVERNIGHT_BACKLOG.md` exists, work through it top to bottom.
- If it doesn't, create it. Survey the codebase, TODOs/FIXMEs, open issues in
  docs, and gaps against the original game's mechanics. Then write an ordered
  list of 8–15 small, self-contained features. Order them by dependency first,
  then value. Each item gets a one-line goal and a clear "done when" criterion.
- Prefer features that complete existing systems over starting new ones.

## Per-feature loop
For each feature:
1. Mark it `[in progress]` in the backlog.
2. Read the relevant code. Keep changes scoped to this feature. Don't refactor
   unrelated code.
3. Implement it. Where behavior must match the original game, base it on
   evidence in the repo (parsed data, existing notes, documented formats).
   When you have to guess, put a `// ASSUMPTION:` comment in the code and note
   it in the log.
4. Add or extend tests that cover the new behavior.
5. Verify: `cargo fmt`, `cargo build`, `cargo test`, `cargo clippy` (no new
   warnings), and the WASM build. Everything must pass.
6. Commit with a Conventional Commit message, e.g.
   `feat(combat): resolve fleet bombardment against planetary shields`.
   The body says what changed, why, and any assumptions.
   One feature = one commit, or a few if they're logically separate.
7. Mark the item `[done]` and append an entry to `OVERNIGHT_LOG.md`.

## When stuck
- If a feature still fails verification after 3 genuine attempts, do NOT
  commit broken code. Run `git stash` or `git checkout .`, mark it `[blocked]`
  with the reason and what you tried, and move on.
- If a feature turns out much larger than expected, split it into sub-items in
  the backlog and do only the first slice.
- Never disable, delete, or weaken existing tests to make things pass.

## Hard rules
- Never push, never touch remotes, never rewrite history (no rebase, amend of
  earlier commits, or force operations). Never switch back to or modify `main`.
- Never commit original game assets or extracted copyrighted data (.dat files,
  sprites, audio, text dumps). Check .gitignore covers them. Code and format
  documentation only.
- Don't add new dependencies unless clearly necessary. If you do, justify it
  in the commit body.
- Don't delete files outside what a feature requires.
- Commit the backlog and log files as you go, so progress survives a crash.

## Morning report
When the backlog is exhausted or you're near the end of your budget, add a
"Summary" section at the top of `OVERNIGHT_LOG.md` with:
- Features completed (with commit hashes)
- Features blocked and why
- Assumptions about original-game behavior that I should verify
- Suggested next items
Commit it as `docs: overnight session summary`.