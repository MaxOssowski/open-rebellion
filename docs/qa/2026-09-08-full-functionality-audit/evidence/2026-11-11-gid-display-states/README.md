# GID system states against A0 (2026-11-11)

Surface ledger CMD-02: C018 (known), C020 (uninhabited), C022 (blockade),
C023 (mission) and C024 (fleet). Also notes on C002, C003 and C017.

## How the marks were found in A0

`FUN_00425d00` puts each system's 17 by 17 cell at a fixed corner:
x = (SYSTEMSD x × 607 >> 10) + 21 or 84, y = (y × 437 >> 10) + 25 or 27
(Alliance cockpit or Empire cockpit). `FUN_0042cde0` blits the mark keyed
from that corner. See `../2026-10-30-gid-system-positions/`.

`findmarks-corners.mjs` (this folder) checks every system's corner in a
capture for every STRATEGY mark 10130..10181, pixel for pixel. It was run
over all 2,723 PNGs in `.artifacts/a0-vm/`. The 5-pixel marks (10149,
10150, 10154) also match inside the larger mark of the same colour, so the
largest match at a corner is the mark.

The art, read from the BMPs:
- 10146..10149: the Alliance's marks, red, largest first.
- 10150..10153: the Empire's marks, green, smallest first.
- 10154..10157: the no-side marks, cyan, smallest first.
- 10158: unexplored. 10160: both sides' fleets (yellow).

No A0 capture shows 10160 at any system.

## A0 captures used

All in `.artifacts/a0-vm/` (Windows 10 VM, original `REBEXE.EXE`); the
hashes are in `a0-sha256.txt`.

| Capture | Side, day, display |
|---|---|
| `agent-menu/alliance-easy-day1.png` | Alliance, new Easy game, day 1, Popular Support |
| `agent-menu/alliance-expert-day1.png` | Alliance, new Expert game, day 1, Popular Support |
| `2026-10-28-nav/fleet-averam.png` | Alliance, "test" save, Popular Support |
| `gid/d-popular-support.png` | Empire, day 209, Popular Support |
| `gid/d-idle-fleets.png` | Empire, day 209, Idle Fleets |
| `gid/d-energy.png` | Empire, day 209, Available Energy |
| `gid/d-active-personnel.png` | Empire, day 209, Active Personnel (not compared; see C023) |
| `gid-displays/legend-alt-1.png`, `alt-1.png` | Empire, days 461 to 463, Popular Support (Alt+1) |
| `gid-displays/legend-alt-3.png`, `alt-3.png` | Empire, days 461 to 463, Idle Fleets (Alt+3) |

The `gid-selection/*` captures were not used: they come from the
DS_flip_allian save, whose sides are mixed.

## The gate

`run.mjs` now runs `verifyGidStateMarksAgainstA0` in the `known`,
`uninhabited`, `blockade`, `mission` and `fleet` scenarios, at 640 by 480,
for both sides. All five open on Popular Support.

1. It finds the mark at the primary system's corner in the port's frame
   and checks it is the mark the fixture reports (`primary_gid_marks`).
2. It checks which mark that is (`GID_STATE_EXPECTED`):
   - Popular Support: the player's support is 92% for the Alliance (its
     largest, 10146) and 8% for the Empire (its smallest, 10150), in the
     holder's colour (`FUN_0042b330` display `0x11`, bands 81/80/60/59/50
     through `FUN_00426e70`).
   - Idle Fleets (Alt+3, display `0x21`, bands 3/2/2/1/1): one fleet is the
     medium mark in its side's colour. Alliance 10148, Empire 10151. In
     the blockade, the fleet is the enemy's: 10151 for the Alliance player,
     10148 for the Empire player.
   - Active Personnel (Alt+6, display `0x44`, bands 1): the agent on a
     mission (`+0x78` bit 7) makes it the player's largest: 10146 or 10153.
3. It compares each of those marks, every non-key pixel, with the same mark
   where an A0 capture shows it at a system's exact corner
   (`GID_A0_MARKS`). Every comparison has 0 differing pixels.
4. After Alt+3 or Alt+6 it presses Alt+1 and checks the frame returns to
   the first Popular Support frame byte for byte. It also checks each Alt
   key changed the frame.

A0 references per mark:

| Mark | Alliance cockpit | Empire cockpit |
|---|---|---|
| 10146 | alliance-easy-day1, Corsin (245, 155) | d-energy, Commenor (257, 201) |
| 10148 | fleet-averam, Umgul (367, 272) | d-idle-fleets, Yavin (526, 236) |
| 10150 | alliance-easy-day1, Coruscant (226, 150) | d-idle-fleets, Coruscant (289, 152) |
| 10151 | alliance-easy-day1, Vagran (191, 228) | d-idle-fleets, Averam (277, 148) |
| 10153 | — | d-popular-support, Coruscant (289, 152) |

