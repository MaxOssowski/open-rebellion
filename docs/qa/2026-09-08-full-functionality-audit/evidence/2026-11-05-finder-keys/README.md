# The Finders' keys against A0 (2026-11-05)

This covers OBJ-04-C008 ("object navigation").

## A0

A new Alliance game. The captures are in `.artifacts/a0-vm/finder-keys/`.
- **Fleet Finder (F3).** After a click on Fleet 1 (`f1`), Down chooses Fleet 2 (`f2`) and Fleet 3 (`f3`), Up Fleet 2 (`f4`), End the last row, Fleet 4 (`f5`), and Home Fleet 1 (`f6`). Each copies the row's name into the Fleet Name box.
- **Fleet Finder reopened, focus in the name box** (`g0`..`g3`). Down still steps the list. So the box passes Up and Down on.
- **Troop Finder (F4).** It opens on its first row, Drall (`t0`). Down chooses Fleet 1 and then Fleet 4, and Up Fleet 1 (`t1`..`t3`), each named in the box.
- **Personnel Finder (F5).** It opens with no row chosen (`p0`). Down chooses the second row, Han Solo (`p1`), then Jan Dodonna (`p2`). The list's own current row is its first even when none shows chosen.

## Before

The port's Finders took no Up, Down, Home or End.

## Now

- **`list_prefix`.** It has `ListStep`, `step_of`, `key_step` and `stepped_row`, shared by the Fleet (and Ship), System, Personnel and Troop Finders.
  - Up and Down always step the chosen row. Home and End do so only while the name box has not the focus, since there they move its caret.
  - A step copies the row's name into the box and scrolls it into view, as a click does.
  - With no row chosen, Down takes the second row.
- **Unit test.** `a_key_steps_the_chosen_row_and_stops_at_the_lists_ends` does not compile without the change, since the functions are new. Mutants: `stepped_row` and `step_of` all caught. `key_step` → `None` survives the unit tests; the gate catches it.
- **`fleet-finder.mjs`** (Linux, both sides), the new `keys` case: from none chosen, Down chooses row 1. After a click on row 0, Down, Down, Up, End and Home choose rows 1, 2, 1, the last and 0, each with its name in the box.
- **`encyclopedia-placement.mjs --window=finders`** (Linux, the Alliance). The reopened Troop Finder's first row is chosen; Down lights the second row and Up the first again (brightest ink 252 against 119).
- **The b83 batch.** It ran all 21 Linux gates, and every one passed. The in-diff mutants: 19 caught and 6 unviable. Three survive in the draw paths: the chosen row's `position` test in the Fleet and System Finders, and `key_step` → `None`. Both gates' key cases cover them.

## Still open

The Personnel Finder (OBJ-03) shares the code but has no browser case for its keys.
