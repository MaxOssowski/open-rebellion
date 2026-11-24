# Pause: when orders act and object notices reach the windows

Traced 2026-11-17 from the typed decompile corpus
(`~/Games/rebexe-decompile/decompiled-typed/`) and `objdump` of REBEXE.EXE
(GOG install). For ledger cell OBJ-13-C006 ("HQ in transit"): while the
game is paused, are the HQ's parent notice `0x300` and the map redraw
delivered at once, or only when the clock runs?

Answer: only when the clock runs. A held pause stops the frame step before
the phase that executes orders and flushes notices. And the pause alert
blocks the command center, so in normal play no order can be given while
held.

## The frame step and the pause hold

`FUN_0041dff0` runs one phase a frame, `+8` = 0..`0xd`, then round again
(`timer-scheduler.md`). After every phase it calls `FUN_0041cf60` ->
`FUN_004361d0`.

| Phase | Call | Advances when |
|---|---|---|
| 0 | `FUN_0041e390` | the 0.25 s timer has elapsed (`FUN_004010d0`) and `+0x14` is 0; then `FUN_0051dea0`, one sub-tick |
| 1 | `FUN_0041e410` -> `FUN_0041d070` -> `FUN_00436550` | both players' slot `+0x30(sub-tick)` |
| 2 | `FUN_0041e300` | the sub-tick (`FUN_0041e1b0`, scheduler `+0xc`) is **not** the stop at `+0x10` |
| 3 | `FUN_0041cf40` -> `FUN_00436250` | both players' slot `+0x2c` |
| 5 | `FUN_0041e460` -> `FUN_0051df30` | no queue item and no order left to run |
| 6 | `FUN_0041e490` -> `FUN_0051e050` | always |

Pause (`FUN_00486fb0` cases `0x20`/`0x25`) calls `FUN_0041d2f0(0)` ->
`FUN_00436890`: the stop is the larger of the players' `+0x28`, plus one.
`FUN_0041dba0` -> `FUN_0041e290` stores it at `+0x10` (it only moves later)
and calls `FUN_00401930` -> `FUN_00415e60`, which posts `0x46c` with
dialog `0x1203`: REBDLOG 4611, the "Resume Game Play?" alert. The players
also get slot `+0x174(stop)` (`Class_0065abf0_vt174` stores it at `+0x48`).

So the step runs on until the sub-tick reaches the stop, then waits in
phase 2. Phases 3 to 6 do not run while it waits. `FUN_004361d0` lifts the
stop (`FUN_0041dbf0` -> `FUN_0041e2d0`, which clears `+0x10` and calls
`FUN_00401950`) only when held (`FUN_0041dc10`) and both players' `+0x48`
are 0 (`FUN_004888c0`).

## Where a player's order acts

1. `FUN_00487740` validates the order (slot `+0x1c`; a refusal goes to
   `FUN_00487c90` at once). An order that asks (`FUN_00487cc0`) opens the
   confirmation (`FUN_0048a340`); otherwise `FUN_0048aa90` stamps `+0x18`
   from the player's slot `+0x18` when unset, and `FUN_0041cee0` ->
   `FUN_004360f0` hands it to both players (slots `+0x20` and `+0x1c`).
   For a side's player, `Class_0065b070_vt1c` (`FUN_0048a920`) queues it
   with `FUN_00536fe0` (`0x536fe0`: sorted by `+0x18`, `FUN_005f5440`).
2. Phase 3: `Class_0065b070_vt2c` (`FUN_0048a960`) drops stale entries,
   takes the next order and passes it to `FUN_0041db80` -> `FUN_004fd220`
   -> `FUN_0051d880`, which queues it on the scheduler's list `+0xa4`.
3. Phase 5: `FUN_0051df30` serves the timer queues `+0xb0`, `+0xa8`,
   `+0xac`; when none is due it takes the head of `+0xa4`
   (`0x51dfee`..`0x51e011`: `FUN_00568ee0`, `FUN_00537030`, then
   `FUN_0051dd20`), which calls the order's slot `+0x20` and deletes it.
   For Move (`0x201`) that is `Event201_Fire` (`FUN_00578f30`) ->
   `FUN_00556390`, which moves the HQ (`headquarters-move.md`).

A held pause therefore runs no order, whenever it was given.

## Where the notice goes

1. The HQ's set-parent (`FUN_004f9d40`) calls slot `+0x15c`,
   `FUN_004fbc20`, which builds UI message `0x300` with `FUN_0053fe40`
   (factory `FUN_0043e540`, class `0x00659698`) and hands it to
   `FUN_004fd3b0` -> hook `DAT_006b2ad4` (`FUN_004fd030`) ->
   `FUN_0051d910`. Kind 1 goes to `FUN_0051e670` -> `FUN_0051eaa0`: it
   calls the message's slot `+0x14` (`0x00619730`, which does nothing) and
   appends it to the scheduler's list `+0xb4`.
   (`FUN_004f8aa0`'s `GameObjParentNotif` text is a debug log behind
   `FUN_006158b0`, which returns 0.)
