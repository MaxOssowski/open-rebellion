# Event ids and the original's timers (F-038)

Date: 2026-10-19.

This is the parity tracker's item "Identyfikatory zdarzeń kolidują z
timerami", in the area "Zdrada i fabuła".

## Finding

- The original's timers are `0x380..0x396` (`FUN_0053fb00`,
  `ghidra/notes/timer-scheduler.md`).
- The port's story events used several of those numbers: `0x386`,
  `0x387`, `0x390..0x396` and `0x39b..0x3a0`. The events have since been
  replaced by the original's rules:
  - the Dagobah and Final Battle chains;
  - Jabba's palace;
  - the Emperor's arrival;
  - the Force events (2026-10-19).
- Today `story_events::define_story_events` defines no event, so no event
  id meets a timer id at run time.

## Change

- `events.rs` loses 21 `u32` constants that nothing reads:
  - the invented ids above;
  - ids of no current use: `0x12c`, `0x14d`, `0x14e`, `0x180`, `0x1e5`,
    `0x210`, `0x302`, `0x340`, `0x370`, `0x105`, `0x200`, `0x230`,
    `0x231`.
- The remaining 18 constants are the original's notification ids in use,
  `0x100..0x362`, all below the timers.
- `no_notification_id_takes_a_timer_s_id` guards the range.
- The provenance baseline drops the 21 removed items: 77 -> 56.
- No behaviour changes, so no campaign, golden or browser run is needed:
  - the constants had no reader;
  - `cargo test --workspace`: 2460 passed, 0 failed.

Hashes of the committed files are in `SHA256SUMS`.
