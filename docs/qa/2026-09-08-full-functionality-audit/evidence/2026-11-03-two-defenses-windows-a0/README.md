# Two Defenses windows at once, one minimized (2026-11-03)

Surface ledger CMD-04 C007, "multiple system windows".

## A0

The A0 run used the Alliance's DS_flip_allian save, day 229. The captures are in `.artifacts/a0-vm/personnel-states/`, with hashes in `a0-sha256.txt`.

- **`25-mw-c-two-defenses`.** Pzob's and Mon Calamari's Defenses windows are open side by side. Both were opened from the Troop Finder.
- **`25-mw-d-two-defenses`.** Pzob's minimize box sends only Pzob's window to the rail; its rail slot reads "Pzob". Mon Calamari's window stays open.

## The port

The new `two-defenses` case of `sector-quadrants.mjs` (Linux, both sides):

1. opens the primary system's Defenses window from its sector icon;
2. moves that window right by its title strip;
3. opens the second system's Defenses window;
4. asserts that both are open;
5. minimizes the second.

It then asserts that the second system is on the rail and no longer open, and that the primary's window stays open and off the rail.

Run `sector-quadrants-2026-11-03T07-29-14-001Z-2369463`: pass. `one-railed-screen.png` shows F'tral on the rail and Akrit'tar open, as A0 shows Pzob and Mon Calamari.

## Gate note

A double click right after the drag's release did not open the second window. The case now moves the pointer away and waits 500 ms first. That keeps the drag's release from pairing with the first click.

## A Fleet window beside a Defenses window (CMD-03 C013)

**A0** (day 229): `26-mm1-fleet-and-defenses`, `26-mm2-fleet-and-defenses`.

- The Fleet Finder opened Mon Calamari's Fleet window over its open Defenses window.
- Moved by its title strip, the Fleet window sits beside the Defenses window, and both stay open: Fleet 4, Fleet 9, Quick and Haste on one side; Trooper Regiments on the other.

**The port.** `fleet-window.mjs` covers the same states on Linux, in every full batch (most recently `b39`):

- its `load` cases keep the primary system's Defenses and Fleet windows open together and drag a regiment from one to the other;
- its `move` case moves the Fleet window by its title strip.

`sector-quadrants.mjs` `two-defenses` adds two Defenses windows at once.
