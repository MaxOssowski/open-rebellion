# The event messages' clips: the agent holds, the second droid plays (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy". This follows the event
delivery (`../2026-10-29-advisor-events/`). Most event messages (C3POACT
`0x2a..0x44`, IMP22ACT `0x21..0x41`) are op-`0x14` records, which the clip
player had ended at once.

## The original

| Op | Class | Read |
|---|---|---|
| `0x14` | `Class_006587e8` | Its loader `FUN_00411b80` reads `[count, a frame per layer, a record]` from the record's DLL. It shows the frame and clears `DAT_006b12b0`. Then `FUN_0041d690` → `FUN_0042b290` queue the record and `0x2d37` from slot 9 on the `0xbeef` player, the second droid's. Its step `FUN_00411b60` returns `DAT_006b12b0`. |
| `0x15` | `Class_00658818` | `0x2d37` = `[0x15, 0x29a]`, and `0x29a` is the second droid's still (ALSPRITE 3331, EMSPRITE 3001). It shows that for a tick, like op `0x11` (`FUN_00412260`). Its end slot `FUN_00412080` sets `DAT_006b12b0 = 1`. |

For example, ALSPRITE 9512 = `[0x14, 4412]`, and 4412 = `[0, 2001, 10512]`:
C-3PO holds BMP 2001. 10512 = `[4, 5012]`, R2-D2's clip: WAVE 1512, 16
frames from 3331.

Neither class runs an action. Both vtables' slot `+0x14` is
`FUN_00403ef0`.

## The port (`advisor_clip.rs`, `advisor.rs`)

- The agent's `ClipPlayer` owns a second player for the `0xbeef`
  aperture.
- Op `0x14` shows its frame, queues the record and `0x2d37` on the second
  player, and holds until an op-`0x15` clip ends there.
- The second player's voices reach the app as usual. `voice_started`
  routes each length to the player whose clip started it.
- `draw_advisor` shows the second player's frame in the second aperture in
  place of its idle frames while it plays.
- hyp: the second aperture steps after the agent's in each period
  (`FUN_00441d70`'s order is not read).
- The clock-running interface fixtures now give the player's advisor its
  messages, without the opening briefing (`AdvisorSchedule::from_save`
  with an empty table). This is test-only (`interface-test-fixtures`).

## Browser (`tools/interface-parity/advisor-events.mjs`, new)

The gate opens the ProductionDelivery fixture (code 75) at Fast for each
side. It waits for:
1. a delivered event code;
2. a hold record's voice from the sprite DLL (the second droid's WAVE
   range).

Then it keeps a screenshot.

Run `…T15-08-58-342Z` passed for both sides. The test WASM hash is
`fb3797ac…`. Arch Chromium, not the pinned build.

| Side | Codes delivered first | Second droid's voice | `hold.png` sha256 |
|---|---|---|---|
| Alliance | 1, 1, 2, 2 (systems joining) | `alsprite-dll` WAVE 1506 | `798de102…` |
| Empire | 2, 1, 2, 1, 2, 1 | `emsprite-dll` WAVE 1806 | `62e2874c…` |

In each screenshot the agent holds its frame:
- the Alliance: C-3PO standing, with R2-D2 showing his clip's frame;
- the Empire: the agent droid, with the console droid showing its clip's
  frame.

This also shows the delivery of `../2026-10-29-advisor-events/` in the
browser.

The release WASM (`2f7d2ecf…`) passes `agent-objectives.mjs` at
`…T15-13-28-447Z`.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `a_hold_shows_its_frame_until_the_second_droid_has_played_and_shown_its_still`;
  - `every_event_messages_hold_ends_after_the_second_droids_clip`, which
    reads the staged sprite DLLs and is ignored by default; it passes with
    `--include-ignored` for all 22 Alliance and 20 Empire hold records.
- Changed: `an_unplayable_clip_ends_at_once_with_its_action` uses op
  `0x12`, since `0x14` now plays.
- `cargo mutants --in-place --in-diff`: 35 mutants, 7 missed.
  - 6 are in `draw_advisor` and its texture helper. That painting has no
    tests, before this change or after.
  - 1 is equivalent: `index * 2 + 2` against `index + 2 + 2` in a word's
    slice, which reads the same two bytes.
