# The Message Index's Conflict list against A0 (2026-11-05)

This covers CMD-08-C011 ("combat"): a Conflict list compared with A0.

## A0

The VM's Alliance game, day 633 (`messages-day633/index-tab7`, the index at (62, 49)). Conflict Messages holds two rows:
- "Assault on Coruscant", chosen;
- the older "Assault on Umgul" below it.

Both rows show icon STRATEGY `0x2ad7` (10967, 15 by 15) at the row's (1, 1). The index was already read, so each title is in font 10 (`FUN_00468ab0`: `+0x38` 10 for a read message).

## Before

- **The unchosen row's icon.** The port drew it whole. A0 drops its last column. `0x2ad7`'s column 14 has two dark pixels (indices 110 and 105, rows 10 and 11). The chosen row shows them; the unchosen row shows the list's blue there.
- **The fixture.** No scenario filed a Conflict-rail message, so no gate looked at this list.

## Now

- **`row_icon_shown`.** An unchosen row draws the icon without its last column and row. `FUN_00468ab0` builds each item's image at the size of the loaded resource (`FUN_005fbda0` with `FUN_005fc0e0`/`FUN_005fc0f0`) and copies the icon into it at (1, 1) (`FUN_005fcc30`), so the image cuts the icon's last column and row off. The decompile drops the `this` of those size calls; A0 settles it. The chosen row keeps the whole icon, as A0 shows.
- **`post_conflict_report`.** The EventMessages fixture files "Assault on Umgul" and then "Assault on Coruscant" on the reader's Conflict rail.

## Tests

- **Unit tests.**
  - `an_unchosen_rows_icon_loses_its_last_column_and_row` fails without the change. Its mutants: 18 caught, 1 unviable.
  - `the_event_messages_scenario_files_a_conflict_report_for_its_reader`: its function's 3 mutants are caught.
- **`message-display.mjs`** (Linux; the Alliance against A0). The gate opens both reports so they are read, then chooses Coruscant.
  - **The chosen row.** The icon is exact in the port and in A0, and the 32 by 17 head is 0 differing.
  - **The unchosen row.** The cut icon is exact in both, and the head is 0 differing; before the change it was 2.
  - **The text.** Its bright extent is x 32..136 against A0's 33..136. The port's TTF glyphs are smoothed where GDI's are not, which is the accepted text deviation, so the 596 differing pixels of the text are recorded, not asserted.
- **The batch.** The b70 batch ran all 21 Linux gates, and every one passed.

## Delete in A0

`.artifacts/a0-vm/message-delete/del3..del9` record a separate check, Delete in A0, for CMD-08-C016. Delete removes the chosen row without a question and chooses the row after it, or the row before it at the list's end.
