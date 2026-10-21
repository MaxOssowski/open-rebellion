//! Blockade mechanics: manufacturing halt and troop regiment losses.
//!
//! A blockade occurs when a hostile fleet is present at a system with no
//! defending fleet (`FUN_0050b8e0`, `ghidra/notes/blockade-bit.md`). This
//! module:
//!
//! 1. **Determines blockaded systems** each tick from fleet disposition.
//! 2. **Emits `BlockadeEvent`s** when blockade state changes (enter/exit).
//! 3. **Halts manufacturing** — `BlockadeState::is_blockaded()` is consulted
//!    by `production_cycle::advance_yards` to suspend the yards of blocked systems.
//! 4. **Destroys regiments running the blockade** — a regiment carried into a
//!    blockaded system takes that system's withdraw percent, and rolls against
//!    it when its fleet leaves. Surface garrisons and landed regiments are
//!    never rolled.
//!
//! # Architecture
//!
//! Follows the stateless advance pattern:
//! ```text
//! BlockadeSystem::advance(&mut BlockadeState, &world, &[TickEvent])
//!     -> Vec<BlockadeEvent>
//! BlockadeSystem::running_regiments(&mut BlockadeState, &world, &movement, &transport)
//!     -> Vec<RunningRegiment>
//! BlockadeSystem::resolve_running(&[RunningRegiment], &rolls, tick)
//!     -> Vec<BlockadeEvent>
//! ```
//! The caller applies `BlockadeEvent::TroopDestroyed` to `GameWorld` and to
//! `TroopTransportState` (the regiment is removed from its fleet's cargo and
//! from the arena).
//!
//! # Source
//!
//! Ghidra RE: `economy-systems.md §4`.
//! - `FUN_0050e820` (production eligibility): step 8 — fleet active check gates
//!   manufacturing when a hostile blockader is present.
//! - `FUN_005122d0` (`SystemBlockadeNotif`), event `0x14e` (334).
//! - `FUN_00512540` (`BattlePendingCausedCurrentBlockade`): hostile fleet arrival
//!   entails both `BattlePending` and `Blockade` simultaneously.
//!
//! Running the blockade (recovered 2026-09-26, notes in `ghidra/notes`):
//! - `FUN_0050b310` sets the system's withdraw percent (`+0x74`): 100, unless
//!   the system is blockaded (`+0x88` bit 5) and has no active KDY-150
//!   (family `0x22`, iterator `FUN_00527550`). Then it is
//!   `FUN_0055a020(ships, fighters)` =
//!   `max(0, 100 - ships * GNPRTB[7684] - fighters * GNPRTB[7685])`, counting
//!   every active fleet's capital ships and fighters at the system.
//! - A regiment added to a system copies that value (`FUN_00514a60` calls the
//!   system's `+0xd0` slot `FUN_00508660`, which calls `FUN_0050c540` then
//!   `FUN_00504470`). Later changes to the system value are not pushed to
//!   regiments already there: the system's `+0x1ec` slot is `FUN_00524b10`,
//!   which only returns 1.
//! - Activating a regiment resets it to 100 (`FUN_004f7410` calls slot `+0x110`
//!   `FUN_00504960`).
//! - When the regiment enters transit (`FUN_004f7640` sets `+0x50` bit 5), slot
//!   `+0x124` `FUN_00504990` keeps it only if `random(0..=99) < percent`
//!   (`FUN_0053e2f0`). Otherwise `FUN_00504400` marks it destroyed, which
//!   raises `TroopRegDestroyedRunningBlockade` (`FUN_00504a00`, event `0x340`)
//!   and removes it through slot `+0x1f0` (`FUN_005049e0`).
//!
//! Support under blockade (2026-10-17, `ghidra/notes/economy-cadence.md`):
//! - The blockade hook `FUN_00511300` arms timer `0x386` on the system's
//!   record `+0x54`+`0x30`, whose minimum `FUN_0050e200` sets to GNPRTB 7694
//!   (30). Arming with the bit clear cancels it (`FUN_0053fa60`).
//! - Each time it fires while blockaded, `FUN_0050cb20` takes the blockading
//!   fleet's side (`FUN_00509890`: the first fleet at the system) and shifts
//!   that side's support through `FUN_0050c9f0`: by GNPRTB 7693 (1) when it is
//!   the side with more support (`FUN_00509980`, `FUN_00509930`), else by
//!   GNPRTB 7695 (-1).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ids::{DatId, SystemKey, TroopKey};
use crate::movement::MovementState;
use crate::tick::TickEvent;
use crate::troop_transport::TroopTransportState;
use crate::world::ControlKind;
use crate::world::GameWorld;

// ---------------------------------------------------------------------------
// Event IDs (from RE annotated-functions.md)
// ---------------------------------------------------------------------------

/// `SystemBlockadeNotif` event ID — fires when blockade state changes
/// (`FUN_005122d0`: `FUN_0053fdd0(0x14e, ...)`).
pub const EV_BLOCKADE: u32 = 0x14e; // 334

/// `TroopRegDestroyedRunningBlockade` event ID (`FUN_00504a00`:
/// `FUN_0053fe40(0x340, ...)`).
pub const EV_TROOP_BLOCKADE_KILL: u32 = 0x340; // 832

/// DEFFACSD record 1, the KDY-150 (TEXTSTRA 8704, family `0x22`). An active one
/// keeps the system's withdraw percent at 100 (`FUN_0050b310`).
pub const KDY_150_CLASS_ID: u32 = 1;

/// GNPRTB 7684 (`0x1e04`): withdraw penalty per capital ship. `FUN_00558bb0`
/// loads it into `DAT_006bb458`.
pub const GNPRTB_WITHDRAW_PENALTY_PER_CAPSHIP: u16 = 7684;

/// GNPRTB 7685 (`0x1e05`): withdraw penalty per fighter squadron. `FUN_00558bb0`
/// loads it into `DAT_006bb3cc`.
pub const GNPRTB_WITHDRAW_PENALTY_PER_FIGHTER: u16 = 7685;

/// `DAT_00661a88`, the constant 100: the full withdraw percent and the bound of
/// the survival roll.
pub const FULL_WITHDRAW_PERCENT: u8 = 100;

/// GNPRTB 7694 (`DAT_006bb3d8`) = 30: timer `0x386`'s minimum
/// (`FUN_0050e200`, record `+0x54`+`0x30`).
pub const GNPRTB_BLOCKADE_SUPPORT_PERIOD: u16 = 7694;

/// GNPRTB 7693 (`DAT_006bb48c`) = 1: the blockader's support shift when it is
/// the more popular side (`FUN_0050cb20`).
pub const GNPRTB_BLOCKADE_SUPPORT_POPULAR: u16 = 7693;

