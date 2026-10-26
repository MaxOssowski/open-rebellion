# A unit fires at its force's target or nearest enemy (P24)

Date: 2026-10-26. This follows `../2026-10-26-weapon-turn-gate/`. Before
this stage the port chose targets on its own: a capital ship with no
target fired at the strongest enemy capital ship in range, and a fighter
group with no target at the strongest capital ship or group. In the
original, a unit fires only at:

- its force's target;
- its force's nearest enemy;
- and, with laser cannons or on Attack Fighters, at the enemy fighter
  groups nearby.

No unit picks a capital ship as a target by itself.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository, checked against the `objdump` disassembly).

## The original

### The force's nearest enemy

Each turn, slot `+0x10` of each force (`FUN_005c65e0`, run from `+0x84`
before the units' `+0x88`) calls `FUN_005cb6e0`. It clears `+0xa4` and
refills it from `FUN_005aa970`:

- the class comes from the force's first member (`FUN_005c6070`):
  - a fighter group when it carries laser cannons (`FUN_005b7ce0`);
  - a capital ship when it carries only ion cannons or turbolasers
    (`FUN_005b7d60`);
  - none when it is unarmed (`FUN_005b7d10`);
- the distance is measured from the mover's anchor (`+0x54` + `0xc`,
  `FUN_005a2260`: `+0x30..+0x3c`);
- the candidates are the enemy units in the fight, in the order of the unit
  list `+0x988`. A strict `<` keeps the first of equal distances. A
  fallback loop starts from a null node and never runs.

`+0x34` (the force is remote) must be clear. `FUN_005c9db0` copies it from
the first unit's `+0x124`, which is always clear in single play.

### `FUN_005b3a40` past its gate

`0x5b3aaa`..`0x5b3ef8`:

1. `T` is the first unit of the force's target list `+0x8c`
   (`FUN_005d2190`).
2. With `T`:
   - slot `+0x84` (`FUN_005b4c60`) is called with `T`'s class. When it
     returns 4 (no arc can hurt that class), the turn ends. Ion cannons
     count against capital ships only;
   - otherwise `r` is the direct shot at `T` (slot `+0xa8`,
     `FUN_005b6530`).

   Without `T`, `r` starts at 2.
3. While `r` is not 1, 3 or 4, the direct shot goes to each unit of
   `+0xa4`, the nearest enemy.
4. `FUN_005b2140` (the tractor beam) is called with `T`. The next stage
   covers it.
5. On order 4 (Attack Fighters), or with laser cannons and `r` = 2, the
   unit searches (`0x5b3ba0`):
   - `FUN_005cdf50` is called on the battle's fighter tree `+0x900`. It
     takes the enemy groups in the fight, not in state 6, inside a box of
     the longest range. It walks out from the unit by x: down from it,
     then up;
   - for each group, the arc toward it (the ±45° tests, or the fore arc
     when `+0x140` = 1, as for groups, `FUN_005b9c60`) must be ready. Its
     laser cannons and turbolasers within their ranges offer it.

     A family takes the group when the strength it keeps is below the
     family's full strength. It keeps that strength scaled by the hull
     (`FUN_005b1770`). So a whole unit keeps the first group it walks to,
     and a damaged one keeps the last;
   - `FUN_005b3f10` fires.

### The direct shot, `FUN_005b6530`

| Result | When |
|---|---|
| 1 | a fighter target and the fore arc (`+0x1e0`) not ready |
| 2 | beyond the longest range of an installed family (`FUN_005b18d0`), compared squared |
| 1 | the arc toward the target not ready |
| 3 or 4 | `FUN_005b3f10`'s result: 3 when nothing fired, 4 when something did |

The arc's families within range take the target with no hull scale. Ion
cannons take only capital ships.

### Firing, `FUN_005b3f10`

Until no arc offers a positive total:

- the first arc with the strictly highest total fires every family it
  holds, in the order ion, laser, turbolaser;
- the arc becomes not ready, loses each fired family's count of energy,
  and enters the recharge queue once.

## The port

`BattleSession::fire_unit_weapons` follows the steps above for capital
ships and fighter groups alike:

| Port function | Original |
|---|---|
| `force_target` | the force's target `+0x8c`: the unit's attack target while it is an enemy in the fight |
| `force_nearest_enemy` | `FUN_005cb6e0`; `force_lead_and_anchor` gives the first member and the mover's anchor |
| `direct_fire` | `FUN_005b6530`, with `OriginalDirectFire` for its results 1 to 4 |
| `fire_at_nearby_fighters` | the search |
| `fire_candidates` | `FUN_005b3f10`; a group fires its torpedoes after its guns at a capital target without shields (`push_fighter_torpedo`), as before |

Removed, since the battle no longer used them:

- `fire_side`, `original_capital_weapon_candidates` and their candidate
  types;
