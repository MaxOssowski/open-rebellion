# The Empire's Assault Summary against the unedited A0

Date: 2026-11-08. Closes TAC-08-C002, TAC-08-C004 and EVT-03-C005. Adds to
TAC-08-C003.

## A0

`.artifacts/a0-vm/assault-clean/` holds the unedited "test" save (Alliance,
day 720), in which the Empire took Yavin. The SHA-256 hashes are in
`2026-11-06-display-buttons-keyed`.

| Capture | What it shows |
|---|---|
| `01-summary-yavin` | Title "Assault on Yavin" and "Imperial troops have taken control of the Alliance system Yavin", in the attacker's green, over the Imperial assault picture 0x2b99 (STRATEGY 11161) |
| `02-attackers` | Imperial Forces, Trooper Regiments: Operational Stormtrooper Regiments; Destroyed "No Casualties" |
| `03-system-forces` | System Forces: Operational "None"; Destroyed an Alliance Army Regiment |

Earlier, the assault-taken case compared only the Alliance's side, against
`messages-day633/assault-coruscant*`. That save is DS_flip_allian, whose
messages are built for the Empire, so it gives no evidence that depends on
the side.

## Port

`fleet-window.mjs` 'assault-taken' takes Akrit'tar on each side. The Empire's
side is now compared with `assault-clean`:

- the picture 11161, rows 25..205: 0 pixels from the bitmap and from A0;
- the green title's ink, centred as A0's, and the body's first line, at
  A0's left and top within 2 px;
- the subtitle "Imperial Forces" / "System Forces", the category label, and
  the column heads on both force pages, at A0's left and top within 2 px,
  with their widths within 15 %;
- the Operational "None" on System Forces.

These are not compared on the Empire's side:

- **The category buttons, scroll bar and rows.** The buttons are the
  viewer's side's art. A0 is the Alliance's view of the Empire's assault;
  the port's case is the Empire's own view.
- **The regiments.** The fixture's regiments are Imperial Fleet Regiments,
  not Stormtroopers.
- **System Forces' Destroyed column.** A0 lost a regiment; the fixture's
  defenders lose none.

The Alliance's side keeps its pixel checks of the bar, the scroll bar and
the second row.

## Run

`fleet-window.mjs --only=empire/assault-taken --only=alliance/assault-taken`
(Linux Chromium): 2 of 2 passed (`.artifacts/interface-parity/fleet-window-2026-11-08T10-*`).
Batch b136 runs it in full.
