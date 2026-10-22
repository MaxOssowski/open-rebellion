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

## The report's texts

The report class (vtable `0x0065bc08`, built by `FUN_0049ac00` for one
viewer) stores the title `0x7001` at `+0x88`, the headline at `+0x94`, the
body at `+0xa0` (its sentences appended), the message line at `+0xac`
and the Display page's line (`0x7013`/`0x7014`) at `+0xb8`. Fields:
`+0x50` the winner and `+0x54` the side that withdrew (the system's
`+0x78` bits 0..1 and 2..3, `FUN_0049b060` on `0x14d`), `+0x58` the
viewer's side, `+0x5c` from the source's `+0x4c`, `+0x48` the withdrawn
fleet's destination. `FUN_0049b0f0` calls `FUN_0049b150` with no winner,
`FUN_0049b330` for the viewer's win and `FUN_0049b4d0` for its loss:

- no winner: `0x7004` "The battle at {1:1} is indecisive."; `0x700e`
  "There has been no victor."; then `0x700f` "All {1:4} and {2:4} ships
  have been destroyed." when nobody withdrew. When the viewer withdrew it
  is `0x700d` "...withdrawn to {2:1}." and the enemy's `0x700b`. When the
  enemy withdrew it is the viewer's `0x700b` and the enemy's `0x700c`;
- the viewer's win: `0x7002` "The {1:4} fleet is victorious."; by the
  system's side: neutral `0x7007` "cleared of {enemy}", the viewer's
  `0x7005` "successfully defended from {enemy}", the enemy's `0x7006`
  "now under blockade by {viewer}". When `+0x5c` equals the system's side
  it is instead `0x7016` "broken the blockade" or `0x7015` "maintained
  the blockade". Then the enemy's `0x700b` "completely destroyed" or
  `0x700c` "withdrawn";
- the viewer's loss: `0x7003` "The {1:4} fleet is defeated."; by the
  system's side: neutral `0x700a`, the viewer's `0x7008` "now under
  blockade by {enemy}", the enemy's `0x7009` "The {viewer} attack on
  {1:1} has failed."; then the viewer's `0x700b` or `0x700d`.

## Ported (2026-10-22)

`tactical_view.rs` (`battle_force_page`, `battle_summary_page`, drawn by
`draw_page_ops`) follows the above. The port's own
"Damaged" column is gone: a damaged unit is operational.

port:
- The rows' pictures come from the world's classes: a ship's or
  squadron's portrait, a regiment's portrait on 11514, a character's mini
  on 11500. Names under them.
- The battle captures and kills no personnel, so they are all survivors.
- No scroll bar. Since 2026-10-22 the wheel over the list scrolls all
  its columns together a row at a time (two rows of 70 show, or three of
  50 for Personnel). Another tab or category starts at the top.
- A regiment is lost when the surviving ships cannot carry it, or at
  strength 0. This is the port's earlier rule.

## The Display page (read 2026-10-22)

`FUN_0046f360` paints the message line `+0xac` at (24, 40), 328 wide,
word-wrapped, font 6 (18 px), and the instruction `+0xb8` at (24, 130),
328 wide, font 7 (18 px bold; `FUN_0060eed0` builds the fonts), both in
the viewer's colour. The report fills them:

- the viewer's win (`FUN_0049b330`): `0x7012` "The {1:4} fleet is
  victorious. The world {2:1} is now under {1:4} blockade." and `0x7013`
  "Select one of the following buttons to close this display and go
  directly to...";
- the viewer's withdrawal (`FUN_0049b150` when `+0x54` is the viewer,
  `FUN_0049b4d0` when `+0x54` is set): `0x7011` "The {1:4} fleet was
  defeated at {2:1}. Your fleet has withdrawn to {3:1}." and `0x7013`;
- otherwise `0x7010` "... Your fleet has been destroyed." and `0x7014`
  "Select the following button ...".

The system button `0xcb` sits at (29, 182), 169 by 96, art `0x2a0c`
(10764), pressed `0x2a0f` / `0x2a10` by the viewer's side. The fleet
button `0xca` sits at (229, 182) only when `FUN_004f36a0` finds the
viewer's fleet at the system; it keeps that fleet at `+0x170`. Its art is
`0x2a0d` / `0x2a0e` by the viewer's side, pressed `0x2a0f` / `0x2a10`.
The labels sit under the buttons at y 278 (`0x116`), 169 wide, centred,
white, font 4: the system's name, and "Fleet" (string `0x1942`).

Ported in `tactical_view.rs` (`battle_display_lines`,
`battle_display_page`). port: the withdrawn fleet's destination is
decided after the report, so `0x7011` drops " to {3:1}". The fleet
button follows the text: it shows when the viewer won or withdrew.
