//! Save / load for the full game state.
//!
//! # Format (`SAVE_VERSION`)
//!
//! Binary `bincode` encoding. A save file is:
//!
//! ```text
//! [magic: 8 bytes "OPENREB\0"]
//! [version: u32, little-endian]
//! [save_name: length-prefixed UTF-8 string]
//! [timestamp_secs: u64 Unix seconds]
//! [mod_count: u32]
//! for each mod:
//!   [mod_name: length-prefixed UTF-8]
//!   [mod_version: length-prefixed UTF-8]
//! [mod_hash: u64]
//! [fingerprint_version: u16]
//! [state_fingerprint: u64]                  // canonical logical state
//! [bincode-encoded SaveState]
//! ```
//!
//! Only the current version loads. The port has no released saves, so a
//! layout change bumps `SAVE_VERSION` and rejects older files instead of
//! migrating them.
//!
//! `SaveState` wraps all mutable simulation state, including the random-number
//! generator needed to continue deterministically.
//! `GameWorld` (the entity
//! arenas) is included because fleet positions, popularity, etc. change during
//! play. Slotmap keys are stable across a session but are NOT portable across
//! different `load_game_data` calls — the save includes world state, not DAT
//! data. Loading always re-loads DAT files first, then applies the save on top.
//!
//! # WASM
//!
//! File IO is gated with `#[cfg(not(target_arch = "wasm32"))]`. On WASM,
//! saves are stored in browser localStorage through three small imports exposed
//! by the vendored miniquad `gl.js` loader. This keeps the binary compatible
//! with miniquad's raw WASM loader without requiring wasm-bindgen glue.
//!
//! # Save slots
//!
//! Saves live at `<saves_dir>/<slot_index>.reb`. The UI manages up to
//! `MAX_SAVE_SLOTS` named slots. `list_saves()` returns metadata for all
//! occupied slots.

use rand_xoshiro::Xoshiro256PlusPlus;
use serde::{Deserialize, Serialize};

use rebellion_core::ai::AIState;
use rebellion_core::blockade::BlockadeState;
use rebellion_core::economy::EconomyState;
use rebellion_core::events::EventState;
use rebellion_core::fog::FogState;
use rebellion_core::ids::SystemKey;
use rebellion_core::delivery::DeliveryState;
use rebellion_core::manufacturing::ManufacturingState;
use rebellion_core::missions::MissionState;
use rebellion_core::movement::MovementState;
use rebellion_core::production_cycle::ProductionState;
use rebellion_core::repair::RepairState;
use rebellion_core::research::ResearchState;
use rebellion_core::tick::GameClock;
use rebellion_core::troop_transport::TroopTransportState;
use rebellion_core::uprising::UprisingState;
use rebellion_core::victory::VictoryState;
use rebellion_core::world::{CampaignConfig, GameWorld};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Binary magic at the start of every save file. 8 bytes.
pub const SAVE_MAGIC: &[u8; 8] = b"OPENREB\0";

/// Current save format version. Increment when `SaveState` layout changes;
/// saves of any other version are rejected.
pub const SAVE_VERSION: u32 = 138;

/// The oldest version that still loads: v116, from before the object
/// views. Its body lacks `SaveState::object_views` ([`migrate_body`]).
pub const MIGRATED_VERSION: u32 = 116;

/// v117: the object views without the system views
/// (`ObjectViews::alliance_systems`, `empire_systems`), its last fields.
pub const V117: u32 = 117;

/// v118: the object views without the fleet, mission and traveller copies
/// (`ObjectViews::alliance_extras`, `empire_extras`), its last fields.
pub const V118: u32 = 118;

/// v119: the object views without the full fleet and mission copies
/// (`ObjectViews::alliance_details`, `empire_details`), its last fields.
pub const V119: u32 = 119;

/// v120: the object views without the character copies
/// (`ObjectViews::alliance_characters`, `empire_characters`), its last
/// fields.
pub const V120: u32 = 120;

/// v121: the object views without the character copies' state bits
/// (`ObjectViews::alliance_character_states`, `empire_character_states`),
/// its last fields.
pub const V121: u32 = 121;

/// v122: the object views without the character copies' command words
/// (`ObjectViews::alliance_character_commands`, `empire_character_commands`),
/// its last fields.
pub const V122: u32 = 122;

/// v123: the save without the unfinished products (`SaveState::products`),
/// its last field.
pub const V123: u32 = 123;

/// v124: the save whose products lack their trips (`Products::trips`), the
/// products' last field.
pub const V124: u32 = 124;

/// v125: the save without the squadrons' losses
/// (`SaveState::squadron_damage`), its last field.
pub const V125: u32 = 125;

/// v126: the save without the system copies' explored and destroyed bits
/// (`SaveState::system_copy_bits`), its last field.
pub const V126: u32 = 126;

/// v127: the save whose blockade state tracks regiments aboard fleets
/// (`BlockadeState::forget_fleet_borne_percents`); the layout is v128's.
pub const V127: u32 = 127;

/// v128: the save without the AI's movement planners
/// (`SaveState::ai_command_planners`), its last field.
pub const V128: u32 = 128;

/// v129: the save without the movement planners' sector links
/// (`SaveState::ai_sector_links`), its last field.
pub const V129: u32 = 129;

/// v130: the save without the system copies' uprising bits
/// (`SaveState::system_copy_uprisings`), its last field.
pub const V130: u32 = 130;

/// v131: the save without the AI's day (`SaveState::ai_day`), its last
/// field.
pub const V131: u32 = 131;

/// v132: the save without the day's rolls (`SaveState::day_rolls`), its
/// last field.
pub const V132: u32 = 132;

/// v133: the save without the research planners' `+0x20` bits
/// (`SaveState::research_planner_bits`), its last field.
pub const V133: u32 = 133;

/// v134: the save with the retired tuning configuration
/// ([`RetiredTuning`]) after `SaveState::combat_cooldowns`.
pub const V134: u32 = 134;

/// v135: the save without the AI's module `0xa` (`SaveState::ai_module_a`),
/// its last field.
pub const V135: u32 = 135;

/// v136: the save without the AI's module 9 (`SaveState::ai_module9`), its
/// last field.
pub const V136: u32 = 136;

/// v137: the save without the production fleet Destinations
/// (`SaveState::production_fleets`), its last field.
pub const V137: u32 = 137;

/// Whether a save of `version` loads (the current one, or v116 to v137
/// through their migrations).
#[must_use]
pub fn version_loads(version: u32) -> bool {
    (MIGRATED_VERSION..=SAVE_VERSION).contains(&version)
}

/// The two system view maps v118 added at the end of `ObjectViews`, empty.
type SystemViewsTail = (
    std::collections::HashMap<rebellion_core::ids::DatId, rebellion_core::object_views::SystemView>,
    std::collections::HashMap<rebellion_core::ids::DatId, rebellion_core::object_views::SystemView>,
);

/// The two extras maps v119 added at the end of `ObjectViews`, empty.
type ExtrasTail = (
    std::collections::HashMap<rebellion_core::ids::DatId, rebellion_core::object_views::SystemExtras>,
    std::collections::HashMap<rebellion_core::ids::DatId, rebellion_core::object_views::SystemExtras>,
);

/// The two details maps v120 added at the end of `ObjectViews`, empty.
type DetailsTail = (
    std::collections::HashMap<rebellion_core::ids::DatId, rebellion_core::object_views::SystemDetails>,
    std::collections::HashMap<rebellion_core::ids::DatId, rebellion_core::object_views::SystemDetails>,
);

/// The two character copy maps v121 added at the end of `ObjectViews`,
/// empty.
type CharactersTail = (
    std::collections::HashMap<
        rebellion_core::ids::DatId,
        rebellion_core::object_views::CharacterCopy,
    >,
    std::collections::HashMap<
        rebellion_core::ids::DatId,
        rebellion_core::object_views::CharacterCopy,
    >,
);

/// The two character copy state maps v122 added at the end of
/// `ObjectViews`, empty.
type StatesTail = (
    std::collections::HashMap<
        rebellion_core::ids::DatId,
        rebellion_core::object_views::CharacterCopyState,
    >,
    std::collections::HashMap<
        rebellion_core::ids::DatId,
        rebellion_core::object_views::CharacterCopyState,
    >,
);

/// The two character copy command maps v123 added at the end of
/// `ObjectViews`, empty.
type CommandsTail = (
    std::collections::HashMap<
        rebellion_core::ids::DatId,
        rebellion_core::character_command::CommandKind,
    >,
    std::collections::HashMap<
        rebellion_core::ids::DatId,
        rebellion_core::character_command::CommandKind,
    >,
);

/// An older body as a current one. `SaveState::object_views` is the last
/// field, and each version's additions are its last fields: a v116 body
/// takes an empty `ObjectViews`'s encoding, a v117 body six empty maps'
/// (the system views, the extras and the details), a v118 body four (the
/// extras and the details), a v119 body two (the details); each of v117 to
/// v120 then takes the two character copy maps, and each of v117 to v121
/// the two character copy state maps. A v116 to v134 body then loses its
/// [`RetiredTuning`], which comes back beside the body.
///
/// # Errors
/// Returns an error if the empty fields cannot be encoded or the retired
/// tuning cannot be read.
fn migrate_body(
    version: u32,
    mut body: Vec<u8>,
) -> anyhow::Result<(Vec<u8>, Option<RetiredTuning>)> {
    if version == MIGRATED_VERSION {
        body.extend(bincode::serialize(
            &rebellion_core::object_views::ObjectViews::default(),
        )?);
    } else if version == V117 {
        body.extend(bincode::serialize(&SystemViewsTail::default())?);
        body.extend(bincode::serialize(&ExtrasTail::default())?);
        body.extend(bincode::serialize(&DetailsTail::default())?);
    } else if version == V118 {
        body.extend(bincode::serialize(&ExtrasTail::default())?);
        body.extend(bincode::serialize(&DetailsTail::default())?);
    } else if version == V119 {
        body.extend(bincode::serialize(&DetailsTail::default())?);
    }
    if (V117..=V120).contains(&version) {
        body.extend(bincode::serialize(&CharactersTail::default())?);
    }
    if (V117..=V121).contains(&version) {
        body.extend(bincode::serialize(&StatesTail::default())?);
    }
    if (V117..=V122).contains(&version) {
        body.extend(bincode::serialize(&CommandsTail::default())?);
    }
    if version <= V123 {
        body.extend(bincode::serialize(
            &rebellion_core::products::Products::default(),
        )?);
    } else if version == V124 {
        body.extend(bincode::serialize(&Vec::<(u64, u64)>::new())?);
    }
    if version <= V125 {
        // Every squadron of an older save is at full strength.
        body.extend(bincode::serialize(
            &Vec::<rebellion_core::squadrons::DamagedSquadron>::new(),
        )?);
    }
    if version <= V126 {
        // No copy of an older save holds bits: each side reads a system's
        // own exploration, as before (object_views::side_explored).
        body.extend(bincode::serialize(&rebellion_core::object_views::SystemCopyBitsMap::default())?);
    }
    if version <= V128 {
        // An older save's AI made no movement planners: module 0xc starts
        // its cycle afresh and the strategy asks for them again.
        body.extend(bincode::serialize(&AiCommandPlanners::default())?);
    }
    if version <= V129 {
        // An older save's movement planners are the strategy's (types 1
        // and 6): none serves a sector planner, and module 0xc's search
        // for a sector planner's request starts from the first.
        body.extend(bincode::serialize(&AiSectorLinks::default())?);
    }
    if version <= V130 {
        // An older save's copies held no uprising bits; decode_body seeds
        // them from each system's own state.
        body.extend(bincode::serialize(
            &rebellion_core::object_views::SystemCopyUprisings::default(),
        )?);
    }
    if version <= V131 {
        // An older save was made between days, after every AI pass of
        // its day had run.
        body.extend(bincode::serialize(&rebellion_core::ai_schedule::AiDay::default())?);
    }
    if version <= V132 {
        // An older save keeps no day's rolls: a day it was saved within
        // ends with padded rolls (simulation::SliceRolls).
        body.extend(bincode::serialize(&Vec::<f64>::new())?);
    }
    if version <= V133 {
        // An older save's research planners kept no +0x20 bits: none has
        // passed its failure limit or holds an unread check or abort.
        body.extend(bincode::serialize(&AiResearchBits::default())?);
    }
    let mut tuning = None;
    if version <= V134 {
        let (stripped, retired) = strip_retired_tuning(&body)?;
        body = stripped;
        tuning = Some(retired);
    }
    if version <= V135 {
        // An older save ran no module 0xa: it starts its cycle afresh,
        // with no sites.
        body.extend(bincode::serialize(&AiModuleA::default())?);
    }
    if version <= V136 {
        // An older save ran no module 9: it has no planners, and the
        // strategy adds them afresh.
        body.extend(bincode::serialize(&AiModule9::default())?);
    }
    if version <= V137 {
        // An older save's production areas named no fleet as their
        // Destination: the port refused one.
        body.extend(bincode::serialize(&ProductionFleets::default())?);
    }
    Ok((body, tuning))
}

/// The tuning configuration (`GameConfig`) a v116 to v134 body holds after
/// `SaveState::combat_cooldowns`: the AI's six heuristic limits, the
/// production doctrine's two and the three scoring targets. No rule read
/// it once the production heuristic went; v135 drops it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct RetiredTuning {
    ai: RetiredAiTuning,
    production: RetiredProductionTuning,
    scoring: RetiredScoringTuning,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct RetiredAiTuning {
    diplomacy_target_popularity_cap: f32,
    max_covert_ops_per_eval: u64,
    max_construction_yards: u64,
    troop_garrison_min: u64,
    troop_garrison_donor_threshold: u64,
    max_recon_per_eval: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct RetiredProductionTuning {
    capship_threshold: u64,
    fighter_ratio: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct RetiredScoringTuning {
    victory_center: u32,
    combat_pct_center: f64,
    mfg_target: u32,
}

impl Default for RetiredTuning {
    /// The values every v116 to v134 save of the app held.
    fn default() -> Self {
        // port: the retired heuristics' defaults (the removed tuning.rs).
        Self {
            ai: RetiredAiTuning {
                diplomacy_target_popularity_cap: 0.8,
                max_covert_ops_per_eval: 3,
                max_construction_yards: 5,
                troop_garrison_min: 1,
                troop_garrison_donor_threshold: 3,
                max_recon_per_eval: 2,
            },
            production: RetiredProductionTuning {
                capship_threshold: 5,
                fighter_ratio: 3,
            },
            scoring: RetiredScoringTuning {
                victory_center: 3000,
                combat_pct_center: 10.0,
                mfg_target: 50,
            },
        }
    }
}

/// The fields a v116 to v134 body holds before its [`RetiredTuning`], in
/// `SaveState`'s order.
type HeadBeforeTuning = (
    (
        GameWorld,
        GameClock,
        ManufacturingState,
        MissionState,
        EventState,
        AIState,
        MovementState,
        FogState,
        FogState,
        bool,
        BlockadeState,
        UprisingState,
        ResearchState,
        VictoryState,
        EconomyState,
        Xoshiro256PlusPlus,
    ),
    (
        Option<AIState>,
        RepairState,
        ProductionState,
        rebellion_core::autoscrap::AutoscrapState,
        rebellion_core::informant::InformantState,
        rebellion_core::healing::HealingState,
        rebellion_core::force_discovery::ForceDiscoveryState,
        rebellion_core::dagobah::DagobahState,
        rebellion_core::final_battle::FinalBattleState,
        rebellion_core::jabba::JabbaState,
        rebellion_core::seat_of_power::SeatOfPowerState,
        std::collections::HashMap<SystemKey, u64>,
    ),
);

/// A v134 layout `body` without its [`RetiredTuning`], and that tuning.
fn strip_retired_tuning(body: &[u8]) -> anyhow::Result<(Vec<u8>, RetiredTuning)> {
    let mut rest = body;
    let _: HeadBeforeTuning = anyhow::Context::context(
        bincode::deserialize_from(&mut rest),
        "reading the fields before the retired tuning",
    )?;
    let head = body.len() - rest.len();
    let tuning: RetiredTuning = anyhow::Context::context(
        bincode::deserialize_from(&mut rest),
        "reading the retired tuning",
    )?;
    let mut stripped = body[..head].to_vec();
    stripped.extend_from_slice(rest);
    Ok((stripped, tuning))
}

/// Decodes a save body of `version`, checks it against the fingerprint the
/// file holds, and readies it for play: the views go into the world, and a
/// v116 to v118 save's missing views are seeded from each side's fog
/// (`object_views::seed_from_fog`: a v117 save keeps its regiment views
/// and gains its system views; each gains its fleet and mission copies).
///
/// # Errors
/// Returns an error for an unsupported version, an undecodable body or a
/// fingerprint mismatch.
pub fn decode_body(
    version: u32,
    body: Vec<u8>,
    expected: StateFingerprint,
) -> anyhow::Result<(SaveState, StateFingerprint)> {
    anyhow::ensure!(
        version_loads(version),
        "save version {version} is not supported by this build (it reads only versions \
         {MIGRATED_VERSION} to {SAVE_VERSION}). Please start a new game."
    );
    let (body, tuning) = migrate_body(version, body)?;
    let mut state: SaveState =
        anyhow::Context::context(bincode::deserialize(&body), "deserializing save state")?;
    let found = fingerprint_with(&state, version, tuning.unwrap_or_default())?;
    anyhow::ensure!(
        expected == found,
        "save state fingerprint mismatch: expected {expected}, computed {found}"
    );
    state.world.object_views = state.object_views.clone();
    state.world.products = state.products.clone();
    restore_production_fleets(
        &mut state.manufacturing,
        &mut state.world.products,
        &state.production_fleets,
    );
    rebellion_core::squadrons::restore_damaged_squadrons(
        &mut state.world,
        &mut state.troop_transport.squadrons,
        &state.squadron_damage,
    );
    state.world.object_views.system_bits = state.system_copy_bits.clone();
    state.world.object_views.system_uprisings = state.system_copy_uprisings.clone();
    state.restore_command_planners();
    let fogs = [&state.fog_alliance, &state.fog_empire];
    if version == MIGRATED_VERSION {
        rebellion_core::object_views::seed_from_fog(
            &mut state.world,
            &state.troop_transport,
            &state.movement,
            fogs,
        );
    } else if version == V117 {
        rebellion_core::object_views::seed_systems_from_fog(&mut state.world, &state.movement, fogs);
    }
    if version < V119 {
        rebellion_core::object_views::seed_extras_from_fog(
            &mut state.world,
            &state.movement,
            &state.missions,
            fogs,
        );
    }
    if version < V120 {
        // Seeds the character copies too, from the same state.
        rebellion_core::object_views::seed_details_from_fog(
            &mut state.world,
            &state.movement,
            &state.missions,
            &state.troop_transport,
            fogs,
        );
    } else if version == V120 {
        rebellion_core::object_views::seed_characters_from_copies(&mut state.world);
    }
    if version <= V130 {
        // port: an older save's copies (and those seeded above) take each
        // system's uprising bit now.
        rebellion_core::object_views::seed_uprisings(&mut state.world, &state.uprising);
        state.system_copy_uprisings = state.world.object_views.system_uprisings.clone();
    }
    state.object_views = state.world.object_views.clone();
    if version <= V123 {
        // A v123 save's queued units have no products yet: they are
        // placed in their containers now, as their orders would have
        // (crate::products::reconcile). Its deliveries stay trips.
        rebellion_core::products::reconcile(&mut state.world, &state.manufacturing);
    }
    state.products = state.world.products.clone();
    if version <= V127 {
        // Its withdraw percents were copied into regiments aboard fleets,
        // which the original never rolls (ghidra/notes/move-order.md,
        // "Re-traced 2026-11-19"); every regiment starts at 100.
        state.blockade.forget_fleet_borne_percents();
    }
    let fingerprint = compute_state_fingerprint(&state)?;
    Ok((state, fingerprint))
}

/// Current state-fingerprint algorithm version.
///
/// Version 1 is domain-separated FNV-1a over a canonical JSON projection of
/// the logical save state. JSON object keys and known set fields are sorted;
/// meaningful sequence order is preserved. It is an informational determinism
/// and corruption signal, not a cryptographic authentication mechanism.
pub const STATE_FINGERPRINT_VERSION: u16 = 1;

/// Maximum number of named save slots.
pub const MAX_SAVE_SLOTS: usize = 10;

// ---------------------------------------------------------------------------
// Mod hash
// ---------------------------------------------------------------------------

/// Compute a deterministic hash from sorted (name, version) pairs.
///
/// Uses FNV-1a (64-bit). The mod list is sorted before hashing so that
/// insertion order does not affect the result.
#[must_use]
pub fn compute_mod_hash(mods: &[(String, String)]) -> u64 {
    let mut sorted = mods.to_vec();
    sorted.sort();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    for (name, version) in &sorted {
        for byte in name
            .bytes()
            .chain(b":".iter().copied())
            .chain(version.bytes())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV-1a prime
        }
        hash ^= 0xff; // separator between mod entries
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

// ---------------------------------------------------------------------------
// State fingerprint
// ---------------------------------------------------------------------------

/// Versioned fingerprint of a logical game-state snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateFingerprint {
    /// Fingerprint algorithm version, independent of [`SAVE_VERSION`].
    pub version: u16,
    /// Non-cryptographic 64-bit digest.
    pub value: u64,
}

impl std::fmt::Display for StateFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "v{}:{:016x}", self.version, self.value)
    }
}

