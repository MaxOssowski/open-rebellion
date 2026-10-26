//! Runtime simulation types for the game world.
//!
//! These are the "layer 2" types that game logic, rendering, and save/load operate on.
//! They use slotmap keys for entity references (stable, arena-backed handles)
//! and rich enums for state rather than raw bytes.
//!
//! The `GameWorld` struct is the root of the entire simulation state.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dat::{ExplorationStatus, Faction, GalaxySize};
use crate::ids::{
    CapitalShipKey, CharacterKey, DatId, DefenseFacilityKey, FighterKey, FleetKey,
    ManufacturingFacilityKey, ProductionFacilityKey, SectorKey, SpecialForceKey, SystemKey,
    TroopKey,
};

/// Control state of a star system.
///
/// Maps to the 2-bit `faction_side` field at `entity+0x24 bits 6-7` in REBEXE.EXE:
/// 0=Uncontrolled, 1=Alliance, 2=Empire, 3=Contested.
/// Extended with `Uprising` for active uprising state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ControlKind {
    /// No faction controls this system (neutral / unclaimed).
    #[default]
    Uncontrolled,
    /// A single faction holds this system.
    Controlled(crate::dat::Faction),
    /// Both factions have military presence — active engagement.
    Contested,
    /// An uprising is in progress — faction control is unstable.
    Uprising(crate::dat::Faction),
}

impl ControlKind {
    /// Returns the controlling faction, if any single faction controls.
    #[must_use]
    pub fn faction(&self) -> Option<crate::dat::Faction> {
        match self {
            ControlKind::Controlled(f) | ControlKind::Uprising(f) => Some(*f), // still nominally controlled
            _ => None,
        }
    }

    /// True if the given faction controls this system (including during uprising).
    #[must_use]
    pub fn is_controlled_by(&self, faction: crate::dat::Faction) -> bool {
        self.faction() == Some(faction)
    }
}

/// New-game seeding difficulty.
///
/// This stays in `rebellion-core` so headless crates can share setup values
/// without depending on rendering/UI enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SeedDifficulty {
    Easy,
    #[default]
    Medium,
    Hard,
}

impl SeedDifficulty {
    /// Convert to the original GNPRTB difficulty column for the chosen player side.
    #[must_use]
    pub fn gnprtb_index(self, player_faction: Faction) -> u8 {
        match (player_faction, self) {
            (Faction::Alliance | Faction::Neutral, SeedDifficulty::Medium) => 2,
            (Faction::Alliance | Faction::Neutral, SeedDifficulty::Hard) => 3,
            (Faction::Empire, SeedDifficulty::Easy) => 4,
            (Faction::Empire, SeedDifficulty::Medium) => 5,
            (Faction::Empire, SeedDifficulty::Hard) => 6,
            (Faction::Alliance | Faction::Neutral, SeedDifficulty::Easy) => 1,
        }
    }

    /// Recover the difficulty tier from an existing world's side-aware
    /// GNPRTB column. This is used only when migrating saves created before
    /// campaign setup was persisted explicitly.
    #[must_use]
    pub fn from_gnprtb_index(index: u8) -> Self {
        match index {
            1 | 4 => Self::Easy,
            3 | 6 => Self::Hard,
            _ => Self::Medium,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Medium => "Intermediate",
            Self::Hard => "Expert",
        }
    }
}

/// Original new-game victory-condition selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VictoryConditions {
    /// Capturing the enemy headquarters only wins after the faction's two
    /// principal leaders are also held captive.
    #[default]
    Standard,
    /// Capturing the enemy headquarters is sufficient by itself.
    HeadquartersOnly,
}

impl VictoryConditions {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard Game",
            Self::HeadquartersOnly => "Headquarters Only",
        }
    }
}

/// Setup choices that remain part of a live campaign after one-time seeding.
///
/// Unlike [`SeedOptions`], this record intentionally excludes the random seed:
/// the simulation RNG state is persisted separately by the save system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignConfig {
    pub galaxy_size: GalaxySize,
    pub difficulty: SeedDifficulty,
    pub player_faction: Faction,
    pub victory_conditions: VictoryConditions,
}

impl Default for CampaignConfig {
    fn default() -> Self {
        Self {
            galaxy_size: GalaxySize::Standard,
            difficulty: SeedDifficulty::Medium,
            player_faction: Faction::Alliance,
            victory_conditions: VictoryConditions::Standard,
        }
    }
}

impl CampaignConfig {
    #[must_use]
    pub fn from_seed_options(options: SeedOptions, victory_conditions: VictoryConditions) -> Self {
        Self {
            galaxy_size: options.galaxy_size,
            difficulty: options.difficulty,
            player_faction: options.player_faction,
            victory_conditions,
        }
    }

    /// Best-effort migration for saves that predate explicit campaign setup.
    /// Galaxy size and game type were not recoverable from those bodies.
    #[must_use]
    pub fn from_legacy_world(world: &GameWorld, player_is_alliance: bool) -> Self {
        Self {
            galaxy_size: GalaxySize::Standard,
            difficulty: SeedDifficulty::from_gnprtb_index(world.difficulty_index),
            player_faction: if player_is_alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            },
            victory_conditions: VictoryConditions::Standard,
        }
    }

    #[must_use]
    pub fn summary(self) -> String {
        let galaxy_size = match self.galaxy_size {
            GalaxySize::Standard => "Small Galaxy",
            GalaxySize::Large => "Medium Galaxy",
            GalaxySize::Huge => "Large Galaxy",
        };
        format!(
            "{}, {}, {}",
            self.difficulty.label(),
            galaxy_size,
            self.victory_conditions.label()
        )
    }
}

/// New-game setup values that influence one-time campaign seeding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedOptions {
    pub galaxy_size: GalaxySize,
    pub difficulty: SeedDifficulty,
    pub player_faction: Faction,
    /// Optional deterministic seed for startup randomization.
    pub rng_seed: Option<u64>,
}

impl Default for SeedOptions {
    fn default() -> Self {
        Self {
            galaxy_size: GalaxySize::Standard,
            difficulty: SeedDifficulty::Medium,
            player_faction: Faction::Alliance,
            rng_seed: None,
        }
    }
}

impl SeedOptions {
    #[must_use]
    pub fn gnprtb_index(self) -> u8 {
        self.difficulty.gnprtb_index(self.player_faction)
    }
}

/// A star system in the galaxy — the atomic unit of territory and production.
///
/// Systems belong to a sector and hold all surface assets (facilities, ground units)
/// as well as the fleets in orbit or departing from them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    /// Original .DAT identifier, preserved for round-trip serialization.
    pub dat_id: DatId,
    pub name: String,
    /// The sector this system belongs to.
    pub sector: SectorKey,
    /// Galactic map X coordinate (in sector-relative units).
    pub x: u16,
    /// Galactic map Y coordinate (in sector-relative units).
    pub y: u16,
    /// Whether this system has been explored (from SYSTEMSD `family_id`).
    /// Unexplored systems reveal name only; facilities and units are hidden.
    pub exploration_status: ExplorationStatus,
    /// Alliance popularity fraction in [0.0, 1.0].
    pub popularity_alliance: f32,
    /// Empire popularity fraction in [0.0, 1.0].
    pub popularity_empire: f32,
    /// True if the original startup generator considers this system populated.
    #[serde(default)]
    pub is_populated: bool,
    /// Planetary energy capacity used by initial facility generation.
    #[serde(default)]
    pub total_energy: u8,
    /// Planetary raw-material capacity used by initial facility generation.
    #[serde(default)]
    pub raw_materials: u8,
    /// System espionage counter-intelligence rating. Subtracted from incite
    /// uprising and espionage mission probability calculations.
    /// Populated from SYSTEMSD.DAT field; defaults to 0 (no counter-intel).
    #[serde(default)]
    pub espionage_rating: f32,
    /// Fleets currently orbiting this system. Transit lives in `MovementState`.
    pub fleets: Vec<FleetKey>,
    /// Ground troop units stationed on the surface.
    pub ground_units: Vec<TroopKey>,
    /// Fighter squadrons stationed on the planet (`FUN_005039d0`, families
    /// `0x1c..0x1f`), of either side: a squadron's side is its class's.
    #[serde(default)]
    pub fighters: Vec<FighterEntry>,
    /// Special forces units assigned to this system.
    pub special_forces: Vec<SpecialForceKey>,
    /// Planetary shields, turbolaser batteries, and similar fixed defenses.
    pub defense_facilities: Vec<DefenseFacilityKey>,
    /// Shipyards and troop training centers.
    pub manufacturing_facilities: Vec<ManufacturingFacilityKey>,
    /// Mines, refineries, and other resource extractors.
    pub production_facilities: Vec<ProductionFacilityKey>,
    /// True while this system contains a surviving faction headquarters.
    ///
    /// The Empire must destroy the mobile Alliance headquarters before taking
    /// its system. The Alliance instead captures and holds Coruscant. The flag
    /// is cleared when bombardment destroys the Alliance-HQ facility.
    pub is_headquarters: bool,
    /// True if this system's planet has been destroyed (Death Star fired; `alive_flag` bit0 == 0).
    ///
    /// From RE: the Death Star fires when the target's `alive_flag` bit0 == 0 — inverted from
    /// normal combat units. A destroyed planet cannot produce resources or be colonized.
    pub is_destroyed: bool,
    /// Control state of this system — who holds it and whether it's contested.
    ///
    /// Derived from the 2-bit `faction_side` field (`entity+0x24 bits 6-7`):
    /// 0 = neutral, 1 = Alliance, 2 = Empire, 3 = contested.
    pub control: ControlKind,
    /// `+0x78` bits 6..7: the side of the last fleet (`0x08..0x0f`) or
    /// squadron (`0x1c..0x1f`) to join the system (slot `+0xd0`,
    /// `FUN_00508660` -> `FUN_0050a1b0`); `None` for 0. A battle with no
    /// winner goes to this side (`FUN_0040c780`, `FUN_00516ef0`). See
    /// [`System::unit_joined`].
    #[serde(default)]
    pub last_arrival: Option<crate::dat::Faction>,
}

impl System {
    /// `FUN_00508660` (slot `+0xd0`) with a fleet or squadron of the side
    /// `is_alliance` joining: `FUN_0050a1b0` stores its side in `+0x78`
    /// bits 6..7.
    pub fn unit_joined(&mut self, is_alliance: bool) {
        self.last_arrival = Some(if is_alliance {
            crate::dat::Faction::Alliance
        } else {
            crate::dat::Faction::Empire
        });
    }
}

/// A sector — a named galactic region containing multiple star systems.
///
/// Sectors are the strategic layer above systems: capturing a sector
/// shifts diplomatic and morale values across all its systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sector {
    /// Original .DAT identifier.
    pub dat_id: DatId,
    pub name: String,
    /// Galactic region (Core, Inner Rim, Outer Rim).
    pub group: crate::dat::SectorGroup,
    /// Map X coordinate of the sector's representative position.
    pub x: u16,
    /// Map Y coordinate of the sector's representative position.
    pub y: u16,
    /// All systems within this sector.
    pub systems: Vec<SystemKey>,
}

/// Class definition for a capital ship — a template, not a unit instance.
///
/// Individual hulls are `ShipInstance` records in `Fleet::capital_ships`, each
/// carrying a `CapitalShipKey` back to this class definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalShipClass {
    pub dat_id: DatId,
    pub name: String,
    /// True if this class is buildable/usable by the Rebel Alliance.
    pub is_alliance: bool,
    /// True if this class is buildable/usable by the Empire.
    pub is_empire: bool,
    pub refined_material_cost: u32,
    pub maintenance_cost: u32,
    /// Position in the tech tree (lower = earlier unlock).
    pub research_order: u32,
    pub research_difficulty: u32,
    pub hull: u32,
    pub shield_strength: u32,
    pub sub_light_engine: u32,
    pub maneuverability: u32,
    pub hyperdrive: u32,
    /// Number of fighter squadrons this ship can carry.
    pub fighter_capacity: u32,
    /// Number of troop units this ship can transport.
    pub troop_capacity: u32,

    // ── Combat stats (from CAPSHPSD.DAT — needed for War Machine) ────────────
    /// Sensor range for detecting enemy units.
    pub detection: u32,
    /// Turbolaser batteries per arc (fore/aft/port/starboard).
    pub turbolaser_fore: u32,
    pub turbolaser_aft: u32,
    pub turbolaser_port: u32,
    pub turbolaser_starboard: u32,
    /// Ion cannon batteries per arc.
    pub ion_cannon_fore: u32,
    pub ion_cannon_aft: u32,
    pub ion_cannon_port: u32,
    pub ion_cannon_starboard: u32,
    /// Laser cannon batteries per arc.
    pub laser_cannon_fore: u32,
    pub laser_cannon_aft: u32,
    pub laser_cannon_port: u32,
    pub laser_cannon_starboard: u32,
    /// Shield recharge rate per combat round.
    pub shield_recharge_rate: u32,
    /// Hull repair rate per combat round.
    pub damage_control: u32,
    /// Orbital bombardment attack stat (used in bombardment formula §4).
    pub bombardment_modifier: u32,

    // ── Extended combat stats (DAT fields promoted for full combat parity) ──
    /// Aggregate attack power (sum of all arcs × attack strength). DAT offset: `overall_attack_strength`.
    #[serde(default)]
    pub overall_attack_strength: u32,
    /// Weapon energy recharge rate per combat round. DAT offset: `weapon_recharge_rate`.
    #[serde(default)]
    pub weapon_recharge_rate: u32,
    /// Per-weapon-type attack strength scalars. DAT offsets: `turbolaser_attack_strength`,
    /// `ion_cannon_attack_strength`, `laser_cannon_attack_strength`.
    #[serde(default)]
    pub turbolaser_attack_strength: u32,
    #[serde(default)]
    pub ion_cannon_attack_strength: u32,
    #[serde(default)]
    pub laser_cannon_attack_strength: u32,
    /// Per-weapon-type engagement ranges. DAT offsets: `turbolaser_range`,
    /// `ion_cannon_range`, `laser_cannon_range`.
    #[serde(default)]
    pub turbolaser_range: u32,
    #[serde(default)]
    pub ion_cannon_range: u32,
    #[serde(default)]
    pub laser_cannon_range: u32,
    /// Tractor beam stats for interception/capture mechanics. DAT offsets: `tractor_beam_power`,
    /// `tractor_beam_range`.
    #[serde(default)]
    pub tractor_beam_power: u32,
    #[serde(default)]
    pub tractor_beam_range: u32,
    /// Gravity well projector strength (Interdictor-class — prevents hyperspace escape).
    /// DAT offset: `gravity_well_projector`.
    #[serde(default)]
    pub gravity_well_projector: u32,
    /// Interdiction field strength. DAT offset: `interdiction_strength`.
    #[serde(default)]
    pub interdiction_strength: u32,
    /// Bombardment/uprising suppression defense value. DAT offset: `uprising_defense`.
    #[serde(default)]
    pub uprising_defense: u32,
    /// Hyperdrive rating when the ship has taken hull damage. DAT offset: `hyperdrive_if_damaged`.
    #[serde(default)]
    pub hyperdrive_if_damaged: u32,
}

