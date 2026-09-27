---
title: "Uprising System"
description: "Recovered revolt lifecycle, uprising incident, Subdue support gain, and natural disaster"
category: "agent-docs"
created: 2026-03-14
updated: 2026-09-26
tags: [uprising, incite, subdue, disaster, simulation]
---

# Uprising System

`uprising.rs` ports the recovered revolt lifecycle, the uprising incident, and
the natural disaster (F-026). Ghidra evidence: `ghidra/notes/uprising-incident.md`.

## Types

| Type | Purpose |
|------|---------|
| `UprisingState` | Active revolts (`ActiveUprising`: start tick, next incident tick) and the galaxy disaster timer |
| `UprisingStateV14` | The v14 save shape, kept for migration |
| `UprisingEvent` | `UprisingBegan`, `UprisingEnded`, `UprisingIncident` (codes, losses, support change), `Disaster` |
| `IncidentLoss` | A destroyed facility or regiment, an injured character, a freed prisoner |

## API

```rust
// Each tick, after the economy summary:
let events = UprisingSystem::advance(
    &mut state, &world, &economy, &missions, &tick_events, &rolls);
apply_uprising_event(&mut world, &event); // integrator and app

// A Subdue Uprising success:
let gain = subdue_support_gain(&world, system, side, roll);
apply_support_change(&mut world, system, side, gain);
let ended = end_if_garrisoned(&mut state, &world, system, tick);
```

## Lifecycle

1. A revolt starts when a held, populated system has fewer regiments than its
   garrison requirement (`FUN_0050b800`). Control never changes hands.
2. It continues while the system stays held and populated, even after the
   garrison is restored, and ends when the system is lost or emptied.
3. A Subdue Uprising success raises support (`FUN_0055cb10`: 1..20 on its own
   side's system, 1..10 when contested) and ends the revolt once all regiments
   cover the requirement without the uprising doubling (`FUN_0050c910`).
4. While a revolt lasts, an incident fires every 30 to 100 ticks. It scores two
   `1..10` draws, the support shortfall below 60, the holder's regiments, the
   Empire's Stormtroopers, and the Incite and Subdue agents' leadership, then
   applies the UPRIS1TB and UPRIS2TB step-lookup codes: a lost facility or
   regiment, an injured character, or freed prisoners. An active Incite
   mission then costs the holder 2 support points (halved when support is
   strong).

## Disaster

A galaxy timer fires every 1 to 400 ticks and picks a random system with
energy or raw materials. It erodes both and destroys each facility not en
route, of either side, at 10 percent.

## Garrison

`economy::garrison_requirement` (`FUN_00559fe0`) halves the requirement only
for a strongly supported Empire system and doubles it only during a revolt.

## Open

- Injury has no character field in the port; it is reported only.
- The informant and resource incidents still fire on invented triggers (F-029).
