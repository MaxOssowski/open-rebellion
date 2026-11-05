# The main menu's Quit asks first (2026-11-05)

This covers PRE-02-C014 ("exit").

## A0

The Windows 10 VM, the main menu: `.artifacts/a0-vm/main-menu-quit/q0..q5`, a screen every 0.6 s after Quit. The hashes are in `a0-sha256.txt`.
- **The question.** Quit shows REBDLOG 10623 at (114, 152) with "Are you sure you want to quit?", the check and the X.
- **The cockpit.** Its buttons go on animating behind the dialog.
- **The palette.** The dialog's colours are 10623's mapped to their nearest entries of COMMON 20001's palette, entry 254 white. Outside the text band and the buttons, all of its pixels match that mapping.

## The original

`FUN_00414330`, the application's state switch, shows the quit question with `FUN_00401110(0x470, 0, 0x1202)`. Game Options' Exit asks the same 0x1202.

## Before

The port's Quit left at once (`MainMenuAction::Quit`).

## Now

- **The dialog.** It is drawn by a new module, `rebdlog_dialog`, taken out of Game Options unchanged. It takes the screen's realized palette (`rebdlog_dialog::realized_palette`):
  - Game Options passes COMMON 20002's, as before;
  - the main menu passes COMMON 20001's.
- **`MainMenuState`.** It keeps `quit_question`. Quit opens it, and the cockpit takes no click while it shows.
  - Yes, or Enter, quits.
  - No, or Escape, closes it.

## Tests

- **Unit tests.**
  - `quit_asks_first_and_yes_quits`. It fails with Quit leaving at once.
  - `the_realized_palette_is_the_bitmaps_table_with_entry_254_white`.
- **`main-menu-states.mjs`** (Linux).
  - Quit from the menu shows the question. Against A0 `q0`, the dialog's frame matches pixel for pixel, 0 differing, and so do its two buttons. The line's ink is (223, 215)..(414, 227) against A0's (222, 215)..(414, 227).
  - No keeps the menu.
- **`game-options.mjs`** (Linux). It still matches A0's Restart and Exit questions after the move.

Mutants:
- On `realized_palette`, `activate` and `quit_question_open`: 10 caught, 2 unviable.
- In the batch's in-diff run, one survived: `/` to `%` in the question's scale (`canvas.width() / LOGICAL_WIDTH`). The unit test draws the fallback buttons, for want of the bitmaps. `main-menu-states.mjs` covers the scale against A0, pixel for pixel.

## Return from an Alliance game (PRE-03-C013)

- **A0** (`game-options-alliance/00`, `01`). An Alliance save loaded from the menu, then F1. The Return button is the Alliance frame 10020 at (162, 382), with 0 of 1763 opaque pixels off. Return goes back to the galaxy as it was before F1 (`01`); only the pointer differs.
- **`game-options.mjs`.** A second page opens the Alliance fixture and presses F1. Its Return button matches A0 pixel for pixel, 0 differing. A click on it logs `[game_options] command=return origin=CommandCenter`, and the window's Saved Games strip is gone. The Empire's 10023 was already matched against A0 `tooltip/200`.
- **The gate's click.** The release must be read under the pointer on the next frame, so the gate waits 300 ms before moving the pointer away.
