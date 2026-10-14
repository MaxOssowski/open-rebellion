# Ship, squadron and fleet mission targets (F-019 phase 7i)

Date: 2026-10-14. Runner: Claude Code with `agent-browser` 0.38.1 and
headless Chromium 153 at 640 by 480; driver `mission-flow.py` (its
`nodialog` mode checks that no dialog opens and nothing is dispatched).

The creation checks (`ghidra/notes/mission-lifecycle.md`) leave a fleet and
an unfinished object with no class, and give an ordinary capital ship or a
fighter squadron to Sabotage.

| Scenario (code) | Sides | Clicked | Result |
|---|---|---|---|
| `ShipTargeting` (61) | Alliance, Empire | the enemy fleet's only capital ship in its Fleet window's ship list | Sabotage alone, named "Strike Cruiser" (Alliance) and "Mon Calamari Cruiser" (Empire); after Begin Mission, Sabotage → `CapitalShip` is active |
| `FleetTargeting` (62) | Alliance, Empire | the same fleet's entry in the left list | no dialog, no mission (`4-no-dialog.png`) |

Every run: 4 of 4 requests HTTP 200, 0 console errors. The screenshots were
inspected. Codes 58 to 60 (Rescue, Sabotage of a shipyard, Death Star
Sabotage) were rerun on the same build with unchanged results, and
`scripts/check-replay-equivalence.py --skip-build --json` passes.

Hashes are in `SHA256SUMS`.
