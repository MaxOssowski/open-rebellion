# P58-B21 shared post-battle orchestration

P58-B21 routes played tactical battles and automatic space-combat resolution
through one post-battle campaign path. The shared path now applies orbital
bombardment, lands surviving troop cargo, continues contested ground combat,
occupies an unopposed system, and returns the player through the existing
Battle Results destination controls.

This checkpoint improves production behavior without changing the 106-cell
denominator. Strict original-view acceptance remains 0 of 106 because no
lossless A0 baseline was supplied.

## Implementation result

- Interactive and automatic space-battle outcomes call the same
  `resolve_post_battle` entry point.
- Bombardment, troop landing, ground-roster construction, occupation, and
  Alliance-headquarters survival are no longer duplicated in the app loop.
- A contested landing enters the production `GroundCombatState`; an unopposed
  landing applies occupation and returns to the galaxy.
- The browser Battle Results journey requires a
  `shared-post-battle-campaign-path` probe before it can pass.
- The existing Alliance system destination and Imperial fleet destination both
  remain usable after strategic loss persistence and post-battle resolution.

## Verification

| Gate | Result |
|---|---|
| Complete Rust workspace | 794 passed, 0 failed, 34 ignored |
| Focused tactical-flow tests | 7 passed, 0 failed |
| Harness tests | 26 passed, 0 failed |
| Fixture build validation | 38 scenarios and 152 executions; production fixture-token exclusion passed |
| Deterministic tactical matrix | 106 of 106 A1 cells mapped; denominator unchanged |
| Muted browser journey | 4 of 4 passed across both factions and both viewports |
| Browser startup and cleanup | 4 four-request starts, 4 muted launches, 4 closed browsers |
| Shared campaign-path probe | Present in all four browser cases |
| Independent code review | PASS after the post-battle browser assertion was hardened against an all-zero false positive |
| Independent browser review | PASS; all 40 retained PNGs inspected, no blank, clipped, or corrupted panel found |
| Strict 106-cell gate | Not run; A0 coverage and acceptance remain 0 of 106 |

The final browser run is `2026-09-27T10-30-56-889Z-17718`. Raw screenshots,
console logs, request ledgers, and per-case results remain under the ignored
artifact tree. Durable hashes and the acceptance boundary are recorded in the
[artifact bundle](p58-b21-tactical-post-battle-orchestration/README.md).

## Remaining boundary

- Exact global campaign RNG sequencing remains open.
- Original arrival callbacks, recovery trajectories, power allocation, planet
  placement, and native Death Star beam timing remain open.
- Native trench-run playback and the remaining completion, destruction,
  warning, and ordered trench voice callers remain open.
- Audio mixing and audible native comparison remain open.
- Every strict tactical cell still requires lossless owned-original A0 evidence
  and the complete comparison contract.