/// GNPRTB 7695 (`DAT_006bb494`) = -1: the blockader's support shift
/// otherwise (`FUN_0050cb20`).
pub const GNPRTB_BLOCKADE_SUPPORT_UNPOPULAR: u16 = 7695;

// ---------------------------------------------------------------------------
// BlockadeEvent
// ---------------------------------------------------------------------------

/// Events emitted by `BlockadeSystem::advance` each tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockadeEvent {
    /// A system has entered a blockaded state (hostile fleet, no defender).
    ///
    /// Manufacturing is halted for the controlling faction until blockade ends.
    /// Corresponds to event `0x14e` (`SystemBlockadeNotif`) with state = enter.
    BlockadeStarted { system: SystemKey, tick: u64 },

    /// A system's blockade has ended (defending fleet arrived or attacker withdrew).
    ///
    /// Corresponds to event `0x14e` (`SystemBlockadeNotif`) with state = exit.
    BlockadeEnded { system: SystemKey, tick: u64 },

    /// A regiment was destroyed leaving the blockaded `system` it had been
    /// carried into.
    ///
    /// From RE: `FUN_00504990` roll, `FUN_00504a00` notification, event `0x340`.
    /// The caller must remove `troop` from its fleet's cargo and from
    /// `GameWorld::troops`.
    TroopDestroyed {
        system: SystemKey,
        troop: TroopKey,
        tick: u64,
    },

    /// Timer `0x386` fired at a blockaded system (`FUN_0050cb20`): the
    /// blockading `side`'s support shifts by `support_delta` points. The
    /// caller applies it with `uprising::shift_support`.
    SupportShifted {
        system: SystemKey,
        side: crate::dat::Faction,
        support_delta: i32,
        tick: u64,
    },
}

impl BlockadeEvent {
    #[must_use]
    pub fn tick(&self) -> u64 {
        match self {
            BlockadeEvent::BlockadeStarted { tick, .. }
            | BlockadeEvent::BlockadeEnded { tick, .. }
            | BlockadeEvent::TroopDestroyed { tick, .. }
            | BlockadeEvent::SupportShifted { tick, .. } => *tick,
        }
    }
}

// ---------------------------------------------------------------------------
// BlockadeState
// ---------------------------------------------------------------------------

/// Per-system blockade status, maintained across ticks.
///
/// `BlockadeSystem::advance` updates this state and emits change events.
/// `production_cycle::advance_yards` asks `is_blockaded(system)` to skip
/// queue advancement at blocked systems.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockadeState {
    /// Systems currently under blockade (hostile fleet present, no defender).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_set",
        deserialize_with = "crate::serde_ordered::deserialize_hash_set"
    )]
    blockaded: HashSet<SystemKey>,
    /// Last observed position and withdraw percent of each embarked regiment.
    ///
    /// Persisted since save v15. Loading a v14 or older save starts every
    /// embarked regiment at full withdraw percent, which never destroys one
    /// wrongly (FUN_00504960 resets to 100 on activation).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    embarked: HashMap<TroopKey, EmbarkedRegiment>,
    /// The tick timer `0x386` next fires at each blockaded system (v62).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    support_timers: HashMap<SystemKey, u64>,
    /// Each blockaded system's blockading side and its assault and
    /// bombardment losses since the blockade began (`System_vt220`,
    /// `FUN_00511300`, clears the system record's counters `+0x48..+0x58`
    /// and stores the blockader in `+0x60`; save v89).
    #[serde(
        default,
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    losses: HashMap<SystemKey, BlockadeLosses>,
    /// The losses of the blockades that ended in the last
    /// [`BlockadeSystem::advance`], for the caller to settle.
    #[serde(skip)]
    ended: Vec<(SystemKey, BlockadeLosses)>,
}

/// A blockade's blockading side and the losses at its system since it
/// began (`ghidra/notes/betrayal-and-loyalty.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockadeLosses {
    pub blockader: crate::dat::Faction,
    pub losses: crate::loyalty::SideLosses,
}

/// An embarked regiment's last observed orbit and its withdraw percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbarkedRegiment {
    /// The system its fleet orbits, or `None` while in transit.
    pub orbit: Option<SystemKey>,
    /// Chance in percent that it survives leaving `orbit` (regiment `+0x60`).
    pub withdraw_percent: u8,
}

/// A regiment whose fleet has just left a system with a withdraw percent below
/// 100. `BlockadeSystem::resolve_running` decides whether it survives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunningRegiment {
    pub troop: TroopKey,
    pub system: SystemKey,
    pub withdraw_percent: u8,
}

impl BlockadeState {
    #[must_use]
    pub fn new() -> Self {
        BlockadeState {
            blockaded: HashSet::new(),
            embarked: HashMap::new(),
            support_timers: HashMap::new(),
            losses: HashMap::new(),
            ended: Vec::new(),
        }
    }

    /// Add `losses` to `system`'s blockade, while it is blockaded
    /// (`System_vtd0` for DestroyedReason `0xe` and `0xf`).
    pub fn record_losses(&mut self, system: SystemKey, losses: crate::loyalty::SideLosses) {
        if let Some(entry) = self.losses.get_mut(&system) {
            entry.losses.merge(losses);
        }
    }

    /// `system`'s blockade's losses so far.
    #[must_use]
    pub fn blockade_losses(&self, system: SystemKey) -> Option<BlockadeLosses> {
        self.losses.get(&system).copied()
    }

    /// The blockades that ended in the last [`BlockadeSystem::advance`],
    /// with their losses, taken once.
    pub fn take_ended(&mut self) -> Vec<(SystemKey, BlockadeLosses)> {
        std::mem::take(&mut self.ended)
    }

    /// The tracked position and withdraw percent of an embarked regiment.
    #[must_use]
    pub fn embarked_regiment(&self, troop: TroopKey) -> Option<EmbarkedRegiment> {
        self.embarked.get(&troop).copied()
    }

    /// Remove blockade state for a system (used when system is destroyed).
    pub fn clear_blockade(&mut self, system: SystemKey) {
        self.blockaded.remove(&system);
        self.support_timers.remove(&system);
        self.losses.remove(&system);
    }

    /// The tick timer `0x386` next fires at `system`, while it is blockaded.
    #[must_use]
    pub fn support_timer(&self, system: SystemKey) -> Option<u64> {
        self.support_timers.get(&system).copied()
    }

    /// Returns `true` if `system` is currently blockaded.
    ///
    /// Asked by `production_cycle::advance_yards` to halt production.
    #[must_use]
    pub fn is_blockaded(&self, system: SystemKey) -> bool {
        self.blockaded.contains(&system)
    }

