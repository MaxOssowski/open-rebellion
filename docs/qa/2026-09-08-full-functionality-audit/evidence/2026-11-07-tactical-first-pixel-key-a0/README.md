# The battle screen's controls are keyed on their first pixel (A0)

Date: 2026-11-07.

## Source

The 106 A0 TAC captures of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`) in
the ignored `.artifacts/interface-parity/a0/` store. Only cell IDs are cited
here.

Each TACTICAL.DLL bitmap was searched for in every capture with its
first-pixel index left out. That index is the first byte of the DIB's bits,
which is the bottom row's first pixel; `FUN_005fd170` stores it as the
bitmap's transparent key for an uncompressed 8-bit DIB.

- The group bar. Its headers (1001, 1004, 1008, 1010) and its task-force
  and squadron buttons (1005..1007, 1012..1020) match every other pixel.
  Under each key pixel the capture shows the backdrop 1000. The keys are
  grey: 72 on 1001 and 1008, 32 on 1004 and the pressed squadron buttons,
  and 40 on the normal and disabled squadron buttons (TAC-02-C012,
  TAC-01-C002, TAC-02-C002..C005). The port drew those greys.
- The squadron buttons are 25 x 15 (`FUN_0059c030` creates them as
  `0x19 x 0xf`). The bitmaps are 25 x 17, and in every capture the backdrop
  shows in their last two rows. The task-force buttons are 25 x 17
  (`FUN_0059be80`, `0x19 x 0x11`), and their bitmaps show whole.
- The right-hand panel's buttons have keyed corners or fields too, each
  showing the panel under its key: Previous/Next 1101/1103 (grey 212 and
  154, TAC-01-C004), the order buttons 1105/1107 and the assign buttons
  1113/1115 (a blue corner), the maneuvers 1133..1142 (blue), the
  Stand Off/Surround toggle 1117/1118 (yellow, and dark red on Surround,
  TAC-05-C001 and TAC-05-C005), the missions buttons 1170..1192 (grey 136, or 58 when lit,
  TAC-05-C007..C010), the options buttons 1149..1157 (blue, and green on
  Game Options, TAC-06-C001), and the highlight toggles 1034/1035 and the
  camera target button 1058 (a magenta corner). The Death Star laser control
  1021..1023 is keyed on black (TAC-05-C017, TAC-05-C018).
- The panels 1303 and 1304 are not keyed. Their first-pixel grey 124 shows
  in every capture, so they are blitted whole.

No bitmap has a second palette index with its key's colour, so keying by
colour (the port's way) and keying by index give the same pixels.

## Change

- `bmp_cache::uses_first_pixel_key` keys the TACTICAL controls above.
  The panels 1301..1310 stay opaque.
- `tactical_view`: `FIGHTER_GROUP_BUTTONS` are 25 x 15, and the squadron
  bitmaps are drawn cut to that size. Their hit area shrinks with them.
- `run.mjs` (`--battle`): `verifyTacticalBitmap` leaves out the first-pixel
  index of these bitmaps. Under the group bar's keys it checks for the
  backdrop 1000. `verifySquadronButton` checks a button's first 15 rows and
  the backdrop in its last two.

## Open

- The highlights (TAC-04-C006, C007): A0 draws a pure red or green outline
  around each ship, the port a lit wireframe. Not changed here.

## The task-force number

A0 draws Task Force 1's "1" ten rows tall over x 70..72, rows 5..14. It is
black at rest (TAC-02-C008, TAC-02-C012) and white while the task force is
selected (TAC-02-C011, TAC-03-C001). `FUN_0059be80` sets state 4's text
colour to `0x2ffffff`. The port drew a smaller number, always black, in
macroquad's own font. It now draws the panel's Arial at 14 px (caps 10
rows), centred on the button, with the cap top at row 5, white when the
button is down. In `run.mjs`, `task-force-1-number-selected` and
`task-force-1-number-at-rest` find the number's box within a pixel of A0's.

## Recover was the other side's

In TACTICAL.DLL, 1170..1172 are an X-wing and 1173..1175 a TIE. The A0
captures are all from Alliance games. They show 1170 (TAC-05-C021), the
pressed 1171 (TAC-05-C010) and the disabled 1172 (TAC-05-C007, C008, C015)
at (568, 157). The port's constants named 1170..1172 the Empire's, so an
Alliance player saw a TIE there. `BTN_RECOVER_ALLIANCE_*` are now
1170..1172 and the Empire's 1173..1175; `run.mjs` expects the same. This is
the swap that `gate-self-referential-ids` warns about: the gate compared
the port with its own table.

## Maneuvers in A0

The chosen maneuver shows its pressed art and its preview at (496, 42):

| Cell | Chosen | Button | Preview |
|---|---|---|---|
| TAC-05-C001 | Left Hook | 1134 at (495, 187) | 2101 |
| TAC-05-C002, C014 | Hammer | 1138 at (495, 157) | 2103 (animated) |
| TAC-05-C003 | Anvil, after Hammer | 1140 at (525, 157) | 2104 (animated) |
| TAC-05-C004 | Stop | 1142 at (557, 172) | 2105 |

The toggle reads 1117 on Stand Off and 1118 on Surround (TAC-05-C005,
C006). `run.mjs` already checked Hammer, Anvil and the toggle. It now also
chooses Left Hook and then Stop, and checks 1134 and 2101, then 1142, 1133
and 2105. The Imperial maneuvers panel (TAC-05-C016) is 1309 with the same
buttons as the Alliance's.

## Game Options in a battle (TAC-06-C007)

- Restart is lit (10035). `FUN_00407180` never disables it; the port had
  greyed it in a battle, waiting for an A0 capture. It now asks 0x476 first,
  as it does in the command centre.
- The display options are grey (128, 128, 128), where the port drew
  (80, 80, 80). `FUN_00408bc0` sets them to `0x2505050`, a PALETTERGB. With
  the default palette, its nearest colour is the system grey 128. The
  window's other PALETTERGBs, `0x200ff00` and `0x2008000`, are system
  colours already.
- The lamps (10045) are 0 pixels from A0, and so are Return (10026) and
  Exit.

## Battle Options after a withdrawal (TAC-06-C009)

`FUN_005dab90` builds the Battle Options buttons and hides some of them:

- Withdraw, when no unit can jump (`FUN_00596a90`);
- Get Immediate Result and Observe / Take Command, once either side has
  begun to withdraw (`FUN_00596ab0` → `FUN_005adde0`: a side commander's
  `+0x84` is non-zero, as `FUN_005ad530` makes it).

After a confirmed withdrawal, A0 shows all three blank: 0 pixels from the
panel 1306 there. The port kept Get Immediate Result and Observe. Now
`BattleOptionsGates` hides them while `withdrawal_ms` or
`ai_withdrawal_ms` is set. The port never clears either, as the original
never clears `+0x84`. In `run.mjs`, `battle-options-withdraw-disabled`
compares the panel under all three controls.

## The battle palette (TAC-03-C001..C011)

The subsystem icons (1201..1225) are keyed on black. Their state art
matches A0 at (492 + 27k, 131). But A0 shows the full weapons and engines
icons with (230, 0, 0) and (255, 230, 60) where the bitmaps have pure red
and yellow: 44 pixels on 1210, 13 on 1220 and 12 on 1208. The battle screen
is drawn in the battle's palette (TACTICAL 5531..5557). The icons carry a
palette of their own, and its entries 140 and 146 are pure red and yellow.
In every battle palette those entries are (230, 0, 0) and (255, 230, 60),
and the icons use no other entry where the two palettes differ.
`bmp_cache::battle_palette_entries` gives the icons those two entries. In
`run.mjs`, `verifyTacticalBitmap` expects them on 1201..1225.

The other HUD bitmaps share the backdrop's palette, which differs from the
battle palettes only in entries 160..245, the planet's colours. 26 of them
use one of those entries, for 1 to 94 pixels each (1008 has one, the
maneuver previews 2101..2105 have 3 to 9). They keep the bitmap's colours;
this is open.

