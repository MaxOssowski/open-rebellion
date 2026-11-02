# A press raises a window, against A0 (2026-11-02)

Surface ledger CMD-04 C005, "raised".

## A0

The A0 run used the Windows 10 VM and the Empire's "resource turn 14" save, day 192. The captures are in `.artifacts/a0-vm/raise/`, with hashes in `a0-sha256.txt`.

- **`01-coruscant-front`.** Coruscant's Defenses window was moved left over Yavin's Manufacturing window and stands in front of it, its title strip green.
- **`02-yavin-raised`.** A press on Yavin's title strip at (500, 101) brings Yavin's window in front. Its whole frame is drawn over Coruscant's right edge, which it now covers.

## The port

`fleet-window.mjs`'s move case (Arch Chromium 155, Linux) checks the same rule. A press on a window's surface or title brings it to the front (`2026-10-28-window-move`).
