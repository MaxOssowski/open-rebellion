# The victory conditions word (galaxy `+0x74`)

Read 2026-10-18 for F-038 (story message rows 20 and 21) and F-005.

## The check (`FUN_005180e0`)

It reads these characters and objects:

- Luke `0x32000242`, Mon Mothma `0x30000240`, Vader `0x35000281`, the
  Emperor `0x34000280` (`FUN_00505190`);
- Coruscant `0x90000109` (`FUN_00505f60`);
- the Alliance headquarters (`FUN_00506110`).

With all of them found, it builds a word:

| Bit | Meaning |
|---|---|
| `0x1` | the Alliance headquarters is gone or destroyed (`+0x50` bit 3) |
| `0x10` | Luke captured (`+0xac` bit 0) |
| `0x20` | Mon Mothma captured |
| `0x100` | Coruscant's side is the Alliance's (`+0x24 & 0xc0 == 0x40`) |
| `0x200` | Coruscant destroyed (`+0x50` bit 3) |
| `0x1000` | Vader captured |
| `0x2000` | the Emperor captured |
| `0x100000` | the Empire wins |
| `0x1000000` | the Alliance wins |
| `0x10000000` | the game is over (either win) |

The wins depend on the game type, galaxy `+0x60`:

- 0, standard:
  - the Empire wins with Luke and Mon Mothma captured and the HQ gone;
  - the Alliance wins with Vader and the Emperor captured and Coruscant
    held or destroyed.
- 1, headquarters:
  - the Empire wins with the HQ gone;
  - the Alliance wins with Coruscant held or destroyed.
- Any other value builds no word.

When both sides would win at once, the win already recorded in the old
word stands: `0x1000000` keeps the Alliance's, `0x100000` the Empire's,
and the Alliance takes it when neither is recorded.

The function stores the word in galaxy `+0x74` and marks it changed
(`FUN_004fd490`).

## Its notification (`0x128`)

- `FUN_00518460` copies galaxy `+0x74` into both sides' VictoryConditions
  (`+0xbc`, `FUN_0052f600`). The two views sync it in `FUN_005322c0`.
- On a change, the side's slot `+0x260` (`FUN_00532f90`, side vtable
  `0x00660af8`) posts `0x128` with the old and new words
  (`FUN_0053fcf0`). Its name is `SideVictoryConditionsNotif`.
- `FUN_00518460` runs from `FUN_0051e050`, which also handles the end of
  a game (not read further).

## The messages

- Row 21, Alliance Headquarters Destroyed (`FUN_0048e5a0`): on `0x128`
  whose new word has bit `0x1`. Both sides read TEXTSTRA `0x7188` /
  `0x7189` "The Empire has destroyed the Alliance Headquarters.". It sets
  speaker `0x11` and `+0x5c` = `0x73` (hyp: a film).
- Row 20, Coruscant Captured (`FUN_0048f790`): on any `0x128` while
  Coruscant's side is the Alliance's. Both sides read `0x7170` / `0x7171`
  "The Alliance has seized the Imperial capital system of Coruscant.". It
  sets the speaker from the side and `+0x5c` = `0x74`.
- Neither builder has a once flag. Each change of the word posts the
  message again while its condition holds (hyp: the message log does
  not drop repeats).

## Against the port

- The port's `victory.rs` keeps its own win conditions (`VictoryState`:
  headquarters keys, Death Star, `resolved`). Before 2026-10-18 it had no
  word and no `0x128`; the game type is `VictoryConditions`.
- Ported 2026-10-18 (save v79):
  - `victory::conditions_word` (`FUN_005180e0`) and
    `advance_conditions_word`;
  - the word lives in `VictoryState::conditions_word`;
  - simulation step 14 and the app check it daily before the port's
    victory check, and record `victory_conditions` (old, new);
  - the app posts `story_messages::victory_conditions_messages` (rows 20
    and 21) on each change.
- port:
  - the headquarters is the Alliance HQ system, lost when it is
    destroyed or no longer holds the HQ;
  - the check runs daily.
- Not ported: deciding the game by the word's win bits. The port's
  `VictorySystem::check` still decides it (F-005).
