# A character dragged out of the Defenses window moves (F-040)

Date: 2026-10-20. Stage M5i of the "Misje" area.

## The original

- `FUN_00422ce0` handles the `0x29a` drop by window type
  (`fleet-window.md`, `move-order.md`):
  - types 4 (Fleet) and 10 (System Defenses) issue `0x201`;
  - type 9 (the System window, which lists only facilities) issues
    `0x214`.
- A character in the Defenses window therefore drags as a Move, which is
  the Move mission order `0x241` (manual p. 111).
- This settles the open question of the previous stage: `0x214` never
  carries a character. A character's slot `+0x200` is a removal
  (`FUN_00534b70` -> `+0x204` -> `+0xac(0x14)`).

## Port

- `DefensesWindowState` holds a drag, `end_drag` (`FUN_006083c0`'s
  threshold of 24 list pixels, outside the list) and `is_dragging`.
- A left press on a character's cell starts the drag; the release gives
  `DefensesWindowAction::DragCharacter`.
- The app resolves the release like the Fleet window's drop, into
  `PanelAction::MoveCharacter`, and blocks the galaxy map meanwhile.
- port:
  - only a character drags (a regiment, special force or squadron moves
    from its menu);
  - the drag image and cursor are not drawn.

## Tests

- `defenses_window.rs`:
  `a_character_dragged_out_of_the_list_drops_where_the_button_comes_up`
  covers:
  - the drop;
  - no drop inside the list;
  - the list-pixel threshold at scale 2.
- It fails without the drag (`fail-without.txt`).
- `cargo test --workspace`: 2488 passed, 0 failed.

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the gate's interface build (WASM SHA-256
`d795ced03bee30244b68a91efa61baf4241d63fdffd164689fee9e15994ee706`), both
sides. Each run dragged Luke Skywalker from Akrit'tar's Defenses window
onto F'tral's planet in the sector window.

| Fixture | F'tral | Result |
|---|---|---|
| 69 | the player's | `result=Ok(0)`, the Move |
| 68 | the enemy's | `result=Err("1/0x28")` |

- No console errors; 4 requests per run, all HTTP 200.
- Screenshots are not committed (original artwork). Their SHA-256 are in
  `screenshots.sha256`, under
  `target/acceptance/defenses-drag/{alliance,empire}-{move,refused}-{1-dragging,2-after}.png`.

## Gates

- The 24 dual-AI campaigns are identical to the stage before.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `0f8e0b313891d3aef11a7abd58337a8eaf24058edca1030083a5e75e54a89909`;
  golden unchanged).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 24.6 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.9 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - the first run: 17 mutants, 10 caught, 5 unviable, 2 missed;
  - after the threshold cases were added: 11 caught, 5 unviable, 1 missed.
- The missed mutant turns `is_pointer_button_down_on() && rect_contains(cell,
  press)` into `||`. It is equivalent here: a press that holds the
  cell's response already lies in the cell. The second term mirrors the
  Fleet window's guard.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
