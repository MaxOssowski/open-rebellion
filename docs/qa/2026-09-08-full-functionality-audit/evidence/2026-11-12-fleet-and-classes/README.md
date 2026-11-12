# Fleet cargo, band menus, classes and message boxes (2026-11-12)

Cells: OBJ-10-C006 (aboard fleet), OBJ-14-C008 (manufacturing menus),
OBJ-08-C007 (available classes), CMD-09-C010 (advanced variants) and
CMD-11-C029 (message boxes). This stage has one port fix, one browser
check and a list of the A0 captures that are still missing.

## A0 menus3/34: the menu for nothing in a system window

`.artifacts/a0-vm/menus3/34-icon-menu-reserved.png` (SHA-256
`a3cd63cf1c0f3b5012132e745857878090e8b27775da1379034c23aa78c079b5`) was
not described before. It is the Empire Death Star game of
`2026-11-01-band-menus-a0`, day 573, right after `33`.

- The pointer is at (204, 211), on the left edge of Coruscant's system
  window (the window starts at x 203). This is left of the bands.
- No band is selected. All three band titles are the dim green
  (91, 179, 75).
- The menu is at (204, 211), 90 by 42. It lists Encyclopedia and Status.
  Both are grey.

The original does this:

- `FUN_004534f0` is the window procedure. On a press (0x201, 0x204) it
  looks for a band only inside the band rects of `FUN_00458480`:
  (55, 57)-(221, 136) and the two below it. A press off the bands selects
  nothing and clears nothing.
- It passes every right release (0x205) to the base procedure. The base
  procedure calls slot 7, `FUN_004ac5c0`. That function opens the pop-up
  for slot 22's selection (`FUN_00453ee0`): the selected bands on the
  overview, or the list's selected objects on a facility page.
- With nothing selected, the menu has only Encyclopedia and Status
  (`object-popup-menu.md`). Both are disabled.

The port opened a menu only over a band or over a facility page's list.
A right click elsewhere on the window did nothing.

## The fix

`system_window.rs`: a right release on the window's own surface, off the
title buttons, tabs, bands and list, now opens the pop-up. The menu is
for the window's selection (`surface_menu_object`): the selected band's
manager on the overview, the selected item on a facility page, or
nothing.

The unit test
`a_right_click_off_the_bands_opens_the_menu_for_the_selection_or_nothing`
checks three cases:

- nothing selected: the menu for nothing, and still no selection;
- the training band selected: its manager's menu, and the band stays
  selected;
- a shipyard page with its yard selected, clicked left of the list: the
  yard's menu.

The test fails without the fix (the window gives no action) and passes
with it. `cargo test -p rebellion-render --lib`: 1477 passed.

## Browser check

`status-window.mjs`, case `production-manager`, both sides, Linux
Chromium. Before it opens the band's Status, the case right-clicks 35
pixels left of the first band, with no band selected. It asserts the
menu is 90 by 42 and lists Encyclopedia and Status, both disabled, as
A0 `34`. The screenshot is `off-band-menu.png`.

Run `status-window-2026-11-12T02-00-22-229Z-3381868`: 26 of 26 cases
pass (Chromium 155.0.8059.84, test WASM
`f4a29c95da5d5b403bbb54f33c10b9e2d6622c305c7335dc44d07d968b6db4b0`).
On both sides the menu is at (245, 153), 90 by 42, with Encyclopedia
and Status disabled.

The menu's background is translucent, so the pixels depend on what is
under it. The text uses the accepted font deviation. So the check
compares size, rows and states, as the other `a0Menu` checks in this
gate do.

## OBJ-10-C006: units aboard a fleet

`fleet-window.mjs` passes in full on Linux Chromium with the fix above:
run `fleet-window-2026-11-12T02-00-22-233Z-3381876`, 88 of 88 cases,
the same test WASM. Its case
`a0-coruscant` compares A0 `fleet-aboard/00..12` (Coruscant's Fleet 1:
six TIE Fighter squadrons, three Stormtrooper Regiments and Veers aboard)
tab by tab, and the right list's scroll bar pixel for pixel.

Special forces aboard a fleet still have no A0 capture. Needed:

1. In an Alliance game, a system with an Alliance fleet and a special
   force (for example Bothan Spies trained at Yavin, or a starting unit).
2. Right-click the special force in the System Defenses window's
   Personnel page, choose Move, and release on the fleet (walk the
   pointer there, rest 0.9 s, hold the button 0.4 s).
3. Open the Fleet window. Capture the fleet's Personnel tab, then the
   ship's Personnel tab with the ship selected.
4. Capture the special force's right-click menu in that tab, its Status
   ("Attached:" names the fleet), and the Personnel Finder's SpecForces
   view, where the fleet should be a row of its own.

## OBJ-14-C008: manufacturing menus

The band menus were compared with A0 `menus3/30..33` before (rows,
order, enabled states, the check mark, 107 by 122). This stage adds the
window's menu off the bands (`34`). The Reserved order for every manager
from the facility icon is ported (2026-11-10). The player agent's Manage
Production and Manage Garrisons skip Reserved yards (2026-11-11, unit
tests). That is agent behaviour with nothing on screen to capture.

## OBJ-08-C007 and CMD-09-C010

Both cells are checked in `build-selection.mjs`. Another agent works on
that gate tonight, so this stage did not touch it.

- OBJ-08-C007: Confirm's state per class is still not asserted. A0
  `build-selection*/` has no Confirm state per class beyond `15-t01`.
- CMD-09-C010: there is no A0 of a finished advanced base. Needed: in a
  game late enough to research them, a finished Advanced Shipyard,
  Advanced Training Facility and Advanced Construction Yard. For each,
  open the system window on its facility page and capture it, for both
  sides (STRATEGY 9010, 9018, 9026 and their side offsets, as
  `FUN_00458fe0` picks them).

## CMD-11-C029: message boxes

- To Cockpit (REBDLOG 0x190e) and Acknowledged (0x190f) appear only in
  multiplayer boxes. Callers: `FUN_00413560` (0x1204 "Your Opponent Has
  Left The Game"), `FUN_0040ec60` (`MultiPlayerWindow::DoCancelMulti`,
  0x1205 "Your Opponent Has Left This Session"), `FUN_00410dd0` and
  `FUN_00410c70` (0x1208, a DirectPlay provider failed). Multiplayer is
  out of scope (user decision 2026-10-30). These two buttons cannot be
  reached in a single-player game.
- The save notice (`FUN_00408240` -> `FUN_00417020`: REBDLOG 10622 with
  0x1217 "Selected Game Saved Successfully" or 0x1219 "Selected Game
  Failed To Save", one button with the tip 0x1300 "Yes") has no A0.
  The port draws it (`game_options.rs`, `rebdlog_dialog.rs`). Needed: in
  a game, Game Options, Save Game to a slot, then capture the notice, and
  again with the pointer resting on its button at (176, 134) until the
  tip shows. The failed notice needs a save that fails, for example a
  read-only save file (`attrib +r` from a Win+R command prompt).

## Batch b195

Full gate batch b195 (2026-11-12 05:18) is green: 30 gates, check, workspace
(3605) and fixture (157) tests with 0 failures, 168 battle PASS (42
scenarios). Passed: CMD-02-C002, CMD-02-C003, TAC-07-C014, OBJ-14-C008.
