---
title: "Save/Load System"
description: "Native and browser save v63, canonical fingerprints, campaign setup, continuation state, troop cargo, embarked tracking, deliveries, and the no-migration rule"
category: "agent-docs"
created: 2026-03-15
updated: 2026-10-17
tags: [save-load, bincode, migration, serialization, wasm, determinism]
---

# Save/Load System

`crates/rebellion-data/src/save.rs` owns native files and browser storage.
`crates/rebellion-app/src/main.rs` converts between a live campaign and the
serializable snapshot. The current format is v63, and it is the only one
that loads.

## Native format (v63)

```text
[magic: 8 bytes "OPENREB\0"]
[version: u32 LE]             — SAVE_VERSION = 37
[save_name: u32 len + UTF-8]
[timestamp_secs: u64 LE]
[mod_count: u32 LE]
  for each mod:
    [name_len: u32 + name: UTF-8]
    [version_len: u32 + version: UTF-8]
[mod_hash: u64 LE]            — FNV-1a over sorted name/version pairs
[fingerprint_version: u16 LE]
[state_fingerprint: u64 LE]
[bincode body: SaveState]
```

The fingerprint is versioned separately from the save format. It is a
domain-separated FNV-1a digest of canonical JSON and is intended for replay
comparison and corruption detection, not authentication.

## SaveState

Every mutable campaign subsystem required by the app is serialized:

