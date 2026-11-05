# Battle Alert: the Simulate Results route, against A0 (2026-11-05)

This covers EVT-02-C006 ("simulate").

## A0

The captures are in `.artifacts/a0-vm/`.
- `message-rail/battle-averam-result` (the Expert game, day 43). Simulate on the Battle at Averam alert resolves the battle and shows its results over the strategy view: "Battle at Averam", "The Alliance fleet is defeated.", and the blockade and withdrawal text.
- `2026-10-27-controls/results-summary-sim`. The same window for Yavin.

## Port

`run.mjs --battle`, scenario `production-battle-alert-entry` (Linux, both sides):
- **640×480.** The scenario still takes Take Command into the paused tactical battle, with the alert's art, tabs and scroll bar checked as before.
- **1280×800 letterboxed.** It now takes Simulate (278, 363.5) instead.
  - The app logs `[battle_alert] command=simulate … status=routed`, and no take_command.
  - The frame changes to the Battle Results window over the strategy view. For the Alliance it reads "Battle at Akrit'tar", "The Alliance fleet is defeated.", and "The Imperial fleet was victorious at the neutral world Akrit'tar. The Alliance fleet has been completely destroyed." That is the same window as A0's.
  - Pixels are not compared at that scale (`verifyStrategyBitmap` checks only 640×480); the window's art is compared in TAC-07.
  - That route never enters the tactical scene, so its run skips the planet and participant-family checks. The 640×480 run keeps them.

Gate run: 2026-11-05T16-32-53-653Z-1596731, all four executions pass.

## Retreat (EVT-02-C005, still partial)

**A0** (a new Expert Alliance game, day 37: `message-rail/retreat-alert-commenor`, `retreat-result-commenor`). Retreat on the Battle at Commenor alert shows the same results window: "The Alliance fleet is defeated." "Commenor is now under blockade by Imperial forces. The Alliance fleet has withdrawn to Bortras."

**Port.** The Empire's letterboxed run now presses Retreat (144, 363.5). The app logs `[battle_alert] command=retreat … status=simulated` and does not enter the tactical scene, and the results window shows: "The Imperial fleet is defeated. … has been completely destroyed."

**The outcome.** It follows the abstract battle: a side with the retreat order leaves only with a working hyperdrive and once its strength falls under 0.33 of the enemy's (`FUN_005d0f50`, `FUN_005d19a0`). The fixture's Imperial fleet died before it could leave, while A0's Alliance fleet withdrew. The forces differ, so the outcome is not compared.

Gate run: 2026-11-05T16-35-34-366Z-1600102, all four executions pass.
