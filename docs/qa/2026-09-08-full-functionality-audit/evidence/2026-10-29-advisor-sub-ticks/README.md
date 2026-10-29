# The advisor's schedule counts sub-ticks (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy". This closes the fault
that `../2026-10-29-agent-advice/` found: the port's advisor schedule
counted the original's sub-ticks as days.

## The original

- The schedule's times come from `DAT_006b28cc`, which
  `FUN_004c0a60` / `FUN_004c2b20` / `FUN_004c2dd0` refresh from
  `FUN_004fcee0`.
- `FUN_004fcee0` is the scheduler's `+0xc`, which `FUN_0051dea0` raises
  once a sub-tick. The day `+0x10` rises once the sub-ticks reach the
  speed's rate: 600 Very Slow, 60 Slow, 12 Medium, 4 Fast
  (`ghidra/notes/timer-scheduler.md`, `agent-advice-list.md`).
- These times are therefore sub-ticks:
  - the messages' last times and next free times (`+0x168`, `+0x16c`,
    `+0x3c`);
  - the briefing's 40000;
  - the reactions' spans (10, `0x32`, 5);
  - the refusals' and events' 10;
  - the quip's 50 quiet sub-ticks (`FUN_00439ef0`).

## The port

- `AdvisorSchedule` is unchanged in its rules. Its times are renamed
  sub-ticks (`now`, `last_tick`, `span`), and the module docs say so.
- The app gives it `advisor_now`: `GameClock::sub_tick` plus one. hyp:
  the count starts at 1, so the briefing plays on the opening, as
  `FUN_004c0a60`'s next-free test needs.
- The saved table (`PlayerAgent::advisor_last_day`, save v109) keeps its
  layout and name, and now holds sub-ticks. A v109 save from before this
  change holds day values. No such save was published beyond this
  session's runs.

## Browser (Arch Chromium, release WASM `cada9575…`)

- **`agent-objectives.mjs`** passed at `…T21-34-32-232Z`
  (`agent-objectives-result.json`, `*-quip.log`). At Fast the quip
  (Alliance WAVE 1291, Empire 22005) now plays on day 15. Before this
  change it played around day 57: 50 sub-ticks are 12.5 days at 4 a
  day, plus the idle rolls.
- **Regression runs, all passed:**
  - `briefing.mjs`, all four cases, at `…T21-35-21-672Z`;
  - `agent-advice.mjs` at `…T21-41-56-181Z`;
  - `advisor-events.mjs` (interface WASM `8611bf02…`) at
    `…T21-41-51-084Z`.

## Tests

- `cargo test --workspace`: all pass.
- New test: `the_advisor_counts_sub_ticks_from_one` (app). Without the
  change, `advisor_now` would give the day plus one (2, not 13).
- `cargo mutants --in-place --in-diff` (excluding the app's `main.rs`):
  62 mutants, 58 caught, 4 unviable.
