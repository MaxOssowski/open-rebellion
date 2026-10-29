# The agent's advice, save v109 (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy", "oprawa doradców": the
Advice rail control lit in A0 and never in the port, because the agent's
advice was not ported. Trace: `ghidra/notes/agent-advice-list.md`.

## The original

- **The list.** `FUN_0048b460` reads TEXTSTRA `RT_RCDATA` `0x6000`
  (Alliance) or `0x6800` (Empire): a count of 31, then advice `n` at
  `base + 3n` (the kind in byte 0's low nibble, a `u16` key at `+2`). The
  agent's list `+0x154` keeps key order.
- **Posting.** `FUN_0048a590` → `FUN_0048b2e0` (class `0x0065b188`): the
  reader's Advice message (`0x200`), its title and text TEXTSTRA
  `base + 3n + 1` / `+ 2`, background `0x42f` / `0x430`, `+0x28` = `0x2a`
  for the advisor. The posted node leaves the list.
- **The briefing's end.** Action `0xd` closes the Message Index; with
  advice on, `FUN_00439f20` builds the list, posting the nine kind-7
  advice, and `FUN_0041d770(1, 0x82)` opens the index on Advice.
- **A window's first opening** (`FUN_0043a0b0`): the first advice of its
  kind. Sector 6, System 2, Fleet 3, Defenses 5, Missions 4 (none).
- **The period** (`FUN_00439bc0` → `FUN_00439fb0`): every 300 sub-ticks,
  the first advice whose window was opened, or of another kind.
- **Sub-ticks.** `+0x148` and `DAT_006b28cc` come from `FUN_004fcee0`,
  the scheduler's `+0xc`. It counts sub-ticks: 600 a day at Very Slow,
  60 at Slow, 12 at Medium, 4 at Fast (`FUN_0051dea0`).
- **Saved** with the agent (`FUN_004397a0`): `+0x148`, `+0x150`, `+0x154`.

## A0 (Windows 10 VM, Empire, days 175..178)

The captures stay outside Git; their hashes are in `a0-captures.sha256`.

- `00-resume.png`: the Advice rail control is lit.
- `01-advice-index.png`: the Advice list, newest first. Planetary
  Assaults (key 280), Bombardments, Blockades, Exploration and
  Colonization, Creating Fleets, The Message Window, Galaxy Overview, The
  Galactic Encyclopedia, Finders (160).
  - Keys rise from bottom to top.
  - Kinds 2 and 5 are skipped, since those windows were never opened.
  - More advice arrived in 178 days than a 300-day period allows.
- `02-advice-message.png`: Planetary Assaults opened, with the IMP-22
  picture `0x430`, the title and the text.

## The port

- **Data.** `dat_dumper::types::agent_advice` decodes the records. The
  round trip is tested. `dat-dumper --extract-strings` writes
  `textstra-advice.json` for the browser pack (`build-runtime-pack.py`).
  `rebellion_data::agent_advice_records` reads the DLL natively, or that
  JSON in WASM.
- **Logic.** `rebellion_core::agent_advice::AgentAdvice` has `build`,
  `window_opened` and `check`. `PlayerAgent` gates them on Agent Advice.
- **Clock.** `GameClock::sub_tick` is the scheduler's count: each day adds
  the speed's rate, and the current day adds its run part.
- **Message.** `story_messages::agent_advice_message` builds it.
- **Windows.** The Sector, System, Fleet, Defenses and Missions window
  states report a created (not raised) window through `take_created`.
- **The app:**
  - it checks the windows and the period every frame on the galaxy view;
  - the briefing's `0xd` closes an open index, builds the list and opens
    the index on Advice.
- **Save v109.** `PlayerAgent` and `GameClock` gain the advice state and
  the sub-tick count. Older saves are rejected (the no-migration rule).
  The seed-42 replay golden was regenerated for this save format change,
  which the user approved: only its fingerprints changed.

Two faults this exposed, fixed here:

- **Pop-up menus drew under the Message Index.** Both took egui's Tooltip
  order, and the index raised itself each frame. `FUN_00442860` opens a
  pop-up over every child window, so `game_menu` now takes the top order
  (`Debug`).
- **The Advice speed hold resumed a paused game.** `FUN_00487eb0` sets
  only the rate index (`+0x54`, 1..4), and `FUN_00487ff0` restores
  `+0x58` itself, with no `- 1` as `agent-advice.md` had claimed. The
  pause (`FUN_0041d2f0`) is apart. `hold_speed_for_advice` now changes
  only the rate, and leaves a Paused clock paused. Before, closing the
  index on a new game left a pause pending, and the clock stopped after
  one day.

## Browser (Arch Chromium, release WASM `f3b2a54b…`)

`tools/interface-parity/agent-advice.mjs` passed for both sides at
`…T20-43-16-824Z` (`agent-advice-result.json`, `*-advice.log`). It starts
a new game at the menu's default, Easy, and skips the briefing. Then:

- advice 1..9, kind 7, are posted with their titles;
- the index opens on Advice;
- opening the first row shows its message.

| Side | Advice index | Advice message |
|---|---|---|
| Alliance | `56e39684…` | `9582fd50…` |
| Empire | `dc86aa01…` | `7ac0e5ef…` |

The Empire's message picture matches A0's `0x430`. The Advice rail
control is lit, as in A0.

Regression runs on the same WASM:

- `briefing.mjs` (4 cases) at `…T20-43-36-651Z`;
- `agent-objectives.mjs` at `…T20-41-51-271Z`. It failed before the two
  fixes above: the menu sat under the index, and the clock held after a
  day;
- `advisor-events.mjs` (interface WASM `d76c14cb…`) at `…T20-50-05-708Z`.

## Open

- **The advisor's schedule.** `AdvisorSchedule` counts `DAT_006b28cc` as
  days, but it is sub-ticks. Its reactions, quips and next-free times
  therefore run at the wrong pace.
- **Message Index rows.** These are older faults of the index, not of the
  advice:
  - A0 lists the newest first, with a droid icon per row and the header
    "Advice Messages";
  - the port lists the oldest first, with neither, and a smaller text
    font. `message-index-rows.md` says oldest first, which A0
    contradicts.
- **Not compared in A0:**
  - the briefing's opening of the index;
  - a window's first-opening advice.

  Both are read from the code.
- **The agent's active flag** (`+0x10`) is taken as always on (hyp, as in
  the Agent menu).

