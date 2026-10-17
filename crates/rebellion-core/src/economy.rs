//! Per-system economy tick loop — the territorial control feedback system.
//!
//! Implements the 18 sub-functions from the original `FUN_00508250`
//! (the community dump calls it `FUN_005073d0_adjust_and_deploy_each_system`).
//! The port runs it every tick for every system; the original runs it once at
//! setup (`ghidra/notes/timer-scheduler.md`). Governs//! the smuggling percent, collection rates, garrison requirements, and
//! resource allocation. It never moves popular support
//! (`ghidra/notes/economy-cadence.md`).
//!
//! Open Souls mapping: this is a "cognitive step" — a pure function that transforms
//! economy state + read-only `GameWorld` into a Vec of effects.
//!
//! GNPRTB parameters used:
//!   7686 = 10 (smuggling: per capital ship)
//!   7687 = 5  (smuggling: per squadron)
//!   7688 = 2  (smuggling: per regiment)
//!   7732 = 40 (smuggling: support ceiling)
//!   7733 = 25 (smuggling base above 30 support)
//!   7734 = 30 (smuggling: middle band's top)
//!   7735 = 50 (smuggling base from 21 to 30 support)
//!   7736 = 20 (smuggling: low band's top)
//!   7737 = 75 (smuggling base up to 20 support)
//!   7761 = 60 (garrison requirement threshold)
//!   7762 = -10 (garrison requirement divisor)
//!   7763 = 100 (collection rate base)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ids::SystemKey;
use crate::tick::TickEvent;
use crate::world::{ControlKind, GameWorld, GnprtbParams};

// ---------------------------------------------------------------------------
// GNPRTB parameter IDs
// ---------------------------------------------------------------------------

// FUN_00559c40's weights, bands and bases (`parameters.tsv`).
/// `DAT_006bb3f4` (10): per capital ship (`FUN_00559c40`).
const GNPRTB_SMUGGLING_CAPITAL_SHIP: u16 = 7686;
/// `DAT_006bb3c0` (5): per squadron (`FUN_00559c40`).
const GNPRTB_SMUGGLING_SQUADRON: u16 = 7687;
/// `DAT_006bb3bc` (2): per regiment (`FUN_00559c40`).
const GNPRTB_SMUGGLING_REGIMENT: u16 = 7688;
/// `DAT_006bb478` (40): the most support that smuggles (`FUN_00559c40`).
const GNPRTB_SMUGGLING_SUPPORT_CEILING: u16 = 7732;
/// `DAT_006bb3f8` (25): the base above the middle band (`FUN_00559c40`).
const GNPRTB_SMUGGLING_BASE_HIGH: u16 = 7733;
/// `DAT_006bb434` (30): the middle band's top (`FUN_00559c40`).
const GNPRTB_SMUGGLING_MID_BAND: u16 = 7734;
/// `DAT_006bb4b0` (50): the middle band's base (`FUN_00559c40`).
const GNPRTB_SMUGGLING_BASE_MID: u16 = 7735;
/// `DAT_006bb438` (20): the low band's top (`FUN_00559c40`).
const GNPRTB_SMUGGLING_LOW_BAND: u16 = 7736;
/// `DAT_006bb424` (75): the low band's base (`FUN_00559c40`).
const GNPRTB_SMUGGLING_BASE_LOW: u16 = 7737;
const GNPRTB_GARRISON_THRESHOLD: u16 = 7761;
const GNPRTB_GARRISON_DIVISOR: u16 = 7762;
const GNPRTB_COLLECTION_RATE_BASE: u16 = 7763;
/// `FUN_0050b2c0`'s value with no holder, and the facility default
/// (`FUN_0055a600`).
const FULL_PRODUCTION_MODIFIER: i32 = 100;

// Phase 3b: new constants from source tree cross-reference
const GNPRTB_EMPIRE_TROOP_MULT: u16 = 7680; // =2: Empire regiments doubled, garrison halved near a Death Star (+0x88 bit 11)
const GNPRTB_UPRISING_GARRISON_MULT: u16 = 7682; // =2: garrison doubles during uprising
const GNPRTB_KDY_CAPSHIP_PENALTY: u16 = 7684; // =5: KDY production penalty per capital ship
const GNPRTB_KDY_FIGHTER_PENALTY: u16 = 7685; // =2: KDY production penalty per fighter
/// GNPRTB 7760 (60, `DAT_006bb4c0`): the support a side needs to hold a
/// system without regiments (`FUN_0055a080`).
const GNPRTB_SUPPORT_CONTROL_THRESHOLD: u16 = 7760;
const GNPRTB_MAINTENANCE_RATE_CONTROLLED: u16 = 7694; // =30: ticks between maintenance checks (controlled)

// ---------------------------------------------------------------------------
// Economy state
// ---------------------------------------------------------------------------

/// Incident state flags (original system `+0x88` bits 16-19).
/// When a flag turns on between ticks, the corresponding incident message fires.
///
/// In the original each incident is a one-tick pulse raised by a timer event,
/// and the system vtable (`0x0065e640`) holds each incident's effect.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "These independent flags preserve the existing state and serialization model."
)]
pub struct IncidentFlags {
    /// Bit 16 (0x10000), `FUN_0050aa50`; views get
    /// `SystemUprisingIncidentNotif` (`FUN_00512580`). Raised by
    /// `UprisingSystem`; always clear here, kept for the save layout.
    pub uprising: bool,
    /// Bit 17 (0x20000), `FUN_0050aac0`; views get
    /// `SystemInformantIncidentNotif` (`FUN_005125d0`). Our trigger has no
    /// source (F-029).
    pub informant: bool,
    /// Bit 18 (0x40000), `FUN_0050ab30`; views get
    /// `SystemDisasterIncidentNotif` (`FUN_00512620`). Raised by
    /// `UprisingSystem`; always clear here, kept for the save layout.
    pub disaster: bool,
    /// Bit 19 (0x80000), `FUN_0050aba0`; views get
    /// `SystemResourceIncidentNotif` (`FUN_00512670`). Our overcap trigger
    /// has no source (F-029).
    pub resource: bool,
}

/// Fleet posture summary for a system (`FUN_0050bc60`/`FUN_0050be00`/`FUN_0050c350`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FleetPosture {
    /// Number of Alliance capital ship hulls at this system.
    pub alliance_capships: u32,
    /// Number of Empire capital ship hulls at this system.
    pub empire_capships: u32,
    /// True if both sides have fleets (2+ each) present.
    pub is_contested: bool,
}

/// Fighter posture for a system (`FUN_0050b8e0`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FighterPosture {
    /// True if both sides have fighters present.
    pub is_contested: bool,
    /// Alliance fighter count.
    pub alliance_fighters: u32,
    /// Empire fighter count.
    pub empire_fighters: u32,
}

/// Derived summary computed per-system per-tick (functions 9-15 of economy pipeline).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemSummary {
    /// Troops present minus garrison requirement (negative = deficit).
    /// `FUN_0050b500`.
    pub troop_surplus: i32,
    /// Total troops for the controlling faction.
    /// `FUN_0050ba90`.
    pub total_controlling_troops: u32,
    /// Whether an orbital shipyard is present.
    /// `FUN_0050bb70`.
    pub has_shipyard: bool,
    /// Fleet posture (3-pass result).
    pub fleet_posture: FleetPosture,
    /// Fighter posture.
    pub fighter_posture: FighterPosture,
    /// System `+0x88` bit 11, `DeathStarNearby` ([`death_star_nearby`]).
    pub death_star_nearby: bool,
}

/// Discrete bands of the controlling faction's popular support.
/// Used to detect `EVT_SUPPORT_CHANGE` (0x100) transitions without inventing
/// new `GameWorld` state — the previous band is cached on `SystemEconomy`.
///
/// Matches the GNPRTB support thresholds already in use:
///   Critical: support ≤ 20 (GNPRTB[7736])
///   Low:      21–30          (GNPRTB[7734])
///   Moderate: 31–40          (GNPRTB[7732])
///   High:     41–60          (GNPRTB[7761] — garrison stable band)
///   Secure:   > 60           (no smuggling, garrison = 0)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SupportTier {
    #[default]
    Critical = 0,
    Low = 1,
    Moderate = 2,
    High = 3,
    Secure = 4,
}

impl SupportTier {
    /// Compute the tier from integer support (0-100).
    ///
    /// hyp: the band edges are the stock values of the drift and garrison
    /// thresholds listed on the type, hard-coded here; no recovered function
    /// shows the original announcing a support change at these edges (audit
    /// F-031).
    #[must_use]
    pub fn from_support_int(support_int: i32) -> Self {
        if support_int <= 20 {
            Self::Critical
        } else if support_int <= 30 {
            Self::Low
        } else if support_int <= 40 {
            Self::Moderate
        } else if support_int <= 60 {
            Self::High
        } else {
            Self::Secure
        }
    }
}

/// The inputs whose System hooks recompute the smuggling percent and the
/// `ProductionModifier` (`ghidra/notes/economy-cadence.md`, "The hooks"): the
/// support `+0x58` (slot `+0x1d0`, `FUN_005109f0`), `+0x88` bit 11 (slot
/// `+0x238`, `FUN_00511740`) and the side bits `+0x24` (slot `+0xf0`,
/// `FUN_00510820`). Ships, squadrons and regiments coming or going
/// (`FUN_00508660`) recompute neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookInputs {
    /// The Alliance's support in points (`+0x58`).
    pub alliance_support: i32,
    /// `+0x88` bit 11 ([`death_star_nearby`]).
    pub death_star_nearby: bool,
    /// The side bits: the holder, if any.
    pub side: Option<crate::dat::Faction>,
}

/// Per-system resource and economy tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "These independent flags preserve the existing state and serialization model."
)]
pub struct SystemEconomy {
    /// The hook inputs seen on the last run, or `None` before the first
    /// (the setup rebuild, `FUN_00508250`, computes every field) (v63).
    pub hook_inputs: Option<HookInputs>,
    /// System `+0x6c` `SmugglingPercent`, -100..100 (`FUN_0050b230`).
    pub smuggling_percent: i32,
    /// System `+0x70` `ProductionModifier` ([`production_modifier`]), kept
    /// under its old name and type for the save layout.
    pub collection_rate: f32,
    /// Number of troops required to prevent uprising.
    pub garrison_requirement: u32,
    /// Production speed modifier from fleet/KDY presence (0-100).
    pub production_modifier: i8,
    /// Current energy output (sum of production facility outputs, capped at system capacity).
    /// `FUN_0050ad60`: if `energy_allocated` > `System.total_energy`, cap it.
    pub energy_allocated: u32,
    /// Current raw material output (sum of mine outputs, capped at system capacity).
    /// `FUN_0050b0b0`: if `raw_material_allocated` > `System.raw_materials`, cap it.
    pub raw_material_allocated: u32,
    /// True if facilities exceed energy capacity (facility pruning needed).
    pub energy_overcapped: bool,
    /// True if mines exceed raw material capacity (mine pruning needed).
    pub raw_material_overcapped: bool,
    /// Derived troop/fleet/shipyard summary (functions 9-15).
    pub summary: SystemSummary,
    /// Incident state flags (original system `+0x88` bits 16-19).
    /// When one turns on from the previous tick, its incident message fires.
    pub incident_flags: IncidentFlags,
    /// System `+0x88` bit 9 (`FUN_0050bb00`): held, populated, in revolt, and
    /// not short of troops.
    pub uprising_visible: bool,
    /// Previous-tick support band for the controlling faction. Drives
    /// `EVT_SUPPORT_CHANGE` (0x100) transition detection. Persists across
    /// save/load because it lives in `EconomyState` (see #F2) — not on
    /// `world::System`. No `#[serde(default)]`: v8 is the migration
    /// boundary (see Dabora 1 cleanup — bincode is positional).
    pub last_support_tier: SupportTier,
    /// True once the controlling faction has established this system in
    /// a prior tick. Suppresses `EVT_RESOURCE_DISCOVERY` (0x155) on the
    /// very first eval — the galaxy seed itself is not a "discovery".
    pub resource_discovery_armed: bool,
}

