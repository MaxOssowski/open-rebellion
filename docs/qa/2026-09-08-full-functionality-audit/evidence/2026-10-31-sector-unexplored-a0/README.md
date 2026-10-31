# Unexplored planets in the sector window against A0 (CMD-03)

Date: 2026-10-31.

## A0

The Churba window on the left half (`.artifacts/a0-vm/sector-windows/04-left.png`,
Popular Support display; hashes in `a0-captures.sha256`) holds ten
unexplored planets. Each shows its picture and a white name, with no
tracks, no quadrant icons and no galaxy marker.

The same holds for the unexplored planets of Sumitra
(`build-selection/05-closed`), Orus (`build-selection/07-sector`) and
Moddell (`2026-10-28-nav/sector-a`). There the player's systems (Yavin,
Kimanan, Vjun) and the explored ones (Sesswenna, `sector-b`) do carry
their marker at the picture's bottom left.

## Finding and fix

The port blitted the galaxy view's unexplored "+" (`GID_UNEXPLORED`) at the
bottom left of every unexplored planet. `FUN_0042b330` routes an unexplored
(or destroyed) system to `LAB_0042b4f5`, which sets the map item's flag
`0x80` and gives `FUN_0060bd20` bitmap 0. The galaxy view draws the mark
from the flag (`FUN_0042cde0`), but `FUN_0042d860` returns the empty bitmap
`+0x20`, so `FUN_0045bbb0` adds nothing to the sector window
(`ghidra/notes/sector-quadrants.md`).

The galaxy view now hands the sector window only the items that hold a
bitmap (`gid_item_bitmap`). The galaxy itself still draws every mark.

## Gate

`sector-windows.mjs` (Linux Chromium) compares each Churba planet's 37 by
37 picture and the 4 rows below it with A0, pixel for pixel. These rows are
where the marker would go. Six planets lie clear of the galaxy's own
markers behind the window, which show each game's state: Lelmra, Deyer,
Gentes, Allyuen, Hoth and Storthus.
- **Without the fix:** each planet differs in 9 pixels, the "+".
- **With the fix:** each differs in none.

The ten names are asserted white, with their centre within 2.5 pixels and
their top and bottom within 1 pixel of A0's.

Their widths are only recorded, at 1.10 to 1.26 times A0's. This is more
than the roughly 9 % text deviation accepted on 2026-10-30, so it is
reported as an open text-rendering finding.
- A0 draws font 10 (Arial, `lfHeight` 14) as GDI's hinted, unsmoothed
  glyphs. They match FreeType's Liberation Sans at 11 ppem in width and
  cap height (8 pixels).
- The port draws em 12.53, antialiased, with 9-pixel capitals.
- The title (font 4) behaves the same way: A0 is 39 pixels wide, close to
  Liberation at 12 ppem; the port's is 46.

Not asserted: the tracks and icons around each planet. The galaxy markers
behind those areas differ between the two games; the diff image shows
nothing else.

Other gates touched by the change are listed in `gates.txt`.