    /// All currently blockaded systems.
    #[must_use]
    pub fn blockaded_systems(&self) -> &HashSet<SystemKey> {
        &self.blockaded
    }
}

// ---------------------------------------------------------------------------
// BlockadeSystem
// ---------------------------------------------------------------------------

/// Stateless blockade evaluator.
pub struct BlockadeSystem;

impl BlockadeSystem {
    /// The systems blockaded now, by [`Self::system_is_blockaded`]: what
    /// [`Self::advance`] would store, for a check that runs between
    /// advances.
    #[must_use]
    pub fn blockaded_now(world: &GameWorld) -> HashSet<SystemKey> {
        world
            .systems
            .iter()
            .filter(|(_, sys)| Self::system_is_blockaded(world, sys))
            .map(|(key, _)| key)
            .collect()
    }

    /// Advance blockade state for all systems.
    ///
    /// For each system in `world`:
    /// - If a hostile fleet is present AND no defending fleet: mark blockaded.
    /// - If previously blockaded but conditions no longer hold: clear blockade.
    /// - Emit `BlockadeStarted` / `BlockadeEnded` on transitions.
    /// - Stationed `ground_units` remain intact; regiments running the blockade
    ///   come from `running_regiments` and `resolve_running`.
    ///
    /// Does **not** mutate `GameWorld` — returns events for the caller to apply.
    /// Skips zero-tick frames.
    pub fn advance(
        state: &mut BlockadeState,
        world: &GameWorld,
        tick_events: &[TickEvent],
    ) -> Vec<BlockadeEvent> {
        let Some(last_tick_event) = tick_events.last() else {
            return Vec::new();
        };

        let tick = last_tick_event.tick;
        let mut events = Vec::new();

        // Compute current blockade set from world fleet disposition
        let now_blockaded: HashSet<SystemKey> = world
            .systems
            .iter()
            .filter_map(|(sys_key, sys)| {
                if Self::system_is_blockaded(world, sys) {
                    Some(sys_key)
                } else {
                    None
                }
            })
            .collect();

        // HashSet iteration order is randomized per process. Events mutate
        // world state downstream, so emit transitions by stable system key.
        let mut newly_blockaded: Vec<_> = now_blockaded
            .difference(&state.blockaded)
            .copied()
            .collect();
        newly_blockaded.sort_unstable();

        // port: the record's spread is never set (FUN_0050e200 sets only the
        // minimum), so the delay is the minimum. port: FUN_005109f0 resets
        // the minimum to GNPRTB 7694 or 7696 (both 30) when the support
        // changes; the port keeps 7694.
        let period = u64::try_from(
            world
                .gnprtb
                .value(GNPRTB_BLOCKADE_SUPPORT_PERIOD, world.difficulty_index)
                .max(1),
        )
        .unwrap_or(1);

        // New blockades (entered this tick): FUN_00511300 arms timer 0x386.
        for sys_key in newly_blockaded {
            events.push(BlockadeEvent::BlockadeStarted {
                system: sys_key,
                tick,
            });
            state.support_timers.insert(sys_key, tick + period);
            // FUN_00511300: clear the losses, store the blockader
            // (FUN_00509890: the side of the fleets there).
            if let Some(blockader) = Self::blockader(world, sys_key) {
                state.losses.insert(
                    sys_key,
                    BlockadeLosses {
                        blockader,
                        losses: crate::loyalty::SideLosses::default(),
                    },
                );
            }
        }

        // Cleared blockades (ended this tick)
        let mut cleared: Vec<_> = state
            .blockaded
            .difference(&now_blockaded)
            .copied()
            .collect();
        cleared.sort_unstable();
        state.ended.clear();
        for sys_key in cleared {
            events.push(BlockadeEvent::BlockadeEnded {
                system: sys_key,
                tick,
            });
            state.support_timers.remove(&sys_key);
            if let Some(entry) = state.losses.remove(&sys_key) {
                state.ended.push((sys_key, entry));
            }
        }

        state.blockaded = now_blockaded;

        // Timer 0x386 (FUN_0050cb20), in system order.
        for (sys_key, sys) in &world.systems {
            let Some(due) = state.support_timers.get_mut(&sys_key) else {
                continue;
            };
            if *due > tick {
                continue;
            }
            *due += period;
            if let Some(event) = Self::support_shift(world, sys_key, sys, tick) {
                events.push(event);
            }
        }
        events
    }

    /// `FUN_00509890`: the blockading side, the side of the fleets at
    /// `system` (a blockade has one side's).
    fn blockader(world: &GameWorld, system: SystemKey) -> Option<crate::dat::Faction> {
        world
            .systems
            .get(system)?
            .fleets
            .iter()
            .find_map(|key| world.fleets.get(*key))
            .map(|fleet| crate::dat::Faction::of_alliance(fleet.is_alliance))
    }

    /// `FUN_0050cb20`: the blockading side's support shift at `sys`, or
    /// `None` when the system has no fleet or is unpopulated
    /// (`FUN_0050c9f0` needs `+0x88` bit 0).
    fn support_shift(
        world: &GameWorld,
        system: SystemKey,
        sys: &crate::world::System,
        tick: u64,
    ) -> Option<BlockadeEvent> {
        use crate::dat::Faction;
        use crate::uprising::{support_change, support_points};
        if !sys.is_populated {
            return None;
        }
        let fleet = sys.fleets.iter().find_map(|key| world.fleets.get(*key))?;
        let side = if fleet.is_alliance {
            Faction::Alliance
        } else {
            Faction::Empire
        };
        // FUN_00509930: the side with more support, or neither on a tie.
        let (ours, theirs) = match side {
            Faction::Alliance => (
                support_points(sys, Faction::Alliance),
                support_points(sys, Faction::Empire),
            ),
            _ => (
                support_points(sys, Faction::Empire),
                support_points(sys, Faction::Alliance),
            ),
        };
        let parameter = if ours > theirs {
            GNPRTB_BLOCKADE_SUPPORT_POPULAR
        } else {
            GNPRTB_BLOCKADE_SUPPORT_UNPOPULAR
        };
        let delta = world.gnprtb.value(parameter, world.difficulty_index);
        let support_delta = support_change(world, sys, side, delta);
        // A shift of 0 leaves the support as it is.
        (support_delta != 0).then_some(BlockadeEvent::SupportShifted {
            system,
            side,
            support_delta,
            tick,
        })
    }

