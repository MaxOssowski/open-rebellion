# Simulation coverage map

Built 2026-09-27 for the invention audit, from
`ghidra/notes/timer-scheduler.md`, the recovered notes it cites, and a grep of
`crates/`. Each row is one scheduled or state-driven simulation handler in
REBEXE.EXE, paired with the port code that stands for it.

Status:

- **matched:** the port follows the recovered rule and cites it.
- **partial:** the port has the behavior, but its trigger, timing or formula
  differs from the recovery, or has no source.
- **missing:** the port has no counterpart.
- **port-invented:** the port has a behavior under this id or function that
  the original does not have.

A status is only as good as the recovery behind it. Rows whose original
action is still unread say so.

## Original handlers

### Timers (`FUN_0053fb00`, ids `0x380..0x396`)

In the original, a timer fires after `min + rand(0..spread)` days, repeats
with a fresh delay, and dies when its target's state record changes
(`FUN_005862a0`).

| Id | Original (fire -> action) | Behavior | Port counterpart | Status |
|----|---------------------------|----------|------------------|--------|
| `0x380` | `FUN_0057c2c0` -> ship slot `+0x228` | ship/fighter timer, action unread | none found | missing (semantics open) |
| `0x381` | `FUN_005783b0` -> `FUN_00530460` | each side re-sums three per-system quantities over its systems every 30 + rand(30) days (GNPRTB 7169/7170) | no side tally in the tick; the nearest code is setup-time `seeds.rs:1678` `compute_faction_maintenance` | missing (which quantities `FUN_00528040` sums is open) |
| `0x382` | `FUN_00578060` -> `FUN_00530350` | a side under its `+0x70/+0x74` limits locks (bit 13) one random own completed object | none | missing (semantics open) |
| `0x383` | `FUN_00565400` -> `FUN_0050ca80` | clear system `+0x88` bit 2 | none | missing (bit meaning open) |
| `0x384` | `FUN_00565730` -> `FUN_0050caa0` | support shift GNPRTB 7697 (-1) while `+0x88` bit 2 | `economy.rs` `calculate_support_drift` (`FUN_00559c40`), run every tick; GNPRTB 7697 appears only in test fixtures (`economy.rs:1373`) | partial: timer-driven shifts replaced by a per-tick drift |
| `0x385` | `FUN_00565a60` -> `FUN_0050cad0` | support shift GNPRTB 7691 (0) under bit 8 | same as above; 7691 only in test fixtures | partial |
| `0x386` | `FUN_00565d90` -> `FUN_0050cb20` | support shift GNPRTB 7693 (+1) or 7695 (-1) under bit 5 | same as above; 7693 and 7695 only in test fixtures; `events.rs:101` also names `0x386` `EVT_SIDE_CHANGE` | partial, and an id collision (see Misreads) |
| `0x387` | `FUN_0057bf80` -> `FUN_004f7fc0` | object arrival after Euclidean travel time (`build-delivery.md`) | `movement.rs` transit for fleets only, with its own formula; built objects arrive instantly (`manufacturing.rs`, F-030); `events.rs:102` names `0x387` `EVT_JABBA_CAPTURES_CHEWIE` | partial, and an id collision |
| `0x388` | `FUN_0057cca0` -> `FUN_004ef100` | character injury `+0x94` (`uprising-incident.md:141`) recovers by GNPRTB 2563 (2) or 2564 (1) per firing | none; the port has no injury field (`uprising-incident.md:192`) | missing |
| `0x389` | `FUN_0057cff0` -> `FUN_00560a30` | character timer, action unread | none known | missing (semantics open) |
| `0x38a` | `FUN_0057d330` -> `FUN_004ef150` | character `+0xac` bit 10 pulse while bit 8 | none known | missing (semantics open) |
| `0x38b` | `FUN_0054d6c0` -> `FUN_00522a10` | mission in state 8 sets `+0xa4` bit 0 | `missions.rs` resolves by `ticks_remaining`, with no mission state machine | missing (semantics open) |
| `0x38c` | `FUN_0054da10` -> `FUN_00522a30` | mission in state 11 calls slot `+0xac(0x14)` | same | missing (semantics open) |
| `0x38d` | `FUN_005660c0` -> `FUN_0050cb80` | uprising incident every 30 + rand(70) days | `uprising.rs` (event `0x38d`, `FUN_00510f20`) | matched (commit e101e6c) |
| `0x38e` | `FUN_00566410` -> `FUN_0050cbe0` | informant incident per held system | `economy.rs:1018` `evaluate_incident_flags` sets `informant` from `troop_surplus < 0`, with no source | port-invented trigger (F-029) |
| `0x38f` | `FUN_00566760` -> `FUN_00556b50` | disaster on a random existing system every 1 + rand(399) days | `uprising.rs` `resolve_disaster`, `next_disaster_tick` | matched |
| `0x390` | `FUN_00566a90` -> `FUN_00556be0` | resource incident every 1 + rand(499) days (GNPRTB 7719/7720) | `economy.rs:1018` sets `resource` from overcapped energy or raw materials, with no source; 7719/7720 appear nowhere in `crates/` | port-invented trigger (F-029) |
| `0x392` | `FUN_0054e620` -> `FUN_00543af0(1)` | galaxy manager `+0x58` bit 0 every 300 + rand(100) days (GNPRTB 6146/6147) | none; 6146..6149 appear nowhere in `crates/` | missing (semantics open) |
| `0x393` | `FUN_0054e970` -> `FUN_00546fa0` | galaxy manager timer, 300 + rand(300) days (inference), action unread | none | missing (semantics open) |
| `0x394` | `FUN_00578700` -> `FUN_0053aa20` | type `0x28..0x30` object in state 2 sets `+0x60` bit 2 | none known | missing (semantics open) |

