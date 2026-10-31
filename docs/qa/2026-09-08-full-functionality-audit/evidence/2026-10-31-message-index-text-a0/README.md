# The Message Index's text against A0 (interface text)

Date: 2026-10-31. Follows `../2026-10-31-build-selection-text-a0/`.

## Finding

- **Unread rows.** `FUN_00468ab0` sets an item's font (`+0x38`) to 13 for
  an unread message and 10 once read (`FUN_00469de0`). The port drew every
  row in font 8, a hypothesis made to match A0's narrow unread rows. A0
  `tooltip/170-index` shows unread "Garrisons" at 47 by 7, with glyphs one
  pixel wide. That is Arial Bold at 10 ppem: font 13 (the bold 12-pixel
  cell) as GDI draws it, black-and-white with v35 hinting.
- **Category heading.** "Advice Messages" is 101 wide in A0, which is
  Arial at 13 ppem: the default font, not font 10.

## Change

Through `theme::paint_gdi_text`:
- **Rows.** Font 13 while unread, font 10 once read, from the line 2
  below the row's top.
- **Heading.** The default font.
- **Title.** "Message Index" in font 11.

The unit test `an_unread_row_takes_font_13_and_a_read_one_font_10` fails
with the unread font set to 10.

## Check

`agent-advice.mjs --a0=.artifacts/a0-vm/tooltip` compares the first
advice row's offset from the heading with A0's. A0's index is the Empire's
and lists other topics, so only offsets are compared: 24 right and 40
down in A0, 23 and 40 in the port.

That check also holds for the old drawing. The font choice rests on the
bitmap match above and on the unit test.

The port's bold rows are antialiased (egui), so they look heavier than
A0's black-and-white ones.

## Gates

`gates.txt`.
