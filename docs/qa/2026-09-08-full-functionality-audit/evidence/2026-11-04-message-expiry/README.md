# Messages expire 300 sub-ticks after posting (2026-11-04)

This covers surface ledger cell CMD-08-C015 ("clear"). The reading is in `ghidra/notes/message-expiry.md`.

## The original

- **The walk.** `FUN_0048a2a0` drops every message whose class slot `+0x30` says it has expired. It then rebuilds the unread mask.
- **When it runs.** After each filed message (`FUN_0048a060`), and when the Message Index closes (`FUN_0048a610(0)`). Nothing runs while the index is open.
- **The base rule.** `FUN_004c4fe0` expires a message once the scheduler's sub-tick passes its `+0x38` plus 300. `FUN_004c4ce0` stamps `+0x38` with `FUN_004fcee0`, the sub-tick, not the day. A day has 600, 60, 12 or 4 sub-ticks by speed, so a message lasts half a day at Very Slow, 5 days at Slow, 25 at Medium and 75 at Fast.
- **Never expire.** The agent's advice (`0x65b188`), Future Jedi Discovered and Leia Uses Force (`0x65b520`), the Assault Summary reports (`0x65b870`) and Chat (`0x65ba20`).
- **Facility Idle.** It lasts while its manager exists, is idle and is its side's (`FUN_0048c2b0`).
- **Manual.** p. 79: "Messages are eventually deleted whether or not you read them, except for agent advice messages."

## Before

The port kept at most 500 messages and dropped the oldest when a new one came, advice included. Nothing expired by age.

## Now

- **The log.** `MessageLog` has no cap.
- **Expiry.** `expire` follows `FUN_0048a2a0` (`GameMessage::expired`, `MessageLifetime`).
  - `push` stamps each message with the log's sub-tick (`set_now`, `GameClock::sub_tick`).
  - `expire_if_due` runs expiry after a filed message, and `set_index_open` holds it off while the index is open.
- **The app.** `main.rs` calls both every frame.
- **Lifetimes.**
  - Future Jedi Discovered is `Kept`.
  - Facility Idle is `WhileIdle`. It lasts while the system is the player's, the area's queue is empty and a facility of the area stands there.

## Tests

Unit tests (`message_log.rs`, `story_messages.rs`):
- `a_message_expires_300_sub_ticks_after_it_is_posted_unless_its_class_keeps_it`. A message posted at sub-tick 10 stays at 310 and goes at 311. The Advice rail, `Kept` and an Assault Summary report are kept.
- `a_facility_idle_message_lasts_while_its_manager_stands_idle`.
- `expiry_waits_for_a_posted_message_and_for_the_index_to_close`.
- `an_expired_message_can_rest_its_rail`. It replaces the cap's "dropping the oldest message can rest its rail", which tested the invented cap.
- `ids_survive_expiry_and_never_reuse`. It replaces `ids_survive_capacity_eviction_and_never_reuse`.
- `a_discovery_tells_the_trainers_side_what_it_found` now also checks that the message never expires.
- `a_filed_message_runs_the_expiry_that_an_idle_frame_does_not`. It was added for three mutants that survived the batch's first run.

Full batch b56:
- the workspace tests (3369) and fixture tests (116) pass, and provenance-scan is clean;
- all 21 Linux gates pass, `message-display`, `agent-objectives` and `tooltip` among them.

Mutants (`message_log.rs`):
- In-diff, 3 survived: `|=` in `push`, `expire_if_due` emptied, and `&&` in `set_index_open`. The new test closes them.
- On the rerun of `push`, `expire*`, `set_index_open`, `set_now` and `expired`, all 26 mutants were caught.

## Open

- **No browser check and no A0 timing.** The A0 VM's index held a single recent message, which fits the rule but times nothing.
