# The AI's research reserve (F-036)

Date: 2026-10-18.

The AI never sent a character on a covert mission. The cause is the
records' research reserve, which the port held for the whole game
(`ghidra/notes/ai-mission-planning.md`, "The research reserve").

## The original

- `FUN_00402230` (lines 72-93) marks a record reserved (`0x8000000`) for
  each research capability it has (`+0x34` bits `0x800`, `0x1000`,
  `0x2000`) while the controller's bit for that tree (`0x2000000`,
  `0x4000000`, `0x8000000`) is clear.
- The bits are set by `FUN_004198a0`, `FUN_00419870` and `FUN_004197b0`
  once the side's research reaches its target, which is the side's
  finished tree.
- The capability comes from `FUN_0054c440`, which needs a nonzero skill
  in the tree.
- The covert selectors forbid reserved records. The forbid masks
  `0x4f400051` and `0x0f400051` hold `0x8000000`.

## Port

- `AgentRecord::reserve` and `Planner::research_done`. The AI evaluation
  passes the three trees' `ai_research_done`.
- The port's earlier reading was a `hyp:`. It took the bits as always
  clear, and it reserved every character that a Research record admits,
  whether or not that character had any research skill.
- No save format change.

## Tests

- `mission_planning.rs`:
  - `a_character_able_for_an_unfinished_research_tree_is_reserved_from_sabotage`
    replaces `a_character_free_for_research_is_reserved_from_sabotage`,
    which asserted the always-clear reading. The new test checks that
    another tree's bit keeps the reserve and the character's own tree
    frees it;
  - `a_character_with_no_research_skill_is_not_reserved_for_research`.
- Shown failing without the change (`fail-without.txt`).

## Campaigns (`playtest-research-reserve.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the persistent-planner commit:

| | Before | After |
|---|---|---|
| Missions dispatched | 13,590 | 12,217 |
| Teams with two characters | 0 | 613 |
| Sabotage with characters | 0 | 1,467 |
| Incite with characters | 0 | 615 |
| Rescue / Abduction with characters | 0 | 187 / 95 |
| Successes | 14,889 | 12,030 |

- Successes fall while characters join covert missions. No AI betrayal
  was learned in these runs.

## Gates

- `cargo test --workspace`: 2414 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause. The reserve now reads the
  research capability, which frees characters with no research skill from
  day one.
  - The initial fingerprint is unchanged (`v1:5ba66653ae1fb76d`).
  - Final `v1:7869f8168ab1782f` -> `v1:2d45f0777ed8cd0d`.
  - Two recordings are identical, and only the fingerprints changed.
  - The replay equivalence check passes with 0 browser errors.
  - Release WASM SHA-256
    `95057194571c454a2e18dcaa7afbaf8f7887f51b709292e2ada914e212623e8b`.
- The Fleet window gate passes all 48 cases (interface WASM
  `68fdcc0e20c74a9ee9b6d76c49de5bd67c9f3fc4148bc85c7ad3bc26b047c85d`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 30.1 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.7 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 21 mutants, 19 caught, 2 unviable.
- `provenance-scan check`: 88 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
