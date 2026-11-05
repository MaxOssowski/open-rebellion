# Delete, Ctrl and Shift in the Message Index against A0 (2026-11-05)

This covers three ledger cells:
- CMD-08-C016 ("delete");
- CMD-08-C018 (Ctrl-click toggle);
- CMD-08-C019 (Shift-click range).

## A0

The VM's Alliance save "test", day 8, Advice Messages. The captures are in `.artifacts/a0-vm/message-delete/`; `a0-sha256.txt` lists their hashes.

**Delete (`0x91`, the window's (340, 87))** asks no question.
- `del3` → `del4`: deleting the first row, "Maintenance Points", chooses the row after it, "Victory Conditions".
- `del5` → `del6`: deleting "Positioning Fleets" chooses the row after it, "The Battle Alert".
- `del7` → `del8`: deleting "Furthering Our Cause" chooses "Advice Available".
- `del9`: deleting that last row chooses the row before it, "Adjusting Time".
- `sel2` → `sel3`: with rows 0 and 2..4 chosen, Delete removes all four and chooses the row after the first of them, "Galaxy Overview".

This is `FUN_00467f10`, case `0x91` in mode 1. At the first chosen item it keeps that item's next one (vtable `+0xc`), or its previous one (`+0x10`) when there is no next. It deletes every chosen item and then chooses the kept one (`FUN_00609500(.., 1)`).

**Ctrl and Shift.**
- `sel0..sel2`: a click on row 0, Ctrl on row 2 and Shift on row 4 choose rows 0 and 2..4. Shift adds its range from the last clicked row and keeps row 0.
- `sh0..sh2`: a click on row 1 and Shift on row 4 choose 1..4. Shift back on row 2 still leaves 3 and 4 chosen.

## Before

- **Delete.** The port cleared the choice, so no row was chosen.
- **Shift.** It replaced the choice with the range.

## Now

- **`delete_selected(rows)`.** It chooses the row after the first chosen one, or the one before it at the list's end. The anchor moves to that row. port: if that row is deleted too, no row is chosen, because the original would keep a freed item.
- **Shift-click.** It adds its range to the rows already chosen.

## Tests

- **Unit tests.**
  - `delete_returns_the_ids_and_chooses_the_row_after_the_first` replaces the old `..._clears_selection`; A0 shows the old expectation was wrong.
  - `deleting_the_last_row_chooses_the_one_before`, `a_successor_deleted_too_leaves_no_row_chosen` and `deleting_the_only_row_chooses_none` cover the other cases.
  - `shift_click_adds_its_range_to_the_rows_already_chosen` covers Shift.
  - The new tests fail without the change. Mutants: `delete_selected` 10 caught and 1 unviable; `apply_click` 7 caught.
  - Two mutants in the batch's in-diff run survive. They delete the draw loop's `0x65` and `0x91` command arms. Unit tests do not cover the draw pass, but the browser gate below presses both buttons.
- **`message-display.mjs`** (Linux, the Alliance):
  - **Delete.** In the Conflict list, Delete removes "Assault on Coruscant" and chooses "Assault on Umgul". That row's 32 by 17 head is 0 differing from A0 `index-tab7`'s chosen head.
  - **Ctrl and Shift.** In the Mission list, a click on row 0, Ctrl on row 2, Shift on row 4 and then Shift on row 3 leave rows 0 and 2..4 chosen, as A0 does.
- **The batch.** The b71 batch ran all 21 Linux gates, and every one passed.