2. `FUN_0051eaf0` empties `+0xb4`: each message goes to `FUN_004fd300` ->
   `DAT_006b2ae0` = `LAB_0041d100` (set in `FUN_00409280`) ->
   `FUN_004365d0`, which passes it by side to the player's slot `+0x50`.
   For the local player, `Class_0065ad70_vt50` (`0x486580`) calls the
   message's slot `+0x18` and files it (`FUN_004bef90`). Class
   `0x00659698`'s `+0x18` (`0x43e5b0`) calls the player's `+0x14c`,
   `FUN_00486d80`, which redraws origin and destination
   (`2026-11-12-hq-move-refresh`).
3. `FUN_0051eaf0` is called only at the end of `FUN_0051df30` (phase 5,
   `0x51e02e`) and from `FUN_0051e050` (phase 6), plus `FUN_0051ddf0`
   (`FUN_0040a050` state 9, after setup or a load). `FUN_0051eb40`, the
   same drain for `+0xb8`, is reached only from `FUN_0051e050`.

So the notice is flushed in the same phase 5 call that executed the order:
the redraw comes in the frame of the move. While held, neither runs.

## The pause alert holds the input

`FUN_00415e60` opens the "Resume Game Play?" alert as soon as the stop is
set. A0 (`speed2/pause-modal-*`, `draw_pause_alert`) shows the command
center takes no input under it: no sector window closes, no rail opens the
index, no speed menu opens. In normal play no order is given while paused.
An order given just before the pause (still in a player's list or `+0xa4`)
acts in phase 5 of the first round after the stop is lifted.

## Per-side gate `FUN_004fd600`

At `0x4f9e79`..`0x4f9ea4`: `FUN_004fd3d0` (hook `DAT_006b2af0` =
`0x4fd060`) returns the scheduler's `+0x2c`, which `FUN_0051d800` sets to 1
when setup ends. `FUN_004fd600(side)` is called on the caller's context
(`[esp+0x30]`, the same context passed on to `+0x15c`) and returns its
`+0xc` for side 1 or `+0x10` for side 2. For an order, `Event201_Fire`
builds that context with `FUN_004fd450(order + 0x20)`, which copies the
order's `+0x24..+0x38`; so the gate reads the order's `+0x2c` and `+0x30`.
`FUN_0051fa20` builds an embedded object at `+0x2c` (`FUN_004f4340`,
vtable `0x0065d040`, word 1 cleared by `FUN_005f4ed0`). hyp: the copy
treats that object's words as the two side flags; what fills them is not
traced. This stays open.

## The port

- Before 2026-11-17 the port applied every `PanelAction` in the frame it
  was given, paused or not. The pause alert holds the command center
  (`draw_pause_alert`), but the move confirmation (`Order::Tooltip`) sits
  above that hold, so a confirmation open when a pause was chosen could
  still move the HQ while held.
- Now `GameClock::holding` says a pause holds the clock (its stop day
  reached), and `rebellion_render::panels::admit_orders` keeps the
  player's orders (`PanelAction::is_order`) while it holds and applies
  them, in order, in the first frame the clock runs. Window, save, speed
  and play-test actions act at once. Tests:
  `orders_given_while_a_pause_holds_wait_until_the_game_runs`,
  `orders_given_while_the_game_runs_act_at_once`,
  `window_save_and_speed_actions_are_not_orders`,
  `a_pause_holds_only_once_its_day_has_finished`.
- port: the port pauses at a day, not a sub-tick; a pending pause still
  applies orders at once, as the original's step runs until the stop.
- port: a held order's refusal shows when it is applied, not when given
  (the original validates at both). Held orders are not saved, and a load
  or a new game drops them; whether the original saves `+0xa4` is not
  traced.
- port: the confirmation answers main.rs applies itself (Scrap, Abort
  mission) bypass `PanelAction` and are not held.
- port: the new game's clock at `GameSpeed::Paused` (no alert) is not a
  hold; orders act at once there.
- The starting speed (2026-11-24): a new game's setup
  (`Class_0065ad70_vt10`, after the agent's `FUN_00439950`) sets the rate
  to 2, Slow (`FUN_00487eb0(2)`), while the game's `+0x14` holds the
  galaxy through the briefing; the briefing's action 0xb
  (`FUN_004c30c0`/`FUN_004c0fc0` case 0xb -> `FUN_0041dbe0` ->
  `FUN_0041e320`) clears the hold, so day 1 runs at Slow. With advice on,
  action 0xd then opens the index on Advice, whose hold drops to Very Slow
  until it closes (`FUN_00487ff0`). The user confirmed it on the original
  (Windows VM): at Expert, with no advice window, the game runs at once at
  Slow. Port: `advisor_actions::release_new_game_hold` sets Slow at a new
  game's first briefing 0xb.
- The map reads the HQ position every frame, so the HQ mark moves in the
  frame its order is applied, as `FUN_0051df30` flushes the notice in the
  frame it executes the order.
