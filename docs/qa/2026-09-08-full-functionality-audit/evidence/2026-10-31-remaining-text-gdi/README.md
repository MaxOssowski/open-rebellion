# The remaining game text through GDI's rules (interface text)

Date: 2026-10-31. Follows `../2026-10-31-dialog-text-a0/`.

## Change

The last texts drawn with egui's own placement now go through
`theme::paint_gdi_text` or `theme::paint_gdi_text_block`:
- **GID caption.** The default font, `DT_CENTER` across the galaxy view
  from its top (`FUN_00425d00`).
- **GID menu rows.** Font 0, from the row's top.
- **GID legend's texts.** Each in its own font: centred across the legend
  at `y == 1`, otherwise from its point.
- **Story movie caption.** Font 4, `DT_WORDBREAK` in the 580 by 70
  TextScrollField, in `main.rs`; only this hunk of the file is staged.
- **Game Options name boxes.** The edit fields in the command centre now
  use the default font, as the shown names do.
  - A0 `tooltip/200-options` sets "test" with its baseline at the box's
    y + 14: the EDIT control's line starts 1 below the box's top.
  - The field's rectangle therefore moves to put egui's baseline there.
  - The first try, `DT_VCENTER`, set it 2 pixels low, and `game-options.mjs`
    failed on that.

`theme::original_font`, which sized egui by `lfHeight × 2048 / 2288`, is
removed. No text uses it any more.

## Gates (Linux Chromium)

`gates.txt` holds the full run. `run.mjs --all` passes 152 of 152, and
`game-options.mjs --a0` passes after the edit-box fix.

The other failures are the ones recorded before this work, with the same
counts:
- `encyclopedia-index-shell`;
- `message-index-shell`;
- `fleet-move`;
- `encyclopedia-art`, which needs `REBELLION_EDATA_DIR`.
