# The Troop Finder's name search, against A0 (2026-11-02)

Surface ledger OBJ-04 C010, "Troop Finder system-name search".

## A0

The A0 run used the Windows 10 VM and the Alliance's DS_flip_allian save, day 179. The captures are in `.artifacts/a0-vm/finder-search/`, with hashes in `a0-sha256.txt`.

- **`01-open`.** F4 opens the Troop Finder on the Alliance's rows: Duros, Fleet 7, Mon Calamari, Pzob, Xyquine.
- **`02-typed-P`.** Typing "P" fills the Troop Location box and chooses Pzob. The row turns white, and the others are grey (`FUN_00609650`'s prefix match).
- **`03-typed-Pz`.** "Pz" keeps Pzob.

## The port

`encyclopedia-placement.mjs --window=finders` (Alliance) now types into the open Troop Finder. The fixture lists Bpfassh, Ghorman, Ithor and Yavin, so the gate types "I".

The gate measures each row's brightest name pixel: 119, 119, 254, 119. Only Ithor is white. The prefix match is `list_prefix`, with unit tests.

Run `encyclopedia-placement-2026-11-02T19-24-27-439Z` (Arch Chromium): pass.

## Reopened, the Finder forgets the choice (OBJ-04 C015)

**A0.** `f4-reopened-day171` (2026-10-29) shows a Troop Finder opened again after a row was chosen. The choice is gone, and the first row is chosen.

**The port.** The same gate presses Escape after "I" and opens the Finder again with F4. The rows' brightest pixels are 252, 119, 120, 119: the first row is chosen, and Ithor is grey again. Run `encyclopedia-placement-2026-11-02T19-26-09-697Z`: pass.
