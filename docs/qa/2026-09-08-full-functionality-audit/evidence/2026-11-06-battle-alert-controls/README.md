# The Battle Alert's controls by side, and three battle results (2026-11-06)

## The controls' art was swapped

**Source.** `FUN_0044f860` builds the Battle Alert's three controls and picks their art by the viewer's side (`+0x118 +0x9c`). Side 1, the Alliance, gets:
- Simulate `0x29e2..0x29e4` (10722..10724);
- Take Command `0x29e5..0x29e7` (10725..10727);
- Retreat `0x2ade..0x2ae0` (10974..10976).

The Empire gets `0x29dc..`, `0x29df..` and `0x2adb..` (10716.., 10719.., 10971..).

**A0.** With the key colour skipped, each control matches its bitmap in 0 differing pixels:
- the Alliance's TAC-01-C010 (Wine) and the unedited "test" save's alerts (`.artifacts/a0-vm/battle-clean/00`, `10`, `20`) show 10974, 10722 and 10725 at (74/208/342, 345);
- the Empire's `battle-alert-empire/00-alert-tralus-day300` shows 10971, 10716 and 10719 at (137/271/405, 348).

In A0 the Alliance's controls are the red ones.

**Before.** `bmp_cache` had the two sets the other way round, so the port drew the Empire's green controls for the Alliance. `run.mjs` checked the port's screen against the port's own table, which had the same swap, so it never failed.

**Now.**
- The constants follow `FUN_0044f860`.
- `battle_alert_skins_match_recovered_faction_resources_and_geometry` asserts every control of both sides. It fails with the old constants.
- `run.mjs` (after the b107 batch) also checks each control's bitmap against the A0 capture of its side, at the port's own coordinates.

## Three battle results in the unedited Alliance game

All three results came from Simulate Results. The unit tests build the lines from the shipped templates and match A0 word for word.

| Day, battle | A0 | Summary lines | Display page | Test |
|---|---|---|---|---|
| 650, Yavin | `battle-clean/01`, `02` | "The Alliance fleet is defeated." / "Yavin is now under blockade by Imperial forces. The Alliance fleet has withdrawn to Umgul." | `0x7011` with the destination; both buttons | `the_alliances_withdrawal_from_its_own_system_reads_as_a0` |
| 672, Averam | `battle-clean/11`, `12` | "The Alliance fleet is victorious." / "Averam has been successfully defended from Imperial forces. The Imperial fleet has withdrawn." | `0x7012`; both buttons | `the_alliances_defence_of_its_own_system_reads_as_a0` |
| 718, Umgul | `battle-clean/23`, `24` | "The Alliance fleet is defeated." / "Umgul is now under blockade by Imperial forces. The Alliance fleet has been completely destroyed." | `0x7010` over `0x7014`; the system's button alone | `the_alliances_destroyed_fleet_reads_as_a0` |

At Umgul the Retreat control was disabled (10976). The Empire's Fleet 41 held one ship, the Gorgon, with three regiments (`22-umgul-imperial-forces`). `FUN_00516730` bars a retreat when an enemy ship projects a gravity well, or when none of the viewer's units can move or find a destination. The capture does not show which of these held, so it is not compared with the port's `retreat_offered`.

## Hashes (sha256)

| Capture | sha256 |
|---|---|
| `battle-clean/20-alert-umgul-day718-tactical-grey.png` | `17dc5427f46026e75f7d23724cdf1de63a91cb2d9c1dc1ed256b5528cc30bd5d` |
| `battle-clean/21-umgul-alliance-forces.png` | `29986db2b470a676b88431d94f152d90be9a11b308b818b5bd641f2fcb79dd05` |
| `battle-clean/22-umgul-imperial-forces.png` | `c7ac78e4d2b695b6eedb68a92ddf5000b1317a12d1297399c5c9e37884783a9f` |
| `battle-clean/23-summary-destroyed-umgul.png` | `2351048b38aeefe6a1dd2affdb08d89030f2a709785722c6929eaf7a637df6a3` |
| `battle-clean/24-display-destroyed-umgul.png` | `b736edf25ff3c8dde9bc64309f9dcf0419a310c29f95bf96e114d18f624a32bb` |

