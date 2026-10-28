# Fleet, Defenses and Missions windows: list, count and label fonts (P06, P09)

Date: 2026-10-28. The reading is in `ghidra/notes/fleet-window.md`, "List
and label fonts".

## The original

- The Fleet window's lists and counts are font 10 (`FUN_004a4b10`).
- Its selected label is font 5, left and top aligned at (100, 29).
- The System Defenses list is font 10 and its tab names are font 4
  (`FUN_004a8790`).
- The Missions window's lists are font 10 (`FUN_0049f540`,
  `FUN_004a1ba0`).

A0, Windows 10 VM, original `REBEXE.EXE`, Averam's Fleet window with Fleet 2
selected:

| Capture (`.artifacts/a0-vm/2026-10-28-nav/`, ignored) | SHA-256 |
|---|---|
| `fleet-averam.png` | `db7c6159215ab329c2ac7f01fd12070fd47d0e909b37869c3cae96f1204d8ade` |
| `fleet-selected.png` | `5f104ce2e91e9f3fb1876e1789b1bce1403f92778583c86f03ba3f7e6b1fbe36` |

The label "Fleet 2" is red and bold from (101, 31). The ship's name under
its picture has a 9-row capital, which fits font 10.

## The port

All of these were proportional fonts of 9 or 10 pixels. They are now
`theme::original_font(10)` for the lists and counts, `original_font(4)`
for the Defenses tab names, and `original_font(5)` for the Fleet window's
label. The label was centred on x 164 and is now at (100, 29), left and top
aligned. Its side colour stays, and the A0 capture shows it red for the
Alliance.

## Tests

The windows' text tests now expect these fonts instead of the port's sizes
(18 and 20 at scale 2), and the label at (100, 29), left-aligned.

## Browser probe changes

- `fleet-window.mjs`: the selected-name mask now covers the label's own
  rect, (100, 29) 128 by 25 (`FUN_004a4b10`), instead of a 13-row band.
  The font 5 label reaches below that band.
- `sector-quadrants.mjs`: a mission row's text band, which the mini
  comparison skips, is now font 10's 14-row cell instead of 12 rows. The
  list draws the name over the picture from the row's top.

## Gates

- Workspace: 3034 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff): 3 mutants, 3 caught.
- Browser:
  - `fleet-window.mjs`: 60 of 60 (run
    `fleet-window-2026-10-28T18-05-30-368Z-3675657`);
  - `sector-quadrants.mjs`: 12 of 12 (run
    `sector-quadrants-2026-10-28T18-04-55-857Z-3673367`);
  - GID: 152 of 152 (run `2026-10-28T18-09-25-786Z-3688409`).
- rustfmt: only the files' baseline differences remain.
- Release WASM SHA-256 `632f85ab8f89a34d8e306261b2b0d6f8d73fac02dcc4f1d4b2f45526ceeb18ad`.