- `fire_capitals_at_fighters`, `fighter_step`, `fighter_target`,
  `fighter_target_in_range` and `fighter_target_strength`.

A capital arc that fires at a fighter group now loses only the fired
families' energy, as in `FUN_005b3f10`. The port used to empty it.

`port:` choices:

- The force's nearest enemy is found when the unit fires, not in the
  force's turn slot. Nothing moves between the two, so the result is the
  same.
- `FUN_005cdf50`'s box holds every group a family can reach, so the port
  walks all enemy groups in the same order. Only that order shows: it
  decides which group a family keeps.
- The units in the fight are those not jumping, in the port's order.
- The Death Star run (order 6) keeps its own producer, as before.

## Changed tests

| Test | Change | Evidence |
|---|---|---|
| `recovered_fighter_clears_attack_fighters_without_capital_fire_or_audio` (render) | Its "autonomous" ship, with lasers and no order, fired at a capital ship. Now that ship fires at nothing, and the contrast is the same ship with the capital ship as its target. | `FUN_005cb6e0`, `FUN_005b3a40` (`+0x900`) |
| `live_ship_fire_uses_the_shared_subsystem_damage_path` (render) | Its ship now has the target as its force's target. | `FUN_005b6530` |
| `automatic_weapon_callback_keeps_first_arc_on_ties_and_fires_each_ready_arc` (render) | The two targets are now fighter groups, on Attack Fighters. Each assertion stands, with targets compared by position. | `FUN_005cdf50` on `+0x900` |
| `weapon_candidates_enforce_range_and_apply_hull_scaling_only_to_auto_fire` (render) | It reads shot strengths: the direct shot at a capital ship (26.67, unscaled), and the search at a group (× 0.5 for a turbolaser against fighters, × 0.5 for the hull). The range boundary is unchanged. | `FUN_005b6530`, `FUN_005b1770`, `FUN_005d2490` |
| `the_players_withdrawal_ends_after_the_sides_time` | Its "damaged" ship had condition 0 with no hits. The idle turbolaser ships now fire at their nearest enemy, and each hit refreshes every ship's condition from its hits, which set it back to 100. The ship now takes all its hyperdrive hits, so the assertions stand. | `FUN_005b1970`, `FUN_005cb6e0` |
| `a_jumping_fighter_group_drops_out_of_the_fight` | The removed `fighter_target` checks are now "fires at someone" through `fire_unit_weapons`. | `FUN_005ad870`, `FUN_005cb6e0` |

The other weapon tests now call `fire_unit_weapons` in place of the
removed functions. Their assertions are unchanged.

## New tests

In `abstract_battle.rs`:

- `an_idle_force_fires_only_at_its_nearest_enemy`;
- `the_nearest_enemy_is_measured_from_the_forces_anchor`;
- `a_target_out_of_range_falls_to_the_nearest_enemy`;
- `no_weapon_against_the_targets_class_ends_the_turn`;
- `attack_fighters_fires_at_its_target_then_at_groups_nearby`;
- `the_search_keeps_the_first_group_when_whole_and_the_last_when_damaged`;
- `the_search_takes_enemy_groups_in_range_of_a_ready_arc`;
- after the first mutation run:
  - `only_lasers_out_of_range_or_attack_fighters_search`;
  - `a_friend_or_a_dead_ship_is_no_target`;
  - `distances_take_every_axis_squared`;
  - `the_search_walks_out_from_the_unit_by_x`, for a capital ship and for
    a group on its mover's anchor;
  - `torpedoes_need_no_shields_range_and_a_rating`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2885 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T18-03-48-552Z-1255165`; Arch Chromium 155, not the pinned build) |
| Replay equivalence | passed |
| `cargo mutants` | 147 tested: 131 caught, 6 unviable, 2 timeouts, 8 missed (first run: 28 missed) |
| Release WASM SHA-256 | `0d36e1cb2b001f1a01b0ab25e52edbb45e8702a0ce372c6309d6171257aad83e` |

The two timeouts are `<` replaced in `fire_candidates`' strongest-arc test.
That loop then never ends, so the mutants were detected.

One of the 8 missed mutants was a gap: `delta.x * delta.x` with `+` in
`force_nearest_enemy`. `distances_take_every_axis_squared` now covers it,
and the mutant, applied by hand, fails it. The other 7 are equivalent:

- `unit_has_weapons_against` returning `true`, and its `&&` replaced by
  `||`. A unit that cannot hurt the target's class shoots nothing at it
  anyway. The early return only shows with the tractor beam, which comes
  next;
- `* ORIGINAL_LASER_VS_FIGHTER` replaced by `/`: the factor is 1.0;
- the ion arm of `family_strength` against fighters, twice: it is never
  reached, since ion cannons are left out against groups first;
- `at + 1` replaced by `at * 1` in a group's walk: the group's own node is
  then walked, but it is a friend and is left out;
- `hull_max > 0.0` replaced by `>=`: every live group has hull.