impl Default for SystemEconomy {
    fn default() -> Self {
        Self {
            hook_inputs: None,
            smuggling_percent: 0,
            collection_rate: 1.0,
            garrison_requirement: 0,
            production_modifier: 0,
            energy_allocated: 0,
            raw_material_allocated: 0,
            energy_overcapped: false,
            raw_material_overcapped: false,
            summary: SystemSummary::default(),
            incident_flags: IncidentFlags::default(),
            uprising_visible: false,
            last_support_tier: SupportTier::default(),
            resource_discovery_armed: false,
        }
    }
}

/// Economy state across all systems.
///
/// Simulation state (not world state). Saved via `SaveState::economy`
/// since Dabora 1 (#F2) to close the incident re-fire bug. The two
/// `*_maintenance_cooldown` counters implement the per-faction 30-tick
/// `EVT_MAINTENANCE_SHORTFALL_EVENT` (0x304) timer from the plan's #K4
/// — they decrement each tick in `EconomySystem::advance` and emit +
/// reset to `GNPRTB[7694]` when they reach zero **and** at least one
/// controlled system has a garrison deficit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EconomyState {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub per_system: HashMap<SystemKey, SystemEconomy>,
    /// Ticks until the next Alliance maintenance-shortfall check fires.
    /// Initialized lazily to `GNPRTB[7694]` (30) on the first tick.
    pub alliance_maintenance_cooldown: u32,
    /// Ticks until the next Empire maintenance-shortfall check fires.
    pub empire_maintenance_cooldown: u32,
}

// ---------------------------------------------------------------------------
// Economy events (system output)
// ---------------------------------------------------------------------------

/// Events produced by the economy tick.
#[derive(Debug, Clone)]
pub enum EconomyEvent {
    /// The system's smuggling percent changed (`FUN_00509ec0`, which
    /// notifies only on a change).
    SmugglingPercentChanged { system: SystemKey, percent: i32 },
    /// Collection rate recalculated.
    CollectionRateChanged { system: SystemKey, new_rate: f32 },
    /// Garrison requirement recalculated.
    GarrisonRequirementChanged {
        system: SystemKey,
        new_requirement: u32,
    },
    /// System control resolved from troop presence (`FUN_0050b610`).
    ControlResolved {
        system: SystemKey,
        new_control: ControlKind,
    },
    /// Incident state changed — fire the corresponding notification (`FUN_0050b800` + `FUN_0050e5b0`).
    IncidentTriggered {
        system: SystemKey,
        incident_type: &'static str,
    },
    /// Energy allocated exceeds system capacity (`FUN_0050ad60`).
    EnergyOvercapped {
        system: SystemKey,
        allocated: u32,
        capacity: u32,
    },
    /// Raw material output exceeds system capacity (`FUN_0050b0b0`).
    RawMaterialOvercapped {
        system: SystemKey,
        allocated: u32,
        capacity: u32,
    },
    // ── Knesset Shamash-Bet notification events (Dabora 2) ──────────────
    /// K1: `EVT_SUPPORT_CHANGE` (0x100) — controlling faction's popular
    /// support crossed a tier boundary (see `SupportTier`).
    SupportChanged {
        system: SystemKey,
        from: SupportTier,
        to: SupportTier,
    },
    /// K3: `EVT_RESOURCE_DISCOVERY` (0x155) — a new mine came online at
    /// a previously-seeded system. Detected as a positive delta on
    /// `eco.raw_material_allocated` (after the `resource_discovery_armed`
    /// gate disarms the seed tick).
    ResourceDiscovered { system: SystemKey, new_output: u32 },
    /// K4: `EVT_MAINTENANCE_SHORTFALL_EVENT` (0x304) — the per-faction
    /// 30-tick timer fired AND the faction has at least one controlled
    /// system with a garrison deficit.
    MaintenanceShortfall {
        faction_is_alliance: bool,
        deficit_system_count: u32,
    },
}

// ---------------------------------------------------------------------------
// Economy system
// ---------------------------------------------------------------------------

pub struct EconomySystem;

impl EconomySystem {
    /// Run the per-system economy tick with no system in revolt. See
    /// [`EconomySystem::advance_with_uprisings`].
    pub fn advance(
        state: &mut EconomyState,
        world: &GameWorld,
        tick_events: &[TickEvent],
        difficulty: u8,
    ) -> Vec<EconomyEvent> {
        Self::advance_with_uprisings(
            state,
            world,
            tick_events,
            difficulty,
            &crate::uprising::UprisingState::default(),
        )
    }

    /// Run the per-system economy tick. Mutates `EconomyState` in-place,
    /// returns events for the integrator.
    ///
    /// `uprisings` supplies system `+0x88` bit 2, which doubles the garrison
    /// requirement (`FUN_0050b5a0`).
    ///
    /// Runs BEFORE manufacturing in the tick order (economy affects production).
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    pub fn advance_with_uprisings(
        state: &mut EconomyState,
        world: &GameWorld,
        tick_events: &[TickEvent],
        difficulty: u8,
        uprisings: &crate::uprising::UprisingState,
    ) -> Vec<EconomyEvent> {
        if tick_events.is_empty() {
            return Vec::new();
        }

        let gnprtb = &world.gnprtb;
        let mut events = Vec::new();

        // Per-faction maintenance-shortfall deficit tally (K4 intra-tick scratch).
        // Counted during the per-system loop and checked against the 30-tick
        // cooldown at the end of `advance`.
        let mut alliance_deficit_systems: u32 = 0;
        let mut empire_deficit_systems: u32 = 0;

        // Collect system keys first to avoid borrow issues.
        let system_keys: Vec<SystemKey> = world.systems.keys().collect();

        for sys_key in system_keys {
            let Some(sys) = world.systems.get(sys_key) else {
                continue;
            };

            // Only process populated, non-destroyed systems.
            if !sys.is_populated || sys.is_destroyed {
                continue;
            }

            // Count military presence at this system.
            let presence = count_military_presence(world, sys);

            // 0a. Resource capacity enforcement (FUN_0050ad60 + FUN_0050ad80 + FUN_0050b0b0).
            // Sum facility/mine outputs and cap at system limits.
            let eco = state.per_system.entry(sys_key).or_default();
            let prev_raw_allocated = eco.raw_material_allocated;
            let (energy_alloc, raw_alloc) = calculate_resource_allocation(world, sys);
            let energy_cap = u32::from(sys.total_energy);
            let raw_cap = u32::from(sys.raw_materials);

            eco.energy_allocated = energy_alloc.min(energy_cap);
            eco.energy_overcapped = energy_alloc > energy_cap;
            eco.raw_material_allocated = raw_alloc.min(raw_cap);
            eco.raw_material_overcapped = raw_alloc > raw_cap;

            // K3: EVT_RESOURCE_DISCOVERY (0x155).
            // Fire when a new mine comes online (capped raw material output
            // strictly increases). Suppress on the first armed pass — the
            // galaxy seed itself is not a "discovery".
            let new_raw = eco.raw_material_allocated;
            if eco.resource_discovery_armed && new_raw > prev_raw_allocated {
                events.push(EconomyEvent::ResourceDiscovered {
                    system: sys_key,
                    new_output: new_raw,
                });
            }
            eco.resource_discovery_armed = true;

            if eco.energy_overcapped {
                events.push(EconomyEvent::EnergyOvercapped {
                    system: sys_key,
                    allocated: energy_alloc,
                    capacity: energy_cap,
                });
            }
            if eco.raw_material_overcapped {
                events.push(EconomyEvent::RawMaterialOvercapped {
                    system: sys_key,
                    allocated: raw_alloc,
                    capacity: raw_cap,
                });
            }

            // The hooks' inputs. port: a change made after the economy has
            // run is seen on the next day's run.
            let nearby = death_star_nearby(world, sys);
            let inputs = HookInputs {
                alliance_support: crate::uprising::support_points(
                    sys,
                    crate::dat::Faction::Alliance,
                ),
                death_star_nearby: nearby,
                side: sys.control.faction(),
            };
            let last = eco.hook_inputs.replace(inputs);
            let support_changed =
                last.is_none_or(|last| last.alliance_support != inputs.alliance_support);

            // 1. The smuggling percent (FUN_0050b230), recomputed by the
            // support, bit 11 and side hooks only. It does not move the
            // support.
            if last != Some(inputs) {
                let percent = smuggling_percent(world, sys, nearby, gnprtb, difficulty);
                if percent != eco.smuggling_percent {
                    eco.smuggling_percent = percent;
                    events.push(EconomyEvent::SmugglingPercentChanged {
                        system: sys_key,
                        percent,
                    });
                }
            }

            // 2. ProductionModifier (FUN_0050b2c0), recomputed by the
            // support hook only.
            let controlling_support = match sys.control {
                ControlKind::Controlled(crate::dat::Faction::Alliance) => sys.popularity_alliance,
                ControlKind::Controlled(crate::dat::Faction::Empire) => sys.popularity_empire,
                _ => 0.5, // Neutral/contested: baseline
            };
            #[expect(
                clippy::cast_precision_loss,
                reason = "The modifier is at most 10000 times GNPRTB 7763."
            )]
            let new_rate = if support_changed {
                let holder_support = crate::uprising::holder(sys)
                    .map(|side| crate::uprising::support_points(sys, side));
                production_modifier(holder_support, gnprtb, difficulty) as f32
            } else {
                eco.collection_rate
            };

            // 3. Calculate garrison requirement.
            let new_garrison = calculate_garrison_requirement(
                controlling_support,
                sys.control,
                nearby,
                uprisings.is_uprising(sys_key),
                gnprtb,
                difficulty,
            );

            // 4. Blockade withdraw percent (FUN_0050b310 + FUN_0055a020).
            // Original formula: 100 unless the system is blockaded (+0x88 bit5)
            // with no KDY-150, else max(0, 100 - ships*GNPRTB[7684] - fighters*GNPRTB[7685]).
            // Our production modifier use has no recovered source (audit finding F-023).
            let capship_penalty = gnprtb.value(GNPRTB_KDY_CAPSHIP_PENALTY, difficulty);
            let fighter_penalty = gnprtb.value(GNPRTB_KDY_FIGHTER_PENALTY, difficulty);
            let total_capships =
                presence.alliance_capships.cast_signed() + presence.empire_capships.cast_signed();
            let total_fighters =
                presence.alliance_fighters.cast_signed() + presence.empire_fighters.cast_signed();
            let new_prod_mod =
                (100 - total_capships * capship_penalty - total_fighters * fighter_penalty)
                    .clamp(0, 100) as i8;

            // 5. The holder (FUN_0050b610): the regiments' side, else the
            // support side.
            let resolved_control = resolve_system_control(
                sys,
                presence.alliance_troops > 0,
                presence.empire_troops > 0,
                gnprtb,
                difficulty,
            );
            if resolved_control != sys.control {
                events.push(EconomyEvent::ControlResolved {
                    system: sys_key,
                    new_control: resolved_control,
                });
            }

            // Update per-system economy state (eco already bound above in step 0a).
            let old_rate = eco.collection_rate;
            let old_garrison = eco.garrison_requirement;
            eco.collection_rate = new_rate;
            eco.garrison_requirement = new_garrison;
            eco.production_modifier = new_prod_mod;

            // 6. Troop/fleet summary propagation (FUN_0050b500 through FUN_0050b8e0).
            eco.summary = compute_system_summary(world, sys, &presence, eco.garrison_requirement);

            // 7. Incident state + uprising visibility (FUN_0050bb00).
            // The uprising and disaster incidents are timer events that
            // `UprisingSystem` raises (F-026). The informant and resource
            // triggers below have no recovered source (F-029).
            let new_flags = evaluate_incident_flags(&eco.summary, eco);
            let old_flags = &eco.incident_flags;
            if new_flags.informant && !old_flags.informant {
                events.push(EconomyEvent::IncidentTriggered {
                    system: sys_key,
                    incident_type: "informant",
                });
            }
            if new_flags.resource && !old_flags.resource {
                // NOTE: this is the legacy "grid overcap" incident, not K3.
                // K3 (EVT_RESOURCE_DISCOVERY) fires above on positive
                // raw_material_allocated deltas.
                events.push(EconomyEvent::IncidentTriggered {
                    system: sys_key,
                    incident_type: "resource",
                });
            }
            eco.incident_flags = new_flags;

            // FUN_0050bb00 sets system +0x88 bit 9 for a populated, held
            // system in revolt whose troop surplus is not negative.
            eco.uprising_visible = sys.control.faction().is_some()
                && uprisings.is_uprising(sys_key)
                && eco.summary.troop_surplus >= 0;

