# The Battle Summary's headline keeps its trailing space; message arrows

Date: 2026-11-09. Adds to TAC-07-C001 and TAC-01-C006, and moves
CMD-08-C004 ("summary") from pending to partial.

## The headline

### A0

Three captures show the same headline position:

| Capture | Headline | Headline x (window) | Centre | Title centre |
| --- | --- | --- | --- | --- |
| `.artifacts/a0-vm/empire-game3/battle-result-coruscant.png` (SHA-256 `bbdb1cf693fb3650…`) | The Imperial fleet is defeated. | 71..344 | 207.5 | 212 |
| `.artifacts/interface-parity/a0/TAC-01-C006/original.png` | The Alliance fleet is defeated. | 72..343 | 207.5 | 212 |
| `.artifacts/interface-parity/a0/TAC-01-C013/original.png` | The Alliance fleet is defeated. | 72..343 | 207.5 | 211 |

`FUN_0046f360` gives the title and the headline the same box: x 12, 400
wide, centred, font `0xb` (`FUN_00601b30(0xc, 0x12)` and `(0xc, 0xe1)`,
width 400 at `+0x128`). The title is centred on 212. The headline sits
4.5 pixels further left.

### The cause

The headline records end in a space before their `0x01` end mark
(TEXTSTRA.DLL RT_RCDATA, read with a PE resource walk):

- `0x7002`: `The |\x01\x04\x00\x00\x00 fleet is victorious. \x01`
- `0x7003`: `The |\x01\x04\x00\x00\x00 fleet is defeated. \x01`
- `0x7004`: `The battle at |\x01\x01\x00\x00\x00 is indecisive. \x01`

GDI measures the trailing space when it centres the line, so the
visible text moves left by half a space. The port trimmed the space
(`battle_report_lines`, `trim_end`) and centred the visible text on 212.

### The change

The headline now keeps its trailing space. Only control characters are
trimmed. The port's text measure (macroquad `measure_text`) adds every
character's advance, so the space shifts the line as in GDI.

Seven unit tests that compare the report's lines word for word now expect
the record's trailing space. They fail without the change (7 of 290
tactical tests). The fallback English used when the texts are missing is
unchanged.

### Browser

`fleet-window.mjs` `squadron-battle` (Empire, Linux) simulates a battle
and compares its Battle Summary with A0 `battle-result-coruscant`:

- the scene (x 137..539, y 95..269): 0 pixels differ;
- the rail and its five buttons (x 541..594, y 64..369): 0 pixels differ;
- the headline's centre: the port's 208 against A0's 207.5 (allowed: 1).
  The b146 build put it at 211.

The visible text is 266 pixels wide against A0's 273, the font-metric
deviation the owner accepted on 2026-10-30.

## The message screen's arrows (CMD-08-C004)

Manual p. 79: "The Message Summary screen (Fig. 3.19) has arrows at the
top of the message graphic. Click on these arrows to read through your
messages. If you have selected a Message Category tab, you will scroll
through messages only in that category. To go back to the list, click on
the Display Message Index button."

The Message Summary screen is the Message Index's mode 2, the open
message. Its picture, title, text and rail were compared with A0 for many
messages in `message-display.mjs`. Display Message returns to the list,
and Display opens the subject's window.

The arrows are controls `0x96` (window (390, 15)) and `0x9a` (367, 15),
shown in mode 2 (`FUN_004665f0`). `FUN_0046a200` enables each one only
while its neighbour exists. `message-display.mjs` now opens the Conflict
list's newest report, "Assault on Coruscant". `0x96` shows the next row,
"Assault on Umgul", and `0x9a` comes back. Display Message then returns
to the list.

Still open: which of the two arrows goes to the older message is inferred
from their art (up 10948, down 10919). The A0 VM handling notes from
2026-11-08 used (461, 70) for older and (436, 70) for newer, which agrees,
but no A0 capture records a step. The cell stays partial.

## A0 hash

`battle-result-coruscant.png`: see `SHA256SUMS` beside this README.

## Batch b148

23 of 23 interface gates on Linux; workspace tests 3492 passed, fixture
tests 139; the diff's 4 mutants caught. Tactical matrix
`2026-11-09T02-31-18-762Z-605154`: 148 of 148.
