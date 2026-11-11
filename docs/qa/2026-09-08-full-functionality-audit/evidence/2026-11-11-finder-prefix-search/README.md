# Finder prefix search, traced across the four Finders

Date: 2026-11-11. For OBJ-03-C019 (SpecForces Finder system-name search).
It settles the open point of `../2026-10-29-troop-finder-first-row/`:
"the Personnel Finder's case `100` (`FUN_00464890`) does not match a
prefix". Code and unit tests only; no A0 capture and no browser gate were
run for it.

## The original

The four Finders handle typing the same way. Each name box (`0xcb`, a
`CoolStringField`) posts `0x408` to its window on every edit: a
character typed, Backspace, or Delete (`FUN_00605160` at its
`PostMessageA(.., 0x408, ..)` sites). The window's slot `+0x14` answers it:

| Finder | `+0x14` | `0x408` list | id store |
|---|---|---|---|
| System (`0x65a000`) | `FUN_00461000` | its list | `+0x160` |
| Fleet (`0x65a090`) | `FUN_00462a50` | its list | `+0x16c` |
| Personnel (`0x65a130`) | `FUN_00464c20` | `+0x150` (disassembly `0x464cf4`) | `+0x178` |
| Troop (`0x65a270`) | `FUN_0046e5f0` | its list | `+0x17c` |

In each one:

1. `FUN_005f3040` measures the name box's string (`+0x98`): its length
   plus one, and 0 only when the string has no buffer. `FUN_00604cf0`
   builds the box without one (`FUN_005f2f50` sets `+4 = 0`). Every edit
   or set gives it one, an empty one included: `FUN_005f3ad0`,
   `FUN_005f3c30` (it allocates the shortened copy), and `FUN_00604f90`.
2. If the measure is not 0, `FUN_00609650(text, 0, 0)` runs on the shown
   list. The rule is the one in `list_prefix::best_prefix_row`:
   - it ignores case (`FUN_005f3430`);
   - the longest shared beginning wins, and a tie keeps the earlier row;
   - with no letter shared, the first row is chosen;
   - the row is chosen (`FUN_00609500`) and scrolled to (`FUN_00609ed0`).
   `FUN_0060a860` then finds that row's id for the window's store.
3. With no row, which only happens for an empty list, the store is
   emptied.

So the typed prefix is the name box's whole text:

- there is no timeout and no accumulated key buffer;
- each edit rescans from the top, so nothing wraps around;
- a name emptied with Backspace chooses the first row.

