# A character's Move from its menu (F-040)

Date: 2026-10-20. Stage M5c of the "Misje" area (tracker item "Powrót,
automatyczna trasa i dryf postaci": the player's character moves).

The reading is in `ghidra/notes/move-order.md`, "A character's Move".

## The original

- `0x201` expands a character into a `0x241` order (`FUN_0053c810`). That
  order is the mission order class of `0x240`, building the Move mission
  `0x41000001`.
- Enabled: `FUN_004f4a00` only builds the command, so Move is enabled for
  any character that resolves.
- Validated (`FUN_004f4b60`): a movement kind (family `0x40..0x4f`) whose
  target's side bits are not the order's side is refused with `1`/`0x28`.
- The command `0x250` (`FUN_0054d360`) then applies the member rules.

## Manual (p. 46, pp. 110-111)

- Move from the right-click menu: Present.
- Confirmed Move with its transit window: Disabled (port:).
- Dragging a character onto a system or fleet: Missing.
- A character's base, which Move changes and the Return goes to:
  - the Return is Partial: it goes to the nearest own system;
  - changing the base is Missing (the `+0x70` writer is untraced).
- The table is in the note.

## Port

- `OrderGates::character_move`: a living character of the player's side;
  a prisoner reads its captor's side.
- The menu's Move opens targeting (`TargetOrder::CharacterMove`).
- A release on a system gives `PanelAction::MoveCharacter`:
  - `missions::move_target_allowed` (`1`/`0x28`);
  - then `dispatch_guarded` for the Move mission, logged as
    `[interface] character_move`;
  - a refusal posts "Move refused".
- port:
  - a release on a fleet is refused;
  - Confirmed Move stays disabled;
  - a special force's Move and the drag of a character are not ported.
- Fixture scenario 69, `CharacterMove`: as `CharacterCommand`, with the
  secondary system the player's.

## Tests

- `object_menu.rs`: `a_characters_move_row_follows_its_move_rule`. It
  fails without the gate (`fail-without.txt`).
- `missions.rs`: `a_move_order_targets_only_a_system_its_side_holds`.
- The scenario tables of `interface_test_fixture.rs` include scenario 69.
- `cargo test --workspace`: 2477 passed, 0 failed.

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the interface build (WASM SHA-256
`6677cf42dce215cfaafc34fb64f42de5523de61ccdb8fc8ba57965f80881d69a`), both
sides. Each run right-clicked the agent (Luke Skywalker, in the Defenses
window of Akrit'tar), chose Move and clicked F'tral's planet in the sector
window.

- Fixture 69, where F'tral is the player's: `result=Ok(0)`, the Move
  mission.
- Fixture 68, where F'tral is the enemy's: `result=Err("1/0x28")`.
- The menu showed Move enabled and Confirmed Move and Retire disabled.
- No console errors; 4 requests per run, all HTTP 200.
- Screenshots are not committed (original artwork):
  - `target/acceptance/character-move/{alliance,empire}-{move,refused}-{1-menu,2-targeting,3-after}.png`;
  - their SHA-256 are in `screenshots.sha256`.

## Gates

- The 24 dual-AI campaigns are identical to the stage before.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `b44b55c251b4e77c8ce6335dae3af76fec411f73ba5335653a1d65e0ecfc2efc`;
  golden unchanged).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.1 ms |
| Fast | 3601 | 0.5 ms | 1.5 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-render`): 9
  mutants, 8 caught, 1 unviable. The app's wiring is covered by the
  browser acceptance.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