/// CAPSHPSD.DAT record id of the Death Star (TEXTSTRA 10120 "Death Star").
pub const DEATH_STAR_CLASS_ID: u32 = 136;

impl CapitalShipClass {
    /// True for the Death Star class: the CAPSHPSD record whose object
    /// family (record `+0x10`) is `0x18`, where every other record has
    /// `0x14`. The original finds a fleet's Death Stars by that family
    /// (`FUN_005003d0`, families `0x18..0x1b`; `FUN_0040b0e0` ->
    /// `FUN_0040bcd0`). In the shipped CAPSHPSD only record 136 (`0x88`) has
    /// it.
    ///
    /// port: the class keeps its record id, not the family, so the record
    /// id stands for it. Family `0x34` is the Emperor's (MJCHARSD
    /// `0x34000280`), not a Death Star (F-025).
    #[must_use]
    pub fn is_death_star(&self) -> bool {
        self.dat_id == DatId::new(DEATH_STAR_CLASS_ID)
    }
}

impl Default for CapitalShipClass {
    fn default() -> Self {
        Self {
            dat_id: DatId::new(0),
            name: String::new(),
            is_alliance: false,
            is_empire: false,
            refined_material_cost: 0,
            maintenance_cost: 0,
            research_order: 0,
            research_difficulty: 0,
            hull: 0,
            shield_strength: 0,
            sub_light_engine: 0,
            maneuverability: 0,
            hyperdrive: 0,
            fighter_capacity: 0,
            troop_capacity: 0,
            detection: 0,
            turbolaser_fore: 0,
            turbolaser_aft: 0,
            turbolaser_port: 0,
            turbolaser_starboard: 0,
            ion_cannon_fore: 0,
            ion_cannon_aft: 0,
            ion_cannon_port: 0,
            ion_cannon_starboard: 0,
            laser_cannon_fore: 0,
            laser_cannon_aft: 0,
            laser_cannon_port: 0,
            laser_cannon_starboard: 0,
            shield_recharge_rate: 0,
            damage_control: 0,
            bombardment_modifier: 0,
            overall_attack_strength: 0,
            weapon_recharge_rate: 0,
            turbolaser_attack_strength: 0,
            ion_cannon_attack_strength: 0,
            laser_cannon_attack_strength: 0,
            turbolaser_range: 0,
            ion_cannon_range: 0,
            laser_cannon_range: 0,
            tractor_beam_power: 0,
            tractor_beam_range: 0,
            gravity_well_projector: 0,
            interdiction_strength: 0,
            uprising_defense: 0,
            hyperdrive_if_damaged: 0,
        }
    }
}

/// Class definition for a fighter squadron — template, not an instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FighterClass {
    pub dat_id: DatId,
    pub name: String,
    pub is_alliance: bool,
    pub is_empire: bool,
    pub refined_material_cost: u32,
    pub maintenance_cost: u32,
    /// Position in the tech tree (lower = earlier unlock). DAT offset: `research_order`.
    pub research_order: u32,
    pub research_difficulty: u32,
    /// Number of individual craft in one squadron.
    pub squadron_size: u32,
    pub torpedoes: u32,
    /// Torpedo engagement range. DAT offset: `torpedoes_range`.
    pub torpedoes_range: u32,
    /// Fighter attack stat for combat resolution.
    pub overall_attack_strength: u32,
    /// Bombardment defense modifier.
    pub bombardment_defense: u32,

    // ── Extended fighter stats (DAT fields promoted for combat parity) ───────
    /// Shield strength (fighters rarely have shields but field exists). DAT offset: `shield_strength`.
    #[serde(default)]
    pub shield_strength: u32,
    /// Sub-light engine rating (speed in tactical combat). DAT offset: `sub_light_engine`.
    #[serde(default)]
    pub sub_light_engine: u32,
    /// Maneuverability rating (evasion in combat). DAT offset: `maneuverability`.
    #[serde(default)]
    pub maneuverability: u32,
    /// Sensor detection range. DAT offset: `detection`.
    #[serde(default)]
    pub detection: u32,
    /// Uprising/bombardment suppression defense. DAT offset: `uprising_defense`.
    #[serde(default)]
    pub uprising_defense: u32,
    /// Weapon batteries per arc — fighters typically have fore weapons only.
    /// DAT offsets: `turbolaser_fore`, `ion_cannon_fore`, `laser_cannon_fore`.
    #[serde(default)]
    pub turbolaser_fore: u32,
    #[serde(default)]
    pub ion_cannon_fore: u32,
    #[serde(default)]
    pub laser_cannon_fore: u32,
    /// Per-family tactical engagement ranges. DAT offsets: `turbolaser_range`,
    /// `ion_cannon_range`, and `laser_cannon_range`.
    #[serde(default)]
    pub turbolaser_range: u32,
    #[serde(default)]
    pub ion_cannon_range: u32,
    #[serde(default)]
    pub laser_cannon_range: u32,
    /// Per-weapon-type attack strength scalars. DAT offsets: `turbolaser_attack_strength`,
    /// `ion_cannon_attack_strength`, `laser_cannon_attack_strength`.
    #[serde(default)]
    pub turbolaser_attack_strength: u32,
    #[serde(default)]
    pub ion_cannon_attack_strength: u32,
    #[serde(default)]
    pub laser_cannon_attack_strength: u32,
    /// Hyperdrive rating, the squadron's travel speed (`FUN_00502f80`).
    /// DAT offset: `hyperdrive`.
    #[serde(default)]
    pub hyperdrive: u32,
    /// Hyperdrive rating when damaged, used when `hyperdrive` is 0. DAT
    /// offset: `hyperdrive_if_damaged`.
    #[serde(default)]
    pub hyperdrive_if_damaged: u32,
}

impl Default for FighterClass {
    fn default() -> Self {
        Self {
            dat_id: DatId::new(0),
            name: String::new(),
            is_alliance: false,
            is_empire: false,
            refined_material_cost: 0,
            maintenance_cost: 0,
            research_order: 0,
            research_difficulty: 0,
            squadron_size: 0,
            torpedoes: 0,
            torpedoes_range: 0,
            overall_attack_strength: 0,
            bombardment_defense: 0,
            shield_strength: 0,
            sub_light_engine: 0,
            maneuverability: 0,
            detection: 0,
            uprising_defense: 0,
            turbolaser_fore: 0,
            ion_cannon_fore: 0,
            laser_cannon_fore: 0,
            turbolaser_range: 0,
            ion_cannon_range: 0,
            laser_cannon_range: 0,
            turbolaser_attack_strength: 0,
            ion_cannon_attack_strength: 0,
            laser_cannon_attack_strength: 0,
            hyperdrive: 0,
            hyperdrive_if_damaged: 0,
        }
    }
}

/// A character — either a named major hero/villain or a generic minor character.
///
/// Characters can be assigned as admirals, generals, or diplomats.
/// Their skills are stored as `SkillPair` (base + variance) to support
/// both fixed major characters and procedurally-generated minors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "These independent flags preserve the existing state and serialization model."
)]
pub struct Character {
    pub dat_id: DatId,
    pub name: String,
    pub is_alliance: bool,
    pub is_empire: bool,
    /// Major characters (Luke, Vader, etc.) have fixed identities;
    /// minor characters are generic and reusable.
    pub is_major: bool,
    pub diplomacy: SkillPair,
    pub espionage: SkillPair,
    pub ship_design: SkillPair,
    pub troop_training: SkillPair,
    pub facility_design: SkillPair,
    pub combat: SkillPair,
    pub leadership: SkillPair,
    pub loyalty: SkillPair,
    /// Probability (0–100) this character becomes Force-sensitive.
    pub jedi_probability: u32,
    pub jedi_level: SkillPair,
    pub can_be_admiral: bool,
    pub can_be_commander: bool,
    pub can_be_general: bool,

    // ── DAT-promoted fields ─────────────────────────────────────────────────
    /// Immune to betrayal missions (Luke, Vader). From MJCHARSD.DAT `is_unable_to_betray`.
    #[serde(default)]
    pub is_unable_to_betray: bool,
    /// Can train other Jedi (Yoda). From MJCHARSD.DAT `is_jedi_trainer`.
    #[serde(default)]
    pub is_jedi_trainer: bool,
    /// Publicly known Force user. From MJCHARSD.DAT `is_known_jedi`.
    #[serde(default)]
    pub is_known_jedi: bool,
    /// Fleet speed bonus (Han Solo). Default 0.
    #[serde(default)]
    pub hyperdrive_modifier: i16,
    /// Mission bonus loyalty, 0-100. Default 0.
    #[serde(default)]
    pub enhanced_loyalty: i16,
    /// Currently assigned to a mission.
    #[serde(default)]
    pub on_mission: bool,
    /// Mission concealed from opponent.
    #[serde(default)]
    pub on_hidden_mission: bool,
    /// Story-forced assignment, blocks resignation.
    #[serde(default)]
    pub on_mandatory_mission: bool,

    // ── Captivity ─────────────────────────────────────────────────────────
    /// Faction that captured this character (None if free).
    #[serde(default)]
    pub captured_by: Option<crate::dat::Faction>,
    /// Tick when character was captured (for escape timing).
    #[serde(default)]
    pub capture_tick: Option<u64>,
    /// True if character is currently held captive.
    #[serde(default)]
    pub is_captive: bool,

    // ── Location tracking ───────────────────────────────────────────────────
    /// System where this character is currently located.
    #[serde(default)]
    pub current_system: Option<SystemKey>,
    /// Fleet this character is currently assigned to.
    #[serde(default)]
    pub current_fleet: Option<FleetKey>,

    // ── Story state ────────────────────────────────────────────────────
    /// True once the player has witnessed the Luke–Vader paternity reveal.
    /// Gates the Final Battle BMP variant in the render layer.
    ///
    /// NOTE: No `#[serde(default)]` — bincode is positional and the attribute is
    /// inoperative under bincode. Field additions on `Character` require a save
    /// version bump (see `rebellion-data/src/save.rs`). The v8 bump guards this field.
    pub heritage_known: bool,

    /// True once this character has been killed (Death Star cleanup, assassination).
    /// Drives `EVT_CHARACTER_KILLED` story events on the next tick and provides
    /// built-in uniqueness for death-triggered events (DI-M3 in Knesset Shamash-Bet).
    ///
    /// Killed characters remain in the arena so that their `dat_id` / `name` can
    /// still be looked up by next-tick story events; they are removed from all
    /// fleet rosters immediately at death time.
    ///
    /// NOTE: Like `heritage_known`, this field lands under the v8 save bump — no
    /// `#[serde(default)]` under bincode.
    pub is_killed: bool,

    /// Destroyed by a Retire order (`0x242`), whose reason `0x14` the
    /// Personnel Finder reads as "Retired" (`FUN_00534c00`, `FUN_00465bb0`);
    /// `is_killed` is set too. Save v104.
    pub is_retired: bool,

    /// The side has recruited this character (`+0x50` bit 1). Recruitment
    /// picks among the side's characters without it (`FUN_0055ef30`,
    /// `FUN_0055fc80`) and sets it (`FUN_0055fe70` via `FUN_004f7480`).
    pub recruited: bool,

    /// Character `+0x94`, the injury (`FUN_004ee2d0`, clamped to
    /// `0..=0x7fff`): set by the injury roll `FUN_0053e990` and lowered by
    /// the healing timer `0x388` (`ghidra/notes/character-injury.md`).
    pub injury: u16,

    /// Character `+0xac` bit `0x20`, `ForcePotential` (`FUN_004ee9c0`):
    /// rolled once at creation from the class `jedi_probability`
    /// (`FUN_004f11f0`, `ghidra/notes/force-awareness.md`).
    pub force_potential: bool,
    /// Character `+0xac` bit `0x10`, `ForceAware` (`FUN_004ee950`): the class
    /// `is_known_jedi` at creation (`FUN_004f11f0`).
    pub force_aware: bool,
    /// Character `+0x8e`, the Force base (`FUN_004ee150`, `0..=0x7fff`):
    /// 0 until `FUN_004eee60` rolls it for a character with both
    /// `force_potential` and `force_aware` ([`Character::force_base_due`]).
    pub force_base: u16,
    /// Character `+0xa4`: the trainer who discovered this character's Force
    /// (`FUN_004ee680`, timer `0x389`), by record.
    pub force_discoverer: Option<DatId>,
    /// Character `+0x90`, `ForceTraining` (`FUN_004ee1d0`, `0..=0x7fff`):
    /// what Jedi Training added (`FUN_005712b0`).
    pub force_training: u16,
    /// The abode the character is in, when its container is an abode
    /// (family `0x98`); its system and fleet are then `None` (save v72).
    pub current_abode: Option<crate::ids::AbodeKey>,
    /// Luke's `DagobahCompleted` (`+0xb0` bit `0x8`, `FUN_0056f730`), set at
    /// the Dagobah mission's end (`FUN_005751d0`; save v73).
    pub dagobah_completed: bool,
    /// Character `+0x9c`, the `Encounter` key (`FUN_004ee500`): the Force
    /// user this one detected or was detected by on a mission
    /// (`FUN_0055ff60`), by record, until their confrontation clears it
    /// (`FUN_00560200`; save v74).
    pub encounter: Option<DatId>,
    /// The encounter day (`FUN_004ef1c0`, record `+0x54` -> `+0x34`), 0
    /// without an encounter (save v74).
    pub encounter_day: u64,
    /// `CanEscape` (`+0xac` bit `0x100`) cleared while held: the Final
    /// Battle bars its prisoners from escaping (`FUN_004eeb10(0)`,
    /// `FUN_0054b040`) until a new capture sets it again (`FUN_004f1a80`;
    /// save v76).
    pub escape_barred: bool,
    /// Adrift (`+0x78` bit 6, `Role_SetAdrift` `FUN_00534790`): the
    /// character's container was destroyed under it (`FUN_004ef2e0`), and it
    /// waits for an Adrift order or to settle (`FUN_0054c7d0`; save v93).
    #[serde(default)]
    pub adrift: bool,
    /// Han's `CapturedByBountyHunters` (`+0xb0` bit 2, `FUN_00572700`),
    /// set when the Bounty mission captures him (`FUN_00575de0`; save v77).
    pub bounty_captured: bool,
    /// `CommandKind` (`+0x96`, `FUN_004ee350`): the command the Command
    /// orders give ([`crate::character_command`]; save v85).
    pub command: crate::character_command::CommandKind,
    /// The name for each command, none first: TEXTSTRA of the class record's
    /// name `+0x30` plus 0, `0x6800`, `0x6c00` and `0x7000`, which the
    /// command's hook renames the character to (`FUN_004f17f0`; save v86).
    /// An empty entry keeps the name.
    pub command_names: [String; 4],
}

impl Character {
    /// Slot `+0x1f0`: effective combat `+0x86` (`FUN_004eed90` ->
    /// `FUN_0053e900`): combat plus combat times the Force level (`+0x8c`)
    /// over 100 (`FUN_0053e190`), less the injury, at least 0
    /// (`FUN_0053e120`) and at most `0x7fff` (`FUN_004edf30`).
    ///
    /// port: the combat is the rolled template, base plus half the
    /// variance, as `missions::member_skill` reads skills.
    #[must_use]
    pub fn effective_combat(&self) -> u32 {
        let combat = i64::from(self.combat.base + self.combat.variance / 2);
        let force = i64::from(self.force_level());
        let value = (combat * force / 100 + combat - i64::from(self.injury)).clamp(0, 0x7fff);
        u32::try_from(value).unwrap_or(0)
    }

