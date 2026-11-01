# A disabled menu row against A0 (2026-11-01)

## A0

`status/02-menu.png` (an enemy ship's menu) draws Move, Confirmed Move,
Create Fleet, Rename and Scrap disabled. Their pixels are (128, 128, 128),
the colour `0x2808080` `FUN_004aba60` gives a disabled item; the enabled
Encyclopedia and Status are (255, 255, 255). Hashes in `a0-sha256.txt`.

## Port

`game_menu.rs` draws a disabled row in `0x808080`. The `abort` case of
`personnel-orders.mjs` opens the primary system's mission icon menu while
the missions are only ordered, so Abort is disabled
(`FUN_00521600`). It now also reads the screenshot: the Abort row's text
band has no pixel brighter than 128 and at least ten at exactly
(128, 128, 128); the Encyclopedia row has white. Both sides pass
(`gate-personnel-orders.txt`, screenshots in `port-sha256.txt`).

The first version of the check counted the bottom frame as part of the
row (white 83); it now reads the 16-pixel text line 2 pixels into the
20-pixel row.

Accepted deviation: the glyphs are anti-aliased, so only their cores reach
128 where GDI's whole glyph is 128.
