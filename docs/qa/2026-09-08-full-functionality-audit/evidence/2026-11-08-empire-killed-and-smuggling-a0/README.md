# The Empire's Character Killed and Smuggling Benefits against A0

Date: 2026-11-08. Adds the Empire's side to EVT-01-C016 and EVT-01-C003,
which pass for the Alliance since b140 and b139.

## A0

A new Empire game in the Windows 10 VM (`.artifacts/a0-vm/empire-game3/`).

| File | SHA-256 | What it shows |
|---|---|---|
| `jerjerrod-killed.png` | `5d5c78c88a519d8447b1469b77b93ecf79f8ed8a891712712b1433aa3d6efa2f` | day 682: "Jerjerrod has been killed." over the Empire's coffin `0x433` |
| `smuggling-benefits.png` | `6f6432b089ba0268a9224ad67ae8b9bc551cadb6594d78524c68af88c0dde3bf` | day 681: "Smugglers from Coruscant are providing us with additional resources." over `0x3ec` |

The Coruscant message came after the Alliance had seized Coruscant. The
game later ended (ending movie, back to the main menu) and was not saved.

## Port

The fixture now gives each reader its own case:

- **Killed:** Jan Dodonna for the Alliance reader, Jerjerrod for the
  Empire's (`killed_character`).
- **Smuggling:** the other side's system, Yavin held by the Empire for the
  Alliance reader, and Coruscant held by the Alliance for the Empire's
  (`smuggled_system`, `smuggling_message`).

`message-display.mjs` (Linux, Empire) checks the exact text of both. The
picture, title image and rail are each 0 pixels from A0.