    /// `+0x8c`, the Force level: `ForceExperience` `+0x8e` (the port's
    /// `force_base`) plus `ForceTraining` `+0x90` (`FUN_004eee40`, run by
    /// both fields' hooks, slots `+0x320` and `+0x324`), clamped to
    /// `0..=0x7fff` (`FUN_004ee0d0`).
    #[must_use]
    pub fn force_level(&self) -> u32 {
        (u32::from(self.force_base) + u32::from(self.force_training)).min(0x7fff)
    }

    /// Add `amount` to `ForceTraining` `+0x90` through `FUN_004ee1d0`,
    /// clamped to `0..=0x7fff`.
    pub fn add_force_training(&mut self, amount: i32) {
        let value = (i32::from(self.force_training) + amount).clamp(0, 0x7fff);
        self.force_training = u16::try_from(value).unwrap_or(0x7fff);
    }

    /// `FUN_004eee60`'s guard: a character with no Force base, with both
    /// `ForcePotential` and `ForceAware`, gets one. It runs at creation and
    /// from both flags' hooks (slots `+0x358`, `+0x35c`: `FUN_004f1b30`).
    #[must_use]
    pub fn force_base_due(&self) -> bool {
        self.force_base == 0 && self.force_potential && self.force_aware
    }

    /// `FUN_004eee60`'s roll: the class Force base (`+0xa0`) plus `roll`,
    /// a draw from `0..=` the class variance (`+0xa4`, `FUN_0053e290`),
    /// set through `FUN_004ee150`, clamped to `0..=0x7fff`.
    pub fn roll_force_base(&mut self, roll: u32) {
        let value = self.jedi_level.base.saturating_add(roll).min(0x7fff);
        self.force_base = u16::try_from(value).unwrap_or(0x7fff);
    }

    /// Add `amount` to `ForceExperience` `+0x8e` (the port's `force_base`)
    /// through its setter `FUN_004ee150`, clamped to `0..=0x7fff`.
    pub fn add_force_experience(&mut self, amount: i32) {
        let value = (i32::from(self.force_base) + amount).clamp(0, 0x7fff);
        self.force_base = u16::try_from(value).unwrap_or(0x7fff);
    }

    /// Add `injury` points through slot `+0x2f0` (`FUN_004ef6d0`): a
    /// positive amount raises `+0x94`, clamped to `0x7fff`.
    pub fn injure(&mut self, injury: i32) {
        if injury > 0 {
            let total = i32::from(self.injury).saturating_add(injury).min(0x7fff);
            self.injury = u16::try_from(total).unwrap_or(0x7fff);
            // The injury hook (slot +0x32c, FUN_004f1720) runs FUN_004eeed0,
            // which clears the command of an injured character.
            if !crate::character_command::keeps(self) {
                self.set_command(crate::character_command::CommandKind::None);
            }
        }
    }

    /// `FUN_004ee350`: set `CommandKind` `+0x96`; on a change its hook
    /// (slot `+0x330`, `FUN_004f17f0`) renames the character to the
    /// command's name ([`Character::command_names`], through
    /// `FUN_004f6e60`).
    pub fn set_command(&mut self, kind: crate::character_command::CommandKind) {
        if self.command == kind {
            return;
        }
        self.command = kind;
        let name = &self.command_names[kind as usize];
        if !name.is_empty() {
            self.name.clone_from(name);
        }
    }

    /// Put the character aboard `fleet` (or on the planet when `None`) at
    /// `system`. `FUN_00560d50`, which the move event `FUN_00514a60` runs: a
    /// character that moves loses its command (`FUN_004ee350(0)`).
    ///
    /// port: the original keeps it across a move between two capital ships
    /// of the same fleet; the port keeps characters on the fleet, not a
    /// ship, so such a move does not happen. A capital ship that moves takes
    /// the commands of everyone aboard; the port's fleet merge moves every
    /// character with the ships (`fleet_join::merge_fleet_into`), and a
    /// split moves none.
    pub fn move_to(&mut self, fleet: Option<FleetKey>, system: Option<SystemKey>) {
        if (self.current_fleet, self.current_system) != (fleet, system) {
            self.set_command(crate::character_command::CommandKind::None);
        }
        self.current_fleet = fleet;
        self.current_system = system;
    }

    /// Mark this character as killed and clear all "alive" state so that
    /// other systems filter it out correctly.
    ///
    /// Clears `current_system`, `current_fleet`, `on_mission`,
    /// `on_hidden_mission`, `on_mandatory_mission`, and captivity state, but
    /// leaves the character record in the arena so that `dat_id` / `name`
    /// remain resolvable by next-tick reactive story events.
    ///
    /// Call from both `cleanup_destroyed_system` (Death Star kills) and the
    /// `MissionEffect::CharacterKilled` integrator arm (assassinations).
    /// Idempotent — calling twice is a no-op.
    ///
    /// Systems that iterate over `world.characters` should short-circuit on
    /// `is_killed == true` rather than relying on the arena-absent invariant
    /// that existed before Knesset Shamash-Bet #R11.
    pub fn mark_killed(&mut self) {
        // Slot +0x11c (FUN_004f1340), on the destroyed bit (+0x50 0x8,
        // FUN_004f7560): FUN_004eeed0 takes the command.
        self.set_command(crate::character_command::CommandKind::None);
        self.is_killed = true;
        self.current_system = None;
        self.current_fleet = None;
        self.on_mission = false;
        self.on_hidden_mission = false;
        self.on_mandatory_mission = false;
        self.is_captive = false;
        self.captured_by = None;
        self.capture_tick = None;
    }
}

impl Default for Character {
    fn default() -> Self {
        Self {
            dat_id: DatId::new(0),
            name: String::new(),
            is_alliance: false,
            is_empire: false,
            is_major: false,
            diplomacy: SkillPair {
                base: 0,
                variance: 0,
            },
            espionage: SkillPair {
                base: 0,
                variance: 0,
            },
            ship_design: SkillPair {
                base: 0,
                variance: 0,
            },
            troop_training: SkillPair {
                base: 0,
                variance: 0,
            },
            facility_design: SkillPair {
                base: 0,
                variance: 0,
            },
            combat: SkillPair {
                base: 0,
                variance: 0,
            },
            leadership: SkillPair {
                base: 0,
                variance: 0,
            },
            loyalty: SkillPair {
                base: 0,
                variance: 0,
            },
            jedi_probability: 0,
            jedi_level: SkillPair {
                base: 0,
                variance: 0,
            },
            can_be_admiral: false,
            can_be_commander: false,
            can_be_general: false,
            is_unable_to_betray: false,
            is_jedi_trainer: false,
            is_known_jedi: false,
            hyperdrive_modifier: 0,
            enhanced_loyalty: 0,
            on_mission: false,
            on_hidden_mission: false,
            on_mandatory_mission: false,
            captured_by: None,
            capture_tick: None,
            is_captive: false,
            current_system: None,
            current_fleet: None,
            heritage_known: false,
            is_killed: false,
            is_retired: false,
            recruited: false,
            injury: 0,
            force_potential: false,
            force_aware: false,
            force_base: 0,
            force_discoverer: None,
            force_training: 0,
            current_abode: None,
            dagobah_completed: false,
            encounter: None,
            encounter_day: 0,
            escape_barred: false,
            adrift: false,
            bounty_captured: false,
            command: crate::character_command::CommandKind::None,
            command_names: Default::default(),
        }
    }
}

/// A base value paired with a random variance for character skill generation.
///
/// Final skill = `base + rng(0..=variance)` at scenario start.
/// Major characters typically use `variance = 0` to lock their stats.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SkillPair {
    pub base: u32,
    pub variance: u32,
}

/// A fleet — a collection of ships and characters at a system location.
///
/// Fleets are the primary unit of strategic movement. They orbit systems,
/// travel hyperlanes, and carry characters in command roles.
///
/// Capital ships are stored as individual `ShipInstance` records (per-hull
/// state with `hull_current` and `alive`). Fighter squadrons remain as
/// aggregate `(class_key, count)` pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fleet {
    /// Last orbiting system. An active movement order is authoritative in transit.
    pub location: SystemKey,
    /// Per-hull capital ship records. Each element is one physical hull with
    /// its own `hull_current` and `alive` state. Replaces the old aggregate
    /// `Vec<ShipEntry>` representation.
    pub capital_ships: Vec<ShipInstance>,
    /// Fighter squadron class references with counts.
    pub fighters: Vec<FighterEntry>,
    /// Characters assigned to this fleet (admiral, general, etc.).
    pub characters: Vec<CharacterKey>,
    /// True if this fleet belongs to the Rebel Alliance; false = Empire.
    pub is_alliance: bool,
    /// True if this fleet contains a Death Star (a CAPSHPSD family `0x18`
    /// object, `FUN_005003d0`; [`CapitalShipClass::is_death_star`]).
    ///
    /// Enables the Death Star win-condition check in `VictorySystem`.
    /// Set by `rebellion-data` when loading fleet composition from CAPSHPSD.
    pub has_death_star: bool,
    /// Special forces aboard (`FUN_00536da0(ship, 3)` lists them with the
    /// characters on the Fleet window's Personnel tab). port: on the fleet,
    /// not a ship, as the port keeps its characters; a unit aboard is in no
    /// system's list.
    pub special_forces: Vec<SpecialForceKey>,
}

impl Fleet {
    /// Total number of alive capital ships in this fleet.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
    )]
    pub fn ship_count(&self) -> u32 {
        self.capital_ships.iter().filter(|s| s.alive).count() as u32
    }

    /// Group alive ships by class, returning `(class_key, count)` pairs.
    /// Used by render panels for "Star Destroyer ×3" display.
    #[must_use]
    pub fn ship_counts_by_class(&self) -> Vec<(CapitalShipKey, u32)> {
        let mut counts: Vec<(CapitalShipKey, u32)> = Vec::new();
        for ship in &self.capital_ships {
            if !ship.alive {
                continue;
            }
            if let Some(entry) = counts.iter_mut().find(|(k, _)| *k == ship.class) {
                entry.1 += 1;
            } else {
                counts.push((ship.class, 1));
            }
        }
        counts
    }

    /// True if this fleet has no alive capital ships, fighter squadrons, or
    /// separate Death Star tactical object.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.has_death_star
            && !self.capital_ships.iter().any(|s| s.alive)
            && self.fighters.iter().all(|e| e.count == 0)
    }
}

/// One entry in a fleet's or a system's fighter roster: a class, the number
/// of squadrons, and the capital ship they ride on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FighterEntry {
    pub class: FighterKey,
    pub count: u32,
    /// The `ShipInstance::id` of the capital ship that carries them. In the
    /// original a squadron's container is a capital ship (galaxy types
    /// `0x14..0x1c`, `FUN_00500b40`) or a system, never the fleet itself
    /// (`FUN_004fe160`); `None` on a system, and in a fleet for the port's
    /// squadrons not yet boarded.
    pub carrier: Option<u64>,
}

/// A single hull of a capital ship — the primary ship record in Fleet.
///
/// Each element in `Fleet::capital_ships` is one physical hull with its own
/// health and alive state. This is the unit-level record used by combat,
/// repair, and all fleet logic.
///
/// Mirrors the C++ entity object fields confirmed by Ghidra:
/// - `hull_current` → offset +0x60 (int)
/// - `subsystem_hits` → offset +0x64 (six 4-bit hit counters, [`Subsystem`])
/// - `alive` → offset +0xac bit0
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipInstance {
    /// Reference to the class template in `GameWorld::capital_ship_classes`.
    pub class: CapitalShipKey,
    /// Current hull. Starts at `CapitalShipClass::hull`, reduced by combat.
    pub hull_current: i32,
    /// `+0x64`: the six 4-bit subsystem hit counters
    /// (`ghidra/notes/ship-subsystem-damage.md`); see [`Subsystem`] and
    /// [`ShipInstance::hits`]. A battle starts from them (`FUN_0040b8a0`)
    /// and writes them back (`FUN_0040cfc0`).
    #[serde(default)]
    pub subsystem_hits: u32,
    /// True while `hull_current` > 0 and the ship has not been destroyed.
    pub alive: bool,
    /// The name the player gave it (`+0x34`, order 0x203); `None` shows the
    /// class's (`FUN_004f6270`).
    pub name: Option<String>,
    /// port: the ship's identity, 0 until [`GameWorld::number_ships`]
    /// numbers it. The original's AI keeps a record per ship object
    /// (`FUN_00484230`); the port's AI keys its per-ship state on this.
    #[serde(default)]
    pub id: u64,
}

impl ShipInstance {
    /// Create a new ship at full hull strength.
    #[must_use]
    pub fn new(class: CapitalShipKey, hull: i32, _is_alliance: bool) -> Self {
        ShipInstance {
            class,
            hull_current: hull,
            subsystem_hits: 0,
            alive: true,
            name: None,
            id: 0,
        }
    }

    /// Create `count` instances of the same class at full hull.
    #[must_use]
    pub fn make(class: CapitalShipKey, hull: i32, is_alliance: bool, count: u32) -> Vec<Self> {
        (0..count)
            .map(|_| Self::new(class, hull, is_alliance))
            .collect()
    }

    /// The hits counted against `subsystem` (`+0x64`, read by
    /// `FUN_0040d6b0`..`FUN_0040d700`).
    #[must_use]
    pub fn hits(&self, subsystem: Subsystem) -> u8 {
        ((self.subsystem_hits >> subsystem.shift()) & 0xf) as u8
    }

    /// Store the hits against `subsystem`, at most 15: the setters
    /// `FUN_00501510`..`FUN_00501820` keep the low four bits.
    pub fn set_hits(&mut self, subsystem: Subsystem, hits: u8) {
        let shift = subsystem.shift();
        self.subsystem_hits =
            (self.subsystem_hits & !(0xf << shift)) | (u32::from(hits.min(0xf)) << shift);
    }
}

/// A capital ship's subsystem with a hit counter in `+0x64`
/// (`ghidra/notes/ship-subsystem-damage.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subsystem {
    /// Bits 0..3, ShieldRechargeRateCHAllocated (`FUN_00501510`).
    Shield,
    /// Bits 4..7, WeaponRechargeRateCHAllocated (`FUN_005015a0`).
    Weapon,
    /// Bits 8..11, TractorBeamPowerCHAllocated (`FUN_00501640`).
    Tractor,
    /// Bits 12..15, SpeedCHAllocated (`FUN_005016e0`).
    Speed,
    /// Bits 16..19, PrimaryHyperdriveModifier (`FUN_00501780`).
    PrimaryHyperdrive,
    /// Bits 20..23, BackupHyperdriveModifier (`FUN_00501820`).
    BackupHyperdrive,
}

