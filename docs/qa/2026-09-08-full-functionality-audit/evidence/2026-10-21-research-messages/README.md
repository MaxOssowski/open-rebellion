# Research names the class it opened, and says when a tree is done

Date: 2026-10-21 (overnight run, "Produkcja i naprawy").

## The original

- A side's research order rising posts `SideShipyardRdOrderNotif`
  (`0x125`), `SideTrainingFacilRdOrderNotif` (`0x126`) or
  `SideConstructionYardRdOrderNotif` (`0x127`).
- Research Complete (`FUN_00499aa0`) asks `FUN_00567a90` for the first
  class of the tree's family whose research order equals the new level.
  - That is `FUN_00567880` with only "equal" listed.
  - It names the class in TEXTSTRA `0x7048` "{1:1} Research Complete" and
    `0x7049` "R&D Reports that the {1:1} is now available for
    manufacture.".
  - With no such class there is no message.
- A tree with no next level is marked Done (`FUN_0052f440` and its
  siblings). That posts `0x129` (ships), `0x12a` (troops) or `0x12b`
  (facilities).
- Research Exhausted (`FUN_0048abc0`) answers with `0x7218` "Research
  Exhausted" and `0x7219`, `0x721a` or `0x721b`.

## What the port did before

- Each level reached posted "<Side> <Tree> tech advanced to level N".
- A tree marked Done posted nothing.

## Port

- `story_messages::research_complete_texts` takes the first class of
  `build_selection::research_classes` at that order. Its name comes from
  the capital ship, fighter or catalog class.
- `story_messages::research_exhausted_texts` picks the body by tree.
- `apply_mission_result` (it now takes the message texts) posts:
  - Research Complete for each level reached that opens a class;
  - Research Exhausted when the tree turns Done.
- port: the old line stays only when TEXTSTRA is missing.

## Tests

- New: `research_names_the_class_it_opened_and_says_when_a_tree_is_done`.
  It covers a capital ship, the Empire's class at the same order, a
  fighter, a level with no class, and each tree's Exhausted text.
- `cargo test --workspace`: 2505 passed, 0 failed.
- `cargo mutants --in-diff`:
  - the first run missed the fighter lookup;
  - fighters were added to the test, and the rerun caught 13 of 13.

## Gates (`f30`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `f7d5b6841950588cd624a0511cf384a499ab0f089bc7322ce132986e77f8bffb`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- Not checked: the messages in the browser. No fixture finishes a
  research mission.

Hashes of the committed files are in `SHA256SUMS`.