    /// The chance in percent that a regiment carried into `system` now survives
    /// leaving it (`FUN_0050b310`).
    ///
    /// 100 unless the system is blockaded and has no KDY-150. Otherwise
    /// `max(0, 100 - ships * GNPRTB[7684] - fighters * GNPRTB[7685])` over every
    /// fleet at the system and the squadrons on its planet, both sides
    /// (`FUN_005039d0`, `FUN_0055a020`, clamped by `FUN_0053e120`).
    #[must_use]
    pub fn withdraw_percent(world: &GameWorld, system: SystemKey) -> u8 {
        let Some(sys) = world.systems.get(system) else {
            return FULL_WITHDRAW_PERCENT;
        };
        if !Self::system_is_blockaded(world, sys) {
            return FULL_WITHDRAW_PERCENT;
        }
        let kdy_150 = DatId::new(KDY_150_CLASS_ID);
        let has_kdy_150 = sys
            .defense_facilities
            .iter()
            .filter_map(|&key| world.defense_facilities.get(key))
            .any(|facility| facility.class_dat_id == kdy_150);
        if has_kdy_150 {
            return FULL_WITHDRAW_PERCENT;
        }
        let (ships, fighters) = sys
            .fleets
            .iter()
            .filter_map(|&key| world.fleets.get(key))
            .fold((0_i64, 0_i64), |(ships, fighters), fleet| {
                let squadrons: i64 = fleet.fighters.iter().map(|f| i64::from(f.count)).sum();
                (ships + i64::from(fleet.ship_count()), fighters + squadrons)
            });
        let fighters = fighters + i64::from(crate::squadrons::total(&sys.fighters));
        let param = |id| i64::from(world.gnprtb.value(id, world.difficulty_index));
        let penalty = ships * param(GNPRTB_WITHDRAW_PENALTY_PER_CAPSHIP)
            + fighters * param(GNPRTB_WITHDRAW_PENALTY_PER_FIGHTER);
        let percent = (i64::from(FULL_WITHDRAW_PERCENT) - penalty).clamp(0, 100);
        u8::try_from(percent).unwrap_or(FULL_WITHDRAW_PERCENT)
    }

    /// Track embarked regiments and return those whose fleet has just left a
    /// system with a withdraw percent below 100.
    ///
    /// A regiment first seen aboard a fleet was loaded from the surface, so it
    /// starts at 100. Arriving in a system copies that system's withdraw
    /// percent; leaving a system reports the regiment for a roll. Regiments
    /// no longer embarked (landed or destroyed) are forgotten, which resets
    /// them to 100 as `FUN_00504960` does on activation.
    pub fn running_regiments(
        state: &mut BlockadeState,
        world: &GameWorld,
        movement: &MovementState,
        transport: &TroopTransportState,
    ) -> Vec<RunningRegiment> {
        let mut running = Vec::new();
        let mut aboard = HashSet::new();
        for fleet_key in transport.fleet_keys() {
            let Some(fleet) = world.fleets.get(fleet_key) else {
                continue;
            };
            let orbit = (!movement.is_in_transit(fleet_key)).then_some(fleet.location);
            for &troop in transport.cargo(fleet_key) {
                aboard.insert(troop);
                let regiment = state.embarked.entry(troop).or_insert(EmbarkedRegiment {
                    orbit,
                    withdraw_percent: FULL_WITHDRAW_PERCENT,
                });
                match (regiment.orbit, orbit) {
                    (previous, current) if previous == current => {}
                    (Some(origin), None) => {
                        if regiment.withdraw_percent < FULL_WITHDRAW_PERCENT {
                            running.push(RunningRegiment {
                                troop,
                                system: origin,
                                withdraw_percent: regiment.withdraw_percent,
                            });
                        }
                    }
                    (_, Some(system)) => {
                        regiment.withdraw_percent = Self::withdraw_percent(world, system);
                    }
                    (None, None) => {}
                }
                regiment.orbit = orbit;
            }
        }
        state.embarked.retain(|troop, _| aboard.contains(troop));
        running
    }

