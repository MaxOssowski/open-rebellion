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

## Fourth pass: no scroll to a row chosen already, and the Topic box

### The original

**`FUN_00609650` skips a row already chosen.** At `0x60973f`
(`test byte [esi+0x3c],1; jne 0x609764`), it chooses the best row
(`FUN_00609500`) and scrolls to it (`FUN_00609ed0`) only when the row is
not chosen already. It returns the row's id either way.

- An edit that keeps the same best row (typing "c0" after "c" when C00 is
  already chosen) leaves the list where the user scrolled it.
- A tab's or a category's rebuilt rows are never chosen, so a rebuild
  always scrolls to the match.

**Which control holds the keys in the Encyclopedia.** `WM_CHAR` reaches
only the focused window:

- the list (`FUN_006083c0`) forwards unhandled `WM_KEYDOWN` to the
  window;
- the window (`FUN_0045fe60`) passes letters on to its own parent;
- the base handler (`FUN_00606650`) has no `WM_CHAR` case.

So letters reach the Topic box only while it holds the focus:

| Event | Focus after it | Source |
|---|---|---|
| Opening | unchanged (no `SetFocus`) | `FUN_00429f30`, `FUN_0045d400`, `FUN_0045ddc0` |
| A press on the box | the box | `FUN_00605160` `0x201` |
| Index (`0x68`), or Topic (`0x67`) with nothing chosen | the box `+0x128` | `FUN_0045da70` case `0x66` |
| A category icon (`0x6e`) | the list `+0x12c` | `FUN_0045da70` |
| A press on a row | the list | `FUN_006083c0` |
| A topic shown, Previous / Next | the topic text `+0x13c` | `FUN_0045f480(2)` |

In the box, `0x408` runs `FUN_00609650` and stores the item in `+0x148`
(`FUN_0045d8f0`). Enter (`0x407`) with an item opens its topic.

### The change

- **`list_prefix::row_to_show`:** the best row, unless it was chosen
  before the edit. All four Finders' `0x408` paths scroll only to that
  row. The Fleet and System Finders read the earlier row from their
  chosen object's position in the list.
- **`encyclopedia.rs`:**
  - `EncyclopediaState::original_box_focused` follows the table. It is
    set by:
    - `command_focus`;
    - a press on the box (a new interact on (143, 45, 245, 18));
    - a row press;
    - `0x68` in `apply_original_topic_command`.
  - The opening clears it.
  - While it holds, `type_into_topic_box` takes typed text and Backspace
    into the box's text (`original_topic_text`), chooses with
    `best_prefix_row`, and reveals the row only through `row_to_show`.
  - Enter with an object chosen shows its topic.
  - The box keeps its painted look.
  - port: the caret is neither drawn nor placed at the pointer (it stays
    at the end), and Delete is not ported.

### Tests

Each was shown to fail with its change reverted alone, and the files were
restored afterwards.

- **Finders:** `an_edit_that_keeps_the_chosen_row_leaves_the_list_where_it_is`
  in `system_finder`, `fleet_finder`, `personnel_finder` and
  `troop_finder`. Each fails with its finder's `row_to_show` call
  replaced by the plain choice.
  - The Fleet test first passed without the change (its row was still in
    view). It now scrolls to 100, where the row is hidden.
- **`list_prefix`:** `an_edit_scrolls_only_to_a_row_not_chosen_already`
  fails with the helper ignoring the earlier row, and so do three of the
  Finder tests.
- **Encyclopedia:** `typing_reaches_the_topic_box_only_once_it_is_pressed`
  covers:
  - typing before a press does nothing;
  - after a press, "s1" chooses S10 and scrolls to 60;
  - "s10" keeps S10 and the list stays at 0;
  - Enter shows the topic.

  It fails without the box's press focus, without the focus gate, without
  Enter, and with the scroll always revealed.
- **Encyclopedia:** `a_category_icon_or_a_row_takes_the_keys_from_the_topic_box`
  covers:
  - a category leaves typing without effect;
  - Topic with nothing chosen focuses the box;
  - Index focuses the box;
  - a row press takes the focus again.

  It fails without the category rule, without the row rule, without
  `0x68`'s focus, and without the focus gate. A category icon's press is
  keyed on its bitmap, which the test cache lacks, so this test applies
  the command as the draw does.

