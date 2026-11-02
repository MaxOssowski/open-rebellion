# A band's Stop asks first; Build Selection's quantity, Cancel and Confirm in A0 (2026-11-02)

Surface ledger:

- OBJ-07 (Manufacturing): C007 "queue populated" and C014 "cancel".
- OBJ-08 (Build Selection): C009 "quantity", C011 "accept" and C012 "cancel".

## A0 (Windows 10 VM, the Empire's "resource turn 14" save, days 140..147)

Captures are in `.artifacts/a0-vm/build-selection2/` (hashes in
`a0-sha256.txt`). Coruscant's Manufacturing window is at (203, 91) and
Build Selection at (255, 87). The Empire's maintenance reads 226.

| Capture | State |
|---|---|
| `01-opened` | Build on the troops band shows the Imperial Army Regiment: 6, 3, "24 Days", "0 Days", number 1. |
| `02-up1`, `03-up2` | The up arrow at (189, 196) twice: number 3, values 18, 9 and "72 Days". |
| `04-down1` | The down arrow: number 2, values 12, 6 and "48 Days". |
| `05-cancelled` | Cancel at (141, 224) closes the window. The band still reads "No Troops in training". |
| `06-reopened` | Build again: the number is back to 1, with the same class. |
| `07-confirmed` | Confirm: the band names the Imperial Army Regiment, shows its mini and "Training: 1". The maintenance monitor falls from 226 to 223 at the order. |
| `08-band-menu` | The populated band's menu: Build, Stop, Destination, Encyclopedia, Status, Reserved, all white. |
| `09-stopped` | Stop opens the confirmation at (148, 52): picture 1039, "Are you sure you want to stop construction of the following?", "1  Imperial Army Regiment", checkmark and X. |
| `10-stop-confirm-hover`, `11-stopped` | The checkmark empties the band. The monitor is back at 226. |
| `12-three`, `13-three-queued` | Three ordered: "Training: 3", monitor 217. |
| `14-stop-dialog-three` | The confirmation lists "3  Imperial Army Regiment". |
| `15-stop-declined` | The X keeps "Training: 3". |

## The original

`FUN_0049a350`, case 0x213, builds the Stop's confirmation:

- the sound `+0x30` = 0x465;
- the template 0x7053, "Are you sure you want to stop construction of the following?";
- for each manager in the order's list, 0x7055 ("\n{1:6}  {1:5}") from `FUN_004f2f60(side, id) + 0x30`;
- the picture `+0x2e` = 0x40e (1038) for side 1, 0x40f (1039) otherwise.

A0 `09` has 1039 at (160, 82), all 80 000 pixels exact. That is the window's (12, 30), the place the other confirmations use. The window is centred in the galaxy view as they are.

## The port

- **The order.** `move_confirmation.rs` has `ConfirmedOrder::Stop { area }`. Its system is the confirmation's `destination`, and its lines are `(name, count)`.
  - `text` follows 0x7053 and 0x7055.
  - `picture` is 1038 or 1039.
  - The checkmark answers `MoveConfirmationAction::Stop`.
- **Wiring in `main.rs`.** A band's Stop opens the confirmation with the band's product name and its queued units, the band's "Training: N". The checkmark runs `PanelAction::StopProduction`. Before this change, Stop emptied the band at once.
- **Gate.** `build-selection.mjs` now meets the confirmation after the band's Stop. For each side it checks:
  - the picture (1038 or 1039) against the bitmap, every pixel;
  - for the Empire, the template's two lines against A0 `09`, each ink box within 2 pixels (the accepted text deviation);
  - that the checkmark empties the band.

## Not compared yet

- **Quantity and values.** A0 `02`..`04` show the number and the values after the arrows. `build-selection.mjs` does not press them, and the port's values for this class at Coruscant were not set up to match.
- **The band with its queue.** A0 `07`, `13`: the band's text, mini and progress are not compared with the port's `confirmed-screen.png`.

## Tests

- `a_stop_asks_with_its_own_text_and_picture_and_stops_on_the_checkmark` (`move_confirmation.rs`). The `Stop` variant is new, so this test does not build without the change.
