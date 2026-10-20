# A character's Confirmed Move (F-040)

Date: 2026-10-20. Stage M5f of the "Misje" area.

## The manual (pp. 110-111)

"Another way to move a character is to click on Confirmed Move. This
brings up a window that tells you the transit time (in days) it will take
for the character to reach his destination. To confirm the move, click the
checkmark. To cancel, click the X button."

## The original

- `0x202` expands a character into the same Move mission order `0x241`.
- `FUN_00487cc0` opens the confirmation window. Its text has one line per
  moving object, from `FUN_0053c2e0` (`FUN_004f4850` for `0x241`).
- `FUN_0049a350` takes the evacuation warning `0x7056` and pictures
  1031/1030 when the first member is not a fleet and its container is a
  blockaded system.
- The checkmark resubmits the order (`FUN_0041ce20(order, 1)`).

## Port

- The character menu's Confirmed Move is enabled as its Move is.
- `TargetOrder::CharacterMove { confirmed }`. On release,
  `confirm_character_move` checks the target (`1`/`0x28`), then opens
  `MoveConfirmation` with `characters`, the line (name,
  `missions::move_days`) and `blockaded`.
- The checkmark issues `PanelAction::MoveCharacter`, to a system or aboard
  a fleet.
- `missions::move_days`: the character's system to the target at the
  speed `FUN_00548370` sets (GNPRTB 3083 for a free Han Solo).

## Tests

- `object_menu.rs`: `a_characters_move_row_follows_its_move_rule` now
  expects Confirmed Move.
- `move_confirmation.rs`:
  `a_blockaded_characters_move_shows_the_evacuation_warning`.
- Both fail without the change (`fail-without.txt`).
- `missions.rs`: `a_moves_days_follow_the_travel_rule_and_han_solo_s_speed`.
- `cargo test --workspace`: 2485 passed, 0 failed.

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the gate's interface build (WASM SHA-256
`ca835298d48711a73bf669bde28c7f56f53dc4347a94a2693e26d4e87bf69ad4`), both
sides. Each run right-clicked Luke Skywalker, chose Confirmed Move, clicked
the target and pressed Enter (`FUN_0044f640`).

| Fixture | Target | Window | After Enter |
|---|---|---|---|
| 69 | the player's F'tral | "Transit time in days / Luke Skywalker: 17" | the Move, `Ok(0)` |
| 70 | the player's fleet at F'tral | the same, 17 days | the Move aboard the fleet, `Ok(0)` |
| 68 | the enemy F'tral | none: "Move refused" (`1`/`0x28`) | - |

- No console errors; 4 requests per run, all HTTP 200.
- The blockade warning is covered by the unit test only.
- Screenshots are not committed (original artwork). Their SHA-256 are in
  `screenshots.sha256`, under
  `target/acceptance/character-confirm/{alliance,empire}-{confirm,board,refused}-*.png`.

## Gates

- The 24 dual-AI campaigns are identical to the stage before.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `b1a5efcd49bdb3cc839b27edbaeba82c62374cf9389b45e7f4d512f711ca137d`;
  golden unchanged).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.3 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.7 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-render`): 20
  mutants, 18 caught, 2 unviable.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