impl Subsystem {
    /// Every counter, in bit order.
    pub const ALL: [Self; 6] = [
        Self::Shield,
        Self::Weapon,
        Self::Tractor,
        Self::Speed,
        Self::PrimaryHyperdrive,
        Self::BackupHyperdrive,
    ];

    /// The counter's bit offset in `+0x64`, as `FUN_0040d6b0`..`FUN_0040d700`
    /// shift it (0, 4, 8, `0xc`, `0x10`, `0x14`).
    const fn shift(self) -> u32 {
        match self {
            Self::Shield => 0,
            Self::Weapon => 4,
            Self::Tractor => 8,
            Self::Speed => 12,
            Self::PrimaryHyperdrive => 16,
            Self::BackupHyperdrive => 20,
        }
    }
}

/// Game-balance parameters loaded from GNPRTB.DAT.
///
/// Each entry in GNPRTB.DAT has a `parameter_id` (0-212) and 8 i32 values
/// keyed by difficulty/faction mode. The `value()` accessor returns the
/// appropriate value for a given difficulty index (0-7).
///
/// Difficulty index mapping (8 levels, from the DAT binary format):
///   0 = development
///   1 = Alliance SP Easy (`alliance_sp_easy`)
///   2 = Alliance SP Medium (`alliance_sp_medium`)
///   3 = Alliance SP Hard (`alliance_sp_hard`)
///   4 = Empire SP Easy (`empire_sp_easy`)
///   5 = Empire SP Medium (`empire_sp_medium`)
///   6 = Empire SP Hard (`empire_sp_hard`)
///   7 = Multiplayer
///
/// The C++ `difficulty_packed` at offset +0x24 bits 4-5 is a 2-bit selector
/// (0-3) used by `FUN_004fd600` to pick Alliance(1) or Empire(2). The full
/// 8-level index is computed at the caller level.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GnprtbParams {
    /// All 213 entries, indexed by `parameter_id`.
    entries: Vec<GnprtbEntry>,
}

/// One entry from GNPRTB.DAT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnprtbEntry {
    pub parameter_id: u32,
    pub development: i32,
    pub alliance_sp_easy: i32,
    pub alliance_sp_medium: i32,
    pub alliance_sp_hard: i32,
    pub empire_sp_easy: i32,
    pub empire_sp_medium: i32,
    pub empire_sp_hard: i32,
    pub multiplayer: i32,
}

impl GnprtbParams {
    /// Construct from raw entries (called by `rebellion-data` loader).
    #[must_use]
    pub fn new(entries: Vec<GnprtbEntry>) -> Self {
        Self { entries }
    }

    /// Parameters holding the same value at every difficulty.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn uniform(values: &[(u16, i32)]) -> Self {
        let entry = |&(id, value): &(u16, i32)| GnprtbEntry {
            parameter_id: u32::from(id),
            development: value,
            alliance_sp_easy: value,
            alliance_sp_medium: value,
            alliance_sp_hard: value,
            empire_sp_easy: value,
            empire_sp_medium: value,
            empire_sp_hard: value,
            multiplayer: value,
        };
        Self::new(values.iter().map(entry).collect())
    }

    /// Return the parameter value for `param_id` at `difficulty`: one of the
    /// eight value columns of GNPRTB.DAT's `GeneralParamTableEntry`
    /// (`agent_docs/dat-formats.md`), in file order.
    ///
    /// `difficulty`: 0=development, `1=alliance_easy`, `2=alliance_medium`, `3=alliance_hard`,
    ///               `4=empire_easy`, `5=empire_medium`, `6=empire_hard`, 7=multiplayer.
    /// Returns 0 if `param_id` is out of range.
    #[must_use]
    pub fn value(&self, param_id: u16, difficulty: u8) -> i32 {
        self.entries
            .iter()
            .find(|e| e.parameter_id == u32::from(param_id))
            .map_or(0, |e| match difficulty {
                0 => e.development,
                1 => e.alliance_sp_easy,
                2 => e.alliance_sp_medium,
                3 => e.alliance_sp_hard,
                4 => e.empire_sp_easy,
                5 => e.empire_sp_medium,
                6 => e.empire_sp_hard,
                _ => e.multiplayer,
            })
    }
}

/// Side-aware seeding parameters loaded from SDPRTB.DAT.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SdprtbParams {
    entries: Vec<SdprtbEntry>,
}

/// One entry from SDPRTB.DAT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdprtbEntry {
    pub parameter_id: u32,
    pub dev_alliance: i32,
    pub dev_empire: i32,
    pub alliance_sp_easy_alliance: i32,
    pub alliance_sp_easy_empire: i32,
    pub alliance_sp_medium_alliance: i32,
    pub alliance_sp_medium_empire: i32,
    pub alliance_sp_hard_alliance: i32,
    pub alliance_sp_hard_empire: i32,
    pub empire_sp_easy_alliance: i32,
    pub empire_sp_easy_empire: i32,
    pub empire_sp_medium_alliance: i32,
    pub empire_sp_medium_empire: i32,
    pub empire_sp_hard_alliance: i32,
    pub empire_sp_hard_empire: i32,
    pub multiplayer_alliance: i32,
    pub multiplayer_empire: i32,
}

impl SdprtbParams {
    #[must_use]
    pub fn new(entries: Vec<SdprtbEntry>) -> Self {
        Self { entries }
    }

    /// Whether the table has `param_id`.
    #[must_use]
    pub fn contains(&self, param_id: u16) -> bool {
        self.entries
            .iter()
            .any(|e| e.parameter_id == u32::from(param_id))
    }

    /// Return a side-aware seeding parameter for the requested difficulty
    /// column: SDPRTB.DAT's `SideParamTableEntry` holds an Alliance and an
    /// Empire value for each of its eight columns, in file order
    /// (`agent_docs/dat-formats.md`).
    #[must_use]
    pub fn value(&self, param_id: u16, difficulty: u8, faction: Faction) -> i32 {
        self.entries
            .iter()
            .find(|e| e.parameter_id == u32::from(param_id))
            .map_or(0, |entry| match (difficulty, faction) {
                (0, Faction::Alliance) => entry.dev_alliance,
                (0, Faction::Empire) => entry.dev_empire,
                (1, Faction::Alliance) => entry.alliance_sp_easy_alliance,
                (1, Faction::Empire) => entry.alliance_sp_easy_empire,
                (2, Faction::Alliance) => entry.alliance_sp_medium_alliance,
                (2, Faction::Empire) => entry.alliance_sp_medium_empire,
                (3, Faction::Alliance) => entry.alliance_sp_hard_alliance,
                (3, Faction::Empire) => entry.alliance_sp_hard_empire,
                (4, Faction::Alliance) => entry.empire_sp_easy_alliance,
                (4, Faction::Empire) => entry.empire_sp_easy_empire,
                (5, Faction::Alliance) => entry.empire_sp_medium_alliance,
                (5, Faction::Empire) => entry.empire_sp_medium_empire,
                (6, Faction::Alliance) => entry.empire_sp_hard_alliance,
                (6, Faction::Empire) => entry.empire_sp_hard_empire,
                (_, Faction::Alliance) => entry.multiplayer_alliance,
                (_, Faction::Empire) => entry.multiplayer_empire,
                (_, Faction::Neutral) => 0,
            })
    }
}

/// One MISSNSD.DAT record: the rules a mission class reads from its record
/// (`ghidra/notes/mission-lifecycle.md`, "The mission record").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionRecord {
    /// Record id, `family << 24 | index` (e.g. `0x51000010` for Diplomacy).
    pub dat_id: crate::ids::DatId,
    /// Mission timer minimum in days (record `+0x50`, `FUN_005236e0`).
    pub timer_min_days: u32,
    /// Mission timer spread in days (record `+0x54`, `FUN_005236e0`).
    pub timer_spread_days: u32,
    /// Phase 10 loops back to phase 8 (record `+0x58`, `FUN_00520b60`, read
    /// by the stepper `FUN_005227d0`).
    pub repeats: bool,
    /// Members are on a hidden mission (record `+0x5c`, `FUN_00520b70`).
    pub hidden: bool,
    /// The decoy and detection phases run (record `+0x60`, `FUN_00520b80`).
    pub detection_phases: bool,
    /// Members may resign (record `+0x64`, `FUN_00520b90`).
    pub can_resign: bool,
    /// How members leave the ended mission (record `+0x68`, `FUN_00520ba0`,
    /// read by `FUN_00521220`; save v91): 0 they stay, 1 they return from a
    /// place of another side, 2 they return.
    pub leave_rule: u32,
    /// What the validator requires of a running mission's target and
    /// container (record `+0x6c..+0x94`).
    pub rules: MissionTargetRules,
    /// Who may be a member (record `+0x40..+0x4c`, `FUN_00583320`).
    pub members: MissionMemberRules,
}

/// The MISSNSD columns the member check `FUN_00583320` reads
/// (`ghidra/notes/ai-mission-planning.md`, "Legality").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionMemberRules {
    /// Alliance members are allowed (`+0x40`).
    pub alliance: bool,
    /// Empire members are allowed (`+0x44`).
    pub empire: bool,
    /// The special-force mission bits allowed (`+0x48`); every bit of the
    /// members' SPECFCSD masks must be here.
    pub special_force_mask: u32,
    /// The character bits allowed (`+0x4c`); a character contributes
    /// `0x10000` (`FUN_004ed260`).
    pub character_mask: u32,
}

/// The MISSNSD columns the running-mission validator reads
/// (`ghidra/notes/mission-lifecycle.md`, "The running checks").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionTargetRules {
    /// End 7 when the container is destroyed (`+0x6c`, `FUN_00523450`).
    pub container_loss_ends: bool,
    /// End `0xd` unless the container is a populated system (`+0x74`,
    /// `FUN_00592600`).
    pub needs_populated_container: bool,
    /// End 6 when the target is destroyed (`+0x78`, `FUN_00592600`).
    pub target_loss_ends: bool,
    /// A target on the mission's side is allowed, else end 8 (`+0x7c`).
    pub own_side_target: bool,
    /// A target of a third side is allowed (`+0x80`).
    pub other_side_target: bool,
    /// A target of the opponent is allowed (`+0x84`).
    pub opponent_target: bool,
    /// A system target in an uprising is allowed, else end 6 (`+0x88`,
    /// `FUN_005868c0`).
    pub revolting_target: bool,
    /// A system target not in an uprising is allowed (`+0x8c`).
    pub calm_target: bool,
    /// A prisoner character target is allowed, else end 6 (`+0x90`,
    /// `FUN_00586e20`).
    pub prisoner_target: bool,
    /// A free character target is allowed (`+0x94`).
    pub free_target: bool,
}

/// A lookup table loaded from one of the `*MSTB.DAT` / `*TB.DAT` files.
///
/// Each table is a sorted list of `(threshold, value)` pairs where `threshold`
/// is a signed skill delta (negative = below average, 0 = average, positive =
/// above average). The original lookup (`FUN_00595090`) is a step function,
/// which `step_lookup()` reproduces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MstbTable {
    /// Entries sorted ascending by threshold.
    entries: Vec<MstbEntry>,
}

/// One row in an `MstbTable`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MstbEntry {
    pub threshold: i32,
    pub value: u32,
}

impl MstbTable {
    /// Construct from raw `(threshold, value)` pairs. Sorts by threshold.
    #[must_use]
    pub fn new(mut entries: Vec<MstbEntry>) -> Self {
        entries.sort_by_key(|e| e.threshold);
        Self { entries }
    }

    /// Look up the value for `x` as the original does (`FUN_00595090`): the
    /// entry with the largest threshold not above `x`, or the first entry when
    /// `x` is below every threshold. Returns 0 for an empty table.
    #[must_use]
    pub fn step_lookup(&self, x: i32) -> u32 {
        self.entries
            .iter()
            .take_while(|e| e.threshold <= x)
            .last()
            .or_else(|| self.entries.first())
            .map_or(0, |e| e.value)
    }
}

/// A troop regiment stationed at a system — a deployed instance of a troop class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TroopUnit {
    /// The class definition (from TROOPSD.DAT).
    pub class_dat_id: DatId,
    pub is_alliance: bool,
    /// Current regiment strength (C++ offset +0x96). Starts at class max, reduced by ground combat.
    pub regiment_strength: i16,
}

/// A special-forces unit stationed at a system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialForceUnit {
    /// The class definition (from SPECFCSD.DAT).
    pub class_dat_id: DatId,
    pub is_alliance: bool,
    /// Skills in [`Skill`] order, rolled at creation from the class:
    /// base + rand(0..=variance) (`FUN_00535e40`). Read through slots
    /// `+0x1dc..+0x1f8` of vtable `0x0065e160` as the shorts `+0x58..+0x66`.
    pub skills: [u32; 8],
    /// Currently a mission member (role flag bit 7, `RoleOnMissionNotif`
    /// `FUN_00536b00`; special forces share the role flags with characters).
    pub on_mission: bool,
}

/// What one unit of a regiment, special-force or facility class costs to
/// build and when it becomes available: the head that TROOPSD, SPECFCSD,
/// DEFFACSD, MANFACSD and PROFACSD records share. A class record in memory
/// sits 0x28 bytes past its DAT record (troop detection `+0x5c`,
/// `decoy-roll.md`), so `FUN_0053b860`'s `+0x48` is the refined material
/// cost and `FUN_0053b870`'s `+0x4c` the maintenance cost.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildableClass {
    /// The class's TEXTSTRA name.
    pub name: String,
    pub is_alliance: bool,
    pub is_empire: bool,
    pub refined_material_cost: u32,
    pub maintenance_cost: u32,
    /// The research level that makes the class available.
    pub research_order: u32,
    pub research_difficulty: u32,
    /// A yard's days per unit of build progress, class `+0x5c` that
    /// `FUN_00520b70` reads: MANFACSD and PROFACSD `processing_rate`. Zero
    /// for the other files.
    pub processing_rate: u32,
    /// The Status window's class figures (`ghidra/notes/status-window.md`).
    #[serde(default)]
    pub stats: ClassStats,
}

/// A class record's figures that only the Status window reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassStats {
    /// "Bombardment Value" / "Bombardment Defense Strength": the
    /// `bombardment_defense` field of TROOPSD (class `+0x60`), DEFFACSD,
    /// MANFACSD and PROFACSD (class `+0x58`, `FUN_00520b60`).
    pub bombardment: u32,
    /// DEFFACSD `attack_strength`, class `+0x5c` (`FUN_00520b70`): "Weapons
    /// Rating".
    pub attack_strength: u32,
    /// DEFFACSD `shield_strength`, class `+0x60` (`FUN_00520b80`): "Shield
    /// Strength".
    pub shield_strength: u32,
}

impl BuildableClass {
    /// Whether the side may build the class (`+0x18`/`+0x1c` of the record).
    #[must_use]
    pub const fn serves(&self, is_alliance: bool) -> bool {
        if is_alliance {
            self.is_alliance
        } else {
            self.is_empire
        }
    }

    /// Whether an object of the class may belong to `side`
    /// (`FUN_004f27d0`): neutral needs both flags.
    #[must_use]
    pub const fn serves_side(&self, side: Faction) -> bool {
        match side {
            Faction::Alliance => self.is_alliance,
            Faction::Empire => self.is_empire,
            Faction::Neutral => self.is_alliance && self.is_empire,
        }
    }
}

