# The advisor's reactions to refused missions (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy". This follows the
reactions to the Agent menu and the toggles
(`../2026-10-29-advisor-reactions/`). The reading is in
`ghidra/notes/mission-dialog.md`, "Refusal".

## The original

A refused order shows no text. `FUN_00487c90` hands the validator's two
status words to the advisor's slot `+0xc`: `FUN_004c2940` for the Alliance,
`FUN_004c0870` for the Empire. Each schedules one message ten days ahead
(`+0x168[id] = today + 10`):

| Case | Alliance | Empire |
|---|---|---|
| the same status as the last one | `0xf`, then `0x10` and the memory clears | `0x10`, then `0x11` |
| `0x40`/`0x91`, no target object, a decoy list | `0x14` | `0x15` |
| anything else, `FUN_0041cd80(2)` | `0x11` or `0x12` | `0x12` or `0x13` |

Each of these messages holds one sprite-DLL record, read from the owned
`.SPT` files with `dat-dumper`:
- C3POACT: `0xf` 10110, `0x10` 10111, `0x11` 10201, `0x12` 10202,
  `0x14` 10212;
- IMP22ACT: `0x10` 11111, `0x11` 11112, `0x12` 11201, `0x13` 11202,
  `0x15` 11212.

## The port

- `AdvisorSchedule::refused(status, decoys_without_target, today)` keeps the
  last status and whether it repeated (`+0x174`, `+0x17c`). It schedules the
  side's message as in the table above.
- The app's `refusal_status` maps the dispatch's `MissionRefusal` to the
  validators' words:
  - `EmptyTeam`: `0x40`/`0x91` (`FUN_0054bb90`), with the decoy case when
    the order has decoys and no target;
  - `TargetUnavailable`: `0x40`/`0x28` (`FUN_0056a110`);
  - `MembersNotAllowed`: `0x40`/`0x01` (`FUN_00583320`).
- hyp: `MemberUnavailable` (`FUN_00522b30`) sends `0x40`/`0x91` without the
  decoy case, because its word is not read.
- hyp: the generic pick uses the port's xorshift in place of the interface's
  generator.
- port: a game without the advisor's messages still writes the message-log
  line it wrote before.

## Browser

No gate drives a refused mission, so this stage has no browser capture of
a refusal. The regression gate `tools/interface-parity/agent-objectives.mjs`
passed for both sides at `…T13-39-46-144Z`. The WASM hash is `23688c81…`.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `a_refusal_schedules_its_reaction_and_a_repeat_the_two_after_it`, for
    both sides: the generic pick from the port's seed, ten days, the
    repeat's two messages, the forgetting, the decoy case, and both generic
    messages over eight refusals;
  - `only_an_emptied_team_with_decoys_and_no_target_takes_the_decoy_reaction`.
- `cargo mutants --in-place --in-diff` on the stage's diff: 44 mutants,
  8 missed.
  - 4 are in the user's uncommitted `window_conf` / `original_game_dir`
    changes, which this stage does not touch or commit.
  - 4 are in the app loop, which has no tests: `main`, `apply_panel_action`,
    and the two lines there that compute `has_target` and `has_decoys`.
  - The schedule's generic pick (`% 2 == 0`) was equivalent at first. The
    test now pins the first pick from the port's seed and kills it.
