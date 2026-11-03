# The System window's close box against A0 (2026-11-03)

Surface ledger CMD-04 C006, "closed".

## A0

The A0 run used the Alliance's DS_flip_allian save, day 228.

- **`31-mon-calamari-system-open`.** Mon Calamari's System window (Manufacturing) is open at (305, 86) beside the Sluis sector window.
- **`31-mon-calamari-system-closed`.** Its close box at the window's (209, 3) closes it. No rail slot takes it: the rail shows only Pzob, minimized earlier.

These join the earlier A0 captures of the sector window (`sector-windows/05`) and the Fleet window (`system-windows/11`), each closed by its close box.

## The port

`sector-quadrants.mjs` `open-system` (Linux, both sides) now does, after minimizing the window to the rail:

1. double-clicks the System icon, which brings the window back;
2. clicks its close box at (209, 3);
3. asserts the window is neither open nor on the rail.

Run `sector-quadrants-2026-11-03T18-37-49-335Z-3320068`: pass.
