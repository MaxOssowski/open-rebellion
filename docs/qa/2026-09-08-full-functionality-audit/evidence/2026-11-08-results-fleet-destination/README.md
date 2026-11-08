# The results page's Fleet destination opens the fleet with its roster

Date: 2026-11-08 (night). Cell: TAC-07-C012 (go to fleet).

## What was open

A0 TAC-07-C012 (Wine) shows the "go directly to" page with its
Coruscant and Fleet buttons. Since b126 the port's page has matched it
0 pixels off in the pictures and buttons, and its text is solid red. The
ledger left one thing open: the fleet destination's roster confirmation.
The gate had checked only the route's log line.

## Now

When the Fleet button routes (`TacticalAction::OpenBattleFleet`), the
app:

- opens the battle fleet's Fleet window with the fleet chosen, as the
  Fleet Finder's Display does (`FUN_00429440`);
- logs `[tactical-results] fleet_window system=… ships=N selected=fleet`.

`run.mjs` (production-battle-results-presentation, the Empire's route)
finds that line once, with the battle's two surviving hulls of the
fixture's three. Linux, 640x480 and letterboxed.

## Not compared

The original window that this press opens has no A0 capture. The port
follows `FUN_00429440`'s object branch, as the Fleet Finder's Display
does.
