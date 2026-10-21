# Strategic battles are abstract battles (F-033, W2f)

Date: 2026-10-21. Every strategic space battle the port resolves on its
own now runs the abstract battle engine
(`crates/rebellion-core/src/abstract_battle.rs`), as the original's state
8 of `FUN_0040a050` does. The port's round formula
(`CombatSystem::resolve_space`) no longer decides any battle.

## Wiring

- `BattleSession::new_for_fleets` builds one battle from every fleet of
  both sides at the system and records each unit's fleet.
  - `FUN_0040c780` applies a battle to every fleet there
    (`FUN_004ffe70(system, 3)`).
  - `new` now calls it with one fleet per side and is otherwise unchanged.
- `abstract_battle::resolve_space_battle` runs the battle. It returns the
  existing `SpaceCombatResult`: each hull's new value and each fighter
  entry's surviving squadrons, by fleet.
- Callers:
  - `integrator::resolve_system_space_combat` (headless campaigns, all
    fleets);
  - the app's AI-versus-AI battles and the player's Auto Resolve (the
    named pair, as before).
- `apply_space_combat_result_inner` now removes every empty fleet the
  result touches.
- The ship events run from the last hull to the first. The applier counts
  live hulls as it marks them dead; the first gate run showed one battle
  that destroyed five Imperial ships leaving two in the fleet. The test
  `every_hull_lost_in_one_battle_leaves_the_fleet` fails without this
  order.
- `port:` the battle's random stream is seeded from the campaign's first
  combat roll (`abstract_seed`).

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days.
Every campaign changes. Per-seed figures are in `campaigns.json`.

| | Before (f44) | After (f45) |
|---|---|---|
| space battles | 901 | 1043 |
| draws | 258 | 1 |
| stalemates | 236 | 3 |
| Alliance wins | 118 | 256 |
| Empire wins | 525 | 786 |
| capital ships lost | 401 | 502 |

- Battles now end with one side destroyed, as an abstract battle does.
  A drawn battle no longer holds a system under its cooldown, so more
  battles happen.
- No campaign reaches victory in 1500 days, before or after; both sides
  stay in every one.
- The seed-42 replay golden has no battle and is unchanged
  (`replay_manifest`: 2 passed).

## Integrator tests

Three tests described the old rounds. They now state the original's
rules, each with its source:

- `system_engagement_resolves_in_one_abstract_battle`: one battle,
  `rounds` = 1.
- `system_engagement_includes_every_orbiting_task_force`: unchanged
  expectations.
- `system_engagement_stall_destroys_two_equal_unarmed_sides`: two unarmed
  sides stall for 1200 seconds and are both destroyed (`FUN_005ae9c0`,
  `FUN_005af7a0`). Before, they were kept as a no-progress draw.

The fixtures now give their ships a weapon recharge equal to their attack
value. `FUN_005b7780` makes damage per second the weapons' mean strength
times the recharge, so a ship with no recharge never fires.

## Gates (`f45`)

| Gate | Result |
|---|---|
| Workspace tests (fixture feature) | 2549 passed, 0 failed |
| provenance-scan | 0 uncited |
| Mutants on the diff | 16 caught, 3 unviable, 2 equivalent (`mutants.txt`) |
| Replay equivalence | passed, 0 browser errors; release WASM `9cda77d995e1a6f21acaa78c85bc0f385b66b299eabebe87ca72f2cffc288f73` |
| Perf (very slow / fast) | 0 frames over 50 ms |
| Status window | pass |
| Tactical Death Star scenarios | 4 of 4 pass |
| Fleet window suite | 58 of 58 pass |

The Fleet window suite failed twice first, each time on 1–3 different
cases at "the fleet icon opens the Fleet window". Both runs overlapped a
`cargo mutants` build on the same machine. Run alone, the suite passed 58
of 58. Those fixtures cannot reach the changed code: with two factions,
every battle involves the player and opens the tactical view, and only
Auto Resolve and AI-only battles changed.