            // K4 pre-tally: track per-faction controlled systems with a
            // garrison deficit. The actual emission happens after the loop
            // when the 30-tick cooldown fires.
            if eco.summary.troop_surplus < 0 {
                match sys.control {
                    ControlKind::Controlled(crate::dat::Faction::Alliance) => {
                        alliance_deficit_systems += 1;
                    }
                    ControlKind::Controlled(crate::dat::Faction::Empire) => {
                        empire_deficit_systems += 1;
                    }
                    _ => {}
                }
            }

            // K1: EVT_SUPPORT_CHANGE (0x100).
            // Compare previous tier (cached in eco) against the freshly-
            // computed current tier. Emit on any transition; update the
            // cache unconditionally so the next tick sees the new band.
            let current_support_int = match sys.control {
                ControlKind::Controlled(crate::dat::Faction::Alliance) => {
                    (sys.popularity_alliance * 100.0).round() as i32
                }
                ControlKind::Controlled(crate::dat::Faction::Empire) => {
                    (sys.popularity_empire * 100.0).round() as i32
                }
                _ => -1, // Sentinel: uncontrolled/contested has no tier
            };
            if current_support_int >= 0 {
                let new_tier = SupportTier::from_support_int(current_support_int);
                if new_tier != eco.last_support_tier {
                    events.push(EconomyEvent::SupportChanged {
                        system: sys_key,
                        from: eco.last_support_tier,
                        to: new_tier,
                    });
                    eco.last_support_tier = new_tier;
                }
            }

            if (new_rate - old_rate).abs() > 0.01 {
                events.push(EconomyEvent::CollectionRateChanged {
                    system: sys_key,
                    new_rate,
                });
            }

            if new_garrison != old_garrison {
                events.push(EconomyEvent::GarrisonRequirementChanged {
                    system: sys_key,
                    new_requirement: new_garrison,
                });
            }
        }

        // K4: EVT_MAINTENANCE_SHORTFALL_EVENT (0x304) per-faction 30-tick
        // timer. Ticks elapsed this frame = tick_events.len(). If any of
        // our counters reach zero AND the faction has at least one
        // deficit system, emit and reset to GNPRTB[7694] (30).
        let elapsed = tick_events.len() as u32;
        let maintenance_rate = gnprtb
            .value(GNPRTB_MAINTENANCE_RATE_CONTROLLED, difficulty)
            .max(1) as u32;

        // Initialize lazily on first tick — `EconomyState::default()`
        // leaves cooldowns at 0, which is the "fire immediately on
        // first eval" state. Arm to one full cycle so the first
        // emission happens after `maintenance_rate` ticks.
        if state.alliance_maintenance_cooldown == 0 && elapsed > 0 && alliance_deficit_systems == 0
        {
            state.alliance_maintenance_cooldown = maintenance_rate;
        }
        if state.empire_maintenance_cooldown == 0 && elapsed > 0 && empire_deficit_systems == 0 {
            state.empire_maintenance_cooldown = maintenance_rate;
        }

        state.alliance_maintenance_cooldown =
            state.alliance_maintenance_cooldown.saturating_sub(elapsed);
        state.empire_maintenance_cooldown =
            state.empire_maintenance_cooldown.saturating_sub(elapsed);

        if state.alliance_maintenance_cooldown == 0 && alliance_deficit_systems > 0 {
            events.push(EconomyEvent::MaintenanceShortfall {
                faction_is_alliance: true,
                deficit_system_count: alliance_deficit_systems,
            });
            state.alliance_maintenance_cooldown = maintenance_rate;
        }
        if state.empire_maintenance_cooldown == 0 && empire_deficit_systems > 0 {
            events.push(EconomyEvent::MaintenanceShortfall {
                faction_is_alliance: false,
                deficit_system_count: empire_deficit_systems,
            });
            state.empire_maintenance_cooldown = maintenance_rate;
        }

        events
    }
}

// ---------------------------------------------------------------------------
// Military presence counting
// ---------------------------------------------------------------------------

/// Military presence at a system, used by the holder, summary and KDY calculations.
struct MilitaryPresence {
    alliance_fleets: u32,
    empire_fleets: u32,
    alliance_fighters: u32,
    empire_fighters: u32,
    alliance_troops: u32,
    empire_troops: u32,
    /// Total alive capital ship hulls (count of alive `ShipInstances` across all fleets).
    /// Used by blockade withdraw percent (`FUN_0050b310`); production modifier use has no recovered source.
    alliance_capships: u32,
    empire_capships: u32,
}

fn count_military_presence(world: &GameWorld, sys: &crate::world::System) -> MilitaryPresence {
    let mut presence = MilitaryPresence {
        alliance_fleets: 0,
        empire_fleets: 0,
        alliance_fighters: 0,
        empire_fighters: 0,
        alliance_troops: 0,
        empire_troops: 0,
        alliance_capships: 0,
        empire_capships: 0,
    };

    for &fleet_key in &sys.fleets {
        if let Some(fleet) = world.fleets.get(fleet_key) {
            let capship_count: u32 = fleet.ship_count();
            let fighter_count: u32 = fleet.fighters.iter().map(|f| f.count).sum();
            if fleet.is_alliance {
                presence.alliance_fleets += 1;
                presence.alliance_capships += capship_count;
                presence.alliance_fighters += fighter_count;
            } else {
                presence.empire_fleets += 1;
                presence.empire_capships += capship_count;
                presence.empire_fighters += fighter_count;
            }
        }
    }

    // The squadrons on the planet count with the fleets' (FUN_005039d0;
    // FUN_0050b310 counts both).
    for entry in &sys.fighters {
        match crate::squadrons::is_alliance(world, entry.class) {
            Some(true) => presence.alliance_fighters += entry.count,
            Some(false) => presence.empire_fighters += entry.count,
            None => {}
        }
    }

    for &troop_key in &sys.ground_units {
        if let Some(troop) = world.troops.get(troop_key) {
            if troop.is_alliance {
                presence.alliance_troops += 1;
            } else {
                presence.empire_troops += 1;
            }
        }
    }

    presence
}

// ---------------------------------------------------------------------------
// Smuggling percent (FUN_0050b230 -> FUN_00559c40)
// ---------------------------------------------------------------------------

/// `FUN_00559c40(side, support, bit 11, capital ships, squadrons, regiments,
/// Death Stars)`: system `+0x6c` `SmugglingPercent`.
///
/// With no Death Star and the holder's support at most GNPRTB 7732 (40), a
/// base by support band (7737: 75 up to 7736 (20), 7735: 50 up to 7734 (30),
/// else 7733: 25) less 7686 (10) per capital ship, 7687 (5) per squadron and
/// 7688 (2) per regiment, clamped to 0..100 (`FUN_0053e0d0`). The Empire's
/// regiments count GNPRTB 7680 (2) times with `+0x88` bit 11
/// (`FUN_00559b60`). Negative for the Empire; otherwise 0.
#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors the original function's seven parameters."
)]
fn smuggling_formula(
    side: crate::dat::Faction,
    support: i32,
    bit_11: bool,
    capital_ships: i32,
    squadrons: i32,
    regiments: i32,
    death_stars: i32,
    gnprtb: &GnprtbParams,
    difficulty: u8,
) -> i32 {
    let empire = side == crate::dat::Faction::Empire;
    let regiments = if bit_11 && empire {
        regiments * gnprtb.value(GNPRTB_EMPIRE_TROOP_MULT, difficulty)
    } else {
        regiments
    };
    let mut percent = 0;
    if support <= gnprtb.value(GNPRTB_SMUGGLING_SUPPORT_CEILING, difficulty) && death_stars == 0 {
        let base = if support <= gnprtb.value(GNPRTB_SMUGGLING_LOW_BAND, difficulty) {
            gnprtb.value(GNPRTB_SMUGGLING_BASE_LOW, difficulty)
        } else if support <= gnprtb.value(GNPRTB_SMUGGLING_MID_BAND, difficulty) {
            gnprtb.value(GNPRTB_SMUGGLING_BASE_MID, difficulty)
        } else {
            gnprtb.value(GNPRTB_SMUGGLING_BASE_HIGH, difficulty)
        };
        percent = (base
            - capital_ships * gnprtb.value(GNPRTB_SMUGGLING_CAPITAL_SHIP, difficulty)
            - squadrons * gnprtb.value(GNPRTB_SMUGGLING_SQUADRON, difficulty)
            - regiments * gnprtb.value(GNPRTB_SMUGGLING_REGIMENT, difficulty))
        .clamp(0, 100);
    }
    if empire {
        -percent
    } else {
        percent
    }
}

/// `FUN_0050b230`: the system's `SmugglingPercent` for its holder, or 0 for a
/// system no side holds.
///
/// The inputs are the holder's capital ships in its fleets (`FUN_00509090`),
/// its squadrons on the planet (`FUN_00508fb0`, not those aboard fleets), its
/// regiments (`FUN_00509020`), the Death Stars in its fleets
/// (`FUN_00509140`), and its support (`FUN_00507270`).
///
/// It only sets the field: the support never moves with it
/// (`ghidra/notes/economy-cadence.md`).
#[expect(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    reason = "Unit counts at one system are far below i32::MAX."
)]
fn smuggling_percent(
    world: &GameWorld,
    sys: &crate::world::System,
    bit_11: bool,
    gnprtb: &GnprtbParams,
    difficulty: u8,
) -> i32 {
    use crate::dat::Faction;
    let Some(side @ (Faction::Alliance | Faction::Empire)) = sys.control.faction() else {
        return 0;
    };
    let alliance = side == Faction::Alliance;
    let (mut capital_ships, mut death_stars) = (0, 0);
    for fleet in sys.fleets.iter().filter_map(|key| world.fleets.get(*key)) {
        if fleet.is_alliance != alliance {
            continue;
        }
        for ship in fleet.capital_ships.iter().filter(|ship| ship.alive) {
            capital_ships += 1;
            if world
                .capital_ship_classes
                .get(ship.class)
                .is_some_and(crate::world::CapitalShipClass::is_death_star)
            {
                death_stars += 1;
            }
        }
    }
    let squadrons: u32 = sys
        .fighters
        .iter()
        .filter(|entry| crate::squadrons::is_alliance(world, entry.class) == Some(alliance))
        .map(|entry| entry.count)
        .sum();
    let regiments = crate::uprising::regiments(world, sys, side).len();
    smuggling_formula(
        side,
        crate::uprising::support_points(sys, side),
        bit_11,
        capital_ships,
        squadrons as i32,
        regiments as i32,
        death_stars,
        gnprtb,
        difficulty,
    )
}

// ---------------------------------------------------------------------------
// Resource capacity (FUN_0050ad60 + FUN_0050ad80 + FUN_0050b0b0)
// ---------------------------------------------------------------------------

/// Calculate total energy and raw material allocation from facilities and mines.
///
/// Returns (`energy_allocated`, `raw_material_allocated`).
/// The original sums facility outputs (virtual method at +0x1c8) and mine outputs,
/// then caps at system limits. If overcapped, the original randomly prunes facilities/mines.
/// We report overcap via events and let the caller decide on pruning.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn calculate_resource_allocation(world: &GameWorld, sys: &crate::world::System) -> (u32, u32) {
    // Sum production facility outputs (energy generators).
    // Each production facility contributes 1 unit of energy output.
    let energy_output = sys.production_facilities.len() as u32;

    // Sum mine outputs (raw material generators).
    // Mines are production facilities with is_mine = true.
    let raw_material_output = sys
        .production_facilities
        .iter()
        .filter(|k| {
            world
                .production_facilities
                .get(**k)
                .is_some_and(|f| f.is_mine)
        })
        .count() as u32;

    (energy_output, raw_material_output)
}

// ---------------------------------------------------------------------------
// Troop-based side resolution (FUN_0050b610)
// ---------------------------------------------------------------------------

