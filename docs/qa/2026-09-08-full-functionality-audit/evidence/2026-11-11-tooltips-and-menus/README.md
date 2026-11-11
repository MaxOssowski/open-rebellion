# Tooltips, the manufacturing band menu and the window rail: A0 search

Date: 2026-11-11. Cells: CMD-11-C017, CMD-11-C029, CMD-11-C030,
OBJ-14-C008, CMD-04-C018, CMD-09-C010 and CMD-10-C010. The ledger JSON is
not edited here.

## What was searched

Every folder in `.artifacts/a0-vm/` (git-ignored) was listed. The ones
that could hold these states were opened: `tooltip/` (every file by
name, 311..318, 300, 301 and 350..358 by eye), `menus3/28`, `29-assault-1..4`,
`assault-clean/`, `assault-defender-day902/`, `modal/`, `modal2/`,
`battle-clean/`, `battle-display/`, `load-in-game/`, `load-from-menu/`,
`game-options-*`, `2026-10-27-controls/`, `2026-10-28-ctrl-squadrons/`,
`2026-10-24-*`, `pending-facility/` and the files whose names hold
"tip", "assault", "bombard", "training" or "advanced".

None of them shows the missing state of any of these cells. Details are
below, cell by cell.

## OBJ-14-C008 (manufacturing band menu): the agent now reads Reserved

The band menu itself already matches A0 (`menus3/30..33`,
`pending-facility/33`, `36`), and `build-selection.mjs` checks it on
Linux. The last open item was "no agent reads Reserved".

**The original.**
- The manual, shipyard menu: Reserved "Prevents your agent from using
  that shipyard for any of its automated management functions. It may,
  however, still use that shipyard when you select Build Ships from the
  Agent menu." The training facility menu says the same for Manage
  Garrison.
- `FUN_004ebe40` builds the agent's system records. It sets bit `0x400`,
  `0x800` or `0x1000` when the shipyard, training or construction yard
  manager is Reserved (manager `+0x70`). The idle bits `0x80`, `0x100`
  and `0x200` do not look at Reserved.
- `FUN_004c6460` keeps a record only when it has every required bit and
  none of the excluded bits (`(bits & param_4) == 0`).
- Manage Production (`FUN_004c6d90`) asks `FUN_0049da10(.., 0x204, 0,
  0x1000, ..)`: an idle construction yard, not Reserved.
- Manage Garrisons (`FUN_004c7860`) asks `FUN_0049da70(.., 0x104, 0,
  0x800, ..)`: an idle training facility, not Reserved.
- The agent menu's Build orders (`agent-build-orders.md`) use only the
  idle bits, so they still use a Reserved producer. The port's
  `agent_build.rs` already does that.

**The port before.** `agent_automation.rs` picked the first owned
system with a yard, Reserved or not.

**The change.** `run_production` skips a system whose construction yard
area is Reserved. `find_training_system` skips a system whose training
area is Reserved.

**Tests.** Both fail without the change and pass with it:
- `agent_automation::tests::production_skips_a_reserved_construction_yard`
- `agent_automation::tests::garrisons_skip_a_reserved_training_facility`

`cargo test -p rebellion-core --lib`: 1730 passed. `provenance-scan
check`: ok, 0 uncited items. Mutants: see "Mutants" below.

**Browser.** There is no browser check of the agent's choice. The
agents' orders are not visible on one screen; a test would need several
days of game time with the module on. The band menu's Reserved row
(enabled, its check mark, 107 wide) is already checked by
`build-selection.mjs` against A0 `menus3/30..33`. No new gate run was
made for this change.

## Cells with no A0 of the missing state

Each needs a new capture in the Windows 10 VM. Keep the pointer still,
wait one second for the tip, and take one capture without the tip
(`-base`) as `tooltip/` does.

