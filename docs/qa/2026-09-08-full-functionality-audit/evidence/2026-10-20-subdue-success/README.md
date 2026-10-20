# A Subdue Uprising success ends the revolt in the app (F-019)

Date: 2026-10-20. Stage M6 of the "Misje" area (tracker item "Ujarzmienie
(Subdue) i pełny skład misji").

F-019 left one item pending. The app's call to
`apply_mission_state_effects` runs in the frame loop, which no unit test
reaches, so a Subdue Uprising success needed a browser pass to confirm the
uprising clears.

## The original

- Subdue Uprising (MISSNSD `0x57000080`, timer 2 + rand(0..=10) days,
  repeating) rolls SUBDMSTB in phase 10 (`FUN_00569b90`).
- A success adds support (`FUN_0055cb10`), then runs the end check
  `FUN_0050c910` (`FUN_00569c20`), which ends the revolt once regiments
  cover the garrison.

## Fixture

- Scenario 72, `SubdueSuccess`:
  - the primary system, the player's, revolts (`ControlKind::Uprising`,
    `UprisingState::active_uprisings`);
  - eight of the player's regiments garrison it;
  - the agent is on a Subdue Uprising mission there (`dispatch_guarded`).
- `runs_the_clock` sets the game speed to Fast for it.
- The app logs `[interface] uprising_subdued` where it posts "Uprising
  subdued at" (notification 3).
- `interface_test_fixture::apply` now takes the uprising state.

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the interface build (WASM SHA-256
`0935d4861ecb4ed891b398eb42de0c2743cbde64e7763a84310d2a9d6e04829d`), both
sides:

| Side | Log | Day |
|---|---|---|
| Alliance | `uprising_subdued system=Akrit'tar` | 9 |
| Empire | `uprising_subdued system=Akrit'tar` | 6 |

- No console errors; 4 requests per run, all HTTP 200.
- Screenshots are not committed (original artwork). Their SHA-256 are in
  `screenshots.sha256`, under
  `target/acceptance/subdue-success/{alliance,empire}-{1-start,2-subdued}.png`.

## Tests

- `interface_test_fixture.rs`:
  `the_subdue_scenario_revolts_the_primary_and_sends_the_agent_to_subdue_it`.
- The scenario tables cover scenario 72.
- `cargo test --workspace`: 2487 passed, 0 failed.

## Gates

- The 24 dual-AI campaigns are identical to the stage before.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `ee4f13eb3ec426fa09fee39313bb9e16ebe3b0bd3b3c1eb8483a3b465cbf60a6`;
  golden unchanged).
- The Fleet window gate passes all 48 cases (interface WASM `0935d486...`,
  the acceptance build).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 24.9 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-render`): no
  mutants. The change is in the app and its fixture; the browser pass
  covers it.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
