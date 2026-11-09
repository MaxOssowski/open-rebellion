# The Message Index's frame lies over its panes

Date: 2026-11-09. Adds to CMD-08-C003 ("index") and CMD-08-C004
("summary").

## A0

`.artifacts/a0-vm/empire-game3/index-joins-chosen.png`: the Empire's
Message Index, All Messages, Commenor Joins Enemy chosen, the window at
(125, 52). The gate had not compared it before.

## The defect

The window's base (STRATEGY 10335 for the Alliance, 10336 for the Empire)
is keyed on its blue (palette index 252). Its rounded top corners have
white arcs that reach into the list pane's rectangle (10822 at (12, 13),
400 by 306).

The port painted the base first and the pane over it, so the pane's flat
grey (palette index 64, RGB 59, 59, 59) covered the arcs: window x 12..17
and 405..411, y 13..20, differed from A0.

## The evidence for the order

For every base pixel that is not blue and lies inside a pane's rectangle,
the A0 colour was checked against the base's colour and the pane's
colour (palettes from the DLL bitmaps):

| A0 capture | Pane | Base's colour | Pane's colour | Both agree |
| --- | --- | --- | --- | --- |
| `empire-game3/index-joins-chosen` | 10822 | 86 | 0 | 1 |
| `message-index/category-1-day228` (Alliance) | 10822 | 498 | 0 | 2 |
| `empire-game3/joins-enemy` (mode 2) | 0x2ab5 title strip | 27 | 0 | 2 |
| `empire-game3/joins-enemy` (mode 2) | 0x2ab4 text area | 45 | 0 | 0 |

So the panes lie under the frame in both modes and for both sides. Not
first-pixel keying of the pane: the pane's index-64 pixels elsewhere show
A0's grey, not the base.

## The change

`shell_layers` (`message_index.rs`) now lists the panes first, then the
base, then the rail. The unit test `the_frames_corner_arcs_lie_over_the_panes`
checks the order for both sides and both modes, and the layer list in
`mode_two_shows_the_message_pane_without_the_list_or_its_controls` now
expects it. Both fail without the change.

## Browser

`message-display.mjs` (Empire, Linux) opens All Messages after Commenor
Joins Enemy and compares the window with A0 `index-joins-chosen`:

- the left corner (x 0..29, y 0..21) and the right corner (x 380..413,
  y 0..39): 0 pixels differ (before the change, the arcs' pixels did);
- the category tabs (x 0..413, y 44..87): 0 pixels differ;
- the rail (x 414..467, y 11..295): 0 pixels differ.

Not compared: the rows (the fixture's history is not A0's game), and the
white titles "Message Index" and "All Messages", whose anti-aliasing is
the font deviation the owner accepted on 2026-10-30.

`message-index-shell.mjs` composed its expected shell in the port's old
order (base, rail, pane), so it checked the port against itself. It now
lays the pane first, then the base keyed on its blue, then the rail.

## Other windows on the same base

The Fleet, System, Personnel and Troop Finders already paint their panel
at (12, 13) before the base 10335 / 10336 (`fleet_finder.rs`,
`system_finder.rs`, `personnel_finder.rs`, `troop_finder.rs`). The
Encyclopedia and the Assault Summary paint the frame first; A0 shows
their arcs too (next batch).

## Batch b149

23 of 23 interface gates on Linux; workspace tests 3493 passed, fixture
tests 139; the diff's 163 mutants caught. Tactical matrix
`2026-11-09T04-01-18-496Z-1069758`: 148 of 148.
