# A character in the Fleet window: its menu and its drag (F-040)

Date: 2026-10-20. Stage M5e of the "Misje" area.

## The manual

- p. 56: "drag Jerjerrod onto the fleet", then "right-click on each
  character icon and select Command".
- p. 111: "you can move a character by simply dragging his icon onto a
  destination system or fleet".

## The original

- A drag out of a Fleet window list posts `0x29a` (`FUN_006083c0`).
  `FUN_00422ce0` then issues `0x201` against the window under the release,
  and for a character that expands to the Move mission order `0x241`
  (`ghidra/notes/move-order.md`).

## Port

- The Personnel tab lists each character as an object,
  `ItemObject::Character`:
  - a right click opens the character's menu (Move, Mission, Command, ...);
  - a drag out of the list gives `FleetWindowAction::DragCharacter`.
- The app resolves the release window as for a regiment
  (`release_destination`) and issues `PanelAction::MoveCharacter`: to a
  system, or aboard a fleet.
- `FleetWindowState::show_tab` is for the fixture.
- Fixture scenario 71, `CharacterAboard`: the agent aboard the player's
  fleet, shown on its Personnel tab.
- port: the drag of a character out of a Defenses or system window (`0x214`)
  is not ported.

## Tests

- `fleet_window.rs`:
  - `a_character_dragged_out_of_the_personnel_tab_drops_where_the_button_comes_up`.
    It fails without the object (`fail-without.txt`);
  - `show_tab_switches_an_open_window_and_refuses_a_closed_one`.
- `interface_test_fixture.rs`: the scenario tables include scenario 71.
- `cargo test --workspace`: 2483 passed, 0 failed.

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the gate's interface build (WASM SHA-256
`00ec589ca86b1001ad011927cc4ff08a457a1fd17532b74ab198a8135ab3289f`), both
sides.

- A right click on Luke Skywalker in the Personnel tab of Fleet 4 (or
  Fleet 3) opened his menu, 7 rows: Move, Confirmed Move (disabled),
  Mission, Command, Encyclopedia, Status, Retire (disabled).
- A drag from his icon to the Defenses window of Akrit'tar logged
  `character_move ... to=Some("Akrit'tar") fleet=None result=Ok(0)`.
- No console errors; 4 requests per run, all HTTP 200.
- Screenshots are not committed (original artwork). Their SHA-256 are in
  `screenshots.sha256`, under
  `target/acceptance/character-drag/{alliance,empire}-{0-aboard,1-menu,2-dragging,3-after}.png`.

## Gates

- The 24 dual-AI campaigns are identical to the stage before.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `f450542ad1d84ec6a7ff0beda3f39dca80efc24cb188afeb62491bcdbd614948`;
  golden unchanged).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.5 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - the first run: 10 mutants, 4 caught, 4 unviable, 2 missed
    (`show_tab`);
  - after its test was added: 10 mutants, 6 caught, 4 unviable.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
