# Killed by Imperial Assassins and Death Star Sabotaged (F-038)

Date: 2026-10-18.

This stage traced how story notifications reach the message classes
(`ghidra/notes/story-message-classes.md`, "Delivery"), and then ported
rows 18 and 19.

## The original

- The UI queue's posters build a record through the UI factory table
  (`FUN_0051f770`, `FUN_0051f730`, `FUN_005400f0`) and queue it
  (`FUN_004fd3b0`). `FUN_0053fe40` posts the codes `0x300`..`0x391` with
  two object keys.
- `FUN_004fc080`, the game objects' Destroyed notification, posts `0x302`
  (and `0x303` on arrival), then a code chosen by the DestroyedReason:
  - 6, sabotage: `0x305`;
  - 7, assassination: `0x306`;
  - `0x15`, autoscrap: `0x304`.

  Each comes with the object and its location.
- Row 18, `FUN_0048b050` (`0x306`): "{1:1} Killed" / "{1:1} was killed
  by Imperial Assassins at {3:1}.", with the victim's name and the system
  around its location. Background `0x42e`, Alliance only.
- Row 19, `FUN_0048d470` (`0x305`): the object must be of family
  `0x18`..`0x1b` (the Death Star) and its location must stand at a
  system. "Death Star Sabotaged" / "The Rebel Alliance has sabotaged the
  Death Star at {1:1}.", shown to both sides. It sets no background but
  writes `+0x5c` = `0x72` (hyp: a film).
- The previous stage read `0x305` and `0x306` only as the simulation's
  events (blockade, arrival). That was incomplete: the UI numbers are
  these notifications.
- Row 16 (`0x230`) is read too: SeatOfPower (`FUN_005617b0`) holds when
  the Emperor is free at an Empire-held Coruscant. Porting it needs a
  save format change, so it is left for a stage with that consent.

## Port

- `story_messages::assassins_message` and `death_star_sabotaged_message`.
- After each mission result, the app posts:
  - Killed by Imperial Assassins for an Assassination's
    `CharacterKilled`. Since the previous stages, only a minor character
    can die of the assassination's injury;
  - Death Star Sabotaged for `TargetSabotaged` of the Death Star.
- The app's `CharacterKilled` now marks the character killed
  (`missions::kill_character`), as the integrator does, instead of
  removing it from the arena. The message can then still name it.
- The story-message fixture posts both messages.
- 5 of the 21 classes remain unmapped.

## Tests

- `story_messages.rs`:
  `the_assassins_and_death_star_sabotage_messages_name_their_places`
  checks the texts, the audiences and the 0x42e background. The functions
  are new, so the test cannot pass without them.
- The app's wiring has no unit test. It is checked in the browser below.

## Browser

- The fixture (Alliance, code 312), checked with `agent-browser`:
  - "Chewbacca Killed" / "Chewbacca was killed by Imperial Assassins at
    Yavin.", background `0x42e` (1070);
  - "Death Star Sabotaged" / "The Rebel Alliance has sabotaged the Death
    Star at Endor.", with no picture.
- Screenshots not committed (original artwork):
  - `4e48397fe852b069b621cb63a002720b95e26dff743d5d6f18e355c645589e78
    target/acceptance/story-messages-2/assassins.png`;
  - `be7fdcb406a803e6a621e629d0bcd38052512e3de4f15f044545c907903bdb63
    target/acceptance/story-messages-2/death-star-sabotaged.png`.
- The Fleet window gate passes all 48 cases (interface WASM
  `91d02c0ed791fd4461dcbbe93190e8856bc8cbc60790e7f95a13821d22b18781`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.8 ms | 24.8 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 3.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.

## Gates

- `cargo test --workspace`: 2399 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:5474ae69134d1255` -> `v1:4cd75bad1e5062b8`) and 0 browser errors.
  Release WASM SHA-256
  `2955fbdfaa2ced7b877340a9c257127fdd672c6dd8474e7e982acd68e593b5e3`.
- `cargo mutants --in-diff` (`story_messages.rs`): 4 mutants, 2 caught,
  2 unviable.
- No simulation file changed, so `provenance-scan` was not rerun.

Hashes of the committed files are in `SHA256SUMS`.
