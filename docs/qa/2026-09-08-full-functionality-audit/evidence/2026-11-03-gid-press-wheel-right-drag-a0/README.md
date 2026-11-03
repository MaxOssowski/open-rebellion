# The galaxy view: press and release, wheel, right drag (2026-11-03)

Surface ledger CMD-02 C027 (selection), C028 (fixed-map right-drag no-op) and C029 (fixed-map wheel no-op).

## A0

The A0 run used the Alliance's DS_flip_allian save, days 229 and 230, with no window open over the galaxy. The captures are in `.artifacts/a0-vm/gid-selection/`, with hashes in `a0-sha256.txt`.

- **`01-held`, `02-released`.** A left press on the Moddell cluster at (155, 123), held for a second, shows only the hover label "Moddell Sector". The release opens Moddell's sector window.
- **`03`, `04`.** Three wheel-up and five wheel-down notches (QMP `input-send-event`, `wheel-up` and `wheel-down`), with the pointer at (330, 230). The galaxy view's (60..540, 60..395) differs in 0 pixels before and after.
- **`05`, `06`.** A right press at (300, 200), dragged to (420, 280) and released. The same region differs in 0 pixels; no menu and no pan.

This matches `FUN_00422ce0`: it has no wheel case and no right-button drag, and it opens the sector on `WM_LBUTTONUP` (`FUN_00429ce0`).

## The port

`run.mjs` (Linux, both sides, 640 by 480 and 1280 by 800 letterboxed):

- **`pan`, `zoom`.** A right drag and a wheel notch leave the frame byte-identical. This was already gated.
- **`hover`.** New: after the hover label, a left press held over the probe system leaves the frame byte-identical to the hover frame, and the release opens the sector, so the frame changes.

Run `2026-11-03T09-58-25-989Z-2519713` (`--scenario=hover`, four views): pass. `system-released.png` shows the Sluis sector opened.
