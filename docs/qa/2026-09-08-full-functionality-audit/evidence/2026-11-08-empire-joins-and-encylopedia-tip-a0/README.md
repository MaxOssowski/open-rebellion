# The Empire's Joins Enemy, and Display's "Encylopedia" tip, against A0

Date: 2026-11-08. Closes CMD-11-C012. Adds to EVT-01-C019.

## A0

A new Empire game in the Windows 10 VM (`.artifacts/a0-vm/empire-game3/`):

| File | SHA-256 |
|---|---|
| `joins-enemy.png` | `2abfb52331c0ee69acf9102545946d02901414beeece49a4753b3559745c4866` |
| `index-joins-chosen.png` | `91c4c2169df6424ef04f551e599f2cee4a36d587b8aefb1fa6a241dd3dd3c74e` |
| `index-research-chosen.png` | `90408a1d85fc0fe4d53e5442213af729ca4903afd350e58022a71ab3fef670f7` |
| `display-encyclopedia-tooltip.png` | `659a262ba2477013d34eddf75bce704e8a5b872378374377513a1e7b344bd045` (also `tooltip/531-mi-display-encyclopedia.png`) |
| `display-tooltip-kind3.png` | `0d68fb172fa8c5ea85480d979081662d3d6a6bd25fbec9115c9d7d6c1a3dfff2` |
| `ozzel-captured.png` | `f2654f58074f06896844aa53951ce5ea07f4f011f5342a743e1221813c288535` (kept for later) |

- **Commenor Joins Enemy.** The Empire's reader sees "Popular dissent on
  Commenor has caused that world to join the Alliance." This is
  `FUN_00499760`, `0x7039` / `0x703c`, over `0x3ed`. The Alliance's reader
  of the same event reads "join the Empire".
- **The Display tips.** With "Advanced Training Facility Research Complete"
  chosen (kind 4), Display's tip reads "Encylopedia" (`0x8031`, spelled as
  shipped). With a kind-3 row chosen ("Corfai Joins Enemy"), it reads
  "Display" (`0x8030`).

## Browser (Linux)

**`message-display.mjs` (Empire).** The fixture files Commenor joining the
Alliance for the Empire reader (`systems_joining_the_alliance`, through the
app's `system_joins_message`). The Alliance reader keeps its Corellia and
Commenor joining the Empire. The gate checks the exact text, and the
picture, title image and rail are each 0 pixels from A0.

**`tooltip.mjs --a0=.artifacts/a0-vm/tooltip` (Empire).** The message
arrows step to the fixture's Research Complete, and the pointer rests on
Display at A0's pointer (575, 289). The tip's box matches A0 `531`. The
probe used to stop at "Display" (`409`); the probe for the kind-4 variant is
new.