| Field | Type |
|-------|------|
| `world` | `GameWorld`; each system keeps the squadrons on its planet (v46); each capital ship its id and the world the last id given (v51) |
| `clock` | `GameClock` |
| `manufacturing` | `ManufacturingState`: one queue and one Destination per system and production area (v30); a queue keeps its build points and each unit its cost in points (v34) |
| `missions` | `MissionState`: missions, travelling members, and each captive's escape timer and escaped characters on the road (v35) |
| `events` | `EventState` |
| `ai` | `AIState`: busy characters (the battle cooldowns of the port's fleet heuristics went in v58), the strategy's research planners with their held designer, site, target, mission, unread report and failures (v37), the day's order count and the round robin's module (v38; the evaluation tick is gone), the priority scan's cursor, each module's priority and undecided orders, and the mission module's next entry (v39), the yard marks the modules keep on system records (v40), the kept record `+0x30` bits, the controller's yard token and module 1's fields and sites (v41), module 2's fields and sites (v42), and module 3's fields and sites and the controller's shield goal `+0x374` (v43), module 5's fields and sites (v44), module 6's fields and sites (v45), module 7's fields and sites and the controller's squadron goals `+0x388..+0x390` (v47), module `0xb`'s fields and sites (v48), and the strategy's posture, kind, growth flags and sector plans, the controller's garrison goals `+0x37c..+0x384`, and the day each system was last seen (record `+0x44`) (v49), module 8's fields and sector planners (v50), the AI's ship record holds by ship id (v51), its fleet record holds and each sector planner's ships, fleets, counts, requests and plan wants (v52), each plan object's fleet and search cursor (v53), each fleet record hold's plan target `+0x2c` (v54), each plan object's walk cursor `+0x30` (v55), kind 2's visited systems `+0x5c` and the swept sectors (sector record `+0x2c` `0x20000000`) (v56), and kind 4's target system `+0x5c` (v57); v58 drops module 8's `decided` flag with the stand-in fleet heuristics it served |
| `movement` | `MovementState` |
| `fog_alliance`, `fog_empire` | `FogState` |
| `player_is_alliance` | `bool` |
| `blockade` | `BlockadeState`; v62 adds the support timer `0x386` of each blockaded system |
| `uprising` | `UprisingState`; v62 adds each revolt's support timer `0x384`, v63 its side |
| `death_star` | `DeathStarState` |
| `research` | `ResearchState`: each side's level, points and Done flag per tree; research runs only as missions, so the invented projects are gone (v36) |
| `jedi` | `JediState` |
| `victory` | `VictoryState` |
| `betrayal` | `BetrayalState` |
| `economy` | `EconomyState`; v61 adds each system's smuggling percent (`+0x6c`), v63 the hook inputs last seen |
| `sim_rng` | `Xoshiro256PlusPlus` |
| `ai2` | `Option<AIState>` |
| `repair` | `RepairState` |
| `production` | `ProductionState`: each side's raw and refined stockpiles, the refineries waiting for raw material, and every mine's and refinery's cycle and upkeep allocation (v33); the yards waiting for refined material and every yard's cycle (v34) |
| `combat_cooldowns` | `HashMap<SystemKey, u64>` |
| `game_config` | `GameConfig` (v59 drops `AiConfig`'s `alliance_deploy_budget`, `empire_deploy_budget` and `ds_retreat_strength_ratio`; v60 its `max_attack_fronts`, `battle_cooldown_ticks`, `proximity_divisor` and the four `weight_*` attack scoring weights) |
| `campaign_config` | `CampaignConfig` |
| `troop_transport` | `TroopTransportState`; it keeps the squadrons travelling on their own (v47); v59 drops the fleets' hold |
| `deliveries` | `DeliveryState` |
| `player_agent` | `PlayerAgent` |

Loading restores the RNG, dual-AI state, repair episodes, and combat memory
instead of reseeding or clearing them. `campaign_config` keeps the selected
difficulty, galaxy-size label, player faction, and victory mode. The clock
keeps its original Game Speed and partial day, so a game saved while paused
reloads paused. The save also holds blockade embarked-regiment tracking,
the fleets holding regiments loaded through the Fleet window (v24),
regiments travelling on their own (v25), the fleet a moving fleet joins on
arrival (v26), the fleet naming mode (v28), and the player's Manage
Garrisons and Manage Production automation, the Agent Advice setting, renamed
fleets and ships, and each production area's destination (v29),
`UprisingState`, queue destinations and en-route deliveries, missions with
their member lists, phase, and timer, members travelling to a mission
target, and the MISSNSD and SPECFCSD records.

## Deterministic fingerprints

Unordered sets and maps use stable human-readable serialization for the
fingerprint. Typed-key maps become sorted key/value sequences because JSON
object keys must be strings. The adapters intentionally leave non-human
serialization unchanged, preserving the historical bincode layout.

Sequence order that carries gameplay meaning remains ordered. A matching
fingerprint proves only that one recorded logical snapshot matches. F-011B4
adds exact-artifact native/WASM checkpoint equivalence for the seed-42 fixture;
interactive app/playtest and combat-path convergence remain open.

## No migration

The port has no released saves, so the loader reads only `SAVE_VERSION`. Any
other version is rejected with a message to start a new game.

Changing `SaveState`, or any type it holds, including `GameWorld`, changes the
bincode layout. Bincode is positional, so `#[serde(default)]` does not keep old
files readable. Bump `SAVE_VERSION` for every layout change. Once saves are
released, restore versioned migration before the next layout change.

## Browser storage

WASM stores base64 bincode and versioned JSON metadata in `localStorage`:

```text
rebellion_save_v26_<slot>
rebellion_meta_v26_<slot>
```

Metadata includes the full save name, game tick, and fingerprint with its
`u64` value encoded as a decimal string so JavaScript cannot truncate it. The
key prefix follows `SAVE_VERSION`, so entries from older builds are ignored.
Their stale entries remain in `localStorage` until the browser clears them.

This path is functional but not the production persistence target: base64 and
synchronous `localStorage` can block the main thread or hit quota limits. M3
moves it to versioned, compressed, asynchronous IndexedDB and adds browser
quota/corruption acceptance tests.

## API

```rust
let fingerprint = save_slot(saves_dir, slot, name, &state, &active_mods)?;
let fingerprint = save_slot_no_mods(saves_dir, slot, name, &state)?;
let (meta, state) = load_slot(saves_dir, slot)?;
let occupied = list_saves(saves_dir);
delete_slot(saves_dir, slot)?;
```

Native saves live at `<saves_dir>/<slot>.reb`. The UI exposes ten slots.

## Safe change checklist

1. Add the field to `SaveState` and the live snapshot/restore path.
2. Bump `SAVE_VERSION`. The browser key prefixes follow it.
3. Add round-trip, corruption, and exact-continuation tests.
4. Regenerate the seed-42 replay golden, since the version is hashed into
   every fingerprint, and name the cause in the commit.
5. Run workspace tests, the seeded fingerprint probe, WASM/package checks, and
   an independent browser save/reload/load/continue pass with bitmap and error gates.

Current verification evidence:
[F-011A fingerprints](../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-08-state-fingerprints.md)
[F-011B1 continuation](../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-09-state-continuation.md),
[F-011B2 replay contract](../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-09-replay-contract.md),
[F-011B3 replay execution](../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-09-replay-execution.md),
[F-011B4 native/WASM replay equivalence](../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-09-replay-wasm-equivalence.md),
[F-016B campaign setup](../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-09-game-setup-propagation.md),
and [F-007E troop transport and occupation](../docs/qa/2026-09-08-full-functionality-audit/evidence/2026-09-10-troop-transport-occupation.md).