**CMD-11-C017 (Assault Summary tips).** The battle summary's tips are
in A0 (`tooltip/350..358`), but the Assault Summary's are not.
`menus3/29-assault-1..4` show the summary with the pointer on the
picture and no tip. Needed, in the Alliance game, from the Conflict list
as `assault-clean/01`:
1. the pointer on each of the four rail buttons: Summary (0x1944),
   Imperial Forces (0x1938), System Forces (0x1943), Display (0x1939);
2. on the System Forces page, the pointer on each of the six category
   buttons: Capital Ships (0x1928), Fighter Squadrons (0x1929),
   Manufacturing (0x1835), Defense Facilities (0x1946), Trooper Regiments
   (0x1930), Personnel (0x1931).
The `fleet-window.mjs` case `enemy-assault-taken` already opens the
same report over A0's background, so a tip check can be added there once
the captures exist.

**CMD-11-C029 (message box tips).** No capture shows the save notice
(REBDLOG 10622 with 0x1217 "Selected Game Saved Successfully" or 0x1219
"Selected Game Failed To Save"). Needed: Game Options, Save Game to a
slot, then the pointer on the notice's one button (176, 134), with and
without the tip. Another agent works on save and load, so this was left
alone.

**CMD-11-C030 (tactical side panels' tips).** The tactical captures
(`2026-10-27-controls`, `2026-10-28-ctrl-squadrons`,
`2026-10-24-first-tactical`) show no tip. Needed, in a paused Take
Command battle: the pointer on the capital-ship panel's Previous and
Next, its hull and shield bars, each of the five power buttons, the
maneuvers panel's Surround and Stand Off, the missions panel's buttons,
and the fighter-group panel's Recover.

**CMD-04-C018 (Battle Summary is modal).** `modal2` tests the Status
window, the Encyclopedia and the Message Index, not the Battle Summary.
`tooltip/350` shows the summary over the galaxy view, so modality can be
tested there. Needed: open a Battle Summary from the Conflict list (as
`battle-clean/01`), capture it, then click the GID control (537, 446),
right-click the agent droid, click a rail button and double-click a
planet in a sector window behind it, and capture after each. Repeat for
an Assault Summary. Note: the port's Assault Summary blocks only its own
rectangle (`main.rs`, `assault_summary_state.contains_screen_point`); it
is not in `modal_window_open`. If A0 shows the summaries are modal, both
need the gate the Status window has.

**CMD-09-C010 (advanced bases as built).** Only the Advanced Training
Facility under construction (9020) is in A0. Needed: a finished Advanced
Shipyard, Advanced Training Facility and Advanced Construction Yard, each
on its System window page (Shipyards, Training Facilities, Construction
Yards), for at least one side. They are built from a construction yard;
the VM game must run until they finish.

**CMD-10-C010 (disabled controls).** The original disables the seven
cockpit controls while a game loads. `load-in-game/00` shows only the
question and `load-from-menu/01` the game after the load. A0 `tooltip/
301`, `311..318` show the galaxy with live controls. Needed: a capture
during a load, after the galaxy view is created and before the game is
ready (wait cursor showing). A load in the VM is short, so this may need
a large save and several QMP screenshots taken one after another.

## Mutants

`cargo mutants --in-place -p rebellion-core --file
crates/rebellion-core/src/agent_automation.rs --re
'run_production|find_training_system' -- --lib`: 32 mutants, 31 caught,
1 unviable, 0 missed.

## Proposed status

| Cell | Proposal | Why |
|---|---|---|
| OBJ-14-C008 | pass | The band menu matches A0 and passes on Linux; the agent now skips Reserved producers as `FUN_004c6d90` and `FUN_004c7860` do. The agent's choice has no A0 or browser check of its own. |
| CMD-11-C017, C029, C030 | stay partial | No A0 of the tips. |
| CMD-04-C018 | stays partial | No A0 of clicks behind a summary. |
| CMD-09-C010 | stays partial | No A0 of a finished advanced base. |
| CMD-10-C010 | stays partial | No A0 during a load. |
