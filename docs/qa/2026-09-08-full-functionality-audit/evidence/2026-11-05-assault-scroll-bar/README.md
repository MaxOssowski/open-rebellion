# The Assault Summary's list scroll bar (2026-11-05)

This covers EVT-03-C010 ("force tabs"), the part `2026-11-05-assault-taken/` left open.

## The original

- **The bar.** `FUN_0044c630` gives the forces list (`FUN_00607ea0` at (37, 136), 347 by 166) its bar with `FUN_0060a490(.., 7, 0x29fc, ..)`. RCDATA `0x29fc` (10748) is the Battle Alert's art (`list_scroll_bar::ALERT_ART`), 10 wide.
- **The arrows.** They are child buttons moved to `width / 2 - 13 / 2` (`FUN_0060fa80`), one column left of the bar, and the bar clips them.
- **The track and thumb.** They are copied into the bar's own buffer (`FUN_0060b3d0`, `FUN_005fcc30`).

## A0

The hashes are in `a0-sha256.txt`.
- **Alliance Forces** (`messages-day633/assault-coruscant-alliance-regiments`). The bar stands at the window's (374, 136). Its thumb runs from row 145 to row 201, 57 pixels: six 70-pixel rows, 425 with the list's margin, give 148 × 166 / 425. Its coloured columns start one right of the arrows'.
- **System Forces** ("None"). No bar.

## Now

- **The list.** `assault_summary` draws the bar with `list_scroll_bar::ui_bar`. The bar's pixels round to the list's whole rows, and its arrows, track and thumb scroll it.
- **The fixture.** `FleetAssaultTaken` carries six of the player's regiments, on copies of the fleet's ship enough to hold them, as A0's Alliance Forces.
- **`list_scroll_bar::pieces`.** It copies the track and the thumb from the bar's column 0 when `width / 2 - 13 / 2` is negative. This is a hyp: A0's thumb stands one column right of the arrows. For a 12-wide bar the offset is 0 and nothing moves.

## Tests

- **Unit tests.**
  - `the_forces_list_shows_its_scroll_bar_while_the_rows_overflow`: the arrows at x 373, the thumb at 374, the down arrow scrolls a row, and no bar for two rows;
  - `a_narrow_bars_thumb_starts_at_its_column_0_and_its_arrows_one_left`;
  - the fixture's assault test now expects six regiments aboard.
- **`fleet-window.mjs` `assault-taken`** (Linux, both sides). For the Alliance, the bar's 10 by 166 pixels match A0 exactly on Attackers, with the six regiments, and on System Forces, with none.
- **`run.mjs` `production-battle-alert-entry`.** The Battle Alert's thumb now sits at window x 329, its last three columns clipped. It passes on Linux.
