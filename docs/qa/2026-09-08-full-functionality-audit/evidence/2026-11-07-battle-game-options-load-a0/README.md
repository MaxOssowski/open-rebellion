# Load stays lit in a battle's Game Options (A0)

Date: 2026-11-07. Cell PRE-03-C006 (disabled controls, the battle context).

## A0

`TAC-06-C007` shows the Game Options window opened from a battle, from the
`test` game with six occupied save slots:

- every Save control is COMMON 10048, the disabled art of 10046;
- every occupied slot's Load control is 10049, its normal art, the same
  pixels as the galaxy context's (`game-options-alliance/00-from-galaxy`,
  at most 1 of 1440 pixels off per slot);
- Restart is lit (10035), and Return (10026) and Exit match the port.

## The reading

`FUN_00407180` disables Save (`0x273e`) when the window's `+0xa8` is 0,
when `+0xa8` lies in `0x14..0x17` (`bVar2`), or in a multiplayer session.
It disables Load (`0x2741`) only for `bVar2`, the multiplayer case, or an
empty slot. Load is lit in A0, so `bVar2` is clear for the window in a
battle. Why Save is grey there is not settled by this reading.

A second A0 run, a new Easy Empire game on day 660, entered the battle at
Corsin with Take Command (`.artifacts/a0-vm/research-display/31..38`):

- `34-game-options`: the same layout, every Save grey, every occupied
  slot's Load lit;
- `35-restart`: Restart asks "Returning to the shuttle cockpit will cause
  unsaved changes to be lost" / "Return without saving?" (REBDLOG 0x121a,
  0x121b), as the port does from a battle; the X cancels (`36`).

## Change

`GameOptionsState::enabled` and `control_at` offer Load on an occupied
slot in a battle, and the window draws 10049 there. A Load asks
"Load without saving?" first, as in the command centre. Delete, the
port's own control, stays off in a battle.

## Tests

- `tactical_options_reject_save_and_delete_but_ask_load_and_restart`: in a
  battle, Save, Delete and an empty slot's Load do nothing; an occupied
  slot's Load asks; the Load rectangle is a control. Without the change,
  Load did nothing.
- cargo mutants on the diff: 3 caught, 2 unviable. The miss is the pressed
  art's comparison in `draw_game_options`.

## Not compared in the browser

The battle fixture has no saves, so the lit Load art in a battle is
checked by the unit test only.