/// A facility at a system, from any of the three facility arenas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FacilityRef {
    Defense(DefenseFacilityKey),
    Manufacturing(ManufacturingFacilityKey),
    Production(ProductionFacilityKey),
}

/// A defense facility instance on a system surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseFacilityInstance {
    /// The class definition (from DEFFACSD.DAT).
    pub class_dat_id: DatId,
    /// Its side (`+0x24` bits 6..7). It follows its system's holder:
    /// seeding copies the system's side, and a change of control hands it
    /// over (`ghidra/notes/facility-ownership.md`).
    pub side: Faction,
}

/// A manufacturing facility instance (shipyard, training center, construction yard).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManufacturingFacilityInstance {
    /// The class definition (from MANFACSD.DAT).
    pub class_dat_id: DatId,
    /// Its side (`+0x24` bits 6..7). It follows its system's holder:
    /// seeding copies the system's side, and a change of control hands it
    /// over (`ghidra/notes/facility-ownership.md`).
    pub side: Faction,
    /// True if this facility is a shipyard (can build/repair ships).
    /// Set during loading from the DAT `production_family` field.
    #[serde(default)]
    pub is_shipyard: bool,
}

/// A production facility instance (mine, refinery).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionFacilityInstance {
    /// The class definition (from PROFACSD.DAT).
    pub class_dat_id: DatId,
    /// Its side (`+0x24` bits 6..7). It follows its system's holder:
    /// seeding copies the system's side, and a change of control hands it
    /// over (`ghidra/notes/facility-ownership.md`).
    pub side: Faction,
    /// True if this facility is a mine (raw material extraction).
    /// Set during loading from the DAT `production_family` field.
    #[serde(default)]
    pub is_mine: bool,
}

/// Class definition for a troop type — a template loaded from TROOPSD.DAT.
///
/// Instances (`TroopUnit`) reference this by `class_dat_id`.
/// Used by ground combat to look up per-class attack/defense values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TroopClassDef {
    /// Ground attack strength of this troop class.
    pub attack_strength: u32,
    /// Ground defense strength of this troop class.
    pub defense_strength: u32,
    /// Detection, the class record's `+0x5c` that a regiment reads as a
    /// detector (slot `+0x1c4`, `ghidra/notes/decoy-roll.md`).
    pub detection: u32,
    /// TROOPSD's field before `detection` (`uprising_defense` in the DAT
    /// codec), the class `+0x58` that slot `+0x1c0` returns: the value a
    /// lost regiment adds to its side's losses (`System_vtd0`,
    /// `ghidra/notes/betrayal-and-loyalty.md`).
    #[serde(default)]
    pub uprising_defense: u32,
}

/// Class definition for a special-forces unit, from SPECFCSD.DAT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialForceClassDef {
    /// Skill templates in [`Skill`] order (class record `+0x58..+0x94`).
    pub skills: [SkillPair; 8],
    /// Missions the unit may join (class record `+0x98`, `FUN_00503b40`).
    pub mission_mask: u32,
}

/// The eight person skills, in the order of the character and special-force
/// slots `+0x1dc..+0x1f8` (`ghidra/notes/decoy-roll.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Skill {
    Diplomacy,
    Espionage,
    ShipDesign,
    TroopTraining,
    FacilityDesign,
    Combat,
    Leadership,
    Loyalty,
}

impl Skill {
    /// Every skill in slot order.
    pub const ALL: [Skill; 8] = [
        Skill::Diplomacy,
        Skill::Espionage,
        Skill::ShipDesign,
        Skill::TroopTraining,
        Skill::FacilityDesign,
        Skill::Combat,
        Skill::Leadership,
        Skill::Loyalty,
    ];
}

impl Character {
    /// The skill template for `skill`.
    #[must_use]
    pub fn skill(&self, skill: Skill) -> SkillPair {
        match skill {
            Skill::Diplomacy => self.diplomacy,
            Skill::Espionage => self.espionage,
            Skill::ShipDesign => self.ship_design,
            Skill::TroopTraining => self.troop_training,
            Skill::FacilityDesign => self.facility_design,
            Skill::Combat => self.combat,
            Skill::Leadership => self.leadership,
            Skill::Loyalty => self.loyalty,
        }
    }

    /// The effective value of `skill`, as the slots `+0x1dc..+0x1f8` read
    /// it. Diplomacy (`+0x7c`) and espionage (`+0x7e`) gain the Force share
    /// (`FUN_004eecb0`, `FUN_004eecf0` -> `FUN_0053e940`: skill + skill ×
    /// Force level / 100, set through `FUN_004edcb0`, `FUN_004edd30`,
    /// clamped to `0..=0x7fff`); combat is [`Character::effective_combat`].
    /// The others are the skill itself (`FUN_004eed30`..`FUN_004eed70`).
    ///
    /// port: a skill is its rolled template, base plus half the variance,
    /// as `missions::member_skill` reads it. Leadership's share of `+0x92`
    /// (`FUN_004eede0`) needs the world:
    /// [`crate::seat_of_power::effective_leadership`].
    #[must_use]
    pub fn effective_skill(&self, skill: Skill) -> u32 {
        let pair = self.skill(skill);
        let value = pair.base + pair.variance / 2;
        match skill {
            Skill::Combat => self.effective_combat(),
            Skill::Diplomacy | Skill::Espionage => {
                let value = u64::from(value);
                let shared = value * u64::from(self.force_level()) / 100 + value;
                u32::try_from(shared.min(0x7fff)).unwrap_or(0x7fff)
            }
            _ => value,
        }
    }

    /// The skill pair for `skill`, to change it.
    pub fn skill_mut(&mut self, skill: Skill) -> &mut SkillPair {
        match skill {
            Skill::Diplomacy => &mut self.diplomacy,
            Skill::Espionage => &mut self.espionage,
            Skill::ShipDesign => &mut self.ship_design,
            Skill::TroopTraining => &mut self.troop_training,
            Skill::FacilityDesign => &mut self.facility_design,
            Skill::Combat => &mut self.combat,
            Skill::Leadership => &mut self.leadership,
            Skill::Loyalty => &mut self.loyalty,
        }
    }
}

/// Class definition for a defense facility — a template loaded from DEFFACSD.DAT.
///
/// Instances (`DefenseFacilityInstance`) reference this by `class_dat_id`.
/// Used by the bombardment system to sum per-facility defense contributions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseFacilityClassDef {
    /// Bombardment defense contribution of this facility class.
    /// Summed across all facility instances during orbital bombardment resolution.
    pub bombardment_defense: i32,
}

/// The complete game world state — the root of all simulation data.
///
/// All entity arenas live here. Cross-entity references use slotmap keys;
/// they're meaningless outside the arena they index into.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameWorld {
    pub systems: slotmap::SlotMap<SystemKey, System>,
    pub sectors: slotmap::SlotMap<SectorKey, Sector>,
    pub capital_ship_classes: slotmap::SlotMap<CapitalShipKey, CapitalShipClass>,
    pub fighter_classes: slotmap::SlotMap<FighterKey, FighterClass>,
    pub characters: slotmap::SlotMap<CharacterKey, Character>,
    pub fleets: slotmap::SlotMap<FleetKey, Fleet>,
    /// Deployed troop regiments (instances, not class definitions).
    pub troops: slotmap::SlotMap<TroopKey, TroopUnit>,
    /// Deployed special-forces units.
    pub special_forces: slotmap::SlotMap<SpecialForceKey, SpecialForceUnit>,
    /// Defense facilities on system surfaces.
    pub defense_facilities: slotmap::SlotMap<DefenseFacilityKey, DefenseFacilityInstance>,
    /// Manufacturing facilities (shipyards, training centers, construction yards).
    pub manufacturing_facilities:
        slotmap::SlotMap<ManufacturingFacilityKey, ManufacturingFacilityInstance>,
    /// Production facilities (mines, refineries).
    pub production_facilities: slotmap::SlotMap<ProductionFacilityKey, ProductionFacilityInstance>,
    /// Troop class definitions keyed by `DatId` (from TROOPSD.DAT).
    /// Used by ground combat to look up per-class attack/defense values.
    /// Saved with the world (bincode ignores `serde(default)`).
    #[serde(default)]
    pub troop_classes: HashMap<crate::ids::DatId, TroopClassDef>,
    /// Defense facility class definitions keyed by `DatId` (from DEFFACSD.DAT).
    /// Used by bombardment to look up per-class `bombardment_defense` values.
    /// Saved with the world (bincode ignores `serde(default)`).
    #[serde(default)]
    pub defense_facility_classes: HashMap<crate::ids::DatId, DefenseFacilityClassDef>,
    /// Special-forces class definitions keyed by `DatId` (from SPECFCSD.DAT).
    /// Saved with the world.
    pub special_force_classes: HashMap<crate::ids::DatId, SpecialForceClassDef>,
    /// Regiment, special-force and facility classes keyed by `DatId`, with
    /// what each costs to build (TROOPSD, SPECFCSD, DEFFACSD, MANFACSD,
    /// PROFACSD). Saved with the world.
    pub buildable_classes: HashMap<crate::ids::DatId, BuildableClass>,
    /// Game-balance parameters from GNPRTB.DAT (combat formulas, bombardment divisors, etc.).
    pub gnprtb: GnprtbParams,
    /// Side-aware startup parameters from SDPRTB.DAT.
    pub sdprtb: SdprtbParams,
    /// Mission probability tables keyed by DAT file stem (e.g. "DIPLMSTB", "ESPIMSTB").
    pub mission_tables: HashMap<String, MstbTable>,
    /// MISSNSD.DAT records in file order. Saved with the world.
    pub mission_records: Vec<MissionRecord>,
    /// GNPRTB difficulty column index (0-7) for this game session.
    /// Set from `SeedOptions::gnprtb_index()` at game start. Default 2 (Alliance Medium).
    #[serde(default = "default_difficulty_index")]
    pub difficulty_index: u8,
    /// Per side (Alliance, Empire): the side recruited its last pool
    /// character (side `+0xb8`, set by `FUN_0052f590` from `FUN_0055fc80`).
    /// Recruitment then ends with code `0x10` (`FUN_0056b370`).
    pub recruit_pool_empty: [bool; 2],
    /// Each fleet's name (`+0x34`) and each side's default-name counter.
    pub fleet_names: FleetNames,
    /// The last id [`Self::number_ships`] gave a capital ship.
    #[serde(default)]
    pub last_ship_id: u64,
    /// The abodes (ABODESD), children of the galaxy (`FUN_00519910`), in
    /// file order (save v72).
    pub abodes: slotmap::SlotMap<crate::ids::AbodeKey, Abode>,
}

/// An abode (ABODESD, family `0x98`, class vtable `0x00669260` built by
/// `FUN_00577530`): a place in the galaxy outside every system, with no
/// position (`ghidra/notes/dagobah.md`, "Abodes and travel to them").
/// Missions treat it as a place like a system (`FUN_00521070`), and a trip
/// to or from it takes 1 day (`FUN_00555b30`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Abode {
    pub dat_id: DatId,
    pub name: String,
    /// The side the galaxy's setup gives it (`FUN_00519910`): the Alliance
    /// Jail the Alliance, the Empire Jail and Jabba's Palace the Empire,
    /// Dagobah and the Hermitage none.
    pub side: crate::dat::Faction,
    /// The special forces it holds: the Bounty Hunters the bounty timer
    /// creates in Jabba's Palace (`FUN_00546fa0`, `FUN_004f7d50`; save v77).
    pub special_forces: Vec<crate::ids::SpecialForceKey>,
}

/// The fleet record's name (`+0x34`, TEXTSTRA 11523): a fleet's name until
/// it is numbered, and the stem of its default name.
pub const FLEET_RECORD_NAME: &str = "Fleet";

/// How a game names its new fleets. It is chosen before the game starts and
/// saved with it, so a loaded game never renames a fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FleetNaming {
    /// "Fleet 1", "Fleet 2" (`FUN_00517760`).
    #[default]
    Original,
    /// port: a name from the side's `fleet_name_bank`, then "Fleet N" once
    /// every bank name is held.
    Canonical,
}

/// Fleet names and the counters that number them (`ghidra/notes/fleet-names.md`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FleetNames {
    names: slotmap::SecondaryMap<FleetKey, String>,
    /// Per side (Alliance, Empire): the last number given (the side's
    /// counter list, `DAT_006b2bb0 + 0xc4`/`+0xc8`, node `[8]`).
    last_numbers: [u32; 2],
    mode: FleetNaming,
    /// Fleets the player renamed (order 0x203).
    renamed: slotmap::SecondaryMap<FleetKey, ()>,
}

/// port: a save written before the difficulty was stored loads at column 2,
/// Alliance medium.
fn default_difficulty_index() -> u8 {
    2
}

impl GameWorld {
    /// The facilities at `system`: defense, then manufacturing, then
    /// production.
    #[must_use]
    pub fn facilities_at(&self, system: SystemKey) -> Vec<FacilityRef> {
        self.systems.get(system).map_or_else(Vec::new, |value| {
            value
                .defense_facilities
                .iter()
                .map(|&key| FacilityRef::Defense(key))
                .chain(
                    value
                        .manufacturing_facilities
                        .iter()
                        .map(|&key| FacilityRef::Manufacturing(key)),
                )
                .chain(
                    value
                        .production_facilities
                        .iter()
                        .map(|&key| FacilityRef::Production(key)),
                )
                .collect()
        })
    }

    /// A facility's class and side.
    #[must_use]
    pub fn facility(&self, facility: FacilityRef) -> Option<(DatId, Faction)> {
        match facility {
            FacilityRef::Defense(key) => self
                .defense_facilities
                .get(key)
                .map(|value| (value.class_dat_id, value.side)),
            FacilityRef::Manufacturing(key) => self
                .manufacturing_facilities
                .get(key)
                .map(|value| (value.class_dat_id, value.side)),
            FacilityRef::Production(key) => self
                .production_facilities
                .get(key)
                .map(|value| (value.class_dat_id, value.side)),
        }
    }

    /// Set a facility's side.
    pub fn set_facility_side(&mut self, facility: FacilityRef, side: Faction) {
        match facility {
            FacilityRef::Defense(key) => {
                if let Some(value) = self.defense_facilities.get_mut(key) {
                    value.side = side;
                }
            }
            FacilityRef::Manufacturing(key) => {
                if let Some(value) = self.manufacturing_facilities.get_mut(key) {
                    value.side = side;
                }
            }
            FacilityRef::Production(key) => {
                if let Some(value) = self.production_facilities.get_mut(key) {
                    value.side = side;
                }
            }
        }
    }

    /// Remove a facility from `system` and from its arena.
    pub fn remove_facility(&mut self, system: SystemKey, facility: FacilityRef) {
        let Some(value) = self.systems.get_mut(system) else {
            return;
        };
        match facility {
            FacilityRef::Defense(key) => {
                value.defense_facilities.retain(|k| *k != key);
                self.defense_facilities.remove(key);
            }
            FacilityRef::Manufacturing(key) => {
                value.manufacturing_facilities.retain(|k| *k != key);
                self.manufacturing_facilities.remove(key);
            }
            FacilityRef::Production(key) => {
                value.production_facilities.retain(|k| *k != key);
                self.production_facilities.remove(key);
            }
        }
    }

