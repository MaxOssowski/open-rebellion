# Absent states, second pass: save delete, report acknowledge, chat, destroyed ship

Date: 2026-11-10. This follows the owner's decision of the same day:
states absent from the original are removed
(`2026-11-08-states-absent-from-the-original`). Four more cells were
traced and found absent from the original.

## String tables

The string tables (RT_STRING) of REBDLOG.DLL, TEXTSTRA.DLL, COMMON.DLL,
STRATEGY.DLL and TEXTCOMM.DLL were read with a PE resource walk.

**REBDLOG** holds 39 strings. The one-button boxes' labels are Resume
`0x190d`, To Cockpit `0x190e`/`0x1910`/`0x1911` and Acknowledged
`0x190f`. Their callers:

| Function | Box text | Label | Context |
| --- | --- | --- | --- |
| `FUN_00413560` | `0x1204` "Your Opponent Has Left The Game" | `0x190e` | multiplayer |
| `FUN_00410dd0`, `FUN_00410c70` | `0x1208`/`0x1207`, DirectPlay provider failed | `0x190f` | multiplayer |
| `FUN_0040ec60` | `0x1205` "Your Opponent Has Left This Session" | `0x190f` | multiplayer |

The save and load strings are only the overwrite and load questions
(`0x121a`..`0x121f`) and the saved or failed notices (`0x1217`,
`0x1219`). No string anywhere names a delete, except TEXTSTRA `0x8009`
"Delete Selected Messages" (the Message Index). None names "Acknowledge"
or "No result".

## The cells

| Cell | State | Why it is absent |
| --- | --- | --- |
| PRE-03-C005 | delete a saved game | `FUN_00407180` (Game Options) builds no delete control, and no string names one (`ghidra/notes/game-options.md`) |
| OBJ-12-C013 | acknowledge a mission report | A report's only controls are the continue question's check mark and X (`mission-report-question.md`; OBJ-12-C011 and C012 pass). "Acknowledged" is a multiplayer error box's label |
| CMD-08-C012 | chat | Chat needs a multiplayer session. Multiplayer is out of scope (owner, 2026-10-30), and A0 single player shows Compose Chat disabled |
| OBJ-06-C004 | destroyed ship status | The status words (TEXTSTRA `0x8740`..`0x8799`) have no Destroyed. A destroyed ship leaves its fleet. "Destroyed" (`0x1933`) is a summary column |

All four are excluded from the parity denominator.

## The port

Only one of these states was ported: the save delete, an invented one.
It is now removed:

- the replacement save/load panel's "✕ Delete" button
  (`panels/save_load.rs`);
- `PanelAction::DeleteSave`;
- `GameOptionsAction::Delete`, with its made-up question "Delete the
  selected saved game?";
- the two handlers in `main.rs`.

`rebellion_data::save::delete_slot` stays as a data-layer function. No
interface calls it.

The unit test `tactical_options_reject_save_but_ask_load_and_restart`
(formerly `..._reject_save_and_delete_...`) no longer lists Delete. The
smoke script `game-options-smoke.mjs`, which is not in the batch, now
checks that the save panel shows no Delete label, where it used to
delete slots.

`ghidra/notes/game-options.md` records that the original has no delete.

## Not changed

CMD-11-C029 (message boxes) stays partial. Its unchecked boxes, To
Cockpit and Acknowledged, appear only in multiplayer. The single-player
save notices (`0x1217`/`0x1219` with Yes) are not yet compared with A0.

## Batch b168

23 of 23 interface gates on Linux; workspace tests 3498 passed, fixture
tests 139. Tactical matrix `2026-11-10T06-27-35-004Z-4074942`: 148 of
148.

The diff's mutants: 12 caught, 2 unviable, and 1 missed. The missed one
replaces `draw_save_load` with `None`. The diff touches that function
only by removing its Delete button. The function draws the replacement
save/load panel with egui, and no unit test drives it, before or after
this change. The panel is a replacement window, due for removal with
its tracker item (the replacement-panels decision). Its Save and Load
paths are exercised by the save tests in `rebellion-data` and by the
Game Options gate.