### Setup, events and notifications

| Id / slot | Original | Behavior | Port counterpart | Status |
|-----------|----------|----------|------------------|--------|
| setup stage `0x17` | `FUN_005136d0` -> `FUN_0051b7f0` -> slot `+0xc0` (`FUN_00508250` for systems) | one pass over every object during game setup | `economy.rs` header: "Implements the 18 sub-functions from `FUN_00508250` ... Runs every tick" | port-invented cadence: the original runs this pass at setup, and support recalculation `FUN_0050b230` runs from field hooks (`FUN_00510820`, `FUN_005109f0`, `FUN_00511740`) |
| event `0x303` | `FUN_00510f20`, and `FUN_004fc080` destroyed-on-arrival (`build-delivery.md`) | destroyed-on-arrival notice | none | missing |
| event `0x304` | raised by `FUN_00511780` (sender table) | action unread | `economy.rs:603` fires `EVT_MAINTENANCE_SHORTFALL_EVENT` on a per-faction cooldown reset to GNPRTB 7694 (30), from "the plan's #K4" | port-invented cadence; 7694 sits inside the 7691..7697 group whose odd ids the support timers `0x384..0x386` read, so the even ids may be those timers' delays (inference) |
| event `0x306` | `FUN_004fba10` | route cancel | `events.rs:98` names `0x306` `EVT_CHARACTER_KILLED` | id collision |
| event `0x321` | deployment key (`build-delivery.md`) | build destination change | none | missing (F-030) |
| events `0x300..0x31d` (others) | `FUN_0053f9c0` senders in `timer-scheduler.md` | fire slots not decompiled | not mapped | open |
| notification `0x1f0` | `FUN_00525040` -> UI `FUN_0043c9f0`, formatter `FUN_004927c0` | mission message text | `ai.rs:194` `should_evaluate` with `ai.tick_interval` (7); `ai-parity-tracker.md:92` says this throttles a "0x1f0 daily AI tick" | port-invented rationale: no original AI day tick is recovered, and the AI trigger is unrecovered |
| message 9 | `FUN_0041e270` -> `FUN_0051ddf0` -> `FUN_00516ef0`, `FUN_00517270` | per-location fleet passes | `simulation.rs` combat after movement arrivals | open (sender and meaning unread) |

## Port tick entry points

Each entry point below is from `crates/rebellion-data/src/simulation.rs`, in
tick order. "Original driver" is the handler above that should schedule it.

