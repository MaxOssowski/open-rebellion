# The empty save list against A0 (2026-11-05)

This covers PRE-03-C001 ("save list empty").

## A0

The Windows 10 VM: the six `SAVEGAME.00n` files moved aside, a fresh launch, and Load/Options from the main menu (`.artifacts/a0-vm/game-options-empty/00-empty-save-list.png`). The saves were then moved back.
- **The rows.** All six are empty, with no name and no crest. Each row's Load button shows its disabled art.
- **The Save buttons.** They are disabled as well, because the main menu's window has no game to save (`FUN_00407180`).

## Tests

`game-options.mjs` (Linux) asserts six rows with no name and no crest. It now compares the list's area (78..335, 75..315) with A0's, every pixel, and finds 0 differing: the name and crest frames and the Load buttons.

The Save buttons (34..75) are left out. The gate opens the window from a game, where Save is enabled. A0's comes from the main menu, where it is disabled. The port follows the same rule by origin (`game_options.rs`: Save disabled outside the command center).

## The Exit question (PRE-03-C014)

- **A0** (`game-options-exit/00-quit-question.png`). Exit in the main menu's Game Options asks "Are you sure you want to quit?" with the check and the X.
- **`game-options.mjs`.** Exit's question in the port matches A0's dialog pixel for pixel outside the question line and A0's pointer, 0 differing. The line's ink box is (223, 215)..(414, 227) against A0's (222, 215)..(414, 227). No keeps the window.

## Restart from the main menu (PRE-03-C012)

- **A0** (`game-options-restart/00-after-restart.png`). Restart in the main menu's Game Options of a fresh launch goes straight to the shuttle cockpit, with no question. This is `FUN_00406b80` `0x91` with no game in progress.
- **`main-menu-states.mjs`.** Restart from the port's cockpit context returns to the menu with no question. 272,823 compared pixels equal A0's, 0 differing. Left out: the animated buttons, the game type label, the extension chips, the pointer, and the galaxy size lever and lights. The gate's earlier steps leave the port at another size than A0's fresh Standard.

## Load from the main menu (PRE-03-C004)

- **A0** (`load-from-menu/00`, `01`). Load on the first save in the main menu's Game Options opens the Alliance game at once, with no question.
- **The port.** It asks "Load without saving?" only while a campaign is live (`GameOptionsState::request`: Load confirms when `origin.has_live_campaign()`, `FUN_00406b80`).
- **`game-options.mjs`.** After the in-game Load Yes, the gate presses F1, Restart (and Yes) back to the shuttle cockpit, then Load/Options (431, 250) and Load on slot 0. One click logs `[campaign] restored slot=0`, with no question asked.
