# Weapon shots go through the turn queue (P24)

Date: 2026-10-25. Part of rebuilding the battle's turns
(`ghidra/notes/tactical-event-turns.md`).

## Source

Static reading of `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`).

- **The decision.** `FUN_005b3f10` (from the turn's unit update,
  `0x005a9de0` → slot `+0x88` → `FUN_005b3a40`) decides each shot. It
  spends the arc's energy at once (`0x005b41cf`..`0x005b41ee`). For each
  weapon family it posts (`0x005b40c3`..`0x005b41ae`):
  - a fire event (`0x0d` laser, `0x0e` turbolaser, `0x0f` ion, `0x10`
    torpedo), stamped `+0x87c + 1.0` (`DAT_0066c4d0`), built by
    `0x005ce460`;
  - a hit event (`0x11` laser, `0x12` ion, `0x13` turbolaser, `0x14`
    torpedo), stamped `+0x87c - 1.0 * -2.0` (`DAT_0066c510`), so
    `+0x87c + 2`.
- **The fire event.** Its handler (`FUN_005b1ea0`, `FUN_005b1f60`,
  `FUN_005b2080`, `FUN_005b2c70`) checks both units are still in the fight
  (`FUN_005ad870`). It then calls the target's slot `+0xc8` and starts the
  projectile (`0x005c12d0`).
- **The hit event.** Its handler (`0x005b2a20`, `0x005b2a70`, `0x005b2ac0`,
  `0x005b2b10`) checks the target is still in the fight. It then calls the
  target's slot `+0xe0`, `FUN_005b5f50` for both capital ships and fighter
  groups, with the strength and both ids.

A shot decided in turn k (stamp k + 2 within the turn's updates) therefore
shows its projectile at turn k + 4, and its damage lands at turn k + 5.

## Change

- `fire_side`, `fire_capitals_at_fighters` and `fighter_step` now only
  choose the shots and spend the energy. Each shot is a `TacticalShot`
  (source, target, family, strength).
- `run_turn_updates` posts each shot's `WeaponFire` at the step's stamp
  + 1 and its `WeaponHit` at + 2.
- `fire_weapon_shot` (the fire event):
  - with both units in the fight, queues the projectile;
  - plays the fire sound.
- `hit_weapon_shot` (the hit event), with the target in the fight:
  - draws the damage and applies it (capital ship, fighter group or Death
    Star);
  - queues the impact;
  - plays the impact sound and any destruction voice.
- Before, the damage, impact, projectile and both sounds all came at the
  decision.
- `hyp:` the fire and impact sounds' variants are drawn in their events.
- Not yet changed: `FUN_005b5f50` itself posts the subsystem hits
  (`0x1e`..`0x22`) and the destruction (type 3) through the queue
  (`0x005b6164`..`0x005b624e`). The port still applies them in the hit
  event.

## Tests

- New: `a_shot_fires_and_hits_on_its_turns_while_its_units_fight`:
  - a shot decided at step 4 is stamped 4 (fire) and 5 (hit);
  - a source out of the fight shows nothing;
  - a target out of the fight takes nothing.
- Changed on this evidence:
  - the direct battery tests resolve their shots through the two events.
    Their sound order becomes the queue's: every fire sound (turn + 1),
    then every impact sound (turn + 2);
    - `direct_weapon_callback_fires_every_family_and_queues_the_arc_once`,
      `automatic_weapon_callback_keeps_first_arc_on_ties_and_fires_each_ready_arc`,
      `live_ship_fire_uses_the_shared_subsystem_damage_path` and
      `explicit_attack_order_never_redirects_an_invalid_target`;
    - `fighter_fore_battery_fires_source_families_and_torpedoes_at_capitals`;
    - `dogfight_ignores_ion_and_applies_target_maneuverability`;
    - `capital_attack_fighters_uses_typed_target_without_capital_fallback`;
  - `recovered_fighter_clears_attack_fighters_without_capital_fire_or_audio`:
    the projectile appears at step 20 and the damage at step 24;
  - `losing_the_committed_death_star_attackers_routes_failure_once`: the
    battle ends at step 24;
  - `a_withdrawing_ship_can_be_hit_until_its_jump_starts`: the hit is read
    from its impact at step 24. The turn's recharge after the events can
    refill a one-point hit on the shield;
  - `dead_and_jumping_ships_drop_out_of_the_fight`: 24 steps.
- New: `shot_events_reach_each_kind_of_target_while_it_fights`:
  - a fighter group takes a hit only alive, with craft and not jumping,
    with the capital source's maneuverability + 1;
  - a Death Star takes a hit only with hull and not destroyed, and is
    destroyed at 0;
  - a torpedo is its rating times `ORIGINAL_FIGHTER_TORPEDO_STRENGTH`, and
    its impact plays variant 0 without a draw;
  - each sound draws its variant from 0 through its count − 1, in the
    pinned order.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2784 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| `cargo mutants` | 42 tested; 18 missed in the first run, 3 after the new test, which a variant-sequence check then caught (checked by hand) |
| Release WASM SHA-256 | `2ab4bbbb1a8276a162ec7e098c7a85da7822b7d9e4a954155d0e50dc9c09d83e` |

The new test came after the browser run, the WASM and the equivalence
check; it changes no production code.