const UNORDERED_SET_FIELDS: &[&str] = &["blockaded", "busy_characters", "fired_ids", "visible"];

fn canonicalize_fingerprint_value(
    value: &mut serde_json::Value,
    field_name: Option<&str>,
) -> anyhow::Result<()> {
    match value {
        serde_json::Value::Object(fields) => {
            for (name, child) in fields {
                canonicalize_fingerprint_value(child, Some(name))?;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                canonicalize_fingerprint_value(item, None)?;
            }
            if field_name.is_some_and(|name| UNORDERED_SET_FIELDS.contains(&name)) {
                let mut keyed = items
                    .drain(..)
                    .map(|item| Ok((serde_json::to_vec(&item)?, item)))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                keyed.sort_by(|left, right| left.0.cmp(&right.0));
                items.extend(keyed.into_iter().map(|(_, item)| item));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Canonicalize a snapshot and return the fingerprint used by save files.
///
/// # Errors
/// Returns an error if the canonical save state cannot be serialized for hashing.
pub fn compute_state_fingerprint(state: &SaveState) -> anyhow::Result<StateFingerprint> {
    fingerprint_as(state, SAVE_VERSION)
}

/// The fingerprint a save of `version` wrote for `state`: a v116 save had no
/// `object_views` field, a v117 one no system views in it, and a v116 to
/// v134 one held the default [`RetiredTuning`].
fn fingerprint_as(state: &SaveState, version: u32) -> anyhow::Result<StateFingerprint> {
    fingerprint_with(state, version, RetiredTuning::default())
}

/// [`fingerprint_as`], with the [`RetiredTuning`] a v116 to v134 save held.
fn fingerprint_with(
    state: &SaveState,
    version: u32,
    tuning: RetiredTuning,
) -> anyhow::Result<StateFingerprint> {
    let mut canonical_state = serde_json::to_value(state)?;
    if let serde_json::Value::Object(fields) = &mut canonical_state {
        if version <= V137 {
            fields.remove("production_fleets");
        }
        if version <= V136 {
            fields.remove("ai_module9");
        }
        if version <= V135 {
            fields.remove("ai_module_a");
        }
        if version <= V134 {
            fields.insert("game_config".to_owned(), serde_json::to_value(tuning)?);
        }
        if version <= V133 {
            fields.remove("research_planner_bits");
        }
        if version <= V132 {
            fields.remove("day_rolls");
        }
        if version <= V131 {
            fields.remove("ai_day");
        }
        if version <= V130 {
            fields.remove("system_copy_uprisings");
        }
        if version <= V129 {
            fields.remove("ai_sector_links");
        }
        if version <= V128 {
            fields.remove("ai_command_planners");
        }
        if version <= V126 {
            fields.remove("system_copy_bits");
        }
        if version <= V125 {
            fields.remove("squadron_damage");
        }
        if version <= V123 {
            fields.remove("products");
        } else if version == V124 {
            if let Some(serde_json::Value::Object(products)) = fields.get_mut("products") {
                products.remove("trips");
            }
        }
        if version == MIGRATED_VERSION {
            fields.remove("object_views");
        } else if let Some(serde_json::Value::Object(views)) = fields.get_mut("object_views") {
            if version <= V117 {
                views.remove("alliance_systems");
                views.remove("empire_systems");
            }
            if version <= V118 {
                views.remove("alliance_extras");
                views.remove("empire_extras");
            }
            if version <= V119 {
                views.remove("alliance_details");
                views.remove("empire_details");
            }
            if version <= V120 {
                views.remove("alliance_characters");
                views.remove("empire_characters");
            }
            if version <= V121 {
                views.remove("alliance_character_states");
                views.remove("empire_character_states");
            }
            if version <= V122 {
                views.remove("alliance_character_commands");
                views.remove("empire_character_commands");
            }
        }
    }
    canonicalize_fingerprint_value(&mut canonical_state, None)?;
    let canonical_bytes = serde_json::to_vec(&canonical_state)?;

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in b"OPENREB-STATE-FINGERPRINT\0"
        .iter()
        .copied()
        .chain(STATE_FINGERPRINT_VERSION.to_le_bytes())
        .chain(version.to_le_bytes())
        .chain(canonical_bytes)
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    Ok(StateFingerprint {
        version: STATE_FINGERPRINT_VERSION,
        value: hash,
    })
}

// ---------------------------------------------------------------------------
// SaveState — the full serializable snapshot
// ---------------------------------------------------------------------------

/// Complete serializable game state.
///
/// All fields are `#[serde(skip)]`-free — every field must survive a round-trip.
/// The `gnprtb` and `mission_tables` inside `world` are included; on load the
/// caller should re-populate them from DAT files if the saved values are empty
/// (forward-compat with saves from before those fields existed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveState {
    pub world: GameWorld,
    pub clock: GameClock,
    pub manufacturing: ManufacturingState,
    pub missions: MissionState,
    pub events: EventState,
    pub ai: AIState,
    pub movement: MovementState,
    pub fog_alliance: FogState,
    pub fog_empire: FogState,
    /// Faction the player chose at game start (false = Empire).
    pub player_is_alliance: bool,
    // ── v0.4.1: new simulation states ────────────────────────────────────
    pub blockade: BlockadeState,
    pub uprising: UprisingState,
    pub research: ResearchState,
    pub victory: VictoryState,
    // ── v8: economy state (closes incident re-fire on reload bug) ────────
    pub economy: EconomyState,
    // ── v10: deterministic continuation envelope ────────────────────────
    /// Exact simulation RNG position; restoring it prevents post-load rolls
    /// from diverging from an uninterrupted campaign.
    pub sim_rng: Xoshiro256PlusPlus,
    /// Optional AI controlling the player's nominal faction in dual-AI mode.
    pub ai2: Option<AIState>,
    pub repair: RepairState,
    /// Mines, refineries, yards and each side's raw and refined stockpiles
    /// (v33; yard cycles and build points in the queues, v34).
    pub production: ProductionState,
    /// Each side's autoscrap timer `0x382`, the maintenance shortfall (v64).
    pub autoscrap: rebellion_core::autoscrap::AutoscrapState,
    /// Each held system's informant timer `0x38e` (v66).
    pub informant: rebellion_core::informant::InformantState,
    /// Each healing character's timer `0x388` (v68).
    pub healing: rebellion_core::healing::HealingState,
    /// Each discovering trainer's timer `0x389` (v70).
    pub force_discovery: rebellion_core::force_discovery::ForceDiscoveryState,
    /// The mission manager's Dagobah trigger, timer `0x392` (v73).
    pub dagobah: rebellion_core::dagobah::DagobahState,
    /// The mission manager's FinalBattleReady and FinalBattle bits (v76).
    pub final_battle: rebellion_core::final_battle::FinalBattleState,
    /// The mission manager's `BountyHuntersActive` and timer `0x393` (v77).
    pub jabba: rebellion_core::jabba::JabbaState,
    /// The character manager's `SeatOfPower` bit (v78).
    pub seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState,
    /// Last automatic-combat tick per system.
    #[serde(
        serialize_with = "rebellion_core::serde_ordered::serialize_hash_map",
        deserialize_with = "rebellion_core::serde_ordered::deserialize_hash_map"
    )]
    pub combat_cooldowns: std::collections::HashMap<SystemKey, u64>,
    /// Original difficulty, galaxy-size, faction, and victory-condition choices.
    pub campaign_config: CampaignConfig,
    /// Regiments embarked aboard capital-ship transports.
    pub troop_transport: TroopTransportState,
    // ── v16: en-route manufactured objects (F-030) ──────────────────────
    /// Manufactured objects travelling to their destination.
    pub deliveries: DeliveryState,
    // ── v29: the player's agent (Manage Garrisons / Manage Production) ──
    /// Saved with the game as the original saves its agent (`FUN_004397a0`).
    pub player_agent: rebellion_core::agent_automation::PlayerAgent,
    // ── v117: each side's views of the other side's objects ─────────────
    /// `GameWorld::object_views` (`FUN_004f9a60`; `ghidra/notes/per-side-
    /// object-views.md`). It stays the last field: a v116 body is read by
    /// appending an empty one ([`migrate_body`]). The loaders copy it
    /// into `world.object_views`; the world's own encoding skips it.
    pub object_views: rebellion_core::object_views::ObjectViews,
    // ── v124: the unfinished products ───────────────────────────────────
    /// `GameWorld::products` ([`rebellion_core::products`]): each unit
    /// ordered and not yet finished, in its container. It stays the last
    /// field: an older body is read by appending an empty one
    /// ([`migrate_body`]), then its queues are placed. The loaders copy it
    /// into `world.products`; the world's own encoding skips it.
    pub products: rebellion_core::products::Products,
    // ── v126: the squadrons below full strength ─────────────────────────
    /// Each squadron's losses (`+0x60`) and repair timer, in the fleets,
    /// on the planets and in transit
    /// ([`rebellion_core::squadrons::damaged_squadrons`]). It stays the
    /// last field: an older body is read by appending an empty one
    /// ([`migrate_body`]), so its squadrons load at full strength. The
    /// loaders put it back into the world and the transits, whose own
    /// encodings skip it.
    pub squadron_damage: Vec<rebellion_core::squadrons::DamagedSquadron>,
    // ── v127: the system copies' explored and destroyed bits ────────────
    /// Each side's copies' explored and destroyed bits
    /// (`ObjectViews::system_bits`, `ghidra/notes/fog-of-war.md`). It
    /// stays the last field: an older body is read by appending an empty
    /// one ([`migrate_body`]). The loaders put it back into the world's
    /// views, whose own encoding skips it.
    pub system_copy_bits: rebellion_core::object_views::SystemCopyBitsMap,
    // ── v129: the AI's movement planners ────────────────────────────────
    /// Module `0xc` of each AI player (`ai`, then `ai2`): its cycle, its
    /// movement planners and the strategy's requests
    /// ([`rebellion_core::ai_module_c::ModuleC`]). It stays the last field:
    /// an older body is read by appending empty ones ([`migrate_body`]).
    /// The loaders put it back into the AI states, whose own encoding
    /// skips it ([`SaveState::restore_command_planners`]).
    pub ai_command_planners: AiCommandPlanners,
    // ── v130: the movement planners' sector links ───────────────────────
    /// What module `0xc` of each AI player keeps beyond its own encoding
    /// (`ai`, then `ai2`): the sector planner cursor `+0x44` and `+0x40`,
    /// and the owner, flagship and station of each planner serving a
    /// sector planner ([`rebellion_core::ai_module_c::SectorLinks`]). It
    /// stays the last field: an older body is read by appending empty ones
    /// ([`migrate_body`]). The loaders put it back after
    /// [`SaveState::ai_command_planners`]
    /// ([`SaveState::restore_command_planners`]).
    pub ai_sector_links: AiSectorLinks,
    // ── v131: the system copies' uprising bits ──────────────────────────
    /// Each side's copies' uprising bit (`ObjectViews::system_uprisings`,
    /// `ghidra/notes/fog-of-war.md`). It stays the last field: an older
    /// body is read by appending an empty one ([`migrate_body`]) and its
    /// copies are seeded from each system's own state. The loaders put it
    /// back into the world's views, whose own encoding skips it.
    pub system_copy_uprisings: rebellion_core::object_views::SystemCopyUprisings,
    // ── v132: the AI's day ───────────────────────────────────────────────
    /// The current day's AI passes (`rebellion_core::ai_schedule::AiDay`):
    /// the interactive game runs one a sub-tick, so a save made within a
    /// day resumes at its next pass (`ghidra/notes/ai-scheduling.md`). It
    /// stays the last field: an older body, saved between days, is read
    /// by appending a finished day ([`migrate_body`]).
    pub ai_day: rebellion_core::ai_schedule::AiDay,
    // ── v133: the day's rolls ───────────────────────────────────────────
    /// The rolls the interactive game drew for the current day at its
    /// start, as the headless runner draws a day's
    /// (`replay::REPLAY_ROLLS_PER_TICK` from `sim_rng`); the day's end takes
    /// the rest from `ai_day.roll_cursor` (`simulation::SliceRolls`). Empty
    /// between days in a headless runner's save. It stays the last field:
    /// an older body is read by appending an empty one ([`migrate_body`]).
    pub day_rolls: Vec<f64>,
    // ── v134: the research planners' `+0x20` bits ───────────────────────
    /// Each research planner's `+0x20` bits (`ResearchPlanner::pending`:
    /// `0x80` past its failure limit, the report's check and abort), of
    /// `ai`, then `ai2`, in the planners' order. It stays the last field:
    /// an older body is read by appending empty ones ([`migrate_body`]).
    /// The loaders put it back into the AI states, whose own encoding skips
    /// it ([`restore_research_bits`]).
    pub research_planner_bits: AiResearchBits,
    // ── v136: the AI's module `0xa` ─────────────────────────────────────
    /// Module `0xa` (`FUN_004cc6b0`) of `ai`, then `ai2`. It stays the last
    /// field: an older body is read by appending a fresh one
    /// ([`migrate_body`]). The loaders put it back into the AI states,
    /// whose own encoding skips it ([`restore_module_a`]).
    pub ai_module_a: AiModuleA,
    // ── v137: the AI's module 9 ─────────────────────────────────────────
    /// Module 9 (`FUN_004cf0a0`) and its headquarters planners of `ai`,
    /// then `ai2`. It stays the last field: an older body is read by
    /// appending a fresh one ([`migrate_body`]). The loaders put it back
    /// into the AI states, whose own encoding skips it
    /// ([`restore_module9`]).
    pub ai_module9: AiModule9,
    // ── v138: the production fleet Destinations ─────────────────────────
    /// Each production area whose Destination (`0x214`) is a fleet or one
    /// of its ships (`ManufacturingState::destination_target`), and each
    /// product's fleet target, named ship and the ship it waits in
    /// (`products::ProductFleetTarget`) by id. It stays the last field: an
    /// older body is read by appending empty ones ([`migrate_body`]). The
    /// loader puts it back into `manufacturing` and `products`, whose own
    /// encodings skip it ([`restore_production_fleets`]).
    pub production_fleets: ProductionFleets,
}

/// [`SaveState::production_fleets`]: the areas' fleet Destinations and the
/// products' fleet targets.
pub type ProductionFleets = (
    Vec<(
        (
            rebellion_core::ids::SystemKey,
            rebellion_core::manufacturing::ProductionArea,
        ),
        rebellion_core::manufacturing::FleetTarget,
    )>,
    Vec<rebellion_core::products::ProductFleetTarget>,
);

/// The fleet Destinations of `manufacturing` and the fleet targets of
/// `products` for [`SaveState::production_fleets`].
#[must_use]
pub fn production_fleets(
    manufacturing: &rebellion_core::manufacturing::ManufacturingState,
    products: &rebellion_core::products::Products,
) -> ProductionFleets {
    (manufacturing.fleet_destinations(), products.fleet_targets())
}

/// Put `fleets` back into `manufacturing` and `products`.
pub fn restore_production_fleets(
    manufacturing: &mut rebellion_core::manufacturing::ManufacturingState,
    products: &mut rebellion_core::products::Products,
    fleets: &ProductionFleets,
) {
    manufacturing.restore_fleet_destinations(&fleets.0);
    products.restore_fleet_targets(&fleets.1);
}

/// [`SaveState::ai_module9`]: module 9 of `ai` and of `ai2`.
pub type AiModule9 = (
    rebellion_core::ai_module9::Module9,
    Option<rebellion_core::ai_module9::Module9>,
);

/// Module 9 of `ai` and `ai2` for [`SaveState::ai_module9`].
#[must_use]
pub fn module9(ai: &AIState, ai2: Option<&AIState>) -> AiModule9 {
    (ai.module9.clone(), ai2.map(|a| a.module9.clone()))
}

/// Put `saved` back into `ai` and `ai2`; a second side without one starts
/// afresh.
pub fn restore_module9(ai: &mut AIState, ai2: Option<&mut AIState>, saved: &AiModule9) {
    ai.module9 = saved.0.clone();
    if let Some(ai2) = ai2 {
        ai2.module9 = saved.1.clone().unwrap_or_default();
    }
}

/// [`SaveState::ai_module_a`]: module `0xa` of `ai` and of `ai2`.
pub type AiModuleA = (
    rebellion_core::ai_module_a::ModuleA,
    Option<rebellion_core::ai_module_a::ModuleA>,
);

/// Module `0xa` of `ai` and `ai2` for [`SaveState::ai_module_a`].
#[must_use]
pub fn module_a(ai: &AIState, ai2: Option<&AIState>) -> AiModuleA {
    (ai.module_a.clone(), ai2.map(|a| a.module_a.clone()))
}

/// Put `saved` back into `ai` and `ai2`; a second side without one starts
/// afresh.
pub fn restore_module_a(ai: &mut AIState, ai2: Option<&mut AIState>, saved: &AiModuleA) {
    ai.module_a = saved.0.clone();
    if let Some(ai2) = ai2 {
        ai2.module_a = saved.1.clone().unwrap_or_default();
    }
}

/// [`SaveState::research_planner_bits`]: the research planners' `+0x20`
/// bits of `ai` and of `ai2`.
pub type AiResearchBits = (Vec<u32>, Option<Vec<u32>>);

/// The research planners' bits of `ai` and `ai2` for
/// [`SaveState::research_planner_bits`].
#[must_use]
pub fn research_bits(ai: &AIState, ai2: Option<&AIState>) -> AiResearchBits {
    let bits = |a: &AIState| a.research_planners.iter().map(|p| p.pending).collect();
    (bits(ai), ai2.map(bits))
}

/// Put `bits` back into the research planners of `ai` and `ai2`, in their
/// order; a planner without an entry keeps none.
pub fn restore_research_bits(ai: &mut AIState, ai2: Option<&mut AIState>, bits: &AiResearchBits) {
    let restore = |a: &mut AIState, bits: &[u32]| {
        for (at, p) in a.research_planners.iter_mut().enumerate() {
            p.pending = bits.get(at).copied().unwrap_or(0);
        }
    };
    restore(ai, &bits.0);
    if let Some(ai2) = ai2 {
        restore(ai2, bits.1.as_deref().unwrap_or(&[]));
    }
}

/// [`SaveState::ai_command_planners`]: module `0xc` of `ai` and of `ai2`.
pub type AiCommandPlanners = (
    rebellion_core::ai_module_c::ModuleC,
    Option<rebellion_core::ai_module_c::ModuleC>,
);

/// The movement planners of `ai` and `ai2` for
/// [`SaveState::ai_command_planners`].
#[must_use]
pub fn command_planners(ai: &AIState, ai2: Option<&AIState>) -> AiCommandPlanners {
    (ai.module_c.clone(), ai2.map(|a| a.module_c.clone()))
}

/// [`SaveState::ai_sector_links`]: module `0xc`'s sector links of `ai`
/// and of `ai2`.
pub type AiSectorLinks = (
    rebellion_core::ai_module_c::SectorLinks,
    Option<rebellion_core::ai_module_c::SectorLinks>,
);

/// The sector links of `ai` and `ai2` for [`SaveState::ai_sector_links`].
#[must_use]
pub fn sector_links(ai: &AIState, ai2: Option<&AIState>) -> AiSectorLinks {
    (ai.module_c.links(), ai2.map(|a| a.module_c.links()))
}

/// Put `links` back into the movement planners of `ai` and `ai2`, after
/// their [`SaveState::ai_command_planners`].
pub fn restore_sector_links(ai: &mut AIState, ai2: Option<&mut AIState>, links: &AiSectorLinks) {
    ai.module_c.restore_links(&links.0);
    if let (Some(ai2), Some(second)) = (ai2, links.1.as_ref()) {
        ai2.module_c.restore_links(second);
    }
}

impl SaveState {
    /// Put [`Self::ai_command_planners`], then the sector links, the
    /// research planners' bits and modules `0xa` and 9, back into the AI
    /// states.
    pub fn restore_command_planners(&mut self) {
        let (first, second) = self.ai_command_planners.clone();
        self.ai.module_c = first;
        if let (Some(ai2), Some(second)) = (self.ai2.as_mut(), second) {
            ai2.module_c = second;
        }
        restore_sector_links(&mut self.ai, self.ai2.as_mut(), &self.ai_sector_links);
        restore_research_bits(&mut self.ai, self.ai2.as_mut(), &self.research_planner_bits);
        restore_module_a(&mut self.ai, self.ai2.as_mut(), &self.ai_module_a);
        restore_module9(&mut self.ai, self.ai2.as_mut(), &self.ai_module9);
    }
}

// ---------------------------------------------------------------------------
// SaveMeta — slot metadata (no heavy world data)
// ---------------------------------------------------------------------------

/// Lightweight metadata for a save slot — used by the save/load UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMeta {
    /// Slot index (`0..MAX_SAVE_SLOTS`).
    pub slot: usize,
    /// Human-readable name provided by the player.
    pub name: String,
    /// Unix timestamp (seconds since epoch) of when the save was written.
    pub timestamp_secs: u64,
    /// Game tick at save time.
    pub game_tick: u64,
    /// Names of mods that were active when the save was written.
    pub mod_names: Vec<String>,
    /// Deterministic hash of the sorted (name, version) mod list.
    pub mod_hash: u64,
    /// Fingerprint of the canonical logical state, verified on load.
    pub state_fingerprint: StateFingerprint,
    /// The player's side, when known: the Game Options row shows its crest
    /// (`FUN_00407180`: COMMON 10056 Alliance, 10055 Empire).
    pub player_is_alliance: Option<bool>,
}

