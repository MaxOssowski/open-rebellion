# Battle results rows and the group bar's tips against A0

Date: 2026-11-08 (night). Cells:

- TAC-07-C007 (Troops page)
- TAC-07-C005 (rows)
- TAC-02-C007 (unassigned task force)
- TAC-02-C009 (hover)

## Result rows

A0 TAC-07-C007 (the Alliance's Battle at Mon Calamari): the destroyed
Alliance Army Regiment's picture sits at window row 146, 10 rows into its
70-high cell at 136. Its name's baseline is at screen row 255, and A0
TAC-07-C005's capital row has the same baseline.

The port centred the picture over a 14-row name strip,
`(70 - 14 - height) / 2`. That put it 7 rows too high, at 139.

`FUN_004724a0` centres the picture in the whole cell, `(cell - height) /
2`, and hangs the name's label from the picture's foot (`+0x34` = height +
top). The port now does the same. The name's text sits 3 rows below the
foot, which is where its baseline meets A0's row 255 (port: the font is
the accepted Liberation Sans deviation). The layout unit test's
expectations change with this evidence:
`the_forces_page_draws_the_originals_layout`.

The battle-results fixture now carries three Alliance Army Regiments on
the Alliance's three one-regiment hulls (CAPSHPSD 64, troop capacity 1).
The third hull dies, so one regiment is listed destroyed. `run.mjs`
'battle-results-destroyed-regiment-a0' compares that picture, 122 by 50,
with A0 TAC-07-C007's first destroyed row: 0 pixels differ (Alliance,
640x480).

## Tips

The Windows VM's battle at Coruscant (`.artifacts/a0-vm/coruscant-676/03`,
`04`) shows the real tooltips:

| Tip | Pointer | Box | Size |
|---|---|---|---|
| "Task Force 1" | (72, 10) | (72, 31) | 70 × 19 |
| "Unassigned Task Force" | (130, 10) | (130, 31) | 125 × 19 |

The Wine captures had styled them differently.

`tooltip.mjs` now hovers the port's first task force and an empty slot in
the production battle fixture. It checks each tip's place against the
pointer and its height against the VM's, and its width within 12% (the
text is the accepted deviation). Both pass on Linux.

## A0 VM extras (the unsaved Easy Empire game, days 768..769)

The Battle at Sluis Van (`.artifacts/a0-vm/sluis-van-768/`):

- 01..04: the alert disables Retreat with 10973 at (137, 348), the port's
  Empire disabled art, every pixel. The forces lists show Fleet 55
  (Freedom, X-wing, A-wings) against Fleet 2 (Victory Destroyer 1, TIE
  Fighter, Imperial Fleet Regiment, Jerjerrod). No gravity well is visible
  there, so the cause is not settled.
- 05: Simulate gives "The Imperial fleet is defeated." / "Sluis Van is now
  under blockade by Alliance forces. The Imperial fleet has been completely
  destroyed."
- 07: the Sluis sector window draws Sluis Van with its icons 10779 and
  10781 and the Empire's uprising mark 906 at (538, 82), all exact. There is
  no blockade mark.

## Runs

Batch b127 passed 23 of 23 gates.

The tactical matrix passed 146 of 148 (run 2026-11-08T00-36-41-934Z-2214619).
Both trench-run 640x480 cases failed on their movie request while unit
tests were compiling alongside. Rerun alone, each passed 4 of 4:

- 2026-11-08T00-50-06-836Z-2251628
- 2026-11-08T00-50-21-578Z-2252464

## Mutants

The in-place run over the diff tested 28 mutants and left 6 survivors,
all in the name's fallback position for a row without a picture.
`a_row_without_a_picture_keeps_its_name_at_the_cells_foot` covers that
position.
