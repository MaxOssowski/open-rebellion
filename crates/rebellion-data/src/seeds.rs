//! Seed table loading — populates the `GameWorld` with starting forces.
//!
//! # Overview
//!
//! Nine DAT files (Pattern 3 — SeedGroup/SeedItem) define the starting deployment
//! for a new game.  Each `SeedItem.item_id` encodes the entity class via:
//!
//!   `item_id = (family_byte << 24) | sequential_id`
//!
//! where `family_byte` identifies the entity type (see `Identifiers.cs` in the
//! Metasharp reference).  This module cross-references those ids against the
//! already-loaded class arenas to create fleet/unit/facility instances.
//!
//! # Family byte → entity type mapping
//!
//! | Byte  | Entity type                     |
//! |-------|---------------------------------|
//! | 0x14  | Capital ship class              |
//! | 0x1C  | Fighter squadron class          |
//! | 0x10  | Troop regiment class            |
//! | 0x3C  | Special forces unit class       |
//! | 0x20  | Alliance HQ facility            |
//! | 0x22..0x25 | Defense facility classes   |
//! | 0x28..0x2A | Manufacturing fac. classes |
//! | 0x2C..0x2D | Production fac. classes    |
//! | 0x90/0x92  | System (by `DatId`)          |
//! | 0x00  | Null / placeholder (skip)       |
//!
//! # Placement logic (3-system model)
//!
//! The original game routes fixed seed tables to exactly 3 special systems:
//!
//! - **Coruscant** (Empire HQ): CMUNEFTB (Empire fleet), CMUNCRTB (garrison),
//!   FACLCRTB (facilities). Palpatine + Vader placed here.
//! - **Yavin** (Alliance base): CMUNAFTB group 1 (fleet), CMUNYVTB (garrison).
//!   Luke, Leia, Han, Wedge, Chewbacca, Jan Dodonna placed here.
//! - **Rebel HQ** (random rim system, NOT Yavin): CMUNAFTB group 2 (fleet),
//!   CMUNHQTB (garrison), FACLHQTB (facilities). Mon Mothma placed here.
//!
//! Army tables (CMUNEMTB, CMUNALTB) are distributed across the faction's
//! special systems.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;
use dat_dumper::types::seed_table::SeedTableFile;
use dat_dumper::types::syfc_table::SyfcTableFile;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use rebellion_core::dat::{ExplorationStatus, Faction, SectorGroup};
use rebellion_core::ids::{CapitalShipKey, DatId, FighterKey, SystemKey};
use rebellion_core::world::{
    ControlKind, DefenseFacilityInstance, FighterEntry, Fleet, GameWorld,
    ManufacturingFacilityInstance, ProductionFacilityInstance, SeedOptions, ShipInstance,
    SkillPair, SpecialForceUnit, TroopUnit,
};

use crate::read_dat_file;

// ── Known system sequential IDs (lower 24 bits of the Identifiers.cs compound ids)
//
// The `system_key_map` passed to `apply_seeds` is keyed by the raw `id` field
// from SYSTEMSD.DAT records, which is just the sequential index (lower 24 bits).
// The high byte (family: 0x90 = explored, 0x92 = unexplored) is NOT stored in
// the map.  We therefore use the index portion only for lookup.
//
// From Identifiers.cs:
//   Coruscant = 0x90000109  → sequential index 0x109
//   Yavin     = 0x92000121  → sequential index 0x121

/// Empire HQ — Coruscant sequential system id.
const CORUSCANT_SEQ_ID: u32 = 0x109;
/// Alliance Yavin sequential system id.
const YAVIN_SEQ_ID: u32 = 0x121;

/// Number of starting systems for Empire (HQ + 9 nearest).
/// Used by the legacy proximity model; kept for backward compatibility.
const EMPIRE_STARTING_SYSTEMS: usize = 10;
/// Number of starting systems for Alliance (HQ + 2 nearest).
/// Used by the legacy proximity model; kept for backward compatibility.
const ALLIANCE_STARTING_SYSTEMS: usize = 3;

/// The three special systems that receive fixed seed table assets.
#[derive(Debug, Clone, Copy)]
pub struct SpecialSystems {
    /// Coruscant — Empire HQ. Receives CMUNEFTB, CMUNCRTB, FACLCRTB.
    pub coruscant: SystemKey,
    /// Yavin — Alliance base. Receives CMUNAFTB group 1, CMUNYVTB.
    pub yavin: SystemKey,
    /// Rebel HQ — random rim system (not Yavin). Receives CMUNAFTB group 2,
    /// CMUNHQTB, FACLHQTB. Mon Mothma placed here.
    pub rebel_hq: SystemKey,
    /// The sequential `DatId` of the rebel HQ system (for lookup purposes).
    pub rebel_hq_seq_id: u32,
}

/// Select the 3 special systems for seeding: Coruscant, Yavin, and a random
/// rim system as the Rebel HQ.
///
/// The Rebel HQ is chosen by shuffling all rim systems (`RimInner` + `RimOuter`)
/// and picking the first one that is not Yavin. This matches the original
/// game's behavior of selecting the first eligible rim system from a shuffled
/// system list.
fn select_special_systems<R: Rng + ?Sized>(
    world: &GameWorld,
    system_key_map: &HashMap<u32, SystemKey>,
    rng: &mut R,
) -> Option<SpecialSystems> {
    let coruscant_key = system_key_map.get(&CORUSCANT_SEQ_ID).copied()?;
    let yavin_key = system_key_map.get(&YAVIN_SEQ_ID).copied()?;

    // Collect all rim systems (RimInner + RimOuter) that are not Yavin.
    // Sort by seq_id first to ensure deterministic ordering before the shuffle,
    // since HashMap iteration order is non-deterministic.
    let mut rim_candidates: Vec<(u32, SystemKey)> = system_key_map
        .iter()
        .filter_map(|(&seq_id, &sys_key)| {
            if seq_id == YAVIN_SEQ_ID {
                return None; // Yavin is the Alliance base, not eligible for HQ
            }
            let sys = world.systems.get(sys_key)?;
            let sector = world.sectors.get(sys.sector)?;
            match sector.group {
                SectorGroup::RimInner | SectorGroup::RimOuter => Some((seq_id, sys_key)),
                SectorGroup::Core => None,
            }
        })
        .collect();

    // Sort by seq_id for deterministic ordering before shuffle.
    rim_candidates.sort_by_key(|(seq_id, _)| *seq_id);
    rim_candidates.shuffle(rng);

    let (rebel_hq_seq_id, rebel_hq_key) = rim_candidates.first().copied()?;

    Some(SpecialSystems {
        coruscant: coruscant_key,
        yavin: yavin_key,
        rebel_hq: rebel_hq_key,
        rebel_hq_seq_id,
    })
}

/// Mark the 3 special systems as populated, charted, and fully controlled
/// by their respective factions with 100% support.
fn initialize_special_systems(world: &mut GameWorld, special: &SpecialSystems) {
    // Coruscant — Empire HQ
    if let Some(sys) = world.systems.get_mut(special.coruscant) {
        sys.is_populated = true;
        sys.exploration_status = ExplorationStatus::Explored;
        sys.control = ControlKind::Controlled(Faction::Empire);
        sys.popularity_empire = 1.0;
        sys.popularity_alliance = 0.0;
        sys.is_headquarters = true;
    }

    // Yavin — Alliance base
    if let Some(sys) = world.systems.get_mut(special.yavin) {
        sys.is_populated = true;
        sys.exploration_status = ExplorationStatus::Explored;
        sys.control = ControlKind::Controlled(Faction::Alliance);
        sys.popularity_alliance = 1.0;
        sys.popularity_empire = 0.0;
        sys.is_headquarters = false;
    }

    // Rebel HQ — Alliance headquarters (separate from Yavin)
    if let Some(sys) = world.systems.get_mut(special.rebel_hq) {
        sys.is_populated = true;
        sys.exploration_status = ExplorationStatus::Explored;
        sys.control = ControlKind::Controlled(Faction::Alliance);
        sys.popularity_alliance = 1.0;
        sys.popularity_empire = 0.0;
        sys.is_headquarters = true;
    }
}

// ── Family byte constants ─────────────────────────────────────────────────────

const FAM_CAPITAL_SHIP: u8 = 0x14;
const FAM_FIGHTER: u8 = 0x1C;
const FAM_TROOP: u8 = 0x10;
const FAM_SPECIAL: u8 = 0x3C;
// Defense facilities span 0x22–0x25.
const FAM_DEF_MIN: u8 = 0x22;
const FAM_DEF_MAX: u8 = 0x25;
// Manufacturing facilities span 0x28–0x2A.
const FAM_MFG_MIN: u8 = 0x28;
const FAM_MFG_MAX: u8 = 0x2A;
// Production facilities span 0x2C–0x2D.
const FAM_PROD_MIN: u8 = 0x2C;
const FAM_PROD_MAX: u8 = 0x2D;
/// Alliance HQ building (a special single facility).
const FAM_ALLIANCE_HQ: u8 = 0x20;

// ─────────────────────────────────────────────────────────────────────────────

/// Index built from the loaded capital-ship arena: `dat_id.raw() → CapitalShipKey`.
type CapShipIndex = HashMap<u32, CapitalShipKey>;
/// Index built from the fighter arena: `dat_id.raw() → FighterKey`.
type FighterIndex = HashMap<u32, FighterKey>;

