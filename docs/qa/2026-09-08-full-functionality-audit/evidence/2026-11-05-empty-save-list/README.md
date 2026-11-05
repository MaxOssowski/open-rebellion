# The empty save list against A0 (2026-11-05)

This covers PRE-03-C001 ("save list empty").

## A0

The Windows 10 VM: the six `SAVEGAME.00n` files moved aside, a fresh launch, and Load/Options from the main menu (`.artifacts/a0-vm/game-options-empty/00-empty-save-list.png`). The saves were then moved back.
- **The rows.** All six are empty, with no name and no crest. Each row's Load button shows its disabled art.
- **The Save buttons.** They are disabled as well, because the main menu's window has no game to save (`FUN_00407180`).

## Tests

`game-options.mjs` (Linux) asserts six rows with no name and no crest. It now compares the list's area (78..335, 75..315) with A0's, every pixel, and finds 0 differing: the name and crest frames and the Load buttons.

The Save buttons (34..75) are left out. The gate opens the window from a game, where Save is enabled. A0's comes from the main menu, where it is disabled. The port follows the same rule by origin (`game_options.rs`: Save disabled outside the command center).
