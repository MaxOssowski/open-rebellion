---
title: "Combat System"
description: "Space and ground combat resolution via combat.rs and bombardment.rs"
category: "agent-docs"
created: 2026-03-14
updated: 2026-10-16
tags: [combat, space-combat, ground-combat, simulation]
---

# Combat System

`combat.rs` + `bombardment.rs` — Space and ground combat resolution.

## Trigger

Opposing fleets coexist at a system (checked each tick in `main.rs`). Per-system 5-tick cooldown prevents infinite re-trigger on draws.

## Types

| Type | Purpose |
|------|---------|
| `CombatSystem` | Stateless resolver — `resolve_space()` and `resolve_ground()` |
| `SpaceCombatResult` | `{ winner, attacker_fleet, defender_fleet, ship_damage, fighter_losses }` |
| `GroundCombatResult` | `{ winner, system, troop_damage }` |
| `ShipDamageEvent` | `{ fleet, ship_index, hull_after }` — hull=0 means destroyed |
| `FighterLossEvent` | `{ fleet, fighter_index, squads_before, squads_after }` |
| `TroopDamageEvent` | `{ troop, strength_after }` — strength ≤ 0 means destroyed |
| `CombatPhaseFlags` | Custom bitfield (NOT `bitflags!`) — uses `.set()` not `.insert()` |
| `CombatSide` | Attacker / Defender / Draw |
| `bombardment` | `resolve(world, &BombardRequest, draw)`, `apply`, `BombardEnv` (a tick's draws and outcomes) |

## API

```rust
let result = CombatSystem::resolve_space(&world, atk, def, sys, difficulty, &rolls, tick);
let ground = CombatSystem::resolve_ground(&world, sys, alliance_attacks, &rolls, tick);
let outcome = bombardment::resolve(&world, &request, &mut draw);
bombardment::apply(&mut world, &request, &outcome);
```

## Space Combat: 7-Phase Pipeline

From Ghidra RE of REBEXE.EXE:

1. **Weapon fire** — turbolaser/ion/laser per arc, damage rolls
2. **Shield absorption** — shield_strength absorbs damage
3. **Hull damage** — remaining damage reduces hull_current
4. **Fighter engagement** — squadron vs squadron, torpedo bombing
5. **Result** — winner determined by surviving fleet strength

Key gotchas from RE:
- Phase gate: `ACTIVE && !PHASES_ENABLED` (inverted from initial expectation)
- Alt-shield path: exact `== 0x71` family byte (not range `0x71..=0x72`)
- Fighter losses must be computed from initial vs surviving counts

## Bombardment

Only the Fleet menu's Planetary Bombardment orders bombard (`0x220`
Target Military Facilities, `0x221` Target Civilian Facilities, `0x222`
General Bombardment, `0x223` Destroy System); nothing bombards after a
battle. The game resolves an order in four phases (`FUN_0058d770`): a Death
Star destroys the system (`0x223`); the ion cannons (DEFFACSD family `0x22`)
drain the bombarding ships' shields and stop them bombarding, and the
batteries (`0x23`) damage the hulls past them; the ships' bombardment
modifiers and squadrons less the planet's shields (`0x24`) give the shots,
each destroying a drawn target whose bombardment defense loses to a 1-10
draw; civilian damage costs the attacker 20 support. The AI's plan objects
order it (`AIAction::Bombard`); the player's Bombard button orders General
Bombardment. See `ghidra/notes/bombardment-order.md`.

## Effect Application (main.rs)

- `apply_space_combat_result`: Maps destroyed hulls (hull_after == 0) to fleet `capital_ships` count decrements. Removes empty fleets from system fleet lists.
- `apply_ground_combat_result`: Updates `TroopUnit.regiment_strength`. Removes destroyed troops from system and arena.

## Source

- `ghidra/notes/space-combat.md` — full 7-phase pipeline
- `ghidra/notes/ground-combat.md` — troop iteration
- `ghidra/notes/bombardment-order.md` — the bombardment orders and their resolution
- `ghidra/notes/combat-formulas.md` — GNPRTB parameter mapping
