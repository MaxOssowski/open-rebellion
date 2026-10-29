# The Agent menu's nine items and its Objectives window (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy", the Agent menu. The
readings are in:
- `ghidra/notes/agent-menu.md`, "Which items show";
- `ghidra/notes/status-window.md`, "Objectives".

## The original

- **The menu.** `FUN_00487900` builds the Agent menu's list. Each
  `FUN_00520690` adds an item; it does not only enable one. It adds nine:
  - Build Ships, Build Troops, Build Facilities;
  - Galaxy Overview, Objectives;
  - Manage Garrisons, Manage Production;
  - Translate Counterpart, Agent Advice.

  `FUN_0042d050` shows only those, in their records' sort order. Manage
  Maintenance, Not An Operation, Deactivate, Reactivate and Messages never
  show.
- **Objectives** (`0x114`, Alt+H) opens the Status window on the empty id
  (`FUN_004ece30` → `FUN_0041d7f0` → `FUN_0042a440`). `FUN_00443130` takes
  that id down its own branch:
  - the picture is STRATEGY 11590;
  - the name is TEXTSTRA 12564 "Objectives";
  - the Encyclopedia button is disabled;
  - the list is filled by `FUN_0044b2d0` (viewer side 1) or `FUN_0044bb70`.
- **The rows.** Each filler reads the viewer's victory conditions word and
  the game type, and lists:
  - Coruscant and the headquarters;
  - in the standard game also the four leaders: Palpatine, Vader, Luke
    and Mon Mothma.

  Each row is an icon centred in a 37 by 37 cell plus a text that changes
  once its condition is met. The keys put each side's own objectives
  first.
- **Placement.** `FUN_00606980` places the window at `l + (r - l - w) / 2`,
  truncating.

## A0 (original game, Windows 10 VM)

These are Empire captures from 2026-10-29. They stay outside Git; their
hashes are in `a0-captures.sha256`.

| Capture | Day | Shows |
|---|---|---|
| `empire-agent-menu-day137.png` | 137 | the nine items, two of them checked |
| `empire-objectives-day137.png` | 137 | Objectives: the window at (169, 81), six rows from Destroy Rebel HQ to Defend Vader |
| `empire-galaxy-overview-day139.png` | 139 | Galaxy Overview, for the next stage |

## The port

- `agent_menu.rs`:
  - only the nine items;
  - Objectives is always enabled, as `FUN_00487900` sets it.
  - port: the build orders, Galaxy Overview and Translate Counterpart stay
    disabled until they are ported.
- `status_rows.rs`:
  - `StatusObject::Objectives`;
  - `objective_rows`, from `VictoryState::conditions_word` and the
    campaign's `VictoryConditions`;
  - the backgrounds 11554 (Alliance) and 11558 (Empire).
- `status_window.rs`:
  - one column of 35-pixel rows;
  - each icon centred in a 37 by 37 cell, one pixel in from the list's
    left;
  - the text at x 40 from that cell.
  - The Encyclopedia button draws 11612.
  - Every Status window now uses the original fonts: font 5 for the title
    and name, font 10 for the list. The name is drawn from its box's top
    (format `0x11`).
- `mission_dialog.rs`: `galaxy_centered_rect` follows `FUN_00606980`'s
  truncating halves. Every window placed in the galaxy view moves by at
  most one pixel.
- `main.rs`: the menu's Objectives and Alt+H open the window.

## Browser acceptance (`tools/interface-parity/agent-objectives.mjs`)

Run `agent-objectives-2026-10-29T10-04-56-462Z` passed for both sides.
The WASM hash is `efb36faa…` (`browser-result.json`). For each side the
test:
1. starts a new game and skips the briefing;
2. right-clicks the droid, then clicks the menu's Objectives row
   (`source=menu`);
3. closes the window with Escape;
4. presses Alt+H (`source=accelerator`).

No console errors appeared, and every request returned below 400.

Compared with the A0 captures by eye and by measurement:
- The Empire's window is at (169, 81), the same as A0.
- The rows, icons, texts, order and the name match.
- Remaining differences:
  - port: no scroll bar;
  - glyph anti-aliasing;
  - in the menu, the port's Objectives row sits at y 307, A0's at 311,
    because the shared menu's layout is not this stage's.

Pictures: `{alliance,empire}-agent-menu.png`,
`{alliance,empire}-objectives.png`.

## Tests

- `cargo test --workspace`: all pass.
- New tests:
  - `the_menu_holds_the_nine_items_fun_00487900_adds_in_sort_order`. It
    fails against the old list (14 items), as shown by removing the
    filter;
  - `each_side_lists_its_own_objectives_first`;
  - `a_met_condition_changes_its_text_on_both_sides`;
  - `a_headquarters_only_game_lists_no_leaders`;
  - `the_objectives_window_shows_the_galaxy_picture_and_no_encyclopedia`;
  - `an_objective_icon_is_centred_in_the_first_icons_37_pixel_canvas`;
  - `the_objectives_rows_stack_35_apart_with_centred_icons_and_text_at_40`;
  - `a_galaxy_window_is_placed_with_truncated_halves`.
- Changed expectation:
  `the_title_name_and_rows_sit_where_the_window_lays_them_out` now
  expects:
  - the original fonts, where it had the stand-in sizes 22 and 18;
  - the name at its box's top (format `0x11` has no `DT_VCENTER`; A0
    agrees).
- `cargo mutants --in-place --in-diff` on the render changes:
  - first run: 96 mutants, 30 missed, all in the new row drawing;
  - after the drawing test, `draw_status_window` again: 40 mutants, 39
    caught, 1 unviable.
- Clippy on the touched files adds no warning. The unreachable 10234 key
  was removed: it is already keyed as a sector planet.
