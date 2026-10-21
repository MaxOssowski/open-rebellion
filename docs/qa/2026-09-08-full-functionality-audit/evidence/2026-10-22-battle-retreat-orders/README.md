# The battle choice's retreat orders (F-033, W2k)

Date: 2026-10-22. Each side of a battle now answers the battle choice
(`FUN_004362b0`). The reading is in `ghidra/notes/abstract-battle.md`,
"The battle choice".

## Change

- The AI: `ai_strategy::orders_retreat` (`FUN_00485d50`, strategy slot
  `+0x10` = `FUN_004e77f0` and six copies) and `ai_strategy::retreat_orders`.
  - An AI with a strategy orders a retreat everywhere except at its own
    system with a known squadron on an explored, populated planet.
  - There it orders one only when its own fleets have attack (fleet
    record `values[2]`), and never at its headquarters or `0x109`.
- The player: the Battle Alert's Retreat (`0x67`, `FUN_00450540`) runs the
  abstract battle with the player's retreat order. Simulate Results gives
  none.
  - Retreat used to open the tactical battle with the player's ships
    marked as retreating; that path and its voice cue are gone.
- `resolve_space_battle` and `resolve_system_space_combat` take the two
  orders (`retreat_allowed`). The headless simulation, the app's AI
  battles and Auto Resolve pass them.
- The integrator's winner is now the battle's (`FUN_0040c780`), not the
  side left in orbit.
  - A side that escaped stays in orbit until `withdraw_loser` sends it
    away. The orbit rule called those battles draws, so nobody left and
    the battle repeated daily. The first gate run (`w2k`) showed this:
    5536 battles, 4882 draws.
- `port:` Take Command (the tactical battle) does not yet pass the AI
  side's order to the tactical engine.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days, against
a HEAD (`c31d0143`) worktree build. Every campaign changes
(`campaigns.json`). The cause is the retreat orders above.

| | Before | After |
|---|---|---|
| Space battles | 1043 | 2092 |
| Battles without any loss | 0 | 800 |
| Alliance wins | 256 | 741 |
| Empire wins | 786 | 1351 |
| Draws | 1 | 0 |
| Stalemates | 3 | 0 |
| Alliance capital ships lost | 278 | 25 |
| Empire capital ships lost | 224 | 6 |
| Campaigns won | 0 | 1 (seed 24, Alliance takes Coruscant, day 1350) |

A side that falls under a third of the enemy's strength (`FUN_005d19a0`)
now jumps out after the first second of fire (`FUN_005d14d0` step 5). Few
capital ships are lost, and fleets meet again more often.

## Gates

- 7 new tests:
  - `orders_retreat`: no strategy, away from home, the defended-system
    rule;
  - `retreat_orders`: each AI for its own side;
  - the order reaching the battle;
  - an escaped side losing while in orbit;
  - a side alone in orbit without a battle.

  Each of the first five fails with its change neutralized. Workspace:
  2539 passed, 0 failed.
- Replay goldens pass. The only failure is the long-standing
  `seeds::tests::fixed_fleet_tables_only_target_special_systems`.
- Mutants on the diff: the first run left 5 survivors, closed by the
  added tests; then 29 caught, 3 unviable, 0 missed (`mutants.txt`).
- `provenance-scan check`: 0 uncited.
- Browser, Arch Chromium 155.0.8059.84:
  - Fleet window 58 of 58 pass;
  - replay equivalence passed (`v1:e01dc3b9b044c28e` ->
    `v1:7737eb96955b5cd7`, no browser errors, 4 requests);
  - frame timing: very slow median 0.5 ms, p99 0.8 ms; fast median 0.5 ms,
    p99 1.7 ms; 0 frames over 50 ms.
- Release WASM: `6ff7839e09e2a7eee1beeba3cf72e1d290956de610f25869385299c15466bc9f`.
