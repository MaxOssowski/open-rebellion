# The Message Index's posting toggle against A0 (2026-11-03)

Surface ledger CMD-08 C005, "alert".

## A0

The A0 run used the Alliance's DS_flip_allian save, day 301, with the Fleet Messages category open. The captures are in `.artifacts/a0-vm/personnel-states/`, with hashes in `a0-sha256.txt`.

- **`30-posting-toggle-alert`.** The posting toggle at the window's (423, 147), screen (485, 196), shows STRATEGY 10872 (`0x2a78`): 1 of its 992 pixels differs.
- **`30-posting-toggle-silent`.** After a click, the category posts silently and the toggle shows 10951 (`0x2ac7`). Its interior matches exactly. Its 1-pixel border, (35, 36, 35), is not drawn: the rail's metal shows there, 125 pixels.

## The source

`FUN_00602d30` blits button art keyed on the DIB's first pixel (`FUN_005fd170`). That is the bottom-left pixel of a bottom-up DIB.

- For 10872 the first pixel is blue (0, 0, 255).
- For 10951 and 10873 it is (35, 36, 35), the border's colour (`ghidra/notes/keyed-blit.md`).

## The port

Before, the port keyed 10951 and 10873 on blue, so 10951's border was drawn. Both now key on their first pixel (`uses_first_pixel_key`).

Test: `the_alliance_posting_toggles_silent_art_is_keyed_on_its_first_pixel`. The blue-keyed list test no longer lists 10951.

The toggle's behaviour was already checked in the browser by `advisor-events.mjs` and the Message Index cases: it flips the category's silent bit, and the alerted droid comment plays.
