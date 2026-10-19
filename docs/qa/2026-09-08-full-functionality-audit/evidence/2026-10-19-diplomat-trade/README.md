# A Diplomacy planner trades its diplomat for a better free one (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "Diplomacy's slot +0x3c".

## The original

State 2 (`FUN_0047ba40`) asks the class's slot `+0x3c`. For Diplomacy
this is `FUN_00481ef0`. It lets the member go when the best free diplomat
(diplomacy 69 or more) has more diplomacy, and state 3 then takes that
one. Every other class keeps its members (`FUN_0051ebb0`).

## Port

- `Planner::outranked`, asked in `mission_planner_cycle`'s state 2.
- No save format change. The seed-42 golden is unchanged.

## Tests

- `ai.rs`: `a_diplomacy_planner_trades_its_diplomat_for_a_better_free_one`.
  An equal diplomat does not displace the member; a better one does. It
  fails without the trade (`fail-without.txt`).
- `mission_planning.rs`: `a_diplomat_is_outranked_only_by_a_free_better_one`.

## Campaigns (`playtest-diplomat-trade.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the Rescue commit:

| | Before | After |
|---|---|---|
| Diplomacy orders | 1,140 | 1,242 |
| Diplomacy successes / failures | 3,543 / 1,850 | 3,729 / 1,996 |
| Systems at day 1500, summed (Alliance / Empire) | 570 / 579 | 589 / 564 |

## Gates

- `cargo test --workspace`: 2434 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:530e248f2599b293` -> `v1:49ba43802d0d2840`) and 0 browser errors.
  Release WASM SHA-256
  `1525a7ec30e5beb71c8716d78df94a8ecf8dbf89d79c589a9df518f8f52de037`.
- The Fleet window gate passes all 48 cases (interface WASM
  `470e3725f551822c9bec2e58e831d26ec36cc97a6509ef04a27b4b7ab969b65a`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 24.7 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 4.7 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`, `mission_planning.rs`):
  - 10 mutants: 9 caught, 1 missed;
  - the miss (the member's record lookup) is now caught by
    `a_diplomat_is_outranked_only_by_a_free_better_one` (verified by
    hand).
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