/// Select starting systems for each faction based on proximity to their HQ.
///
/// Empire: Coruscant + N nearest systems (`EMPIRE_STARTING_SYSTEMS` total).
/// Alliance: Yavin + N nearest systems (`ALLIANCE_STARTING_SYSTEMS` total),
/// excluding systems already claimed by the Empire.
///
/// Returns `(empire_systems, alliance_systems)` as sequential `DatId` arrays
/// suitable for passing as `system_rotation` to the seed functions.
#[must_use]
#[expect(
    clippy::implicit_hasher,
    reason = "This API uses the same concrete map type as the loaded world and callers."
)]
///
/// # Panics
/// Panics if a candidate system has NaN popularity, which cannot be ordered.
pub fn select_starting_systems(
    world: &GameWorld,
    system_key_map: &HashMap<u32, SystemKey>,
) -> (Vec<u32>, Vec<u32>) {
    // Get HQ coordinates
    let coruscant_key = system_key_map.get(&CORUSCANT_SEQ_ID).copied();
    let yavin_key = system_key_map.get(&YAVIN_SEQ_ID).copied();

    let coruscant_pos = coruscant_key
        .and_then(|k| world.systems.get(k))
        .map_or((500.0, 500.0), |s| (f64::from(s.x), f64::from(s.y)));
    let yavin_pos = yavin_key
        .and_then(|k| world.systems.get(k))
        .map_or((100.0, 200.0), |s| (f64::from(s.x), f64::from(s.y)));

    // Sort all systems by distance to Coruscant
    let mut empire_candidates: Vec<(u32, f64)> = system_key_map
        .iter()
        .filter_map(|(&seq, &key)| {
            let sys = world.systems.get(key)?;
            let dx = f64::from(sys.x) - coruscant_pos.0;
            let dy = f64::from(sys.y) - coruscant_pos.1;
            Some((seq, (dx * dx + dy * dy).sqrt()))
        })
        .collect();
    empire_candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let empire_systems: Vec<u32> = empire_candidates
        .iter()
        .take(EMPIRE_STARTING_SYSTEMS)
        .map(|(seq, _)| *seq)
        .collect();

    // Alliance: nearest to Yavin, excluding Empire starting systems
    let empire_set: std::collections::HashSet<u32> = empire_systems.iter().copied().collect();
    let mut alliance_candidates: Vec<(u32, f64)> = system_key_map
        .iter()
        .filter_map(|(&seq, &key)| {
            if empire_set.contains(&seq) {
                return None;
            }
            let sys = world.systems.get(key)?;
            let dx = f64::from(sys.x) - yavin_pos.0;
            let dy = f64::from(sys.y) - yavin_pos.1;
            Some((seq, (dx * dx + dy * dy).sqrt()))
        })
        .collect();
    alliance_candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let alliance_systems: Vec<u32> = alliance_candidates
        .iter()
        .take(ALLIANCE_STARTING_SYSTEMS)
        .map(|(seq, _)| *seq)
        .collect();

    (empire_systems, alliance_systems)
}

/// Apply all 9 seed tables to the already-populated `world`.
///
/// Missing files are silently ignored (stripped installs, test environments).
/// Unresolvable `item_ids` are silently skipped with a debug log rather than
/// aborting — the game can start in a degraded state if needed.
///
/// # Errors
/// Returns an error if a present seed table cannot be read or parsed.
#[expect(
    clippy::implicit_hasher,
    reason = "This API uses the same concrete map type as the loaded world and callers."
)]
pub fn apply_seeds(
    gdata_path: &Path,
    world: &mut GameWorld,
    system_key_map: &HashMap<u32, SystemKey>,
    seed_options: &SeedOptions,
) -> anyhow::Result<()> {
    let mut rng = make_seed_rng(seed_options);
    apply_seeds_with_rng(gdata_path, world, system_key_map, seed_options, &mut rng)
}

/// Apply all seed tables with an injected RNG for deterministic tests.
///
/// Uses the 3-system model: Coruscant (Empire HQ), Yavin (Alliance base),
/// and a random rim system as the Rebel HQ. Fixed seed tables are routed
/// only to these 3 systems, matching the original 1998 game behavior.
///
/// # Errors
/// Returns an error if a present seed table cannot be read or parsed.
#[expect(
    clippy::implicit_hasher,
    reason = "This API uses the same concrete map type as the loaded world and callers."
)]
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
pub fn apply_seeds_with_rng<R: Rng + ?Sized>(
    gdata_path: &Path,
    world: &mut GameWorld,
    system_key_map: &HashMap<u32, SystemKey>,
    seed_options: &SeedOptions,
    rng: &mut R,
) -> anyhow::Result<()> {
    // Build class-lookup indices once (avoid repeated arena scans).
    let capship_index: CapShipIndex = world
        .capital_ship_classes
        .iter()
        .map(|(k, v)| (v.dat_id.raw(), k))
        .collect();
    let fighter_index: FighterIndex = world
        .fighter_classes
        .iter()
        .map(|(k, v)| (v.dat_id.raw(), k))
        .collect();

    // Select the 3 special systems. If Coruscant or Yavin are missing from the
    // data files, fall back to the legacy proximity model.
    let Some(special) = select_special_systems(world, system_key_map, rng) else {
        // Fallback: use old proximity model if special systems can't be resolved.
        // This should only happen with incomplete data files.
        return apply_seeds_legacy(
            gdata_path,
            world,
            system_key_map,
            &capship_index,
            &fighter_index,
        );
    };

    // Mark special systems as populated, charted, and controlled.
    initialize_special_systems(world, &special);

    // ── Empire fleet (CMUNEFTB) → Coruscant only ─────────────────────────────
    apply_fleet_seed(
        load_seed(gdata_path, "CMUNEFTB.DAT")?.as_ref(),
        system_key_map,
        &capship_index,
        &fighter_index,
        &[CORUSCANT_SEQ_ID],
        false,
        world,
    );

    // ── Alliance fleet (CMUNAFTB) → group 1 at Yavin, group 2 at Rebel HQ ──
    apply_fleet_seed(
        load_seed(gdata_path, "CMUNAFTB.DAT")?.as_ref(),
        system_key_map,
        &capship_index,
        &fighter_index,
        &[YAVIN_SEQ_ID, special.rebel_hq_seq_id],
        true,
        world,
    );

    // ── Empire army (CMUNEMTB) → Coruscant only ─────────────────────────────
    apply_army_seed(
        load_seed(gdata_path, "CMUNEMTB.DAT")?.as_ref(),
        system_key_map,
        &[CORUSCANT_SEQ_ID],
        false,
        world,
    );

    // ── Alliance army (CMUNALTB) → Yavin + Rebel HQ ────────────────────────
    apply_army_seed(
        load_seed(gdata_path, "CMUNALTB.DAT")?.as_ref(),
        system_key_map,
        &[YAVIN_SEQ_ID, special.rebel_hq_seq_id],
        true,
        world,
    );

    // ── Empire Coruscant garrison (CMUNCRTB) → Coruscant ────────────────────
    apply_garrison_seed(
        load_seed(gdata_path, "CMUNCRTB.DAT")?.as_ref(),
        system_key_map,
        CORUSCANT_SEQ_ID,
        false,
        world,
    );

    // ── Alliance HQ garrison (CMUNHQTB) → Rebel HQ (NOT Yavin) ─────────────
    apply_garrison_seed(
        load_seed(gdata_path, "CMUNHQTB.DAT")?.as_ref(),
        system_key_map,
        special.rebel_hq_seq_id,
        true,
        world,
    );

    // ── Alliance Yavin garrison (CMUNYVTB) → Yavin ──────────────────────────
    apply_garrison_seed(
        load_seed(gdata_path, "CMUNYVTB.DAT")?.as_ref(),
        system_key_map,
        YAVIN_SEQ_ID,
        true,
        world,
    );

    // ── Empire Coruscant facilities (FACLCRTB) → Coruscant ──────────────────
    apply_facility_seed(
        load_seed(gdata_path, "FACLCRTB.DAT")?.as_ref(),
        system_key_map,
        CORUSCANT_SEQ_ID,
        false,
        world,
    );

    // ── Alliance HQ facilities (FACLHQTB) → Rebel HQ (NOT Yavin) ───────────
    apply_facility_seed(
        load_seed(gdata_path, "FACLHQTB.DAT")?.as_ref(),
        system_key_map,
        special.rebel_hq_seq_id,
        true,
        world,
    );

    // ── Character stat rolling + placement ──────────────────────────────────
    roll_character_stats(world, rng);
    place_named_characters(world, &special);

    // ── M5: Procedural control buckets, population, and support ──────────
    let bucket_map = assign_control_buckets(world, &special, seed_options, rng);
    initialize_population(world, seed_options, rng);
    initialize_support(world, &special, seed_options, &bucket_map, rng);

    // ── M6: Energy, raw materials, and procedural facility generation ────
    initialize_energy_and_raw_materials(world, seed_options, rng);
    let syfccr = load_syfc_table(gdata_path, "SYFCCRTB.DAT")?;
    let syfcrm = load_syfc_table(gdata_path, "SYFCRMTB.DAT")?;
    generate_procedural_facilities(world, seed_options, syfccr.as_ref(), syfcrm.as_ref(), rng);

    // ── M7: Maintenance-budget common unit seeding ───────────────────────
    let cmunem = load_seed(gdata_path, "CMUNEMTB.DAT")?;
    let cmunal = load_seed(gdata_path, "CMUNALTB.DAT")?;
    seed_maintenance_budget_units(world, seed_options, cmunem.as_ref(), cmunal.as_ref(), rng);
    seed_low_support_garrisons(world, seed_options, rng);

    // Every seeded special force now exists; roll their skills.
    roll_special_force_skills(world, rng);

    Ok(())
}

