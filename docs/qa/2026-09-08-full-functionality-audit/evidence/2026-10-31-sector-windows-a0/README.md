# Sector windows against A0 (CMD-04)

Date: 2026-10-31.

## A0

The session ran in the VM's new Alliance game, on Popular Support (day 3
to 4; `.artifacts/a0-vm/sector-windows`, hashes in `a0-captures.sha256`):

1. A system in Sesswenna (252, 162) opens its sector window on the left
   half, at (60, 35), 235 by 360.
2. A system in Sumitra (470, 240) opens on the right half, at (300, 35).
3. Sesswenna's side switch (270, 44) moves it to the right half. Sumitra,
   which held that half, closes.
4. A system in Churba (115, 215) opens on the freed left half.

## Frame finding and fix

A0's frames are not the port's. `FUN_00459e30` frames the window with
`FUN_00607740(0x2779, 0x277a, 0x277b, 0x2778, 0x2775, 0x2774, 0x2777,
0x2776)`, the same call the GID's menus and legend use:
- **Corners.** 10101 top left, 10100 top right, 10103 bottom left, 10102
  bottom right.
- **Top and bottom.** 10105 and 10106 (1 by 2), in every column.
- **Sides.** 10107 left and 10104 right (2 by 1), on every second row.

`cockpit::frame_tiles` already drew that frame for the menus (checked
against A0 on 2026-10-30). The sector window had its own border, with the
corners and edges taken from the wrong tiles: its top and sides were a
grey-white checker. It now uses `cockpit::paint_gid_frame_border`. The GID
legend had the right corners, top and bottom, but stretched its side
tiles into solid lines; its sides now go on every second row.

## Gate

`tools/interface-parity/sector-windows.mjs` replays the session in the
Alliance Popular Support fixture on Linux (`sector-windows-result.json`).
It passes all four steps:
- **Exact regions.** Each window's buttons, its four frame edges and its
  place match A0 pixel for pixel. Left out are the buttons' keyed corner
  and the rows between the side tiles, where the shaded galaxy (each
  game's state) shows.
- **Title.** Its ink is within 1 pixel at the top, the bottom and the
  centre. The width is within the accepted text deviation.
- **Empty half.** Where A0 has no window, the port has none either.

`sector-quadrants.mjs` and the GID gate's popular-support cases still
pass.
