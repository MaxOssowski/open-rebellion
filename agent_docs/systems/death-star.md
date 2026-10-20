---
title: "Death Star System"
description: "The Death Star as an ordinary capital ship: Destroy System, a destroyed system's cleanup, DeathStarNearby, and the read-only panel"
category: "agent-docs"
created: 2026-03-14
updated: 2026-10-20
tags: [death-star, planet-destruction, simulation]
---

# Death Star System

`death_star.rs` — the Death Star's state and a destroyed system's cleanup.

The Death Star is CAPSHPSD record 136 (TEXTSTRA 10120, research order 0), an
ordinary capital ship built by a shipyard; a completed one marks its fleet
(`Fleet::has_death_star`). The original has no separate construction timer,
so the port's 1,825-day countdown was removed (save v94).

## Types

| Type | Purpose |
|------|---------|
| `DeathStarEvent` | PlanetDestroyed, from a Destroy System order |

The strategic simulation keeps no Death Star state (save v96).

## The Death Star shield

A Death Star shield (DEFFACSD 4, family `0x25`) on the battle system sets
the tactical Death Star's `+0x70` (`FUN_0040bcd0`), and `FUN_005ba450` then
bars the fighters' Attack Death Star (`TacticalDeathStar::attackable`). No
strategic battle code reads it; the port's old hull-damage immunity had no
source and was removed.

## Destroying a planet

Only the Fleet menu's Planetary Bombardment, Destroy System (`0x223`,
`FUN_0058d990`, `bombardment.rs`), destroys a planet; the fleet must hold a
Death Star (`FUN_00582440`). The port's strategic `DeathStarSystem::fire()`
and its panel button had no source and no reachable caller (nothing set
`death_star_fleet`); they were removed on 2026-10-20 (save v95). The
tactical superlaser is the tactical session's
(`ghidra/notes/tactical-death-star-path.md`).

## A destroyed system

`cleanup_destroyed_system` follows the original's cascade
(`ghidra/notes/bombardment-order.md`, "The destroyed system's contents"):
units and facilities go, fleets stay, the characters standing there go
adrift, and the minor characters a Destroy System order hit die.

## Death Star nearby

The system flag DeathStarNearby (`+0x88` bit 11) is set for every system of
a sector holding an active Death Star (`FUN_005178b0`;
`economy::death_star_nearby`). The port's 300-unit `NearbyWarning` message
had no source and was removed.

## The panel (D)

port: a read-only list of the side's fleets holding a Death Star
(`panels::death_star`); the original has no such panel.

## Source

- `ghidra/notes/bombardment-order.md`
- `ghidra/notes/economy-cadence.md`, "Bit 11: DeathStarNearby"
- `ghidra/notes/tactical-death-star-path.md`
