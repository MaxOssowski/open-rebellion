# A new Alliance game and the GID legend against A0

Date: 2026-10-31.

## A0

From the VM's main menu, Alliance was pressed and the screen was captured
every 3 seconds (`.artifacts/a0-vm/new-alliance/t00..t29`). The opening
movie plays, the command centre appears (`t18`), and the agent's advice
opens the Message Index 3 s later (Easy difficulty).

`.artifacts/a0-vm/gid-displays` holds:
- the Empire game's Alt+1..9 displays;
- a hover and a click on the Alliance legend button: the tip "Legend"
  shows, and the click opened nothing visible.

The hashes are in `a0-captures.sha256`.

## The legend: finding and fix

A0 shows only the legend button (STRATEGY 10168) after every display
chosen from the menu (`gid/d-*.png`) or by Alt+1..9. It also shows the
button on a new game's first frame (`t18`, Display Off, before any display
was chosen).

`FUN_00426d00` builds the legend window, then puts it at the bottom of the
z-order (`SetWindowPos(.., HWND_BOTTOM, ..)`), under the galaxy view. It
hides the button only for Display Off.

The port opened the legend on every display choice, and hid the button
whenever the display was Display Off. It now:
- leaves the legend closed on a choice, with the button shown;
- hides the button only through `show_display(DisplayOff)`, so a new game
  shows it.

The legend's own window opens from the button, as before. The interface
fixtures now set their display as the menu does, so `display-off` hides
the button as A0 does. Its button region matches A0's `d-display-off`
exactly.

The GID gate (`run.mjs`) now checks that a chosen display leaves the
legend closed, then opens it with the button. All 152 cases pass on Linux.

## First frame of a new Alliance game

A0 `t18` against the port's `briefing.mjs` `first-voice` frame (Linux, the
release WASM with the fix): 2,696 of 307,200 pixels differ.
- **Identical.** The shell, the galaxy, the legend button, the day field
  and the time bar (both blank).
- **Different.** C-3PO and R2-D2's animation frames, the maintenance
  figure (169 against 141, game state) and A0's pointer.
- **Globe (fixed).** The control at (3, 355) is the Game Options globe,
  not the advisor. `FUN_00427270` passes it its normal art first: 0x271e
  (10014) for the Alliance and 0x272c (10028) for the Empire. A0 shows
  10014 at rest here and in `2026-10-28-nav/galaxy-a.png`. The port had
  normal and pressed swapped on both sides; it now uses the constructor's
  order. After the fix the globe differs from A0 by 1 pixel, and the frame
  by 2,567 pixels in all: the droids' frames, the maintenance figure and
  A0's pointer. The Alliance galaxy comparison against `galaxy-a.png` drops
  from about 5,300 to 3,350 differing pixels.

## Legend tip

`tooltip.mjs` checks the legend button's tip in the Alliance Popular
Support fixture against A0 `tooltip/360-legend.png` (a copy of
`gid-displays/all-legend-hover.png`): box (70, 86), 19 pixels high.

## The opening advice's order

A0 `t19..t29`: on a new Easy game the Message Index opens on Advice. It
lists the nine kind-7 advice with Victory Conditions on top and Advice
Available at the bottom. The port had them the other way round.
- `FUN_00439f20` posts the kind-7 nodes walking from the list's last node
  back by predecessors (`thunk_FUN_005f5080`, `FUN_005f5c60`), so key 90
  posts first and key 10 last.
- The index shows the newest post first (`FUN_005f5440` keys each message
  with the list's running count).

`rebellion_core::agent_advice::AgentAdvice::build` now returns them in
that order. Its unit tests and `agent-advice.mjs` (Linux, both sides)
expect 9 down to 1. The port's `advice-index` frame now lists the rows in
A0's order. Its differences from `t25` are the rows' text (the accepted
deviation), the droids, and the day and time bar (A0 had run to day 2).
