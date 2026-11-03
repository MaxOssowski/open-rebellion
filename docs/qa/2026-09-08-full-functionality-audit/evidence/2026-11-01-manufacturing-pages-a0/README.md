# Manufacturing window pages against A0 (2026-11-01)

A0: the Windows 10 VM, the Empire save (slot 4) on day 156, Coruscant's
System window (title bar at (205, 93), window (203, 91)): 0 shipyards,
1 training facility, 1 construction yard, 2 refineries, 5 mines on 10
deposits. Captures `.artifacts/a0-vm/manufacturing/00-overview` and
`01..05-tab` (Shipyards, Training Facilities, Construction Yards,
Refineries, Mines); hashes in `a0-sha256.txt`.

## What A0 shows

Each page lists its facilities left to right, three to a row, in 69 by
40 cells from the window's (8, 77); every facility's 67 by 35 picture
sits at its cell's (1, 2), matched pixel for pixel:

| Page | Picture | At (window) |
|---|---|---|
| Training Facilities | 9014 | (9, 79) |
| Construction Yards | 9022 | (9, 79) |
| Refineries | 9030 | (9, 79), (78, 79) |
| Mines | 9001 | (9, 79), (78, 79), (147, 79), (9, 119), (78, 119) |
| Mines, empty deposits | 9005 | (146, 115), (8, 155), (77, 155), (146, 155), (8, 195) |

The empty deposits sit at their cell's (0, -2), not (1, 2): `FUN_004568a0`
loads 9005 straight as the item (`FUN_005fbd20(.., 0x232d, 10)`) where a
facility gets a 69 by 40 canvas with its picture centred. The port drew
deposits as facilities; `DEPOSIT_OFFSET` now places them as A0 does
(port: the list's placement of a smaller item is not traced).

## Port

The port's pages use the same grid (`cell_rect`, `PICTURE_OFFSET`,
`facility_picture`: 9006 shipyard, 9014 training, 9022 construction,
9030 refinery, 9001 mine). `build-selection.mjs` now opens the
fixture's Shipyards, Construction Yards and Training Facilities pages
(window at (225, 76)) and finds 9006, 9022 and 9014 at (9, 79), every
opaque pixel exact (2,344, 879 and 690 checked) on Linux Chromium 155
for both sides. The mines and refineries pages follow the same code;
`a_facility_page_shows_its_label_pictures_and_selected_frame` checks a
mine at (9, 79) and a deposit at (77, 75).

The overview's band texts land on A0's rows ("No Ships are being built"
and the others at the band's +60, +140, +220).

Run `build-selection-2026-11-01T16-24-59-224Z-1449978`:

```
d30c192461b4d9a8f09388bfed0ff0073bca8f68ef52d0f57ef792190b521395  alliance/construction-yards-page.png
8ff2d1d805b068b2e95c050745094fd117a128e76cf8255943f711dd5c0edaec  alliance/shipyards-page.png
9693eaefacd732e7982932a127ab37c822caecc86d5f14f25055e70da9b043fa  alliance/training-page.png
cd2ba3d6c85e99533d0c584f332ca00685abdd5be9696e16182ac181e1da11b0  empire/construction-yards-page.png
8ff8519271c3f620e7353cad62937068d2625188b32a5cad2cc3f9c12a3b5260  empire/shipyards-page.png
66ee77bcdded143f2480b4cbc8b434281cb7fcf46604030e2d5a5b19036f245f  empire/training-page.png
```

## Open: Build Selection with the side over its maintenance

In the same save the Empire's maintenance reads -402 (the Death Star
retyped from a Star Destroyer, `2026-10-24-death-star-a0`). A0
`11`, `12`: Build on the troops band (1:1) and on the facilities band
(1:1) opens Build Selection with "n/a" in all four values and Confirm
grey. The port computes the times from the yards alone and enables
Confirm (`estimate`, "maintenance is shown, not charged"). Coruscant
shows no uprising or blockade mark, so the yards are not suspended by
`FUN_0050bbb0` (system `+0x88 & 0x24`); the refusal comes from the
order's validation (`ManuMgr_vt1bc` -> `FUN_004f9860`, a status other
than -1). Which check reads the side's maintenance
(`SideMaintState - MaintCapacityAllocated`, `FUN_0051b1f0`) is not yet
traced.

The Alliance save (slot 1, Vjun, day 9-15) shows "n/a" and a grey
Confirm too (`22`-`26`): from the facilities band and from the agent's
Build Facilities, for a Construction Yard (refined 10, maintenance 10)
and GenCore, with refined 12-20 and maintenance +239 on the top bar but
its first counter at 0. A0 `build-orders/15-t01` (Empire, day 193,
counters 58 / 191 / 189) shows the values. The refusal therefore
follows a side resource the port does not check; which one stays open.


## Build Selection's class list

A0 `24-list-open` (Vjun's Build Facilities): the list opens under the
box at (6, 85), 195 by 144 with its white frame at (6, 85)..(199, 227):
two 70-pixel items (`FUN_00437f80`: `FUN_0060bed0(.., 6, 0x16, 0xc3,
0x3f, 2, ..)`, `+0xa4 = 0x46`), STRATEGY 10598 tiled behind them and the
RCDATA 0x299a bar at the right. Each item's mini sits as in the box (19
under its top), its name 2 pixels lower (53 under it); the shown class's
name is gray (the item's `+0x60`, `FUN_00437880`) and the others white.
The port showed one class at a time and scrolled it with the wheel.

The mission dialog's and Build Selection's drop-downs now share
`selection_list` (the window, tile, frame, items, bar and click), with
each box's item height and visible count. hyp: the two lists colour the
shown item oppositely in A0 (white in `kinds/31`, gray here); which item
state picks the item's gray is not traced, so each follows its capture.
The old test that scrolled the list with the wheel is replaced on A0's
evidence (the same `CoolSelectionBoxClass`, wheel inert, `kinds/02`).

`build-selection.mjs` now captures the open list: against A0 the frame
and the bar match but the thumb's length (the fixture lists two
classes). Run `build-selection-2026-11-01T16-38-56-685Z-1457249` (Linux Chromium 155):

```
4a295995f6cd48f73a2583e321f49e4d7caafb95cc47e11970483f240f3d82d5  alliance/class-list.png
942f0078b607fea39c0d9d6d806afa02d713dc7c9abd9cb16d8918c051fcd74d  empire/class-list.png
```

## Correction (2026-11-03)

The deposit positions in the table above were measured from the title bar at (205, 93), not from the window at (203, 91). From the window, the deposits are at (146, 117), (8, 157), (77, 157), (146, 157) and (8, 197). That is the cell's corner, (0, 0); see `evidence/2026-11-03-facility-pictures-a0/`.