/// Legacy seed application using proximity-based fleet spread.
/// Used as fallback when Coruscant or Yavin can't be found in the data.
fn apply_seeds_legacy(
    gdata_path: &Path,
    world: &mut GameWorld,
    system_key_map: &HashMap<u32, SystemKey>,
    capship_index: &CapShipIndex,
    fighter_index: &FighterIndex,
) -> anyhow::Result<()> {
    let (empire_systems, alliance_systems) = select_starting_systems(world, system_key_map);

    apply_fleet_seed(
        load_seed(gdata_path, "CMUNEFTB.DAT")?.as_ref(),
        system_key_map,
        capship_index,
        fighter_index,
        &empire_systems,
        false,
        world,
    );
    apply_fleet_seed(
        load_seed(gdata_path, "CMUNAFTB.DAT")?.as_ref(),
        system_key_map,
        capship_index,
        fighter_index,
        &alliance_systems,
        true,
        world,
    );
    apply_army_seed(
        load_seed(gdata_path, "CMUNEMTB.DAT")?.as_ref(),
        system_key_map,
        &empire_systems,
        false,
        world,
    );
    apply_army_seed(
        load_seed(gdata_path, "CMUNALTB.DAT")?.as_ref(),
        system_key_map,
        &alliance_systems,
        true,
        world,
    );
    apply_garrison_seed(
        load_seed(gdata_path, "CMUNCRTB.DAT")?.as_ref(),
        system_key_map,
        CORUSCANT_SEQ_ID,
        false,
        world,
    );
    // NOTE: Legacy fallback collapses Alliance HQ to Yavin because the 3-system
    // model (which randomizes Rebel HQ) requires Coruscant to be present.
    // This fallback only runs when special systems can't be identified.
    apply_garrison_seed(
        load_seed(gdata_path, "CMUNHQTB.DAT")?.as_ref(),
        system_key_map,
        YAVIN_SEQ_ID,
        true,
        world,
    );
    apply_garrison_seed(
        load_seed(gdata_path, "CMUNYVTB.DAT")?.as_ref(),
        system_key_map,
        YAVIN_SEQ_ID,
        true,
        world,
    );
    apply_facility_seed(
        load_seed(gdata_path, "FACLCRTB.DAT")?.as_ref(),
        system_key_map,
        CORUSCANT_SEQ_ID,
        false,
        world,
    );
    apply_facility_seed(
        load_seed(gdata_path, "FACLHQTB.DAT")?.as_ref(),
        system_key_map,
        YAVIN_SEQ_ID,
        true,
        world,
    );

    Ok(())
}

/// Apply seed tables from pre-loaded file bytes. Same logic as `apply_seeds` but
/// reads from a `HashMap` instead of the filesystem.
///
/// # Errors
/// Returns an error if a supplied seed table cannot be parsed.
#[expect(
    clippy::implicit_hasher,
    reason = "This API uses the same concrete map type as the loaded world and callers."
)]
pub fn apply_seeds_from_files(
    files: &std::collections::HashMap<String, Vec<u8>>,
    world: &mut GameWorld,
    system_key_map: &HashMap<u32, SystemKey>,
    seed_options: &SeedOptions,
) -> anyhow::Result<()> {
    let mut rng = make_seed_rng(seed_options);
    apply_seeds_from_files_with_rng(files, world, system_key_map, seed_options, &mut rng)
}

/// Apply seed tables from pre-loaded file bytes with an injected RNG.
///
/// # Errors
/// Returns an error if a supplied seed table cannot be parsed.
#[expect(
    clippy::implicit_hasher,
    reason = "This API uses the same concrete map type as the loaded world and callers."
)]
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
pub fn apply_seeds_from_files_with_rng<R: Rng + ?Sized>(
    files: &std::collections::HashMap<String, Vec<u8>>,
    world: &mut GameWorld,
    system_key_map: &HashMap<u32, SystemKey>,
    seed_options: &SeedOptions,
    rng: &mut R,
) -> anyhow::Result<()> {
    let capship_index: CapShipIndex = world
        .capital_ship_classes
        .iter()
        .map(|(k, v)| (v.dat_id.raw(), k))
        .collect();
    let fighter_index: FighterIndex = world
        .fighter_classes
        .iter()
        .map(|(k, v)| (v.dat_id.raw(), k))
        .collect();

    // Use 3-system model when possible.
    let Some(special) = select_special_systems(world, system_key_map, rng) else {
        // Fallback to proximity model for incomplete data
        let (empire_systems, alliance_systems) = select_starting_systems(world, system_key_map);
        apply_fleet_seed(
            load_seed_from_files(files, "CMUNEFTB.DAT")?.as_ref(),
            system_key_map,
            &capship_index,
            &fighter_index,
            &empire_systems,
            false,
            world,
        );
        apply_fleet_seed(
            load_seed_from_files(files, "CMUNAFTB.DAT")?.as_ref(),
            system_key_map,
            &capship_index,
            &fighter_index,
            &alliance_systems,
            true,
            world,
        );
        apply_army_seed(
            load_seed_from_files(files, "CMUNEMTB.DAT")?.as_ref(),
            system_key_map,
            &empire_systems,
            false,
            world,
        );
        apply_army_seed(
            load_seed_from_files(files, "CMUNALTB.DAT")?.as_ref(),
            system_key_map,
            &alliance_systems,
            true,
            world,
        );
        apply_garrison_seed(
            load_seed_from_files(files, "CMUNCRTB.DAT")?.as_ref(),
            system_key_map,
            CORUSCANT_SEQ_ID,
            false,
            world,
        );
        // NOTE: WASM legacy fallback collapses Alliance HQ to Yavin (same as filesystem fallback).
        apply_garrison_seed(
            load_seed_from_files(files, "CMUNHQTB.DAT")?.as_ref(),
            system_key_map,
            YAVIN_SEQ_ID,
            true,
            world,
        );
        apply_garrison_seed(
            load_seed_from_files(files, "CMUNYVTB.DAT")?.as_ref(),
            system_key_map,
            YAVIN_SEQ_ID,
            true,
            world,
        );
        apply_facility_seed(
            load_seed_from_files(files, "FACLCRTB.DAT")?.as_ref(),
            system_key_map,
            CORUSCANT_SEQ_ID,
            false,
            world,
        );
        apply_facility_seed(
            load_seed_from_files(files, "FACLHQTB.DAT")?.as_ref(),
            system_key_map,
            YAVIN_SEQ_ID,
            true,
            world,
        );
        return Ok(());
    };

    initialize_special_systems(world, &special);

    apply_fleet_seed(
        load_seed_from_files(files, "CMUNEFTB.DAT")?.as_ref(),
        system_key_map,
        &capship_index,
        &fighter_index,
        &[CORUSCANT_SEQ_ID],
        false,
        world,
    );
    apply_fleet_seed(
        load_seed_from_files(files, "CMUNAFTB.DAT")?.as_ref(),
        system_key_map,
        &capship_index,
        &fighter_index,
        &[YAVIN_SEQ_ID, special.rebel_hq_seq_id],
        true,
        world,
    );
    apply_army_seed(
        load_seed_from_files(files, "CMUNEMTB.DAT")?.as_ref(),
        system_key_map,
        &[CORUSCANT_SEQ_ID],
        false,
        world,
    );
    apply_army_seed(
        load_seed_from_files(files, "CMUNALTB.DAT")?.as_ref(),
        system_key_map,
        &[YAVIN_SEQ_ID, special.rebel_hq_seq_id],
        true,
        world,
    );
    apply_garrison_seed(
        load_seed_from_files(files, "CMUNCRTB.DAT")?.as_ref(),
        system_key_map,
        CORUSCANT_SEQ_ID,
        false,
        world,
    );
    apply_garrison_seed(
        load_seed_from_files(files, "CMUNHQTB.DAT")?.as_ref(),
        system_key_map,
        special.rebel_hq_seq_id,
        true,
        world,
    );
    apply_garrison_seed(
        load_seed_from_files(files, "CMUNYVTB.DAT")?.as_ref(),
        system_key_map,
        YAVIN_SEQ_ID,
        true,
        world,
    );
    apply_facility_seed(
        load_seed_from_files(files, "FACLCRTB.DAT")?.as_ref(),
        system_key_map,
        CORUSCANT_SEQ_ID,
        false,
        world,
    );
    apply_facility_seed(
        load_seed_from_files(files, "FACLHQTB.DAT")?.as_ref(),
        system_key_map,
        special.rebel_hq_seq_id,
        true,
        world,
    );

    roll_character_stats(world, rng);
    place_named_characters(world, &special);

    // M5: Procedural control buckets, population, and support
    let bucket_map = assign_control_buckets(world, &special, seed_options, rng);
    initialize_population(world, seed_options, rng);
    initialize_support(world, &special, seed_options, &bucket_map, rng);

    // M6: Energy, raw materials, and procedural facilities
    initialize_energy_and_raw_materials(world, seed_options, rng);
    let syfccr = load_syfc_table_from_files(files, "SYFCCRTB.DAT")?;
    let syfcrm = load_syfc_table_from_files(files, "SYFCRMTB.DAT")?;
    generate_procedural_facilities(world, seed_options, syfccr.as_ref(), syfcrm.as_ref(), rng);

    // M7: Maintenance-budget common unit seeding
    let cmunem = load_seed_from_files(files, "CMUNEMTB.DAT")?;
    let cmunal = load_seed_from_files(files, "CMUNALTB.DAT")?;
    seed_maintenance_budget_units(world, seed_options, cmunem.as_ref(), cmunal.as_ref(), rng);
    seed_low_support_garrisons(world, seed_options, rng);

    // Every seeded special force now exists; roll their skills.
    roll_special_force_skills(world, rng);

    Ok(())
}

/// A new campaign's galaxy seed from the wall clock, in seconds since the
/// Unix epoch (the browser's `Date.now()`, miniquad `date::now`). The
/// original game is laid out afresh each time it is played (manual p. 49,
/// "the game begins randomly every time you play"; the Alliance HQ starts
/// "at a random system on the Galactic Rim", p. 45), so a new game must not
/// reuse the fixed seed the browser fixtures load with.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A clock reading only needs to vary between games; microseconds fit in u64."
)]
pub fn campaign_seed_from_clock(seconds: f64) -> u64 {
    (seconds.max(0.0) * 1_000_000.0) as u64
}

fn make_seed_rng(seed_options: &SeedOptions) -> Xoshiro256PlusPlus {
    Xoshiro256PlusPlus::seed_from_u64(seed_options.rng_seed.unwrap_or_else(default_seed))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn default_seed() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos() as u64)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Load a seed table file, returning `Ok(None)` if the file doesn't exist.
fn load_seed(gdata_path: &Path, filename: &str) -> anyhow::Result<Option<SeedTableFile>> {
    let path = gdata_path.join(filename);
    if !crate::file_available(&path) {
        return Ok(None);
    }
    let file: SeedTableFile =
        read_dat_file(&path).with_context(|| format!("parsing seed table {filename}"))?;
    Ok(Some(file))
}

