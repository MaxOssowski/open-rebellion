# A character's Move aboard a fleet (F-040, save v92)

Date: 2026-10-20. The user granted all permissions for this run, save
format changes included.

Stage M5d of the "Misje" area. Before it, the player could not put a
character aboard a fleet, which an Admiral or General needs (manual p. 56,
p. 111).

## The original

- The Move order `0x241` takes a fleet as its target (`FUN_004f4b60`):
  - the fleet is the trip's container (`+0x74`);
  - the validator reads the fleet's side, so a fleet of the order's side
    passes even at an enemy system (`1`/`0x28` otherwise).
- The members land in that container (`FUN_004f7640`).
- A traveller whose target moves is rerouted (`FUN_004fbc20`,
  `GameObjParentReroutedNotif`).

## Port

- `ActiveMission::target_fleet` (save v92). The UI sets it with
  `MissionState::set_target_fleet` after a release on a Fleet window
  entry.
- `target_view` reads the fleet's side and place for a Move with a fleet.
- On landing (`landing`), a member boards the fleet when a fleet of the
  mission's side stands at the system: `MissionEffect::MemberBoarded`,
  `board_fleet`.
- `missions::move_place_allowed`: the fleet's side decides.
- Fixture scenario 70, `CharacterBoard`: the agent in the Defenses window,
  and a fleet of the player's side at the enemy secondary system,
  selected in its Fleet window.
- port: the trip aims at the fleet's system when it is ordered; a fleet
  gone from there leaves the member at the system (no rerouting).

## Save v92

- `SAVE_VERSION` 91 -> 92.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:68bc58d8f3fb696d` -> `v1:e4659ba5e1a2e054`;
  - final `v1:5fc59cefe817d1b4` -> `v1:21001b04a1c4b3b9`;
  - two recordings are identical, and only the fingerprints changed.

## Tests

- `missions.rs`:
  - `a_move_to_a_fleet_lands_its_member_aboard`. Without the change the
    Move ends with code 8 before landing (`fail-without.txt`);
  - `a_move_lands_at_the_system_when_its_fleet_has_left`;
  - `a_move_may_target_its_own_sides_fleet_only`;
  - `a_character_boarding_a_fleet_leaves_the_fleet_it_was_on`.
- `interface_test_fixture.rs`: the scenario tables include scenario 70.
- `cargo test --workspace`: 2481 passed, 0 failed.

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the gate's interface build (WASM SHA-256
`97c17ab2245cf99114e6a4deffe6aaad5e82027253c5d988949d6c0a171e0df2`), both
sides. Each run right-clicked the agent (Luke Skywalker), chose Move and
clicked:

| Fixture | Target | Result |
|---|---|---|
| 70 | the player's fleet at the enemy F'tral | `fleet=Some("Fleet 4")` (Alliance) or `Some("Fleet 3")` (Empire), `result=Ok(0)` |
| 69 | the player's F'tral | `fleet=None`, `result=Ok(0)` |
| 68 | the enemy F'tral | `result=Err("1/0x28")` |

- No console errors; 4 requests per run, all HTTP 200.
- Screenshots are not committed (original artwork). Their SHA-256 are in
  `screenshots.sha256`, under
  `target/acceptance/character-board/{alliance,empire}-{board,move,refused}-{1-menu,2-targeting,3-after}.png`.

## Gates

- The 24 dual-AI campaigns are identical to the stage before.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `df25c63e9da8f16f201185970152a0555fd44786558af6c28e8711892f0130e7`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 22.8 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.4 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - the first run: 25 mutants, 22 caught, 2 unviable, 1 missed
    (`board_fleet`'s removal from the old fleet);
  - after a test was added, `missions.rs`: 25 mutants, 23 caught, 2
    unviable.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
