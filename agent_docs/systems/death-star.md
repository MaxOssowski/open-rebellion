---
title: "Death Star System"
description: "Planet destruction and the nearby-warning scan; the Death Star is built as an ordinary capital ship"
category: "agent-docs"
created: 2026-03-14
updated: 2026-10-20
tags: [death-star, planet-destruction, simulation]
---

# Death Star System

`death_star.rs` — planet destruction and the nearby-warning scan.

The Death Star is CAPSHPSD record 136 (TEXTSTRA 10120, research order 0), an
ordinary capital ship built by a shipyard; a completed one marks its fleet
(`Fleet::has_death_star`). The original has no separate construction timer,
so the port's 1,825-day countdown was removed (save v94).

## Types

| Type | Purpose |
|------|---------|
| `DeathStarState` | `death_star_fleet: Option<FleetKey>`, `shield_generator_active` |
| `DeathStarEvent` | PlanetDestroyed / NearbyWarning |

## API

```rust
// Each tick: scan for nearby warnings
let events = DeathStarSystem::advance(&mut ds_state, &world, &tick_events);

// Player/AI fires superlaser (precondition-checked):
let fired = DeathStarSystem::fire(&world, target_system, tick); // Option<DeathStarEvent>
```

## A destroyed system

`cleanup_destroyed_system` follows the original's cascade
(`ghidra/notes/bombardment-order.md`, "The destroyed system's contents"):
units and facilities go, fleets stay, the characters standing there go
adrift, and the minor characters a Destroy System order hit die.

## Superlaser Fire Preconditions

No recovered source for the superlaser fire path. FUN_005617b0 is the
CharacterMgr SeatOfPower check; FUN_0055f650 stores that flag. The
preconditions below are gameplay requirements, not decompiled logic:
1. Target not already destroyed (`!sys.is_destroyed`)
2. Empire Death Star fleet present at target system
3. Target not Empire-controlled (no self-destruction)

## Nearby Warning

Scans all Alliance-controlled systems within `NEARBY_WARNING_RADIUS = 300` coordinate units of the Death Star fleet. Emits `NearbyWarning` for each. Used for Alliance intelligence messages.

## Source

- `ghidra/notes/economy-systems.md` § SystemDeathStarNearbyNotif
- `ghidra/notes/community-address-remap.md` — FUN_005617b0 is SeatOfPower, not the fire path
- `entity-system.md §4.2` — alive_flag inverted semantics
