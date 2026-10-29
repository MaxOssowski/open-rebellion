# Translate Counterpart and the advisor's saved schedule, save v108 (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy". The user approved the
save format change the same day.

## The original

- Menu item `0x11b`, Alt+V (`0xbc8`), flips `DAT_006b28b0` bit `0x1000`
  (`FUN_00439e80`).
  - Both advisor constructors set it (`FUN_004c27f0`, `FUN_004c0710`).
  - `FUN_00487900` enables the item with the agent and checks it while the
    bit is set.
- The advisor's update plays a message's entries with flag bit 0 only
  while `0x1000` is set and `0x4000` is clear (`FUN_004c0a60`).
- The agent's save (`FUN_004397a0`) writes:
  - `DAT_006b28bc` (the advisor's update runs), `DAT_006b28c0` and
    `DAT_006b28b0`;
  - the advisor's schedule count `+0x164` and its last days `+0x168`.

  The load (`FUN_00439550`) reads them back and zeroes the next free days
  `+0x16c`.

## The port

- `PlayerAgent` (core) carries `translate`, `advisor_enabled` and
  `advisor_last_day`.
  - `for_new_game` turns `translate` on.
  - `toggle_translate`, `advisor_state` and `set_advisor_state` read and
    write them.
- `AdvisorSchedule`:
  - `set_translate` gates bit-0 entries;
  - `from_save` restores the last days and the flag, with the free days at
    0;
  - `saved` gives what the save keeps.
  - hyp: `0x4000` is never set; its writer `FUN_00487f80` is not ported.
- The app:
  - copies the schedule into the agent each frame;
  - rebuilds the schedule from the agent on load (logging
    `[advisor] schedule restored`);
  - passes the flag to the schedule.
  - The Agent menu's Translate Counterpart is enabled and checked; Alt+V
    flips it.
- Save `SAVE_VERSION` 108; `agent_docs/save-load.md`.
- The seed-42 replay golden is regenerated for this format change:
  `v1:7806a75e2fc6a8cf` -> `v1:79ae7796678fc9de`. Only the fingerprints
  in `replay_seed42_v1.json` changed.

Not ported: `DAT_006b28c0`, which the port does not model.

## Browser (`tools/interface-parity/agent-objectives.mjs`)

Run `…T12-59-46-992Z` passed for both sides. In each, Alt+V logs
`translate_counterpart status=off`, then `status=on`. The menu shows
Translate Counterpart and Agent Advice checked, as the A0 capture of the
Empire's menu does (`empire-agent-menu.png`).

## Tests

- `cargo test --workspace`: all pass.
- The replay manifest tests (with the original data, `--include-ignored`)
  pass against the regenerated golden.
- `provenance-scan check`: 0 uncited items.
- New tests:
  - `a_new_games_advisor_translates_and_the_menu_flips_it`;
  - `the_advisors_schedule_and_its_update_flag_travel_with_the_agent`;
  - `without_translate_counterpart_the_bit_zero_entries_stay_silent`;
  - `a_loaded_advisor_keeps_its_last_days_and_flag_and_frees_its_days`;
  - `a_round_trip_keeps_translate_counterpart_and_the_advisors_schedule`
    (save v108);
  - the Agent menu test checks `0x11b`.
- `cargo mutants --in-place --in-diff` (core and render): 33 mutants, 30
  caught, 3 unviable.