For the Yavin and Averam captures, see `../2026-11-06-troop-finder-and-joins/` and `../2026-11-06-saboteurs-strike/`.

## Runs

**b107.** All 21 gates passed. The batch's `run.mjs` covers the GID catalog only. The Battle Alert case is in the tactical catalog, so it ran on its own:

```
node run.mjs --battle --all --no-build --scenario=production-battle-alert-entry
```

- Run `2026-11-06T21-28-46-857Z-1886782`: 4 of 4 passed, both sides at 640×480 and in the letterbox.
- At 640×480 the probes `battle-alert-{retreat,simulate,command}-a0` are `source-bitmap-exact`. For the Alliance's Simulate, that is 10722 over 3617 pixels.
- In the letterbox they report `non-native-scale`, as every bitmap probe there does.

## The Empire's side in a new Easy Empire game (2026-11-07)

`.artifacts/a0-vm/empire-clean/` (day 253, the Alliance attacking Umgul):

| Capture | What it shows | sha256 |
|---|---|---|
| `10-alert-umgul-day253` | Retreat disabled 10973, Simulate 10716, Take Command 10719, each 0 differing pixels with the key skipped | `bae075ae9d27d8f72deeb1eec484a2344a13ddea4f25e0da0af8e48b22b20227` |
| `11-summary-destroyed-umgul` | "The Imperial fleet is defeated." / "Umgul is now under blockade by Alliance forces. The Imperial fleet has been completely destroyed." | `86f9f6516680c1029c6442f9cfac8adcb52d8bbf934889c1b3fe141dc585f963` |
| `12-display-destroyed-umgul` | "The Imperial fleet was defeated at Umgul. Your fleet has been destroyed." and the system's button alone; 10764 matches with its black keyed | `7cac5857da47478bb90135dfa6a1d2bca3c93af4c5393cf02eda7b7023cef540` |

`the_alliances_destroyed_fleet_reads_as_a0` now also builds the Empire's lines and matches them word for word.

## Two more Empire results (2026-11-07, morning)

| Day, battle | A0 | Summary lines | Display page |
|---|---|---|---|
| 689, Chandrila | `empire-clean/40..42`, sha256 `e84cadfb…`, `b66085df…`, `5eee12fa…` | "The Imperial fleet is defeated." / "Chandrila is now under blockade by Alliance forces. The Imperial fleet has withdrawn to Coruscant." | "The Imperial fleet was defeated at Chandrila. Your fleet has withdrawn to Coruscant." |
| 728, Coruscant | `empire-clean/50..52`, sha256 `7d67c444…`, `c4ccd61c…`, `a2a0f771…` | "The Imperial fleet is victorious." / "Coruscant has been successfully defended from Alliance forces. The Alliance fleet has withdrawn." | `0x7012`, "The Imperial fleet is victorious. The world Coruscant is now under Imperial blockade." |

The Chandrila alert had Retreat enabled. On the Chandrila Display page, the Empire's Fleet button 10766 matches A0 with its blue keyed: 0 of 16,055 pixels differ.

The unit tests `the_alliances_withdrawal_from_its_own_system_reads_as_a0` and `the_alliances_defence_of_its_own_system_reads_as_a0` now also build the Empire's lines, and both match A0.

## Mission kinds an Empire character is offered (2026-11-07)

`.artifacts/a0-vm/kinds-empire/` (hashes in `a0-sha256.txt`), Ozzel at Coruscant, day 717..728:
- **Released on the Empire's own Chandrila** (`00`..`02`): Diplomacy above Espionage.
- **Released on his own Coruscant** (`10`, `11`): Espionage alone. The click on the Facilities band targets the system.

Coruscant's support is full for the Empire. Diplomacy ends there with code `0xf` (`diplomacy_ends_with_code_0xf_once_its_side_holds_full_support`), and `available_kinds` drops a kind whose validator ends it. Recruitment is absent at both systems, which is consistent with an emptied pool by day 717. None of these targets offered a research kind.
