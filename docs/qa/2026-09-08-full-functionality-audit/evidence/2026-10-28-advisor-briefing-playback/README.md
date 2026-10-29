# The opening briefing plays: clips, voice, map actions and skip (P34)

Date: 2026-10-28 and 2026-10-29. Tracker item "Droidy-doradcy", stages 2
and 3 of the briefing's playback; stage 1 is
`../2026-10-28-advisor-clip-staging/`. The reading is in
`ghidra/notes/advisor-briefing.md`.

## The original

- **What schedules it.** Each side's advisor constructor schedules the
  briefing for 40000 days:
  - Alliance: `FUN_004c27f0`, class `0x0065c4c8`, `0x58` messages,
    message `0x29` at `+0xa4`;
  - Empire: `FUN_004c0710`, class `0x0065c4a0`, `0x56` messages,
    message `0x4a` at `+0x128`.
- **What starts it.** The update `FUN_004c0a60` runs only while
  `DAT_006b28bc` is set. `FUN_00439ef0` sets it when the idle clip ends:
  op `0x12` (`Class_00658878`), 15 ticks of 67 ms (`0x43`), then
  `FUN_0041cea0`. The update:
  - walks the messages from 1;
  - zeroes a last day under today;
  - plays the first due message whose next free day is under today, and
    sets that day to today + `0x3c`;
  - plays one message per update;
  - with nothing due, calls `FUN_004c0d00` / `FUN_004c2dd0`.
- **Clips.**
  - A definition (op 4) adds one type-302 delta per tick to its first
    bitmap.
  - It ends once its voice is done (hyp: `FUN_00403fe0`).
  - A frame list (op `0x11`) holds 1 tick, then hands on its action.
- **Actions.** The Alliance has its own action table, `FUN_004c30c0`; the
  Empire's is `FUN_004c0fc0`. They differ in:
  - where Coruscant and Yavin are marked;
  - the Alliance headquarters `0x20000005`;
  - the leaders: Mon Mothma and Luke for the Alliance, the Emperor and
    Vader for the Empire;
  - `0x10` gives display `0x16` for the Alliance and `0x14` for the
    Empire.
- **Captions.** `FUN_00425d00` captions displays `0x13..0x16` by the
  viewing side.
- **Skip.** Code `0xc` installs the hooks (`FUN_0041d9d0`):
  - a left or right press, or the release of Escape, sets `0x2000`;
  - `FUN_004c2dd0` / `FUN_004c0d00` then schedule the short form
    `0x28` / `0x49` until today + 40000;
  - `FUN_0041da30` flushes the players once.

## The port

- `crates/rebellion-render/src/advisor_clip.rs`: the clip player.
  - It decodes definitions (`FUN_00404440`) and plays type-302 frames
    cumulatively.
  - It hands on voice and action events.
- `crates/rebellion-render/src/advisor_schedule.rs`:
  - the schedule above;
  - both action tables;
  - the input hold.
- `crates/rebellion-render/src/cockpit.rs`: `GidMode::caption` follows
  `FUN_00425d00`.
- `crates/rebellion-app/src/main.rs`:
  - on a new game, schedules the briefing and starts on Display Off;
  - plays the voices from the briefing DLL;
  - applies each action to the galaxy view;
  - holds the map's input until code `0xd`;
  - lets a click or Escape skip the briefing.
  - A loaded game has no briefing.
  - port: a major character's `DatId` lacks REBEXE's family byte
    (`0x30000240` is family `0x30`, id `0x240`), so a leader mark matches
    the major character by the low 24 bits.
- Browser pack:
  - the briefing DLLs' frames and records are `KIND_ADVISOR_FRAME`;
  - their voices are `KIND_AUDIO` `advisor/{dll}/{id}.wav`;
  - `C3POACT.SPT` and `IMP22ACT.SPT` are raw tables.
  - `runtime_pack.rs` now accepts 32768 entries; the pack holds about
    15000.

## Browser acceptance (`tools/interface-parity/briefing.mjs`)

Run `briefing-2026-10-29T08-42-25-973Z` passed all 4 cases. The WASM hash
is `f4a6df3b…dfad1a8`, in `browser-result.json`. The test serves `web/`,
runs headless Chromium at 640×480, and starts a new game from the main
menu.

| Case | Checked |
|---|---|
| alliance full / empire full | the 18 actions in message order; the 16 voices (Alliance 1155, 1220..1224, 1157..1161, 1225, 1226, 1163, 1164, 1168; Empire 1147, …, 1160), each from the side's briefing DLL with its length; every mark lands on a system |
| alliance skip / empire skip | a click during the first voice gives actions `0xc, 0xb, 0xd` and voices first + 1165 / 1157 |

The marks:

| Side | Mark | System |
|---|---|---|
| Alliance | Yavin (`0x12`), Luke (`0x13`) | `0x121` |
| Alliance | headquarters (`3`), Mon Mothma (`7`) | `0x7b` |
| Alliance | Coruscant (`4`, `0x14`) | `0x109` |
| Empire | Coruscant (`0x12`), the Emperor (`7`), Vader (`0x13`) | `0x109` |
| Empire | Yavin (`3`, `0x14`) | `0x121` |

None of the four cases logged a console error or an HTTP status ≥ 400.

### Pictures

- `caption-comparison.png`: SquakeNet frames on the left, the port on the
  right.
  - 0129: Alliance "Systems Loyal to the Empire";
  - 0139: Alliance "Systems Under Military Control";
  - 0722: Empire "Systems Loyal to the Alliance".
  - The captions match in text, colour and position.
  - The marked systems differ because the galaxy is seeded differently.
- `leader-marks.png`: the Emperor, Vader, Mon Mothma and Luke, each with
  their caption and system.
- `alliance-after-skip.png`: the view after the short form.

The first run (`…T08-17-03-294Z`) passed the same assertions, but its
leader screenshots showed captions without marks. That found the family
byte mismatch above. The gate has asserted every mark since then.

## Tests

- `cargo test --workspace`: all pass.
- Real-data ignored tests:
  - `every_briefing_clip_plays_each_of_its_frames_to_the_end` passes;
  - `each_sides_advisor_script_holds_its_briefing_and_short_form` passes.
  - Among the `seeds` ignored tests, only
    `fixed_fleet_tables_only_target_special_systems` fails, as recorded
    since 2026-10-16. The data diff only adds `load_advisor_script`.
- `scripts/test_build_runtime_pack.py`: 10 tests pass.
- New Rust tests (render):
  - the 11 `advisor_clip` tests and 12 `advisor_schedule` tests;
  - `type302_decoder_refuses_a_frame_of_another_height_than_its_anchor`;
  - `the_loyal_and_military_control_captions_follow_the_viewer`.
- New Rust test (app):
  `a_pack_of_the_briefings_size_parses_and_an_absurd_count_is_refused`.
- Without the change these tests do not compile: the modules and
  `GidMode::caption` are new.

### `cargo mutants --in-place --in-diff` (render and data)

- 214 mutants: 181 caught, 3 timeouts (killed), 13 unviable, 17 missed.
- The sources hashed the same afterwards.
- Three new tests closed the 6 missed in `AdvisorSchedule::update`,
  `play_due` and `skip`: the 23 mutants of those functions are now all
  caught.
- The remaining 11 are drawing or WASM-only, covered by the browser gate
  above:
  - `advisor_resource`: 4, the pack lookup on WASM;
  - `draw_advisor`: 5;
  - `draw_input_hold`: 1;
  - `draw_gid_caption`: 1, the faction passed to the caption, seen in
    `caption-comparison.png`.
