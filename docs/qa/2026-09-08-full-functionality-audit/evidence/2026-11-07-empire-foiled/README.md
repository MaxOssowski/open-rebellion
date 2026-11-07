# The Empire's Enemy Mission Foiled against A0 (2026-11-07)

This covers OBJ-12-C005 (failure: the other side's report) and CMD-08-C007 (the Mission rail) for the Empire.

## A0

A new Easy Empire game, started in the VM after the "test" game ended. The captures are in `.artifacts/a0-vm/empire-clean/`:

| Capture | What it shows | sha256 |
|---|---|---|
| `00-new-easy-advice` | the opening Advice list | `7546ace3588ad659a23f1b286c1296311ac02b31d18e63126d185bdbbc8776b0` |
| `01-day83-mission-list` | "Enemy Mission Foiled" on Mission Messages | `4ee5a0ab5e4c94b8de12e857db7daf1465e617b56351621243d617bc2eb73e02` |
| `02-day83-enemy-foiled-chandrila` | the message | `a5ecf40c5e42156906f04f2f4566624335bb43e0d4b51a6bcffb5a90a1e818ee` |

The message reads "An enemy Espionage mission to Chandrila has been foiled by our forces." over STRATEGY 1074 (`0x432`, the Empire reader's `FUN_00491500` picture). The picture is exact at (137, 85), in the window at (125, 52).

## The port

The fixture gave the Empire no foiled Alliance mission, so its side was never shown.

**Now.**
- `foiled_missions` also ends an Alliance Espionage at Chandrila (code 7), and the app posts it through `push_foiled_missions`.
- `message-display.mjs` (Linux, Empire) opens it. Compared with A0, 0 pixels differ in each of these: the text and background `0x432`, the picture, the title's item image, and the rail.

**Gate repairs.** The extra row pushed "Fleet Arrives" to the tenth place and the asking report down one, so two other steps broke. The scroll helper `shown` now serves the early Display step too, and it scrolls up twelve notches before choosing a row.

Screenshot: `.artifacts/interface-parity/message-display-2026-11-07T03-37-53-172Z/empire/enemy-mission-foiled-empire.png` (sha256 `4fb34141b7b819921d25e329de065d23927b44b0defcacff548641b96afbce0d`).
