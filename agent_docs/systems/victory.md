---
title: "Victory System"
description: "Game-ending condition detection for Alliance and Empire"
category: "agent-docs"
created: 2026-03-14
updated: 2026-10-18
tags: [victory, hq-capture, death-star, simulation]
---

# Victory System

`victory.rs` — Game-ending condition detection.

## Types

| Type | Purpose |
|------|---------|
| `VictoryState` | `{ alliance_hq, empire_hq, death_star_active, death_star_location, resolved, conditions_word }` |
| `VictoryOutcome` | HqCaptured / HqDestroyed / DeathStarVictory |

## API

```rust
// Each tick: check all victory conditions
if let Some(outcome) = VictorySystem::check(
    &victory_state,
    &world,
    &tick_events,
    campaign_config.victory_conditions,
) {
    victory_state.resolved = true;
    // show victory/defeat screen
}
```

## Implemented Contract

Since 2026-10-18 (F-005) the game is decided by the victory conditions
word (`conditions_word`, `FUN_005180e0`; `ghidra/notes/victory-conditions.md`).
`VictorySystem::check` reads the word on a day with events: with its
game-over bit `0x10000000` set, the Alliance win bit `0x1000000` gives
`HqCaptured`, else the Empire wins by `DeathStarVictory` when the Alliance
HQ system is destroyed, or `HqDestroyed`.

The word is missing (and no game ends) while Luke, Mon Mothma, Vader, the
Emperor or Coruscant is absent.

## Current Victory Conditions

The active `VictoryConditions` (galaxy `+0x60`) selects one of two rule
sets.

### Standard

- **Alliance**: Vader and the Emperor captured, and Coruscant held by the
  Alliance or destroyed.
- **Empire**: Luke and Mon Mothma captured, and the Alliance headquarters
  lost. The HQ is lost when its system is destroyed (Death Star) or no
  longer holds the HQ: an Imperial bombardment that destroys the
  headquarters object (ALLFACSD 1, bombardment defense 9) clears it
  (`ghidra/notes/bombardment-order.md`).
- A captive is any captured character (`+0xac` bit 0), whoever holds him.

### Headquarters Only

- **Alliance**: Coruscant held or destroyed.
- **Empire**: the Alliance headquarters lost.

When both sides would win on one day, the win already in the stored word
stands; with none, the Alliance's.

### Against the manual and the earlier contract

The manual ("On to Victory!") says the Empire must destroy the HQ by
bombardment *and take control of the system with an assault*, or destroy
the system with a Death Star, and that the Alliance must hold Coruscant.
`FUN_005180e0` tests only the HQ object's destroyed flag (`+0x50` bit 3)
and Coruscant's side or destruction, so the port follows the binary:

- no Imperial assault is needed after the bombardment;
- a destroyed Coruscant counts for the Alliance;
- the leaders may be held by any captor.

The 2026-09-10 contract in the
[official campaign reference](../../docs/reference/campaign-history/official-campaign-contract.md#standard-victory)
follows the manual and is superseded on these points.

## Initialization

`VictoryState::new(alliance_hq, empire_hq)` — both SystemKeys found at startup by scanning for `sys.is_headquarters && controlling_faction == Alliance/Empire`.

## Lifecycle

1. `VictorySystem::check()` returns `None` every tick until a terminal condition is met
2. First `Some(VictoryOutcome)` → caller sets `resolved = true`
3. Subsequent calls return `None` (resolved flag prevents re-firing)

## Source

- `entity-system.md §4.2` — `SideVictoryConditionsNotif`, `FinalBattle`
- Event IDs `0x12c`/`0x180`
- `FUN_005180e0` (the word), `FUN_00518460` (notification `0x128`)
- `System::is_headquarters` flag
- [`main-menu-parity.md`](../main-menu-parity.md) — original game-type selector and setup contract
- [F-016B evidence](../../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-09-game-setup-propagation.md)
- [Campaign-history source ledger](../../docs/reference/campaign-history/sources.json)
- [Victory-contract verification](../../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-10-victory-contract.md)