`FUN_00604f90` (the box's text set by a row click) posts no `0x408`.

The rows' text is the object's name. In the Personnel Finder,
`FUN_00464e10` gives each item `FUN_004f62d0`'s name (`FUN_005f35e0`): a
character's in Characters, a system's or a fleet's in SpecForces. So the
SpecForces view is searched by system (or fleet) name through the same
rule.

Where the Finders differ is what a tab does:

- System (`FUN_00460d10`), Fleet (`FUN_00462770`) and Troop
  (`FUN_0046df90`), case `100`: they rebuild the list, then run
  `FUN_00609650` on the box's text. If it finds a row, they store its id.
- Personnel, case `100` (side tab) and `0x6e` (view tab) of
  `FUN_00464890`: they only rebuild (`FUN_00464e10`), with no
  `FUN_00609650`. So after a tab no row is chosen, but `+0x178` keeps the
  id typed or clicked before. Display (case `0xc9`) and Enter (`0x407`,
  `FUN_00464c20`) open `+0x178` (`FUN_00429440`).

That is what "does not match a prefix" meant. Typing in the Personnel
Finder does match a prefix; its tabs do not.

## The port before

- **Fleet, Troop, Personnel:** every edit ran `list_prefix` (an empty name
  chose the first row). The Fleet and Troop Finders rematched on a side
  tab. All as above.
- **System Finder:** an edit that emptied the name cleared the choice.
  The note had read `FUN_005f3040` as "non-empty".
- **Personnel Finder:** a side or view tab cleared the choice, and Display
  and Enter opened only the visibly chosen row. A name typed before a tab
  change opened nothing.

## The change

- **`system_finder.rs`:** `edited_choice` is the `0x408` choice. It is
  `prefix_choice` for every edit, so a name emptied by an edit chooses the
  first row.
- **`personnel_finder.rs`:** `OpenFinder::target` is `+0x178`, with the
  side tab it was chosen on (port: that tab picks a system row's unit, as
  `action_for_row` did). A typed name, a click or a key step sets it with
  `chosen`. `set_tab` and `set_view` clear `chosen` (the rebuilt list has
  no chosen row) and keep `target`. Display and Enter open `target`.
- **`list_prefix.rs`:** the `hyp:` about the buffer is replaced by the
  trace above.
- **Notes:**
  - `ghidra/notes/personnel-finder.md`, "Choosing and opening": rewritten
    from the disassembly. It had named the list `+0x174` and left the
    searched names as `hyp:`.
  - `ghidra/notes/system-finder.md`: the `0x408` line is corrected.

## Tests

`cargo test -p rebellion-render --lib finder`: 99 passed. Both new tests
were shown to fail with their change reverted, one revert at a time, with
the file restored afterwards:

- `system_finder::tests::a_name_emptied_by_an_edit_chooses_the_first_row`:
  `""` now chooses the first row, `"cor"` row 2, and an empty list none.
  It fails with the old "non-empty" guard.
- `personnel_finder::tests::display_after_a_tab_change_opens_the_name_typed_before_it`:
  the test types "lu" (Luke is chosen), switches to the Imperial tab (no
  row chosen), and clicks Display. Display opens Luke. It fails when
  Display uses the visibly chosen row.

The existing `a_new_side_tab_clears_the_choice` and
`a_new_view_tab_clears_the_choice` stand: the visible choice still
clears.

Strict Clippy shows nothing in the three files. rustfmt reports nothing
in the new hunks; the files' baseline drift is untouched.

## Edge cases (second pass)

The original:

- **Tab with no rows.** In the tab's case `100`, the System, Fleet and
  Troop Finders store `FUN_00609650`'s row only when it finds one
  (`if (id != 0)`). What the rebuild does to the store first differs, read
  from the disassembly:
  - Fleet: `FUN_00462be0` empties `+0x16c` when the tab changes
    (`0x463114`: `lea ecx,[ebp+0x16c]` … `call 0x4f26d0` with an empty
    id). So an empty tab leaves nothing chosen, which the port already
    does. The first pass's "the Fleet Finder keeps the old id" was wrong,
    and the Fleet Finder is unchanged.
  - Troop: `FUN_0046ea10` does not write `+0x17c`; its only other writer
    is the constructor (`0x46f1de`). So an empty tab keeps the id chosen
    before, and Display opens it.
  - System: `FUN_004611d0` does not write `+0x160`. The port's System
    Finder already keeps its choice (`set_tab` does not clear it).
- **Personnel Finder scroll.** `FUN_00464e10` swaps the list's item
  container (`FUN_00608280`) and lays it out (`FUN_0060a280` →
  `FUN_00609ae0`). That only clamps the first visible pixel `+0xc4` to the
  new range.
- **Backspace in a never-edited box.** It posts `0x408` with no buffer,
  which empties the store without a match. egui reports no change for
  Backspace in an empty `TextEdit`, so the port cannot see this event. It
  is left as documented.

The change:

- **`troop_finder.rs`:**
  - `OpenFinder::target` is `+0x17c`. A typed name sets it (emptied with
    no row), and so do a click, a key step, and the tab's rematch when the
    rematch finds a row.
  - `set_tab` clears only the visible `chosen`.
  - Display and Enter open `target`.
- **`personnel_finder.rs`:** `set_tab` and `set_view` no longer reset
  `scroll`. The draw's bar (`ScrollBar::new`) clamps it to the new range.

Tests (each shown failing with its change reverted alone, files
restored):

- `troop_finder::tests::display_after_a_tab_with_no_rows_opens_the_row_chosen_before`:
  Alpha is chosen on opening. The Imperial tab has no rows, and Display
  opens Alpha. It fails when Display uses the visibly chosen row.
- `personnel_finder::tests::a_tab_keeps_the_lists_place_clamped_to_its_new_rows`:
  the list is scrolled to 30. Switching to the SpecForces view keeps 30,
  the Imperial side keeps 30, and the Characters view with no Imperial
  character clamps to 0. It fails with either reset restored: `set_view`
  fails at "kept by the view", and `set_tab` at "kept by the side".
- The Enter path:
  `display_after_a_tab_change_opens_the_name_typed_before_it` now also
  types "lu", switches tab, and presses Enter, which opens Luke. It fails
  when Enter uses the visibly chosen row.

`cargo test -p rebellion-render --lib`: 1430 passed, 6 ignored. Strict
Clippy shows nothing in the touched files. rustfmt is clean on the new
hunks.

## Third pass: the Fleet and Troop scroll, and the Encyclopedia

**The Fleet and Troop Finders' tab rebuild.** Neither `FUN_00462be0`
(Fleet) nor `FUN_0046ea10` (Troop) writes the list's first visible pixel
`+0xc4`. In the list class only `FUN_00609ed0` moves it and
`FUN_00609ae0` clamps it. Each rebuild:

1. empties and refills the items;
2. lays the list out (`FUN_0060a280` → `FUN_00609ae0`, the clamp);
3. then case `100`'s `FUN_00609650` scrolls only as far as its row needs
   (`FUN_00609ed0`; only when that row is not already chosen, and a
   rebuilt row never is).

The port had reset `scroll` to 0 in `set_tab`. Now:

- `set_tab` keeps `scroll`;
- the rematch uses `rebuilt_scroll`, which clamps to the new range, then
  `scrolled_to` the matched row.

Test, in both Finders:
`a_tab_keeps_the_lists_place_clamped_then_scrolls_to_its_match`. It
fails with the `set_tab` reset restored, and with the clamp removed (4
reverts, one at a time). With an empty name box the rematch chooses row 0
and so still scrolls to the top.

**The Encyclopedia (`0x659fa8`).**

- Typing in the Topic box (`FUN_0045d8f0`):
  - `0x408` runs `FUN_00609650` and stores the item in `+0x148`;
  - `0x407` with a chosen item opens its topic (`FUN_0045f480(2)`).
- A category icon (case `0x6e` of `FUN_0045da70`):
  - `FUN_0045f100` swaps the items and lays the list out, which keeps the
    scroll, clamped;
  - then `FUN_00609650` runs on the box's text, so it chooses the best
    prefix row and scrolls to it.

The port had:

- chosen the first row of a category that lacks the chosen object;
- returned the scroll to the top on every category change, citing
  `FUN_0045f100`, which does not do that;
- no typing in the box at all. It paints the box as a label.

Ported:

- `choose_category_row` takes `best_prefix_row` of the box's text, which
  is the first row for an empty box. That agrees with A0 category-1..6
  and with ex3, where "Adar Tallon" chooses Adega.
  - Test: `a_category_chooses_the_row_the_topic_box_best_begins`. With
    "Corran Horn" in the box, System Database chooses Corellia, not
    Adega. It fails with the first-row rule.
- A new `change_category` keeps the scroll and lets the reconcile clamp
  it and reveal the row.
  - Test: `a_category_keeps_the_lists_place_while_its_row_shows`. A scroll
    of 60 stays 60 while row 6 shows; at 150, row 6 brings it to 120. It
    fails with the reset restored.

Not ported: typing into the Topic box, and Enter. The trace settles what
an edit does, but not which control holds the keys:

- the Index command gives the box the focus;
- a category icon gives it to `+0x12c`;
- after a row click, it is untraced.

The port's box is a painted label, and the browser gates compare it.
`ghidra/notes/encyclopedia-window.md` "The Topic box" records this.

`cargo test -p rebellion-render --lib`: 1434 passed, 6 ignored. Strict
Clippy shows nothing new in the touched code; the existing warnings in
`encyclopedia.rs` are not in these hunks. rustfmt is clean on the new
hunks.

## Not done

- No A0 capture of a Personnel Finder search, of Display or Enter after a
  tab change, or of a Troop tab with no rows. The trace above is the
  evidence. The cell can pass only with that A0 check, or with the owner
  accepting the trace.
- Backspace in a never-edited name box (see above): egui does not report
  it.
- Typing and Enter in the Encyclopedia's Topic box (see above).
- `FUN_00609650` does not choose or scroll when its best row is already
  chosen (`(+0x3c & 1) == 0`). The Finders' port scrolls to the row on
  every edit, even when the user scrolled away from a chosen row whose
  match did not change. Not ported.
- The browser gates (`personnel-states.mjs` category rows,
  `encyclopedia-placement.mjs --window=finders`) were not rerun. Their
  cases start at scroll 0 with an empty box, so they should be
  unaffected.
- `cargo mutants` and the browser gates were not run (worker rules).

## Batch b183

23 of 23 interface gates on Linux (`fleet-finder`
`fleet-finder-2026-11-11T02-02-25-056Z-3282782`, `system-finder`
`system-finder-2026-11-11T02-03-20-927Z`, `encyclopedia-index-shell`
`encyclopedia-index-shell-2026-11-11T02-04-06-234Z-3288029` among
them); workspace tests 3547 passed, fixture tests 148. Tactical matrix
`2026-11-11T02-33-54-936Z-3332584`: 148 of 148. In-place mutants missed
two in the Encyclopedia's category guard, a line from 2026-10-01 next
to the changed one; the next commit moves the guard into
`change_category` with a test. One full `rebellion-render` run before
the batch failed
`status_window::tests::the_objectives_rows_stack_35_apart_with_centred_icons_and_text_at_40`
once; it passed alone three times and in four full runs after, so it is
recorded as a flaky test, not caused by this change.