`cargo test -p rebellion-render --lib`: 1449 passed, 6 ignored. Strict
Clippy shows nothing new in these hunks. rustfmt is clean on them.

### Gates

The box's painted look is unchanged, so no gate's comparison needs
updating:

- `encyclopedia-index-shell.mjs`;
- `encyclopedia-placement.mjs` (index placement);
- `encyclopedia-art.mjs`;
- `personnel-states.mjs` (categories, topic steps);
- `status-window.mjs` `force-character`.

The Finder gate `encyclopedia-placement.mjs --window=finders` types "I"
into a freshly opened Troop Finder. That chooses Ithor, a row not chosen
yet, so it scrolls as before.

New cases worth adding:

- in `personnel-states.mjs`: a press on the Topic box, typing (as in
  A0), and Enter;
- in `encyclopedia-placement.mjs --window=finders`: typing a second
  letter that keeps the row.

Neither has an A0 capture yet. Nothing was run here (worker rules).

## Fifth pass: the galaxy view's shortcuts while a text box has the keys

In the original, keys go to the focused edit control:

- each Finder's name box: the opener focuses the Finder's window, whose
  slot `+0x14` case 7 hands the focus to the box (`FUN_0042a640`;
  `FUN_00461000`, `FUN_00462a50` and `FUN_0046e5f0` handle case 7);
- a rename edit (`FUN_004ac7a0`);
- the Encyclopedia's Topic box, once it is pressed.

`main.rs` ran its plain-key shortcuts unless the Fleet, Troop or
Personnel Finder was open, or a rename was open. Those shortcuts are:

- R, which resets the map;
- Tab, the mod manager;
- the debug palette key;
- F6;
- the Alt accelerators.

So typing "r" into the System Finder's name box, or into the
Encyclopedia's focused Topic box, also reset the galaxy map.

The change:

- `main.rs`'s new `text_box_holds_keys` covers all four Finders, the
  System Finder now among them, plus a rename and
  `EncyclopediaState::topic_box_holds_keys`;
- that method is true while the index is open and the box focused, and
  false in a topic or a closed window.

The F-key accelerators in `cockpit.rs` are unchanged.

Tests (each fails with its change reverted):

- `text_box_key_tests::a_finder_a_rename_or_the_focused_topic_box_holds_the_galaxy_keys`
  (rebellion-app). It fails with the old condition, which had no System
  Finder and no Topic box.
- `only_the_open_indexs_focused_topic_box_holds_the_galaxy_keys`
  (rebellion-render). It fails without the topic-mode term.

`change_category` now holds its own guard (the category range and the
current category), as on the lead's main, and the call site calls it
unconditionally.

`cargo check -p rebellion-app` passes. `cargo test -p rebellion-render
--lib`: 1450 passed. `cargo test -p rebellion-app text_box`: 1 passed.

## Not done

- No A0 capture of a Personnel Finder search, of Display or Enter after a
  tab change, or of a Troop tab with no rows. The trace above is the
  evidence. The cell can pass only with that A0 check, or with the owner
  accepting the trace.
- Backspace in a never-edited name box (see above): egui does not report
  it.
- No A0 capture of typing in the Encyclopedia's Topic box. The focus
  table is static evidence.
- All Databases (`0x6f`) reuses the galaxy view's item list `+0x474`,
  whose items may keep their chosen bit, so a category change to it may
  skip the scroll. The port always reveals the row there.
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

## Batch b185 (fourth and fifth passes)

23 of 23 interface gates on Linux (`encyclopedia-index-shell`
`encyclopedia-index-shell-2026-11-11T04-27-28-187Z-3703108` among
them); workspace tests 3561 passed, fixture tests 149. Outside the
batch, `encyclopedia-placement`
(`encyclopedia-placement-2026-11-11T05-11-46-743Z`) and
`main-menu-states` (`main-menu-states-2026-11-11T05-14-06-377Z`) pass.
Tactical matrix `2026-11-11T04-58-22-110Z-3748674`: 148 of 148.
In-place mutants missed one: `command_focus`'s Index arm (0x68). The
test `a_category_icon_or_a_row_takes_the_keys_from_the_topic_box` now
applies 0x68 through `command_focus` and expects the box focused.