    /// The side a system's facilities belong to under `control`: its
    /// holder, neutral when uncontrolled, and none while contested, when
    /// the last holder keeps them (`ghidra/notes/facility-ownership.md`).
    #[must_use]
    pub const fn facility_holder(control: ControlKind) -> Option<Faction> {
        match control {
            ControlKind::Controlled(side) | ControlKind::Uprising(side) => Some(side),
            ControlKind::Uncontrolled => Some(Faction::Neutral),
            ControlKind::Contested => None,
        }
    }

    /// A change of `system`'s holder (`FUN_004f6f40` → `FUN_004fae40` →
    /// `FUN_004f8680`): each facility of another side passes to the holder
    /// when its class serves that side (`FUN_004f27d0`), and is removed
    /// otherwise. Returns the facilities removed. A class missing from the
    /// catalog keeps its facility (port).
    pub fn hand_over_facilities(&mut self, system: SystemKey) -> Vec<FacilityRef> {
        let Some(holder) = self
            .systems
            .get(system)
            .and_then(|value| Self::facility_holder(value.control))
        else {
            return Vec::new();
        };
        let mut removed = Vec::new();
        for facility in self.facilities_at(system) {
            let Some((class, side)) = self.facility(facility) else {
                continue;
            };
            if side == holder {
                continue;
            }
            let serves = self
                .buildable_classes
                .get(&class)
                .is_none_or(|value| value.serves_side(holder));
            if serves {
                self.set_facility_side(facility, holder);
            } else {
                self.remove_facility(system, facility);
                removed.push(facility);
            }
        }
        removed
    }

    /// The MISSNSD record with id `id` (e.g. `0x51000010` for Diplomacy).
    /// A mission class reads one record through `+0x2c`
    /// (`ghidra/notes/mission-lifecycle.md`). The id, not the family, picks
    /// it: Research (`0x53`) has three records and Vacation (`0x72`) two.
    #[must_use]
    pub fn mission_record(&self, id: crate::ids::DatId) -> Option<&MissionRecord> {
        self.mission_records.iter().find(|record| record.dat_id == id)
    }

    /// Whether `side` recruited its last pool character (side `+0xb8`).
    #[must_use]
    pub fn recruit_pool_empty(&self, side: crate::dat::Faction) -> bool {
        recruit_side_index(side).is_some_and(|index| self.recruit_pool_empty[index])
    }

    /// Insert `fleet` and give it its side's next default name.
    /// Give every capital ship without an id the next one, fleet by fleet
    /// in key order and ship by ship.
    ///
    /// port: the drivers call this before the AI's passes, at a new
    /// campaign and at a load, so the AI's per-ship state
    /// ([`crate::ai::AIState::ship_holds`]) can follow a ship between
    /// fleets.
    /// Number each new ship, then seat each fleet's squadrons that ride on
    /// no living ship of it (`crate::squadrons::seat_squadrons`): a
    /// squadron's container is a capital ship (`FUN_00500b40`), which needs
    /// its id.
    pub fn number_ships(&mut self) {
        let mut last = self.last_ship_id;
        for fleet in self.fleets.values_mut() {
            for ship in fleet.capital_ships.iter_mut().filter(|s| s.id == 0) {
                last += 1;
                ship.id = last;
            }
        }
        self.last_ship_id = last;
        let fleets = self.fleets.keys().collect::<Vec<_>>();
        for fleet in fleets {
            crate::squadrons::seat_squadrons(self, fleet);
        }
    }

    pub fn insert_fleet(&mut self, fleet: Fleet) -> FleetKey {
        let key = self.fleets.insert(fleet);
        self.name_new_fleet(key);
        key
    }

    /// `FUN_00517760`: a fleet still bearing the record's name takes
    /// "Fleet N", N being its side's counter plus one (`FUN_005302c0`). The
    /// counter never goes down, so no number is given twice.
    ///
    /// hyp: the original runs this over every object at load
    /// (`FUN_0051b7f0`); where a fleet made in play is first numbered is
    /// untraced, so the port numbers each fleet as it is made.
    pub fn name_new_fleet(&mut self, fleet: FleetKey) {
        let Some(value) = self.fleets.get(fleet) else {
            return;
        };
        if self.fleet_names.names.contains_key(fleet) {
            return;
        }
        let side = usize::from(!value.is_alliance);
        let number = &mut self.fleet_names.last_numbers[side];
        *number += 1;
        let numbered = format!("{FLEET_RECORD_NAME} {number}");
        let name = match self.fleet_names.mode {
            FleetNaming::Original => numbered,
            FleetNaming::Canonical => self
                .free_bank_name(fleet)
                .map_or(numbered, str::to_owned),
        };
        self.fleet_names.names.insert(fleet, name);
    }

    /// Rename `fleet` (order 0x203): `FUN_004ac950` hands the edit's text to
    /// the name setter `FUN_004f6e60` only when it is not empty, so an empty
    /// name changes nothing. port: no signature name replaces a name the
    /// player gave.
    pub fn rename_fleet(&mut self, fleet: FleetKey, name: &str) -> bool {
        if name.is_empty() || !self.fleets.contains_key(fleet) {
            return false;
        }
        self.fleet_names.names.insert(fleet, name.to_owned());
        self.fleet_names.renamed.insert(fleet, ());
        true
    }

    /// Rename capital ship `index` of `fleet` (order 0x203, `FUN_004f6e60`).
    /// The setter refuses an object that is destroyed (`FUN_0053a000`) and
    /// `FUN_004ac950` an empty name.
    pub fn rename_ship(&mut self, fleet: FleetKey, index: usize, name: &str) -> bool {
        let Some(ship) = self
            .fleets
            .get_mut(fleet)
            .and_then(|value| value.capital_ships.get_mut(index))
            .filter(|ship| ship.alive && !name.is_empty())
        else {
            return false;
        };
        ship.name = Some(name.to_owned());
        true
    }

    /// A capital ship's name (`FUN_004f6270`): its own (`+0x34`), else its
    /// class's.
    #[must_use]
    pub fn ship_name(&self, fleet: FleetKey, index: usize) -> Option<&str> {
        let ship = self.fleets.get(fleet)?.capital_ships.get(index)?;
        ship.name.as_deref().or_else(|| {
            self.capital_ship_classes
                .get(ship.class)
                .map(|class| class.name.as_str())
        })
    }

    /// How this game names its fleets.
    #[must_use]
    pub const fn fleet_naming(&self) -> FleetNaming {
        self.fleet_names.mode
    }

    /// Start a new game's fleet naming in `mode`. port: under Canonical, the
    /// fleets seeded so far trade their numbers for bank names in slot
    /// order; once the bank is spent the rest keep their "Fleet N".
    pub fn start_fleet_naming(&mut self, mode: FleetNaming) {
        self.fleet_names.mode = mode;
        if mode == FleetNaming::Original {
            return;
        }
        let fleets: Vec<FleetKey> = self.fleets.keys().collect();
        for fleet in fleets {
            if let Some(name) = self.free_bank_name(fleet) {
                self.fleet_names.names.insert(fleet, name.to_owned());
            }
        }
        self.name_flagship_fleets();
    }

    /// port: each signature name (`BankName::flagship`) no fleet holds goes
    /// to its side's first fleet, in slot order, that holds a ship of the
    /// flagship's class and that the player has not renamed. That fleet's old name returns to the bank. Only
    /// under Canonical naming.
    pub fn name_flagship_fleets(&mut self) {
        if self.fleet_names.mode != FleetNaming::Canonical {
            return;
        }
        for is_alliance in [true, false] {
            for entry in crate::fleet_name_bank::bank(is_alliance) {
                let Some(class) = entry.flagship else {
                    continue;
                };
                if self.bank_name_held(entry.name) {
                    continue;
                }
                let flagship_fleet = self
                    .fleets
                    .iter()
                    .find(|(key, fleet)| {
                        fleet.is_alliance == is_alliance
                            && !self.fleet_names.renamed.contains_key(*key)
                            && self.holds_class(fleet, class)
                    })
                    .map(|(key, _)| key);
                if let Some(fleet) = flagship_fleet {
                    self.fleet_names.names.insert(fleet, entry.name.to_owned());
                }
            }
        }
    }

    /// The first name in `fleet`'s side's bank that no fleet holds; a
    /// signature name only when `fleet` holds its flagship's class.
    fn free_bank_name(&self, fleet: FleetKey) -> Option<&'static str> {
        let value = self.fleets.get(fleet)?;
        crate::fleet_name_bank::bank(value.is_alliance)
            .iter()
            .filter(|entry| {
                entry
                    .flagship
                    .is_none_or(|class| self.holds_class(value, class))
            })
            .map(|entry| entry.name)
            .find(|name| !self.bank_name_held(name))
    }

    fn bank_name_held(&self, name: &str) -> bool {
        self.fleets
            .keys()
            .any(|fleet| self.fleet_names.names.get(fleet).is_some_and(|held| held == name))
    }

    fn holds_class(&self, fleet: &Fleet, class: crate::ids::DatId) -> bool {
        fleet.capital_ships.iter().any(|ship| {
            ship.alive
                && self
                    .capital_ship_classes
                    .get(ship.class)
                    .is_some_and(|value| value.dat_id == class)
        })
    }

    /// A fleet's name (`FUN_004f62d0`): its own, else the record's.
    #[must_use]
    pub fn fleet_name(&self, fleet: FleetKey) -> Option<&str> {
        self.fleets.get(fleet)?;
        Some(
            self.fleet_names
                .names
                .get(fleet)
                .map_or(FLEET_RECORD_NAME, String::as_str),
        )
    }

    /// Set `side`'s `+0xb8` (`FUN_0052f590(side, 1)`); Neutral has none
    /// (`FUN_0055fc80` takes only sides 1 and 2).
    pub fn set_recruit_pool_empty(&mut self, side: crate::dat::Faction) {
        if let Some(index) = recruit_side_index(side) {
            self.recruit_pool_empty[index] = true;
        }
    }
}

fn recruit_side_index(side: crate::dat::Faction) -> Option<usize> {
    match side {
        crate::dat::Faction::Alliance => Some(0),
        crate::dat::Faction::Empire => Some(1),
        crate::dat::Faction::Neutral => None,
    }
}

#[cfg(test)]
mod facility_ownership_tests {
    use super::*;