## Tests

- `cargo test --workspace`: all pass. With `--include-ignored`, the
  original-data tests pass too:
  - `each_side_holds_thirty_one_advice_records_nine_of_kind_seven`;
  - `original_campaign_replay_matches_after_save_reload`.
- New tests:
  - `agent_advice`: the list build, a first opening, the period, and the
    kind-4 Missions window;
  - `agent_advice_posts_only_while_the_menu_item_is_on`;
  - `the_sub_tick_count_runs_at_the_speeds_rate_within_and_across_days`;
  - `a_round_trip_keeps_the_agents_advice_list_and_seen_windows`;
  - `an_advice_reads_its_sides_texts_files_under_advice_and_wakes_its_droid`;
  - the record decoder's two tests and `a_sides_advice_takes_the_records_kind_nibble`;
  - `the_advice_hold_changes_the_rate_and_never_the_pause`.
- The five window states' tests now assert `take_created` on a creation
  and its absence on a raise.
- `cargo mutants --in-place --in-diff` (excluding the app's `main.rs`):
  123 mutants, 99 caught, 17 unviable, 7 missed.
  - The first run missed 14. New tests closed the `PlayerAgent` gate and
    the kind-4 arm.
  - The 7 left read the player's files. `agent_advice::load` (3) and
    `agent_advice_records` are killed by the `--include-ignored`
    original-data test, which needs TEXTSTRA.DLL.
  - `set_agent_advice_json` is WASM's loader, and the dat-dumper `main` is
    the extraction CLI. Both are exercised by `build-wasm.sh` (31 + 31
    records) and the browser gate.