    /// Roll each running regiment against its withdraw percent (`FUN_0053e2f0`).
    ///
    /// `rolls` holds one uniform value in `[0, 1)` per regiment. The draw is
    /// `99 - floor(roll * 100)`, uniform over `0..=99` like `FUN_0053e2e0`, and
    /// the regiment survives when the draw is below its percent. A missing or
    /// padded roll of 1.0 always survives.
    #[must_use]
    pub fn resolve_running(
        running: &[RunningRegiment],
        rolls: &[f64],
        tick: u64,
    ) -> Vec<BlockadeEvent> {
        running
            .iter()
            .zip(rolls)
            .filter_map(|(regiment, &roll)| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "roll is in [0, 1], so the floored product fits in i32"
                )]
                let draw = 99 - (roll * 100.0).floor() as i32;
                (draw >= i32::from(regiment.withdraw_percent)).then_some(
                    BlockadeEvent::TroopDestroyed {
                        system: regiment.system,
                        troop: regiment.troop,
                        tick,
                    },
                )
            })
            .collect()
    }

    // ── Private ───────────────────────────────────────────────────────────

    /// The system's blockade bit (`+0x88` `0x20`), as `FUN_0050b8e0` writes
    /// it (`ghidra/notes/blockade-bit.md`). Both sides present is a battle
    /// (bit `0x1000`, `FUN_00509710`), never a blockade. Otherwise a system
    /// held by one side is blockaded by any fleet of the other, and a
    /// contested one, when populated, by either side's fleets. A neutral
    /// system never is.
    ///
    /// A side is present by a fleet or a squadron on the planet
    /// (`FUN_00509710` → `FUN_00503a50`), but only a fleet blockades
    /// (`FUN_0050b8e0` walks `FUN_004ffe70`). port: every fleet and squadron
    /// is active. hyp: `Contested` is the original's side 3, and an
    /// `Uprising` system keeps its holder's side.
    #[must_use]
    pub fn system_is_blockaded(world: &GameWorld, sys: &crate::world::System) -> bool {
        use crate::dat::Faction;

        let side_flags = |sides: &mut dyn Iterator<Item = bool>| {
            sides.fold((false, false), |(alliance, empire), is_alliance| {
                (alliance || is_alliance, empire || !is_alliance)
            })
        };
        let (alliance, empire) = side_flags(
            &mut sys
                .fleets
                .iter()
                .filter_map(|&fk| world.fleets.get(fk))
                .map(|fleet| fleet.is_alliance),
        );
        let (alliance_squadron, empire_squadron) = side_flags(
            &mut sys
                .fighters
                .iter()
                .filter(|entry| entry.count > 0)
                .filter_map(|entry| crate::squadrons::is_alliance(world, entry.class)),
        );
        if (alliance || alliance_squadron) && (empire || empire_squadron) {
            return false;
        }
        match sys.control {
            ControlKind::Contested => sys.is_populated && (alliance || empire),
            control => match control.faction() {
                Some(Faction::Alliance) => empire,
                Some(Faction::Empire) => alliance,
                Some(Faction::Neutral) | None => false,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::{ExplorationStatus, Faction, SectorGroup};
    use crate::ids::DatId;
    use crate::tick::TickEvent;
    use crate::world::{Fleet, GameWorld, Sector, System, TroopUnit};

    fn tick(n: u64) -> TickEvent {
        TickEvent { tick: n }
    }

    fn make_world() -> (GameWorld, SystemKey) {
        let mut world = GameWorld::default();
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let sys = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0000),
            name: "Bespin".into(),
            sector,
            x: 10,
            y: 20,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
        });
        (world, sys)
    }

    fn add_fleet(world: &mut GameWorld, system: SystemKey, is_alliance: bool) {
        let fk = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance,
            has_death_star: false,
        });
        world.systems.get_mut(system).unwrap().fleets.push(fk);
    }

    fn add_troop(world: &mut GameWorld, system: SystemKey, is_alliance: bool) -> TroopKey {
        let tk = world.troops.insert(TroopUnit {
            class_dat_id: DatId::new(0x1400_0000),
            is_alliance,
            regiment_strength: 100,
        });
        world.systems.get_mut(system).unwrap().ground_units.push(tk);
        tk
    }

    #[test]
    fn no_events_without_ticks() {
        let (mut world, sys) = make_world();
        add_fleet(&mut world, sys, false);
        let mut state = BlockadeState::new();
        let events = BlockadeSystem::advance(&mut state, &world, &[]);
        assert!(events.is_empty());
    }

    #[test]
    fn blockade_transitions_use_stable_system_key_order() {
        let (mut world, system_a) = make_world();
        let sector = world.systems[system_a].sector;
        let system_b = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Hoth".into(),
            sector,
            x: 30,
            y: 40,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
        });
        add_fleet(&mut world, system_b, false);
        add_fleet(&mut world, system_a, false);

        let mut state = BlockadeState::new();
        let started = BlockadeSystem::advance(&mut state, &world, &[tick(1)]);
        assert_eq!(
            started,
            vec![
                BlockadeEvent::BlockadeStarted {
                    system: system_a,
                    tick: 1,
                },
                BlockadeEvent::BlockadeStarted {
                    system: system_b,
                    tick: 1,
                },
            ]
        );

        world.systems[system_a].fleets.clear();
        world.systems[system_b].fleets.clear();
        let ended = BlockadeSystem::advance(&mut state, &world, &[tick(2)]);
        assert_eq!(
            ended,
            vec![
                BlockadeEvent::BlockadeEnded {
                    system: system_a,
                    tick: 2,
                },
                BlockadeEvent::BlockadeEnded {
                    system: system_b,
                    tick: 2,
                },
            ]
        );
    }

    #[test]
    fn no_blockade_without_hostile_fleet() {
        let (world, _sys) = make_world();
        let mut state = BlockadeState::new();
        let events = BlockadeSystem::advance(&mut state, &world, &[tick(1)]);
        assert!(events.is_empty());
    }

    #[test]
    fn no_blockade_when_contested() {
        let (mut world, sys) = make_world();
        add_fleet(&mut world, sys, true); // Alliance defender
        add_fleet(&mut world, sys, false); // Empire hostile

        let mut state = BlockadeState::new();
        let events = BlockadeSystem::advance(&mut state, &world, &[tick(1)]);
        // Defender present — no blockade
        assert!(!events
            .iter()
            .any(|e| matches!(e, BlockadeEvent::BlockadeStarted { .. })));
        assert!(!state.is_blockaded(sys));
    }

    #[test]
    fn blockade_starts_when_hostile_fleet_uncontested() {
        let (mut world, sys) = make_world();
        add_fleet(&mut world, sys, false); // Empire fleet at Alliance-controlled system

        let mut state = BlockadeState::new();
        let events = BlockadeSystem::advance(&mut state, &world, &[tick(1)]);
        assert!(events
            .iter()
            .any(|e| matches!(e, BlockadeEvent::BlockadeStarted { .. })));
        assert!(state.is_blockaded(sys));
    }

    #[test]
    fn the_blockade_now_is_what_an_advance_would_store() {
        let (mut world, sys) = make_world();
        assert!(BlockadeSystem::blockaded_now(&world).is_empty());
        add_fleet(&mut world, sys, false);
        let mut state = BlockadeState::new();
        BlockadeSystem::advance(&mut state, &world, &[tick(1)]);
        assert_eq!(
            &BlockadeSystem::blockaded_now(&world),
            state.blockaded_systems()
        );
        assert!(BlockadeSystem::blockaded_now(&world).contains(&sys));
    }

    #[test]
    fn a_blockade_keeps_its_blockader_and_losses_until_it_ends() {
        // FUN_00511300 at a blockade's start clears the system record's
        // counters and stores the blockader (+0x60); System_vtd0 adds the
        // assault and bombardment losses; FUN_00561280 reads them at the end.
        use crate::dat::Faction;
        use crate::loyalty::SideLosses;
        let (mut world, sys) = make_world();
        let mut state = BlockadeState::new();
        let loss = SideLosses {
            alliance: 6,
            empire: 0,
        };
        // Losses before the blockade do not count.
        state.record_losses(sys, loss);
        add_fleet(&mut world, sys, false);
        BlockadeSystem::advance(&mut state, &world, &[tick(1)]);
        assert_eq!(
            state.blockade_losses(sys),
            Some(BlockadeLosses {
                blockader: Faction::Empire,
                losses: SideLosses::default()
            })
        );
        state.record_losses(sys, loss);
        state.record_losses(sys, loss);
        assert!(state.take_ended().is_empty());
        add_fleet(&mut world, sys, true);
        BlockadeSystem::advance(&mut state, &world, &[tick(2)]);
        assert_eq!(
            state.take_ended(),
            [(
                sys,
                BlockadeLosses {
                    blockader: Faction::Empire,
                    losses: SideLosses {
                        alliance: 12,
                        empire: 0
                    }
                }
            )]
        );
        assert!(state.take_ended().is_empty());
        assert_eq!(state.blockade_losses(sys), None);
    }

    #[test]
    fn blockade_ends_when_defender_arrives() {
        let (mut world, sys) = make_world();
        add_fleet(&mut world, sys, false); // Empire blockader

        let mut state = BlockadeState::new();
        BlockadeSystem::advance(&mut state, &world, &[tick(1)]); // starts blockade
        assert!(state.is_blockaded(sys));

        // Alliance fleet arrives
        add_fleet(&mut world, sys, true);
        let events = BlockadeSystem::advance(&mut state, &world, &[tick(2)]);
        assert!(events
            .iter()
            .any(|e| matches!(e, BlockadeEvent::BlockadeEnded { .. })));
        assert!(!state.is_blockaded(sys));
    }

    fn blockaded_under(control: ControlKind, alliance_fleet: bool, empire_fleet: bool) -> bool {
        let (mut world, sys) = make_world();
        world.systems[sys].control = control;
        if alliance_fleet {
            add_fleet(&mut world, sys, true);
        }
        if empire_fleet {
            add_fleet(&mut world, sys, false);
        }
        let mut state = BlockadeState::new();
        BlockadeSystem::advance(&mut state, &world, &[tick(1)]);
        state.is_blockaded(sys)
    }

    /// `make_world`'s Alliance system with a squadron of `alliance`'s side on
    /// its planet and, when `enemy_fleet`, an Empire fleet in orbit.
    fn squadron_world(alliance: bool, enemy_fleet: bool) -> (GameWorld, SystemKey) {
        let (mut world, sys) = make_world();
        let class = world.fighter_classes.insert(crate::world::FighterClass {
            is_alliance: alliance,
            is_empire: !alliance,
            ..crate::world::FighterClass::default()
        });
        crate::squadrons::add(&mut world.systems[sys].fighters, class, 1);
        if enemy_fleet {
            add_fleet(&mut world, sys, false);
        }
        (world, sys)
    }

    // FUN_00509710 -> FUN_00503a50: a squadron on the planet makes its side
    // present, so the enemy fleet meets a battle (0x1000), not a blockade.
    #[test]
    fn the_holder_s_squadron_on_the_planet_turns_a_blockade_into_a_battle() {
        let (world, sys) = squadron_world(true, true);
        assert!(!BlockadeSystem::system_is_blockaded(
            &world,
            &world.systems[sys]
        ));
        let (world, sys) = squadron_world(true, false);
        assert!(!BlockadeSystem::system_is_blockaded(
            &world,
            &world.systems[sys]
        ));
    }

    // FUN_00503a50 finds no squadron in an emptied class entry.
    #[test]
    fn an_emptied_squadron_entry_makes_no_side_present() {
        let (mut world, sys) = squadron_world(true, true);
        world.systems[sys].fighters[0].count = 0;
        assert!(BlockadeSystem::system_is_blockaded(
            &world,
            &world.systems[sys]
        ));
    }

    // FUN_0050b8e0 takes the blockade bit from the fleets (FUN_004ffe70)
    // alone: an enemy squadron on the planet does not blockade.
    #[test]
    fn an_enemy_squadron_on_the_planet_does_not_blockade() {
        let (world, sys) = squadron_world(false, false);
        assert!(!BlockadeSystem::system_is_blockaded(
            &world,
            &world.systems[sys]
        ));
        let (world, sys) = squadron_world(false, true);
        assert!(BlockadeSystem::system_is_blockaded(
            &world,
            &world.systems[sys]
        ));
    }

    // FUN_0050b8e0: a populated side-3 system takes the blockade bit from any
    // fleet whose side differs from 3. hyp: `Contested` is side 3.
    #[test]
    fn either_sides_fleets_alone_blockade_a_contested_system() {
        assert!(blockaded_under(ControlKind::Contested, true, false));
        assert!(blockaded_under(ControlKind::Contested, false, true));
        assert!(!blockaded_under(ControlKind::Contested, false, false));
    }

    // FUN_0050b8e0 → FUN_00509710: both sides present sets the battle bit and
    // clears the blockade bit, whoever holds the system.
    #[test]
    fn both_sides_fleets_make_a_battle_not_a_blockade() {
        for control in [
            ControlKind::Contested,
            ControlKind::Controlled(Faction::Alliance),
            ControlKind::Controlled(Faction::Empire),
        ] {
            assert!(!blockaded_under(control, true, true), "{control:?}");
        }
    }

    // FUN_0050b8e0 writes every bit clear for side 0.
    #[test]
    fn a_neutral_system_is_never_blockaded() {
        assert!(!blockaded_under(ControlKind::Uncontrolled, true, false));
        assert!(!blockaded_under(ControlKind::Uncontrolled, false, true));
    }

    // FUN_0050b8e0: a held system is blockaded by the other side's fleet.
    // hyp: an uprising system keeps its holder's side bits.
    #[test]
    fn an_uprising_system_is_blockaded_by_its_holders_enemy() {
        assert!(blockaded_under(
            ControlKind::Uprising(Faction::Empire),
            true,
            false
        ));
        assert!(!blockaded_under(
            ControlKind::Uprising(Faction::Empire),
            false,
            true
        ));
        assert!(blockaded_under(
            ControlKind::Uprising(Faction::Alliance),
            false,
            true
        ));
        assert!(!blockaded_under(
            ControlKind::Uprising(Faction::Alliance),
            true,
            false
        ));
    }

    #[test]
    fn stationed_defending_troop_survives_blockade_start() {
        let (mut world, sys) = make_world();
        let troop = add_troop(&mut world, sys, true); // Alliance troop at Alliance system
        add_fleet(&mut world, sys, false); // Empire blockades

        let mut state = BlockadeState::new();
        let events = BlockadeSystem::advance(&mut state, &world, &[tick(1)]);

        assert!(events.iter().any(|event| matches!(
            event,
            BlockadeEvent::BlockadeStarted { system, .. } if *system == sys
        )));
        assert!(!events.iter().any(|event| matches!(
            event,
            BlockadeEvent::TroopDestroyed { troop: destroyed, .. } if *destroyed == troop
        )));
    }

    // ── Running the blockade (FUN_0050b310, FUN_00504990) ─────────────────

    use crate::movement::MovementState;
    use crate::troop_transport::TroopTransportState;
    use crate::world::{
        CapitalShipClass, DefenseFacilityInstance, FighterClass, FighterEntry, GnprtbEntry,
        GnprtbParams, ShipInstance,
    };

    fn gnprtb_entry(parameter_id: u32, value: i32) -> GnprtbEntry {
        GnprtbEntry {
            parameter_id,
            development: value,
            alliance_sp_easy: value,
            alliance_sp_medium: value,
            alliance_sp_hard: value,
            empire_sp_easy: value,
            empire_sp_medium: value,
            empire_sp_hard: value,
            multiplayer: value,
        }
    }

    /// `make_world`'s system held by `holder` with the Alliance's support at
    /// `alliance` percent, blockaded by the other side's fleet, with the
    /// shipped GNPRTB 7693..7695, 7681 and 7732.
    fn blockaded_world(holder: Faction, alliance: f32) -> (GameWorld, SystemKey) {
        let (mut world, sys) = make_world();
        world.gnprtb = GnprtbParams::new(
            [(7694, 30), (7693, 1), (7695, -1), (7681, 2), (7732, 40)]
                .into_iter()
                .map(|(id, value)| gnprtb_entry(id, value))
                .collect(),
        );
        let system = &mut world.systems[sys];
        system.control = ControlKind::Controlled(holder);
        system.popularity_alliance = alliance / 100.0;
        system.popularity_empire = 1.0 - alliance / 100.0;
        add_fleet(&mut world, sys, holder == Faction::Empire);
        (world, sys)
    }

    fn support_shifts(
        state: &mut BlockadeState,
        world: &GameWorld,
        days: std::ops::RangeInclusive<u64>,
    ) -> Vec<(u64, Faction, i32)> {
        days.flat_map(|day| BlockadeSystem::advance(state, world, &[tick(day)]))
            .filter_map(|event| match event {
                BlockadeEvent::SupportShifted {
                    tick,
                    side,
                    support_delta,
                    ..
                } => Some((tick, side, support_delta)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_popular_blockader_gains_a_support_point_every_thirty_days() {
        // FUN_00511300 arms timer 0x386 (record +0x54+0x30, minimum GNPRTB
        // 7694 = 30 from FUN_0050e200); FUN_0050cb20 gives the blockading
        // side GNPRTB 7693 (1) when FUN_00509930 names it the more popular.
        let (world, sys) = blockaded_world(Faction::Alliance, 30.0);
        let mut state = BlockadeState::new();
        assert_eq!(
            support_shifts(&mut state, &world, 1..=61),
            vec![(31, Faction::Empire, 1), (61, Faction::Empire, 1)]
        );
        assert_eq!(state.support_timer(sys), Some(91));
    }

    #[test]
    fn an_unpopular_or_tied_blockader_loses_a_support_point() {
        // FUN_0050cb20: GNPRTB 7695 (-1) unless the blockader has more
        // support; FUN_00509930 names neither side on a tie.
        for alliance in [30.0, 50.0] {
            let (world, _) = blockaded_world(Faction::Empire, alliance);
            let mut state = BlockadeState::new();
            assert_eq!(
                support_shifts(&mut state, &world, 1..=31),
                vec![(31, Faction::Alliance, -1)],
                "Alliance support {alliance}"
            );
        }
    }

    #[test]
    fn clearing_a_destroyed_system_drops_its_blockade_and_support_timer() {
        let (world, sys) = blockaded_world(Faction::Alliance, 30.0);
        let mut state = BlockadeState::new();
        BlockadeSystem::advance(&mut state, &world, &[tick(1)]);
        state.clear_blockade(sys);
        assert!(!state.is_blockaded(sys));
        assert_eq!(state.support_timer(sys), None);
    }

    #[test]
    fn every_blockade_event_reports_its_tick() {
        let system = SystemKey::default();
        let events = [
            BlockadeEvent::BlockadeStarted { system, tick: 3 },
            BlockadeEvent::BlockadeEnded { system, tick: 4 },
            BlockadeEvent::TroopDestroyed {
                system,
                troop: TroopKey::default(),
                tick: 5,
            },
            BlockadeEvent::SupportShifted {
                system,
                side: Faction::Empire,
                support_delta: 1,
                tick: 6,
            },
        ];
        let ticks: Vec<u64> = events.iter().map(BlockadeEvent::tick).collect();
        assert_eq!(ticks, vec![3, 4, 5, 6]);
    }

    #[test]
    fn lifting_the_blockade_cancels_its_support_timer() {
        // FUN_00511300 arms 0x386 with the cleared bit, which cancels it
        // (FUN_0053fa60).
        let (mut world, sys) = blockaded_world(Faction::Alliance, 30.0);
        let mut state = BlockadeState::new();
        assert_eq!(support_shifts(&mut state, &world, 1..=10), vec![]);
        assert_eq!(state.support_timer(sys), Some(31));
        world.systems[sys].fleets.clear();
        assert_eq!(support_shifts(&mut state, &world, 11..=40), vec![]);
        assert_eq!(state.support_timer(sys), None);
    }

    /// An Empire fleet of `ships` transports and `squadrons` fighters, carrying
    /// one Empire regiment, orbits an uncontrolled `origin`. `target` is the
    /// Alliance system from `make_world`, undefended, so the fleet blockades it
    /// on arrival. GNPRTB 7684 = 5 and 7685 = 2 are the shipped values.
    struct Run {
        world: GameWorld,
        origin: SystemKey,
        target: SystemKey,
        fleet: crate::ids::FleetKey,
        troop: TroopKey,
        transport: TroopTransportState,
        state: BlockadeState,
    }

    fn setup_run(ships: usize, squadrons: u32) -> Run {
        let (mut world, target) = make_world();
        world.gnprtb = GnprtbParams::new(vec![
            gnprtb_entry(u32::from(GNPRTB_WITHDRAW_PENALTY_PER_CAPSHIP), 5),
            gnprtb_entry(u32::from(GNPRTB_WITHDRAW_PENALTY_PER_FIGHTER), 2),
        ]);
        let mut origin_system = world.systems[target].clone();
        origin_system.name = "Origin".into();
        origin_system.control = ControlKind::Uncontrolled;
        let origin = world.systems.insert(origin_system);
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            troop_capacity: 1,
            ..CapitalShipClass::default()
        });
        let fighter = world.fighter_classes.insert(FighterClass::default());
        let fleet = world.fleets.insert(Fleet {
            location: origin,
            capital_ships: (0..ships)
                .map(|_| ShipInstance::new(class, 100, false))
                .collect(),
            fighters: vec![FighterEntry {
                class: fighter,
                count: squadrons,
            }],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        world.systems[origin].fleets.push(fleet);
        let troop = add_troop(&mut world, origin, false);
        let mut transport = TroopTransportState::default();
        transport.embark(&mut world, fleet, &[troop]).unwrap();
        Run {
            world,
            origin,
            target,
            fleet,
            troop,
            transport,
            state: BlockadeState::new(),
        }
    }

    impl Run {
        /// Observe the regiments with the fleet orbiting its location.
        fn observe_orbiting(&mut self) -> Vec<RunningRegiment> {
            BlockadeSystem::running_regiments(
                &mut self.state,
                &self.world,
                &MovementState::new(),
                &self.transport,
            )
        }

        /// Observe the regiments with the fleet in hyperspace from `from`.
        fn observe_departing(&mut self, from: SystemKey, to: SystemKey) -> Vec<RunningRegiment> {
            let mut movement = MovementState::new();
            assert!(movement.order(self.fleet, from, to, 5));
            self.world.systems[from]
                .fleets
                .retain(|&key| key != self.fleet);
            BlockadeSystem::running_regiments(
                &mut self.state,
                &self.world,
                &movement,
                &self.transport,
            )
        }

        /// Move the fleet into `system`'s orbit.
        fn arrive(&mut self, system: SystemKey) {
            self.world.fleets[self.fleet].location = system;
            self.world.systems[system].fleets.push(self.fleet);
        }

        /// Carry the regiment from the origin into the target and observe it.
        fn carry_into_target(&mut self) {
            assert!(self.observe_orbiting().is_empty());
            assert!(self.observe_departing(self.origin, self.target).is_empty());
            self.arrive(self.target);
            assert!(self.observe_orbiting().is_empty());
        }
    }

    #[test]
    fn a_regiment_carried_into_a_blockaded_system_takes_its_withdraw_percent() {
        // FUN_0050b310 + FUN_0055a020: 100 - 10 ships * 5 - 5 squadrons * 2 = 40,
        // copied into the arriving regiment by FUN_0050c540 -> FUN_00504470.
        let mut run = setup_run(10, 5);
        run.carry_into_target();

        assert_eq!(
            run.state.embarked_regiment(run.troop),
            Some(EmbarkedRegiment {
                orbit: Some(run.target),
                withdraw_percent: 40,
            })
        );
        assert_eq!(
            run.observe_departing(run.target, run.origin),
            vec![RunningRegiment {
                troop: run.troop,
                system: run.target,
                withdraw_percent: 40,
            }]
        );
    }

    #[test]
    fn the_withdraw_percent_never_falls_below_zero() {
        // FUN_0053e120 clamps the FUN_0055a020 result at 0: 100 - 30 * 5 < 0.
        let mut run = setup_run(30, 0);
        run.carry_into_target();
        assert_eq!(
            run.state
                .embarked_regiment(run.troop)
                .map(|r| r.withdraw_percent),
            Some(0)
        );
    }

    #[test]
    fn squadrons_on_the_planet_lower_the_withdraw_percent() {
        // FUN_0050b310 counts the fighters aboard each ship and at the system
        // (FUN_005039d0): 100 - 10 ships * 5 - (5 + 3) squadrons * 2 = 34.
        let mut run = setup_run(10, 5);
        let class = run.world.fleets[run.fleet].fighters[0].class;
        crate::squadrons::add(&mut run.world.systems[run.target].fighters, class, 3);
        run.carry_into_target();

        assert_eq!(
            run.state
                .embarked_regiment(run.troop)
                .map(|regiment| regiment.withdraw_percent),
            Some(34)
        );
    }

    #[test]
    fn a_kdy_150_protects_regiments_leaving_a_blockaded_system() {
        // FUN_0050b310 keeps 100 when FUN_00527550 finds an active family-0x22
        // facility (DEFFACSD record 1, KDY-150).
        let mut run = setup_run(10, 5);
        let kdy = run
            .world
            .defense_facilities
            .insert(DefenseFacilityInstance {
                class_dat_id: DatId::new(KDY_150_CLASS_ID),
                side: crate::dat::Faction::Alliance,
            });
        run.world.systems[run.target].defense_facilities.push(kdy);
        run.carry_into_target();

        assert!(run.observe_departing(run.target, run.origin).is_empty());
    }

    #[test]
    fn a_regiment_loaded_at_a_blockaded_system_leaves_it_safely() {
        // Loading onto a fleet does not add the regiment to a new system, so
        // FUN_00508660 never lowers the 100 set when it was activated.
        let mut run = setup_run(10, 5);
        // First seen aboard while already orbiting the blockaded target.
        let (origin, fleet) = (run.origin, run.fleet);
        run.world.systems[origin].fleets.retain(|&key| key != fleet);
        run.arrive(run.target);

        assert!(run.observe_orbiting().is_empty());
        assert_eq!(
            run.state
                .embarked_regiment(run.troop)
                .map(|r| r.withdraw_percent),
            Some(FULL_WITHDRAW_PERCENT)
        );
        assert!(run.observe_departing(run.target, run.origin).is_empty());
    }

    #[test]
    fn landing_resets_a_regiment_to_a_full_withdraw_percent() {
        // Activation resets the percent to 100 (FUN_004f7410 -> FUN_00504960).
        let mut run = setup_run(10, 5);
        run.carry_into_target();
        let landed = run
            .transport
            .disembark_all(&mut run.world, run.fleet, run.target)
            .unwrap();
        assert_eq!(landed, vec![run.troop]);
        assert!(run.observe_orbiting().is_empty());
        assert_eq!(run.state.embarked_regiment(run.troop), None);

        run.transport
            .embark(&mut run.world, run.fleet, &[run.troop])
            .unwrap();
        assert!(run.observe_orbiting().is_empty());
        assert!(run.observe_departing(run.target, run.origin).is_empty());
    }

    #[test]
    fn a_regiment_leaving_an_unblockaded_system_is_never_rolled() {
        let mut run = setup_run(10, 5);
        // An Alliance defender at the target means no blockade.
        add_fleet(&mut run.world, run.target, true);
        run.carry_into_target();
        assert!(run.observe_departing(run.target, run.origin).is_empty());
    }

    #[test]
    fn the_withdraw_roll_keeps_draws_below_the_percent() {
        // FUN_0053e2f0: survive when random(0..=99) < percent. The draw is
        // 99 - floor(roll * 100): 0.605 draws 39, 0.595 draws 40.
        let troop = TroopKey::default();
        let system = SystemKey::default();
        let running = [RunningRegiment {
            troop,
            system,
            withdraw_percent: 40,
        }];
        assert!(BlockadeSystem::resolve_running(&running, &[0.605], 7).is_empty());
        assert_eq!(
            BlockadeSystem::resolve_running(&running, &[0.595], 7),
            vec![BlockadeEvent::TroopDestroyed {
                system,
                troop,
                tick: 7,
            }]
        );
        // The roll pool pads with 1.0, which must never destroy a regiment.
        assert!(BlockadeSystem::resolve_running(&running, &[1.0], 7).is_empty());
        assert!(BlockadeSystem::resolve_running(&running, &[], 7).is_empty());
    }

    #[test]
    fn destroying_a_running_regiment_removes_it_from_cargo_and_the_arena() {
        let mut run = setup_run(1, 0);
        assert!(run.transport.destroy_embarked(&mut run.world, run.troop));
        assert!(run.transport.cargo(run.fleet).is_empty());
        assert!(!run.world.troops.contains_key(run.troop));
        assert!(!run.transport.destroy_embarked(&mut run.world, run.troop));
    }
}
