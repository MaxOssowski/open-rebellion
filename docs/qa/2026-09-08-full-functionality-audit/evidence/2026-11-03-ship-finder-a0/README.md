# The Ship Finder against A0 (2026-11-03)

Surface ledger OBJ-04 C002, "Ship Finder".

## A0

The A0 run used the Alliance's DS_flip_allian save, day 301. The captures are in `.artifacts/a0-vm/personnel-states/`, with hashes in `a0-sha256.txt`.

- **`27-ship-finder-all`.** F3 opens the Fleet Finder, and its Ship Finder button, at the window's (423, 147), turns it into the Ship Finder:
  - the title reads "Ship Finder" and the name label "Ship Name";
  - the tab is All Ships, with nothing chosen;
  - the rows are the side's known ships by name: Arrow, Death Star, Galleon 1, …
- **`28-ship-finder-alliance`.** The Alliance tab chooses its first row, Arrow; the others are grey.

## The port

`fleet-finder.mjs` `chrome` (Linux) switches to the Ship Finder and compares the window with the STRATEGY resources exactly (`ships-screen.png`).

Compared with A0 `27` pixel for pixel over the window (64..530, 52..380), leaving out the list, the tab buttons, the title, the name label and the name box: 34 of 64,938 pixels differ. All of them are at x 240..260, y 120..180, the edge of the selected tab. That differs because the port's capture has the Imperial tab chosen and A0's has All.

Run `fleet-finder-2026-11-03T10-31-41-794Z-2728643` (batch `b41`).

The tab's first-row choice and the typed choice are the Fleet Finder's own rules (OBJ-04 C004), which the Ship mode keeps (FUN_004632d0).
