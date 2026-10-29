# The Fleet window against the original: a structural comparison (P09)

Date: 2026-10-29. Tracker item "Panel i menu floty, rodzina okien
statusu", the comparison of the Fleet window with A0. No code changed.

## A0 (original game, Windows 10 VM, Empire, days 168 and 169)

The captures stay outside Git; their hashes are in `a0-captures.sha256`.
1. In the Fleet Finder, a click on Fleet 1 turns its row white and copies
   its name into the box. This is the port's behaviour since
   `../2026-10-29-fleet-finder-text/`.
2. Display (`0xc9`) opens the sector window (Sumitra, the right column)
   and Yavin's Fleet window.
3. A double click on the fleet icon expands the fleet in the left list and
   shows the fleet on the right:
   - its name in the side's colour;
   - the four tabs;
   - the ship list.

## The port

`tools/interface-parity/encyclopedia-placement.mjs --window=finders` opens
F3, chooses the first row and presses Display (`…T12-39-04-673Z`,
`empire-fleet-window-port.png`). The window shows the same structure:
- the title strip in the side's colour, named after the system;
- the fleets on the left;
- the fleet name, four tabs and the ship list on the right.

Placement: in A0 the Fleet window lies at x 365 (the best background
match). That is the galaxy view's right edge, 600, less the window's 235:
the port's clamping rule (`clamp_window_to_galaxy`). The sector window's
column follows the sector's half of the galaxy: Sumitra in A0 is in the
right half, and the port's Coruscant sector is in the left.

## Not settled

A pixel comparison needs the same game state in both. The games differ:
the A0 window shows an enemy Corellian Corvette at Yavin, the port an own
Imperial Star Destroyer at Coruscant. The Fleet window gate
(`tools/interface-parity/fleet-window.mjs`, 48 fixture cases) keeps
checking the window's bitmaps.