/// Which side holds `sys` (`FUN_0050b610`, word `+0x84`):
/// - the regiments' side (bits 4-5): the Alliance's or the Empire's
///   regiments alone (`FUN_00504cc0` with side 1, 2), the previous side
///   with both, none without any;
/// - the support side (bits 2-3, `FUN_0055a080`): none for an unpopulated
///   system (`+0x88` bit 0 clear, 3); the Alliance when its support
///   (`+0x58`) reaches GNPRTB 7760, the Empire when 100 less it does, else
///   none;
/// - the holder (bits 0-1, `FUN_005588f0`): the regiments' side when there
///   is one, else the support side.
///
/// hyp: the port keeps no regiment bits, so with both sides' regiments the
/// previous side is the side holding the system, if any. port: a side's
/// uprising (`ControlKind::Uprising`) stays while that side holds the
/// system.
#[expect(
    clippy::cast_possible_truncation,
    reason = "GNPRTB values are small integers."
)]
#[must_use]
pub fn resolve_system_control(
    sys: &crate::world::System,
    alliance_troops: bool,
    empire_troops: bool,
    gnprtb: &GnprtbParams,
    difficulty: u8,
) -> ControlKind {
    use crate::dat::Faction;
    let held = sys
        .control
        .faction()
        .filter(|&side| side != Faction::Neutral);
    let regiments = match (alliance_troops, empire_troops) {
        (true, false) => Some(Faction::Alliance),
        (false, true) => Some(Faction::Empire),
        (true, true) => held,
        (false, false) => None,
    };
    let support = if sys.is_populated {
        let threshold = gnprtb.value(GNPRTB_SUPPORT_CONTROL_THRESHOLD, difficulty) as i32;
        let alliance = crate::uprising::support_points(sys, Faction::Alliance);
        if alliance >= threshold {
            Some(Faction::Alliance)
        } else if 100 - alliance >= threshold {
            Some(Faction::Empire)
        } else {
            None
        }
    } else {
        None
    };
    match regiments.or(support) {
        Some(side) if sys.control == ControlKind::Uprising(side) => sys.control,
        Some(side) => ControlKind::Controlled(side),
        None if sys.control == ControlKind::Controlled(Faction::Neutral) => sys.control,
        None => ControlKind::Uncontrolled,
    }
}

/// [`resolve_system_control`] with `sys`'s regiments counted as
/// `FUN_0050b610` counts them: any regiment of a side on the planet.
#[must_use]
pub fn system_holder(world: &GameWorld, sys: &crate::world::System) -> ControlKind {
    let side = |alliance: bool| {
        sys.ground_units.iter().any(|key| {
            world
                .troops
                .get(*key)
                .is_some_and(|troop| troop.is_alliance == alliance)
        })
    };
    resolve_system_control(
        sys,
        side(true),
        side(false),
        &world.gnprtb,
        world.difficulty_index,
    )
}

// ---------------------------------------------------------------------------
// Troop/fleet summary propagation (FUN_0050b500 through FUN_0050b8e0)
// ---------------------------------------------------------------------------

/// Compute the per-system derived summary from troop/fleet/facility presence.
/// Implements functions 9-15 of the economy tick pipeline.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn compute_system_summary(
    world: &GameWorld,
    sys: &crate::world::System,
    presence: &MilitaryPresence,
    garrison_requirement: u32,
) -> SystemSummary {
    let controlling_troops = match sys.control {
        ControlKind::Controlled(crate::dat::Faction::Alliance) => presence.alliance_troops,
        ControlKind::Controlled(crate::dat::Faction::Empire) => presence.empire_troops,
        _ => presence.alliance_troops.max(presence.empire_troops),
    };
    // FUN_0050b500: a held system's surplus is every regiment at it
    // (FUN_00504c40, no side filter) minus the requirement; otherwise 0.
    let troop_surplus = match sys.control {
        ControlKind::Controlled(crate::dat::Faction::Alliance | crate::dat::Faction::Empire) => {
            (presence.alliance_troops + presence.empire_troops).cast_signed()
                - garrison_requirement.cast_signed()
        }
        _ => 0,
    };

    // FUN_0050ba90: total controlling troops
    let total_controlling_troops = controlling_troops;

    // FUN_0050bb70: shipyard presence (check manufacturing facilities for shipyard type).
    let has_shipyard = sys.manufacturing_facilities.iter().any(|k| {
        world
            .manufacturing_facilities
            .get(*k)
            .is_some_and(|f| f.is_shipyard)
    });

    // FUN_0050bc60/FUN_0050be00/FUN_0050c350: fleet posture (3 passes)
    let fleet_posture = FleetPosture {
        alliance_capships: presence.alliance_capships,
        empire_capships: presence.empire_capships,
        is_contested: presence.alliance_fleets >= 2 && presence.empire_fleets >= 2,
    };

    // FUN_0050b8e0: fighter posture
    let fighter_posture = FighterPosture {
        is_contested: presence.alliance_fighters > 0 && presence.empire_fighters > 0,
        alliance_fighters: presence.alliance_fighters,
        empire_fighters: presence.empire_fighters,
    };

    SystemSummary {
        troop_surplus,
        total_controlling_troops,
        has_shipyard,
        fleet_posture,
        fighter_posture,
        death_star_nearby: death_star_nearby(world, sys),
    }
}

/// System `+0x88` bit 11, `DeathStarNearby` (`FUN_0050a890`,
/// `ghidra/generated/setters.tsv`). `FUN_005178b0` sets it on every system
/// of a sector that holds an active Death Star (CAPSHPSD family `0x18`;
/// `FUN_00518690` gives the sector of its system) and clears it elsewhere;
/// `FUN_00514510` and `FUN_00514a60` run it when objects move. It doubles
/// the Empire's regiments in the smuggling percent (`FUN_00559b60`) and the
/// uprising score (`FUN_00559ce0`), halves the Empire's garrison requirement
/// (`FUN_00559fe0`), and halves support shifts that favour the Alliance or
/// hurt the Empire (`FUN_00559be0`).
///
/// port: computed from the fleets standing at the sector's systems when
/// read, which is what the move hooks keep; a fleet in hyperspace stands at
/// no system.
pub(crate) fn death_star_nearby(world: &GameWorld, sys: &crate::world::System) -> bool {
    world
        .systems
        .values()
        .filter(|other| other.sector == sys.sector)
        .flat_map(|other| other.fleets.iter())
        .filter_map(|key| world.fleets.get(*key))
        .flat_map(|fleet| fleet.capital_ships.iter())
        .filter(|ship| ship.alive)
        .any(|ship| {
            world
                .capital_ship_classes
                .get(ship.class)
                .is_some_and(crate::world::CapitalShipClass::is_death_star)
        })
}

// ---------------------------------------------------------------------------
// Incident evaluation
// ---------------------------------------------------------------------------

/// Evaluate the informant and resource incident flags for a system.
///
/// Neither trigger has a recovered source (audit finding F-029): the original
/// raises the informant incident from a per-system timer (event `0x38e`,
/// `FUN_0050cbe0`) and the resource incident from a galaxy timer (event
/// `0x390`, `FUN_00556be0`). The uprising and disaster incidents are raised by
/// `UprisingSystem`, so their flags stay clear.
fn evaluate_incident_flags(summary: &SystemSummary, eco: &SystemEconomy) -> IncidentFlags {
    IncidentFlags {
        uprising: false,
        informant: summary.troop_surplus < 0,
        disaster: false,
        resource: eco.energy_overcapped || eco.raw_material_overcapped,
    }
}

/// `FUN_0050b2c0` -> `FUN_00559c10`: system `+0x70` `ProductionModifier`,
/// the port's "collection rate". With no holder (side bits 0 or 3) it is
/// 100; else GNPRTB 7763 (100) times 100 (`DAT_00661a88`) over the holder's
/// support (`FUN_00507270`), 0 counting as 1, in integers (`FUN_0053e170`,
/// `FUN_0053e150`). `FUN_0050c2b0` copies it to the system's mines and
/// refineries, whose cycles it lengthens (`production_cycle.rs`).
#[must_use]
pub fn production_modifier(
    holder_support: Option<i32>,
    gnprtb: &GnprtbParams,
    difficulty: u8,
) -> i32 {
    let Some(support) = holder_support else {
        return FULL_PRODUCTION_MODIFIER;
    };
    let support = if support == 0 { 1 } else { support };
    gnprtb.value(GNPRTB_COLLECTION_RATE_BASE, difficulty) * 100 / support
}

/// The `ProductionModifier` (`+0x70`) the economy last stored for `system`,
/// or 100 before it has run there.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "The stored value is a whole number held as f32 for the save layout."
)]
pub fn production_modifier_at(economy: &EconomyState, system: SystemKey) -> i32 {
    economy
        .per_system
        .get(&system)
        .map_or(FULL_PRODUCTION_MODIFIER, |eco| eco.collection_rate as i32)
}

// ---------------------------------------------------------------------------
// Garrison requirement (FUN_00559fe0 + FUN_0055a050)
// ---------------------------------------------------------------------------

/// Calculate troops needed to prevent uprising at this system.
///
/// Formula: `(threshold - support_pct) / abs(divisor)` when support < threshold.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn calculate_garrison_requirement(
    support: f32,
    control: ControlKind,
    death_star_nearby: bool,
    in_uprising: bool,
    gnprtb: &GnprtbParams,
    difficulty: u8,
) -> u32 {
    // FUN_0050b5a0 stores FUN_00559fe0(side, support, +0x88 bit 11,
    // +0x88 bit 2, 1) for an Alliance- or Empire-held system and 0 otherwise.
    let Some(side) = control.faction() else {
        return 0;
    };
    // Our support is f32 0.0-1.0; convert to integer 0-100.
    let support_int = (support * 100.0).round() as i32;
    garrison_requirement(
        side,
        support_int,
        death_star_nearby,
        in_uprising,
        gnprtb,
        difficulty,
    )
    .max(0) as u32
}

/// Troops a side must hold at a system (`FUN_00559fe0`): the uprising
/// threshold, divided by GNPRTB 7680 for the Empire with a Death Star in the
/// sector (+0x88 bit 11), and multiplied by GNPRTB 7682 during an uprising.
pub(crate) fn garrison_requirement(
    side: crate::dat::Faction,
    support: i32,
    death_star_nearby: bool,
    in_uprising: bool,
    gnprtb: &GnprtbParams,
    difficulty: u8,
) -> i32 {
    let mut requirement = uprising_threshold(support, gnprtb, difficulty);
    if death_star_nearby && side == crate::dat::Faction::Empire {
        requirement /= gnprtb.value(GNPRTB_EMPIRE_TROOP_MULT, difficulty).max(1);
    }
    if in_uprising {
        requirement *= gnprtb.value(GNPRTB_UPRISING_GARRISON_MULT, difficulty);
    }
    requirement
}

