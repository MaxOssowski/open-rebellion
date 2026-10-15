# The AI's posture and sector plans: module 8, stage 1 (F-036)

Date: 2026-10-15.

Module 8's sector planners carry out plans the strategy (module `0xd`)
makes. This stage ports the strategy's side of it; the planners and their
six plan objects come next. The reading is in
`ghidra/notes/ai-sector-plans.md`.

## Port (save v49)

- `crates/rebellion-core/src/ai_strategy.rs`:
  - the posture (`FUN_004bf490`): the side's fleets' attack against 1.5
    times the opponent's (`DAT_0065c49c`), spare regiments against the
    weakest opponent sector, the free energy and raw materials, open
    sectors and the yard requests keep a posture or choose one of seven;
    weak or busy sets controller `+4` `0x800000`, posture 5 `0x400000`;
  - a strategy per posture (`FUN_004bf7c0`); a new one replans
    (controller `+4` bit 2);
  - state 1 (`FUN_004e5530`): each posture's slot `+0x14`
    (`FUN_004e62b0`, `FUN_004e7990`, `FUN_004e8940`, `FUN_004e9690`,
    `FUN_004ea060`, `FUN_004ead50`, `FUN_004e6c00`) names the sectors and
    their plan kind, its first choice then the shared tail, including
    posture 4's yard target (`FUN_004e5b10`) and posture 5's staleness
    key (`FUN_0041a060`, `FUN_004317e0`);
  - state 2 (`FUN_004e5560`): the review grows or shrinks the list.
- Sector records carry the plan (`+0x30`, `+0x34`, `+0x38`, `+0x3c`) and
  its bits (`FUN_004321e0`'s end: `0x4000000`, `0x10000`, `+0x18c`);
  `Galaxy::planned_sectors` is `FUN_00419e90`, `Galaxy::sectors_near`
  `FUN_00419b80` from a sector.
- `Tallies` gains the posture's sums (`FUN_0041b7e0`, `FUN_0041ba80`) and
  `+0x204`; `Controller` the garrison goals `+0x37c..+0x384`, which the
  system records read; `AIState::seen` keeps record `+0x44`.
- Module 6 now raises the garrison goals under `0x800000` when every
  query fails (`FUN_004df440`) and lowers them under `0x400000` without a
  budget (`FUN_004df9a0`).

port:
- Module 8's planners are not ported: no plan has one, so the review
  grows the list only while it is empty, each side keeps one planned
  sector, and no plan gives an order. Module 8 still runs the port's
  fleet heuristics.
- States 4 to 6 and the Alliance's states after the strategy are not
  ported; record `+0x44` is set from blockades only.
- Posture 6's second fallback (`0x408` by `+0x130`) cannot pick when the
  first (`0x408` by `+0x19c`, the same masks) found nothing; it is ported
  as read.

## Campaign (`playtest-strategy.json`)

`rebellion-playtest data/base --seed 42 --ticks 600 --dual-ai`, with a
temporary trace in `Strategy::step` (removed):
- The Empire starts in posture 6 (attack 3194 against 166, 8 spare
  regiments against 6), then holds posture 4 for most of the game (a
  module waits for the yards), with spells of 5.
- The Alliance moves between 1 (weak, no spare regiments) and 4.
- Each names one sector at a time: the Alliance kind 4 (shipyards, then
  construction yards) and kind 3; the Empire kind 4 and kind 2.
- The garrison goals never moved: no module 6 cycle found every query
  empty under `0x800000`, and posture 5 never met an empty budget there.

## Browser

Release build, `agent-browser` (headless Chromium, 640 by 480), a new
Empire campaign:

| Speed | Time | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|---|
| Very Slow | 60 s | 3748 | 0.5 ms | 0.6 ms | 2.1 ms |
| Fast | 60 s | 3745 | 0.4 ms | 0.6 ms | 3.2 ms |

No frame took over 50 ms. The AI's longest day took 16 ms (day 1 at Very
Slow, 600 passes; `console.txt`). The console has 32 `[ai]` lines and 0
errors; `network.txt` lists 4 requests, all HTTP 200. Native GUI
acceptance was not run.

## Gates

- `cargo test --workspace --no-fail-fast`: 2089 passed, 0 failed.
  - `ai_strategy`: 20 tests, among them
    `the_posture_matches_the_decompiled_rule_at_every_boundary`, which
    compares the port with a line-by-line transcription of
    `FUN_004bf490` over 184,320 cases around every threshold.
  - `ai.rs`: the strategy's first two passes and the refreshed records'
    plan bits; `ai_module6.rs`: the goals' raise and lowering;
    `ai_module1.rs`: the new tallies.
  - Each new test fails with its change neutralized.
- Seed-42 golden regenerated for save v49 and the strategy's queries
  (their coin flips draw from the AI's generator):
  initial `v1:bde18d3a1df8ed02` -> `v1:cca1045386d40ca4`, final
  `v1:e407af1c444ac566` -> `v1:feef56582a180eed`. The replay equivalence
  check (`replay-equivalence.json`) passes with 0 browser errors. WASM
  SHA-256 `1170aa8340d9b782a3c574f106f64a53dd4f491f497451f05964c401e4b95aef`.
- `cargo mutants --in-diff` (`ai_strategy.rs` whole, the changed lines
  of `ai.rs`, `ai_module1.rs`, `ai_module6.rs`, `ai_records.rs`): the
  first run, 317 mutants, missed 91, most in the posture's thresholds; the
  grid test and seven more tests followed. The rerun: 313 mutants, 300
  caught, 3 unviable, 2 timeouts, 8 missed; two of those (`ai.rs`, the
  seen test's `+0x30 & 6`) were then caught by an own blockaded system in
  the test. The other six are equivalent:
  - `Tallies::of` `<` -> `<=` in `+0x210`'s fold: an equal value replaces
    itself;
  - `set_plan` `|` -> `^` and `choose_posture` `|=` -> `^=`: the bit is
    always clear there;
  - `name_sectors` arm 1 deleted: posture 1 then reaches
    `first_choice`, which gives none for it;
  - `first_choice` `!` deleted in posture 6's second fallback: that
    branch cannot pick (above);
  - `staleness` `>` -> `>=`: 0 divided is 0.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy and rustfmt: no findings in the changed lines.

Hashes are in `SHA256SUMS`.
