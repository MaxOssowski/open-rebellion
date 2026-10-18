# Luke's heritage comes only from the confrontation (F-038)

Date: 2026-10-18.

In the original, Luke learns `KnowsHeritage` (`+0xb0` bit 0,
`FUN_0056f650`) the first time he confronts Vader
(`FUN_00560200`, `ghidra/notes/encounter.md`). The previous stage ported
that confrontation. The port's own story event "Final Battle Imminent"
(`0x396`) still set the flag at the end of Dagobah, before any meeting.
This stage removes that.

## Port

- `EventAction::SetHeritageKnown` and its handler in
  `apply_event_action_to_world` are removed. `0x396` only posts its text.
- Save v75: `EventState` stores the event definitions, so a v74 save
  carries the removed action.
- Two original paths set `heritage_known` now:
  - the confrontation (`mission_detection.rs`);
  - Leia's discovery by Luke (`force_discovery.rs`).
- port: the rest of the Final Battle chain (`0x393` to `0x396`, `0x220`)
  is still the port's own. The original's (DarthPickup,
  DarthToLukeFinalBattle, FinalBattleReady, `FUN_0054ba00`) is not read.
  The Final Battle picture still branches on `heritage_known`.

## Tests

- `integrator.rs`:
  `final_battle_imminent_leaves_luke_s_heritage_to_the_confrontation`
  runs the story events through `0x396` and applies their actions. Luke
  does not know his heritage afterwards.
- Shown failing without the change: the test, added to the `HEAD` tree in
  a separate worktree, fails on the heritage assertion.

## Campaigns (`playtest-heritage.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1, 2, 3, 7, 13, 42 and 47,
1200 days.

- Luke now learns his heritage at his first meeting with Vader:
  - seed 2: day 716 (`0x396` fired on day 610);
  - seed 3: day 488 (`0x396` fired on day 410).
- His Force is 80 or more by then (after Dagobah), so he takes no wound.
- On the other seeds he never meets Vader, so his heritage stays
  unknown.

## Browser

- The Fleet window gate passes all 48 cases (interface WASM
  `f66e8c593df6e989a2ad53122d40b844f57ef3d9f0aa5629becae4dc9167380a`).
- The release build ran with `agent-browser` (headless Chromium, 640 by
  480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 24.8 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 3.7 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The screenshots show original artwork, so they are not committed
  (`target/acceptance/heritage/`):

```
f5c0796cf13ff14b29beb7ae520b651968d6681d6e4b0de51cf00f03a38b21b2  1-veryslow.png
9c2ca6033bc8abfd76a0cc6188f62700f13bd1ed2eca85dce5af2a4a45976ff8  2-fast.png
```

- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2357 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (save v75, the removed
  action):
  - initial `v1:57115a2a37530826` -> `v1:ad8518eb172a430f`;
  - final `v1:e6bb717f2403b483` -> `v1:e811b12322f7e20e`;
  - two recordings are identical, and only the fingerprints changed in the
    manifest;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `1d554e8953bb28eb2ce986c709ed85f03696a52708c1d11d9b380bd7a5609f81`.
- `cargo mutants --in-diff`: 2 mutants, 2 caught.
- `provenance-scan check`: 88 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
