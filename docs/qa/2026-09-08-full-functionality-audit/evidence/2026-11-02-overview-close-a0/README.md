# Closing the Galaxy Overview, against A0 (2026-11-02)

Surface ledger CMD-06 C005, "close".

## A0

The A0 run used the Windows 10 VM and the Alliance's DS_flip_allian save, day 179. The captures are in `.artifacts/a0-vm/overview2/`, with hashes in `a0-sha256.txt`.

1. A right click on C-3PO opens his menu (`01`).
2. Galaxy Overview opens the overview at (108, 79) (`02`).
3. The right-hand button at the window's (324, 218) is the close button 10370. The pointer rests on it (`03`), and a click closes the window (`04`).

## The port

`agent-objectives.mjs` now closes the Alliance's overview with the same button, at the window's (324, 218) in the window at (108, 79), and waits for the close. The Empire's run keeps Escape. Run `agent-objectives-2026-11-02T17-34-07-156Z`: pass.

`status-window.mjs` compares the button's art (10370) with the bitmap in its other cases.