| Port entry (`simulation.rs` line) | Original driver | Status |
|-----------------------------------|-----------------|--------|
| `EconomySystem::advance_with_uprisings` (126) | setup pass `FUN_00508250` plus field hooks, not a clock | suspect: per-tick cadence has no source |
| `ManufacturingSystem::advance_tracked` (138) | manager tick `FUN_0052b960` (vtable slot `+0x208`, `build-delivery.md`); caller not traced | suspect: zero citations in `manufacturing.rs`; instant delivery (F-030) |
| `MovementSystem::advance` (150) | `FUN_00556430` travel ticks plus timer `0x387` | suspect: `DISTANCE_SCALE = 2`, `MIN_TRANSIT_TICKS = 10` and `DEFAULT_FIGHTER_HYPERDRIVE = 60` (`movement.rs:59-65`) have no source; the original is `max(1, isqrt(d²) / GNPRTB 5120 × speed / 100)` |
| `BombardmentSystem::resolve_bombardment` (217, 299) | none; the cited chain `FUN_00556430 -> FUN_00555d30 -> FUN_00555b30 -> FUN_0055d8c0` is travel time | port-invented formula (see Misreads) |
| `CombatSystem::resolve_ground` (355) | not mapped here | open |
| `FogSystem::advance` (429) | not mapped; `SENSOR_MULTIPLIER = 15.0` (`fog.rs:140`) has no source | suspect |
| `MissionSystem::advance` (436) | mission state machine, timers `0x38b`/`0x38c`, decoy phases (`decoy-roll.md`) | partial |
| `EventSystem::advance` (556) | events `0x300..0x37f` | partial: ids collide with timer ids |
| `AISystem::advance` (566, 594) | unrecovered | suspect: interval rationale rests on the `0x1f0` misread |
| `BlockadeSystem::advance` (622) | `blockade-troop-withdrawal.md` | cited; not re-audited here |
| `UprisingSystem::advance` (646) | timers `0x38d`, `0x38f` | matched |
| `BetrayalSystem::advance` (659) | none found | suspect: `BETRAYAL_CHECK_INTERVAL = 50` (`betrayal.rs:22`), zero citations |
| `DeathStarSystem::advance` (669) | superlaser path unrecovered (`death_star.rs:17`) | suspect: `DEATH_STAR_CONSTRUCTION_TICKS = 1825` has no source |
| `ResearchSystem::advance` (717) | none found; the design is from `rebellion2/Faction.cs` (`research.rs:12`) | suspect: `RESEARCH_MIN_TICKS = 10` and `RESEARCH_DEFAULT_TICKS = 30` have no source |
| `RepairSystem::advance` (721) | none found; the header cites "community disassembly" notifications only | suspect |
| `JediSystem::advance` (726) | none found | suspect: `DETECTION_CHECK_INTERVAL = 30` and `DETECT_PROB_*` (`jedi.rs:57-66`) have no source |
| `VictorySystem::check` (731) | not mapped | open |

## Misreads found

1. **Bombardment is travel time.** `bombardment.rs` and
   `ghidra/notes/bombardment.md` read `FUN_00556430 -> FUN_00555d30 ->
   FUN_00555b30 -> FUN_0055d8c0` (with `FUN_00509620`) as a bombardment damage
   formula. `build-delivery.md` shows it is the travel-tick computation:
   - the "power" operands are system coordinates;
   - GNPRTB `0x1400` (5120) is the distance divisor;
   - the "damage" is ticks.
   
   `combat.rs` also names `0x1400` a combat difficulty modifier.
2. **`0x1f0` is not a daily AI tick.** `agent_docs/ghidra-re.md:155` and
   `agent_docs/systems/ai-parity-tracker.md:52,92,123` call `FUN_004927c0`
   the master turn processor and say event `0x1f0` runs the AI daily. In fact
   `FUN_004927c0` formats message-window text for notification `0x1f0`, which
   `FUN_00525040` sends from mission code and only a UI factory receives
   (`timer-scheduler.md`). A `tuning.rs` citation of `0x1f0` was not found in
   the current tree.
3. **Uprising leadership averages over the whole mission.** `FUN_00520cd0`
   averages leadership over every team, decoy and captured member
   (`decoy-roll.md`). The port's uprising incident reads only the mission's
   single agent.
4. **Event ids collide with timer ids.** `events.rs:101-102` assigns `0x386`
   and `0x387` to story events (side change, Jabba captures Chewie), and
   `events.rs:338` cites `0x396`. In REBEXE.EXE `0x386` and `0x387` are timer
   ids, and `0x396` has no factory. Either the story events use another id
   space, which is unverified, or these constants are misattributed.
5. **`FUN_00508250` is not a per-tick loop.** `economy.rs` says it "runs
   every tick"; the recovered caller is setup stage `0x17`.
6. **The type family table conflicts.** `ghidra/notes/entity-system.md:383`
   gives `0x30–0x40` as ships. Two iterator recoveries (`decoy-roll.md`,
   `object-state-flags.md`) give `0x30..0x3c` as characters and `0x14..0x1c`
   as capital ships.

## Counts

Original handlers (28 rows):

| Status | Count |
|--------|-------|
| matched | 2 |
| partial | 4 |
| missing | 14 (11 with part of the original action still unread) |
| port-invented | 5 |
| id collision | 1 |
| open | 2 |

Port entry points (17 rows):

| Status | Count |
|--------|-------|
| matched | 1 |
| partial | 2 |
| suspect | 10 |
| port-invented | 1 |
| cited, not re-audited | 1 |
| open | 2 |