/// `FUN_0055a050`: `ceil((GNPRTB 7761 - support) / -GNPRTB 7762)` below the
/// GNPRTB 7761 threshold (60 and -10 shipped), else 0. `FUN_0053e160(a, b)`
/// is `(b - 1 + a) / b`.
pub(crate) fn uprising_threshold(support: i32, gnprtb: &GnprtbParams, difficulty: u8) -> i32 {
    let threshold = gnprtb.value(GNPRTB_GARRISON_THRESHOLD, difficulty);
    if support >= threshold {
        return 0;
    }
    let divisor = (-gnprtb.value(GNPRTB_GARRISON_DIVISOR, difficulty)).max(1);
    (divisor - 1 + threshold - support) / divisor
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DatId;
    use crate::ids::SectorKey;
    use crate::world::GameWorld;

    /// Create a `GnprtbParams` with the stock economy values.
    #[expect(
        clippy::too_many_lines,
        reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
    )]
    fn stock_gnprtb() -> GnprtbParams {
        use crate::world::GnprtbEntry;
        let entries = vec![
            GnprtbEntry {
                parameter_id: 7686,
                development: 10,
                alliance_sp_easy: 10,
                alliance_sp_medium: 10,
                alliance_sp_hard: 10,
                empire_sp_easy: 10,
                empire_sp_medium: 10,
                empire_sp_hard: 10,
                multiplayer: 10,
            },
            GnprtbEntry {
                parameter_id: 7687,
                development: 5,
                alliance_sp_easy: 5,
                alliance_sp_medium: 5,
                alliance_sp_hard: 5,
                empire_sp_easy: 5,
                empire_sp_medium: 5,
                empire_sp_hard: 5,
                multiplayer: 5,
            },
            GnprtbEntry {
                parameter_id: 7688,
                development: 2,
                alliance_sp_easy: 2,
                alliance_sp_medium: 2,
                alliance_sp_hard: 2,
                empire_sp_easy: 2,
                empire_sp_medium: 2,
                empire_sp_hard: 2,
                multiplayer: 2,
            },
            GnprtbEntry {
                parameter_id: 7732,
                development: 40,
                alliance_sp_easy: 40,
                alliance_sp_medium: 40,
                alliance_sp_hard: 40,
                empire_sp_easy: 40,
                empire_sp_medium: 40,
                empire_sp_hard: 40,
                multiplayer: 40,
            },
            GnprtbEntry {
                parameter_id: 7733,
                development: 25,
                alliance_sp_easy: 25,
                alliance_sp_medium: 25,
                alliance_sp_hard: 25,
                empire_sp_easy: 25,
                empire_sp_medium: 25,
                empire_sp_hard: 25,
                multiplayer: 25,
            },
            GnprtbEntry {
                parameter_id: 7734,
                development: 30,
                alliance_sp_easy: 30,
                alliance_sp_medium: 30,
                alliance_sp_hard: 30,
                empire_sp_easy: 30,
                empire_sp_medium: 30,
                empire_sp_hard: 30,
                multiplayer: 30,
            },
            GnprtbEntry {
                parameter_id: 7735,
                development: 50,
                alliance_sp_easy: 50,
                alliance_sp_medium: 50,
                alliance_sp_hard: 50,
                empire_sp_easy: 50,
                empire_sp_medium: 50,
                empire_sp_hard: 50,
                multiplayer: 50,
            },
            GnprtbEntry {
                parameter_id: 7736,
                development: 20,
                alliance_sp_easy: 20,
                alliance_sp_medium: 20,
                alliance_sp_hard: 20,
                empire_sp_easy: 20,
                empire_sp_medium: 20,
                empire_sp_hard: 20,
                multiplayer: 20,
            },
            GnprtbEntry {
                parameter_id: 7737,
                development: 75,
                alliance_sp_easy: 75,
                alliance_sp_medium: 75,
                alliance_sp_hard: 75,
                empire_sp_easy: 75,
                empire_sp_medium: 75,
                empire_sp_hard: 75,
                multiplayer: 75,
            },
            GnprtbEntry {
                parameter_id: 7761,
                development: 60,
                alliance_sp_easy: 60,
                alliance_sp_medium: 60,
                alliance_sp_hard: 60,
                empire_sp_easy: 60,
                empire_sp_medium: 60,
                empire_sp_hard: 60,
                multiplayer: 60,
            },
            GnprtbEntry {
                parameter_id: 7762,
                development: -10,
                alliance_sp_easy: -10,
                alliance_sp_medium: -10,
                alliance_sp_hard: -10,
                empire_sp_easy: -10,
                empire_sp_medium: -10,
                empire_sp_hard: -10,
                multiplayer: -10,
            },
            GnprtbEntry {
                parameter_id: 7763,
                development: 100,
                alliance_sp_easy: 100,
                alliance_sp_medium: 100,
                alliance_sp_hard: 100,
                empire_sp_easy: 100,
                empire_sp_medium: 100,
                empire_sp_hard: 100,
                multiplayer: 100,
            },
            // Phase 3b additions: all economy tick GNPRTB indices (values from GNPRTB.DAT)
            GnprtbEntry {
                parameter_id: 7680,
                development: 2,
                alliance_sp_easy: 2,
                alliance_sp_medium: 2,
                alliance_sp_hard: 2,
                empire_sp_easy: 2,
                empire_sp_medium: 2,
                empire_sp_hard: 2,
                multiplayer: 2,
            }, // Empire troop doubling + garrison halving
            GnprtbEntry {
                parameter_id: 7682,
                development: 2,
                alliance_sp_easy: 2,
                alliance_sp_medium: 2,
                alliance_sp_hard: 2,
                empire_sp_easy: 2,
                empire_sp_medium: 2,
                empire_sp_hard: 2,
                multiplayer: 2,
            }, // Uprising garrison doubler
            GnprtbEntry {
                parameter_id: 7684,
                development: 5,
                alliance_sp_easy: 5,
                alliance_sp_medium: 5,
                alliance_sp_hard: 5,
                empire_sp_easy: 5,
                empire_sp_medium: 5,
                empire_sp_hard: 5,
                multiplayer: 5,
            }, // KDY capship penalty per unit
            GnprtbEntry {
                parameter_id: 7685,
                development: 2,
                alliance_sp_easy: 2,
                alliance_sp_medium: 2,
                alliance_sp_hard: 2,
                empire_sp_easy: 2,
                empire_sp_medium: 2,
                empire_sp_hard: 2,
                multiplayer: 2,
            }, // KDY fighter penalty per unit
            GnprtbEntry {
                parameter_id: 7691,
                development: 0,
                alliance_sp_easy: 0,
                alliance_sp_medium: 0,
                alliance_sp_hard: 0,
                empire_sp_easy: 0,
                empire_sp_medium: 0,
                empire_sp_hard: 0,
                multiplayer: 0,
            }, // Support delta Empire-controlled transition
            GnprtbEntry {
                parameter_id: 7693,
                development: 1,
                alliance_sp_easy: 1,
                alliance_sp_medium: 1,
                alliance_sp_hard: 1,
                empire_sp_easy: 1,
                empire_sp_medium: 1,
                empire_sp_hard: 1,
                multiplayer: 1,
            }, // Support delta favors controller
            GnprtbEntry {
                parameter_id: 7694,
                development: 30,
                alliance_sp_easy: 30,
                alliance_sp_medium: 30,
                alliance_sp_hard: 30,
                empire_sp_easy: 30,
                empire_sp_medium: 30,
                empire_sp_hard: 30,
                multiplayer: 30,
            }, // Maintenance check rate (controlled)
            GnprtbEntry {
                parameter_id: 7695,
                development: -1,
                alliance_sp_easy: -1,
                alliance_sp_medium: -1,
                alliance_sp_hard: -1,
                empire_sp_easy: -1,
                empire_sp_medium: -1,
                empire_sp_hard: -1,
                multiplayer: -1,
            }, // Support delta opposes controller
            GnprtbEntry {
                parameter_id: 7696,
                development: 30,
                alliance_sp_easy: 30,
                alliance_sp_medium: 30,
                alliance_sp_hard: 30,
                empire_sp_easy: 30,
                empire_sp_medium: 30,
                empire_sp_hard: 30,
                multiplayer: 30,
            }, // Maintenance check rate (neutral)
            GnprtbEntry {
                parameter_id: 7697,
                development: -1,
                alliance_sp_easy: -1,
                alliance_sp_medium: -1,
                alliance_sp_hard: -1,
                empire_sp_easy: -1,
                empire_sp_medium: -1,
                empire_sp_hard: -1,
                multiplayer: -1,
            }, // Support delta controlled systems
            GnprtbEntry {
                parameter_id: 7760,
                development: 60,
                alliance_sp_easy: 60,
                alliance_sp_medium: 60,
                alliance_sp_hard: 60,
                empire_sp_easy: 60,
                empire_sp_medium: 60,
                empire_sp_hard: 60,
                multiplayer: 60,
            }, // Energy-based control threshold
        ];
        GnprtbParams::new(entries)
    }

    #[test]
    fn a_death_star_sets_bit_11_on_every_system_of_its_sector_only() {
        // FUN_005178b0: each sector's systems get +0x88 bit 11
        // (DeathStarNearby, FUN_0050a890) when an active Death Star
        // (CAPSHPSD family 0x18) stands at one of its systems
        // (FUN_00518690); support plays no part.
        use crate::dat::Faction::{Alliance, Empire};
        use crate::world::{CapitalShipClass, Fleet, Sector, ShipInstance};
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector = |world: &mut GameWorld| {
            world.sectors.insert(Sector {
                dat_id: DatId::new(0x9200_0000),
                name: "Sector".into(),
                group: crate::dat::SectorGroup::Core,
                x: 0,
                y: 0,
                systems: vec![],
            })
        };
        let (home, other) = (sector(&mut world), sector(&mut world));
        let mut system = |sector, support| {
            let mut sys = held_system(support, ControlKind::Controlled(Empire));
            sys.sector = sector;
            world.systems.insert(sys)
        };
        let (station, neighbour, far) =
            (system(home, 90.0), system(home, 10.0), system(other, 10.0));
        let nearby = |world: &GameWorld| {
            [station, neighbour, far].map(|key| death_star_nearby(world, &world.systems[key]))
        };
        assert_eq!(nearby(&world), [false; 3]);

        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(crate::world::DEATH_STAR_CLASS_ID),
            ..CapitalShipClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: station,
            capital_ships: vec![ShipInstance::new(class, 100, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: true,
        });
        world.systems[station].fleets.push(fleet);
        assert_eq!(nearby(&world), [true, true, false]);

        world.fleets[fleet].is_alliance = true;
        world.systems[station].control = ControlKind::Controlled(Alliance);
        assert_eq!(nearby(&world), [true, true, false]);

        world.fleets[fleet].capital_ships[0].alive = false;
        assert_eq!(nearby(&world), [false; 3]);
    }

    #[test]
    fn smuggling_takes_the_support_band_s_base_less_the_weighted_forces() {
        // FUN_00559c40 with GNPRTB 7732..7737 and 7686..7688 (stock values).
        use crate::dat::Faction::Alliance;
        let g = stock_gnprtb();
        let f = |support, ships, squadrons, regiments| {
            smuggling_formula(
                Alliance, support, false, ships, squadrons, regiments, 0, &g, 2,
            )
        };
        assert_eq!(f(20, 0, 0, 0), 75);
        assert_eq!(f(21, 0, 0, 0), 50);
        assert_eq!(f(30, 0, 0, 0), 50);
        assert_eq!(f(31, 0, 0, 0), 25);
        assert_eq!(f(40, 0, 0, 0), 25);
        assert_eq!(f(41, 0, 0, 0), 0);
        // 75 - 1 * 10 - 2 * 5 - 3 * 2.
        assert_eq!(f(10, 1, 2, 3), 49);
        assert_eq!(f(40, 3, 0, 0), 0);
    }

    #[test]
    fn the_empire_smuggles_negative_and_counts_its_regiments_twice_with_bit_11() {
        // FUN_00559c40 negates for side 2; FUN_00559b60 multiplies side 2's
        // regiments by GNPRTB 7680 (2) when +0x88 bit 11 is set.
        use crate::dat::Faction::{Alliance, Empire};
        let g = stock_gnprtb();
        let f = |side, bit_11| smuggling_formula(side, 10, bit_11, 0, 0, 3, 0, &g, 2);
        // 75 - 3 * 2, and 75 - 3 * 2 * 2.
        assert_eq!(f(Empire, false), -69);
        assert_eq!(f(Empire, true), -63);
        assert_eq!(f(Alliance, true), 69);
    }

    #[test]
    fn a_death_star_in_the_holder_s_fleets_stops_smuggling() {
        // FUN_00559c40: the base applies only when FUN_00509140 counts none.
        use crate::dat::Faction::Alliance;
        let g = stock_gnprtb();
        assert_eq!(smuggling_formula(Alliance, 10, false, 1, 0, 0, 1, &g, 2), 0);
    }

    /// An Alliance system at 10 support with one fleet of one capital ship
    /// carrying 4 squadrons, 1 squadron on the planet, and 1 regiment.
    fn smuggling_world(death_star: bool) -> (GameWorld, SystemKey) {
        use crate::world::{CapitalShipClass, FighterClass, FighterEntry, Fleet, ShipInstance};
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sys = world.systems.insert(held_system(
            10.0,
            ControlKind::Controlled(crate::dat::Faction::Alliance),
        ));
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: if death_star {
                DatId::new(crate::world::DEATH_STAR_CLASS_ID)
            } else {
                DatId::new(0)
            },
            ..CapitalShipClass::default()
        });
        let squadron = world.fighter_classes.insert(FighterClass {
            is_alliance: true,
            ..FighterClass::default()
        });
        let fleet = world.fleets.insert(Fleet {
            location: sys,
            capital_ships: vec![ShipInstance::new(class, 100, false)],
            fighters: vec![FighterEntry {
                class: squadron,
                count: 4,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: death_star,
        });
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId(0),
            is_alliance: true,
            regiment_strength: 100,
        });
        let system = &mut world.systems[sys];
        system.fleets.push(fleet);
        system.ground_units.push(troop);
        system.fighters.push(FighterEntry {
            class: squadron,
            count: 1,
        });
        (world, sys)
    }

    #[test]
    fn smuggling_counts_the_holder_s_ships_planet_squadrons_and_regiments() {
        // FUN_0050b230: capital ships in fleets (FUN_00509090), squadrons on
        // the planet only (FUN_00508fb0 -> FUN_00503a50), regiments
        // (FUN_00509020), Death Stars (FUN_00509140). 75 - 10 - 5 - 2.
        let (world, sys) = smuggling_world(false);
        let s = &world.systems[sys];
        assert_eq!(smuggling_percent(&world, s, false, &world.gnprtb, 2), 58);
        let (world, sys) = smuggling_world(true);
        let s = &world.systems[sys];
        assert_eq!(smuggling_percent(&world, s, false, &world.gnprtb, 2), 0);
    }

    #[test]
    fn a_system_no_side_holds_smuggles_nothing() {
        // FUN_0050b230 passes 0 when the side bits name no side.
        let (mut world, sys) = smuggling_world(false);
        world.systems[sys].control = ControlKind::Uncontrolled;
        let s = &world.systems[sys];
        assert_eq!(smuggling_percent(&world, s, false, &world.gnprtb, 2), 0);
    }

    #[test]
    fn the_economy_stores_the_smuggling_percent_and_leaves_the_support_alone() {
        // FUN_0050b230 stores +0x6c through FUN_00509ec0, which notifies
        // only on a change; nothing in it moves the support
        // (ghidra/notes/economy-cadence.md).
        let (world, sys) = smuggling_world(false);
        let mut state = EconomyState::default();
        let changes = |events: Vec<EconomyEvent>| {
            events
                .into_iter()
                .filter_map(|event| match event {
                    EconomyEvent::SmugglingPercentChanged { system, percent } => {
                        Some((system, percent))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let first = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);
        assert_eq!(changes(first), vec![(sys, 58)]);
        assert_eq!(state.per_system[&sys].smuggling_percent, 58);
        let second = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 2 }], 2);
        assert_eq!(changes(second), vec![]);
    }

    #[test]
    fn the_smuggling_percent_waits_for_the_support_bit_11_or_the_side_to_change() {
        // Only the support (FUN_005109f0), bit 11 (FUN_00511740) and side
        // (FUN_00510820) hooks run FUN_0050b230; a regiment joining runs
        // FUN_00508660, which does not.
        let (mut world, sys) = smuggling_world(false);
        let mut state = EconomyState::default();
        let mut day = 0;
        let mut run = |world: &GameWorld, state: &mut EconomyState| {
            day += 1;
            EconomySystem::advance(state, world, &[TickEvent { tick: day }], 2);
            state.per_system[&sys].smuggling_percent
        };
        assert_eq!(run(&world, &mut state), 58);
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId(0),
            is_alliance: true,
            regiment_strength: 100,
        });
        world.systems[sys].ground_units.push(troop);
        assert_eq!(run(&world, &mut state), 58);
        // One support point more: 75 - 10 - 5 - 2 * 2.
        world.systems[sys].popularity_alliance = 0.11;
        world.systems[sys].popularity_empire = 0.89;
        assert_eq!(run(&world, &mut state), 56);
        world.systems[sys].ground_units.pop();
        assert_eq!(run(&world, &mut state), 56);
        // The side hook: the Empire holds it at 89 support, above the band.
        world.systems[sys].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        assert_eq!(run(&world, &mut state), 0);
    }

    #[test]
    fn the_collection_rate_waits_for_the_support_to_change() {
        // ProductionModifier (FUN_0050b2c0) is recomputed by the support
        // hook FUN_005109f0 only.
        let (mut world, sys) = smuggling_world(false);
        let mut state = EconomyState::default();
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);
        let first = state.per_system[&sys].collection_rate;
        assert!((first - 1000.0).abs() < f32::EPSILON, "{first}");
        world.systems[sys].control = ControlKind::Controlled(crate::dat::Faction::Empire);
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 2 }], 2);
        assert!((state.per_system[&sys].collection_rate - first).abs() < f32::EPSILON);
        world.systems[sys].popularity_alliance = 0.2;
        world.systems[sys].popularity_empire = 0.8;
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 3 }], 2);
        assert!((state.per_system[&sys].collection_rate - 125.0).abs() < f32::EPSILON);
    }

    #[test]
    fn collection_rate_high_support() {
        // FUN_00559c10: GNPRTB 7763 (100) * 100 / 80.
        let gnprtb = stock_gnprtb();
        assert_eq!(production_modifier(Some(80), &gnprtb, 2), 125);
        // Integer division: 10000 / 70.
        assert_eq!(production_modifier(Some(70), &gnprtb, 2), 142);
    }

    #[test]
    fn a_system_without_a_holder_has_a_production_modifier_of_100() {
        // FUN_0050b2c0 stores 100 when the side bits name no side.
        let gnprtb = stock_gnprtb();
        assert_eq!(production_modifier(None, &gnprtb, 2), 100);
        let (mut world, sys) = smuggling_world(false);
        world.systems[sys].control = ControlKind::Uncontrolled;
        let mut state = EconomyState::default();
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);
        assert_eq!(production_modifier_at(&state, sys), 100);
        // A side in revolt still holds the system: 10000 / 10.
        world.systems[sys].control = ControlKind::Uprising(crate::dat::Faction::Alliance);
        let mut state = EconomyState::default();
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);
        assert_eq!(production_modifier_at(&state, sys), 1000);
        assert_eq!(production_modifier_at(&EconomyState::default(), sys), 100);
    }

    #[test]
    fn collection_rate_low_support() {
        let gnprtb = stock_gnprtb();
        // FUN_00559c10: (100 * 100) / 10.
        assert_eq!(production_modifier(Some(10), &gnprtb, 2), 1000);
    }

    #[test]
    fn garrison_requirement_above_threshold_is_zero() {
        let gnprtb = stock_gnprtb();
        let control = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let req = calculate_garrison_requirement(0.7, control, false, false, &gnprtb, 2);
        assert_eq!(req, 0, "above threshold should need 0 garrison");
    }

    #[test]
    fn empire_garrison_is_halved_only_with_a_death_star_nearby() {
        // FUN_00559fe0 divides by GNPRTB 7680 only when param_3 (system
        // +0x88 bit 11) is set and the side is 2.
        let gnprtb = stock_gnprtb();
        let alliance = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let empire = ControlKind::Controlled(crate::dat::Faction::Empire);
        let req = |control, nearby| {
            calculate_garrison_requirement(0.3, control, nearby, false, &gnprtb, 2)
        };
        assert_eq!(req(alliance, false), 3);
        assert_eq!(req(alliance, true), 3);
        assert_eq!(req(empire, false), 3);
        assert_eq!(req(empire, true), 1);
        // Support 0: ceil(60 / 10) = 6, halved to 3.
        assert_eq!(
            calculate_garrison_requirement(0.0, empire, true, false, &gnprtb, 2),
            3
        );
    }

    #[test]
    fn a_revolt_is_visible_only_while_a_populated_held_system_keeps_its_garrison() {
        // FUN_0050bb00 sets system +0x88 bit 9 when the system is populated,
        // held by side 1 or 2, in revolt (bit 2), and its troop surplus
        // (+0x7c) is not negative.
        let alliance = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let visible = |support: f32, control, populated: bool, in_revolt: bool| {
            let (world, sys) = make_singleton_world(support, 1.0 - support, control, populated);
            let mut uprisings = crate::uprising::UprisingState::default();
            if in_revolt {
                uprisings.active_uprisings.insert(
                    sys,
                    crate::uprising::ActiveUprising {
                        started_tick: 0,
                        next_incident_tick: None,
                        next_support_tick: None,
                        side: None,
                    },
                );
            }
            let mut state = EconomyState::default();
            EconomySystem::advance_with_uprisings(
                &mut state,
                &world,
                &[TickEvent { tick: 1 }],
                2,
                &uprisings,
            );
            state
                .per_system
                .get(&sys)
                .is_some_and(|e| e.uprising_visible)
        };
        // Support 90 needs no garrison, so the surplus is 0.
        assert!(visible(0.9, alliance, true, true));
        assert!(!visible(0.9, alliance, true, false));
        assert!(!visible(0.9, alliance, false, true));
        assert!(!visible(0.9, ControlKind::Uncontrolled, true, true));
        // Support 30 in revolt needs 3 * 2 regiments and has none.
        assert!(!visible(0.3, alliance, true, true));
    }

    #[test]
    fn a_system_no_side_holds_needs_no_garrison() {
        // FUN_0050b5a0 stores 0 unless the side is 1 or 2.
        let gnprtb = stock_gnprtb();
        for control in [ControlKind::Uncontrolled, ControlKind::Contested] {
            assert_eq!(
                calculate_garrison_requirement(0.1, control, false, true, &gnprtb, 2),
                0
            );
        }
    }

    #[test]
    fn economy_advance_no_ticks_no_events() {
        // The same world emits a SupportChanged on its first tick.
        let (world, _sys) = make_singleton_world(
            0.10,
            0.90,
            ControlKind::Controlled(crate::dat::Faction::Alliance),
            true,
        );
        let mut state = EconomyState::default();
        let events = EconomySystem::advance(&mut state, &world, &[], 2);
        assert!(events.is_empty());
    }

    #[test]
    fn economy_advance_skips_destroyed_systems() {
        let mut state = EconomyState::default();
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let _sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Destroyed".into(),
            sector: SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 5,
            raw_materials: 5,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: true, // Destroyed!
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });
        let tick_events = vec![TickEvent { tick: 1 }];
        let events = EconomySystem::advance(&mut state, &world, &tick_events, 2);
        // Destroyed system should be skipped entirely.
        assert!(
            events.is_empty(),
            "destroyed system should produce no events"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 3b: Resource capacity enforcement tests
    // -----------------------------------------------------------------------

    #[test]
    fn kdy_production_modifier_reduces_with_ships() {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        // System with fleet presence (capships reduce production)
        let fleet_key = world.fleets.insert(crate::world::Fleet {
            location: crate::ids::SystemKey::default(),
            capital_ships: crate::world::ShipInstance::make(
                crate::ids::CapitalShipKey::default(),
                100,
                true,
                3,
            ),
            fighters: vec![crate::world::FighterEntry {
                class: crate::ids::FighterKey::default(),
                count: 4,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "KDY".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.7,
            popularity_empire: 0.3,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
            espionage_rating: 0.0,
            fleets: vec![fleet_key],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut state = EconomyState::default();
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);

        let eco = state.per_system.get(&sys_key).unwrap();
        // 3 capships * 5 (capship penalty) + 4 fighters * 2 (fighter penalty) = 15 + 8 = 23
        // production_modifier = max(100 - 23, 0) = 77
        assert_eq!(
            eco.production_modifier, 77,
            "production_modifier should be 100 - 3*5 - 4*2 = 77, got {}",
            eco.production_modifier
        );
    }

    #[test]
    fn kdy_production_modifier_floors_at_zero() {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        // System with massive fleet presence
        let fleet_key = world.fleets.insert(crate::world::Fleet {
            location: crate::ids::SystemKey::default(),
            capital_ships: crate::world::ShipInstance::make(
                crate::ids::CapitalShipKey::default(),
                100,
                true,
                10,
            ),
            fighters: vec![crate::world::FighterEntry {
                class: crate::ids::FighterKey::default(),
                count: 30,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Overcrowded".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.7,
            popularity_empire: 0.3,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
            espionage_rating: 0.0,
            fleets: vec![fleet_key],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut state = EconomyState::default();
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);

        let eco = state.per_system.get(&sys_key).unwrap();
        // 10 capships * 5 + 30 fighters * 2 = 50 + 60 = 110 → max(100 - 110, 0) = 0
        assert_eq!(
            eco.production_modifier, 0,
            "production modifier should floor at 0, got {}",
            eco.production_modifier
        );
    }

    #[test]
    fn energy_overcap_emits_event() {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        // System with total_energy=2 but 5 production facilities
        let sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Overcap".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 2,
            raw_materials: 10,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            fighters: Vec::new(),
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x1800_0001),
                        side: crate::dat::Faction::Alliance,
                        is_mine: false,
                    }),
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x1800_0002),
                        side: crate::dat::Faction::Alliance,
                        is_mine: false,
                    }),
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x1800_0003),
                        side: crate::dat::Faction::Alliance,
                        is_mine: false,
                    }),
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x1800_0004),
                        side: crate::dat::Faction::Alliance,
                        is_mine: false,
                    }),
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x1800_0005),
                        side: crate::dat::Faction::Alliance,
                        is_mine: false,
                    }),
            ],
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut state = EconomyState::default();
        let events = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);

        // Should emit EnergyOvercapped (5 facilities > 2 capacity)
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EconomyEvent::EnergyOvercapped { .. })),
            "Expected EnergyOvercapped event"
        );

        // energy_allocated should be capped at capacity
        let eco = state.per_system.get(&sys_key).unwrap();
        assert_eq!(
            eco.energy_allocated, 2,
            "energy should be capped at system capacity"
        );
        assert!(eco.energy_overcapped, "should flag overcap");
    }

    #[test]
    fn no_overcap_when_within_limits() {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        // System with total_energy=10 and only 3 production facilities
        world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Normal".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            fighters: Vec::new(),
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x1800_0001),
                        side: crate::dat::Faction::Alliance,
                        is_mine: false,
                    }),
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x1800_0002),
                        side: crate::dat::Faction::Alliance,
                        is_mine: false,
                    }),
            ],
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut state = EconomyState::default();
        let events = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);

        // Should NOT emit overcap events
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EconomyEvent::EnergyOvercapped { .. })),
            "Should not overcap with 2 facilities and capacity 10"
        );
    }

    #[test]
    fn raw_material_overcap_emits_event() {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        // System with raw_materials=1 but 3 manufacturing facilities
        world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "MineCap".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 10,
            raw_materials: 1,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![],
            fighters: Vec::new(),
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![
                world.manufacturing_facilities.insert(
                    crate::world::ManufacturingFacilityInstance {
                        class_dat_id: DatId(0x1600_0001),
                        side: crate::dat::Faction::Alliance,
                        is_shipyard: false,
                    },
                ),
                world.manufacturing_facilities.insert(
                    crate::world::ManufacturingFacilityInstance {
                        class_dat_id: DatId(0x1600_0002),
                        side: crate::dat::Faction::Alliance,
                        is_shipyard: false,
                    },
                ),
                world.manufacturing_facilities.insert(
                    crate::world::ManufacturingFacilityInstance {
                        class_dat_id: DatId(0x1600_0003),
                        side: crate::dat::Faction::Alliance,
                        is_shipyard: false,
                    },
                ),
            ],
            production_facilities: vec![
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x2D00_0001),
                        side: crate::dat::Faction::Alliance,
                        is_mine: true,
                    }),
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x2D00_0002),
                        side: crate::dat::Faction::Alliance,
                        is_mine: true,
                    }),
                world
                    .production_facilities
                    .insert(crate::world::ProductionFacilityInstance {
                        class_dat_id: DatId(0x2D00_0003),
                        side: crate::dat::Faction::Alliance,
                        is_mine: true,
                    }),
            ],
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut state = EconomyState::default();
        let events = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EconomyEvent::RawMaterialOvercapped { .. })),
            "Expected RawMaterialOvercapped event"
        );
    }

    #[test]
    fn the_fighter_posture_counts_the_squadrons_on_the_planet() {
        // FUN_0050b310 counts the squadrons aboard the fleets and at the
        // system (FUN_005039d0); the posture follows that count.
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Yavin".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
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
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });
        for (alliance, count) in [(true, 2), (false, 3)] {
            let class = world.fighter_classes.insert(crate::world::FighterClass {
                is_alliance: alliance,
                is_empire: !alliance,
                ..Default::default()
            });
            crate::squadrons::add(&mut world.systems[sys_key].fighters, class, count);
        }

        let mut state = EconomyState::default();
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);

        let posture = &state.per_system[&sys_key].summary.fighter_posture;
        assert_eq!((posture.alliance_fighters, posture.empire_fighters), (2, 3));
        assert!(posture.is_contested);
    }

    // -----------------------------------------------------------------------
    // Phase 3b: Troop-based side resolution tests
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 3b: Incident state tests
    // -----------------------------------------------------------------------

    #[test]
    fn informant_incident_on_garrison_shortfall() {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        // System with low support (garrison > 0) but no troops (deficit)
        world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Undermanned".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.4,
            popularity_empire: 0.6,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
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
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut state = EconomyState::default();
        let events = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);
        assert!(
            events.iter().any(|e| matches!(
                e,
                EconomyEvent::IncidentTriggered {
                    incident_type: "informant",
                    ..
                }
            )),
            "Should fire informant incident on garrison shortfall"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 3b: System summary tests
    // -----------------------------------------------------------------------

    #[test]
    fn system_summary_troop_surplus() {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        // System with low support (triggers garrison requirement) and some troops
        let troop1 = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId(0x1400_0100),
            is_alliance: true,
            regiment_strength: 100,
        });
        let troop2 = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId(0x1400_0100),
            is_alliance: true,
            regiment_strength: 100,
        });
        let sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Test".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.3,
            popularity_empire: 0.7,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units: vec![troop1, troop2],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut state = EconomyState::default();
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);

        let eco = state.per_system.get(&sys_key).unwrap();
        // Garrison = ceil(30/10) = 3, troops = 2, surplus = 2 - 3 = -1
        assert_eq!(
            eco.summary.troop_surplus, -1,
            "troop surplus should be troops - garrison = 2 - 3 = -1, got {}",
            eco.summary.troop_surplus
        );
        assert_eq!(eco.summary.total_controlling_troops, 2);
    }

    #[test]
    fn the_troop_surplus_counts_every_regiment_and_is_zero_for_an_unheld_system() {
        // FUN_0050b500: for a system held by side 1 or 2, FUN_00504c40 (no side
        // filter) counts all regiments; any other system keeps surplus 0.
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let ground_units = [true, true, false]
            .map(|is_alliance| {
                world.troops.insert(crate::world::TroopUnit {
                    class_dat_id: DatId(0x1400_0100),
                    is_alliance,
                    regiment_strength: 100,
                })
            })
            .to_vec();
        let sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Test".into(),
            sector,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.3,
            popularity_empire: 0.7,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
            espionage_rating: 0.0,
            fleets: vec![],
            ground_units,
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });
        let surplus = |world: &GameWorld| {
            let mut state = EconomyState::default();
            EconomySystem::advance(&mut state, world, &[TickEvent { tick: 1 }], 2);
            state.per_system[&sys_key].summary.troop_surplus
        };
        // Garrison ceil(30 / 10) = 3, three regiments of either side: 0.
        assert_eq!(surplus(&world), 0);
        world.systems[sys_key].control = ControlKind::Uncontrolled;
        assert_eq!(surplus(&world), 0);
        world.systems[sys_key].ground_units.clear();
        assert_eq!(surplus(&world), 0);
    }

    #[test]
    fn system_summary_fleet_posture() {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let fleet1 = world.fleets.insert(crate::world::Fleet {
            location: crate::ids::SystemKey::default(),
            capital_ships: crate::world::ShipInstance::make(
                crate::ids::CapitalShipKey::default(),
                100,
                true,
                2,
            ),
            fighters: vec![crate::world::FighterEntry {
                class: crate::ids::FighterKey::default(),
                count: 5,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
        });
        let fleet2 = world.fleets.insert(crate::world::Fleet {
            location: crate::ids::SystemKey::default(),
            capital_ships: crate::world::ShipInstance::make(
                crate::ids::CapitalShipKey::default(),
                100,
                false,
                3,
            ),
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
        });
        let sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Contested".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
            espionage_rating: 0.0,
            fleets: vec![fleet1, fleet2],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        });

        let mut state = EconomyState::default();
        EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);

        let eco = state.per_system.get(&sys_key).unwrap();
        assert_eq!(eco.summary.fleet_posture.alliance_capships, 2);
        assert_eq!(eco.summary.fleet_posture.empire_capships, 3);
        assert_eq!(eco.summary.fighter_posture.alliance_fighters, 5);
        assert_eq!(eco.summary.fighter_posture.empire_fighters, 0);
    }

    // -----------------------------------------------------------------------
    // Phase 3b: Troop-based side resolution tests
    // -----------------------------------------------------------------------

    /// A populated system at 10 energy with the Alliance's support
    /// `alliance` percent, held as `control`.
    fn held_system(alliance: f32, control: ControlKind) -> crate::world::System {
        crate::world::System {
            dat_id: DatId(0),
            name: "Test".into(),
            sector: crate::ids::SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: alliance / 100.0,
            popularity_empire: 1.0 - alliance / 100.0,
            is_populated: true,
            total_energy: 10,
            raw_materials: 10,
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
            control,
        }
    }

    #[test]
    fn regiments_of_one_side_alone_hold_the_system_and_both_keep_the_holder() {
        // FUN_0050b610: bits 4-5 from FUN_00504cc0(side 1, side 2): one
        // side's regiments alone give that side; both keep the previous
        // side; the holder (bits 0-1) is that side whatever the support.
        use crate::dat::Faction::{Alliance, Empire};
        let gnprtb = stock_gnprtb();
        let empire = held_system(90.0, ControlKind::Controlled(Empire));
        let holder =
            |sys: &crate::world::System, a, e| resolve_system_control(sys, a, e, &gnprtb, 2);
        assert_eq!(
            holder(&empire, true, false),
            ControlKind::Controlled(Alliance)
        );
        assert_eq!(
            holder(&empire, false, true),
            ControlKind::Controlled(Empire)
        );
        // Both: the previous side (it was ControlKind::Contested before
        // FUN_0050b610 was read, 2026-10-17).
        assert_eq!(holder(&empire, true, true), ControlKind::Controlled(Empire));
        // Both with no side before: the support side.
        let neutral = held_system(90.0, ControlKind::Uncontrolled);
        assert_eq!(
            holder(&neutral, true, true),
            ControlKind::Controlled(Alliance)
        );
        // An unpopulated system follows its regiments too.
        let mut bare = held_system(90.0, ControlKind::Uncontrolled);
        bare.is_populated = false;
        assert_eq!(holder(&bare, false, true), ControlKind::Controlled(Empire));
        assert_eq!(holder(&bare, false, false), ControlKind::Uncontrolled);
    }

    #[test]
    fn without_regiments_the_side_with_60_support_holds_the_system() {
        // FUN_0055a080 on +0x58 (the Alliance's support) against GNPRTB
        // 7760 (DAT_006bb4c0, 60): at least 60 gives the Alliance, 100
        // less it at least 60 the Empire, else no side (3). The energy
        // plays no part (the port compared it with 7760 until 2026-10-17).
        use crate::dat::Faction::{Alliance, Empire};
        let gnprtb = stock_gnprtb();
        let holder = |alliance: f32, control: ControlKind| {
            resolve_system_control(&held_system(alliance, control), false, false, &gnprtb, 2)
        };
        let empire = ControlKind::Controlled(Empire);
        assert_eq!(holder(60.0, empire), ControlKind::Controlled(Alliance));
        assert_eq!(holder(59.0, empire), ControlKind::Uncontrolled);
        assert_eq!(holder(41.0, empire), ControlKind::Uncontrolled);
        assert_eq!(holder(40.0, ControlKind::Uncontrolled), empire);
        let mut rich = held_system(50.0, empire);
        rich.total_energy = 200;
        assert_eq!(
            resolve_system_control(&rich, false, false, &gnprtb, 2),
            ControlKind::Uncontrolled
        );
        // port: an uprising of the holding side stays, and a neutral holder
        // keeps its kind.
        assert_eq!(
            holder(80.0, ControlKind::Uprising(Alliance)),
            ControlKind::Uprising(Alliance)
        );
        assert_eq!(
            holder(80.0, ControlKind::Uprising(Empire)),
            ControlKind::Controlled(Alliance)
        );
        let neutral = ControlKind::Controlled(crate::dat::Faction::Neutral);
        assert_eq!(holder(50.0, neutral), neutral);
    }

    #[test]
    fn the_economy_s_run_gives_a_system_to_the_side_of_its_regiments() {
        // FUN_00508250 -> FUN_0050b610: one side's regiments alone take a
        // system the other side holds, whatever the support; with both
        // sides', the holder keeps it; with none, the support decides.
        use crate::dat::Faction::{Alliance, Empire};
        let run = |alliance: bool, empire: bool, holder| {
            let mut world = GameWorld::default();
            world.gnprtb = stock_gnprtb();
            let sys = world
                .systems
                .insert(held_system(90.0, ControlKind::Controlled(holder)));
            for side in [alliance, empire]
                .into_iter()
                .zip([true, false])
                .filter_map(|(present, side)| present.then_some(side))
            {
                let troop = world.troops.insert(crate::world::TroopUnit {
                    class_dat_id: DatId(0),
                    is_alliance: side,
                    regiment_strength: 100,
                });
                world.systems[sys].ground_units.push(troop);
            }
            let mut state = EconomyState::default();
            EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2)
                .into_iter()
                .find_map(|event| match event {
                    EconomyEvent::ControlResolved { new_control, .. } => Some(new_control),
                    _ => None,
                })
        };
        assert_eq!(
            run(false, true, Alliance),
            Some(ControlKind::Controlled(Empire))
        );
        assert_eq!(run(true, true, Alliance), None);
        assert_eq!(run(true, false, Alliance), None);
        assert_eq!(run(false, false, Alliance), None);
        // The Alliance's regiment alone takes an Empire system; with the
        // Empire's too, the Empire keeps it.
        assert_eq!(
            run(true, false, Empire),
            Some(ControlKind::Controlled(Alliance))
        );
        assert_eq!(run(true, true, Empire), None);
    }

    #[test]
    fn the_holder_counts_any_regiment_on_the_planet() {
        // FUN_0050b610 counts each side's regiments with FUN_00504cc0, at
        // any strength.
        let mut world = GameWorld::default();
        world.gnprtb = stock_gnprtb();
        let mut sys = held_system(50.0, ControlKind::Uncontrolled);
        let troop = world.troops.insert(crate::world::TroopUnit {
            class_dat_id: DatId(0),
            is_alliance: false,
            regiment_strength: 0,
        });
        sys.ground_units.push(troop);
        assert_eq!(
            system_holder(&world, &sys),
            ControlKind::Controlled(crate::dat::Faction::Empire)
        );
        sys.ground_units.clear();
        assert_eq!(system_holder(&world, &sys), ControlKind::Uncontrolled);
    }

    // --- Task #43: Unpopulated system skipping ---

    #[test]
    fn unpopulated_system_skipped_in_advance() {
        let gnprtb = stock_gnprtb();
        let mut world = GameWorld {
            gnprtb,
            ..Default::default()
        };
        let _sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Unpopulated".into(),
            sector: SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: false,
            total_energy: 10,
            raw_materials: 10,
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
            control: ControlKind::Uncontrolled,
        });

        let mut state = EconomyState::default();
        let tick = vec![TickEvent { tick: 1 }];
        let events = EconomySystem::advance(&mut state, &world, &tick, 2);

        // Unpopulated system should produce zero events and no economy entry
        assert!(
            events.is_empty(),
            "unpopulated system should produce no events: {events:?}"
        );
        assert!(
            state.per_system.is_empty(),
            "no economy entry for unpopulated system"
        );
    }

    // --- Task #44: Uprising garrison doubling ---

    #[test]
    fn garrison_uprising_doubling() {
        let gnprtb = stock_gnprtb();
        // FUN_0050b5a0 passes system +0x88 bit 2 to FUN_00559fe0, which
        // multiplies by GNPRTB 7682. Controlled: ceil((60 - 30) / 10) = 3.
        let alliance = ControlKind::Controlled(crate::dat::Faction::Alliance);
        let garrison_controlled =
            calculate_garrison_requirement(0.30, alliance, false, false, &gnprtb, 2);
        // In revolt: 3 * GNPRTB[7682](2) = 6
        let garrison_uprising =
            calculate_garrison_requirement(0.30, alliance, false, true, &gnprtb, 2);
        assert_eq!(garrison_controlled, 3, "controlled garrison");
        assert_eq!(
            garrison_uprising,
            garrison_controlled * 2,
            "uprising garrison should be 2x controlled"
        );
    }

    // --- Task #45: Collection rate at zero support ---

    #[test]
    fn collection_rate_at_zero_support() {
        let gnprtb = stock_gnprtb();
        // FUN_00559c10 counts support 0 as 1: (100 * 100) / 1.
        assert_eq!(production_modifier(Some(0), &gnprtb, 2), 10000);
    }

    // --- Task #46: Fleet posture is_contested + strengthen telemetry ---

    #[test]
    fn fleet_posture_contested_requires_two_per_side() {
        let gnprtb = stock_gnprtb();
        let world = GameWorld {
            gnprtb,
            ..Default::default()
        };

        let sys = crate::world::System {
            dat_id: DatId(0),
            name: "Test".into(),
            sector: SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 60,
            raw_materials: 10,
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
            control: ControlKind::Controlled(crate::dat::Faction::Alliance),
        };
        let presence_1v1 = MilitaryPresence {
            alliance_fleets: 1,
            empire_fleets: 1,
            alliance_fighters: 0,
            empire_fighters: 0,
            alliance_troops: 5,
            empire_troops: 0,
            alliance_capships: 0,
            empire_capships: 0,
        };
        let summary_1v1 = compute_system_summary(&world, &sys, &presence_1v1, 0);
        assert!(
            !summary_1v1.fleet_posture.is_contested,
            "1v1 fleets should NOT be contested"
        );

        let presence_2v2 = MilitaryPresence {
            alliance_fleets: 2,
            empire_fleets: 2,
            alliance_fighters: 0,
            empire_fighters: 0,
            alliance_troops: 5,
            empire_troops: 0,
            alliance_capships: 0,
            empire_capships: 0,
        };
        let summary_2v2 = compute_system_summary(&world, &sys, &presence_2v2, 0);
        assert!(
            summary_2v2.fleet_posture.is_contested,
            "2v2 fleets should be contested"
        );
    }

    // -----------------------------------------------------------------------
    // Knesset Shamash-Bet Dabora 2 #K7: parameterized notification
    // idempotency tests for the state-transition events (K1
    // SupportChanged, K3 ResourceDiscovered, K4 MaintenanceShortfall) —
    // each must fire exactly ONCE per transition, not once per tick. K2's
    // support-based disaster was removed: the disaster is a galaxy timer
    // event (FUN_00556b50) that UprisingSystem raises.
    //
    // Per SIMP-L1 the plan explicitly merges six would-be idempotency
    // tests into this one parameterized fixture.
    // -----------------------------------------------------------------------

    /// Helper: build a minimally-seeded world with one system controlled
    /// by the given faction at the given support level.
    fn make_singleton_world(
        support_alliance: f32,
        support_empire: f32,
        control: ControlKind,
        is_populated: bool,
    ) -> (GameWorld, SystemKey) {
        let mut world = GameWorld {
            gnprtb: stock_gnprtb(),
            ..Default::default()
        };
        let sector_key = world.sectors.insert(crate::world::Sector {
            dat_id: DatId(0),
            name: "S".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let sys_key = world.systems.insert(crate::world::System {
            dat_id: DatId(0),
            name: "Test".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: crate::dat::ExplorationStatus::Explored,
            popularity_alliance: support_alliance,
            popularity_empire: support_empire,
            is_populated,
            total_energy: 10,
            raw_materials: 10,
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
            control,
        });
        (world, sys_key)
    }

    #[test]
    fn k1_support_change_fires_only_on_tier_transition() {
        // Start in the Low band (0.25). First eval: default tier is
        // Critical → Low is a transition, emit once. Second eval (no
        // state change): no emission.
        let (world, _sys) = make_singleton_world(
            0.25,
            0.75,
            ControlKind::Controlled(crate::dat::Faction::Alliance),
            true,
        );
        let mut state = EconomyState::default();
        let events1 = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);
        let count1 = events1
            .iter()
            .filter(|e| matches!(e, EconomyEvent::SupportChanged { .. }))
            .count();
        assert_eq!(
            count1, 1,
            "K1: first tick should emit exactly one SupportChanged"
        );

        let events2 = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 2 }], 2);
        let count2 = events2
            .iter()
            .filter(|e| matches!(e, EconomyEvent::SupportChanged { .. }))
            .count();
        assert_eq!(count2, 0, "K1: unchanged-tier second tick must not re-fire");
    }

    #[test]
    fn k3_resource_discovery_fires_on_positive_delta_not_on_seed() {
        // First tick (armed=false at start): no emission because the
        // seeded mine count isn't a "discovery". Second tick (now
        // armed): still no emission because nothing changed.
        let (mut world, sys_key) = make_singleton_world(
            0.5,
            0.5,
            ControlKind::Controlled(crate::dat::Faction::Alliance),
            true,
        );
        let mut state = EconomyState::default();
        let events1 = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);
        let count1 = events1
            .iter()
            .filter(|e| matches!(e, EconomyEvent::ResourceDiscovered { .. }))
            .count();
        assert_eq!(
            count1, 0,
            "K3: seed tick must NOT emit ResourceDiscovered (not armed)"
        );

        // Seed a new mine. Second tick should detect the positive delta.
        let mine = world
            .production_facilities
            .insert(crate::world::ProductionFacilityInstance {
                class_dat_id: DatId(0x2200_0001),
                is_mine: true,
                side: crate::dat::Faction::Alliance,
            });
        world.systems[sys_key].production_facilities.push(mine);

        let events2 = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 2 }], 2);
        let count2 = events2
            .iter()
            .filter(|e| matches!(e, EconomyEvent::ResourceDiscovered { .. }))
            .count();
        assert_eq!(
            count2, 1,
            "K3: new mine should trigger exactly one ResourceDiscovered"
        );

        // Third tick (no change): should not re-fire.
        let events3 = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 3 }], 2);
        let count3 = events3
            .iter()
            .filter(|e| matches!(e, EconomyEvent::ResourceDiscovered { .. }))
            .count();
        assert_eq!(count3, 0, "K3: stable resource count must not re-fire");
    }

    #[test]
    fn k4_maintenance_shortfall_fires_on_30_tick_cooldown() {
        // Garrison deficit condition (low support, no troops).
        // Cooldown is 30 ticks (GNPRTB[7694]). First eval: the cooldown
        // starts at 0 (EconomyState::default()), so the initial deficit
        // fires immediately and then arms the 30-tick cooldown. The
        // idempotency contract is that subsequent ticks within the
        // cooldown window do NOT re-fire, and the next fire only
        // happens after the full 30 ticks elapse.
        let (world, _sys) = make_singleton_world(
            0.3,
            0.7,
            ControlKind::Controlled(crate::dat::Faction::Alliance),
            true,
        );
        let mut state = EconomyState::default();
        // Tick 1: default cooldown=0, deficit present → immediate fire,
        // cooldown armed to 30.
        let events1 = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 1 }], 2);
        let count1 = events1
            .iter()
            .filter(|e| matches!(e, EconomyEvent::MaintenanceShortfall { .. }))
            .count();
        assert_eq!(
            count1, 1,
            "K4: first tick with deficit should emit exactly once"
        );
        assert_eq!(
            state.alliance_maintenance_cooldown, 30,
            "K4: cooldown should be armed to 30 after fire"
        );

        // Burn 29 ticks — cooldown 30 → 1, no fire.
        let batch29: Vec<TickEvent> = (2..=30).map(|n| TickEvent { tick: n }).collect();
        let events_batch = EconomySystem::advance(&mut state, &world, &batch29, 2);
        let count_batch = events_batch
            .iter()
            .filter(|e| matches!(e, EconomyEvent::MaintenanceShortfall { .. }))
            .count();
        assert_eq!(
            count_batch, 0,
            "K4: partial-cooldown batch must not re-fire"
        );
        assert_eq!(
            state.alliance_maintenance_cooldown, 1,
            "K4: cooldown should have one tick left after 29-tick batch"
        );

        // One more tick drops the counter to 0 and fires again.
        let events_fire = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 31 }], 2);
        let count_fire = events_fire
            .iter()
            .filter(|e| matches!(e, EconomyEvent::MaintenanceShortfall { .. }))
            .count();
        assert_eq!(
            count_fire, 1,
            "K4: cooldown expiry should re-emit exactly once"
        );

        // Immediately after firing, the cooldown resets — next tick
        // must not re-fire.
        let events_next = EconomySystem::advance(&mut state, &world, &[TickEvent { tick: 32 }], 2);
        let count_next = events_next
            .iter()
            .filter(|e| matches!(e, EconomyEvent::MaintenanceShortfall { .. }))
            .count();
        assert_eq!(
            count_next, 0,
            "K4: reset cooldown must not re-emit immediately"
        );
    }
}
