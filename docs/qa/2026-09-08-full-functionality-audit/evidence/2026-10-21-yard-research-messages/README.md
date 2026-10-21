# The yards' own research posts the original research messages

Date: 2026-10-21 (overnight run, "Produkcja i naprawy"). This follows
`de0c4ac1`, which ported the messages for research missions.

## The original

- A level reached by the yards' own research (timer `0x381`, the tally)
  goes through the same points hook (`FUN_005326f0` and siblings) as a
  research mission.
- So it posts the same notifications: Research Complete (`0x125`..`0x127`,
  `FUN_00499aa0`) and, when the tree is marked Done, Research Exhausted
  (`0x129`..`0x12b`, `FUN_0048abc0`).

## What the port did before

The app's yard-tally path still posted "<Side> <Tree> tech advanced to
level N" and nothing for a finished tree.

## Port

- The tally path uses `research_complete_texts` for each level reached.
- It notes each tree's Done flag before the tally, and posts
  `research_exhausted_texts` for each tree that turns Done.

## Gates (`f38`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `54052f1ab96d7d75817907f83857a24d945a848dd4597acf286c5de34c66f5f8`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- The text functions' tests are in `de0c4ac1`. This change is in the
  app's loop only; no new Rust test.

Hashes of the committed files are in `SHA256SUMS`.
