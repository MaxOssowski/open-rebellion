# Ledger states the original does not have (for the owner's decision)

Date: 2026-11-08 (night). This is not a status change. The ledger's
required states came partly from the manual's wording. For the cells below,
the executable and its string tables show no such state. If the owner
agrees, they could be marked excluded ("not in the original") instead of
staying pending.

## Status words

TEXTSTRA's status words 0x8740..0x8799 are, among others:

- Enroute, On Mission, Captured, Injured, Awaiting Orders;
- Under Construction, Active;
- Idle, Building, Training, No Facilities, Reserved;
- Killed, Retired.

There is no "Inactive", "Damaged", "Repairing", "Critical" or "Blocked";
"Damaged Ships:" is the only "Damaged".

| Cell | State | Evidence |
|---|---|---|
| OBJ-09-C005 | facility inactive | `FUN_0044a470` writes a facility's Status as 0x8744 Enroute (`+0x50` bit 0x10), 0x8763 Active (bit 4) or 0x8757 Under Construction, nothing else |
| OBJ-09-C006 | facility damaged | As above. A facility has no hit points: bombardment destroys it (`FUN_0058d770`, `FUN_0058e320`) |
| OBJ-09-C007 | facility repairing | As above. Repair (timer 0x380) is a ship's |
| OBJ-07-C011 | damaged producer | A producer's band reads Idle, Building, Training, No Facilities or Reserved (0x8769..0x8773) |
| OBJ-07-C012 | blocked producer | As above. The core halts production in an uprising or a blockade (6eb24155) without a word for it |
| OBJ-06-C003 | ship critical | No "Critical" string. A ship shows its hull bar and the 0x200 damage mark |
| EVT-03-C004 | bombardment blocked by shields | Shields lower the shots (`FUN_0058df60`). The summary has no shield line, and no TEXTSTRA text names one |
| EVT-03-C012 | bombardment damaged column | The assault/bombardment summary has Operational and Destroyed columns only (`FUN_0044c410`) |

## Not settled

These need a trace before anything is decided:

- OBJ-07-C013 (priority control): `manufacturing-build-selection.md`
  names no such control.
- OBJ-12-C008 (no-result mission report) and OBJ-12-C013 (acknowledge):
  reports go to the Message Index.
- CMD-07-C006 (urgent message) and CMD-07-C012 (missing clip frame).
- CMD-08-C004 (summary).
