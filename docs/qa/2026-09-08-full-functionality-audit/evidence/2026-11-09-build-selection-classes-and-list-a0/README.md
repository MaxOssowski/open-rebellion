# Build Selection's fighters, facilities and defenses, and where its list opens

Date: 2026-11-09. Covers:

- OBJ-08-C002 (fighter);
- OBJ-08-C005 (facility);
- OBJ-08-C006 (defense);
- the class list's scroll when it opens.

## A0

Windows 10 VM, original REBEXE. A new Alliance game (Standard), days
19..38. Kessel's Manufacturing window was opened by double-clicking its
facility icon in the Calaron sector window. Kessel holds one Orbital
Shipyard (ships band "1:1") and one Construction Yard (facilities band
"1:1"), the same yards as the fixture's system. Build was chosen from
each band's menu.

Captures are in `.artifacts/a0-vm/build-selection4/`, with their SHA-256 in
`SHA256SUMS`:

| File | Class | Refined | Maintenance | Completion | Deployment | Window |
| --- | --- | --- | --- | --- | --- | --- |
| `00-kessel-dreadnaught-day19` | Alliance Dreadnaught | 44 | 33 | 176 Days | 0 Days | (192, 84) |
| `02-kessel-x-wing-day34` | X-wing | 5 | 4 | 20 Days | 0 Days | (194, 96) |
| `03-kessel-y-wing` | Y-wing | 5 | 4 | 20 Days | 0 Days | (194, 96) |
| `04-kessel-construction-yard-day37` | Construction Yard | 10 | 10 | 40 Days | 0 Days | (192, 84) |
| `05-kessel-gencore-day38` | GenCore Level I | 12 | 12 | 48 Days | 0 Days | (192, 84) |
| `06-kessel-kdy150` | KDY-150 | 4 | 4 | 16 Days | 0 Days | (192, 84) |
| `07-kessel-lnr` | LNR Series I | 7 | 7 | 28 Days | 0 Days | (191, 84) |
| `08-list-opened-with-lnr-shown` | the open list, LNR Series I shown | | | | | (191, 84) |

The window moved a few pixels while the list was used over QMP. Each
window's place was measured from its title bar.

The Kessel Dreadnaught matches A0 `build-selection3/ships0` (Sluis Van,
another new game) in every value.

## Where the list opens

The port had a `hyp:`: "the list opens with the shown class at its top".
A0 shows otherwise. The list opens at its top and scrolls only as far as
the shown class needs to be seen, as the last of its two items:

- Alliance Dreadnaught shown (the first class): items 1 and 2.
- Alliance Escort Carrier shown (the second): items 1 and 2, Dreadnaught
  first.
- LNR Series I shown (the fourth facility): KDY-150 and LNR Series I
  (`08`).

`DropDown::scroll_to` (`selection_list.rs`) now returns
`item_height * (index - (visible - 1))`, and 0 below that. The unit test
`the_class_list_opens_scrolls_on_its_bar_and_a_click_chooses_a_class`
expects the A0 positions:

- the reopened list with the second class shown stays at 0;
- `scroll_to(3)` is 140.

It failed without the change: the reopened list was scrolled.

`FUN_0060bed0` (the drop-down's constructor) was read. It does not show
the scroll rule, so the rule rests on A0.

## The browser comparison

`build-selection.mjs` (Linux Chromium) needs to step to the next class.
The list now opens with the shown class last, so the gate's `stepClass`
does this:

1. It reads the shown class's index. The fixture report has a new field,
   `build_selection_index`.
2. Past the first class, it clicks the bar's down arrow seven times, 10
   pixels each.
3. It takes the second item.

For the Alliance, `classValuesCase` opens Build on the ships band, then
on the facilities band. For each class above it checks:

- the four values: exactly as A0, for all seven classes;
- the class picture in the shown item (window 6..200 by 22..47): 0 of
  5070 pixels differ from A0 at A0's window place, for all seven;
- the open list with LNR Series I shown: KDY-150's and LNR Series I's
  pictures and the list background are 0 differing from A0 `08`. One
  20 by 20 box is left out, A0's pointer at window (113..132,
  100..119).

The Empire's existing cases (the Imperial Army Regiment against A0
`build-selection2/01`, over maintenance) step with the same helper and
pass.

Run: `.artifacts/interface-parity/build-selection-2026-11-09T12-54-31-486Z-2219205`.

## Not covered

- **The names' colours in the open list.** The port paints the shown
  class grey and the others white (A0 `manufacturing/24`). The new
  captures do not fit that rule:
  - Dreadnaught shown: Dreadnaught white, Escort Carrier grey;
  - Escort Carrier shown: both white;
  - LNR shown: both white.

  Which item state picks the grey is not traced. The name rows are not
  compared.
- **Special forces and regiments (C004).** Kessel has no training
  facility.
- **The Empire's classes.**
- **Escort Carrier.** It showed 32 / 29 / "128 Days" / "0 Days", but that
  capture was overwritten, so it is not used.

## Batch b155

23 of 23 interface gates on Linux; workspace tests 3496 passed, fixture
tests 139.

The diff's mutants: two of `BuildSelection::selected_index` were missed
at first. The unit test now checks the index before and after a choice.
On rerun all are caught: 9 caught, 1 unviable.

Tactical matrix `2026-11-09T13-48-55-209Z-2335467`: 147 of 148. The one
failure was `production-trench-run-success` (Alliance, 640x480), which
missed the `201.webm` request. That case is known to flake. Rerun alone,
`2026-11-09T14-02-49-478Z-2366565`, it passes 4 of 4.
