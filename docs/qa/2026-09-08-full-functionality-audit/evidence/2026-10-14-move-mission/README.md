# The Move pseudo-mission (F-040)

Date: 2026-10-14.

`MissionKind::Move` runs the mission lifecycle on MISSNSD `0x41000001`
(TEXTSTRA 11265): both sides, special forces `0xf0f`, characters
`0x10000`, timer 0 + rand(0..=0) (at least one tick, `FUN_00586130`),
hidden (`+0x5c`), detection on, no resigning, own-side target only. The
class (vtable `0x665578`, `FUN_0056ebc0` = `0x41`) keeps the base phase
handler `FUN_00524b70`, so phase 10 rolls no one and the mission ends with
code 1 and no Mission Report; its rules are the base rules
(`FUN_005215e0` -> `FUN_00523450`). A member's status reads the record's
hidden flag (`FUN_00520b70`), so a character on a Move is not "On Mission".

The research planner's state 7 (`FUN_004bd570`) now orders the Move to the
target while its designer stands elsewhere (another system or a fleet);
state 8 orders the research once the designer stands there. The dialog
never lists Move: its record is hidden and it is outside the planners'
kinds. This corrects `ghidra/notes/ai-mission-planning.md`, which had said
Move has no MISSNSD record and runs no detection.

## Headless campaign

`rebellion-playtest data/base --seed 2 --ticks 600 --dual-ai` (release);
`playtest-seed2-move.jsonl` keeps the Move and research dispatches. The
Alliance AI moved Crix Madine to Umgul on day 28 and ordered Troop Training
Research there on day 70. Seeds 1 to 12 each order 2 to 14 Moves. In seed
42 every designer the AI sends already stands at its site, so no Move is
ordered and the replay golden is unchanged.

## Gates

- `cargo test --workspace`: 1753 passed, 0 failed. New tests fail without
  the change: `a_move_takes_its_member_to_the_target_and_ends_with_no_result`
  (no phase 10 result, hidden status) and
  `a_designer_away_from_the_site_is_moved_there_before_its_research`. Four
  research AI tests now expect the Move first (`FUN_004bd570`) with their
  team and target unchanged; a fifth places its designer at the site to
  keep testing the research order.
- The ignored seed-42 replay passes with the golden unchanged
  (`v1:68580f6dd8f9f6b0`). `scripts/check-replay-equivalence.py
  --skip-build --json` passes: native and WASM agree on all nine
  checkpoints, 4 requests, 0 browser errors. WASM `open-rebellion.wasm`
  SHA-256 `c64e7334375ccf1c115ebb0e44fab2ea605cceb980b57d0546744cd7f26de4c0`.
- `cargo mutants --in-diff`: 39 mutants, 32 caught, 5 unviable, 2 missed.
  None is missed in rebellion-core or rebellion-render; the 2 missed are
  `apply_panel_action` and `apply_mission_result` in the app's `main.rs`,
  where only a log name and message category changed; no native test
  reaches them.
- `provenance-scan check`: 104 uncited items, all within the baseline.
- Clippy and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
