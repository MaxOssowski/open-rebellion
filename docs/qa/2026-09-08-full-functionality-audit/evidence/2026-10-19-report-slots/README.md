# Report slots of Recruitment, Reconnaissance and Espionage (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "Mission Reports of the standing
planners".

## The original

- Recruitment (`FUN_0047dad0`): a success makes an own calm target the
  gathering place; a failure only counts.
- Reconnaissance (`FUN_0047f720`): a success marks the target seen today
  (`FUN_00433dc0`); either result clears the target.
- Espionage (`FUN_00480eb0`): the same, and it also clears the gathering
  place.

## Port

- `mission_planner_reports` reads these three slots.
- `AIState::bind_mission` takes the order's first member of any kind. An
  Espionage team is special forces, so its missions never bound before.
  The integrator and the app pass the member.
- No save format change.

## Tests

- `ai.rs`: `recon_espionage_and_recruitment_read_their_own_reports`. It
  fails without the Recon and Espionage arm (`fail-without.txt`).
- The tests calling `bind_mission` pass a `MissionMember`.

## Campaigns (`playtest-report-slots.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the reports commit:

| | Before | After |
|---|---|---|
| Reconnaissance orders (successes) | 2,771 (2,552) | 1,690 (1,660) |
| Espionage orders | 1,895 | 2,545 |
| Espionage successes / failures | 1,755 / 673 | 1,913 / 629 |
| Systems at day 1500, summed (Alliance / Empire) | 399 / 416 | 607 / 539 |

## Gates

- `cargo test --workspace`: 2428 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
- Seed-42 golden regenerated for this cause (the report slots):
  - initial unchanged (`v1:530e248f2599b293`);
  - final `v1:45ce6a74e7ac0f51` -> `v1:75f7a5f65cee3222`;
  - two recordings are identical, and only the fingerprints changed;
  - the replay equivalence check passes with 0 browser errors;
  - release WASM SHA-256
    `5278786490a517ca9999a6eccf6867b38fd10c4e5fe7b2f399ff3d6270d9e4b7`.
- The Fleet window gate passes all 48 cases (interface WASM
  `75e87eebee416924d84a910e590edc6a3c501edee6faa5f5cdc9ea1316ae7595`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 22.8 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 4.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`ai.rs`):
  - first run: 18 mutants, 7 missed (Recruitment's failure count and
    own check);
  - after the test was extended: 18 mutants, 18 caught.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
