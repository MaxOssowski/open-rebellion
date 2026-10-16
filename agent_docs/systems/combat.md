---
title: "Combat System"
description: "Space combat, bombardment and Planetary Assault resolution via combat.rs, bombardment.rs and assault.rs"
category: "agent-docs"
created: 2026-03-14
updated: 2026-10-16
tags: [combat, space-combat, ground-combat, simulation]
---

# Combat System

`combat.rs` + `bombardment.rs` + `assault.rs` — space combat, Planetary
Bombardment and Planetary Assault resolution. There is no other ground
battle: regiments of two sides on one planet stand together.

## Trigger

Opposing fleets coexist at a system (checked each tick in `main.rs`). Per-system 5-tick cooldown prevents infinite re-trigger on draws.

## Types

| Type | Purpose |
|------|---------|
| `CombatSystem` | Stateless resolver — `resolve_space()` |
| `SpaceCombatResult` | `{ winner, attacker_fleet, defender_fleet, ship_damage, fighter_losses }` |
| `ShipDamageEvent` | `{ fleet, ship_index, hull_after }` — hull=0 means destroyed |
| `FighterLossEvent` | `{ fleet, fighter_index, squads_before, squads_after }` |
| `CombatPhaseFlags` | Custom bitfield (NOT `bitflags!`) — uses `.set()` not `.insert()` |
| `CombatSide` | Attacker / Defender / Draw |
| `bombardment` | `resolve(world, &BombardRequest, draw)`, `apply`, `BombardEnv` (a tick's draws and outcomes) |
| `assault` | `resolve(world, &AssaultRequest, draw)`, `apply`, `order_enabled`, `BombardEnv::assault` |

## API

```rust
let result = CombatSystem::resolve_space(&world, atk, def, sys, difficulty, &rolls, tick);
let outcome = bombardment::resolve(&world, &request, &mut draw);
bombardment::apply(&mut world, &request, &outcome);
let assault = env.assault(&mut world, &mut transport, side, &fleets, system);
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

## Planetary Assault

Only the Fleet menu's Planetary Assault (`0x234`) fights on the ground
(`FUN_00563390` -> `FUN_0058c120`): the system's defenses fire at the
landing regiments (attack / 5 percent each); each attacking regiment duels a
drawn defender (a 0..10 draw plus TROOPSD attack less defense; the attacker
falls at or below 4, the defender at or above 6); each duel may destroy a
facility, energy or raw unit (8%); with no defender left up to 6 attackers
land. Regiments carried to a system land only on their side's standing,
unblockaded planet (`FUN_005073d0`); one side's regiments alone take a
planet (port: the occupation, `simulation.rs` step 3b and
`tactical_flow::land_and_occupy`). The AI orders it with
`AIAction::Assault`. See `ghidra/notes/planetary-assault.md`.

## Effect Application (main.rs)

- `apply_space_combat_result`: Maps destroyed hulls (hull_after == 0) to fleet `capital_ships` count decrements. Removes empty fleets from system fleet lists.

## Source

- `ghidra/notes/space-combat.md` — full 7-phase pipeline
- `ghidra/notes/planetary-assault.md` — the assault order and its resolution
- `ghidra/notes/ground-combat.md` — superseded (its family ranges are wrong)
- `ghidra/notes/bombardment-order.md` — the bombardment orders and their resolution
- `ghidra/notes/combat-formulas.md` — GNPRTB parameter mapping
