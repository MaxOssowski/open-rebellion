# Mission object targets in the browser (F-019 phase 7h)

Date: 2026-10-14. Runner: Claude Code with `agent-browser` 0.38.1 driving
headless Chromium 153.0.8010.52 (Arch Linux) at 640 by 480. Driver:
`mission-flow.py PORT OUT SCENARIO_INDEX FACTIONS`. Test-only interface WASM
from `scripts/build-interface-test-wasm.sh`; fixture code = scenario index
+ 1, with the side in the next byte.

Each run right-clicks the player's free agent in the right-hand Defenses
window, chooses Mission, clicks the target in the left-hand window
(`3-targeting.png`), reads the dialog (`4-dialog.png`, report `dialog`),
presses Begin Mission and reads the active missions (`5-after-begin.png`,
report `after_begin`).

| Scenario (code) | Side | Target window and object | Kinds offered | Named | Active after Begin |
|---|---|---|---|---|---|
| `RescueTargeting` (58) | Alliance, Empire | Defenses, a prisoner of the player's side | Rescue | Leia Organa | Rescue → Leia Organa |
| `SabotageTargeting` (59) | Alliance, Empire | System, Shipyards page, an enemy shipyard | Sabotage | Orbital Shipyard | Sabotage → ManufacturingFacility |
| `DeathStarTargeting` (60) | Alliance | Fleet, the Death Star in the selected enemy fleet's ship list | Death Star Sabotage | Death Star | DeathStarSabotage → DeathStar |

Every run: 4 of 4 requests HTTP 200, 0 console errors or warnings. The
screenshots were inspected; the empty target frame is the known port gap
(no object portrait). Death Star Sabotage is an Alliance-only record, so
its scenario runs for the Alliance alone.

The production `web/open-rebellion.wasm` built with these sources passes
`scripts/check-replay-equivalence.py --skip-build --json`.

Hashes are in `SHA256SUMS`.
