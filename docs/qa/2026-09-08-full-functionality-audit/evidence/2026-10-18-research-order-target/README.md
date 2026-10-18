# The research order goes to the site (F-036)

Date: 2026-10-18.

`ghidra/notes/ai-mission-planning.md`, "The research order's target".

## The fault

In 22 of 24 campaigns, the AI ordered one designer to research at the
same system every few days, 35 to 394 times. Each mission ended the next
day without a result. A temporary trace of the mission ends showed code
`0xc`, no research site (`FUN_0056c510`). The target system held no yard
of the tree. For example, seed 3 ordered Klev to Drall 119 times.

## The original

- `FUN_0047b360` (state 8) orders the mission on the planner's `+0x30`
  (at `0x47b3a0`).
- The research slot `+0x50` copies the site it chose (`+0x34`) into
  `+0x30` (`FUN_0047be50` at `0x47c0cd`).
- `+0x38` (state 6, `FUN_004bd0a0`) is where the team gathers.
- A slot `+0x50` with no site ends the cycle.

## Port

- `research_cycle` ordered on `+0x38`, which kept an old system while
  that system was still the side's. It now orders on the site, and with
  no site the cycle ends.
- No save format change. The seed-42 golden is unchanged: the first 25
  days never move a site.

## Tests

- `ai.rs`: `the_research_order_goes_to_the_site_from_wherever_the_team_gathered`.
  It fails without the fix (`fail-without.txt`).

## Campaigns (`playtest-research-order-target.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the Recruitment commit:

| | Before | After |
|---|---|---|
| Research orders (ship + troop + facility) | 18,414 | 602 |
| Research successes | 7,643 | 8,300 |
| Missions dispatched | 26,519 | 8,728 |
| Seeds with a designer ordered over 20 times | 22 | 0 |
| Systems at day 1500, summed (Alliance / Empire) | 487 / 363 | 500 / 340 |

## Gates

- `cargo test --workspace`: 2418 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:6f750b483b72f71f` -> `v1:4a2572231a842e24`) and 0 browser errors.
  Release WASM SHA-256
  `158a7b37275538f204f6f1caf9a733ad03d36b24e19b2e0fcda5c7ab064cb76b`.
- The Fleet window gate passes all 48 cases (interface WASM
  `2171113a04547dd622300d948725ef4acee513c6d6153401efad53ba1c4aec19`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.3 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 3.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`): 1 mutant, caught.
- `provenance-scan check`: 88 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