/// Load a seed table file from pre-loaded bytes, returning `Ok(None)` if not in the map.
fn load_seed_from_files(
    files: &std::collections::HashMap<String, Vec<u8>>,
    filename: &str,
) -> anyhow::Result<Option<SeedTableFile>> {
    match files.get(filename) {
        Some(data) => {
            let file: SeedTableFile = crate::parse_dat_bytes(data, filename)?;
            Ok(Some(file))
        }
        None => Ok(None),
    }
}

/// Apply a fleet seed table.  Each `SeedGroup` becomes one `Fleet`.
/// `system_rotation` lists the system `DatIds` to cycle through when assigning
/// fleets; if there are more groups than systems the last system is reused.
fn apply_fleet_seed(
    seed: Option<&SeedTableFile>,
    system_key_map: &HashMap<u32, SystemKey>,
    capship_index: &CapShipIndex,
    fighter_index: &FighterIndex,
    system_rotation: &[u32],
    is_alliance: bool,
    world: &mut GameWorld,
) {
    let Some(seed) = seed else { return };

    for (group_idx, group) in seed.groups.iter().enumerate() {
        // Pick the target system for this fleet group.
        let system_dat_id = system_rotation
            .get(group_idx)
            .copied()
            .unwrap_or_else(|| *system_rotation.last().unwrap_or(&CORUSCANT_SEQ_ID));

        let Some(&system_key) = system_key_map.get(&system_dat_id) else {
            continue;
        };

        let mut fleet_capital_ships: Vec<ShipInstance> = Vec::new();
        let mut fleet_fighters: Vec<FighterEntry> = Vec::new();

        for item in &group.items {
            if item.item_id == 0 {
                continue; // null placeholder
            }
            dispatch_fleet_item(
                item.item_id,
                capship_index,
                fighter_index,
                world,
                is_alliance,
                &mut fleet_capital_ships,
                &mut fleet_fighters,
            );
        }

        if fleet_capital_ships.is_empty() && fleet_fighters.is_empty() {
            continue; // Nothing to deploy.
        }

        let fleet = Fleet {
            location: system_key,
            capital_ships: fleet_capital_ships,
            fighters: fleet_fighters,
            is_alliance,
            ..Default::default()
        };
        let fleet_key = world.insert_fleet(fleet);
        world.systems[system_key].fleets.push(fleet_key);
    }
}

/// Apply an army seed table (CMUNEMTB / CMUNALTB).
/// Items can mix capital ships (fleet component) and ground units.
/// All groups are placed at the given system rotation.
fn apply_army_seed(
    seed: Option<&SeedTableFile>,
    system_key_map: &HashMap<u32, SystemKey>,
    system_rotation: &[u32],
    is_alliance: bool,
    world: &mut GameWorld,
) {
    let Some(seed) = seed else { return };

    for (group_idx, group) in seed.groups.iter().enumerate() {
        let system_dat_id = system_rotation
            .get(group_idx)
            .copied()
            .unwrap_or_else(|| *system_rotation.last().unwrap_or(&CORUSCANT_SEQ_ID));

        let Some(&system_key) = system_key_map.get(&system_dat_id) else {
            continue;
        };

        for item in &group.items {
            if item.item_id == 0 {
                continue;
            }
            dispatch_ground_item(item.item_id, system_key, is_alliance, world);
        }
    }
}

/// Apply a single-system garrison seed (CMUNCRTB / CMUNHQTB / CMUNYVTB).
/// The first item in each group is always `0x00000000` (skip it).
///
/// `FUN_0051ae20` (Coruscant, `0x90000109`), `FUN_0051acb0` (Yavin,
/// `0x92000121`) and `FUN_0051ad90` (the Alliance HQ) pass the system itself
/// to `FUN_0051be90` -> `FUN_0051bf30`. That creates a group's first item in
/// the container (`FUN_004f7d50`) and each later item in that first object
/// (`FUN_004f7cf0`). The garrison groups' first item is null
/// (`FUN_004f26e0` gives 0), so every later item is created in the system:
/// regiments on the planet, and fighter squadrons (family `0x1c`) as the
/// planet's squadrons (ghidra/notes/planet-squadrons-in-battle.md).
fn apply_garrison_seed(
    seed: Option<&SeedTableFile>,
    system_key_map: &HashMap<u32, SystemKey>,
    system_dat_id: u32,
    is_alliance: bool,
    world: &mut GameWorld,
) {
    let Some(seed) = seed else { return };
    let Some(&system_key) = system_key_map.get(&system_dat_id) else {
        return;
    };

    for group in &seed.groups {
        for item in &group.items {
            if item.item_id == 0 {
                continue;
            }
            if (item.item_id >> 24) as u8 == FAM_FIGHTER {
                let seq_id = item.item_id & 0x00FF_FFFF;
                if let Some(class) = world
                    .fighter_classes
                    .iter()
                    .find(|(_, class)| class.dat_id.raw() & 0x00FF_FFFF == seq_id)
                    .map(|(key, _)| key)
                {
                    rebellion_core::squadrons::add(
                        &mut world.systems[system_key].fighters,
                        class,
                        1,
                    );
                }
                continue;
            }
            dispatch_ground_item(item.item_id, system_key, is_alliance, world);
        }
    }
}