    fn system(world: &mut GameWorld, control: ControlKind) -> SystemKey {
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Here".into(),
            sector: SectorKey::default(),
            x: 0,
            y: 0,
            exploration_status: ExplorationStatus::Explored,
            popularity_alliance: 0.5,
            popularity_empire: 0.5,
            is_populated: true,
            total_energy: 4,
            raw_materials: 2,
            espionage_rating: 0.0,
            fleets: Vec::new(),
            ground_units: Vec::new(),
            special_forces: Vec::new(),
            defense_facilities: Vec::new(),
            manufacturing_facilities: Vec::new(),
            production_facilities: Vec::new(),
            fighters: Vec::new(),
            is_headquarters: false,
            is_destroyed: false,
            control,
        })
    }

    /// A catalog class serving the given sides.
    fn class(world: &mut GameWorld, id: u32, is_alliance: bool, is_empire: bool) -> DatId {
        let class = DatId::new(id);
        world.buildable_classes.insert(
            class,
            BuildableClass {
                is_alliance,
                is_empire,
                ..BuildableClass::default()
            },
        );
        class
    }

    fn yard(world: &mut GameWorld, at: SystemKey, class: DatId, side: Faction) -> FacilityRef {
        let key = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: class,
                side,
                is_shipyard: class.family() == 0x28,
            });
        world.systems[at].manufacturing_facilities.push(key);
        FacilityRef::Manufacturing(key)
    }

    fn side_of(world: &GameWorld, facility: FacilityRef) -> Option<Faction> {
        world.facility(facility).map(|(_, side)| side)
    }

    #[test]
    fn a_new_holder_takes_the_facilities_its_classes_serve() {
        // FUN_00510d70 -> FUN_004f6f40 -> FUN_004fae40 -> FUN_004f8680: each
        // facility passes to the system's new side when its class serves it
        // (FUN_004f27d0, the DAT is_alliance / is_empire flags).
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Empire));
        let shipyard = class(&mut world, 0x2800_0001, true, true);
        let facility = yard(&mut world, here, shipyard, Faction::Alliance);
        assert!(world.hand_over_facilities(here).is_empty());
        assert_eq!(side_of(&world, facility), Some(Faction::Empire));
    }

    #[test]
    fn a_facility_its_new_holder_cannot_own_is_removed() {
        // FUN_004f8680: a facility whose class does not serve the new side is
        // removed (+0xac(3)), as the Alliance HQ (ALLFACSD, Alliance only) is
        // when its system goes to the Empire.
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Empire));
        let hq = class(&mut world, 0x2000_0001, true, false);
        let facility = yard(&mut world, here, hq, Faction::Alliance);
        assert_eq!(world.hand_over_facilities(here), vec![facility]);
        assert_eq!(side_of(&world, facility), None);
        assert!(world.systems[here].manufacturing_facilities.is_empty());
    }

    #[test]
    fn a_contested_system_keeps_its_last_holders_facilities() {
        // A battle only sets the system's battle bit; ownership changes only
        // with the side (ghidra/notes/facility-ownership.md, "Contested").
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Contested);
        let shipyard = class(&mut world, 0x2800_0001, true, true);
        let facility = yard(&mut world, here, shipyard, Faction::Alliance);
        assert!(world.hand_over_facilities(here).is_empty());
        assert_eq!(side_of(&world, facility), Some(Faction::Alliance));
    }

    #[test]
    fn a_system_gone_neutral_keeps_only_classes_serving_both_sides() {
        // FUN_004f27d0: neutral (side 3) needs both flags.
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Uncontrolled);
        let both = class(&mut world, 0x2c00_0001, true, true);
        let imperial = class(&mut world, 0x2800_0004, false, true);
        let mine = yard(&mut world, here, both, Faction::Empire);
        let yard_kept = yard(&mut world, here, imperial, Faction::Empire);
        assert_eq!(world.hand_over_facilities(here), vec![yard_kept]);
        assert_eq!(side_of(&world, mine), Some(Faction::Neutral));
    }

    #[test]
    fn the_holders_own_facilities_stay_whatever_their_class() {
        // Only an object of another side is handed over (FUN_004fae40 walks
        // the system and FUN_004f8680 acts on a side change).
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Alliance));
        let imperial = class(&mut world, 0x2800_0004, false, true);
        let facility = yard(&mut world, here, imperial, Faction::Alliance);
        assert!(world.hand_over_facilities(here).is_empty());
        assert_eq!(side_of(&world, facility), Some(Faction::Alliance));
    }

    #[test]
    fn a_class_serves_neutral_only_with_both_flags() {
        let class = |is_alliance, is_empire| BuildableClass {
            is_alliance,
            is_empire,
            ..BuildableClass::default()
        };
        assert!(class(true, false).serves_side(Faction::Alliance));
        assert!(!class(true, false).serves_side(Faction::Empire));
        assert!(class(false, true).serves_side(Faction::Empire));
        assert!(!class(true, false).serves_side(Faction::Neutral));
        assert!(!class(false, true).serves_side(Faction::Neutral));
        assert!(class(true, true).serves_side(Faction::Neutral));
    }

    #[test]
    fn the_facility_holder_follows_control_and_holds_while_contested() {
        assert_eq!(
            GameWorld::facility_holder(ControlKind::Controlled(Faction::Empire)),
            Some(Faction::Empire)
        );
        assert_eq!(
            GameWorld::facility_holder(ControlKind::Uprising(Faction::Alliance)),
            Some(Faction::Alliance)
        );
        assert_eq!(
            GameWorld::facility_holder(ControlKind::Uncontrolled),
            Some(Faction::Neutral)
        );
        assert_eq!(GameWorld::facility_holder(ControlKind::Contested), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fleet(is_alliance: bool) -> Fleet {
        Fleet {
            location: SystemKey::default(),
            capital_ships: Vec::new(),
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance,
            has_death_star: false,
            special_forces: Vec::new(),
        }
    }

    #[test]
    fn each_side_numbers_its_new_fleets_from_one() {
        // FUN_00517760 → FUN_00518750: the counter is the fleet's side's
        // (DAT_006b2bb0 + 0xc4 or + 0xc8), so the sides count apart.
        let mut world = GameWorld::default();
        let first = world.insert_fleet(fleet(true));
        let imperial = world.insert_fleet(fleet(false));
        let second = world.insert_fleet(fleet(true));
        assert_eq!(world.fleet_name(first), Some("Fleet 1"));
        assert_eq!(world.fleet_name(second), Some("Fleet 2"));
        assert_eq!(world.fleet_name(imperial), Some("Fleet 1"));
    }

    #[test]
    fn a_gone_fleets_number_is_not_given_again() {
        // FUN_005302c0 only adds one to the counter's node; nothing lowers it.
        let mut world = GameWorld::default();
        let first = world.insert_fleet(fleet(true));
        world.fleets.remove(first);
        let next = world.insert_fleet(fleet(true));
        assert_eq!(world.fleet_name(next), Some("Fleet 2"));
        assert_eq!(world.fleet_name(first), None);
    }

    #[test]
    fn a_numbered_fleet_keeps_its_name() {
        // FUN_00517760 numbers only a fleet whose name is still the
        // record's (FUN_005f3390 against record +0x34).
        let mut world = GameWorld::default();
        let first = world.insert_fleet(fleet(true));
        world.name_new_fleet(first);
        assert_eq!(world.fleet_name(first), Some("Fleet 1"));
        let second = world.insert_fleet(fleet(true));
        assert_eq!(world.fleet_name(second), Some("Fleet 2"));
    }

    #[test]
    fn an_unnumbered_fleet_bears_the_records_name() {
        // FUN_004f62d0 falls back to the record's +0x34.
        let mut world = GameWorld::default();
        let fleet = world.fleets.insert(fleet(true));
        assert_eq!(world.fleet_name(fleet), Some("Fleet"));
    }

    fn canonical_world() -> GameWorld {
        let mut world = GameWorld::default();
        world.start_fleet_naming(FleetNaming::Canonical);
        world
    }

    /// A fleet of `is_alliance`'s side holding one ship of class `dat_id`.
    fn fleet_with(world: &mut GameWorld, is_alliance: bool, dat_id: DatId) -> Fleet {
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id,
            ..CapitalShipClass::default()
        });
        Fleet {
            capital_ships: vec![ShipInstance::new(class, 100, is_alliance)],
            ..fleet(is_alliance)
        }
    }

    #[test]
    fn a_new_game_names_fleets_by_number_unless_canonical_names_are_chosen() {
        let mut world = GameWorld::default();
        assert_eq!(world.fleet_naming(), FleetNaming::Original);
        let first = world.insert_fleet(fleet(false));
        world.start_fleet_naming(FleetNaming::Original);
        assert_eq!(world.fleet_name(first), Some("Fleet 1"));
    }

    #[test]
    fn canonical_fleets_take_their_sides_first_free_name_skipping_the_signature() {
        let mut world = canonical_world();
        let imperial = world.insert_fleet(fleet(false));
        let rebel = world.insert_fleet(fleet(true));
        let second = world.insert_fleet(fleet(false));
        assert_eq!(world.fleet_name(imperial), Some("Seventh Fleet"));
        assert_eq!(world.fleet_name(second), Some("Third Fleet"));
        assert_eq!(world.fleet_name(rebel), Some("Alpha Group"));
    }

    #[test]
    fn a_living_fleets_name_is_not_given_again_but_a_gone_ones_is() {
        let mut world = canonical_world();
        let first = world.insert_fleet(fleet(true));
        let second = world.insert_fleet(fleet(true));
        assert_eq!(world.fleet_name(second), Some("Beta Group"));
        world.fleets.remove(first);
        let third = world.insert_fleet(fleet(true));
        assert_eq!(world.fleet_name(third), Some("Alpha Group"));
    }

    #[test]
    fn a_spent_bank_falls_back_to_the_sides_running_number() {
        let mut world = canonical_world();
        let bank = crate::fleet_name_bank::ALLIANCE.len();
        for _ in 1..bank {
            world.insert_fleet(fleet(true));
        }
        // The signature name is still free; every other name is held.
        let next = world.insert_fleet(fleet(true));
        assert_eq!(world.fleet_name(next).map(str::to_owned), Some(format!("Fleet {bank}")));
    }

    #[test]
    fn a_new_fleet_with_the_flagship_takes_the_signature_name() {
        let mut world = canonical_world();
        let value = fleet_with(&mut world, false, crate::fleet_name_bank::SUPER_STAR_DESTROYER);
        let squadron = world.insert_fleet(value);
        assert_eq!(world.fleet_name(squadron), Some("Death Squadron"));
    }

    #[test]
    fn a_fleet_gaining_the_flagship_is_renamed_and_frees_its_old_name() {
        let mut world = canonical_world();
        let alpha = world.insert_fleet(fleet(true));
        let cruiser = fleet_with(&mut world, true, crate::fleet_name_bank::MON_CALAMARI_CRUISER);
        world.fleets[alpha].capital_ships = cruiser.capital_ships;
        world.name_flagship_fleets();
        assert_eq!(world.fleet_name(alpha), Some("Rebel Command Fleet"));
        let next = world.insert_fleet(fleet(true));
        assert_eq!(world.fleet_name(next), Some("Alpha Group"));
    }

    #[test]
    fn a_renamed_fleet_keeps_its_name_when_it_gains_the_flagship() {
        // port: the player's name wins over a signature name; the next
        // fleet holding the flagship's class takes it instead.
        let mut world = canonical_world();
        let alpha = world.insert_fleet(fleet(true));
        assert!(world.rename_fleet(alpha, "Home One Group"));
        let cruiser = fleet_with(
            &mut world,
            true,
            crate::fleet_name_bank::MON_CALAMARI_CRUISER,
        );
        world.fleets[alpha].capital_ships = cruiser.capital_ships.clone();
        world.name_flagship_fleets();
        assert_eq!(world.fleet_name(alpha), Some("Home One Group"));
        let second = world.insert_fleet(cruiser);
        world.name_flagship_fleets();
        assert_eq!(world.fleet_name(second), Some("Rebel Command Fleet"));
    }

    #[test]
    fn a_renamed_ship_shows_its_own_name_and_a_destroyed_one_refuses() {
        // FUN_004f6270: the object's +0x34, else its class's name;
        // FUN_004f6e60 refuses a destroyed object (FUN_0053a000).
        let mut world = canonical_world();
        let value = fleet_with(
            &mut world,
            true,
            crate::fleet_name_bank::MON_CALAMARI_CRUISER,
        );
        let fleet = world.insert_fleet(value);
        let class_name = world.ship_name(fleet, 0).map(str::to_owned);
        assert!(class_name.is_some());
        assert!(!world.rename_ship(fleet, 0, ""));
        assert!(world.rename_ship(fleet, 0, "Home One"));
        assert_eq!(world.ship_name(fleet, 0), Some("Home One"));
        world.fleets[fleet].capital_ships[0].alive = false;
        assert!(!world.rename_ship(fleet, 0, "Wreck"));
        assert_eq!(world.ship_name(fleet, 0), Some("Home One"));
        assert!(!world.rename_ship(fleet, 9, "Nobody"));
    }

    #[test]
    fn an_empty_name_leaves_the_fleet_as_it_was() {
        // FUN_004ac950 commits the edit's text only when its length is not 0.
        let mut world = canonical_world();
        let alpha = world.insert_fleet(fleet(true));
        assert!(!world.rename_fleet(alpha, ""));
        assert_eq!(world.fleet_name(alpha), Some("Alpha Group"));
        assert!(world.rename_fleet(alpha, "Renegade"));
        assert_eq!(world.fleet_name(alpha), Some("Renegade"));
        // The bank name it gave up is free again.
        let next = world.insert_fleet(fleet(true));
        assert_eq!(world.fleet_name(next), Some("Alpha Group"));
    }

    #[test]
    fn a_signature_name_held_by_one_fleet_is_not_given_to_another_flagship() {
        let mut world = canonical_world();
        let first = fleet_with(&mut world, false, crate::fleet_name_bank::SUPER_STAR_DESTROYER);
        let first = world.insert_fleet(first);
        let second = fleet_with(&mut world, false, crate::fleet_name_bank::SUPER_STAR_DESTROYER);
        let second = world.insert_fleet(second);
        world.name_flagship_fleets();
        assert_eq!(world.fleet_name(first), Some("Death Squadron"));
        assert_eq!(world.fleet_name(second), Some("Seventh Fleet"));
    }

    #[test]
    fn a_signature_name_waits_for_a_flagship_of_its_own_side() {
        let mut world = canonical_world();
        let rebel = world.insert_fleet(fleet(true));
        // An Imperial fleet holding a (captured) Mon Calamari cruiser.
        let captor = fleet_with(&mut world, false, crate::fleet_name_bank::MON_CALAMARI_CRUISER);
        let captor = world.insert_fleet(captor);
        world.name_flagship_fleets();
        assert_eq!(world.fleet_name(rebel), Some("Alpha Group"));
        assert_eq!(world.fleet_name(captor), Some("Seventh Fleet"));
    }

    #[test]
    fn a_destroyed_flagship_does_not_count() {
        let mut world = canonical_world();
        let mut value = fleet_with(&mut world, false, crate::fleet_name_bank::SUPER_STAR_DESTROYER);
        value.capital_ships[0].alive = false;
        let fleet = world.insert_fleet(value);
        assert_eq!(world.fleet_name(fleet), Some("Seventh Fleet"));
    }

    #[test]
    fn original_naming_never_renames_a_flagship_fleet() {
        let mut world = GameWorld::default();
        let value = fleet_with(&mut world, false, crate::fleet_name_bank::SUPER_STAR_DESTROYER);
        let fleet = world.insert_fleet(value);
        world.name_flagship_fleets();
        assert_eq!(world.fleet_name(fleet), Some("Fleet 1"));
    }

    #[test]
    fn starting_canonical_naming_renames_the_seeded_fleets_in_slot_order() {
        let mut world = GameWorld::default();
        let first = world.insert_fleet(fleet(false));
        let value = fleet_with(&mut world, false, crate::fleet_name_bank::SUPER_STAR_DESTROYER);
        let flagship = world.insert_fleet(value);
        world.start_fleet_naming(FleetNaming::Canonical);
        assert_eq!(world.fleet_naming(), FleetNaming::Canonical);
        assert_eq!(world.fleet_name(first), Some("Seventh Fleet"));
        assert_eq!(world.fleet_name(flagship), Some("Death Squadron"));
    }

    #[test]
    fn seeded_fleets_past_the_bank_keep_their_own_numbers() {
        let mut world = GameWorld::default();
        let bank = crate::fleet_name_bank::ALLIANCE.len();
        let fleets: Vec<FleetKey> = (0..=bank).map(|_| world.insert_fleet(fleet(true))).collect();
        world.start_fleet_naming(FleetNaming::Canonical);
        // Bank minus the signature name: the rest keep "Fleet N".
        assert_eq!(world.fleet_name(fleets[bank - 2]), Some("Phoenix Cell"));
        assert_eq!(
            world.fleet_name(fleets[bank - 1]).map(str::to_owned),
            Some(format!("Fleet {bank}"))
        );
        assert_eq!(
            world.fleet_name(fleets[bank]).map(str::to_owned),
            Some(format!("Fleet {}", bank + 1))
        );
    }

    #[test]
    fn a_mission_record_is_found_by_its_id_not_its_family() {
        // A mission class reads one MISSNSD record through +0x2c
        // (ghidra/notes/mission-lifecycle.md); two records can share a
        // family, as Vacation 0x72 does.
        let record = |raw, min| MissionRecord {
            dat_id: crate::ids::DatId::new(raw),
            timer_min_days: min,
            timer_spread_days: 0,
            repeats: false,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            leave_rule: 0,
            rules: MissionTargetRules::default(),
            members: MissionMemberRules::default(),
        };
        let world = GameWorld {
            mission_records: vec![
                record(0x4100_0001, 0),
                record(0x7200_0045, 60),
                record(0x7200_0046, 1000),
            ],
            ..GameWorld::default()
        };
        let min = |raw| {
            world
                .mission_record(crate::ids::DatId::new(raw))
                .map(|r| r.timer_min_days)
        };

        assert_eq!(min(0x7200_0046), Some(1000));
        assert_eq!(min(0x7200_0045), Some(60));
        assert_eq!(min(0x5100_0010), None);
    }

    /// Helper: create a minimal Character for tests.
    fn default_character() -> Character {
        Character {
            name: "Test".into(),
            ..Default::default()
        }
    }

    #[test]
    fn step_lookup_takes_the_last_threshold_reached_and_clamps_below_the_first() {
        // FUN_00595090 walks the sorted rows to the first threshold above x and
        // returns the row before it, the first row when none precedes it, or the
        // last row when none is above x. Rows are the shipped UPRIS2TB.
        let table = MstbTable::new(
            [(1, 0), (9, 3), (11, 4), (12, 5)]
                .into_iter()
                .map(|(threshold, value)| MstbEntry { threshold, value })
                .collect(),
        );
        assert_eq!(table.step_lookup(-5), 0);
        assert_eq!(table.step_lookup(8), 0);
        assert_eq!(table.step_lookup(9), 3);
        assert_eq!(table.step_lookup(10), 3);
        assert_eq!(table.step_lookup(11), 4);
        assert_eq!(table.step_lookup(40), 5);
        assert_eq!(MstbTable::new(vec![]).step_lookup(3), 0);
        // A first row with a non-zero value tells the clamp from the empty 0.
        let raised = MstbTable::new(vec![
            MstbEntry {
                threshold: 1,
                value: 7,
            },
            MstbEntry {
                threshold: 5,
                value: 9,
            },
        ]);
        assert_eq!(raised.step_lookup(0), 7);
    }

    #[test]
    fn only_the_death_star_record_is_the_death_star() {
        let class = |raw| CapitalShipClass {
            dat_id: DatId::new(raw),
            ..Default::default()
        };
        // CAPSHPSD record 136 is the Death Star (its family +0x10 is 0x18,
        // FUN_005003d0); 131 is a Star Destroyer (0x14).
        assert!(class(DEATH_STAR_CLASS_ID).is_death_star());
        // F-025: family 0x34 is the Emperor (MJCHARSD 0x34000280).
        assert!(!class(0x3400_0280).is_death_star());
        assert!(!class(0x3400_0001).is_death_star());
        assert!(!class(131).is_death_star());
        assert!(!class(0x3000_0088).is_death_star());
    }

    #[test]
    fn character_is_unable_to_betray_serde_roundtrip() {
        let mut c = default_character();
        c.is_unable_to_betray = true;
        let json = serde_json::to_string(&c).unwrap();
        let c2: Character = serde_json::from_str(&json).unwrap();
        assert!(c2.is_unable_to_betray);
    }

    #[test]
    fn captive_character_serde_roundtrip() {
        let mut c = default_character();
        c.is_captive = true;
        c.captured_by = Some(crate::dat::Faction::Empire);
        c.capture_tick = Some(42);
        let json = serde_json::to_string(&c).unwrap();
        let c2: Character = serde_json::from_str(&json).unwrap();
        assert!(c2.is_captive);
        assert_eq!(c2.captured_by, Some(crate::dat::Faction::Empire));
        assert_eq!(c2.capture_tick, Some(42));
    }

    #[test]
    fn default_character_new_fields_are_zero_false_none() {
        let c = default_character();
        assert!(!c.is_unable_to_betray);
        assert!(!c.is_jedi_trainer);
        assert!(!c.is_known_jedi);
        assert_eq!(c.hyperdrive_modifier, 0);
        assert_eq!(c.enhanced_loyalty, 0);
        assert!(!c.on_mission);
        assert!(!c.on_hidden_mission);
        assert!(!c.on_mandatory_mission);
        assert!(c.current_system.is_none());
        assert!(c.current_fleet.is_none());
    }

    #[test]
    fn effective_combat_adds_the_force_share_and_takes_the_injury() {
        // FUN_0053e900: combat + combat * force / 100 (FUN_0053e190) - injury,
        // at least 0 (FUN_0053e120); slot +0x2f0 (FUN_004ef6d0) adds a
        // positive injury, clamped to 0x7fff (FUN_004ee2d0).
        let mut c = Character {
            combat: SkillPair {
                base: 90,
                variance: 0,
            },
            force_base: 50,
            ..Character::default()
        };
        assert_eq!(c.effective_combat(), 135);
        c.injure(35);
        assert_eq!(c.effective_combat(), 100);
        c.injure(-5);
        assert_eq!(c.injury, 35);
        c.injure(1000);
        assert_eq!(c.effective_combat(), 0);
        c.injure(i32::MAX);
        assert_eq!(c.injury, 0x7fff);
        // port: the rolled combat template, base plus half the variance:
        // 80 + 80 * 40 / 100 = 112.
        let c = Character {
            combat: SkillPair {
                base: 70,
                variance: 20,
            },
            force_base: 40,
            ..Character::default()
        };
        assert_eq!(c.effective_combat(), 112);
        // The class Force template is not the Force level: with no Force
        // base, combat gains nothing (FUN_004eee40 sums +0x8e and +0x90).
        let c = Character {
            combat: SkillPair {
                base: 90,
                variance: 0,
            },
            jedi_level: SkillPair {
                base: 50,
                variance: 0,
            },
            ..Character::default()
        };
        assert_eq!(c.effective_combat(), 90);
    }

    #[test]
    fn diplomacy_and_espionage_gain_the_force_share() {
        // FUN_004eecb0 / FUN_004eecf0 -> FUN_0053e940: skill + skill * Force
        // / 100, clamped to 0x7fff (FUN_004edcb0, FUN_004edd30). MJCHARSD
        // 578 (Luke): 50 and 50 at Force 50 give the 75 and 75 of the
        // manual's Figure 3.46 (p. 101).
        let luke = Character {
            diplomacy: SkillPair {
                base: 50,
                variance: 0,
            },
            espionage: SkillPair {
                base: 50,
                variance: 0,
            },
            leadership: SkillPair {
                base: 70,
                variance: 0,
            },
            force_base: 50,
            ..Character::default()
        };
        assert_eq!(luke.effective_skill(Skill::Diplomacy), 75);
        assert_eq!(luke.effective_skill(Skill::Espionage), 75);
        // Leadership takes no Force share (FUN_004eede0 reads +0x92).
        assert_eq!(luke.effective_skill(Skill::Leadership), 70);
        // No Force, no share; port: an unrolled template reads base + half.
        let plain = Character {
            diplomacy: SkillPair {
                base: 60,
                variance: 10,
            },
            ..Character::default()
        };
        assert_eq!(plain.effective_skill(Skill::Diplomacy), 65);
        // The clamp: 0x7000 + 0x7000 * 100 / 100 tops out at 0x7fff.
        let strong = Character {
            espionage: SkillPair {
                base: 0x7000,
                variance: 0,
            },
            force_base: 100,
            ..Character::default()
        };
        assert_eq!(strong.effective_skill(Skill::Espionage), 0x7fff);
        // Combat stays the effective combat, with its injury.
        let fighter = Character {
            combat: SkillPair {
                base: 90,
                variance: 0,
            },
            force_base: 50,
            injury: 10,
            ..Character::default()
        };
        assert_eq!(fighter.effective_skill(Skill::Combat), 125);
    }

    #[test]
    fn the_force_level_sums_experience_and_training() {
        // FUN_004eee40: +0x8c = +0x8e + +0x90, clamped to 0x7fff
        // (FUN_004ee0d0); FUN_004ee1d0 clamps +0x90 to 0..=0x7fff.
        let mut c = Character {
            force_base: 50,
            ..Character::default()
        };
        c.add_force_training(30);
        assert_eq!(c.force_training, 30);
        assert_eq!(c.force_level(), 80);
        c.add_force_training(-100);
        assert_eq!(c.force_training, 0);
        c.force_base = 0x7f00;
        c.add_force_training(0x7fff + 5);
        assert_eq!(c.force_training, 0x7fff);
        assert_eq!(c.force_level(), 0x7fff);
    }

    #[test]
    fn only_a_potential_and_aware_character_without_a_force_base_rolls_one() {
        // FUN_004eee60: +0x8e == 0 and +0xac bits 0x20 and 0x10.
        let due = |force_potential, force_aware, force_base| {
            Character {
                force_potential,
                force_aware,
                force_base,
                ..Character::default()
            }
            .force_base_due()
        };
        assert!(due(true, true, 0));
        assert!(!due(false, true, 0));
        assert!(!due(true, false, 0));
        assert!(!due(true, true, 1));
    }

    #[test]
    fn the_force_base_roll_adds_the_draw_to_the_class_base_up_to_0x7fff() {
        // FUN_004eee60: class +0xa0 plus the draw, through FUN_004ee150's
        // clamp to 0..=0x7fff (FUN_0053e0f0).
        let mut c = Character {
            jedi_level: SkillPair {
                base: 10,
                variance: 40,
            },
            ..Character::default()
        };
        c.roll_force_base(25);
        assert_eq!(c.force_base, 35);
        assert_eq!(c.force_level(), 35);
        c.jedi_level.base = 0x7ff0;
        c.roll_force_base(0x20);
        assert_eq!(c.force_base, 0x7fff);
    }

    #[test]
    fn serde_backward_compat_missing_new_fields() {
        // Simulate deserializing a save file that lacks fields added before v8.
        // NOTE: `heritage_known` and `is_killed` (v8), `recruited` (v21) and
        // `injury` (v68), the Force flags and base (v69) and the Force
        // discoverer (v70) are required here because they have no `#[serde(default)]` — the save
        // bump is the migration boundary, not serde field-default. This test still exercises
        // the `#[serde(default)]` path for earlier fields that legitimately have
        // the attribute.
        let json = r#"{
            "dat_id": 0,
            "name": "Old Save Luke",
            "is_alliance": true,
            "is_empire": false,
            "is_major": true,
            "diplomacy": {"base": 80, "variance": 0},
            "espionage": {"base": 60, "variance": 0},
            "ship_design": {"base": 40, "variance": 0},
            "troop_training": {"base": 50, "variance": 0},
            "facility_design": {"base": 30, "variance": 0},
            "combat": {"base": 90, "variance": 0},
            "leadership": {"base": 85, "variance": 0},
            "loyalty": {"base": 95, "variance": 0},
            "jedi_probability": 100,
            "jedi_level": {"base": 80, "variance": 0},
            "can_be_admiral": true,
            "can_be_commander": true,
            "can_be_general": true,
            "heritage_known": false,
            "is_killed": false,
            "is_retired": false,
            "recruited": false,
            "injury": 0,
            "force_potential": false,
            "force_aware": false,
            "force_base": 0,
            "force_discoverer": null,
            "force_training": 0,
            "current_abode": null,
            "dagobah_completed": false,
            "encounter": null,
            "encounter_day": 0,
            "escape_barred": false,
            "bounty_captured": false,
            "command": "None",
            "command_names": ["", "", "", ""]
        }"#;
        let c: Character = serde_json::from_str(json).unwrap();
        // All pre-v8 fields with `#[serde(default)]` should default gracefully
        assert!(!c.is_unable_to_betray);
        assert!(!c.is_jedi_trainer);
        assert!(!c.is_known_jedi);
        assert_eq!(c.hyperdrive_modifier, 0);
        assert_eq!(c.enhanced_loyalty, 0);
        assert!(!c.on_mission);
        assert!(!c.on_hidden_mission);
        assert!(!c.on_mandatory_mission);
        assert!(c.current_system.is_none());
        assert!(c.current_fleet.is_none());
        assert!(!c.heritage_known);
        assert!(!c.is_killed);
    }

    // ──────────────────────────────────────────────────────────────────────
    // Knesset Shamash-Bet #R11 — Character::mark_killed()
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn mark_killed_sets_is_killed_and_clears_alive_state() {
        // We need concrete slotmap keys that mark_killed can write to. Insert
        // throwaway values into live slotmaps and use the returned keys.
        let mut world = GameWorld::default();
        let sector_key = world.sectors.insert(Sector {
            dat_id: DatId::new(0),
            name: "Sec".into(),
            group: crate::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        let dummy_sys = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0),
            name: "Sys".into(),
            sector: sector_key,
            x: 0,
            y: 0,
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
            control: ControlKind::Uncontrolled,
        });
        let dummy_fleet = world.fleets.insert(Fleet {
            location: dummy_sys,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });

        let mut c = default_character();
        c.current_system = Some(dummy_sys);
        c.current_fleet = Some(dummy_fleet);
        c.on_mission = true;
        c.on_hidden_mission = true;
        c.on_mandatory_mission = true;
        c.is_captive = true;
        c.captured_by = Some(Faction::Empire);
        c.capture_tick = Some(42);

        c.mark_killed();

        assert!(c.is_killed, "is_killed must be set");
        assert_eq!(c.current_system, None, "current_system must be cleared");
        assert_eq!(c.current_fleet, None, "current_fleet must be cleared");
        assert!(!c.on_mission, "on_mission must be cleared");
        assert!(!c.on_hidden_mission, "on_hidden_mission must be cleared");
        assert!(
            !c.on_mandatory_mission,
            "on_mandatory_mission must be cleared"
        );
        assert!(!c.is_captive, "is_captive must be cleared");
        assert_eq!(c.captured_by, None, "captured_by must be cleared");
        assert_eq!(c.capture_tick, None, "capture_tick must be cleared");
    }

    #[test]
    fn number_ships_gives_each_unnumbered_ship_the_next_id() {
        // port: ship ids for the AI's per-ship records (FUN_00484230 keeps
        // one record per ship object).
        let mut world = GameWorld::default();
        let class = CapitalShipKey::default();
        let mut fleet = |ships: Vec<ShipInstance>| {
            world.fleets.insert(Fleet {
                location: SystemKey::default(),
                capital_ships: ships,
                fighters: vec![],
                characters: vec![],
                is_alliance: true,
                has_death_star: false,
                special_forces: Vec::new(),
            })
        };
        let a = fleet(ShipInstance::make(class, 10, true, 2));
        let b = fleet(vec![ShipInstance {
            id: 7,
            ..ShipInstance::new(class, 10, true)
        }]);
        world.last_ship_id = 7;
        world.number_ships();
        let ids = |world: &GameWorld, f| -> Vec<u64> {
            world.fleets[f].capital_ships.iter().map(|s| s.id).collect()
        };
        assert_eq!(ids(&world, a), vec![8, 9]);
        assert_eq!(ids(&world, b), vec![7]);
        world.fleets[b]
            .capital_ships
            .push(ShipInstance::new(class, 10, true));
        world.number_ships();
        assert_eq!(ids(&world, b), vec![7, 10]);
        assert_eq!(world.last_ship_id, 10);
    }

    #[test]
    fn mark_killed_is_idempotent() {
        let mut c = default_character();
        c.mark_killed();
        let snapshot_is_killed = c.is_killed;
        c.mark_killed();
        assert_eq!(c.is_killed, snapshot_is_killed);
    }

    #[test]
    fn mark_killed_preserves_name_and_dat_id() {
        // Reactive story events must still resolve name + dat_id after death
        // (DI-M3 in the Knesset Shamash-Bet plan).
        let mut c = default_character();
        c.name = "Luke".into();
        c.dat_id = DatId::new(0x42);
        c.mark_killed();
        assert_eq!(c.name, "Luke");
        assert_eq!(c.dat_id.raw(), 0x42);
    }

    #[test]
    fn death_star_only_fleet_is_not_empty() {
        let fleet = Fleet {
            location: SystemKey::default(),
            capital_ships: Vec::new(),
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance: false,
            has_death_star: true,
            special_forces: Vec::new(),
        };
        assert!(!fleet.is_empty());
    }

    #[test]
    // FUN_00501510..FUN_00501820 each store four bits of +0x64, at bits
    // 0, 4, 8, 12, 16 and 20 (FUN_0040d6b0..FUN_0040d700 read them).
    fn ship_hit_counters_store_independent_bounded_nibbles() {
        let mut ship = ShipInstance::new(CapitalShipKey::default(), 100, true);
        assert!(Subsystem::ALL.iter().all(|&s| ship.hits(s) == 0));

        ship.set_hits(Subsystem::Shield, 4);
        assert_eq!(ship.hits(Subsystem::Shield), 4);
        assert_eq!(ship.hits(Subsystem::Weapon), 0);

        ship.set_hits(Subsystem::Weapon, 9);
        assert_eq!(ship.hits(Subsystem::Shield), 4);
        assert_eq!(ship.hits(Subsystem::Weapon), 9);

        for (value, subsystem) in (1..).zip(Subsystem::ALL) {
            ship.set_hits(subsystem, value);
        }
        assert_eq!(ship.subsystem_hits, 0x0065_4321);
        ship.set_hits(Subsystem::Shield, u8::MAX);
        ship.set_hits(Subsystem::BackupHyperdrive, u8::MAX);
        assert_eq!(ship.subsystem_hits, 0x00f5_432f);
    }
}
