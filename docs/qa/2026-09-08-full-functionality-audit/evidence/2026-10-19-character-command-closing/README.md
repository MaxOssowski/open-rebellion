# Character command: closing (F-038, stage 7)

Date: 2026-10-19.

`ghidra/notes/character-command.md`, `ghidra/notes/object-popup-menu.md`.
This is stage 7, the last, of the tracker item "Dowodzenie i jego utrata
przy ranie".

## The original

**The en route test.** `FUN_004f9860`, the order's base check, refuses an
object that is en route (`+0x50` bit 4) with error 4, after the side,
active and destroyed tests.

**The checked row.**
- `FUN_0051d990` builds a fresh order for each menu item:
  - slot `+0x24` (`FUN_0051fb70`) puts the selection in the team at
    `+0x2c`;
  - slot `+0x2c` sets the target;
  - the list at `+0x40` stays empty.
- The Command orders' slot `+0x14` is `FUN_00580d80`. It returns
  `FUN_0051fd30`, or 0 for more than one selected object (read from the
  image).
- `FUN_0051fd30` (`0x0051fd30..0x0051fe14`) walks that list with
  `FUN_00553720`. The decompile drops two of its blocks. With slot `+0x54`
  (`FUN_0040f340`, 1), it checks the row when some item answers yes and
  none answers no.
- The list is empty, so no Command row is ever checked.

## Port

- `order_allowed` and `give` take `en_route`, the port's
  `MissionState::is_en_route`, and refuse with `Refusal::EnRoute`.
- `main.rs`: `command_rows` and the `OrderCommand` handler pass it.
- `object_menu.rs` no longer checks a row:
  - `ObjectMenuRow::checked`, `OrderGates::command_checked` and
    `CHECK_BITMAP` are removed;
  - this replaces the stage-3 `hyp:` that checked the held kind.
- `object-popup-menu.md`: Mission's "returns 0" now explains why.

## Tests

- `character_command.rs`:
  `a_command_order_needs_a_free_uninjured_character_its_class_allows`
  gains the en route case, and destroyed comes before en route. It fails
  without the test (`fail-without.txt`).
- `object_menu.rs`:
  `a_characters_command_submenu_lists_none_admiral_general_commander`
  now expects no row to show a bitmap.
  - It had expected the held Admiral row to show 11902. That came from the
    `hyp:` this stage disproves (`FUN_00580d80`, `FUN_0051fd30` above).

## Browser acceptance (`acceptance.json`)

Run with `agent-browser` on the interface build, fixture 68, both sides.
Each side chose in turn:
1. Admiral;
2. Admiral again;
3. General.

- The log reads:
  - `"Admiral Skywalker" ... now=Some(Admiral)`;
  - `"Luke Skywalker" ... now=Some(None)`;
  - `"General Skywalker" ... now=Some(General)`.
- No console errors.
- The submenu shows None, Admiral, General and Commander with no check
  mark, also after the command was given (`*-2-after-admiral.png`).
- Screenshots are not committed (original artwork):
  - `target/acceptance/character-command-3/`;
  - their SHA-256 are in `screenshots.sha256`.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 6.

## Gates

- `cargo test --workspace`: 2471 passed, 0 failed.
- The seed-42 golden is unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `4412c73a94b5545cb8247a4492044facf0b7197e36f7ddf781e0fd652773d0e1`).
- The Fleet window gate passes all 48 cases (interface WASM
  `b4c0ca33093cc0c7bbc31bbb42dcb5ff6f372629ca50ed9eec87fb6369a80d69`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.8 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-render`):
  8 mutants, 5 caught, 3 unviable.
- `provenance-scan check`: 85 uncited items, unchanged.

## The item

Every stage of the plan is ported:
- the field and the orders;
- the menu and the names;
- the readers (bombardment, assault, detection, AI records);
- the move;
- the hooks;
- the base check.

What remains open in the note changes no behaviour: which events reach the
move and order handlers, since the port calls both rules directly. The AI
giving commands belongs to module `0xc`, a separate tracker item.

Hashes of the committed files are in `SHA256SUMS`.
