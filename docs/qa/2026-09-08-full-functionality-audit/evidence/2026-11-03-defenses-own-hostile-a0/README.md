# A Defenses window of the player's side and of the other side (2026-11-03)

Surface ledger OBJ-02 C012 (friendly) and C013 (hostile).

## A0

The A0 run used the Alliance player's DS_flip_allian save. The captures are in `.artifacts/a0-vm/personnel-states/`, with hashes in `a0-sha256.txt`.

- **`15-mon-calamari-regiments`.** Mon Calamari, the Alliance's, has its Defenses window at (305, 86). The title strip is red (side 1).
- **`18-umgul-imperial-garrison`.** Umgul, the Empire's, has its window at (305, 86), opened from the Troop Finder's Imperial tab. The title strip is green (side 2), and its regiment page lists the Imperial Army Regiment.

## The port

`sector-quadrants.mjs` (Linux, Alliance) opens both kinds of window:

- `defenses` opens the player's own system, Akrit'tar;
- `defenses-other-side` opens the other side's, F'tral.

Each window's title strip, 235 by 20, compared with A0's:

| Window | Sector icon (20 by 20) | Strip right of the name (55 by 20) | Minimize and close (40 by 20) | Frame sides |
|---|---|---|---|---|
| own, red | 3 of 400 differ | 0 of 1,100 | 2 of 800 | — |
| other side, green | 11 of 400 | 0 of 1,100 | 2 of 800 | 0 of 600 each |

The differing pixels are of two kinds:

- column 19, where the system name's first letter begins (Mon Calamari against Akrit'tar; Umgul against F'tral);
- the bottom-left pixel of each 14 by 14 title button, at the window's (3, 16), (204, 16) and (218, 16). That pixel is palette index 250 in STRATEGY 10209, 10253 and 10108. The port draws it as the bitmap's (0, 255, 0); A0 draws it black. Elsewhere index 250 is a real green that A0 shows (the Empire's title strips 10201 and 10802), so the corner is left as a known one-pixel deviation per button.

The tab art of both windows was already compared exactly by the same cases. Run `sector-quadrants-2026-11-03T10-25-30-042Z-2714862`.

## A note on the garrison line

`18` also shows "Garrison Requirement: 0" on Umgul's regiment page, an Empire system seen by the Alliance player. The Alliance's own Mon Calamari and Pzob show none (`15`, `17`). See `2026-11-02-defenses-pages-a0`: the line follows the Empire's systems in A0, not the player's side.