/// Apply a facility seed table (FACLCRTB / FACLHQTB).
fn apply_facility_seed(
    seed: Option<&SeedTableFile>,
    system_key_map: &HashMap<u32, SystemKey>,
    system_dat_id: u32,
    is_alliance: bool,
    world: &mut GameWorld,
) {
    let Some(seed) = seed else { return };
    let Some(&system_key) = system_key_map.get(&system_dat_id) else {
        return;
    };

    for group in &seed.groups {
        for item in &group.items {
            if item.item_id == 0 {
                continue;
            }
            dispatch_facility_item(item.item_id, system_key, is_alliance, world);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Item dispatch: route an item_id to the correct arena
// ─────────────────────────────────────────────────────────────────────────────

/// Route a fleet-context item (capital ship or fighter) to the right Vec.
///
/// `item_id` is the full compound id `(family_byte << 24) | sequential_id`.
/// The class arenas were loaded from entity DAT files which store only the
/// sequential part in their `id` field, so we strip the high byte before
/// indexing into the lookup maps.
fn dispatch_fleet_item(
    item_id: u32,
    capship_index: &CapShipIndex,
    fighter_index: &FighterIndex,
    world: &GameWorld,
    is_alliance: bool,
    capital_ships: &mut Vec<ShipInstance>,
    fighters: &mut Vec<FighterEntry>,
) {
    let family = (item_id >> 24) as u8;
    let seq_id = item_id & 0x00FF_FFFF;
    match family {
        FAM_CAPITAL_SHIP => {
            if let Some(&class) = capship_index.get(&seq_id) {
                let hull = world
                    .capital_ship_classes
                    .get(class)
                    .map_or(100, |c| c.hull.cast_signed());
                capital_ships.push(ShipInstance::new(class, hull, is_alliance));
            }
        }
        FAM_FIGHTER => {
            if let Some(&class) = fighter_index.get(&seq_id) {
                if let Some(entry) = fighters.iter_mut().find(|e| e.class == class) {
                    entry.count += 1;
                } else {
                    fighters.push(FighterEntry { class, count: 1, carrier: None, damage: None });
                }
            }
        }
        _ => {
            // Other families (troops, facilities) don't belong in a fleet context;
            // silently ignore.
        }
    }
}

/// Route a ground-context item (troop, special forces) to the system's lists.
fn dispatch_ground_item(
    item_id: u32,
    system_key: SystemKey,
    is_alliance: bool,
    world: &mut GameWorld,
) {
    let family = (item_id >> 24) as u8;
    let class_dat_id = DatId::new(item_id);

    match family {
        FAM_TROOP => {
            let unit = TroopUnit {
                class_dat_id,
                is_alliance,
                regiment_strength: 100,
            };
            let key = world.troops.insert(unit);
            world.systems[system_key].ground_units.push(key);
        }
        FAM_SPECIAL => {
            let unit = SpecialForceUnit {
                class_dat_id,
                is_alliance,
                skills: [0; 8],
                on_mission: false,
            };
            let key = world.special_forces.insert(unit);
            world.systems[system_key].special_forces.push(key);
        }
        _ => {
            // Capital ships / facilities in army tables — skip here.
            // Fleet ships in army tables are handled by apply_army_seed if desired.
        }
    }
}

/// Route a facility-context item to the system's facility lists.
fn dispatch_facility_item(
    item_id: u32,
    system_key: SystemKey,
    is_alliance: bool,
    world: &mut GameWorld,
) {
    let family = (item_id >> 24) as u8;
    let class_dat_id = DatId::new(item_id);

    match family {
        FAM_ALLIANCE_HQ => {
            // Alliance HQ is a manufacturing facility (construction yard class).
            let inst = ManufacturingFacilityInstance {
                class_dat_id,
                side: rebellion_core::dat::Faction::Alliance,
                is_shipyard: false,
            };
            let key = world.manufacturing_facilities.insert(inst);
            world.systems[system_key].manufacturing_facilities.push(key);
        }
        f if (FAM_DEF_MIN..=FAM_DEF_MAX).contains(&f) => {
            let inst = DefenseFacilityInstance {
                class_dat_id,
                side: rebellion_core::dat::Faction::of_alliance(is_alliance),
            };
            let key = world.defense_facilities.insert(inst);
            world.systems[system_key].defense_facilities.push(key);
        }
        f if (FAM_MFG_MIN..=FAM_MFG_MAX).contains(&f) => {
            // Family 0x28 = shipyard, 0x29 = training center, 0x2A = construction yard
            let is_shipyard = f == 0x28;
            let inst = ManufacturingFacilityInstance {
                class_dat_id,
                side: rebellion_core::dat::Faction::of_alliance(is_alliance),
                is_shipyard,
            };
            let key = world.manufacturing_facilities.insert(inst);
            world.systems[system_key].manufacturing_facilities.push(key);
        }
        f if (FAM_PROD_MIN..=FAM_PROD_MAX).contains(&f) => {
            // Family 0x2C = mine, 0x2D = refinery (FUN_0056a6f0 places
            // 0x2c000001 as its mine; ghidra/notes/facility-ownership.md).
            let is_mine = f == 0x2C;
            let inst = ProductionFacilityInstance {
                class_dat_id,
                side: rebellion_core::dat::Faction::of_alliance(is_alliance),
                is_mine,
            };
            let key = world.production_facilities.insert(inst);
            world.systems[system_key].production_facilities.push(key);
        }
        _ => {
            // Unknown family in facility table — skip.
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M5: Procedural control buckets, population state, and support initialization
// ─────────────────────────────────────────────────────────────────────────────

/// Political control bucket type for a core system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlBucket {
    AllianceStrong,
    AllianceWeak,
    EmpireStrong,
    EmpireWeak,
    Neutral,
}

/// Assign political control AND support to all non-special core systems using
/// the original bucket algorithm from SDPRTB 7680 (strong%) / 7681 (weak%).
///
/// The original game:
/// 1. Counts all core systems.
/// 2. Computes Alliance strong, Alliance weak, Empire strong, Empire weak as
///    `floor(core_count * percent / 100)`.
/// 3. Subtracts 1 from Empire strong (Coruscant is pre-assigned).
/// 4. Neutral = remainder.
/// 5. Shuffles all systems, then drains buckets in order.
///
/// Returns the bucket assignment map so `initialize_support` can use it.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn assign_control_buckets<R: Rng + ?Sized>(
    world: &mut GameWorld,
    special: &SpecialSystems,
    seed_options: &SeedOptions,
    rng: &mut R,
) -> HashMap<SystemKey, ControlBucket> {
    let diff = seed_options.gnprtb_index();
    let mut bucket_map: HashMap<SystemKey, ControlBucket> = HashMap::new();

    // Collect core systems that are NOT one of the 3 special systems.
    let mut core_systems: Vec<SystemKey> = Vec::new();
    for (sys_key, sys) in &world.systems {
        if sys_key == special.coruscant || sys_key == special.yavin || sys_key == special.rebel_hq {
            continue;
        }
        let Some(sector) = world.sectors.get(sys.sector) else {
            continue;
        };
        if sector.group == SectorGroup::Core {
            core_systems.push(sys_key);
        }
    }

    // Sort for deterministic ordering before shuffle.
    core_systems.sort_by_key(|&k| world.systems.get(k).map_or(0, |s| s.dat_id.raw()));
    core_systems.shuffle(rng);

    let core_count = core_systems.len() as i32;

    // SDPRTB 7680 = strong support percentage, 7681 = weak support percentage.
    let alliance_strong_pct = world.sdprtb.value(7680, diff, Faction::Alliance);
    let alliance_weak_pct = world.sdprtb.value(7681, diff, Faction::Alliance);
    let empire_strong_pct = world.sdprtb.value(7680, diff, Faction::Empire);
    let empire_weak_pct = world.sdprtb.value(7681, diff, Faction::Empire);

    let alliance_strong = (core_count * alliance_strong_pct / 100).max(0) as usize;
    let alliance_weak = (core_count * alliance_weak_pct / 100).max(0) as usize;
    let empire_strong = ((core_count * empire_strong_pct / 100).max(0) as usize).saturating_sub(1);
    let empire_weak = (core_count * empire_weak_pct / 100).max(0) as usize;

    // Drain buckets in order across the shuffled system list.
    let mut idx = 0;
    for _ in 0..alliance_strong {
        if idx >= core_systems.len() {
            break;
        }
        let k = core_systems[idx];
        if let Some(sys) = world.systems.get_mut(k) {
            sys.control = ControlKind::Controlled(Faction::Alliance);
        }
        bucket_map.insert(k, ControlBucket::AllianceStrong);
        idx += 1;
    }
    for _ in 0..alliance_weak {
        if idx >= core_systems.len() {
            break;
        }
        let k = core_systems[idx];
        if let Some(sys) = world.systems.get_mut(k) {
            sys.control = ControlKind::Controlled(Faction::Alliance);
        }
        bucket_map.insert(k, ControlBucket::AllianceWeak);
        idx += 1;
    }
    for _ in 0..empire_strong {
        if idx >= core_systems.len() {
            break;
        }
        let k = core_systems[idx];
        if let Some(sys) = world.systems.get_mut(k) {
            sys.control = ControlKind::Controlled(Faction::Empire);
        }
        bucket_map.insert(k, ControlBucket::EmpireStrong);
        idx += 1;
    }
    for _ in 0..empire_weak {
        if idx >= core_systems.len() {
            break;
        }
        let k = core_systems[idx];
        if let Some(sys) = world.systems.get_mut(k) {
            sys.control = ControlKind::Controlled(Faction::Empire);
        }
        bucket_map.insert(k, ControlBucket::EmpireWeak);
        idx += 1;
    }
    // Remainder: neutral
    while idx < core_systems.len() {
        bucket_map.insert(core_systems[idx], ControlBucket::Neutral);
        idx += 1;
    }

    bucket_map
}

/// Set `is_populated` on systems based on GNPRTB 7730 (core %) and 7731 (rim %).
///
/// Original: core systems have param 7730 = 100 (always populated).
/// Rim systems have param 7731 = 31 (31% chance of populated).
/// Special systems are already marked populated by `initialize_special_systems`.
#[expect(
    clippy::cast_sign_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn initialize_population<R: Rng + ?Sized>(
    world: &mut GameWorld,
    seed_options: &SeedOptions,
    rng: &mut R,
) {
    let diff = seed_options.gnprtb_index();
    let core_pop_pct = world.gnprtb.value(7730, diff) as u32;
    let rim_pop_pct = world.gnprtb.value(7731, diff) as u32;

    // Collect keys + sector info to avoid borrow issues.
    let system_info: Vec<(SystemKey, SectorGroup, bool)> = world
        .systems
        .iter()
        .map(|(k, sys)| {
            let group = world
                .sectors
                .get(sys.sector)
                .map_or(SectorGroup::RimOuter, |s| s.group);
            (k, group, sys.is_populated)
        })
        .collect();

    for (key, group, already_populated) in system_info {
        if already_populated {
            continue; // Special systems already marked.
        }
        let pct = match group {
            SectorGroup::Core => core_pop_pct,
            SectorGroup::RimInner | SectorGroup::RimOuter => rim_pop_pct,
        };
        let populated = if pct >= 100 {
            true
        } else if pct == 0 {
            false
        } else {
            rng.gen_range(0..100) < pct
        };
        if let Some(sys) = world.systems.get_mut(key) {
            sys.is_populated = populated;
        }
    }
}

/// Initialize support (popularity) values for all systems using the bucket map.
///
/// Mapping from original 0-100 support to our dual f32 popularity:
/// - For faction-controlled systems, the controlling faction gets `support / 100.0`
///   and the opposing faction gets `(100 - support) / 100.0`.
/// - For neutral/uncontrolled systems, both factions get `support / 100.0`
///   (centered around 0.5).
///
/// Original formulas:
/// - Core controlled strong: `base(7682) + random(0..extra(7683))` → 60-90 range
/// - Core controlled weak:   `base(7684) + random(0..extra(7685))` → 20-50 range
/// - Core neutral:           `random(0..spread(7764)) + (50 - spread/2)` → ~41-59
/// - Rim populated:          `random(0..spread(7765)) + 50` → 50 (7765=0 in stock)
/// - Unpopulated:            50 (hard-coded neutral)
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn initialize_support<R: Rng + ?Sized>(
    world: &mut GameWorld,
    special: &SpecialSystems,
    seed_options: &SeedOptions,
    bucket_map: &HashMap<SystemKey, ControlBucket>,
    rng: &mut R,
) {
    struct SysInfo {
        key: SystemKey,
        group: SectorGroup,
        is_populated: bool,
        is_special: bool,
    }

    let diff = seed_options.gnprtb_index();

    let core_neutral_spread = world.gnprtb.value(7764, diff).max(0) as u32;
    let rim_pop_spread = world.gnprtb.value(7765, diff).max(0) as u32;

    // Collect system info to avoid borrow issues.
    let sys_infos: Vec<SysInfo> = world
        .systems
        .iter()
        .map(|(k, sys)| {
            let group = world
                .sectors
                .get(sys.sector)
                .map_or(SectorGroup::RimOuter, |s| s.group);
            let is_special = k == special.coruscant || k == special.yavin || k == special.rebel_hq;
            SysInfo {
                key: k,
                group,
                is_populated: sys.is_populated,
                is_special,
            }
        })
        .collect();

    // Support base/extra from SDPRTB 7682-7685 (side-aware).
    let strong_base_a = world.sdprtb.value(7682, diff, Faction::Alliance);
    let strong_extra_a = world.sdprtb.value(7683, diff, Faction::Alliance).max(0);
    let weak_base_a = world.sdprtb.value(7684, diff, Faction::Alliance);
    let weak_extra_a = world.sdprtb.value(7685, diff, Faction::Alliance).max(0);
    let strong_base_e = world.sdprtb.value(7682, diff, Faction::Empire);
    let strong_extra_e = world.sdprtb.value(7683, diff, Faction::Empire).max(0);
    let weak_base_e = world.sdprtb.value(7684, diff, Faction::Empire);
    let weak_extra_e = world.sdprtb.value(7685, diff, Faction::Empire).max(0);

    for info in &sys_infos {
        if info.is_special {
            continue; // Already set to 100% loyalty by initialize_special_systems.
        }

        let bucket = bucket_map.get(&info.key).copied();

        let (pop_a, pop_e) = match info.group {
            SectorGroup::Core => match bucket {
                Some(ControlBucket::AllianceStrong) => {
                    let support = if strong_extra_a > 0 {
                        strong_base_a + rng.gen_range(0..=strong_extra_a)
                    } else {
                        strong_base_a
                    };
                    let s = support.clamp(0, 100) as f32 / 100.0;
                    (s, 1.0 - s)
                }
                Some(ControlBucket::AllianceWeak) => {
                    let support = if weak_extra_a > 0 {
                        weak_base_a + rng.gen_range(0..=weak_extra_a)
                    } else {
                        weak_base_a
                    };
                    let s = support.clamp(0, 100) as f32 / 100.0;
                    (s, 1.0 - s)
                }
                Some(ControlBucket::EmpireStrong) => {
                    let support = if strong_extra_e > 0 {
                        strong_base_e + rng.gen_range(0..=strong_extra_e)
                    } else {
                        strong_base_e
                    };
                    let s = support.clamp(0, 100) as f32 / 100.0;
                    (1.0 - s, s)
                }
                Some(ControlBucket::EmpireWeak) => {
                    let support = if weak_extra_e > 0 {
                        weak_base_e + rng.gen_range(0..=weak_extra_e)
                    } else {
                        weak_base_e
                    };
                    let s = support.clamp(0, 100) as f32 / 100.0;
                    (1.0 - s, s)
                }
                Some(ControlBucket::Neutral) | None => {
                    let support = if core_neutral_spread > 0 {
                        let r = rng.gen_range(0..core_neutral_spread).cast_signed();
                        (50 - core_neutral_spread.cast_signed() / 2 + r).clamp(0, 100) as f32
                            / 100.0
                    } else {
                        0.5
                    };
                    (support, support)
                }
            },
            SectorGroup::RimInner | SectorGroup::RimOuter => {
                if info.is_populated {
                    let support = if rim_pop_spread > 0 {
                        let r = rng.gen_range(0..rim_pop_spread).cast_signed();
                        (50 + r).clamp(0, 100) as f32 / 100.0
                    } else {
                        0.5
                    };
                    (support, support)
                } else {
                    (0.5, 0.5)
                }
            }
        };

        if let Some(sys) = world.systems.get_mut(info.key) {
            sys.popularity_alliance = pop_a;
            sys.popularity_empire = pop_e;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M6: Energy, raw materials, and procedural facility generation
// ─────────────────────────────────────────────────────────────────────────────

/// Load a SYFC table from the filesystem, returning None if missing.
fn load_syfc_table(gdata_path: &Path, filename: &str) -> anyhow::Result<Option<SyfcTableFile>> {
    let path = gdata_path.join(filename);
    if !crate::file_available(&path) {
        return Ok(None);
    }
    let file: SyfcTableFile =
        read_dat_file(&path).with_context(|| format!("parsing syfc table {filename}"))?;
    Ok(Some(file))
}

/// Load a SYFC table from pre-loaded bytes.
fn load_syfc_table_from_files(
    files: &std::collections::HashMap<String, Vec<u8>>,
    filename: &str,
) -> anyhow::Result<Option<SyfcTableFile>> {
    match files.get(filename) {
        Some(data) => {
            let file: SyfcTableFile = crate::parse_dat_bytes(data, filename)?;
            Ok(Some(file))
        }
        None => Ok(None),
    }
}

/// Roll every system's energy and raw materials, populated or not.
///
/// Every system's setup roll sets both values: a Rim system's
/// (`RimSystem_vtbc` `FUN_0056a550`) after its populated roll on both
/// branches, a Core system's (`CoreSystem_vtbc` `FUN_00566d40`) always.
/// The manual shows the tracks under an unpopulated planet ("Endor Has
/// Resources, but Is Unpopulated", p. 49).
///
/// - Core energy `FUN_00559670`: `clamp(7721 + draw(7722), 7713, 7714)`.
/// - Core raw `FUN_005596a0`: `clamp(clamp(7723 + draw(7724), 7711, 7712), 7711, energy)`.
/// - Rim energy `FUN_00559970`: `clamp(7725 + draw(7726) + draw(7727), 7713, 7714)`.
/// - Rim raw `FUN_005599c0`: `clamp(clamp(7728 + draw(7729), 7711, 7712), 7711, energy)`.
///
/// `draw(n)` is `FUN_0053e290`, a draw from `0..=n`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn initialize_energy_and_raw_materials<R: Rng + ?Sized>(
    world: &mut GameWorld,
    seed_options: &SeedOptions,
    rng: &mut R,
) {
    let diff = seed_options.gnprtb_index();
    let param = |id: u16| world.gnprtb.value(id, diff);
    let raw_bounds = (param(7711), param(7712));
    let energy_bounds = (param(7713), param(7714));
    let core = ResourceRoll {
        energy_base: param(7721),
        energy_draws: [param(7722), 0],
        raw_base: param(7723),
        raw_draw: param(7724),
    };
    let rim = ResourceRoll {
        energy_base: param(7725),
        energy_draws: [param(7726), param(7727)],
        raw_base: param(7728),
        raw_draw: param(7729),
    };

    let system_info: Vec<(SystemKey, SectorGroup)> = world
        .systems
        .iter()
        .map(|(k, sys)| {
            let group = world
                .sectors
                .get(sys.sector)
                .map_or(SectorGroup::RimOuter, |s| s.group);
            (k, group)
        })
        .collect();

    for (key, group) in system_info {
        let roll = match group {
            SectorGroup::Core => &core,
            SectorGroup::RimInner | SectorGroup::RimOuter => &rim,
        };
        let (energy, raw) = roll.roll(raw_bounds, energy_bounds, rng);
        if let Some(sys) = world.systems.get_mut(key) {
            sys.total_energy = energy.clamp(0, i32::from(u8::MAX)) as u8;
            sys.raw_materials = raw.clamp(0, i32::from(u8::MAX)) as u8;
        }
    }
}

/// port: gives a save made before every system rolled its resources the
/// energy and raw materials its unpopulated systems never got (save v138
/// and older left them 0/0; [`initialize_energy_and_raw_materials`]).
/// Each system with no energy rolls as a new game would
/// (`FUN_0056a550`, `FUN_00566d40`), from a stream seeded by its id, so
/// the same save always gets the same values. A side's explored copy of it
/// that still shows the 0/0 values takes the new ones, as though it had
/// been seen with them; an unexplored copy stays bare.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Rolled values are clamped to 0..=255 before narrowing."
)]
pub fn roll_missing_resources(world: &mut GameWorld, gnprtb_index: u8) {
    let param = |id: u16| world.gnprtb.value(id, gnprtb_index);
    let raw_bounds = (param(7711), param(7712));
    let energy_bounds = (param(7713), param(7714));
    let core = ResourceRoll {
        energy_base: param(7721),
        energy_draws: [param(7722), 0],
        raw_base: param(7723),
        raw_draw: param(7724),
    };
    let rim = ResourceRoll {
        energy_base: param(7725),
        energy_draws: [param(7726), param(7727)],
        raw_base: param(7728),
        raw_draw: param(7729),
    };
    let missing: Vec<(SystemKey, DatId, SectorGroup)> = world
        .systems
        .iter()
        .filter(|(_, sys)| sys.total_energy == 0 && !sys.is_destroyed)
        .map(|(key, sys)| {
            let group = world
                .sectors
                .get(sys.sector)
                .map_or(SectorGroup::RimOuter, |s| s.group);
            (key, sys.dat_id, group)
        })
        .collect();
    for (key, id, group) in missing {
        let roll = match group {
            SectorGroup::Core => &core,
            SectorGroup::RimInner | SectorGroup::RimOuter => &rim,
        };
        let mut rng =
            Xoshiro256PlusPlus::seed_from_u64(0x5245_534f_5552_4345 ^ u64::from(id.raw()));
        let (energy, raw) = roll.roll(raw_bounds, energy_bounds, &mut rng);
        let energy = energy.clamp(0, i32::from(u8::MAX)) as u8;
        let raw = raw.clamp(0, i32::from(u8::MAX)) as u8;
        if let Some(sys) = world.systems.get_mut(key) {
            sys.total_energy = energy;
            sys.raw_materials = raw;
        }
        let views = &mut world.object_views;
        for (copies, bits) in [
            (&mut views.alliance_systems, &views.system_bits.alliance),
            (&mut views.empire_systems, &views.system_bits.empire),
        ] {
            let explored = bits.get(&id).is_some_and(|b| b.explored);
            if let Some(view) = copies.get_mut(&id) {
                if explored && view.total_energy == 0 {
                    view.total_energy = energy;
                    view.raw_materials = raw;
                }
            }
        }
    }
}

/// One sector group's energy and raw-material roll (GNPRTB 7721..7729).
struct ResourceRoll {
    energy_base: i32,
    /// Core draws once (`FUN_00559670`), Rim twice (`FUN_00559970`).
    energy_draws: [i32; 2],
    raw_base: i32,
    raw_draw: i32,
}

impl ResourceRoll {
    fn roll<R: Rng + ?Sized>(
        &self,
        raw_bounds: (i32, i32),
        energy_bounds: (i32, i32),
        rng: &mut R,
    ) -> (i32, i32) {
        let mut energy = self.energy_base;
        for draw in self.energy_draws {
            energy += original_draw(draw, rng);
        }
        let energy = clamp_original(energy, energy_bounds.0, energy_bounds.1);
        let raw = clamp_original(
            self.raw_base + original_draw(self.raw_draw, rng),
            raw_bounds.0,
            raw_bounds.1,
        );
        (energy, clamp_original(raw, raw_bounds.0, energy))
    }
}

/// `FUN_0053e290(n)`: a draw from `0..=n`, or `n..=0` for a negative `n`.
/// A zero `n` asks `FUN_005f5700` for one value, which takes no draw.
fn original_draw<R: Rng + ?Sized>(n: i32, rng: &mut R) -> i32 {
    if n == 0 {
        0
    } else if n > 0 {
        rng.gen_range(0..=n)
    } else {
        -rng.gen_range(0..=-n)
    }
}

/// `FUN_0053e0f0(value, low, high)`: `FUN_0053e130` takes the larger of
/// value and low, then `FUN_0053e140` the smaller of that and high, so
/// `high` wins when the bounds cross.
fn clamp_original(value: i32, low: i32, high: i32) -> i32 {
    value.max(low).min(high)
}

/// One row of SYFCCRTB/SYFCRMTB.
struct FacilityWeight {
    /// Lowest roll that selects this row.
    threshold: u32,
    /// Packed facility id: `(family_byte << 24) | index`; 0 seeds nothing.
    facility_id: u32,
}

/// Parse SYFC table rows, in file order, into (`threshold`, `facility_id`).
fn parse_facility_weights(table: Option<&SyfcTableFile>) -> Vec<FacilityWeight> {
    let Some(table) = table else {
        return Vec::new();
    };
    table
        .entries
        .iter()
        .map(|e| FacilityWeight {
            // The system_id field is the row's roll threshold.
            threshold: e.system_id,
            facility_id: e.facility,
        })
        .collect()
}

/// Roll a facility class from a seed table. `Some(0)` leaves the slot empty.
fn pick_weighted_facility<R: Rng + ?Sized>(weights: &[FacilityWeight], rng: &mut R) -> Option<u32> {
    // FUN_00559850 rolls FUN_0053e290(100): 0..=100.
    facility_for_roll(weights, rng.gen_range(0u32..=100))
}

/// `FUN_00595090`: the row before the first whose threshold exceeds the roll,
/// or the last row when none does.
fn facility_for_roll(weights: &[FacilityWeight], roll: u32) -> Option<u32> {
    let row = match weights.iter().position(|w| roll < w.threshold) {
        Some(above) => weights.get(above.saturating_sub(1)),
        None => weights.last(),
    };
    row.map(|w| w.facility_id)
}

/// Generate procedural facilities for all populated systems based on energy slots.
///
/// Original algorithm: for each energy slot (`0..total_energy)`:
/// 1. Compute mine chance: `(total_raw - current_mines) * mine_multiplier`
///    where `mine_multiplier` is GNPRTB 7766 (core=4) or 7767 (rim=2).
/// 2. If random(0..100) < `mine_chance`, place a mine (production facility).
/// 3. Otherwise, roll a row of SYFCCRTB (core) or SYFCRMTB (rim); its class-0
///    row leaves the slot empty.
/// 4. Post-adjust: ensure energy >= `used_energy`, raw >= `mine_count`.
#[expect(
    clippy::cast_sign_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn generate_procedural_facilities<R: Rng + ?Sized>(
    world: &mut GameWorld,
    seed_options: &SeedOptions,
    syfccr: Option<&SyfcTableFile>,
    syfcrm: Option<&SyfcTableFile>,
    rng: &mut R,
) {
    let diff = seed_options.gnprtb_index();
    let core_mine_mult = world.gnprtb.value(7766, diff).max(0) as u32; // 4
    let rim_mine_mult = world.gnprtb.value(7767, diff).max(0) as u32; // 2

    let core_weights = parse_facility_weights(syfccr);
    let rim_weights = parse_facility_weights(syfcrm);

    // Collect system info to avoid borrow issues.
    let system_info: Vec<(SystemKey, SectorGroup, u8, u8, bool)> = world
        .systems
        .iter()
        .map(|(k, sys)| {
            let group = world
                .sectors
                .get(sys.sector)
                .map_or(SectorGroup::RimOuter, |s| s.group);
            (
                k,
                group,
                sys.total_energy,
                sys.raw_materials,
                sys.is_populated,
            )
        })
        .collect();

    for (key, group, total_energy, raw_materials, is_populated) in system_info {
        if !is_populated || total_energy == 0 {
            continue;
        }

        let mine_mult = match group {
            SectorGroup::Core => core_mine_mult,
            SectorGroup::RimInner | SectorGroup::RimOuter => rim_mine_mult,
        };
        let weights = match group {
            SectorGroup::Core => &core_weights,
            SectorGroup::RimInner | SectorGroup::RimOuter => &rim_weights,
        };

        let mut mine_count: u32 = 0;

        for _ in 0..total_energy {
            // Mine chance
            let mine_chance = u32::from(raw_materials).saturating_sub(mine_count) * mine_mult;
            let is_mine = mine_chance > 0 && rng.gen_range(0u32..100) < mine_chance;

            if is_mine {
                // FUN_00566de0 / FUN_0056a6f0 place the mine class
                // 0x2c000001. Its side is the system's, set once control is
                // final (assign_facility_sides).
                let inst = ProductionFacilityInstance {
                    class_dat_id: DatId::new(0x2C00_0001),
                    side: rebellion_core::dat::Faction::Neutral,
                    is_mine: true,
                };
                let fac_key = world.production_facilities.insert(inst);
                if let Some(sys) = world.systems.get_mut(key) {
                    sys.production_facilities.push(fac_key);
                }
                mine_count += 1;
            } else if let Some(facility_id) = pick_weighted_facility(weights, rng) {
                // FUN_00566de0 creates nothing for the class-0 row.
                if facility_id != 0 {
                    dispatch_facility_item(facility_id, key, false, world);
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M7: Maintenance-budget common unit seeding
// ─────────────────────────────────────────────────────────────────────────────

/// Each seeded facility takes its system's side: the setup seeders create it
/// with the system's own side bits, a neutral system's facilities belong to
/// neither side (`ghidra/notes/facility-ownership.md`). Run once control is
/// final.
pub(crate) fn assign_facility_sides(world: &mut GameWorld) {
    let systems: Vec<SystemKey> = world.systems.keys().collect();
    for system in systems {
        let holder = GameWorld::facility_holder(world.systems[system].control)
            .unwrap_or(rebellion_core::dat::Faction::Neutral);
        for facility in world.facilities_at(system) {
            world.set_facility_side(facility, holder);
        }
    }
}

/// Compute total maintenance cost of all units belonging to a faction.
fn compute_faction_maintenance(world: &GameWorld, is_alliance: bool) -> u32 {
    let mut total = 0u32;
    for (_, fleet) in &world.fleets {
        if fleet.is_alliance != is_alliance {
            continue;
        }
        for ship in &fleet.capital_ships {
            if ship.alive {
                if let Some(class) = world.capital_ship_classes.get(ship.class) {
                    total += class.maintenance_cost;
                }
            }
        }
        for entry in &fleet.fighters {
            if let Some(class) = world.fighter_classes.get(entry.class) {
                total += class.maintenance_cost * entry.count;
            }
        }
    }
    // Troops have a flat maintenance cost of 1 each in the original.
    for (_, troop) in &world.troops {
        if troop.is_alliance == is_alliance {
            total += 1;
        }
    }
    total
}

/// Compute maintenance cost of a single seed bundle group.
fn compute_bundle_maintenance(
    group: &dat_dumper::types::seed_table::SeedGroup,
    world: &GameWorld,
) -> u32 {
    let capship_index: HashMap<u32, CapitalShipKey> = world
        .capital_ship_classes
        .iter()
        .map(|(k, v)| (v.dat_id.raw(), k))
        .collect();
    let fighter_index: HashMap<u32, FighterKey> = world
        .fighter_classes
        .iter()
        .map(|(k, v)| (v.dat_id.raw(), k))
        .collect();

    let mut cost = 0u32;
    for item in &group.items {
        if item.item_id == 0 {
            continue;
        }
        let family = (item.item_id >> 24) as u8;
        let seq_id = item.item_id & 0x00FF_FFFF;
        match family {
            FAM_CAPITAL_SHIP => {
                if let Some(&class_key) = capship_index.get(&seq_id) {
                    if let Some(class) = world.capital_ship_classes.get(class_key) {
                        cost += class.maintenance_cost;
                    }
                }
            }
            FAM_FIGHTER => {
                if let Some(&class_key) = fighter_index.get(&seq_id) {
                    if let Some(class) = world.fighter_classes.get(class_key) {
                        cost += class.maintenance_cost;
                    }
                }
            }
            FAM_TROOP | FAM_SPECIAL => {
                cost += 1; // Flat troop maintenance
            }
            _ => {}
        }
    }
    cost
}

/// Deploy a bundle group to a system: ships go into a new fleet, troops go to ground.
fn deploy_bundle_to_system<R: Rng + ?Sized>(
    group: &dat_dumper::types::seed_table::SeedGroup,
    system_key: SystemKey,
    is_alliance: bool,
    world: &mut GameWorld,
    _rng: &mut R,
) {
    let capship_index: HashMap<u32, CapitalShipKey> = world
        .capital_ship_classes
        .iter()
        .map(|(k, v)| (v.dat_id.raw(), k))
        .collect();
    let fighter_index: HashMap<u32, FighterKey> = world
        .fighter_classes
        .iter()
        .map(|(k, v)| (v.dat_id.raw(), k))
        .collect();

    let mut fleet_capital_ships: Vec<ShipInstance> = Vec::new();
    let mut fleet_fighters: Vec<FighterEntry> = Vec::new();

    for item in &group.items {
        if item.item_id == 0 {
            continue;
        }
        let family = (item.item_id >> 24) as u8;
        let seq_id = item.item_id & 0x00FF_FFFF;

        match family {
            FAM_CAPITAL_SHIP => {
                if let Some(&class) = capship_index.get(&seq_id) {
                    let hull = world
                        .capital_ship_classes
                        .get(class)
                        .map_or(100, |c| c.hull.cast_signed());
                    fleet_capital_ships.push(ShipInstance::new(class, hull, is_alliance));
                }
            }
            FAM_FIGHTER => {
                if let Some(&class) = fighter_index.get(&seq_id) {
                    if let Some(entry) = fleet_fighters.iter_mut().find(|e| e.class == class) {
                        entry.count += 1;
                    } else {
                        fleet_fighters.push(FighterEntry { class, count: 1, carrier: None, damage: None });
                    }
                }
            }
            FAM_TROOP | FAM_SPECIAL => {
                dispatch_ground_item(item.item_id, system_key, is_alliance, world);
            }
            _ => {}
        }
    }

    // Create fleet if we have ships.
    if !fleet_capital_ships.is_empty() || !fleet_fighters.is_empty() {
        let fleet = Fleet {
            location: system_key,
            capital_ships: fleet_capital_ships,
            fighters: fleet_fighters,
            is_alliance,
            ..Default::default()
        };
        let fleet_key = world.insert_fleet(fleet);
        if let Some(sys) = world.systems.get_mut(system_key) {
            sys.fleets.push(fleet_key);
        }
    }
}

/// SDPRTB row holding each faction's starting maintenance-budget percentage
/// for a galaxy size.
const fn maintenance_budget_param(size: rebellion_core::dat::GalaxySize) -> u16 {
    match size {
        rebellion_core::dat::GalaxySize::Standard => 5168,
        rebellion_core::dat::GalaxySize::Large => 5169,
        rebellion_core::dat::GalaxySize::Huge => 5170,
    }
}

/// Maintenance-budget unit seeding: spend starting budget on random unit bundles.
///
/// The original algorithm:
/// 1. Compute total maintenance from all existing placed assets per faction.
/// 2. Look up available budget percentage from SDPRTB 5168 (standard), 5169 (large),
///    or 5170 (huge) — side-aware and difficulty-aware.
/// 3. Available = `floor(total_maintenance` * `budget_pct` / 100).
/// 4. Repeatedly pick a random owned system, roll a random bundle from CMUNEMTB
///    (Empire) or CMUNALTB (Alliance), deploy if affordable, repeat until exhausted.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn seed_maintenance_budget_units<R: Rng + ?Sized>(
    world: &mut GameWorld,
    seed_options: &SeedOptions,
    cmunem: Option<&SeedTableFile>,
    cmunal: Option<&SeedTableFile>,
    rng: &mut R,
) {
    let diff = seed_options.gnprtb_index();
    let budget_param = maintenance_budget_param(seed_options.galaxy_size);

    // Seed each faction independently.
    for (is_alliance, seed_table) in [(true, cmunal), (false, cmunem)] {
        let Some(table) = seed_table else { continue };
        if table.groups.is_empty() {
            continue;
        }

        let faction = if is_alliance {
            Faction::Alliance
        } else {
            Faction::Empire
        };
        let budget_pct = world.sdprtb.value(budget_param, diff, faction);
        if budget_pct <= 0 {
            continue;
        }

        let existing_maint = compute_faction_maintenance(world, is_alliance);
        let mut budget = (i64::from(existing_maint) * i64::from(budget_pct) / 100).max(0) as u32;

        // Collect eligible owned systems for this faction.
        let mut owned_systems: Vec<SystemKey> = world
            .systems
            .iter()
            .filter(|(_, sys)| sys.control.is_controlled_by(faction) && sys.is_populated)
            .map(|(k, _)| k)
            .collect();
        owned_systems.sort_by_key(|&k| world.systems.get(k).map_or(0, |s| s.dat_id.raw()));

        if owned_systems.is_empty() {
            continue;
        }

        // Spend budget by rolling random bundles.
        let mut attempts = 0;
        let max_attempts = 500; // Safety limit
        while budget > 0 && attempts < max_attempts {
            attempts += 1;

            // Pick random bundle.
            let group_idx = rng.gen_range(0..table.groups.len());
            let group = &table.groups[group_idx];

            // Compute bundle cost.
            let cost = compute_bundle_maintenance(group, world);
            if cost == 0 || cost > budget {
                continue;
            }

            // Pick random owned system.
            let sys_idx = rng.gen_range(0..owned_systems.len());
            let system_key = owned_systems[sys_idx];

            // Deploy.
            deploy_bundle_to_system(group, system_key, is_alliance, world, rng);
            budget = budget.saturating_sub(cost);
        }
    }
}

/// Low-support garrison pass: add troops to controlled systems below the
/// support threshold to prevent immediate uprisings.
///
/// Original formula: if support < threshold (GNPRTB 7761 = 60),
/// add `ceil((threshold - support) / -divisor)` troops (GNPRTB 7762 = -10).
/// i.e., `floor((threshold - support + 9) / 10)` troops.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn seed_low_support_garrisons<R: Rng + ?Sized>(
    world: &mut GameWorld,
    seed_options: &SeedOptions,
    _rng: &mut R,
) {
    struct GarrisonInfo {
        key: SystemKey,
        support_value: i32, // 0-100 original-scale support for the controlling faction
        is_alliance: bool,
    }

    let diff = seed_options.gnprtb_index();
    let threshold = world.gnprtb.value(7761, diff); // 60
    let divisor = world.gnprtb.value(7762, diff).abs().max(1); // 10

    // Alliance regiment DatId: family 0x10, index 2 (ALLIANCE_ARMY_REGIMENT per TheArchitect2018)
    let alliance_regiment = DatId::new(0x1000_0002);
    // Empire regiment DatId: family 0x10, index 8 (IMPERIAL_ARMY_REGIMENT per TheArchitect2018)
    let empire_regiment = DatId::new(0x1000_0008);
    // Guard: if these troop classes don't exist in the data, skip garrison seeding.
    let alliance_exists = world
        .troops
        .values()
        .any(|t| t.class_dat_id == alliance_regiment);
    let empire_exists = world
        .troops
        .values()
        .any(|t| t.class_dat_id == empire_regiment);
    if !alliance_exists && !empire_exists {
        return; // Troop classes not loaded — can't seed garrisons.
    }

    // Collect system info.
    let infos: Vec<GarrisonInfo> = world
        .systems
        .iter()
        .filter_map(|(k, sys)| {
            match sys.control {
                ControlKind::Controlled(Faction::Alliance) => {
                    let support = (sys.popularity_alliance * 100.0) as i32;
                    Some(GarrisonInfo {
                        key: k,
                        support_value: support,
                        is_alliance: true,
                    })
                }
                ControlKind::Controlled(Faction::Empire) => {
                    let support = (sys.popularity_empire * 100.0) as i32;
                    Some(GarrisonInfo {
                        key: k,
                        support_value: support,
                        is_alliance: false,
                    })
                }
                _ => None, // Neutral systems don't get garrisons.
            }
        })
        .collect();

    for info in &infos {
        if info.support_value >= threshold {
            continue; // Support is high enough, no garrison needed.
        }
        let gap = threshold - info.support_value;
        let troops_needed = (gap + divisor - 1) / divisor; // ceil division
        if troops_needed <= 0 {
            continue;
        }

        let class_dat_id = if info.is_alliance {
            alliance_regiment
        } else {
            empire_regiment
        };

        for _ in 0..troops_needed {
            let unit = TroopUnit {
                class_dat_id,
                is_alliance: info.is_alliance,
                regiment_strength: 100,
            };
            let troop_key = world.troops.insert(unit);
            if let Some(sys) = world.systems.get_mut(info.key) {
                sys.ground_units.push(troop_key);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M4: Character stat rolling + named character placement
// ─────────────────────────────────────────────────────────────────────────────

/// Roll concrete stats from SkillPair{base, variance} for all characters.
///
/// After rolling, each `SkillPair` is normalized: `base = rolled_value`, `variance = 0`.
/// This matches the original game's `setup_character()` which generates concrete
/// stats at game start from the DAT base+variance templates.
///
/// Jedi probability is also rolled here: characters with `jedi_probability > 0`
/// have a chance to become Force-sensitive based on their probability value.
fn roll_character_stats<R: Rng + ?Sized>(world: &mut GameWorld, rng: &mut R) {
    for (_, character) in &mut world.characters {
        roll_skill_pair(&mut character.diplomacy, rng);
        roll_skill_pair(&mut character.espionage, rng);
        roll_skill_pair(&mut character.ship_design, rng);
        roll_skill_pair(&mut character.troop_training, rng);
        roll_skill_pair(&mut character.facility_design, rng);
        roll_skill_pair(&mut character.combat, rng);
        roll_skill_pair(&mut character.leadership, rng);
        roll_skill_pair(&mut character.loyalty, rng);

        // FUN_004f11f0 (slot +0x114), after the skills (Role_vt114):
        // ForcePotential is a percent roll on the class jedi_probability
        // (FUN_0053e2f0: rand(0..=99) < p), and FUN_004eee60 then rolls the
        // Force base for a character both potential and aware. The class
        // jedi_level stays the template the later rolls read.
        character.force_potential = rng.gen_range(0..100) < character.jedi_probability;
        if character.force_base_due() {
            let roll = rng.gen_range(0..=character.jedi_level.variance);
            character.roll_force_base(roll);
        }
    }
}

/// Roll each special-forces unit's skills from its class:
/// base + rand(0..=variance) per skill (`FUN_00535e40`). The shipped SPECFCSD
/// variances are all 0, so the shipped data draws nothing.
fn roll_special_force_skills<R: Rng + ?Sized>(world: &mut GameWorld, rng: &mut R) {
    for (_, unit) in &mut world.special_forces {
        let Some(class) = world.special_force_classes.get(&unit.class_dat_id) else {
            continue;
        };
        for (value, template) in unit.skills.iter_mut().zip(class.skills) {
            let mut pair = template;
            roll_skill_pair(&mut pair, rng);
            *value = pair.base;
        }
    }
}

/// Roll a single `SkillPair` into a concrete value: base + random(0..=variance).
/// After rolling, variance is set to 0 so the value is fixed.
fn roll_skill_pair<R: Rng + ?Sized>(pair: &mut SkillPair, rng: &mut R) {
    if pair.variance > 0 {
        let bonus: u32 = rng.gen_range(0..=pair.variance);
        pair.base += bonus;
        pair.variance = 0;
    }
}

/// Place named characters at their starting systems.
///
/// Per the original game:
/// - Luke Skywalker, Leia Organa, Han Solo, Wedge Antilles, Chewbacca,
///   and Jan Dodonna are placed at Yavin.
/// - Mon Mothma is placed at the Rebel HQ (random rim system).
/// - Emperor Palpatine and Darth Vader are placed at Coruscant.
fn place_named_characters(world: &mut GameWorld, special: &SpecialSystems) {
    // Alliance characters at Yavin
    const YAVIN_CHARACTERS: &[&str] = &[
        "Luke Skywalker",
        "Leia Organa",
        "Han Solo",
        "Wedge Antilles",
        "Chewbacca",
        "Jan Dodonna",
    ];

    // Alliance character at Rebel HQ
    const REBEL_HQ_CHARACTERS: &[&str] = &["Mon Mothma"];

    // Empire characters at Coruscant
    const CORUSCANT_CHARACTERS: &[&str] = &["Emperor Palpatine", "Darth Vader"];

    for (_, character) in &mut world.characters {
        let name = character.name.as_str();

        if YAVIN_CHARACTERS.contains(&name) {
            character.current_system = Some(special.yavin);
        } else if REBEL_HQ_CHARACTERS.contains(&name) {
            character.current_system = Some(special.rebel_hq);
        } else if CORUSCANT_CHARACTERS.contains(&name) {
            character.current_system = Some(special.coruscant);
        }
    }
}

#[cfg(test)]
#[path = "seeds_tests.rs"]
mod tests;
