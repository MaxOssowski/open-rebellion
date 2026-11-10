# The Fleet window's left list gains its scroll bar

Date: 2026-11-11. Follows `2026-11-10-fleet-window-scroll-bar-a0` (the
right list's bar). Adds to OBJ-06-C010 (contents) and OBJ-10-C006
(aboard fleet).

## The original

`FUN_004a4b10` creates the left list `+0x160` (id `0xc9`) at (4, 29),
91 by 266 (`0x5b x 0x10a`), with entries 91 by 50, and gives it the same
bar as the right list, `FUN_0060a490(.., 0x29cb, ..)`: STRATEGY RCDATA
10699, 10 wide (`ghidra/notes/fleet-window.md`). `FUN_0060fa80` places a
bar at the list's right edge less its width, so the left bar stands at
window x 85..94, y 29..294. `FUN_0060b5d0` shows it unless the whole
range is in view.

What a selection does to the left list's place: `FUN_004a6390`, id
`0xc9`, code `0x29b` only posts `0x467` and calls `FUN_004a5c00` (the
right pane). Nothing there moves the left list. Expand (`0x309`) adds
the ship entries (`FUN_004a3d40`) and calls `FUN_0060a280`. The port
keeps the left list's place on a selection; the section "Selection from
outside" below settles it for the other path too.

## The port before

No left bar and no scrolling. The list drew the entries that fit from
its top (six, the sixth cut at y 295) and nothing past them; a seventh
fleet, or an expanded fleet's ships past the sixth row, could not be
seen, selected, dragged, or targeted.

## The change (`crates/rebellion-render/src/fleet_window.rs`)

- Each window keeps the left list's first visible pixel and the press
  held on its bar (`left_scroll`, `left_hold`), beside the right list's.
- `left_row_at`, `left_content`, `left_row_top` mirror the right list's
  helpers for (4, 29, 91, 266) and 50-high entries.
- The left list draws its entries from that offset, clipped to the
  list; an entry wholly outside it is skipped.
- The bar shows once the entries pass 266 pixels (`LIST_BAR_ART`, the
  shared `list_scroll_bar::ui_bar`; arrows step 10, `hyp:` as for the
  right list). The track pages and the thumb drags.
- A press on the bar does not select, toggle, open a menu for, or drag
  the entry under it: an entry's hit area stops at the bar while the
  bar shows.
- The bar hides and the list returns to the top once the entries fit
  (`reset_when_hidden`).
- Everything mapping a left row to the screen or back takes the offset:
  `entry_screen_rect` (the fixture's left-entry points), the `+0x70`
  drop (`drop_target`, read by `targeting.rs` through `release_target`),
  the `+0x68` mission target (`mission_target`, read by `targeting.rs`),
  the rename field over a left entry.
- A selection, from a click or from outside (`select`), keeps the left
  scroll; the right list keeps its place too (see "The right list's place
  on a refill").
- `ghidra/notes/fleet-window.md` port rules: "there are no scroll bars"
  removed; the arrow step stays `hyp:`.

## Tests

`cargo test -p rebellion-render --lib`: 1421 passed, 6 ignored (with the
sections below).

| Test | Fails without |
|---|---|
| `the_left_lists_entries_follow_its_scroll` | (new helpers; checks rows at y 29 and the list's end at y 295) |
| `a_scrolled_left_list_moves_its_entries_rects_and_drop_rows_up` | `entry_screen_rect` without the offset; `drop_target` with the old row; `mission_target` with the old row (each reverted alone, each fails) |
| `a_click_on_a_scrolled_left_list_selects_the_entry_shown_and_the_bar_selects_nothing` | rows drawn unscrolled; the entry's hit area over the bar (each reverted alone, each fails) |
| `the_left_lists_bar_hides_and_returns_to_the_top_once_its_entries_fit` | no reset when the bar hides |

The six reverts were applied one at a time by a script and the file
restored byte for byte (`cmp`); each made exactly the named test fail
(79 passed, 1 failed of the `fleet_window::tests` filter).

Also: strict Clippy on `rebellion-render` reports nothing in
`fleet_window.rs`; rustfmt reports nothing in the new hunks (the file's
baseline drift is untouched); `provenance-scan check` is ok.

## Not done

- No A0 capture shows more than five left entries, so the left bar's
  pixels and its overflow are not compared with the original. This is
  code and unit tests only; no browser gate was run.
- The arrow step (10) is not read for this window (`hyp:`).
- A rename begun on an entry or item scrolled out of view puts the field
  outside the list, clipped away (the original places a child edit there,
  possibly over the window's other controls). Not compared.
- The `fleet-window.mjs` gate was not rerun after the right list stopped
  resetting; its bar check opens the Fighters tab on a fresh window, so
  it should not depend on the reset, but that is unchecked.
- `cargo mutants` and the batch gates were not run (worker rules).

## The rename field in a scrolled list

The original. `FUN_004ac7a0` creates the edit once, a child of the window,
at the rect `FUN_004a71d0` returns: the selected item's text rect
(`FUN_0060a2a0` on item `+0x40..+0x4c`) moved by the list's position, its
right edge the list's right less 8. `FUN_00609e20` keeps `+0x40..+0x4c`
at the item's content position less the list's first visible pixel
`+0xc4` (`FUN_00609dd0`), so the edit opens where the item is shown.
It is not moved afterwards, but no scroll can happen under it: the bar
(`FUN_0060f640`, base procedure `FUN_0060aca0`) posts `0x40e` on
`0x201`/`0x203`, the list relays it (`FUN_006083c0`), and the window's
`FUN_004ac3a0` drops the edit (vtable `+0x80`, `FUN_004aca40`), as it does
for its own `0x201`/`0x204`.

The port before. The right-list field (`draw_rename_edit`, item branch)
sat at the unscrolled row, clipped away when the list was scrolled.

The change. The item branch subtracts `right_scroll` (the left branch
already took `left_scroll`). A press on a bar already ended the edit
through the field's lost focus; the comment there now cites the trace
instead of `hyp:` (Escape stays `hyp:`). Notes: `ghidra/notes/
fleet-window.md` "Rename".

Tests:

| Test | Fails without |
|---|---|
| `the_rename_field_opens_on_a_scrolled_right_items_shown_name_line` | the item branch without `- window.right_scroll` |
| `the_rename_field_opens_on_a_scrolled_left_entrys_shown_name_line` | the left branch without `- left_scroll` |
| `a_press_on_a_lists_scroll_bar_ends_the_rename_unissued` | the edit not ending on lost focus. A guard: it passed before this change, since egui drops the field's focus on a press elsewhere |

## Selection from outside

The original. The Fleet Finder's open (`FUN_00429440`) calls the
window's slot `+0x6c`, `FUN_004a73c0`. It finds the entry by id
(`FUN_0060a860`), sets its `+0x3c` bit 0, expands the fleet for a ship
(`FUN_004a3d40`) and relays the list out (`FUN_0060a280` =
`FUN_00609ae0` layout + `FUN_00609a00` repaint), then picks the tab
(`FUN_0060d7e0`). `FUN_00609ae0` only clamps `+0xc4` to the new range;
nothing scrolls the selected entry into view. A click (`0x29b`,
`FUN_004a6390`) does not move the left list either.

The port. Already so: `select` keeps `left_scroll`, and the draw clamps
it to the range. Documented in the field's comment and in a new
"Scroll and selection" section of `ghidra/notes/fleet-window.md`.

Test: `a_selection_from_outside_leaves_the_left_list_where_it_was_scrolled`
fails when `select` resets `left_scroll` (injected; it also fails
`the_rename_field_opens_on_a_scrolled_left_entrys_shown_name_line`).

## The right list's place on a refill

The original. A selection (`0x29b`, `FUN_004a6390`; or slot `+0x6c`,
`FUN_004a73c0` → `FUN_0060d7e0`) and a tab click (`0x6a`/`0x6b`,
`FUN_004a6390`) refill the right pane through `FUN_004a5c00`. Read from
REBEXE.EXE's disassembly as well as the decompile:

- `0x4a5c4a`: `mov ecx,[esi+0x18c]; call 0x5f5b20` empties the right
  list's item container (`FUN_005f5b20` deletes the items, count 0);
- the list is shown or hidden (its vtable `+4(5)` / `+8`), the items are
  added (`FUN_004a6be0`);
- `0x4a61b5`: `mov ecx,[esi+0x164]; call 0x60a280` lays the list out.

Nothing there writes the list's first visible pixel `+0xc4`. In the list
class it is moved only by `FUN_00609ed0` (a scroll from the bar) and
clamped by `FUN_00609ae0` (`if range < +0xc4: +0xc4 = range`, range =
content + 5 - view for a full list, else 0). `FUN_00609650` (the finders'
type-ahead) is the only other caller of `FUN_00609ed0`, and the Fleet
window does not call it. So the right list keeps its place across a
selection or a tab change, clamped to the new range.

The port before. The 2026-11-10 change reset `right_scroll` to 0 on a
selection (`select`, the click path in `draw_fleet_windows`) and on a tab
change (`OpenFleetWindow::show_tab`), without a citation.

The change. The three resets are gone, each site now citing the trace.
The draw already clamps the offset to `max_first` while the bar shows and
returns it to 0 when the bar hides (`reset_when_hidden`), which is
`FUN_00609ae0`'s clamp.

Tests changed on this trace's evidence (they asserted the uncited
reset):

- `a_scrolled_right_list_moves_its_items_up_and_resets_on_a_new_selection`
  is now `..._keeps_its_place_on_a_new_selection`: after `select` the
  offset is 40, not 0.
- `another_tab_starts_its_list_at_the_top` is now
  `another_tab_keeps_the_right_lists_place`: after a tab change the
  offset is 40, not 0. Its first half (the same tab keeps 40) was folded
  into the new assertion.

New: `a_new_selection_draws_the_right_list_from_its_kept_place_clamped_to_the_range`.
Six ships scrolled to 100 keep 100; a click on a fleet of four ships
clamps to 200 + 5 - 164 = 41; a one-ship fleet shows from 0.

Fail-without (each reset put back alone, the file restored by `cmp`):
the `select` reset fails the renamed first test, the tab reset fails
`another_tab_keeps_the_right_lists_place`, and the click-path reset fails
the new clamp test.

## Batch b180

In-place mutants on the render diff: 71 mutants, 66 caught, 5
unviable, none missed. 23 of 23 interface gates on Linux, `fleet-window`
(`fleet-window-2026-11-10T21-55-34-115Z-2094244`) among them, its
a0-coruscant bar check unchanged; workspace tests 3516 passed, fixture
tests 140. Tactical matrix `2026-11-10T22-43-06-796Z-2178902`: 148 of
148.
