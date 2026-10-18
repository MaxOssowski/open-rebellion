# The encounter messages: Detects Enemy, Confronts, Heritage (F-038)

Date: 2026-10-18.

The mission detection now runs Force detection and the confrontation
(`evidence/2026-10-18-encounter/`), but they posted no message. In the
original, one message class (vtable `0x65b408`) reports them. The reading
is in `ghidra/notes/story-message-classes.md`, rows 6 to 8.

## The original

- Notification `0x360` (`CharacterEncounterKeyNotif`) fires when an
  `Encounter` key is set (`FUN_0055ff60`) and when the confrontation
  clears it (`FUN_00560200`).
- The class's `FUN_0048d9a0` files the character by side
  (`FUN_004edc80`): the Alliance's at `+0x68`, the Empire's at `+0x6c`
  (disassembly `0x48da4b`..`0x48da88`). A character of neither side
  suppresses the message.
- `FUN_0048d7f0` sets bit 0 on `0x211` (`LukeKnowsHeritageNotif`). It then
  builds on notification 2, once a day, for each side's view:
  - **Luke Discovers His Heritage** (`FUN_0048dc50`): an Alliance Luke or
    Leia against an Empire Vader or Emperor, with bit 0 set. It shows only
    for Luke and Vader. Text: `0x71a0` / `0x71a1`, then Luke's line
    (captured `0x71a3`, injured `0x71a4`, or escaped `0x71a5` with Vader
    free) and Vader's (captured `0x71a6`, injured `0x71a7`), on background
    `0x422`.
  - **Character Confronts character** (`FUN_0048df20`): the same pairs
    without bit 0. Text: `0x71a8` / `0x71a9` naming both, then a line for
    each one captive (`+0xac` bit 0) or injured (`+0x94`), else `0x71ae`.
    The background is chosen by the pair, and for Luke by his `+0xb0` bit
    8: `0x41f`, `0x420`, `0x423` to `0x426`.
  - **Force user Detects Enemy** (`FUN_0048e1d0`): any other pair. Each
    side's own character on the side's background (`0x412` / `0x413`) with
    its portrait, `0x71b0` / `0x71b1`.
- Each text is appended to the body (`FUN_0060b840` writes through
  `FUN_005f3240`).
- Luke's `+0xb0` bit 8 is `DagobahCompleted` (`dagobah.md`). This
  replaces the note's earlier guess that it is his Jedi appearance.

## Port

- `story_messages::EncounterNotes`:
  - `gather` files the characters of a mission advance's
    `EncounterBegan` and `EncounterEnded` effects and notes
    `HeritageLearned`;
  - `messages` builds the day's messages as above.
- The app posts them after the mission advance.
- The story-message fixture posts three days: an Alliance minor character
  met by Vader, Luke and Vader, and Luke and Vader with the heritage.
- port:
  - the class builds on notification 2 (once a day); the port builds once
    per mission advance;
  - the Final Battle bit (`0x220`) is not gathered, because the port's
    own `0x220` event is not the original's notification;
  - `0x71a2` (Luke taken to the Emperor) needs the Final Battle pickup
    mission, which the port does not have;
  - the media ids `+0x30` and `+0x32` are not read.

## Tests

`story_messages.rs`, 8 new tests:
- the heritage message for Luke and Vader, shown to both sides;
- its capture and injury lines;
- the confrontation text with and without injuries, in either member
  order;
- the confrontation backgrounds for each pair and for Luke's Dagobah;
- Detects Enemy for each side, with its portrait;
- only Luke and Vader get the heritage message;
- a character of neither side, or only one side, shows nothing;
- an encounter that only begins, or only ends, still posts.

The builders are new code, so every test fails without the change. The
first mutation runs found two gaps: the `notified` field was redundant, and
each of the two filing effects needed a test of its own.

## Browser

The interface fixture build was viewed with `agent-browser` (640 by 480).
Scenario 55 is code 312 for the Alliance and 568 for the Empire.

| Side | Message | Background | Portrait |
|---|---|---|---|
| Alliance | "Ackbar Detects Enemy": "Ackbar has detected Darth Vader because of a disturbance in the Force." | `0x412` | Ackbar (6720) |
| Empire | "Darth Vader Detects Enemy": "Darth Vader has detected Ackbar …" | `0x413` | Vader (6273) |
| Alliance | "Luke Skywalker Confronts Darth Vader": "… Both combatants escaped uninjured." | `0x423`, Luke and Vader duelling | — |
| Alliance | "Luke Discovers His Heritage": "Luke has learned that Vader is his father.  Luke escaped capture after the battle." | `0x422`, Luke | — |

On the Empire list, a first click landed on the list header's icon. It
removed that fixture's Detects Enemy message from the list, so the page
was reloaded and the message opened again.

The screenshots show original artwork, so they are not committed
(`target/acceptance/encounter-messages/`):

```
41b17e219d8e6dbe10d2917468aac5ceff5666251d91a9c7d4eafd5190e3c022  alliance-detects-enemy.png
97b76a44d9021cb0886f221edc823d7e2d22c53d035fa92f2ffb080ffde5b676  alliance-list.png
3c20471f72b623c4dc15ba19a5eefa74045afc1c93cc9f87e6a0eda103ba1827  confronts.png
86b69487b70a01a1912daff687492143ac4073e5a025bb37a3cb3f816cefa5c4  empire-detects-enemy.png
01ced99c34e2190109b795b0fbf972e9aae816da1b808d357c89ca7880d2b983  heritage.png
```

- The Fleet window gate passes all 48 cases (interface WASM
  `3c304a525191340b5e9e278493e90d4b93f03ee799bc2106edb5004bd77b2222`).
- The release build ran with `agent-browser`: a new Empire campaign, 60 s
  at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 24.2 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.0 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- The performance screenshots (`target/claude-tmp/run-msg/`, not
  committed):

```
f5c0796cf13ff14b29beb7ae520b651968d6681d6e4b0de51cf00f03a38b21b2  1-veryslow.png
e62e11940ac81ef7dce2f2a3209589b6aa9d3ff5c46a51731d71697c85969b0e  2-fast.png
```

- Native GUI acceptance was not run for this change.

## Gates

- `cargo test --workspace`: 2365 passed, 0 failed.
- The simulation is unchanged:
  - no save change;
  - the replay equivalence check passes on the same golden,
    `v1:ad8518eb172a430f` -> `v1:e811b12322f7e20e`, with 0 browser
    errors;
  - release WASM SHA-256
    `d9cb436bddc3796e57897992da54317941d01a7db995cc3834c0981c036c84e5`.
- `cargo mutants --in-diff` on `story_messages.rs`: 29 mutants, 24 caught,
  5 unviable.
- Clippy (pedantic) and rustfmt (edition 2021) on the changed lines: the
  tests use `assert!(…is_empty())` because `GameMessage` has no
  `PartialEq`; nothing else.

Hashes of the committed files are in `SHA256SUMS`.
