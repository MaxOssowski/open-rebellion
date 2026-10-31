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
- **Unexplained.** The advisor button at (0, 355) is lit in the port and
  unlit in A0, a difference of 807 pixels.

## Legend tip

`tooltip.mjs` checks the legend button's tip in the Alliance Popular
Support fixture against A0 `tooltip/360-legend.png` (a copy of
`gid-displays/all-legend-hover.png`): box (70, 86), 19 pixels high.
