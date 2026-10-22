# The app's battles take every fleet at the system (P23, P24)

Date: 2026-10-23.

## The original

`FUN_0040c780` applies a battle's result to every fleet at the system
(`FUN_004ffe70`), so every fleet of both sides fights. The headless
campaign already did this (`resolve_system_space_combat`).

## Before

The app passed only the two fleets that named the battle to:

- AI-against-AI battles (`resolve_space_battle(.., &[atk], &[def])`);
- the player's Simulate Results and Retreat (the same);
- Take Command's tactical battle (`BattleSession::new` with one fleet per
  side), whose result went back to those two fleets only.

## Change

- `abstract_battle::side_fleets(world, system, lead)`: the lead fleet,
  then every other fleet of its side at the system.
- All three paths pass every fleet of both sides.
- Each tactical unit carries its strategic fleet (`TacticalShip::fleet`,
  `TacticalFighter::fleet`), and `tactical_flow::apply_results` writes
  each fleet's hulls, hit counters and squadrons back from its own units.

## Gates

- Workspace 2684 passed, 0 failed.
  `every_fleet_at_the_system_fights_and_takes_its_own_result` fails
  without the change (the battle held 2 fleets, not 3).
- Mutants on the touched core and render code: 7 tested, 6 caught,
  1 missed (`TacticalState::begin_battle` -> `()`). The render package has
  no test that enters a battle; the rebellion-app tests
  `both_player_sides_enter_the_same_production_tactical_path` and
  `every_fleet_at_the_system_fights_and_takes_its_own_result` fail under
  that mutant (checked by hand; cargo mutants ran each package's own
  tests).
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `9ce3a4f6e16037816f35b1ebbb93df815e6a249e04f0b082aedb0082b1e914e3`.
