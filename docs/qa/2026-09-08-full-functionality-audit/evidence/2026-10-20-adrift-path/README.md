# A destroyed fleet's characters go adrift, not captive (F-040, save v93)

Date: 2026-10-20. Stage M9b-2 of the "Misje" area, on the user's go-ahead
after the overnight run.

## The original

- `FUN_00556710` calls a character's slot `+0xb4` (`FUN_004ef2e0`) when
  its container is destroyed and the character is not. A character on no
  mission then goes adrift (`Role_SetAdrift` `FUN_00534790`, `+0x78`
  bit 6), unless the container was removed with reason `0x14`.
- `FUN_0054c7d0` settles an adrift character that stays (its side's
  standing place), clearing Adrift and Autorouting together.
- It sends the others to the autoroute destination (`FUN_00543040`) on an
  Adrift order `0x44000004` (`FUN_00549230` -> `FUN_00547880`). Its
  two-pass detection (mode word `0x4103`) gives the enemy fleets and
  system their chance to capture them.
- The notes are `mission-lifecycle.md` ("Adrift and the abodes") and
  `decoy-roll.md` (mode 3).

## What the port did before

`integrator.rs` (`apply_space_combat_result_inner`) made the characters of
a losing, emptied fleet the winner's prisoners at once. The rule had no
source ("parity: officers captured on fleet destruction").

## Port

- `Character::adrift` (save v93).
- When the battle removes an emptied fleet, its characters stand at the
  fleet's system. Those on no mission go adrift; none is captive. This
  applies to winner, loser and a draw alike.
- `MissionSystem::send_adrift` runs at the start of each mission step:
  - a character at a standing system of its side settles
    (`MissionEffect::AdriftSettled`);
  - the others of one side at one system leave together on an Adrift order
    (`MissionKind::Adrift`) for the nearest place of their side. The order
    steps at once, and its detection may capture them.
- Telemetry: `adrift_ordered`.
- port:
  - only the battle sets Adrift; no other destroyed container does yet;
  - a character with nowhere to go stays adrift rather than going to its
    side's jail (`FUN_00549580`).
- The native and browser app share both functions, so they behave the
  same.

## Save v93

- `SAVE_VERSION` 92 -> 93.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:e4659ba5e1a2e054` -> `v1:57cfc0d727199c15`;
  - final `v1:21001b04a1c4b3b9` -> `v1:4dee6ff02fdbe5bc`;
  - two recordings are identical, and only the fingerprints changed.

## Tests

- `missions.rs`:
  `adrift_characters_settle_at_their_sides_system_and_leave_others_together`.
  It fails without `send_adrift` (`fail-without.txt`).
- `integrator.rs`:
  - `a_destroyed_fleets_officer_leaves_the_battle_on_an_adrift_order`
    covers the battle, the mission step and the Adrift order;
  - `fighter_losses_update_rosters_and_set_the_defeated_fleets_officers_adrift`
    replaces the test that asserted the unsourced capture, with
    `FUN_00556710` -> `FUN_004ef2e0` as the evidence;
  - `a_return_order_of_the_day_is_recorded_with_its_team_and_destination`
    now also covers `adrift_ordered`.
- `cargo test --workspace`: 2494 passed, 0 failed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- all 24 are identical to the stage before;
- no `adrift_ordered`: the AI puts no characters aboard its fleets yet (its
  command module is not ported), so no character loses its fleet in these
  campaigns;
- the path is shown by the integration test above.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `b8cfddec1f2e64f29075a13d2fb5afb5fca8375b9b18c3de4c46c29f6a148c94`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.6 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - the first run: 16 mutants, 14 caught, 1 unviable, 1 missed
    (`send_adrift`'s id step);
  - after the test checked the order's id: 15 caught, 1 unviable.
- `provenance-scan check`: 54 uncited items, unchanged.
- The tests added after the gates change no code that the WASM builds.

Hashes of the committed files are in `SHA256SUMS`.
