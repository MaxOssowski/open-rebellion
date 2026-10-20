# A fleet emptied by sabotage disbands (F-017, F-019)

Date: 2026-10-20. Stage DS3 of the "Gwiazda Śmierci" area, on the user's
go-ahead.

## The original

- A successful Sabotage or Death Star Sabotage destroys the order's
  object: `FUN_005746e0` calls slot `+0xac(6)`, DestroyedSabotage.
- The manual (p. 105, the mission table) says a Death Star Sabotage
  success "ends in the destruction of the Death Star".
- When a fleet's capital ships change (slot `+0xc4`, `FUN_004fe870`),
  `FUN_004fe630` disbands a fleet left without one (slot `+0xac(0x14)`;
  `fleet-join-split.md`, "Disbanding").

## What the port did before

- The destruction itself was already ported (`missions::destroy_target`),
  and the invented 50-tick construction delay was removed earlier
  (`70916608`).
- But `destroy_target` only dropped the hull, so a fleet whose last ship
  (or only Death Star) was sabotaged stayed in the world empty.
- Its doc claimed a "fleet cleanup" would remove it; no such cleanup
  exists.
- Scrap, Autoscrap and bombardment losses already go through
  `scrap::remove_ship` / `disband_if_empty`.

## Port

- `destroy_target` removes a sabotaged capital ship or Death Star through
  `scrap::remove_ship`. A sabotaged squadron's fleet is checked with
  `disband_if_empty`.
- `Fleet::has_death_star` now follows the fleet's remaining Death Star
  hulls.
- port: the port's rule keeps a fleet that still holds a squadron or a
  character.

## Tests

- `a_sabotaged_last_ship_disbands_its_fleet`.
- `a_fleet_keeps_its_death_star_flag_while_another_death_star_remains`.
- Both fail on the old `destroy_target` (`fail-without.txt`).
- `cargo test --workspace`: 2496 passed, 0 failed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 20 are identical to the stage before;
- seeds 1, 6, 16 and 24 differ from the first fleet a sabotage emptied:
  - in seed 1 the snapshot counts 10 fleets instead of 11;
  - in the other three, a later fleet takes the freed slot
    (`FleetKey`).
- All 24 run to day 1500.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `cd72273644c6945f9a742dd6b11402f23d9bcab9c78a5ee8c75c9bdce0f25049`);
  the seed-42 fingerprints are unchanged (`v1:f47443f73b78e129` ->
  `v1:caadbc1bab82cbda`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.6 ms | 0.8 ms | 23.8 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.6 ms |

- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors; 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 5 mutants, 5 caught.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
