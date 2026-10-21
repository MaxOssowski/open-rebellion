# The battle's winner and the loser's withdrawal (F-033, W2j)

Date: 2026-10-21. This ports the result words, the winner rule
(`FUN_005ae9c0`, `FUN_0040c780`), and the loser's withdrawal
(`FUN_00516ef0`, `FUN_00515480`). The reading is in
`ghidra/notes/battle-flow.md`, "The winner and the loser's withdrawal".

## Change

- `AbstractBattle::finish` builds the result words:
  - words 0 and 2: the side's retreat ran and one of its units escaped;
  - word 1: the Empire has no active unit, word 0 is clear, and no Death
    Star is operational;
  - word 3: the Alliance has no active unit and word 2 is clear.

  The winner comes from words 1 and 3, then from words 0 and 2.
- `port:` when words 0 and 2 are both set, the battle has no winner. The
  original asks the system's last arrived side (`+0x78` bits 6..7), which
  the port does not keep; keeping it needs a save-format change.
- `abstract_battle::withdraw_loser` sends each loser fleet at the system to
  `missions::nearest_place` (`FUN_005545e0`), leaving out the battle's
  system. It runs in:
  - the headless simulation;
  - the app's AI battles;
  - Auto Resolve;
  - the end of a tactical battle.
- `port:` a fleet with no destination, or one that cannot move, stays. The
  loser's planet squadrons stay.

No side may retreat yet (`retreat_allowed` is still false). A loser
therefore has no fleet left to send away, and campaigns do not change.
The W5 read entry said the port counts escaped units as survivors. That
was wrong: the port's winner already counted only active units.

## Gates

- Campaigns: `rebellion-playtest data/base --dual-ai`, seeds 1 to 24,
  1500 days, against a HEAD (`ffb6bd60`) worktree build. 24 of 24 event
  streams are identical (`campaigns.json`).
- Tests:
  - 4 new tests;
  - workspace 2533 passed, 0 failed;
  - replay goldens pass. The only failure is the long-standing
    `seeds::tests::fixed_fleet_tables_only_target_special_systems`
    (ignored data test).
- Each new test fails with its change neutralized.
- Mutants on the diff: 35 tested, 32 caught, 3 unviable (`mutants.txt`).
  The first run left 12 survivors. They were closed by the test
  `a_retreat_word_needs_the_retreat_and_an_escapee`, the second-pass
  Empire case, and by passing the winning `Faction` into `withdraw_loser`.
- `provenance-scan check`: 0 uncited.
- Browser, Arch Chromium 155.0.8059.84:
  - Fleet window 58 of 58 pass;
  - replay equivalence passed (`v1:e01dc3b9b044c28e` ->
    `v1:7737eb96955b5cd7`, no browser errors, 4 requests);
  - frame timing: very slow median 0.5 ms, p99 0.8 ms; fast median 0.5 ms,
    p99 1.6 ms; 0 frames over 50 ms (`frame-timing.json`, screenshots).
- Release WASM: `771bb224621769f746b99ce76acf8845d9c804b4bc3dfefa40a03f05fbd677d9`.
