# The whole ten-system Moddell window against A0

Date: 2026-11-09. Covers CMD-03-C001 (ten-system sector). Earlier
evidence matched this window's parts one by one: its title, names,
tracks, markers and shading. This compares the whole window from one
capture.

## A0

`.artifacts/a0-vm/2026-10-28-nav/sector-a.png` (SHA-256 in `SHA256SUMS`):
a new Alliance game, day 8, with the Moddell sector window on the left
half (x 60..294, y 35..394).

## The browser case

`sector-windows.mjs` now ends with a Moddell step. It uses the Alliance
Popular Support fixture (Linux Chromium, 640x480), opens Adega through
the System Finder (F2), and compares the window with A0:

| Part | Result |
| --- | --- |
| frame, side switch and close box | 0 pixels differ |
| title "Moddell" | same ink box as A0 |
| pictures of Agrilat, Basilisk, Khuiumin, Annaj, Pzob, Hozrel XI, Gandolo IV | 0 pixels differ |
| Adega left of A0's cursor, Endor right of the markers behind its edge | 0 pixels differ |
| nine white names | centred within 1 pixel, tops within 1, widths within 7 % |
| the shaded galaxy behind | 15 differing patches, 373 pixels, the largest 15 by 20 |

The gate fails on any patch larger than a marker (16 by 20). A wrong
shade or a misplaced background would make one large patch.

## What differs, and why

The fixture is another game, so some of what the window shows differs:

- Vjun: in A0 it is the Empire's, with its fleet, its tracks and the
  headquarters mark. In the fixture it is an unexplored neutral system.
  That state's art was matched against A0 in
  `2026-10-28-sector-names-and-marks` and
  `2026-10-28-sector-planet-markers`.
- A0's mouse cursor rests on Adega.
- The galaxy markers seen through the shaded background belong to each
  game's own fleets and systems.
- The names' glyphs differ in anti-aliasing (Liberation for Arial), the
  text deviation the owner accepted on 2026-10-30.

Port capture of the passing run: SHA-256
`6d18a99cb0b085207ada6f8c5a57b00d59335bd545416fefcfc31f85e92b618a`.
The run was on the b152 interface-test build, and only the gate changed.
