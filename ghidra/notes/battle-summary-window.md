# The Battle Summary window's forces page (read 2026-10-22)

The window `FUN_0046f360` (vtable `0x0065a300`, TEXTSTRA `0x1922` "Battle
Summary") is the space battle's twin of the Assault Summary
(`assault-summary-window.md`). It shares the frame, rail, Close, summary
and Display pages. Its forces pages differ in these ways.

- Rail `0x70` / `0x71`: the Alliance's and the Empire's forces. The
  report holds the Alliance's units at `+0x78` and the Empire's at `+0x80`
  (`FUN_004714e0`).
- The paint slot `FUN_00471200` draws the category's label at (37, 99),
  347 wide, left-aligned, font 4, white. It draws the rail label at
  (12, 40), 400 wide, centred, font 6: red (`0xff`) for `0x70`, green
  (`0xff00`) for `0x71`, white otherwise.
- "Filters" (`0x1952`) at (86, 70), left-aligned.
- The category bar `FUN_0060d590(.., 0x82, 0x3b, 0xc4, 0x2a)` at (130,
  59): four buttons, 49 by 41, at x 0, 49, 98, 147. They are Capital
  Ships `0x65` (`0x1928`, families `0x14..0x1c`), Fighter Squadrons `0x66`
  (`0x1929`, `0x1c..0x20`), Trooper Regiments `0x67` (`0x1930`,
  `0x10..0x14`) and Personnel `0x68` (`0x1931`, `0x30..0x40`). The art is
  the Assault Summary's, by the viewer's side.
- The heads: "Operational" `0x1932` (37, 118) and "Destroyed" `0x1933`
  (204, 118), 165 wide. For Personnel, "Survivors" (37), "Captured"
  (149) and "Killed" (260), 110 wide. All are centred.
- The page art: two columns STRATEGY 10761 (`0x2a09`), three columns
  10762 (`0x2a0a`) for Personnel.
- The list (37, 136), 347 by 166. Its cells are 165 by 70, or 110 by 50
  in three columns (`FUN_004714e0`). Each row is `FUN_004724a0`: the
  picture through `FUN_0042c3b0` (a portrait, or a personnel mini) and the
  name under it.
- An empty column (`FUN_004714e0`): the first reads "None" (`0x8526`)
  when every column is empty, else "No Survivors" (`0x1940`). The middle
  reads "None" and the last "No Casualties" (`0x1941`).

## The summary page

`FUN_0046f360` paints the title at (12, 18), 400 wide, centred, font
`0xb`. It paints the outcome's first line at (12, 225), 400 wide, centred,
font `0xb`, and the rest at (24, 255), 352 wide, word-wrapped, font 6,
over a black shadow at (25, 256). All take the viewer's colour (`0xff`,
or `0xff00` for side 2). guide-059 shows it.

## Ported (2026-10-22)

`tactical_view.rs` (`battle_force_page`, `battle_summary_page`, drawn by
`draw_page_ops`) follows the above. The port's own
"Damaged" column is gone: a damaged unit is operational.

port:
- The rows' pictures come from the world's classes: a ship's or
  squadron's portrait, a regiment's portrait on 11514, a character's mini
  on 11500. Names under them.
- The battle captures and kills no personnel, so they are all survivors.
- No scroll bar; the list shows its first rows.
- A regiment is lost when the surviving ships cannot carry it, or at
  strength 0. This is the port's earlier rule.