// ---------------------------------------------------------------------------
// IO functions (native only)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{
        compute_mod_hash, compute_state_fingerprint, decode_body, version_loads, SaveMeta,
        SaveState, StateFingerprint, MAX_SAVE_SLOTS, MIGRATED_VERSION, SAVE_MAGIC, SAVE_VERSION,
        STATE_FINGERPRINT_VERSION,
    };
    use std::io::{Read, Write};
    use std::path::{Path, PathBuf};

    use anyhow::Context;

    /// Default save directory: `<exe_dir>/saves/`.
    #[must_use]
    pub fn default_saves_dir() -> PathBuf {
        // Prefer a directory relative to the executable.  Fall back to the
        // working directory if the executable path is unavailable.
        let base = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("saves")
    }

    /// Path for save slot `slot` under `saves_dir`.
    #[must_use]
    pub fn slot_path(saves_dir: &Path, slot: usize) -> PathBuf {
        saves_dir.join(format!("{slot}.reb"))
    }

    /// Write `state` to slot `slot` in `saves_dir`.
    ///
    /// `active_mods` is a list of `(name, version)` pairs for currently loaded
    /// mods. Pass an empty slice when no mods are active.
    ///
    /// Creates `saves_dir` if it does not exist.
    ///
    /// # Errors
    /// Returns an error if state serialization, fingerprinting, or creating/writing
    /// the save file fails.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Keep the existing fixed-width save encoding; changing overflow handling is outside this lint cleanup."
    )]
    pub fn save_slot(
        saves_dir: &Path,
        slot: usize,
        name: &str,
        state: &SaveState,
        active_mods: &[(String, String)],
    ) -> anyhow::Result<StateFingerprint> {
        std::fs::create_dir_all(saves_dir)
            .with_context(|| format!("creating saves directory {}", saves_dir.display()))?;

        let encoded = bincode::serialize(state).context("serializing save state")?;
        let state_fingerprint = compute_state_fingerprint(state)?;

        let path = slot_path(saves_dir, slot);
        let mut file = std::fs::File::create(&path)
            .with_context(|| format!("creating save file {}", path.display()))?;

        // ── Header ──────────────────────────────────────────────────────────
        file.write_all(SAVE_MAGIC).context("writing save magic")?;
        file.write_all(&SAVE_VERSION.to_le_bytes())
            .context("writing save version")?;

        // Name: u32 length + UTF-8 bytes
        let name_bytes = name.as_bytes();
        file.write_all(&(name_bytes.len() as u32).to_le_bytes())
            .context("writing name length")?;
        file.write_all(name_bytes).context("writing name")?;

        // Timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        file.write_all(&timestamp.to_le_bytes())
            .context("writing timestamp")?;

        // Mod metadata
        file.write_all(&(active_mods.len() as u32).to_le_bytes())
            .context("writing mod count")?;
        for (mod_name, mod_version) in active_mods {
            let nb = mod_name.as_bytes();
            file.write_all(&(nb.len() as u32).to_le_bytes())
                .context("writing mod name length")?;
            file.write_all(nb).context("writing mod name")?;
            let vb = mod_version.as_bytes();
            file.write_all(&(vb.len() as u32).to_le_bytes())
                .context("writing mod version length")?;
            file.write_all(vb).context("writing mod version")?;
        }
        let mod_hash = compute_mod_hash(active_mods);
        file.write_all(&mod_hash.to_le_bytes())
            .context("writing mod hash")?;

        // State fingerprint
        file.write_all(&state_fingerprint.version.to_le_bytes())
            .context("writing state fingerprint version")?;
        file.write_all(&state_fingerprint.value.to_le_bytes())
            .context("writing state fingerprint")?;

        // ── Body ────────────────────────────────────────────────────────────
        file.write_all(&encoded).context("writing save body")?;

        Ok(state_fingerprint)
    }

    /// Convenience wrapper: save with no active mods.
    ///
    /// # Errors
    /// Returns an error if state serialization, fingerprinting, or creating/writing
    /// the save file fails.
    pub fn save_slot_no_mods(
        saves_dir: &Path,
        slot: usize,
        name: &str,
        state: &SaveState,
    ) -> anyhow::Result<StateFingerprint> {
        save_slot(saves_dir, slot, name, state, &[])
    }

    /// Load `SaveState` from slot `slot` in `saves_dir`.
    ///
    /// Only `SAVE_VERSION` loads; any other version is rejected.
    ///
    /// # Errors
    /// Returns an error for unreadable, truncated, corrupt, or unsupported save data,
    /// including invalid text, deserialization failures, and fingerprint mismatches.
    pub fn load_slot(saves_dir: &Path, slot: usize) -> anyhow::Result<(SaveMeta, SaveState)> {
        let path = slot_path(saves_dir, slot);
        let mut file = std::fs::File::open(&path)
            .with_context(|| format!("opening save file {}", path.display()))?;

        // ── Header ──────────────────────────────────────────────────────────
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).context("reading magic")?;
        anyhow::ensure!(
            &magic == SAVE_MAGIC,
            "not a valid save file: bad magic in {}",
            path.display()
        );

        let mut version_buf = [0u8; 4];
        file.read_exact(&mut version_buf)
            .context("reading version")?;
        let version = u32::from_le_bytes(version_buf);
        anyhow::ensure!(
            version_loads(version),
            "save version {version} is not supported by this build (it reads only versions \
             {MIGRATED_VERSION} to {SAVE_VERSION}). Please start a new game."
        );

        let mut name_len_buf = [0u8; 4];
        file.read_exact(&mut name_len_buf)
            .context("reading name length")?;
        let name_len = u32::from_le_bytes(name_len_buf) as usize;
        let mut name_bytes = vec![0u8; name_len];
        file.read_exact(&mut name_bytes).context("reading name")?;
        let name = String::from_utf8(name_bytes).context("invalid save name encoding")?;

        let mut ts_buf = [0u8; 8];
        file.read_exact(&mut ts_buf).context("reading timestamp")?;
        let timestamp_secs = u64::from_le_bytes(ts_buf);

        let mut count_buf = [0u8; 4];
        file.read_exact(&mut count_buf)
            .context("reading mod count")?;
        let mod_count = u32::from_le_bytes(count_buf) as usize;
        let mut mod_names = Vec::with_capacity(mod_count);
        for _ in 0..mod_count {
            let mut len_buf = [0u8; 4];
            file.read_exact(&mut len_buf)
                .context("reading mod name length")?;
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut bytes = vec![0u8; len];
            file.read_exact(&mut bytes).context("reading mod name")?;
            let mod_name = String::from_utf8(bytes).context("invalid mod name encoding")?;

            file.read_exact(&mut len_buf)
                .context("reading mod version length")?;
            let vlen = u32::from_le_bytes(len_buf) as usize;
            let mut vbytes = vec![0u8; vlen];
            file.read_exact(&mut vbytes)
                .context("reading mod version")?;
            // We store name only in meta; version is folded into the hash.
            let _mod_version =
                String::from_utf8(vbytes).context("invalid mod version encoding")?;

            mod_names.push(mod_name);
        }
        let mut hash_buf = [0u8; 8];
        file.read_exact(&mut hash_buf).context("reading mod hash")?;
        let mod_hash = u64::from_le_bytes(hash_buf);

        let mut fingerprint_version_buf = [0u8; 2];
        file.read_exact(&mut fingerprint_version_buf)
            .context("reading state fingerprint version")?;
        let fingerprint_version = u16::from_le_bytes(fingerprint_version_buf);
        anyhow::ensure!(
            fingerprint_version == STATE_FINGERPRINT_VERSION,
            "unsupported state fingerprint version {fingerprint_version} (this build supports {STATE_FINGERPRINT_VERSION})"
        );
        let mut fingerprint_buf = [0u8; 8];
        file.read_exact(&mut fingerprint_buf)
            .context("reading state fingerprint")?;
        let expected = StateFingerprint {
            version: fingerprint_version,
            value: u64::from_le_bytes(fingerprint_buf),
        };

        // ── Body ────────────────────────────────────────────────────────────
        let mut body = Vec::new();
        file.read_to_end(&mut body).context("reading save body")?;
        let (state, state_fingerprint) = decode_body(version, body, expected)?;

        let meta = SaveMeta {
            slot,
            name,
            timestamp_secs,
            game_tick: state.clock.tick,
            mod_names,
            mod_hash,
            state_fingerprint,
            player_is_alliance: Some(state.player_is_alliance),
        };

        Ok((meta, state))
    }

    /// Detect occupancy independently of save decoding. An inaccessible path
    /// fails closed. Game Options does not use it: an unreadable save's row
    /// is empty there (`game_options_saves`).
    #[must_use]
    pub fn slot_occupied(saves_dir: &Path, slot: usize) -> bool {
        slot_path(saves_dir, slot).try_exists().unwrap_or(true)
    }

    /// Return metadata for all occupied save slots in `saves_dir`.
    ///
    /// Slots without a file are silently skipped. Corrupt files are reported
    /// as `Err` entries in the returned vector.
    #[must_use]
    pub fn list_saves(saves_dir: &Path) -> Vec<anyhow::Result<SaveMeta>> {
        (0..MAX_SAVE_SLOTS)
            .filter_map(|slot| {
                let path = slot_path(saves_dir, slot);
                if path.exists() {
                    Some(load_slot(saves_dir, slot).map(|(meta, _)| meta))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Delete a save slot file. No-op if the slot doesn't exist.
    ///
    /// # Errors
    /// Returns an error if an existing save file cannot be deleted.
    pub fn delete_slot(saves_dir: &Path, slot: usize) -> anyhow::Result<()> {
        let path = slot_path(saves_dir, slot);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("deleting save file {}", path.display()))?;
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{
    default_saves_dir, delete_slot, list_saves, load_slot, save_slot, save_slot_no_mods,
    slot_occupied, slot_path,
};

pub use crate::browser_saves::WriteStatus;

/// What became of the last `save_slot` that returned `Ok`. A native save
/// is written before `save_slot` returns; a browser save commits in the
/// background (`browser_saves`).
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn save_write_status() -> WriteStatus {
    WriteStatus::Committed
}

// ---------------------------------------------------------------------------
// WASM browser storage
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
pub mod wasm_impl {
    use super::*;
    use crate::browser_saves::{self, SaveStore, WriteStatus};
    use std::cell::Cell;
    use std::path::{Path, PathBuf};

    #[link(wasm_import_module = "env")]
    extern "C" {
        fn rebellion_storage_set(
            key_ptr: *const u8,
            key_len: usize,
            value_ptr: *const u8,
            value_len: usize,
        ) -> i32;
        fn rebellion_storage_get(
            key_ptr: *const u8,
            key_len: usize,
            output_ptr: *mut u8,
            output_capacity: usize,
        ) -> i32;
        fn rebellion_saves_get(
            key_ptr: *const u8,
            key_len: usize,
            output_ptr: *mut u8,
            output_capacity: usize,
        ) -> i32;
        fn rebellion_saves_put(
            key_ptr: *const u8,
            key_len: usize,
            value_ptr: *const u8,
            value_len: usize,
        ) -> i32;
        fn rebellion_saves_remove(key_ptr: *const u8, key_len: usize) -> i32;
        fn rebellion_saves_commit() -> i32;
        fn rebellion_saves_status(ticket: u32) -> i32;
    }

    thread_local! {
        /// The ticket of the last save's commit.
        static LAST_SAVE: Cell<Option<u32>> = const { Cell::new(None) };
    }

    /// Reads a string through a bridge `get`: -1 absent, below that an
    /// error, else the value's UTF-8 length (asked with capacity 0 first).
    fn bridge_get(
        key: &str,
        get: unsafe extern "C" fn(*const u8, usize, *mut u8, usize) -> i32,
        what: &str,
    ) -> anyhow::Result<Option<String>> {
        let required = unsafe { get(key.as_ptr(), key.len(), std::ptr::null_mut(), 0) };
        match required {
            -1 => return Ok(None),
            n if n < 0 => anyhow::bail!("{what} could not be read"),
            _ => {}
        }
        let mut bytes = vec![0; required as usize];
        let written = unsafe { get(key.as_ptr(), key.len(), bytes.as_mut_ptr(), bytes.len()) };
        if written < 0 || written as usize != bytes.len() {
            anyhow::bail!("{what} changed while being read")
        }
        Ok(Some(String::from_utf8(bytes)?))
    }

    /// The settings' `localStorage` (`rebellion_settings_v1`).
    pub(crate) fn storage_set(key: &str, value: &str) -> anyhow::Result<()> {
        let status =
            unsafe { rebellion_storage_set(key.as_ptr(), key.len(), value.as_ptr(), value.len()) };
        if status == 0 {
            Ok(())
        } else {
            anyhow::bail!("localStorage.setItem failed (access denied or quota exceeded)")
        }
    }

    pub(crate) fn storage_get(key: &str) -> anyhow::Result<Option<String>> {
        bridge_get(key, rebellion_storage_get, "localStorage")
    }

    /// The page's save store (`web/gl.js`, `rebellionSaveStore`): IndexedDB
    /// behind an in-memory copy, or `localStorage` where IndexedDB is
    /// unavailable.
    struct PageStore;

    impl SaveStore for PageStore {
        fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
            bridge_get(key, rebellion_saves_get, "save storage")
        }

        fn put(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
            let status = unsafe {
                rebellion_saves_put(key.as_ptr(), key.len(), value.as_ptr(), value.len())
            };
            anyhow::ensure!(status == 0, "save storage refused a write");
            Ok(())
        }

        fn remove(&mut self, key: &str) -> anyhow::Result<()> {
            let status = unsafe { rebellion_saves_remove(key.as_ptr(), key.len()) };
            anyhow::ensure!(status == 0, "save storage refused a removal");
            Ok(())
        }

        fn commit(&mut self) -> anyhow::Result<u32> {
            let ticket = unsafe { rebellion_saves_commit() };
            anyhow::ensure!(ticket > 0, "save storage refused the commit");
            Ok(ticket as u32)
        }

        fn status(&self, ticket: u32) -> WriteStatus {
            WriteStatus::from_code(unsafe { rebellion_saves_status(ticket) })
        }
    }

    pub fn default_saves_dir() -> PathBuf {
        PathBuf::from("saves")
    }

    /// Start writing the save; [`save_write_status`] says when it is done.
    pub fn save_slot(
        _saves_dir: &Path,
        slot: usize,
        name: &str,
        state: &SaveState,
        _active_mods: &[(String, String)],
    ) -> anyhow::Result<StateFingerprint> {
        LAST_SAVE.set(None);
        let (fingerprint, ticket) = browser_saves::save_slot(&mut PageStore, slot, name, state)?;
        LAST_SAVE.set(Some(ticket));
        Ok(fingerprint)
    }

    pub fn save_slot_no_mods(
        saves_dir: &Path,
        slot: usize,
        name: &str,
        state: &SaveState,
    ) -> anyhow::Result<StateFingerprint> {
        save_slot(saves_dir, slot, name, state, &[])
    }

    /// What became of the last [`save_slot`] that returned `Ok`.
    pub fn save_write_status() -> WriteStatus {
        LAST_SAVE
            .get()
            .map_or(WriteStatus::Committed, |ticket| PageStore.status(ticket))
    }

    pub fn load_slot(_saves_dir: &Path, slot: usize) -> anyhow::Result<(SaveMeta, SaveState)> {
        browser_saves::load_slot(&PageStore, slot)
    }

    pub fn slot_occupied(_saves_dir: &Path, slot: usize) -> bool {
        browser_saves::slot_occupied(&PageStore, slot)
    }

    pub fn list_saves(_saves_dir: &Path) -> Vec<anyhow::Result<SaveMeta>> {
        browser_saves::list_saves(&PageStore)
    }

    pub fn delete_slot(_saves_dir: &Path, slot: usize) -> anyhow::Result<()> {
        browser_saves::delete_slot(&mut PageStore, slot).map(|_| ())
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_impl::{
    default_saves_dir, delete_slot, list_saves, load_slot, save_slot, save_slot_no_mods,
    save_write_status, slot_occupied,
};

/// The saves the Game Options window lists in its first `rows` rows, in
/// slot order. A slot whose save cannot be read is left out, so its row is
/// empty: no name, no crest, Load disabled, and a Save over it asks no
/// overwrite question.
///
/// The original reads each row's header (`FUN_00407180` ->
/// `FUN_00401860` / `FUN_00401910` -> `FUN_00415cd0`). It reads the header
/// (`FUN_00411900`) only when the open (`FUN_005f4a60`) succeeded; else the
/// defaults stay: an empty name and six zero DWORDs (`FUN_00411730`). Load
/// is enabled only when the DWORD at +0x18 is 1, and the Save question
/// (`FUN_00406b80` commands 0x6f..0x74) needs `FUN_00401910` non-zero, the
/// same field. A port save that reads is a single-player save (+0x18 = 1).
#[must_use]
pub fn game_options_saves(saves_dir: &std::path::Path, rows: usize) -> Vec<SaveMeta> {
    // `list_saves` walks the slots in order.
    list_saves(saves_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|meta| meta.slot < rows)
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) mod tests {
    use super::*;
    use rand::{RngCore, SeedableRng};
    use rebellion_core::ai::{AIState, AiFaction};
    use rebellion_core::dat::Faction;
    use rebellion_core::world::ControlKind;

    pub(crate) fn minimal_save_state() -> SaveState {
        // Create a minimal world with two systems for VictoryState
        let mut world = GameWorld::default();
        let sector_key = world.sectors.insert(rebellion_core::world::Sector {
            dat_id: rebellion_core::ids::DatId::new(0x9200_0000),
            name: "Test".into(),
            group: rebellion_core::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let sys_a = world.systems.insert(rebellion_core::world::System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0000),
            name: "A".into(),
            sector: sector_key,
            x: 0,
            y: 0,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: true,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Alliance),
            espionage_rating: 0.0,
        });
        let sys_b = world.systems.insert(rebellion_core::world::System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "B".into(),
            sector: sector_key,
            x: 100,
            y: 100,
            exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 0,
            raw_materials: 0,
            fleets: vec![],
            ground_units: vec![],
            special_forces: vec![],
            defense_facilities: vec![],
            manufacturing_facilities: vec![],
            production_facilities: vec![],
            fighters: Vec::new(),
            is_headquarters: true,
            is_destroyed: false,
            control: ControlKind::Controlled(Faction::Empire),
            espionage_rating: 0.0,
        });
        SaveState {
            world,
            clock: GameClock::default(),
            manufacturing: ManufacturingState::new(),
            missions: MissionState::new(),
            events: EventState::new(),
            ai: AIState::new(AiFaction::Empire),
            movement: MovementState::new(),
            fog_alliance: FogState::new(Faction::Alliance),
            fog_empire: FogState::new(Faction::Empire),
            player_is_alliance: false,
            blockade: BlockadeState::new(),
            uprising: UprisingState::new(),
            research: ResearchState::new(),
            victory: rebellion_core::victory::VictoryState::new(sys_a, sys_b),
            economy: EconomyState::default(),
            sim_rng: Xoshiro256PlusPlus::seed_from_u64(42),
            ai2: None,
            repair: RepairState::default(),
            production: ProductionState::new(),
            autoscrap: rebellion_core::autoscrap::AutoscrapState::default(),
            informant: rebellion_core::informant::InformantState::default(),
            healing: rebellion_core::healing::HealingState::default(),
            force_discovery: rebellion_core::force_discovery::ForceDiscoveryState::default(),
            dagobah: rebellion_core::dagobah::DagobahState::default(),
            final_battle: rebellion_core::final_battle::FinalBattleState::default(),
            jabba: rebellion_core::jabba::JabbaState::default(),
            seat_of_power: rebellion_core::seat_of_power::SeatOfPowerState::default(),
            combat_cooldowns: std::collections::HashMap::new(),
            campaign_config: CampaignConfig::default(),
            troop_transport: TroopTransportState::default(),
            deliveries: DeliveryState::default(),
            player_agent: rebellion_core::agent_automation::PlayerAgent::default(),
            object_views: rebellion_core::object_views::ObjectViews::default(),
            products: rebellion_core::products::Products::default(),
            squadron_damage: Vec::new(),
            system_copy_bits: rebellion_core::object_views::SystemCopyBitsMap::default(),
            system_copy_uprisings: rebellion_core::object_views::SystemCopyUprisings::default(),
            ai_command_planners: AiCommandPlanners::default(),
            ai_sector_links: AiSectorLinks::default(),
            ai_day: rebellion_core::ai_schedule::AiDay::default(),
            day_rolls: Vec::new(),
            research_planner_bits: AiResearchBits::default(),
            ai_module_a: AiModuleA::default(),
            ai_module9: AiModule9::default(),
            production_fleets: ProductionFleets::default(),
        }
    }

    /// Create a unique temp directory scoped to this test.
    fn tmp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("open_rebellion_save_tests")
            .join(name);
        // Start empty: a file left by an earlier run must not stand in for
        // a fixture this run failed to write.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create tmp dir");
        dir
    }

    // ── Existing tests (updated for new save_slot signature) ────────────────

    #[test]
    fn save_and_load_round_trip_preserves_state() {
        let saves_dir = tmp_dir("round_trip_v5");

        let state = minimal_save_state();
        save_slot(&saves_dir, 0, "Test Save", &state, &[]).expect("save should succeed");

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("load should succeed");

        assert_eq!(meta.slot, 0);
        assert_eq!(meta.name, "Test Save");
        assert_eq!(meta.game_tick, loaded.clock.tick);
        assert!(meta.mod_names.is_empty());
        assert_eq!(meta.mod_hash, compute_mod_hash(&[]));
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).expect("fingerprint loaded state")
        );
    }

    #[test]
    fn round_trip_preserves_deterministic_continuation_envelope() {
        let saves_dir = tmp_dir("continuation_envelope_v13");
        let mut state = minimal_save_state();
        state.sim_rng = Xoshiro256PlusPlus::seed_from_u64(0x5eed);
        state.ai2 = Some(AIState::new(AiFaction::Alliance));
        state.ai2.as_mut().unwrap().orders_today = 7;
        let system = state.world.systems.keys().next().unwrap();
        state.combat_cooldowns.insert(system, 61);
        state.campaign_config.galaxy_size = rebellion_core::dat::GalaxySize::Huge;
        state.campaign_config.difficulty = rebellion_core::world::SeedDifficulty::Hard;
        state.campaign_config.victory_conditions =
            rebellion_core::world::VictoryConditions::HeadquartersOnly;

        let facility = state.world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
                side: rebellion_core::dat::Faction::Empire,
                is_shipyard: true,
            },
        );
        state.world.systems[system]
            .manufacturing_facilities
            .push(facility);
        let class =
            state
                .world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    dat_id: rebellion_core::ids::DatId::new(0x1400_0001),
                    name: "Repair fixture".into(),
                    hull: 100,
                    damage_control: 5,
                    troop_capacity: 1,
                    ..Default::default()
                });
        let fleet = state.world.fleets.insert(rebellion_core::world::Fleet {
            location: system,
            capital_ships: vec![rebellion_core::world::ShipInstance::new(class, 75, false)],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        state.world.systems[system].fleets.push(fleet);
        // The drivers number new ships before each day (GameWorld::number_ships);
        // repair keys its timers on the ship id.
        state.world.number_ships();
        let repair_events = rebellion_core::repair::RepairSystem::advance(
            &mut state.repair,
            &state.world,
            &[rebellion_core::tick::TickEvent { tick: 1 }],
        );
        assert!(!repair_events.is_empty());
        assert!(state.repair.is_repairing(fleet));

        let troop = state.world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x1000_0008),
            is_alliance: false,
            regiment_strength: 100,
        });
        state.world.systems[system].ground_units.push(troop);
        state
            .troop_transport
            .embark(&mut state.world, fleet, &[troop])
            .unwrap();

        save_slot(&saves_dir, 0, "Continuation", &state, &[]).unwrap();
        let mut uninterrupted_rng = state.sim_rng.clone();
        let expected_rolls = (0..8)
            .map(|_| uninterrupted_rng.next_u64())
            .collect::<Vec<_>>();

        let (_, mut loaded) = load_slot(&saves_dir, 0).unwrap();
        let loaded_rolls = (0..8)
            .map(|_| loaded.sim_rng.next_u64())
            .collect::<Vec<_>>();

        assert_eq!(loaded_rolls, expected_rolls);
        assert_eq!(loaded.ai2.as_ref().unwrap().orders_today, 7);
        assert_eq!(loaded.combat_cooldowns.get(&system), Some(&61));
        assert_eq!(loaded.campaign_config, state.campaign_config);
        assert!(loaded.repair.is_repairing(fleet));
        assert_eq!(loaded.troop_transport.cargo(fleet), &[troop]);
    }

    #[test]
    fn repeated_snapshots_have_identical_fingerprints() {
        let first_run = minimal_save_state();
        let second_run = minimal_save_state();

        let first = compute_state_fingerprint(&first_run).expect("fingerprint first snapshot");
        let second = compute_state_fingerprint(&second_run).expect("fingerprint second snapshot");

        assert_eq!(first.version, STATE_FINGERPRINT_VERSION);
        assert_eq!(first, second);
        assert!(first.to_string().starts_with("v1:"));
    }

    #[test]
    fn fingerprint_normalizes_unordered_set_insertion() {
        let mut forward = minimal_save_state();
        let mut reverse = forward.clone();
        let system_keys = forward.world.systems.keys().collect::<Vec<_>>();

        forward.fog_alliance.visible.insert(system_keys[0]);
        forward.fog_alliance.visible.insert(system_keys[1]);
        reverse.fog_alliance.visible.insert(system_keys[1]);
        reverse.fog_alliance.visible.insert(system_keys[0]);

        assert_eq!(
            compute_state_fingerprint(&forward).unwrap(),
            compute_state_fingerprint(&reverse).unwrap()
        );
    }

    #[test]
    fn fingerprint_normalizes_typed_map_insertion() {
        let mut forward = minimal_save_state();
        let mut reverse = forward.clone();
        let system_keys = forward.world.systems.keys().collect::<Vec<_>>();

        forward.combat_cooldowns.insert(system_keys[0], 10);
        forward.combat_cooldowns.insert(system_keys[1], 20);
        reverse.combat_cooldowns.insert(system_keys[1], 20);
        reverse.combat_cooldowns.insert(system_keys[0], 10);

        assert_eq!(
            compute_state_fingerprint(&forward).unwrap(),
            compute_state_fingerprint(&reverse).unwrap()
        );
    }

    #[test]
    fn fingerprint_covers_rng_and_campaign_choices() {
        let original = minimal_save_state();
        let mut rng_changed = original.clone();
        rng_changed.sim_rng.next_u64();
        let mut campaign_changed = original.clone();
        campaign_changed.campaign_config.victory_conditions =
            rebellion_core::world::VictoryConditions::HeadquartersOnly;

        let original_fingerprint = compute_state_fingerprint(&original).unwrap();
        assert_ne!(
            original_fingerprint,
            compute_state_fingerprint(&rng_changed).unwrap()
        );
        assert_ne!(
            original_fingerprint,
            compute_state_fingerprint(&campaign_changed).unwrap()
        );
    }

    #[test]
    fn fingerprint_mismatch_rejects_tampered_body() {
        const SAVE_NAME: &str = "Tamper Check";

        let saves_dir = tmp_dir("fingerprint_tamper_v9");
        let state = minimal_save_state();
        save_slot(&saves_dir, 0, SAVE_NAME, &state, &[]).unwrap();

        let path = slot_path(&saves_dir, 0);
        let mut bytes = std::fs::read(&path).unwrap();
        let body_offset = SAVE_MAGIC.len() + 4 + 4 + SAVE_NAME.len() + 8 + 4 + 8 + 2 + 8;
        let mut changed = state.clone();
        changed.clock.tick = 1;
        bytes.truncate(body_offset);
        bytes.extend(bincode::serialize(&changed).unwrap());
        std::fs::write(path, bytes).unwrap();

        let error = load_slot(&saves_dir, 0).expect_err("tampered body must be rejected");
        assert!(error.to_string().contains("fingerprint mismatch"));
    }

    #[test]
    fn list_saves_empty_dir() {
        let saves_dir = tmp_dir("list_empty");
        let metas = list_saves(&saves_dir);
        assert!(metas.is_empty());
    }

    #[test]
    fn list_saves_after_write() {
        let saves_dir = tmp_dir("list_after_write_v5");

        let state = minimal_save_state();
        save_slot(&saves_dir, 2, "Slot 2", &state, &[]).unwrap();
        save_slot(&saves_dir, 5, "Slot 5", &state, &[]).unwrap();

        let metas: Vec<_> = list_saves(&saves_dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .collect();

        assert_eq!(metas.len(), 2);
        assert!(metas.iter().any(|m| m.slot == 2 && m.name == "Slot 2"));
        assert!(metas.iter().any(|m| m.slot == 5 && m.name == "Slot 5"));
    }

    #[test]
    fn an_unreadable_or_empty_save_gives_an_empty_game_options_row() {
        // FUN_00415cd0 reads the header (FUN_00411900) only when the open
        // succeeded; a short read keeps FUN_00411730's defaults (+0x18 = 0),
        // so FUN_00407180 shows no name and disables Load, and FUN_00406b80
        // 0x6f..0x74 asks no overwrite question (FUN_00401910 gives 0).
        let saves_dir = tmp_dir("game_options_unreadable");
        let state = minimal_save_state();
        save_slot(&saves_dir, 3, "Readable", &state, &[]).unwrap();
        std::fs::write(slot_path(&saves_dir, 0), b"").unwrap();
        std::fs::write(slot_path(&saves_dir, 1), b"corrupt save fixture").unwrap();

        let rows = game_options_saves(&saves_dir, 6);

        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].slot, rows[0].name.as_str()), (3, "Readable"));
        // The unreadable files stay as they were.
        assert!(std::fs::read(slot_path(&saves_dir, 0)).unwrap().is_empty());
        assert_eq!(
            std::fs::read(slot_path(&saves_dir, 1)).unwrap(),
            b"corrupt save fixture"
        );
    }

    #[test]
    fn game_options_lists_only_its_rows_in_slot_order() {
        // FUN_00407180 builds six rows; a save in slot 6 is not listed.
        let saves_dir = tmp_dir("game_options_rows");
        let state = minimal_save_state();
        for slot in [6, 5, 0] {
            save_slot(&saves_dir, slot, &format!("Slot {slot}"), &state, &[]).unwrap();
        }

        let slots: Vec<usize> = game_options_saves(&saves_dir, 6)
            .iter()
            .map(|meta| meta.slot)
            .collect();

        assert_eq!(slots, [0, 5]);
    }

    #[test]
    fn delete_slot_removes_file() {
        let saves_dir = tmp_dir("delete_slot_v5");

        let state = minimal_save_state();
        save_slot(&saves_dir, 1, "To Delete", &state, &[]).unwrap();
        assert!(slot_path(&saves_dir, 1).exists());

        delete_slot(&saves_dir, 1).unwrap();
        assert!(!slot_path(&saves_dir, 1).exists());
    }

    // ── New tests (Tasks 3–5) ───────────────────────────────────────────────

    #[test]
    fn mod_hash_round_trip() {
        let saves_dir = tmp_dir("mod_hash_rt");
        let state = minimal_save_state();
        let mods = vec![("TestMod".to_string(), "1.0".to_string())];

        save_slot(&saves_dir, 0, "Modded", &state, &mods).expect("save with mods should succeed");

        let (meta, _) = load_slot(&saves_dir, 0).expect("load modded save should succeed");

        assert_eq!(meta.mod_names, vec!["TestMod".to_string()]);
        assert_eq!(meta.mod_hash, compute_mod_hash(&mods));
        // A different mod set yields a different hash, so callers can detect
        // a save made under other mods.
        let other_mods = vec![("OtherMod".to_string(), "2.0".to_string())];
        assert_ne!(meta.mod_hash, compute_mod_hash(&other_mods));
    }

    #[test]
    fn a_save_of_any_other_version_is_rejected_with_a_new_game_message() {
        let saves_dir = tmp_dir("other_version");
        save_slot(&saves_dir, 0, "Other", &minimal_save_state(), &[]).unwrap();
        let path = slot_path(&saves_dir, 0);
        let original = std::fs::read(&path).unwrap();

        // v116 loads through its migration (migrate_body); the one
        // before it does not.
        for version in [MIGRATED_VERSION - 1, SAVE_VERSION + 1] {
            let mut bytes = original.clone();
            bytes[SAVE_MAGIC.len()..SAVE_MAGIC.len() + 4].copy_from_slice(&version.to_le_bytes());
            std::fs::write(&path, bytes).unwrap();

            let msg = load_slot(&saves_dir, 0).expect_err("only the current version loads").to_string();
            assert!(
                msg.contains(&format!("save version {version}")) && msg.contains("new game"),
                "error should name the version and suggest a new game: {msg}"
            );
        }
    }

    /// `state` written as a v116 file: no `object_views` field, and the
    /// fingerprint v116 wrote. Returns where the fingerprint sits.
    fn write_v116(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        let views = bincode::serialize(&state.object_views).unwrap();
        write_old(saves_dir, state, MIGRATED_VERSION, views.len())
    }

    /// `state` written as a v117 file: no system views, and the fingerprint
    /// v117 wrote. `state` holds no system views.
    fn write_v117(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.object_views.alliance_systems.is_empty());
        assert!(state.object_views.empire_systems.is_empty());
        let tail = bincode::serialize(&SystemViewsTail::default()).unwrap().len()
            + bincode::serialize(&ExtrasTail::default()).unwrap().len()
            + bincode::serialize(&DetailsTail::default()).unwrap().len()
            + characters_tail();
        write_old(saves_dir, state, V117, tail)
    }

    /// `state` written as a v118 file: no extras, and the fingerprint v118
    /// wrote. `state` holds no extras.
    fn write_v118(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.object_views.alliance_extras.is_empty());
        assert!(state.object_views.empire_extras.is_empty());
        let tail = bincode::serialize(&ExtrasTail::default()).unwrap().len()
            + bincode::serialize(&DetailsTail::default()).unwrap().len()
            + characters_tail();
        write_old(saves_dir, state, V118, tail)
    }

    /// `state` written as a v119 file: no details. `state` holds none.
    fn write_v119(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.object_views.alliance_details.is_empty());
        assert!(state.object_views.empire_details.is_empty());
        let tail = bincode::serialize(&DetailsTail::default()).unwrap().len() + characters_tail();
        write_old(saves_dir, state, V119, tail)
    }

    /// The length of the two empty character copy maps v121 added and the
    /// two empty state maps v122 added.
    fn characters_tail() -> usize {
        bincode::serialize(&CharactersTail::default())
            .unwrap()
            .len()
            + states_tail()
    }

    /// The length of the two empty character copy state maps v122 added
    /// and the two empty command maps v123 added.
    fn states_tail() -> usize {
        bincode::serialize(&StatesTail::default()).unwrap().len() + commands_tail()
    }

    /// The length of the two empty character copy command maps v123 added
    /// and the empty products v124 added.
    fn commands_tail() -> usize {
        bincode::serialize(&CommandsTail::default()).unwrap().len() + products_tail()
    }

    /// `state` written as a v124 file: products without trips. `state`
    /// has none travelling.
    fn write_v124(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.products.travelling().count(), 0);
        write_old(saves_dir, state, V124, trips_tail())
    }

    /// The length of the empty trips v125 added and the empty squadron
    /// losses v126 added.
    fn trips_tail() -> usize {
        bincode::serialize(&Vec::<(u64, u64)>::new()).unwrap().len() + damage_tail()
    }

    /// The length of the empty products v124 added and the empty squadron
    /// losses v126 added.
    fn products_tail() -> usize {
        bincode::serialize(&rebellion_core::products::Products::default())
            .unwrap()
            .len()
            + damage_tail()
    }

    /// The length of the empty squadron losses v126 added, and what came
    /// after.
    fn damage_tail() -> usize {
        bincode::serialize(&Vec::<rebellion_core::squadrons::DamagedSquadron>::new())
            .unwrap()
            .len()
            + bits_tail()
    }

    /// The length of the empty system copy bits v127 added, and what came
    /// after.
    fn bits_tail() -> usize {
        bincode::serialize(&rebellion_core::object_views::SystemCopyBitsMap::default())
            .unwrap()
            .len()
            + planners_tail()
    }

    /// The length of the empty movement planners v129 added, and what
    /// came after.
    fn planners_tail() -> usize {
        bincode::serialize(&AiCommandPlanners::default())
            .unwrap()
            .len()
            + links_tail()
    }

    /// The length of the empty sector links v130 added, and what came
    /// after.
    fn links_tail() -> usize {
        bincode::serialize(&AiSectorLinks::default()).unwrap().len() + uprisings_tail()
    }

    /// The length of the empty copy uprising bits v131 added, and what
    /// came after.
    fn uprisings_tail() -> usize {
        bincode::serialize(&rebellion_core::object_views::SystemCopyUprisings::default())
            .unwrap()
            .len()
            + day_tail()
    }

    /// The length of the finished AI day v132 added, and what came after.
    fn day_tail() -> usize {
        bincode::serialize(&rebellion_core::ai_schedule::AiDay::default())
            .unwrap()
            .len()
            + rolls_tail()
    }

    /// The length of the empty day's rolls v133 added.
    fn rolls_tail() -> usize {
        bincode::serialize(&Vec::<f64>::new()).unwrap().len() + research_tail()
    }

    /// The length of the empty research planner bits v134 added.
    fn research_tail() -> usize {
        bincode::serialize(&AiResearchBits::default()).unwrap().len() + module_a_tail()
    }

    /// The length of the fresh module `0xa` v136 added, and what came
    /// after.
    fn module_a_tail() -> usize {
        bincode::serialize(&AiModuleA::default()).unwrap().len() + module9_tail()
    }

    /// The length of the fresh module 9 v137 added.
    fn module9_tail() -> usize {
        bincode::serialize(&AiModule9::default())
            .unwrap()
            .len()
            + production_fleets_tail()
    }

    /// The length of the empty production fleets v138 added.
    fn production_fleets_tail() -> usize {
        bincode::serialize(&ProductionFleets::default())
            .unwrap()
            .len()
    }

    /// `state` written as a v137 file: no production fleets. `state` holds
    /// none.
    fn write_v137(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.production_fleets, ProductionFleets::default());
        write_old(saves_dir, state, V137, production_fleets_tail())
    }

    /// `state` written as a v136 file: no module 9. `state` holds a fresh
    /// one.
    fn write_v136(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.ai_module9, AiModule9::default());
        write_old(saves_dir, state, V136, module9_tail())
    }

    /// `state` written as a v135 file: no module `0xa`. `state` holds a
    /// fresh one.
    fn write_v135(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.ai_module_a, AiModuleA::default());
        write_old(saves_dir, state, V135, module_a_tail())
    }

    /// `state` written as a v133 file: no research planner bits. `state`
    /// holds none.
    fn write_v133(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.research_planner_bits, AiResearchBits::default());
        write_old(saves_dir, state, V133, research_tail())
    }

    /// `state` written as a v132 file: no day's rolls. `state` holds none.
    fn write_v132(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.day_rolls.is_empty());
        write_old(saves_dir, state, V132, rolls_tail())
    }

    /// `state` written as a v131 file: no AI day. `state` holds a
    /// finished one.
    fn write_v131(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.ai_day, rebellion_core::ai_schedule::AiDay::default());
        write_old(saves_dir, state, V131, day_tail())
    }

    /// `state` written as a v130 file: no copy uprising bits. `state`
    /// holds none.
    fn write_v130(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(
            state.system_copy_uprisings,
            rebellion_core::object_views::SystemCopyUprisings::default()
        );
        write_old(saves_dir, state, V130, uprisings_tail())
    }

    /// `state` written as a v129 file: no sector links. `state` holds
    /// none.
    fn write_v129(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.ai_sector_links, AiSectorLinks::default());
        write_old(saves_dir, state, V129, links_tail())
    }

    /// `state` written as a v128 file: no movement planners. `state` holds
    /// none.
    fn write_v128(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.ai_command_planners, AiCommandPlanners::default());
        write_old(saves_dir, state, V128, planners_tail())
    }

    /// `state` written as a v126 file: no system copy bits. `state` holds
    /// none.
    fn write_v126(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert_eq!(state.system_copy_bits, rebellion_core::object_views::SystemCopyBitsMap::default());
        write_old(saves_dir, state, V126, bits_tail())
    }

    /// `state` written as a v125 file: no squadron losses. `state` holds
    /// none.
    fn write_v125(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.squadron_damage.is_empty());
        write_old(saves_dir, state, V125, damage_tail())
    }

    /// `minimal_save_state` with an Empire squadron of 12 craft on its
    /// first system's planet.
    fn state_with_planet_squadron() -> (SaveState, rebellion_core::ids::SystemKey) {
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let tie = state
            .world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass {
                is_empire: true,
                squadron_size: 12,
                ..rebellion_core::world::FighterClass::default()
            });
        rebellion_core::squadrons::add(&mut state.world.systems[system].fighters, tie, 1);
        (state, system)
    }

    /// `state` written as a v123 file: no products. `state` holds none.
    fn write_v123(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.products.all().is_empty());
        write_old(saves_dir, state, V123, products_tail())
    }

    /// `state` written as a v122 file: no character copy commands. `state`
    /// holds none.
    fn write_v122(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.object_views.alliance_character_commands.is_empty());
        assert!(state.object_views.empire_character_commands.is_empty());
        write_old(saves_dir, state, V122, commands_tail())
    }

    /// `state` written as a v121 file: no character copy states. `state`
    /// holds none.
    fn write_v121(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.object_views.alliance_character_states.is_empty());
        assert!(state.object_views.empire_character_states.is_empty());
        write_old(saves_dir, state, V121, states_tail())
    }

    /// `state` written as a v120 file: no character copies. `state` holds
    /// none.
    fn write_v120(saves_dir: &std::path::Path, state: &SaveState) -> usize {
        assert!(state.object_views.alliance_characters.is_empty());
        assert!(state.object_views.empire_characters.is_empty());
        write_old(saves_dir, state, V120, characters_tail())
    }

    /// `state` written as a `version` file whose body is `cut` bytes short.
    fn write_old(saves_dir: &std::path::Path, state: &SaveState, version: u32, cut: usize) -> usize {
        let mut body = bincode::serialize(state).unwrap();
        body.truncate(body.len() - cut);
        if version <= V134 {
            body = with_retired_tuning(&body, RetiredTuning::default());
        }
        // The old body alone is not a current one.
        assert!(bincode::deserialize::<SaveState>(&body).is_err());
        write_body(saves_dir, state, version, &body)
    }

    /// `body` with `tuning` after its `combat_cooldowns`, where a v116 to
    /// v134 body held it.
    fn with_retired_tuning(body: &[u8], tuning: RetiredTuning) -> Vec<u8> {
        let mut rest = body;
        let _: HeadBeforeTuning = bincode::deserialize_from(&mut rest).unwrap();
        let head = body.len() - rest.len();
        let mut old = body[..head].to_vec();
        old.extend(bincode::serialize(&tuning).unwrap());
        old.extend_from_slice(rest);
        old
    }

    /// `body` written as a `version` file fingerprinted from `state`.
    fn write_body(
        saves_dir: &std::path::Path,
        state: &SaveState,
        version: u32,
        body: &[u8],
    ) -> usize {
        let fingerprint = fingerprint_as(state, version).unwrap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(SAVE_MAGIC);
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(b"Old");
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&compute_mod_hash(&[]).to_le_bytes());
        bytes.extend_from_slice(&STATE_FINGERPRINT_VERSION.to_le_bytes());
        let at = bytes.len();
        bytes.extend_from_slice(&fingerprint.value.to_le_bytes());
        bytes.extend_from_slice(body);
        std::fs::create_dir_all(saves_dir).unwrap();
        std::fs::write(slot_path(saves_dir, 0), bytes).unwrap();
        at
    }

    /// `minimal_save_state` with an Imperial regiment on its first system,
    /// which the Alliance's fog shows.
    fn state_with_imperial_regiment() -> (SaveState, rebellion_core::ids::TroopKey) {
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        state.world.systems[system].control =
            rebellion_core::world::ControlKind::Controlled(Faction::Empire);
        let troop = state.world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x1000_0002),
            is_alliance: false,
            regiment_strength: 4,
        });
        state.world.systems[system].ground_units.push(troop);
        state.fog_alliance.reveal(system);
        (state, troop)
    }

    #[test]
    fn a_v116_save_loads_with_views_seeded_from_each_sides_fog() {
        // FUN_004f9a60: the Alliance neither holds the system nor has a
        // fleet there, so its seeded copy is frozen.
        let saves_dir = tmp_dir("v116_migration");
        let (state, troop) = state_with_imperial_regiment();
        write_v116(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v116 save migrates");
        let id = loaded.world.object_views.regiment_id(troop).unwrap();
        let view = loaded
            .world
            .object_views
            .view(Faction::Alliance, id)
            .expect("the fog's system seeds the Alliance's view");
        assert!(!view.live());
        assert!(loaded.world.object_views.empire.is_empty());
        assert_eq!(
            loaded.object_views.alliance.len(),
            loaded.world.object_views.alliance.len()
        );
        assert_eq!(meta.state_fingerprint, compute_state_fingerprint(&loaded).unwrap());
    }

    #[test]
    fn a_v117_save_keeps_its_regiment_views_and_gains_system_views() {
        // FUN_004f9a60: the Alliance's frozen copy keeps its old strength;
        // the system it saw in its fog gets a frozen system copy.
        let saves_dir = tmp_dir("v117_migration");
        let (mut state, troop) = state_with_imperial_regiment();
        let system = state.world.systems.keys().next().unwrap();
        rebellion_core::object_views::seed_from_fog(
            &mut state.world,
            &state.troop_transport,
            &state.movement,
            [&state.fog_alliance, &state.fog_empire],
        );
        state.world.object_views.alliance_systems.clear();
        state.world.object_views.empire_systems.clear();
        state.object_views = state.world.object_views.clone();
        state.world.troops[troop].regiment_strength = 1;
        write_v117(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v117 save migrates");
        let views = &loaded.world.object_views;
        let id = views.regiment_id(troop).unwrap();
        match views.view(Faction::Alliance, id).unwrap() {
            rebellion_core::object_views::ObjectView::Regiment(view) => {
                assert_eq!(view.strength, 4);
            }
        }
        let dat_id = loaded.world.systems[system].dat_id;
        let copy = views.system(Faction::Alliance, dat_id).expect("the fog's system copy");
        assert!(!copy.live);
        assert_eq!(meta.state_fingerprint, compute_state_fingerprint(&loaded).unwrap());
    }

    #[test]
    fn a_v118_save_keeps_its_views_and_gains_fleet_and_mission_copies() {
        // FUN_004f9a60: the Alliance's frozen system copy stays as saved;
        // the system its fog shows gets a frozen extras copy.
        let saves_dir = tmp_dir("v118_migration");
        let (mut state, _) = state_with_imperial_regiment();
        let system = state.world.systems.keys().next().unwrap();
        rebellion_core::object_views::seed_from_fog(
            &mut state.world,
            &state.troop_transport,
            &state.movement,
            [&state.fog_alliance, &state.fog_empire],
        );
        state.object_views = state.world.object_views.clone();
        let dat_id = state.world.systems[system].dat_id;
        let saved = state.object_views.system(Faction::Alliance, dat_id).cloned().unwrap();
        state.world.systems[system].total_energy = 99;
        write_v118(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v118 save migrates");
        let views = &loaded.world.object_views;
        assert_eq!(views.system(Faction::Alliance, dat_id), Some(&saved));
        let extras = views.system_extras(Faction::Alliance, dat_id).expect("the fog's extras");
        assert!(!extras.live);
        assert_eq!(meta.state_fingerprint, compute_state_fingerprint(&loaded).unwrap());
    }

    #[test]
    fn a_v119_save_keeps_its_extras_and_gains_full_fleet_and_mission_copies() {
        // FUN_004f9a60: the saved extras stay; the fog's system gets its
        // full copy (SystemDetails).
        let saves_dir = tmp_dir("v119_migration");
        let (mut state, _) = state_with_imperial_regiment();
        let system = state.world.systems.keys().next().unwrap();
        rebellion_core::object_views::seed_extras_from_fog(
            &mut state.world,
            &state.movement,
            &state.missions,
            [&state.fog_alliance, &state.fog_empire],
        );
        state.object_views = state.world.object_views.clone();
        let dat_id = state.world.systems[system].dat_id;
        let saved = state
            .object_views
            .system_extras(Faction::Alliance, dat_id)
            .cloned()
            .unwrap();
        // An Imperial fleet arrives after the copy: re-seeding the extras
        // would copy it.
        state.world.insert_fleet(rebellion_core::world::Fleet {
            location: system,
            capital_ships: Vec::new(),
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        write_v119(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v119 save migrates");
        let views = &loaded.world.object_views;
        assert_eq!(views.system_extras(Faction::Alliance, dat_id), Some(&saved));
        assert!(views.system_details(Faction::Alliance, dat_id).is_some());
        assert_eq!(meta.state_fingerprint, compute_state_fingerprint(&loaded).unwrap());
        // The v119 fingerprint leaves the details out.
        assert_eq!(
            fingerprint_as(&state, V119).unwrap(),
            fingerprint_as(&loaded, V119).unwrap()
        );
    }

    #[test]
    fn a_v120_save_keeps_its_details_and_gains_character_copies_from_them() {
        // The character copies v121 adds (CharacterCopy, FUN_00465bb0) come
        // from the saved fleet copies, not from the live fleets: an older
        // copy kept no seat, so the fleet names the place.
        let saves_dir = tmp_dir("v120_migration");
        let (mut state, _) = state_with_imperial_regiment();
        let system = state.world.systems.keys().next().unwrap();
        let id = state.world.systems[system].dat_id;
        let vader = rebellion_core::ids::DatId::new(0x3000_0001);
        state.object_views.alliance_details.insert(
            id,
            rebellion_core::object_views::SystemDetails {
                fleets: vec![rebellion_core::object_views::FleetDetail {
                    serial: 1,
                    name: "Fleet 2".into(),
                    is_alliance: false,
                    ships: Vec::new(),
                    fighters: Vec::new(),
                    troops: Vec::new(),
                    characters: vec![vader],
                }],
                ..Default::default()
            },
        );
        let saved = state.object_views.alliance_details.clone();
        write_v120(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v120 save migrates");
        let views = &loaded.world.object_views;
        assert_eq!(views.alliance_details, saved);
        assert_eq!(
            views.character(Faction::Alliance, vader),
            Some(&rebellion_core::object_views::CharacterCopy {
                system: Some(id),
                ship: Some("Fleet 2".into()),
            })
        );
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        // The v120 fingerprint leaves the character copies out.
        assert_eq!(
            fingerprint_as(&state, V120).unwrap(),
            fingerprint_as(&loaded, V120).unwrap()
        );
        assert_ne!(
            fingerprint_as(&loaded, SAVE_VERSION).unwrap(),
            fingerprint_as(&loaded, V120).unwrap()
        );
    }

    #[test]
    fn a_v121_save_keeps_its_character_copies_and_gains_no_state_bits() {
        // v122 adds the copies' state bits (CharacterCopyState,
        // FUN_00465bb0's "( On Mission )" and "( Enroute )"); a v121 copy
        // loads without them.
        let saves_dir = tmp_dir("v121_migration");
        let (mut state, _) = state_with_imperial_regiment();
        let vader = rebellion_core::ids::DatId::new(0x3000_0001);
        let copy = rebellion_core::object_views::CharacterCopy {
            system: Some(rebellion_core::ids::DatId::new(0x9000_0001)),
            ship: None,
        };
        state
            .object_views
            .alliance_characters
            .insert(vader, copy.clone());
        write_v121(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v121 save migrates");
        let views = &loaded.world.object_views;
        assert_eq!(views.character(Faction::Alliance, vader), Some(&copy));
        assert_eq!(
            views.character_state(Faction::Alliance, vader),
            rebellion_core::object_views::CharacterCopyState::default()
        );
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V121).unwrap(),
            fingerprint_as(&loaded, V121).unwrap()
        );
    }

    #[test]
    fn a_v128_save_loads_with_no_movement_planners() {
        // v129 adds module 0xc of each AI (FUN_004c8260); a v128 save made
        // none, so the strategy asks for them again (FUN_004e57e0).
        let saves_dir = tmp_dir("v128_migration");
        let state = minimal_save_state();
        write_v128(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v128 save migrates");
        assert_eq!(loaded.ai_command_planners, AiCommandPlanners::default());
        assert_eq!(
            loaded.ai.module_c,
            rebellion_core::ai_module_c::ModuleC::default()
        );
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V128).unwrap(),
            fingerprint_as(&loaded, V128).unwrap()
        );
    }

    #[test]
    fn a_v129_save_loads_its_planners_with_no_sector_links() {
        // v130 keeps the owner, flagship and station of the planners that
        // serve a sector planner (FUN_00476d10, FUN_004bc5d0); a v129 save
        // held only the strategy's types 1 and 6, which have none.
        let saves_dir = tmp_dir("v129_migration");
        let mut state = minimal_save_state();
        let planner = rebellion_core::ai_module_c::CommandPlanner {
            id: 2,
            kind: 1,
            ..Default::default()
        };
        state.ai.module_c.planners.push(planner);
        state.ai_command_planners = command_planners(&state.ai, state.ai2.as_ref());
        write_v129(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v129 save migrates");
        assert_eq!(loaded.ai_sector_links, AiSectorLinks::default());
        assert_eq!(loaded.ai.module_c, state.ai.module_c);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V129).unwrap(),
            fingerprint_as(&loaded, V129).unwrap()
        );
    }

    #[test]
    fn a_v130_save_seeds_its_copies_uprising_bits_from_each_system() {
        // v131 keeps each copy's uprising bit (+0x88 bit 2, synced by
        // FUN_0050e5b0 mode 1; ghidra/notes/fog-of-war.md). A v130 save
        // held none; its copies take each system's own bit.
        let saves_dir = tmp_dir("v130_migration");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let id = state.world.systems[system].dat_id;
        let view =
            rebellion_core::object_views::SystemView::of(&state.world, &state.world.systems[system], false);
        state.object_views.alliance_systems.insert(id, view);
        state.uprising.active_uprisings.insert(
            system,
            rebellion_core::uprising::ActiveUprising {
                started_tick: 1,
                next_incident_tick: None,
                next_support_tick: None,
                side: None,
            },
        );
        write_v130(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v130 save migrates");
        assert_eq!(loaded.system_copy_uprisings.alliance.get(&id), Some(&true));
        assert!(loaded.world.object_views.system_uprisings.get(Faction::Alliance, id));
        assert!(loaded.system_copy_uprisings.empire.is_empty());
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V130).unwrap(),
            fingerprint_as(&loaded, V130).unwrap()
        );
    }

    #[test]
    fn a_v131_save_loads_between_days() {
        // v132 keeps the AI's day (ai_schedule::AiDay); a v131 save was
        // made after each day's passes had all run.
        let saves_dir = tmp_dir("v131_migration");
        let state = minimal_save_state();
        write_v131(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v131 save migrates");
        assert!(loaded.ai_day.passes_done());
        assert!(!loaded.ai_day.owes_close());
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V131).unwrap(),
            fingerprint_as(&loaded, V131).unwrap()
        );
    }

    #[test]
    fn a_v132_save_loads_without_the_day_s_rolls() {
        // v133 keeps the day's rolls (SaveState::day_rolls); a v132 save
        // had none.
        let saves_dir = tmp_dir("v132_migration");
        let state = minimal_save_state();
        write_v132(&saves_dir, &state);
        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v132 save migrates");
        assert!(loaded.day_rolls.is_empty());
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V132).unwrap(),
            fingerprint_as(&loaded, V132).unwrap()
        );
    }

    #[test]
    fn a_v135_save_loads_with_a_fresh_module_a() {
        // v136 keeps module 0xa (SaveState::ai_module_a); a v135 save ran
        // none, and both sides start its cycle afresh.
        use rebellion_core::ai::{AIState, AiFaction};
        let saves_dir = tmp_dir("v135_migration");
        let mut state = minimal_save_state();
        state.ai2 = Some(AIState::new(AiFaction::Alliance));
        write_v135(&saves_dir, &state);
        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v135 save migrates");
        assert_eq!(loaded.ai_module_a, AiModuleA::default());
        assert_eq!(loaded.ai.module_a, Default::default());
        assert_eq!(loaded.ai2.as_ref().unwrap().module_a, Default::default());
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V135).unwrap(),
            fingerprint_as(&loaded, V135).unwrap()
        );
    }

    #[test]
    fn a_save_keeps_module_a_of_both_sides() {
        // AIState::module_a is skipped by the AI's own encoding; the save
        // carries it after the other fields and the loader puts it back.
        use rebellion_core::ai::{AIState, AiFaction};
        use rebellion_core::ai_module_a::{ModuleA, SQUADRON_CYCLE, YARD_CYCLE};
        let saves_dir = tmp_dir("module_a");
        let mut state = minimal_save_state();
        state.ai.module_a = ModuleA {
            phase: SQUADRON_CYCLE,
            state: 4,
            planner: 3,
            next_planner: 5,
            count: 2,
            ..ModuleA::default()
        };
        let mut ai2 = AIState::new(AiFaction::Alliance);
        ai2.module_a = ModuleA {
            phase: YARD_CYCLE,
            state: 0xb,
            room: 7,
            ..ModuleA::default()
        };
        state.ai2 = Some(ai2);
        state.ai_module_a = module_a(&state.ai, state.ai2.as_ref());
        save_slot(&saves_dir, 0, "Module A", &state, &[]).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.ai.module_a, state.ai.module_a);
        assert_eq!(
            loaded.ai2.as_ref().unwrap().module_a,
            state.ai2.as_ref().unwrap().module_a
        );
        // Module 0xa counts in the current fingerprint.
        let mut cleared = state.clone();
        cleared.ai_module_a = AiModuleA::default();
        assert_ne!(
            compute_state_fingerprint(&cleared).unwrap(),
            compute_state_fingerprint(&state).unwrap()
        );
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&state).unwrap()
        );
    }

    #[test]
    fn a_v137_save_loads_with_no_fleet_destination() {
        // v138 keeps each production area's fleet Destination and each
        // product's fleet target (SaveState::production_fleets); the port
        // refused a fleet Destination before, so a v137 save had none.
        let saves_dir = tmp_dir("v137_fleet_migration");
        let state = minimal_save_state();
        write_v137(&saves_dir, &state);
        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v137 save migrates");
        assert_eq!(loaded.production_fleets, ProductionFleets::default());
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V137).unwrap(),
            fingerprint_as(&loaded, V137).unwrap()
        );
    }

    #[test]
    fn a_save_keeps_a_shipyards_fleet_destination_and_its_products_targets() {
        // FUN_00537180 lets a shipyard manager (0xa2..0xa3) name a fleet;
        // the manufacturing and product encodings skip it, so the save
        // carries it in its last field and the loader puts it back.
        use rebellion_core::manufacturing::{BuildableKind, ProductionArea, QueueItem};
        use rebellion_core::world::{CapitalShipClass, Fleet, ShipInstance};
        let saves_dir = tmp_dir("production_fleets");
        let mut state = minimal_save_state();
        let yard = state.world.systems.keys().next().expect("a system");
        let class = state.world.capital_ship_classes.insert(CapitalShipClass {
            hull: 10,
            ..CapitalShipClass::default()
        });
        let fleet = state.world.insert_fleet(Fleet {
            location: yard,
            capital_ships: vec![ShipInstance::new(class, 10, false)],
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        state.world.systems[yard].fleets.push(fleet);
        assert!(state.manufacturing.set_fleet_destination(
            yard,
            ProductionArea::Shipyard,
            fleet,
            yard
        ));
        let item = QueueItem::new(BuildableKind::CapitalShip(class), 10).ordered(&mut state.world);
        state.manufacturing.build_units(yard, vec![item]);
        rebellion_core::products::reconcile(&mut state.world, &state.manufacturing);
        state.products = state.world.products.clone();
        state.production_fleets = production_fleets(&state.manufacturing, &state.products);
        assert_eq!(state.production_fleets.0.len(), 1);
        assert_eq!(state.production_fleets.1.len(), 1);
        save_slot(&saves_dir, 0, "Fleet", &state, &[]).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(
            loaded
                .manufacturing
                .destination_fleet(yard, ProductionArea::Shipyard),
            Some(fleet)
        );
        assert_eq!(loaded.products, state.products);
        assert_eq!(loaded.world.products, state.products);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&state).unwrap()
        );
    }

    #[test]
    fn a_v136_save_loads_with_a_fresh_module9() {
        // v137 keeps module 9 (SaveState::ai_module9); a v136 save ran
        // none, and both sides start with no planners.
        use rebellion_core::ai::{AIState, AiFaction};
        let saves_dir = tmp_dir("v136_migration");
        let mut state = minimal_save_state();
        state.ai2 = Some(AIState::new(AiFaction::Alliance));
        write_v136(&saves_dir, &state);
        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v136 save migrates");
        assert_eq!(loaded.ai_module9, AiModule9::default());
        assert_eq!(loaded.ai.module9, Default::default());
        assert_eq!(loaded.ai2.as_ref().unwrap().module9, Default::default());
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V136).unwrap(),
            fingerprint_as(&loaded, V136).unwrap()
        );
    }

    #[test]
    fn a_save_keeps_module9_of_both_sides() {
        // AIState::module9 is skipped by the AI's own encoding; the save
        // carries it after the other fields and the loader puts it back.
        use rebellion_core::ai::{AIState, AiFaction};
        use rebellion_core::ai_module9::{HqPlanner, Module9};
        let saves_dir = tmp_dir("module9");
        let mut state = minimal_save_state();
        state.ai.module9 = Module9 {
            state: 3,
            current: 1,
            next: 0,
            planners: vec![HqPlanner {
                id: 1,
                state: 4,
                ships: vec![7, 9],
                balance: -120,
                dirty: true,
                ..HqPlanner::default()
            }],
            next_id: 2,
        };
        let mut ai2 = AIState::new(AiFaction::Alliance);
        ai2.module9 = Module9 {
            state: 2,
            next_id: 5,
            ..Module9::default()
        };
        state.ai2 = Some(ai2);
        state.ai_module9 = module9(&state.ai, state.ai2.as_ref());
        save_slot(&saves_dir, 0, "Module 9", &state, &[]).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.ai.module9, state.ai.module9);
        assert_eq!(
            loaded.ai2.as_ref().unwrap().module9,
            state.ai2.as_ref().unwrap().module9
        );
        // Module 9 counts in the current fingerprint.
        let mut cleared = state.clone();
        cleared.ai_module9 = AiModule9::default();
        assert_ne!(
            compute_state_fingerprint(&cleared).unwrap(),
            compute_state_fingerprint(&state).unwrap()
        );
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&state).unwrap()
        );
    }

    #[test]
    fn a_v133_save_loads_without_the_research_planners_bits() {
        // v134 keeps the research planners' +0x20 bits
        // (SaveState::research_planner_bits); a v133 save had none, and
        // its planners load with none.
        use rebellion_core::ai::ResearchPlanner;
        use rebellion_core::research::TechType;
        let saves_dir = tmp_dir("v133_migration");
        let mut state = minimal_save_state();
        state.ai.research_planners.push(ResearchPlanner {
            id: 1,
            tech: TechType::Ship,
            member: None,
            site: None,
            target: None,
            mission: None,
            report: None,
            failures: 3,
            pending: 0,
        });
        write_v133(&saves_dir, &state);
        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v133 save migrates");
        assert_eq!(loaded.research_planner_bits, AiResearchBits::default());
        assert_eq!(loaded.ai.research_planners, state.ai.research_planners);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V133).unwrap(),
            fingerprint_as(&loaded, V133).unwrap()
        );
    }

    #[test]
    fn a_v134_save_loads_without_its_tuning_configuration() {
        // v135 drops the tuning configuration no rule read; a v134 save
        // held it after combat_cooldowns, and its fingerprint covered it.
        let saves_dir = tmp_dir("v134_migration");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        state.combat_cooldowns.insert(system, 61);
        let mut tuning = RetiredTuning::default();
        tuning.ai.troop_garrison_min = 13;
        // A v134 body also lacks module 0xa (v136).
        let mut current = bincode::serialize(&state).unwrap();
        current.truncate(current.len() - module_a_tail());
        let body = with_retired_tuning(&current, tuning);
        assert_eq!(
            body.len(),
            current.len() + 76,
            "the tuning's eleven fixed-width fields"
        );
        let fingerprint = fingerprint_with(&state, V134, tuning).unwrap();
        let at = write_body(&saves_dir, &state, V134, &body);
        let path = slot_path(&saves_dir, 0);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[at..at + 8].copy_from_slice(&fingerprint.value.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v134 save migrates");
        assert_eq!(loaded.combat_cooldowns.get(&system), Some(&61));
        assert_eq!(loaded.campaign_config, state.campaign_config);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_ne!(
            fingerprint,
            fingerprint_as(&state, V134).unwrap(),
            "the fingerprint covered the saved tuning"
        );
    }

    #[test]
    fn a_save_keeps_the_research_planners_bits_of_both_sides() {
        // ResearchPlanner::pending (+0x20) is skipped by the AI's own
        // encoding; the save carries it after the other fields and the
        // loader puts it back by the planners' order.
        use rebellion_core::ai::{AIState, AiFaction, ResearchPlanner};
        use rebellion_core::research::TechType;
        let saves_dir = tmp_dir("research_bits");
        let mut state = minimal_save_state();
        let planner = |id, pending| ResearchPlanner {
            id,
            tech: TechType::Troop,
            member: None,
            site: None,
            target: None,
            mission: Some(id),
            report: None,
            failures: 0,
            pending,
        };
        state.ai.research_planners = vec![planner(1, 0x80), planner(2, 0x4000_0000)];
        let mut ai2 = AIState::new(AiFaction::Alliance);
        ai2.research_planners = vec![planner(3, 0x2000_0080)];
        state.ai2 = Some(ai2);
        state.research_planner_bits = research_bits(&state.ai, state.ai2.as_ref());
        assert_eq!(
            state.research_planner_bits,
            (vec![0x80, 0x4000_0000], Some(vec![0x2000_0080]))
        );
        save_slot(&saves_dir, 0, "Bits", &state, &[]).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.ai.research_planners, state.ai.research_planners);
        assert_eq!(
            loaded.ai2.as_ref().unwrap().research_planners,
            state.ai2.as_ref().unwrap().research_planners
        );
        // The bits count in the current fingerprint.
        let mut cleared = state.clone();
        cleared.research_planner_bits = AiResearchBits::default();
        assert_ne!(
            compute_state_fingerprint(&cleared).unwrap(),
            compute_state_fingerprint(&state).unwrap()
        );
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&state).unwrap()
        );
    }

    #[test]
    fn a_save_keeps_the_day_s_rolls() {
        let saves_dir = tmp_dir("day_rolls");
        let mut state = minimal_save_state();
        state.day_rolls = vec![0.25, 0.5];
        save_slot(&saves_dir, 0, "Rolls", &state, &[]).unwrap();
        let (_, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.day_rolls, [0.25, 0.5]);
    }

    #[test]
    fn loading_a_mid_day_save_resumes_at_its_pass() {
        // SaveState::ai_day (v132): the interactive game runs one AI pass
        // a sub-tick (FUN_0041dff0 phase 8), so a save can fall within a
        // day; it keeps the passes run and the end of the day owed.
        let saves_dir = tmp_dir("mid_day");
        let mut state = minimal_save_state();
        let mut day = rebellion_core::ai_schedule::AiDay::begin(
            9,
            61,
            0x1234,
            rebellion_core::loyalty::LoyaltyWatch::default(),
            1,
        );
        day.done = 17;
        state.ai_day = day;
        save_slot(&saves_dir, 0, "Mid-day", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.ai_day, day);
        assert_eq!(loaded.ai_day.done, 17);
        assert!(!loaded.ai_day.passes_done());
    }

    #[test]
    fn a_save_keeps_its_copies_uprising_bits() {
        // SaveState::system_copy_uprisings (v131): the views' own encoding
        // skips them.
        let saves_dir = tmp_dir("copy_uprisings");
        let mut state = minimal_save_state();
        let id = state.world.systems.values().next().unwrap().dat_id;
        state.system_copy_uprisings.empire.insert(id, true);
        save_slot(&saves_dir, 0, "Uprisings", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.system_copy_uprisings, state.system_copy_uprisings);
        assert!(loaded.world.object_views.system_uprisings.get(Faction::Empire, id));
    }

    #[test]
    fn a_save_keeps_the_sector_links_of_the_officers_planners() {
        // SaveState::ai_sector_links (v130): module 0xc's own encoding
        // skips a planner's owner (+0x54), flagship (+0x28) and station
        // (+0x48) and the sector cursor (+0x44, +0x40); the save carries
        // them and the loader puts them back.
        let saves_dir = tmp_dir("sector_links");
        let mut state = minimal_save_state();
        let station = state.world.systems.keys().next();
        let planner = rebellion_core::ai_module_c::CommandPlanner {
            id: 5,
            kind: 4,
            owner: 3,
            flagship: Some(77),
            station,
            ..Default::default()
        };
        state.ai.module_c.planners.push(planner);
        state.ai.module_c.sector_cursor = 3;
        state.ai.module_c.sector_found = 2;
        state.ai_command_planners = command_planners(&state.ai, state.ai2.as_ref());
        state.ai_sector_links = sector_links(&state.ai, state.ai2.as_ref());
        save_slot(&saves_dir, 0, "Links", &state, &[]).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.ai.module_c, state.ai.module_c);
        assert_eq!(loaded.ai.module_c.planners[0].owner, 3);
        assert_eq!(loaded.ai.module_c.sector_cursor, 3);
        // The links count in the current fingerprint.
        let mut unlinked = state.clone();
        unlinked.ai_sector_links = AiSectorLinks::default();
        assert_ne!(
            compute_state_fingerprint(&unlinked).unwrap(),
            compute_state_fingerprint(&state).unwrap()
        );
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&state).unwrap()
        );
    }

    #[test]
    fn a_save_keeps_each_ai_s_movement_planners() {
        // SaveState::ai_command_planners (v129): the AI state's own
        // encoding skips module 0xc, so the save carries it and the loader
        // puts it back.
        let saves_dir = tmp_dir("command_planners");
        let mut state = minimal_save_state();
        let planner = rebellion_core::ai_module_c::CommandPlanner {
            id: 3,
            kind: 1,
            ..Default::default()
        };
        state.ai.module_c.planners.push(planner);
        state.ai.module_c.made = [1, 0];
        state.ai_command_planners = command_planners(&state.ai, state.ai2.as_ref());
        save_slot(&saves_dir, 0, "Planners", &state, &[]).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.ai.module_c, state.ai.module_c);
        assert_eq!(loaded.ai_command_planners, state.ai_command_planners);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&state).unwrap()
        );
    }

    #[test]
    fn a_v126_save_loads_with_no_copy_bits() {
        // v127 adds each side's copies' explored and destroyed bits
        // (ghidra/notes/fog-of-war.md); a v126 save kept none, so its
        // copies read the systems' own exploration
        // (object_views::side_explored).
        let saves_dir = tmp_dir("v126_migration");
        let state = minimal_save_state();
        write_v126(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v126 save migrates");
        assert_eq!(loaded.system_copy_bits, rebellion_core::object_views::SystemCopyBitsMap::default());
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V126).unwrap(),
            fingerprint_as(&loaded, V126).unwrap()
        );
    }

    #[test]
    fn a_save_keeps_each_sides_copy_bits() {
        // SaveState::system_copy_bits (v127): the views' own encoding skips
        // the bits, so the save carries them and the loader puts them back.
        let saves_dir = tmp_dir("system_copy_bits");
        let mut state = minimal_save_state();
        let id = state.world.systems.values().next().unwrap().dat_id;
        state.system_copy_bits.alliance.insert(
            id,
            rebellion_core::object_views::SystemCopyBits {
                explored: false,
                destroyed: true,
            },
        );
        save_slot(&saves_dir, 0, "Bits", &state, &[]).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.system_copy_bits, state.system_copy_bits);
        assert_eq!(
            loaded.world.object_views.system_bits,
            state.system_copy_bits
        );
        assert_eq!(meta.state_fingerprint, compute_state_fingerprint(&state).unwrap());
    }

    #[test]
    fn a_v125_save_loads_every_squadron_at_full_strength() {
        // v126 adds the squadrons' losses (+0x60, FUN_0040d260); a v125
        // save kept none, so each of its squadrons loads whole.
        let saves_dir = tmp_dir("v125_migration");
        let (state, system) = state_with_planet_squadron();
        write_v125(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v125 save migrates");
        assert!(loaded.squadron_damage.is_empty());
        assert_eq!(loaded.world.systems[system].fighters[0].damage, None);
        assert_eq!(loaded.world.systems[system].fighters[0].count, 1);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V125).unwrap(),
            fingerprint_as(&loaded, V125).unwrap()
        );
    }

    #[test]
    fn a_v125_fingerprint_has_no_squadron_losses() {
        // A v125 file was fingerprinted without SaveState::squadron_damage
        // (v126), so its check ignores the field.
        let (mut state, system) = state_with_planet_squadron();
        let before = fingerprint_as(&state, V125).unwrap();
        state.world.systems[system].fighters[0].damage =
            Some(rebellion_core::world::SquadronDamage { lost: 1, repair: None });
        state.squadron_damage = rebellion_core::squadrons::damaged_squadrons(
            &state.world,
            &state.troop_transport.squadrons,
        );
        assert_eq!(fingerprint_as(&state, V125).unwrap(), before);
    }

    #[test]
    fn a_save_keeps_each_squadrons_losses_and_repair_timer() {
        // SaveState::squadron_damage (v126): the world's own encoding skips
        // the losses, so the save carries them and the loader puts them back.
        let saves_dir = tmp_dir("squadron_damage");
        let (mut state, system) = state_with_planet_squadron();
        let damage = rebellion_core::world::SquadronDamage {
            lost: 5,
            repair: Some(rebellion_core::repair::RepairTimer { due: 40, fast: false }),
        };
        state.world.systems[system].fighters[0].damage = Some(damage);
        state.squadron_damage = rebellion_core::squadrons::damaged_squadrons(
            &state.world,
            &state.troop_transport.squadrons,
        );
        save_slot(&saves_dir, 0, "Losses", &state, &[]).unwrap();

        let (meta, loaded) = load_slot(&saves_dir, 0).unwrap();
        assert_eq!(loaded.world.systems[system].fighters[0].damage, Some(damage));
        assert_eq!(loaded.squadron_damage, state.squadron_damage);
        assert_eq!(meta.state_fingerprint, compute_state_fingerprint(&state).unwrap());
    }

    #[test]
    fn a_v124_save_keeps_its_products_with_no_trips() {
        // v125 adds the products' trips (a finished product travelling in
        // its container, FUN_0052bee0); a v124 save had none.
        use rebellion_core::manufacturing::{BuildableKind, QueueItem};
        let saves_dir = tmp_dir("v124_migration");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let wing = state
            .world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass::default());
        state
            .manufacturing
            .enqueue(system, QueueItem::new(BuildableKind::Fighter(wing), 10));
        rebellion_core::products::reconcile(&mut state.world, &state.manufacturing);
        state.products = state.world.products.clone();
        write_v124(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v124 save migrates");
        assert_eq!(loaded.world.products, state.products);
        assert_eq!(loaded.world.products.all().len(), 1);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V124).unwrap(),
            fingerprint_as(&loaded, V124).unwrap()
        );
    }

    #[test]
    fn a_v123_save_places_its_queued_units_in_their_containers() {
        // v124 keeps the unfinished products (FUN_0052a140 creates each
        // ordered unit at the order, in its container). A v123 save had
        // only the queue: its queued ship goes into a new spare, its
        // squadron into the system (crate::products::reconcile).
        use rebellion_core::manufacturing::{BuildableKind, QueueItem};
        let saves_dir = tmp_dir("v123_migration");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        state.world.systems[system].control =
            rebellion_core::world::ControlKind::Controlled(Faction::Empire);
        let ship = state
            .world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass::default());
        let wing = state
            .world
            .fighter_classes
            .insert(rebellion_core::world::FighterClass::default());
        state
            .manufacturing
            .enqueue(system, QueueItem::new(BuildableKind::CapitalShip(ship), 10));
        state
            .manufacturing
            .enqueue(system, QueueItem::new(BuildableKind::Fighter(wing), 10));
        write_v123(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v123 save migrates");
        let products = loaded.world.products.all();
        assert_eq!(products.len(), 2);
        assert!(matches!(
            products[0].container,
            rebellion_core::products::Container::Fleet(_)
        ));
        assert_eq!(
            products[1].container,
            rebellion_core::products::Container::System(system)
        );
        assert_eq!(loaded.products, loaded.world.products);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        // The v123 fingerprint leaves the products out.
        let mut placed = loaded.clone();
        placed.products = rebellion_core::products::Products::default();
        assert_eq!(
            fingerprint_as(&placed, V123).unwrap(),
            fingerprint_as(&loaded, V123).unwrap()
        );
    }

    #[test]
    fn a_v122_save_keeps_its_character_copies_and_gains_no_command_words() {
        // v123 adds the copies' command words (+0x96, FUN_004f04f0); a v122
        // copy loads without them.
        let saves_dir = tmp_dir("v122_migration");
        let (mut state, _) = state_with_imperial_regiment();
        let vader = rebellion_core::ids::DatId::new(0x3000_0001);
        let copy = rebellion_core::object_views::CharacterCopy {
            system: Some(rebellion_core::ids::DatId::new(0x9000_0001)),
            ship: None,
        };
        state
            .object_views
            .alliance_characters
            .insert(vader, copy.clone());
        write_v122(&saves_dir, &state);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v122 save migrates");
        let views = &loaded.world.object_views;
        assert_eq!(views.character(Faction::Alliance, vader), Some(&copy));
        assert_eq!(
            views.character_command(Faction::Alliance, vader),
            rebellion_core::character_command::CommandKind::None
        );
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
        assert_eq!(
            fingerprint_as(&state, V122).unwrap(),
            fingerprint_as(&loaded, V122).unwrap()
        );
        // The v122 fingerprint leaves the command words out.
        let old = fingerprint_as(&state, V122).unwrap();
        let current = fingerprint_as(&state, SAVE_VERSION).unwrap();
        state.object_views.alliance_character_commands.insert(
            vader,
            rebellion_core::character_command::CommandKind::General,
        );
        assert_eq!(fingerprint_as(&state, V122).unwrap(), old);
        assert_ne!(fingerprint_as(&state, SAVE_VERSION).unwrap(), current);
    }

    #[test]
    fn the_v121_fingerprint_leaves_out_the_character_copy_states() {
        let (mut state, _) = state_with_imperial_regiment();
        let old = fingerprint_as(&state, V121).unwrap();
        let current = fingerprint_as(&state, SAVE_VERSION).unwrap();
        state.object_views.alliance_character_states.insert(
            rebellion_core::ids::DatId::new(0x3000_0001),
            rebellion_core::object_views::CharacterCopyState {
                on_mission: false,
                hidden_mission: false,
                enroute: true,
            },
        );
        assert_eq!(fingerprint_as(&state, V121).unwrap(), old);
        assert_ne!(fingerprint_as(&state, SAVE_VERSION).unwrap(), current);
    }

    #[test]
    fn the_v118_fingerprint_leaves_out_the_extras() {
        let (mut state, _) = state_with_imperial_regiment();
        let old = fingerprint_as(&state, V118).unwrap();
        rebellion_core::object_views::seed_extras_from_fog(
            &mut state.world,
            &state.movement,
            &state.missions,
            [&state.fog_alliance, &state.fog_empire],
        );
        state.object_views = state.world.object_views.clone();
        assert!(!state.object_views.alliance_extras.is_empty());
        assert_eq!(fingerprint_as(&state, V118).unwrap(), old);
        assert_ne!(fingerprint_as(&state, SAVE_VERSION).unwrap(), old);
    }

    #[test]
    fn the_v117_fingerprint_leaves_out_the_system_views() {
        let (mut state, _) = state_with_imperial_regiment();
        let old = fingerprint_as(&state, V117).unwrap();
        rebellion_core::object_views::seed_systems_from_fog(
            &mut state.world,
            &state.movement,
            [&state.fog_alliance, &state.fog_empire],
        );
        state.object_views = state.world.object_views.clone();
        assert!(!state.object_views.alliance_systems.is_empty());
        assert_eq!(fingerprint_as(&state, V117).unwrap(), old);
        assert_ne!(fingerprint_as(&state, SAVE_VERSION).unwrap(), old);
    }

    #[test]
    fn the_v116_fingerprint_leaves_out_the_object_views() {
        // A v116 save had no object_views field, so its fingerprint cannot
        // depend on one; v117's does.
        let (mut state, _) = state_with_imperial_regiment();
        let old = fingerprint_as(&state, MIGRATED_VERSION).unwrap();
        let new = compute_state_fingerprint(&state).unwrap();
        rebellion_core::object_views::seed_from_fog(
            &mut state.world,
            &state.troop_transport,
            &state.movement,
            [&state.fog_alliance, &state.fog_empire],
        );
        state.object_views = state.world.object_views.clone();
        assert_eq!(fingerprint_as(&state, MIGRATED_VERSION).unwrap(), old);
        assert_ne!(compute_state_fingerprint(&state).unwrap(), new);
    }

    #[test]
    fn a_v116_save_with_the_v117_fingerprint_is_rejected() {
        let saves_dir = tmp_dir("v116_tampered");
        let (state, _) = state_with_imperial_regiment();
        let at = write_v116(&saves_dir, &state);
        let path = slot_path(&saves_dir, 0);
        let mut bytes = std::fs::read(&path).unwrap();
        let wrong = compute_state_fingerprint(&state).unwrap();
        bytes[at..at + 8].copy_from_slice(&wrong.value.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();
        let error = load_slot(&saves_dir, 0).expect_err("fingerprint mismatch");
        assert!(error.to_string().contains("fingerprint mismatch"), "{error}");
    }

    #[test]
    fn a_v117_save_keeps_each_sides_views() {
        let saves_dir = tmp_dir("v117_views");
        let (mut state, troop) = state_with_imperial_regiment();
        rebellion_core::object_views::seed_from_fog(
            &mut state.world,
            &state.troop_transport,
            &state.movement,
            [&state.fog_alliance, &state.fog_empire],
        );
        state.object_views = state.world.object_views.clone();
        // A frozen copy keeps an old strength the save must not refresh.
        state.world.troops[troop].regiment_strength = 1;
        save_slot(&saves_dir, 0, "Views", &state, &[]).unwrap();
        let (_, loaded) = load_slot(&saves_dir, 0).unwrap();
        let id = loaded.world.object_views.regiment_id(troop).unwrap();
        match loaded.world.object_views.view(Faction::Alliance, id).unwrap() {
            rebellion_core::object_views::ObjectView::Regiment(view) => {
                assert_eq!(view.strength, 4);
            }
        }
        assert!(!loaded.world.object_views.see_everything);
    }

    /// A save state with one regiment on its first system's planet holding
    /// a withdraw percent of 40, as a battle withdrawal copies it
    /// (`BlockadeSystem::copy_to_garrison`).
    fn state_with_tracked_regiment() -> (SaveState, rebellion_core::ids::TroopKey) {
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let troop = state.world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x1000_0008),
            is_alliance: false,
            regiment_strength: 100,
        });
        state.world.systems[system].ground_units.push(troop);
        state.blockade.set_withdraw_percent(troop, 40);
        (state, troop)
    }

    /// A regiment's withdraw percent (`+0x60`, copied by `FUN_0050c540` and
    /// rolled by `FUN_00504990` when it leaves on its own) must survive a
    /// save, or a reload would let a regiment run a blockade without its
    /// roll (F-021).
    #[test]
    fn a_round_trip_preserves_a_regiments_withdraw_percent() {
        use rebellion_core::blockade::RegimentWithdraw;

        let saves_dir = tmp_dir("v128_withdraw_roundtrip");
        let (state, troop) = state_with_tracked_regiment();

        save_slot(&saves_dir, 0, "Withdraw", &state, &[]).unwrap();
        let (_, loaded) = load_slot(&saves_dir, 0).unwrap();

        assert_eq!(
            loaded.blockade.regiment_withdraw(troop),
            Some(RegimentWithdraw {
                journey_from: None,
                withdraw_percent: 40,
            })
        );
    }

    #[test]
    fn a_v127_save_drops_its_fleet_borne_withdraw_percents() {
        // v127 copied percents into regiments aboard fleets, which the
        // original never rolls (FUN_00504990 runs for the moved object only,
        // ghidra/notes/move-order.md "Re-traced 2026-11-19"); v128 keeps the
        // layout, so a v127 body loads and its entries are dropped.
        let saves_dir = tmp_dir("v127_migration");
        let (state, troop) = state_with_tracked_regiment();
        let body = with_retired_tuning(
            &bincode::serialize(&state).unwrap(),
            RetiredTuning::default(),
        );
        write_body(&saves_dir, &state, V127, &body);

        let (meta, loaded) = load_slot(&saves_dir, 0).expect("a v127 save loads");
        assert_eq!(loaded.blockade.regiment_withdraw(troop), None);
        assert_eq!(
            meta.state_fingerprint,
            compute_state_fingerprint(&loaded).unwrap()
        );
    }

    #[test]
    fn a_v126_save_drops_its_fleet_borne_withdraw_percents_too() {
        // The same entries in a v126 body, which also lacks the system copy
        // bits (v127).
        let saves_dir = tmp_dir("v126_withdraw_migration");
        let (state, troop) = state_with_tracked_regiment();
        write_v126(&saves_dir, &state);

        let (_, loaded) = load_slot(&saves_dir, 0).expect("a v126 save loads");
        assert_eq!(loaded.blockade.regiment_withdraw(troop), None);
    }

    #[test]
    fn a_round_trip_preserves_revolt_and_disaster_timers() {
        let saves_dir = tmp_dir("v15_uprising_round_trip");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        state.uprising.active_uprisings.insert(
            system,
            rebellion_core::uprising::ActiveUprising {
                started_tick: 3,
                next_incident_tick: Some(45),
                next_support_tick: Some(46),
                side: Some(rebellion_core::dat::Faction::Empire),
            },
        );
        state.uprising.next_disaster_tick = Some(250);
        save_slot(&saves_dir, 0, "V15 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("v15 save should load");

        let revolt = &loaded.uprising.active_uprisings[&system];
        assert_eq!(revolt.started_tick, 3);
        assert_eq!(revolt.next_incident_tick, Some(45));
        assert_eq!(revolt.next_support_tick, Some(46));
        assert_eq!(revolt.side, Some(rebellion_core::dat::Faction::Empire));
        assert_eq!(loaded.uprising.next_disaster_tick, Some(250));
    }

    /// A save holding a remote build order and a delivery en route, for the
    /// F-030 layout tests.
    fn state_with_delivery() -> (SaveState, SystemKey, SystemKey) {
        let mut state = minimal_save_state();
        let mut systems = state.world.systems.keys();
        let (origin, destination) = (systems.next().unwrap(), systems.next().unwrap());
        let class = state
            .world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass::default());
        let kind = rebellion_core::manufacturing::BuildableKind::CapitalShip(class);
        state.manufacturing.enqueue(
            origin,
            rebellion_core::manufacturing::QueueItem::new(kind, 10).delivered_to(destination),
        );
        state.deliveries.depart(
            &state.world,
            &[rebellion_core::manufacturing::Departure {
                origin,
                destination,
                tick: 7,
                kind,
                name: None,
            }],
        );
        (state, origin, destination)
    }

    #[test]
    fn a_round_trip_keeps_build_destinations_and_deliveries_en_route() {
        let saves_dir = tmp_dir("v16_delivery_round_trip");
        let (state, origin, destination) = state_with_delivery();
        save_slot(&saves_dir, 0, "V16 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.deliveries, state.deliveries);
        assert_eq!(loaded.deliveries.en_route().len(), 1);
        let queue = loaded
            .manufacturing
            .queue(
                origin,
                rebellion_core::manufacturing::ProductionArea::Shipyard,
            )
            .unwrap();
        assert_eq!(queue.active().unwrap().destination, Some(destination));
    }

    // Each production area keeps its own queue and Destination (v30,
    // FUN_00509670); both survive a save, whose fingerprint reads the state
    // as JSON.
    #[test]
    fn a_round_trip_keeps_each_production_areas_queue_and_destination() {
        use rebellion_core::manufacturing::ProductionArea;
        let saves_dir = tmp_dir("v30_area_round_trip");
        let (mut state, origin, destination) = state_with_delivery();
        state
            .manufacturing
            .set_destination(origin, ProductionArea::TrainingFacility, destination);
        state.manufacturing.enqueue(
            origin,
            rebellion_core::manufacturing::QueueItem::new(
                rebellion_core::manufacturing::BuildableKind::Troop(
                    rebellion_core::ids::DatId::new(0x1000_0001),
                ),
                4,
            ),
        );
        save_slot(&saves_dir, 0, "V30 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(
            loaded
                .manufacturing
                .destination(origin, ProductionArea::TrainingFacility),
            Some(destination)
        );
        let troops = loaded
            .manufacturing
            .queue(origin, ProductionArea::TrainingFacility)
            .unwrap();
        assert_eq!(troops.active().unwrap().destination, Some(destination));
        assert_eq!(loaded.manufacturing.queued_at(origin), 2);
    }

    #[test]
    fn a_round_trip_keeps_special_forces_aboard_a_fleet() {
        // v105: a fleet's special forces aboard (FUN_00536da0(ship, 3)).
        let saves_dir = tmp_dir("v105_special_forces_aboard");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let unit = state
            .world
            .special_forces
            .insert(rebellion_core::world::SpecialForceUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0004),
                is_alliance: true,
                skills: [0; 8],
                on_mission: false,
            });
        let fleet = state.world.fleets.insert(rebellion_core::world::Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: vec![unit],
        });
        state.world.systems[system].fleets.push(fleet);
        save_slot(&saves_dir, 0, "Aboard", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.world.fleets[fleet].special_forces, vec![unit]);
    }

    #[test]
    fn a_round_trip_keeps_the_ship_a_squadron_rides_on() {
        // v106: a squadron's container is a capital ship (FUN_00500b40,
        // FUN_004fe160), kept as its ship's id.
        let saves_dir = tmp_dir("v106_squadron_carrier");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let fleet = state.world.fleets.insert(rebellion_core::world::Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![rebellion_core::world::FighterEntry {
                class: rebellion_core::ids::FighterKey::default(),
                count: 2,
                carrier: Some(7),
                damage: None,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: vec![],
        });
        state.world.systems[system].fleets.push(fleet);
        save_slot(&saves_dir, 0, "Carried", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.world.fleets[fleet].fighters[0].carrier, Some(7));
    }

    #[test]
    fn a_round_trip_keeps_the_ship_a_character_rides_on() {
        // v107: a character's container is a capital ship (FUN_00500b40,
        // FUN_004fe160), kept as its ship's id.
        let saves_dir = tmp_dir("v107_character_ship");
        let mut state = minimal_save_state();
        let character = state
            .world
            .characters
            .insert(rebellion_core::world::Character {
                ship: Some(7),
                ..Default::default()
            });
        save_slot(&saves_dir, 0, "Aboard", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.world.characters[character].ship, Some(7));
    }

    #[test]
    fn a_round_trip_keeps_translate_counterpart_and_the_advisors_schedule() {
        // v108: FUN_004397a0 saves DAT_006b28bc, DAT_006b28b0 (here its
        // 0x1000) and the advisor's +0x168 table.
        let saves_dir = tmp_dir("v108_advisor");
        let mut state = minimal_save_state();
        state.player_agent =
            rebellion_core::agent_automation::PlayerAgent::for_new_game(rebellion_core::world::SeedDifficulty::Medium);
        state.player_agent.set_advisor_state(&[0, 40_000, 3], true);
        save_slot(&saves_dir, 0, "Advisor", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert!(loaded.player_agent.translate());
        assert_eq!(loaded.player_agent.advisor_state(), (&[0, 40_000, 3][..], true));
    }

    #[test]
    fn a_round_trip_keeps_the_agents_advice_list_and_seen_windows() {
        // v109: FUN_004397a0 saves the agent's +0x148 (the last advice's
        // sub-tick), +0x150 (the windows seen, the list built) and +0x154
        // (the advice not yet posted).
        use rebellion_core::agent_advice::{AdviceRecord, AdviceWindow};
        let saves_dir = tmp_dir("v109_advice");
        let mut state = minimal_save_state();
        state.player_agent = rebellion_core::agent_automation::PlayerAgent::for_new_game(
            rebellion_core::world::SeedDifficulty::Easy,
        );
        let records = [
            AdviceRecord {
                number: 1,
                kind: 7,
                key: 10,
            },
            AdviceRecord {
                number: 2,
                kind: 3,
                key: 120,
            },
            AdviceRecord {
                number: 3,
                kind: 3,
                key: 220,
            },
            AdviceRecord {
                number: 4,
                kind: 1,
                key: 140,
            },
        ];
        let posted = state
            .player_agent
            .advice_window_opened(AdviceWindow::Fleet, 7, &records);
        assert_eq!(posted.len(), 2);
        save_slot(&saves_dir, 0, "Advice", &state, &[]).unwrap();

        let (_, mut loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(
            loaded.player_agent.advice_entries(),
            state.player_agent.advice_entries()
        );
        // The Fleet window stays seen and the list built; the period runs
        // from sub-tick 7.
        assert!(
            loaded
                .player_agent
                .advice_window_opened(AdviceWindow::Fleet, 8, &records)
                .is_empty()
        );
        assert!(loaded.player_agent.advice_check(306, &records).is_empty());
        assert_eq!(loaded.player_agent.advice_check(307, &records).len(), 1);
    }

    #[test]
    fn a_round_trip_keeps_mission_members_and_special_force_skills() {
        use rebellion_core::missions::{
            MissionFaction, MissionKind, MissionMember, MissionRequest,
        };
        use rebellion_core::world::{Character, MissionRecord, SpecialForceUnit};
        let saves_dir = tmp_dir("v17_mission_members_round_trip");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let class_id = rebellion_core::ids::DatId::new(0x3c00_0001);
        let lead = state.world.characters.insert(Character {
            is_alliance: true,
            current_system: Some(system),
            ..Default::default()
        });
        let prisoner = state.world.characters.insert(Character {
            is_alliance: true,
            is_captive: true,
            current_system: Some(system),
            ..Default::default()
        });
        let unit = state.world.special_forces.insert(SpecialForceUnit {
            class_dat_id: class_id,
            is_alliance: true,
            skills: [1, 2, 3, 4, 5, 6, 7, 8],
            on_mission: false,
        });
        state.world.systems[system].special_forces.push(unit);
        state.world.mission_records.push(MissionRecord {
            dat_id: rebellion_core::ids::DatId::new(0x5100_0010),
            timer_min_days: 5,
            timer_spread_days: 10,
            repeats: true,
            hidden: false,
            detection_phases: true,
            can_resign: false,
            leave_rule: 1,
            rules: rebellion_core::world::MissionTargetRules {
                container_loss_ends: true,
                explored_container: true,
                target_loss_ends: true,
                ..Default::default()
            },
            members: rebellion_core::world::MissionMemberRules {
                alliance: true,
                empire: false,
                special_force_mask: 0x402,
                character_mask: 0x1_0000,
            },
        });
        let request = MissionRequest {
            kind: MissionKind::Diplomacy,
            faction: MissionFaction::Alliance,
            team: vec![MissionMember::Character(lead)],
            decoys: vec![
                MissionMember::SpecialForce(unit),
                MissionMember::Character(prisoner),
            ],
            target_system: system,
            target_character: None,
            target_object: None,
            tick: 0,
        };
        state
            .missions
            .dispatch_guarded(request, &mut state.world)
            .expect("the mission should dispatch");
        save_slot(&saves_dir, 0, "V17 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        let mission = &loaded.missions.missions()[0];
        assert_eq!(mission.team, vec![MissionMember::Character(lead)]);
        assert_eq!(mission.decoys, vec![MissionMember::SpecialForce(unit)]);
        assert_eq!(mission.captured, vec![MissionMember::Character(prisoner)]);
        let loaded_unit = &loaded.world.special_forces[unit];
        assert_eq!(loaded_unit.skills, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(loaded_unit.on_mission);
        assert_eq!(loaded.world.mission_records, state.world.mission_records);
    }

    #[test]
    fn a_round_trip_keeps_escape_timers_and_escapees_on_the_road() {
        // v35: timer 0x38a and an escapee's trip (ghidra/notes/captive-escape.md).
        use rebellion_core::dat::Faction;
        use rebellion_core::missions::MissionSystem;
        use rebellion_core::world::{Character, ControlKind, MstbEntry, MstbTable};
        let saves_dir = tmp_dir("v35_escape_round_trip");
        let mut state = minimal_save_state();
        let prison = state.world.systems.keys().next().unwrap();
        state.world.systems[prison].control = ControlKind::Controlled(Faction::Empire);
        let mut haven = state.world.systems[prison].clone();
        haven.x += 100;
        haven.control = ControlKind::Controlled(Faction::Alliance);
        state.world.systems.insert(haven);
        state.world.mission_tables.insert(
            "ESCAPETB".into(),
            MstbTable::new(vec![MstbEntry {
                threshold: 0,
                value: 50,
            }]),
        );
        for _ in 0..2 {
            state.world.characters.insert(Character {
                is_alliance: true,
                is_captive: true,
                captured_by: Some(Faction::Empire),
                current_system: Some(prison),
                ..Default::default()
            });
        }
        // Arm both timers, then let only the first escape.
        MissionSystem::advance_escapes(&mut state.missions, &state.world, 0, &[0.0, 0.9]);
        let effects = MissionSystem::advance_escapes(
            &mut state.missions,
            &state.world,
            1,
            &[0.0, 0.0, 0.9, 0.0],
        );
        assert!(!effects.is_empty());
        assert_eq!(state.missions.escape_timers().len(), 1);
        assert_eq!(state.missions.escapees().len(), 1);
        save_slot(&saves_dir, 0, "V35 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(
            loaded.missions.escape_timers(),
            state.missions.escape_timers()
        );
        assert_eq!(loaded.missions.escapees(), state.missions.escapees());
    }

    #[test]
    fn a_round_trip_keeps_research_points_done_flags_and_research_missions() {
        // v36: each side's points (+0x90..+0x98) and Done flags (+0xac..
        // +0xb4) beside its levels; research runs as missions.
        use rebellion_core::missions::{
            MissionFaction, MissionKind, MissionMember, MissionRequest,
        };
        use rebellion_core::research::TechType;
        use rebellion_core::world::Character;
        let saves_dir = tmp_dir("v36_research_round_trip");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let designer = MissionMember::Character(state.world.characters.insert(Character {
            is_alliance: true,
            current_system: Some(system),
            ..Default::default()
        }));
        state.research.alliance.ship = 3;
        state.research.alliance_progress.ship.points = 17;
        state.research.empire_progress.facility.done = true;
        state.missions.dispatch(MissionRequest {
            kind: MissionKind::TroopTrainingResearch,
            faction: MissionFaction::Alliance,
            team: vec![designer],
            decoys: vec![],
            target_system: system,
            target_character: None,
            target_object: None,
            tick: 12,
        });
        save_slot(&saves_dir, 0, "V36 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.research.level(true, TechType::Ship), 3);
        assert_eq!(loaded.research.progress(true, TechType::Ship).points, 17);
        assert!(loaded.research.progress(false, TechType::Facility).done);
        assert!(!loaded.research.progress(true, TechType::Facility).done);
        assert_eq!(
            loaded.missions.missions()[0].kind,
            MissionKind::TroopTrainingResearch
        );
    }

    #[test]
    fn a_round_trip_keeps_the_ai_s_research_planners() {
        // v37: the strategy's research planners (side +0xa8) with their held
        // designer (+0x3c), site (+0x34), target (+0x38), mission (+0x28),
        // unread report and failure count (+0x6c).
        use rebellion_core::ai::{AIState, AiFaction, ResearchPlanner};
        use rebellion_core::research::TechType;
        let saves_dir = tmp_dir("v37_research_planners_round_trip");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let designer = state
            .world
            .characters
            .insert(rebellion_core::world::Character::default());
        let mut ai = AIState::new(AiFaction::Empire);
        ai.next_planner_id = 4;
        ai.research_planners.push(ResearchPlanner {
            id: 3,
            tech: TechType::Facility,
            member: Some(designer),
            site: Some(system),
            target: Some(system),
            mission: Some(9),
            report: Some(rebellion_core::missions::MissionOutcome::Failure),
            failures: 12,
            // v134: the +0x20 bits, past the failure limit with the abort.
            pending: 0x2000_0080,
        });
        state.ai = ai;
        state.research_planner_bits = research_bits(&state.ai, None);
        save_slot(&saves_dir, 0, "V37 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.ai.research_planners, state.ai.research_planners);
        assert_eq!(loaded.ai.research_planners[0].pending, 0x2000_0080);
        assert_eq!(loaded.ai.next_planner_id, 4);
    }

    #[test]
    fn a_round_trip_keeps_the_ai_s_day_and_module_rounds() {
        // v38: the day's order count (+0x420) and the round robin's module
        // (+0x84 -> +0x10). v39: the priority scan's cursor (+0x14), each
        // module's priority (+0x34) and undecided orders, and the mission
        // module's next entry (+0x4c). v40: the yard marks.
        use rebellion_core::ai::{AIAction, AIState, AiFaction, MissionEntry};
        let saves_dir = tmp_dir("v39_ai_modules_round_trip");
        let mut state = minimal_save_state();
        let mut ai = AIState::new(AiFaction::Empire);
        ai.orders_today = 6;
        ai.module_cursor = Some(8);
        ai.priority_cursor = Some(3);
        ai.module_priority[8] = 1;
        ai.module_pending[8] = vec![AIAction::AbortMission { mission_id: 4 }];
        // A pending Scrap (0x200) keeps its target.
        ai.module_pending[1] = vec![AIAction::Scrap {
            team: vec![rebellion_core::scrap::ScrapTarget::Facility(
                rebellion_core::world::FacilityRef::Production(Default::default()),
            )],
        }];
        ai.mission_next = Some(MissionEntry::Group(2));
        // v40: the yard marks the modules keep (record +0x28 & 0x3e00000).
        ai.set_yard_marks(Default::default(), rebellion_core::ai::MARK_MINES);
        state.ai = ai;
        save_slot(&saves_dir, 0, "V40 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.ai.orders_today, 6);
        assert_eq!(loaded.ai.module_cursor, Some(8));
        assert_eq!(loaded.ai.priority_cursor, Some(3));
        assert_eq!(loaded.ai.module_priority, state.ai.module_priority);
        assert_eq!(loaded.ai.module_pending, state.ai.module_pending);
        assert_eq!(loaded.ai.mission_next, Some(MissionEntry::Group(2)));
        assert_eq!(loaded.ai.yard_marks, state.ai.yard_marks);
        assert_eq!(loaded.ai.yard_marks.len(), 1);
    }

    #[test]
    fn a_round_trip_keeps_resign_requests_and_troop_detection() {
        use rebellion_core::missions::{ActiveMission, MissionFaction, MissionKind, MissionMember};
        use rebellion_core::world::{Character, TroopClassDef};
        let saves_dir = tmp_dir("v20_resign_detection_round_trip");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let lead = MissionMember::Character(state.world.characters.insert(Character {
            is_alliance: true,
            current_system: Some(system),
            ..Default::default()
        }));
        let class = rebellion_core::ids::DatId::new(0x1000_0001);
        state.world.troop_classes.insert(
            class,
            TroopClassDef {
                attack_strength: 1,
                defense_strength: 2,
                detection: 7,
                uprising_defense: 0,
            },
        );
        let mut mission = ActiveMission::new(
            0,
            MissionKind::Sabotage,
            MissionFaction::Alliance,
            vec![lead],
            system,
            0,
        );
        mission.resigning = vec![lead];
        // MissionState exposes no mutable missions; build it from its serde form.
        let mut missions = serde_json::to_value(&state.missions).unwrap();
        missions["missions"] = serde_json::json!([mission]);
        state.missions = serde_json::from_value(missions).unwrap();
        save_slot(&saves_dir, 0, "V20 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.missions.missions()[0].resigning, vec![lead]);
        assert_eq!(loaded.world.troop_classes[&class].detection, 7);
    }

    #[test]
    fn a_round_trip_keeps_mission_phases_and_members_in_transit() {
        use rebellion_core::missions::{
            move_member, MissionEffect, MissionFaction, MissionKind, MissionMember, MissionRequest,
            MissionSystem, PHASE_TRANSIT,
        };
        use rebellion_core::tick::TickEvent;
        use rebellion_core::world::Character;
        let saves_dir = tmp_dir("v18_mission_transit_round_trip");
        let mut state = minimal_save_state();
        let mut systems = state.world.systems.keys();
        let (from, to) = (systems.next().unwrap(), systems.next().unwrap());
        let envoy = state.world.characters.insert(Character {
            is_alliance: true,
            current_system: Some(from),
            ..Default::default()
        });
        state
            .missions
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Diplomacy,
                    MissionFaction::Alliance,
                    envoy,
                    to,
                    None,
                    0,
                ),
                &mut state.world,
            )
            .expect("the mission should dispatch");
        let advance = MissionSystem::advance(
            &mut state.missions,
            &state.world,
            &state.uprising,
            &state.research,
            &[TickEvent { tick: 1 }],
            &[],
        );
        for effect in &advance.effects {
            if let MissionEffect::MemberMoved { member, to } = effect {
                move_member(&mut state.world, *member, *to);
            }
        }
        save_slot(&saves_dir, 0, "V18 Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        let (before, after) = (
            &state.missions.missions()[0],
            &loaded.missions.missions()[0],
        );
        assert_eq!(after.phase, PHASE_TRANSIT);
        assert_eq!(
            (
                after.ready,
                after.origin,
                after.speed,
                after.wait_start,
                after.timer_due
            ),
            (before.ready, Some(from), 100, 0, None)
        );
        assert_eq!(loaded.missions.en_route(), state.missions.en_route());
        assert!(loaded.missions.is_en_route(MissionMember::Character(envoy)));
        assert_eq!(loaded.world.characters[envoy].current_system, None);
    }

    #[test]
    fn a_round_trip_keeps_a_mission_s_armed_timer_and_its_seen_loss() {
        use rebellion_core::missions::{
            move_member, MissionEffect, MissionFaction, MissionKind, MissionRequest, MissionSystem,
            PHASE_TIMER,
        };
        use rebellion_core::tick::TickEvent;
        use rebellion_core::world::Character;
        let saves_dir = tmp_dir("mission_timer_round_trip");
        let mut state = minimal_save_state();
        let mut systems = state.world.systems.keys();
        let (from, to) = (systems.next().unwrap(), systems.next().unwrap());
        let envoy = state.world.characters.insert(Character {
            is_alliance: true,
            current_system: Some(from),
            ..Default::default()
        });
        state
            .missions
            .dispatch_guarded(
                MissionRequest::single(
                    MissionKind::Diplomacy,
                    MissionFaction::Alliance,
                    envoy,
                    to,
                    None,
                    0,
                ),
                &mut state.world,
            )
            .expect("the mission should dispatch");
        let step = |state: &mut SaveState, tick: u64| {
            let advance = MissionSystem::advance(
                &mut state.missions,
                &state.world,
                &state.uprising,
                &state.research,
                &[TickEvent { tick }],
                &[],
            );
            for effect in &advance.effects {
                if let MissionEffect::MemberMoved { member, to } = effect {
                    move_member(&mut state.world, *member, *to);
                }
            }
        };
        step(&mut state, 1);
        let arrival = state.missions.en_route()[0].arrival;
        state.world.systems[to].is_destroyed = true;
        step(&mut state, 2);
        step(&mut state, arrival);
        save_slot(&saves_dir, 0, "Timer Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        let (before, after) = (
            &state.missions.missions()[0],
            &loaded.missions.missions()[0],
        );
        assert_eq!(after.phase, PHASE_TIMER);
        assert!(before.timer_due.is_some());
        assert_eq!(after.timer_due, before.timer_due);
        assert!(after.container_loss_seen);
    }

    /// A recruit (`+0x50` bit 1) and an emptied pool (side `+0xb8`) must
    /// survive a save, or a reload would recruit the same character again
    /// and let Recruitment run past its end `0x10` (`FUN_0056b370`).
    #[test]
    fn a_round_trip_keeps_recruits_and_an_emptied_recruit_pool() {
        use rebellion_core::dat::Faction;
        use rebellion_core::world::Character;
        let saves_dir = tmp_dir("recruit_pool_round_trip");
        let mut state = minimal_save_state();
        let recruit = state.world.characters.insert(Character {
            is_empire: true,
            recruited: true,
            ..Default::default()
        });
        state.world.set_recruit_pool_empty(Faction::Empire);
        save_slot(&saves_dir, 0, "Recruit Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert!(loaded.world.characters[recruit].recruited);
        assert!(loaded.world.recruit_pool_empty(Faction::Empire));
        assert!(!loaded.world.recruit_pool_empty(Faction::Alliance));
    }

    /// A Sabotage order's object (`+0x4c`, read by `FUN_00521030`) must
    /// survive a save, or a reload would leave the mission nothing to
    /// destroy (`FUN_005746e0`).
    #[test]
    fn a_round_trip_keeps_a_sabotage_mission_s_target_object() {
        use rebellion_core::missions::{
            MissionFaction, MissionKind, MissionRequest, MissionTarget,
        };
        use rebellion_core::world::{Character, ManufacturingFacilityInstance};
        let saves_dir = tmp_dir("sabotage_target_round_trip");
        let mut state = minimal_save_state();
        let system = state.world.systems.keys().next().unwrap();
        let yard = state
            .world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
                side: rebellion_core::dat::Faction::Empire,
                is_shipyard: true,
            });
        state.world.systems[system]
            .manufacturing_facilities
            .push(yard);
        let spy = state.world.characters.insert(Character {
            is_alliance: true,
            current_system: Some(system),
            recruited: true,
            ..Default::default()
        });
        let target = MissionTarget::ManufacturingFacility(yard);
        state
            .missions
            .dispatch_guarded(
                MissionRequest {
                    target_object: Some(target),
                    ..MissionRequest::single(
                        MissionKind::Sabotage,
                        MissionFaction::Alliance,
                        spy,
                        system,
                        None,
                        0,
                    )
                },
                &mut state.world,
            )
            .expect("the sabotage should dispatch");
        save_slot(&saves_dir, 0, "Sabotage Save", &state, &[]).unwrap();

        let (_, loaded) = load_slot(&saves_dir, 0).expect("the save should load");

        assert_eq!(loaded.missions.missions()[0].target_object, Some(target));
    }

    #[test]
    fn deterministic_hash_order_independent() {
        let mods_forward = vec![
            ("Alpha".to_string(), "1.0".to_string()),
            ("Beta".to_string(), "2.0".to_string()),
        ];
        let mods_reverse = vec![
            ("Beta".to_string(), "2.0".to_string()),
            ("Alpha".to_string(), "1.0".to_string()),
        ];

        assert_eq!(
            compute_mod_hash(&mods_forward),
            compute_mod_hash(&mods_reverse),
            "hash should be order-independent"
        );
    }
}