The table in `run.mjs` lists a few more marks for later cases.

### The blockade

A0 has a blockaded system after all. In the Empire game of
`gid-displays/` (days 461 to 463, 2026-10-31), on one game day, 3 seconds
apart:
- `gid-displays/legend-alt-1.png` (Popular Support, Alt+1) shows Yavin
  with 10149, the Alliance's smallest mark. So the Alliance holds it.
- `gid-displays/legend-alt-3.png` (Idle Fleets, Alt+3) shows Yavin with
  10151, the Empire's medium mark, not the both-sides 10160. So one Empire
  fleet orbits it and no Alliance fleet does.

The earlier pair `alt-1.png` / `alt-3.png` (two minutes before) shows the
same marks at Yavin.

`FUN_0050b8e0` sets the blockade bit (`+0x88` 0x20) when there is no battle
(not both sides present) and a fleet of another side than the holder's is
in the system (`ghidra/notes/blockade-bit.md`). Yavin is blockaded.

On day 209 (`gid/d-popular-support.png`), Yavin held its own Alliance fleet
(`d-idle-fleets`: 10148), so it was not blockaded. Yavin's cell plus a
2-pixel margin (21 by 21) is identical in the two Popular Support captures:
0 of 441 pixels differ. The blockade draws nothing on the GID, as
`FUN_0042b330` and `FUN_0042cde0` never read bit 0x20.

The gate asserts that A0 fact, and in the port:
- the blockaded primary system draws one bitmap (as before);
- under `--all`, its 21 by 21 cell equals the `known` case's cell, byte
  for byte;
- on Idle Fleets it shows the enemy fleet's medium mark, matching A0.

## Port changes

None. Every state already matched A0. No Rust code changed, so there are
no new unit tests or mutants.

## Runs (Linux, Arch Chromium, test WASM built from this tree)

- Full catalog (`run.mjs --all --no-build`):
  `.artifacts/interface-parity/2026-11-11T16-42-37-223Z-1403128`, 152 of
  152 pass. Its blockade cases report the A0 Yavin cell 0 of 441 pixels
  different and the port's cell equal to the known case's (both sides).
- Scenario run `blockade` alone: `2026-11-11T16-42-09-090Z-1402028`, 4 of
  4 (the comparison with the known case is skipped when it runs alone).

## Cells

- **C018 known: A0 + browser.** Explored systems show their display's
  mark at the exact corner in every A0 display. The port's explored
  primary system shows 10146 / 10150, equal to A0's marks pixel for pixel.
- **C022 blockade: A0 + browser.** A0's blockaded Yavin shows only its
  display's mark; the port's blockaded system does the same and its cell
  equals the unblockaded one.
- **C024 fleet: single-side marks A0 + browser.** Idle Fleets marks for
  each side match A0. Still open: the both-sides mark 10160. No A0 capture
  shows it.
- **C023 mission: art only.** The Active Personnel mark matches A0's art,
  but no A0 capture shows a system with a mission under way on Active
  Personnel. `d-active-personnel` (Empire, day 209) shows only smallest
  marks.
- **C020 uninhabited: art only.** No A0 capture shows a system known to be
  uninhabited. The sector window marks one by its missing support track;
  none of the A0 sector windows shows that.
- **C017 matching legend.** No new evidence. The legend window still has
  no A0 capture.
- **C002 sector, C003 system.** No new evidence here.

## A0 captures still needed

1. **Both-sides fleets (C024).** Any game. A system where both sides have a
   fleet in orbit at the same time (for example during a battle alert,
   before resolving it). Choose Idle Fleets (Alt+3). Capture the galaxy
   with no window over that system, and note the system's name. Expected:
   10160 at its corner.
2. **Mission (C023).** Any game. Send a character on a mission to an
   explored system that holds no other character of yours, and keep it
   visible on the galaxy. While the mission runs, capture Active Personnel
   (Alt+6) and Popular Support (Alt+1). Note the system and the character.
   Expected: the player's largest mark (10146 or 10153) on Active
   Personnel.
3. **Uninhabited (C020).** Any game. Open a sector window that shows a
   system without a support track (uninhabited, `FUN_0045baf0`) and
   capture it. Then close the window and capture the galaxy on Popular
   Support. Note the system's name and whether it is explored.
4. **Legend (C017).** Any side. Open the legend window from the legend
   button with a real mouse click in the VM window (a QMP click opened
   nothing visible), for Popular Support and for one other display.
