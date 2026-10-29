# Delivered messages reach the advisor's event slot (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy". The reading is in
`ghidra/notes/advisor-briefing.md`, "The events". A subagent swept the
decompile corpus for the senders, and `FUN_0048a060` and the code writes
below were checked by hand.

## The original

- `FUN_0048a060` delivers a message to a side's panel. For kinds 3, 4, 5
  and 8, when its category's flag 4 is clear, it calls the side's advisor
  slot `+0x14` with the message's `+0x28`.
- `FUN_004c44b0` (Alliance) and `FUN_004c22f0` (Empire) turn the code into
  `+0x168[message] = today + days`:
  - code `0x2a` picks one of four advice messages, only while none waits
    (`FUN_004c4430`, `FUN_004c2270`);
  - code `0x2b` picks one of two (`FUN_004c4480`, `FUN_004c22c0`).

## The port

- `AdvisorSchedule::event` holds both sides' cases.
- `GameMessage::with_advisor_event(s)` carries the code per reading side.
- `MessageLog::push` collects the codes of messages filed on a side's rail.
  The app hands the player's codes to the schedule each frame and logs
  `[advisor] reaction event code=…`.
- port: the per-category option (flag 4, menu items `0x171..0x179`) is not
  ported, so every category is on.

Codes given to the port's original-class messages:

| Message | Builder | Code |
|---|---|---|
| System joins | `FUN_00499760` | 1 to the gaining side, 2 to the other |
| Uprising begins / ends | `FUN_00499460` | 1 when (reader holds) differs from (begins), else 2 |
| Units deployed (also en route) | `FUN_00497940` | 3 |
| Facility lost on arrival | `FUN_00497690` | 3 |
| Facility idle | `FUN_0048be60` | 3 |
| Research complete / exhausted | `FUN_00499aa0` / `FUN_0048abc0` | 4 |
| Unit arrival (delivered product) | `FUN_004981c0` | 5 for a capital ship, 6 otherwise |
| Fleet arrival | `FUN_004981c0` case 1 (hyp: the port's notification `0xd`) | 5 |
| Regiment / squadron arrival | `FUN_004981c0` case 3 | 6 |
| Maintenance shortfall | `FUN_00498970` | `0xc` |
| Blockade | `FUN_004960f0` | `0xe` to the holder, `0xd` to the blockader |
| Evacuation losses | `FUN_00495e90` | `0x10`; neither side answers it |
| Informants; disaster; resources | `FUN_00491060`; `FUN_0048e940` | `0x1c` |

Not yet given codes:
- the speaker-mapped messages: mission reports, character health,
  encounters, capture and escape (`FUN_0048db30`, `FUN_00490340`);
- the port's own lines without an original class: bombardment, assault,
  occupation;
- repaired ships (codes 8 and 9; the port posts no such message).

Many of the scheduled messages are op-`0x14` clips (the second droid);
the clip player still ends those at once. That is the next stage.

## Browser

`tools/interface-parity/agent-objectives.mjs` passed for both sides at
`…T14-53-26-738Z`. The WASM hash is `f0842b60…`.

No code was delivered in that run: neither 57-day new game posted the
player a coded message (no production was queued, and the Ithor battle's
lines are the port's own). So the delivery is shown by the tests below,
not in the browser.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `each_sides_events_schedule_their_messages`: every case of both
    switches, from their byte offsets, and the codes a side ignores;
  - `the_advice_picks_one_of_four_only_while_none_waits`;
  - `a_filed_message_hands_each_reading_side_its_advisor_event`;
  - `a_ships_arrival_hands_on_5_and_any_other_units_6`;
  - the app's `joins_event`, `uprising_event` and `blockade_event` tests.
- `cargo mutants --in-place --in-diff`:
  - render crate: 102 mutants, all caught.
  - app: the survivors are in the user's uncommitted `window_conf` /
    `original_game_dir` changes and in the untested main loop (the match
    arms that attach codes, and the player-side compare).
