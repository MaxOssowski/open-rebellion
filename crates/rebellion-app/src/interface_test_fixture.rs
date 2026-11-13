//! Test-only deterministic interface fixture bridge.
//!
//! This module is compiled for the dedicated WASM acceptance artifact and
//! native unit tests. Production builds contain no fixture bridge.

use rebellion_core::blockade::{BlockadeState, BlockadeSystem};
use rebellion_core::dat::{ExplorationStatus, Faction};
use rebellion_core::economy::EconomyState;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{FleetKey, SystemKey, TroopKey};
use rebellion_core::manufacturing::ManufacturingState;
use rebellion_core::missions::{
    available_kinds, MissionFaction, MissionKind, MissionMember, MissionState,
};
use rebellion_core::movement::{
    begin_fleet_transit, fleet_speed, reconcile_fleet_orbits, MovementState,
};
use rebellion_core::research::ResearchState;
use rebellion_core::tick::TickEvent;
use rebellion_core::troop_transport::{regiment_system, TroopTransportState};
use rebellion_core::uprising::UprisingState;
use rebellion_core::world::{ControlKind, GameWorld, ShipInstance, TroopUnit};
use rebellion_render::defenses_window::DEFENSES_WINDOW_WIDTH;
use rebellion_render::fleet_finder::{FinderControl, FinderMode, FinderTab, FleetFinderState};
use rebellion_render::fleet_window::{
    FleetWindowEntry, FleetWindowState, FleetWindowTab, FLEET_WINDOW_WIDTH,
};
use rebellion_render::game_speed::day_readout_rect;
use rebellion_render::mission_dialog::{MissionDialogPage, MissionDialogState};
use rebellion_render::object_menu::{ObjectMenuCommand, ObjectMenuState};
use rebellion_render::quadrant_icons::Quadrant;
use rebellion_render::{
    strategic_primary_controls, CockpitButton, CockpitFaction, CockpitState, GalaxyMapState,
    GameMessage, GidMode, SectorWindowState, SystemWindowState,
};
use rebellion_render::{DefensesPage, DefensesWindowState, MissionsTab, MissionsWindowState};
use serde::Serialize;

use crate::GameMode;

const FIXTURE_ABSENT: u32 = 0;
/// How far right of the galaxy view's centre the targeting scenario puts its
/// target system, clear of the system window it opens on the left.
#[cfg(test)]
const SCENARIO_COUNT: u8 = 116;

extern "C" {
    fn open_rebellion_interface_fixture_code() -> u32;
    fn open_rebellion_interface_fixture_emit(ptr: *const u8, len: usize);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureRequest {
    pub scenario: Scenario,
    pub faction: CockpitFaction,
    pub code: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Scenario {
    Galaxy = 0,
    Sector = 1,
    System = 2,
    DisplayOff = 3,
    PopularSupport = 4,
    Uprising = 5,
    Fleets = 6,
    Personnel = 7,
    Energy = 8,
    RawMaterial = 9,
    Mines = 10,
    Refineries = 11,
    Shipyards = 12,
    Training = 13,
    Construction = 14,
    Defenses = 15,
    MatchingLegend = 16,
    Known = 17,
    Unknown = 18,
    Uninhabited = 19,
    Headquarters = 20,
    Blockade = 21,
    Mission = 22,
    Fleet = 23,
    DeathStarIntel = 24,
    Hover = 25,
    Selection = 26,
    Pan = 27,
    Zoom = 28,
    FleetsEnroute = 29,
    ActivePersonnel = 30,
    IdleShipyards = 31,
    IdleTraining = 32,
    IdleConstruction = 33,
    Troopers = 34,
    FighterSquadrons = 35,
    DeathStarShields = 36,
    PlanetaryShields = 37,
    EncyclopediaArtwork = 38,
    MessageIndexShell = 39,
    EncyclopediaIndexShell = 40,
    EncyclopediaIndexCatalog = 41,
    MissionDialogMission = 42,
    MissionDialogAgents = 43,
    MissionTargeting = 44,
    FleetMove = 45,
    FleetMoveBlockade = 46,
    FleetLoad = 47,
    FleetLoadFull = 48,
    Quadrants = 49,
    RegimentUnloadRefused = 50,
    FleetJoin = 51,
    FleetFinder = 52,
    ProductionDestination = 53,
    BuildSelection = 54,
    /// Two story messages wait in the Mission category: Leia Uses Force and
    /// Jabba Captures Chewbacca (`story_messages`).
    StoryMessages = 55,
    /// One message of each pictured report class waits in the Mission
    /// category (`message_pictures`).
    EventMessages = 56,
    /// An agent free at the primary system and a prisoner of the player's
    /// side alone at the secondary, enemy system, each system's Defenses
    /// window open: a Mission released on the prisoner offers Rescue.
    RescueTargeting = 57,
    /// The agent as in `RescueTargeting`, and the enemy secondary system's
    /// only shipyard first on its System window's Shipyards page.
    SabotageTargeting = 58,
    /// The agent as in `RescueTargeting`, and an enemy fleet carrying the
    /// Death Star at the secondary system, selected in its Fleet window.
    DeathStarTargeting = 59,
    /// As `DeathStarTargeting` with an ordinary enemy capital ship instead,
    /// the fleet alone at the system: a Mission on the ship offers Sabotage.
    ShipTargeting = 60,
    /// The `ShipTargeting` fleet, targeted by its left-list entry: no
    /// mission takes a fleet, so no dialog opens.
    FleetTargeting = 61,
    /// As `MissionTargeting` with a reconnaissance special force (the
    /// Longprobe Y-wing Recon Team, or the Imperial Probe Droid) in the
    /// agent's place: only those classes may undertake Reconnaissance.
    ReconTargeting = 62,
    /// As `MissionTargeting` with the target system the player's, at full
    /// support, holding the player's shipyard, the side's recruit pool
    /// empty, and the agent a ship designer: only Ship Design Research
    /// lists.
    ResearchTargeting = 63,
    /// As `FleetLoad` with a carrier of fighters and two of the player's
    /// squadrons on the primary system's planet, the Defenses window on its
    /// squadron page (`ghidra/notes/planet-squadrons.md`).
    SquadronLoad = 64,
    /// As `FleetLoad` with two of the other side's squadrons on the primary
    /// system's planet: a running clock starts a battle there
    /// (`ghidra/notes/planet-squadrons.md`, "Battle").
    SquadronBattle = 65,
    /// As `FleetLoad` with the primary system the other side's, its
    /// facilities gone and two of that side's regiments on its planet, and
    /// the fleet's ship one that bombards: the fleet icon's Planetary
    /// Bombardment submenu (`ghidra/notes/bombardment-order.md`).
    FleetBombardment = 66,
    /// As `FleetBombardment` with one of the player's regiments aboard the
    /// fleet: the fleet icon's Planetary Assault
    /// (`ghidra/notes/planetary-assault.md`).
    FleetAssault = 67,
    /// As `MissionTargeting` with the first character whose class may be an
    /// admiral as the agent: the Command submenu's orders
    /// (`ghidra/notes/character-command.md`).
    CharacterCommand = 68,
    /// As `CharacterCommand` with the secondary system the player's: a
    /// character's Move (`0x201` -> `0x241`) to its planet
    /// (`ghidra/notes/move-order.md`, "A character's Move").
    CharacterMove = 69,
    /// The `CharacterCommand` agent, and a fleet of the player's side at the
    /// enemy secondary system, selected in its Fleet window: a character's
    /// Move aboard the fleet (`0x241` with the fleet as its target).
    CharacterBoard = 70,
    /// As `CharacterBoard` with the agent aboard the fleet, shown on the
    /// Fleet window's Personnel tab: the drag of a character (manual
    /// p. 111) to the primary system's Defenses window.
    CharacterAboard = 71,
    /// The primary system, the player's, in an uprising with eight of the
    /// player's regiments, and the agent on a Subdue Uprising mission there
    /// (MISSNSD `0x57000080`); the clock runs Fast. The app's mission step
    /// ends the revolt (`FUN_00569c20` -> `FUN_0050c910`).
    SubdueSuccess = 72,
    /// As `FleetBombardment` with the fleet's ship the player's Death Star
    /// (CAPSHPSD 136, the Empire's only) and one of the other side's
    /// characters standing on the planet: the submenu's Destroy System
    /// (`0x223`, `FUN_00582440`, `FUN_0058d990`).
    FleetDestroySystem = 73,
    /// As `ProductionDestination` with a capital ship of the player's one
    /// point from done in the primary shipyard's queue, bound for the
    /// loading target; the clock runs Fast. The ship departs, travels and
    /// arrives (`FUN_0052bee0`, `FUN_0055d8c0`, event `0x387`;
    /// `ghidra/notes/build-delivery.md`).
    ProductionDelivery = 74,
    /// As `FleetLoad` with the primary system, the player's, in revolt,
    /// its uprising incident (event `0x38d`, `FUN_0050d030`) due on day 5
    /// and the disaster (event `0x38f`, `FUN_00511930`) on day 8; a system
    /// with neither energy nor raw materials gets one energy, so the
    /// disaster's pick always strikes. The clock runs Fast
    /// (`ghidra/notes/uprising-incident.md`).
    UprisingIncident = 75,
    /// As `FleetLoad` with the loading target the other side's, held by its
    /// support, and the player's fleet already bound there; the clock runs
    /// Fast. On arrival the fleet blockades it, and Blockade
    /// (`FUN_004960f0`, `0x14e`) tells the player "Fleet Initiates Blockade
    /// of" the target.
    BlockadeArrival = 76,
    /// As `FleetLoad` with the loading target's support all the other
    /// side's; it has no regiment, so the economy's first day hands it over
    /// (`FUN_0050b610`) and System Joins (`FUN_00499760`, `0x100`) tells the
    /// player it "Joins Enemy". The clock runs Fast.
    SupportFlip = 77,
    /// Death Star Sabotaged waits as a story movie (MDATA.104 under its
    /// caption, `FUN_0048d470`); the clock runs Fast, so it plays at once
    /// (`FUN_00488b90`).
    StoryMovie = 78,
    /// The Alliance's Fleet window over Kimanan as A0 shows it
    /// (`.artifacts/a0-vm/menus/23-fleet1-*.png`, day 318): Fleet 1 with
    /// Corellian Corvette 1, Medium Transport 1 and two Alliance Fleet
    /// Regiments, selected; Fleet 4, Corellian Corvette 2, in hyperspace to
    /// Kimanan; the window at A0's (305, 86).
    FleetWindowA0 = 79,
    /// The Alliance's Fleet window over Vjun as A0 shows it
    /// (`.artifacts/a0-vm/fleet2/vjun-fleet5-admiral-*.png`, day 676):
    /// Fleet 5, Corellian Corvette 2, with Garm Bel Iblis aboard as its
    /// Admiral, selected; the window at A0's (305, 86).
    FleetWindowVjun = 80,
    /// The Alliance's Fleet window over Umgul after A0's battle there
    /// (`.artifacts/a0-vm/fleet2/umgul-*.png`, day 722): Fleet 1 with
    /// Corellian Corvette 1 (hull 240 of 500) and Medium Transport 1 (317
    /// of 360, its hyperdrive hit) and two Alliance Fleet Regiments,
    /// selected and expanded; Fleet 3, Medium Transport 2 (damaged) with
    /// two Alliance Army Regiments; the window at A0's (305, 86).
    FleetWindowUmgul = 81,
    /// The Alliance's Fleet window over Selonia as A0 shows it
    /// (`.artifacts/a0-vm/fleet2/selonia-fleet4-*.png`, day 775): Fleet 1,
    /// Corellian Corvette 1 (damaged), in hyperspace to Selonia, expanded;
    /// Fleet 4, Alliance Escort
    /// Carrier 2 with an X-wing and a Y-wing squadron, selected; the window
    /// at A0's (305, 86).
    FleetWindowSelonia = 82,
    /// The Alliance's Fleet window over Umgul with no fleet left there, as
    /// A0 shows it after its fleets withdrew
    /// (`.artifacts/a0-vm/fleet2/umgul-empty.png`, day 766).
    FleetWindowEmpty = 83,
    /// The Alliance's Fleet window over Yavin at the start of A0's `test`
    /// game (`.artifacts/a0-vm/fleet3/01-yavin-fleet1-before-split.png`,
    /// day 9): Fleet 1 with Corellian Corvette 1 and Medium Transport 1
    /// carrying two Alliance Fleet Regiments, selected and expanded; the
    /// window at A0's (305, 86). A0 then split, transferred and merged.
    FleetWindowYavin = 84,
    /// The Alliance's Fleet window over Qat Chrystac, the destination, as
    /// A0 shows it (`.artifacts/a0-vm/fleet4/03-*.png`, day 25): Fleet 8,
    /// Corellian Corvette 1 and Medium Transport 1 with two Alliance Fleet
    /// Regiments, in hyperspace from Yavin, selected; at A0's (305, 86).
    FleetWindowDestination = 85,
    /// The Alliance's Fleet window over Imperial Bortras, blockaded by
    /// Fleet 8 after A0's battle there
    /// (`.artifacts/a0-vm/fleet4/bortras-blockade-*.png`, day 119): Corellian
    /// Corvette 1 damaged, Medium Transport 1 with two Alliance Fleet
    /// Regiments, selected; at A0's (305, 86).
    FleetWindowBlockade = 86,
    /// `BuildSelection` with the player's side over its maintenance, as A0
    /// shows the Empire at -402 (`.artifacts/a0-vm/manufacturing/11`): the
    /// side has no mine or refinery left, so no order with a maintenance
    /// cost fits (`FUN_0052e970`).
    BuildSelectionOverMaintenance = 87,
    /// `BuildSelection` with the secondary system's sector open, so a
    /// band's Destination can be released on its planet (A0
    /// build-selection2/21: the ships band's Destination on Chandrila).
    BuildSelectionDestination = 88,
    /// As `RescueTargeting` with the character at the secondary system the
    /// other side's, free: a Mission released on it offers Assassination
    /// and Abduction (A0 kinds2/50..55: Veers on Leia at Yavin).
    EnemyCharacterTargeting = 89,
    /// As `MissionTargeting` with the target system the player's and in
    /// uprising against it: the dialog lists Subdue Uprising first (A0
    /// kinds2/61..63: Jerjerrod on Selonia lists Subdue Uprising above
    /// Espionage).
    UprisingTargeting = 90,
    /// The primary system's Defenses window at the galaxy view's right
    /// edge, on its Planetary Batteries page, the player's KDY-150 first
    /// (A0 status/22, 23: Coruscant's KDY-150 and its Status).
    BatteryStatus = 91,
    /// Three of the player's characters for the Personnel Finder: the first
    /// on a mission at the primary system, the second travelling to the
    /// secondary, the third a General (A0 specforces/00: "Han Solo - Drall
    /// ( On Mission )", "Adar Tallon - Mon Calamari ( Enroute )", "Huoba
    /// Neva - Pzob ( General )").
    PersonnelStates = 92,
    /// As `MissionTargeting` with a Force user for its agent: a Trainee for
    /// the Alliance, a Jedi Master for the Empire (A0 personnel-states/19:
    /// Luke "Trainee"; status/33-emperor: "Jedi Master").
    ForceStatus = 93,
    /// As `BuildSelectionDestination` with the secondary system's Defenses
    /// window at the galaxy view's left in place of its sector window: a
    /// band's Destination released on it (A0 build-selection2/25:
    /// Coruscant's Defenses window takes the ships band's Destination).
    BuildSelectionDefensesDestination = 94,
    /// As `BuildSelectionDefensesDestination` with a player's fleet moved to
    /// the secondary system and its Fleet window in place of the Defenses
    /// window: a band's Destination released on the fleet is refused (A0
    /// build-selection2/30, 32, 35).
    BuildSelectionFleetDestination = 95,
    /// As `FleetAssault` with no regiment of the other side's on the
    /// planet: the assault takes the system, and the Assault Summary reads
    /// TEXTSTRA `0x7091` (A0 messages-day633/assault-coruscant).
    FleetAssaultTaken = 96,
    /// The player's side has lost: for the Empire, Coruscant is the
    /// Alliance's and the Emperor and Vader captive; for the Alliance, Luke
    /// and Mon Mothma captive (the fixtures' galaxies hold no Alliance
    /// headquarters). The clock runs Fast, so the first day's victory
    /// conditions word (`FUN_005180e0`) ends the game (`FUN_00425c90`),
    /// as A0 empire-clean/60..64 shows for the Empire.
    EndingDefeat = 97,
    /// As `EndingDefeat` with the sides swapped: the player's side wins on
    /// the first day (Alliance MDATA.105, Empire MDATA.108).
    EndingVictory = 98,
    /// The Empire's Fleet window over Coruscant at the start of A0's new
    /// Empire game (`.artifacts/a0-vm/fleet-aboard/*.png`, day 10): Fleet 1,
    /// Imperial Star Destroyer 1 carrying six TIE Fighter squadrons, three
    /// Stormtrooper Regiments and Veers, selected and expanded; the window
    /// at A0's (225, 92).
    FleetWindowCoruscant = 99,
    /// `BuildSelection` with a regiment in training at the primary, part
    /// made: for the Alliance one Alliance Army Regiment one point of its
    /// six in (A0 2026-11-11-manufacturing-focus/06, Averam, day 38), for
    /// the Empire three Imperial Army Regiments, the first three points of
    /// six in (A0 build-selection2/22, Coruscant).
    BuildSelectionTraining = 100,
    /// As `PopularSupport` with four of Moddell's systems as A0
    /// headquarters/00-moddell shows them: explored, uninhabited, held by
    /// no side, with no facility, and A0's energy and raw-material slots.
    ModdellUninhabited = 101,
    /// As `ResearchTargeting` with a troop trainer for the agent and one
    /// training facility the target's only yard: only Troop Training
    /// Research and Espionage list (A0 2026-11-11-mission-kinds/10, 11:
    /// Adar Tallon on his own Averam, one Training Facility, shows Troop
    /// Training Research first of two kinds).
    TroopResearchTargeting = 102,
    /// As `ReconTargeting` with a special force of a class whose en route
    /// mark is STRATEGY 11520 travelling to the primary system: the
    /// Alliance's Infiltrators, the Empire's Imperial Commandos. Its
    /// destination's Defenses window lists it first (A0 sfstatus/01..03:
    /// the Alliance's Infiltrators en route to Yavin, and their Status).
    SpecialForceEnroute = 103,
    /// The primary system, the player's, on its Defenses window's Trooper
    /// Regiments page: an Alliance Army (Imperial Army) Regiment travelling
    /// there by Move from the third system, then one of the same class in
    /// training there (A0 2026-11-11-regiment-en-route/00: Averam lists the
    /// regiment from Yavin, then the one in training).
    RegimentEnroute = 104,
    /// The Alliance's view of the Empire's assault on its Yavin, as A0
    /// assault-clean/01..03 shows it (an unedited Alliance game, day 720):
    /// an Imperial fleet with nine Stormtrooper Regiments aboard takes
    /// Yavin from its one Alliance Army Regiment, and the report the
    /// Conflict rail files (`FUN_00497530` kind 3) is opened as the
    /// message index's Display opens it (`FUN_00469de0`).
    EnemyAssaultTaken = 105,
    /// Coruscant held by the Alliance, as A0 coruscant-676/14 shows it (the
    /// VM's Empire game, day 679, after Get Immediate Result gave the
    /// Alliance Coruscant): one Alliance regiment there and nothing else of
    /// the Alliance's, the system in sight, and its sector window open.
    /// The quadrant gate's second system is Coruscant (`quadrant_systems`).
    CoruscantSeized = 106,
    /// The primary system's yards replaced by a built Advanced Shipyard,
    /// Advanced Training Facility and Advanced Construction Yard of the
    /// player's (MANFACSD 4, 5, 6), its System window open where the Build
    /// Selection scenario's stands, so each facility page shows the base's
    /// built picture first (`FUN_00458fe0`: 9010, 9018, 9026).
    AdvancedBases = 107,
    /// Coruscant the Empire's with ten TIE Fighter squadrons on its Fighter
    /// Squadrons page, as A0 2026-11-11-icon-menu-topic/05 and 06 show its
    /// Defenses window with the list's scroll bar, and its sector window
    /// open. The quadrant gate's second system is Coruscant
    /// (`quadrant_systems`).
    CoruscantSquadrons = 108,
    /// The primary system, the player's, on its Defenses window's Fighter
    /// Squadrons page: one A-wing (TIE Fighter) squadron travelling there by
    /// Move from the third system, which `FUN_00556390` puts in its
    /// destination at once, in hyperspace, as A0 2026-11-11-regiment-en-
    /// route/00 shows for a regiment. A second one travels to the second
    /// system, emptied of the defenses icon's objects, in the open sector
    /// window (`quadrant_systems`), whose defenses icon it alone lights
    /// (`FUN_0045ce80`).
    SquadronEnroute = 109,
    /// As `SquadronLoad` without squadrons on the planet, the fleet's hold
    /// empty: a regiment, an A-wing (TIE Fighter) squadron and a character
    /// of the player's on their way into the fleet by Move from a system in
    /// another sector. `FUN_00556390` puts each in the fleet at once, in
    /// hyperspace, so its Fleet window lists them (`regiment-unload.md`).
    FleetArrivals = 110,
    /// As `FleetJoin`, but the second fleet's one ship was sent by Move
    /// onto the first fleet from a system in another sector and is on its
    /// way: the ship is the first fleet's member at once, in hyperspace
    /// (manual p. 122; `FUN_004ffc90`, `FUN_004feca0`), so the Fleet window
    /// lists one fleet of three ships.
    FleetJoinEnroute = 111,
    /// The Alliance's galaxy as A0 2026-11-13-gid shows it (days 243 to
    /// 258): Kubindi explored, uninhabited and held by no side (B2), and
    /// Adar Tallon alone at Averam on a Troop Training Research mission
    /// there (C0..C2), on Popular Support (`place_gid_a0`).
    GidA0 = 112,
    /// One Alliance special force, of the SpecForces view's first column,
    /// at Yavin and no other of the Alliance's anywhere, as A0
    /// 2026-11-13-gid/E1 shows the Personnel Finder's SpecForces view: one
    /// row, "Yavin" (`place_specforces_yavin`).
    SpecForcesYavin = 113,
    /// As `CoruscantSeized`, with a second Alliance Army Regiment there:
    /// the player's own two regiments on the Defenses window's Trooper
    /// Regiments page, as A0 2026-11-13-drag/30..33 select and drag
    /// Averam's two. The quadrant gate's second system is Coruscant.
    CoruscantRegiments = 114,
    /// As `ResearchTargeting` with the target below full support, so
    /// Diplomacy lists too: Ship Design Research, Diplomacy and Espionage,
    /// in `FUN_0046cab0`'s key order (A0 2026-11-13-mission-dialogs/40..43:
    /// Wedge Antilles on his own Drall, which has a shipyard).
    ResearchDiplomacyTargeting = 115,
}

impl Scenario {
    fn decode(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Galaxy,
            1 => Self::Sector,
            2 => Self::System,
            3 => Self::DisplayOff,
            4 => Self::PopularSupport,
            5 => Self::Uprising,
            6 => Self::Fleets,
            7 => Self::Personnel,
            8 => Self::Energy,
            9 => Self::RawMaterial,
            10 => Self::Mines,
            11 => Self::Refineries,
            12 => Self::Shipyards,
            13 => Self::Training,
            14 => Self::Construction,
            15 => Self::Defenses,
            16 => Self::MatchingLegend,
            17 => Self::Known,
            18 => Self::Unknown,
            19 => Self::Uninhabited,
            20 => Self::Headquarters,
            21 => Self::Blockade,
            22 => Self::Mission,
            23 => Self::Fleet,
            24 => Self::DeathStarIntel,
            25 => Self::Hover,
            26 => Self::Selection,
            27 => Self::Pan,
            28 => Self::Zoom,
            29 => Self::FleetsEnroute,
            30 => Self::ActivePersonnel,
            31 => Self::IdleShipyards,
            32 => Self::IdleTraining,
            33 => Self::IdleConstruction,
            34 => Self::Troopers,
            35 => Self::FighterSquadrons,
            36 => Self::DeathStarShields,
            37 => Self::PlanetaryShields,
            38 => Self::EncyclopediaArtwork,
            39 => Self::MessageIndexShell,
            40 => Self::EncyclopediaIndexShell,
            41 => Self::EncyclopediaIndexCatalog,
            42 => Self::MissionDialogMission,
            43 => Self::MissionDialogAgents,
            44 => Self::MissionTargeting,
            45 => Self::FleetMove,
            46 => Self::FleetMoveBlockade,
            47 => Self::FleetLoad,
            48 => Self::FleetLoadFull,
            49 => Self::Quadrants,
            50 => Self::RegimentUnloadRefused,
            51 => Self::FleetJoin,
            52 => Self::FleetFinder,
            53 => Self::ProductionDestination,
            54 => Self::BuildSelection,
            55 => Self::StoryMessages,
            56 => Self::EventMessages,
            57 => Self::RescueTargeting,
            58 => Self::SabotageTargeting,
            59 => Self::DeathStarTargeting,
            60 => Self::ShipTargeting,
            61 => Self::FleetTargeting,
            62 => Self::ReconTargeting,
            63 => Self::ResearchTargeting,
            64 => Self::SquadronLoad,
            65 => Self::SquadronBattle,
            66 => Self::FleetBombardment,
            67 => Self::FleetAssault,
            68 => Self::CharacterCommand,
            69 => Self::CharacterMove,
            70 => Self::CharacterBoard,
            71 => Self::CharacterAboard,
            72 => Self::SubdueSuccess,
            73 => Self::FleetDestroySystem,
            74 => Self::ProductionDelivery,
            75 => Self::UprisingIncident,
            76 => Self::BlockadeArrival,
            77 => Self::SupportFlip,
            78 => Self::StoryMovie,
            79 => Self::FleetWindowA0,
            80 => Self::FleetWindowVjun,
            81 => Self::FleetWindowUmgul,
            82 => Self::FleetWindowSelonia,
            83 => Self::FleetWindowEmpty,
            84 => Self::FleetWindowYavin,
            85 => Self::FleetWindowDestination,
            86 => Self::FleetWindowBlockade,
            87 => Self::BuildSelectionOverMaintenance,
            88 => Self::BuildSelectionDestination,
            89 => Self::EnemyCharacterTargeting,
            90 => Self::UprisingTargeting,
            91 => Self::BatteryStatus,
            92 => Self::PersonnelStates,
            93 => Self::ForceStatus,
            94 => Self::BuildSelectionDefensesDestination,
            95 => Self::BuildSelectionFleetDestination,
            96 => Self::FleetAssaultTaken,
            97 => Self::EndingDefeat,
            98 => Self::EndingVictory,
            99 => Self::FleetWindowCoruscant,
            100 => Self::BuildSelectionTraining,
            101 => Self::ModdellUninhabited,
            102 => Self::TroopResearchTargeting,
            103 => Self::SpecialForceEnroute,
            104 => Self::RegimentEnroute,
            105 => Self::EnemyAssaultTaken,
            106 => Self::CoruscantSeized,
            107 => Self::AdvancedBases,
            108 => Self::CoruscantSquadrons,
            109 => Self::SquadronEnroute,
            110 => Self::FleetArrivals,
            111 => Self::FleetJoinEnroute,
            112 => Self::GidA0,
            113 => Self::SpecForcesYavin,
            114 => Self::CoruscantRegiments,
            115 => Self::ResearchDiplomacyTargeting,
            _ => return None,
        })
    }

    fn moves_a_fleet(self) -> bool {
        matches!(self, Self::FleetMove | Self::FleetMoveBlockade)
    }

    /// The scenarios whose gate works the primary system's Fleet window.
    fn uses_the_fleet_window(self) -> bool {
        matches!(
            self,
            Self::FleetLoad
                | Self::FleetLoadFull
                | Self::SquadronLoad
                | Self::SquadronBattle
                | Self::FleetBombardment
                | Self::FleetAssault
                | Self::FleetAssaultTaken
                | Self::FleetDestroySystem
                | Self::ProductionDelivery
                | Self::UprisingIncident
                | Self::BlockadeArrival
                | Self::SupportFlip
                | Self::RegimentUnloadRefused
                | Self::FleetJoin
                | Self::ProductionDestination
                | Self::FleetArrivals
                | Self::FleetJoinEnroute
        )
    }

    /// The scenarios whose gate opens Build Selection from the primary
    /// system's Manufacturing window.
    fn opens_build_selection(self) -> bool {
        matches!(
            self,
            Self::BuildSelection
                | Self::BuildSelectionTraining
                | Self::BuildSelectionOverMaintenance
                | Self::BuildSelectionDestination
                | Self::BuildSelectionDefensesDestination
                | Self::BuildSelectionFleetDestination
        )
    }

    /// The scenarios whose gate opens the object pop-up menu.
    fn reports_object_menu(self) -> bool {
        self == Self::MissionTargeting
            || self == Self::ForceStatus
            || self == Self::CharacterCommand
            || self == Self::CharacterMove
            || self == Self::CharacterBoard
            || self == Self::CharacterAboard
            || self == Self::ReconTargeting
            || self == Self::SpecialForceEnroute
            || self == Self::RegimentEnroute
            || self == Self::SquadronEnroute
            || self == Self::ResearchTargeting
            || self == Self::TroopResearchTargeting
            || self == Self::ResearchDiplomacyTargeting
            || self == Self::UprisingTargeting
            || self == Self::BatteryStatus
            || self == Self::PersonnelStates
            || self.targets_an_object()
            || self.opens_build_selection()
            || self == Self::Quadrants
            || self.moves_a_fleet()
            || self.uses_the_fleet_window()
            || self == Self::FleetWindowYavin
            || self == Self::FleetWindowA0
    }

    /// The scenarios whose Mission targets an object in a window.
    fn targets_an_object(self) -> bool {
        matches!(
            self,
            Self::RescueTargeting
                | Self::EnemyCharacterTargeting
                | Self::SabotageTargeting
                | Self::DeathStarTargeting
                | Self::ShipTargeting
                | Self::FleetTargeting
        )
    }

    pub fn mode(self) -> GidMode {
        match self {
            Self::DisplayOff => GidMode::DisplayOff,
            Self::Uprising => GidMode::Uprisings,
            Self::Fleets => GidMode::IdleFleets,
            Self::Personnel => GidMode::IdlePersonnel,
            Self::Energy => GidMode::AvailableEnergy,
            Self::RawMaterial => GidMode::AvailableRawMaterial,
            Self::Mines => GidMode::Mines,
            Self::Refineries => GidMode::Refineries,
            Self::Shipyards => GidMode::Shipyards,
            Self::Training => GidMode::TrainingFacilities,
            Self::Construction => GidMode::ConstructionYards,
            Self::Defenses => GidMode::PlanetaryDefenseBatteries,
            Self::FleetsEnroute => GidMode::FleetsEnRoute,
            Self::ActivePersonnel => GidMode::ActivePersonnel,
            Self::IdleShipyards => GidMode::IdleShipyards,
            Self::IdleTraining => GidMode::IdleTrainingFacilities,
            Self::IdleConstruction => GidMode::IdleConstructionYards,
            Self::Troopers => GidMode::Troopers,
            Self::FighterSquadrons => GidMode::FighterSquadrons,
            Self::DeathStarShields => GidMode::DeathStarShields,
            Self::PlanetaryShields => GidMode::PlanetaryShieldGenerators,
            _ => GidMode::PopularSupport,
        }
    }
}

pub fn requested() -> Option<FixtureRequest> {
    let code = unsafe { open_rebellion_interface_fixture_code() };
    if code == FIXTURE_ABSENT || code >> 16 != 0 {
        return None;
    }
    let scenario = Scenario::decode(((code & 0xff) as u8).checked_sub(1)?)?;
    let faction = match (code >> 8) & 0xff {
        1 => CockpitFaction::Alliance,
        2 => CockpitFaction::Empire,
        _ => return None,
    };
    Some(FixtureRequest {
        scenario,
        faction,
        code,
    })
}

#[allow(clippy::too_many_arguments)]
#[expect(
    clippy::too_many_lines,
    reason = "Keep the deterministic browser fixture setup in its existing order."
)]
/// The deliveries a scenario starts with, after [`apply`]. Build Selection:
/// an Orbital Shipyard, a Mine and a Refinery travelling from the
/// secondary to the primary, their pages' third cells (FUN_00458fe0:
/// base + 3 en route; A0 personnel-states/34, Mon Calamari's 9009,
/// build-selection4/26, Yavin's mine en route after its built ones, 9004,
/// and build-selection4/29, Yavin's refinery en route, 9033).
/// Their departure day lies past any the gate reaches, so they stay en
/// route.
pub fn place_deliveries(
    request: FixtureRequest,
    world: &GameWorld,
    deliveries: &mut rebellion_core::delivery::DeliveryState,
) {
    if request.scenario != Scenario::BuildSelection {
        return;
    }
    let system_keys: Vec<_> = world.systems.keys().take(2).collect();
    let [primary, secondary] = system_keys[..] else {
        return;
    };
    let build = |raw| rebellion_core::manufacturing::FacilityBuild {
        class: rebellion_core::ids::DatId::new(raw),
        is_alliance: request.faction == CockpitFaction::Alliance,
    };
    let departure = |kind| rebellion_core::manufacturing::Departure {
        origin: secondary,
        destination: primary,
        tick: 100_000,
        kind,
    };
    deliveries.depart(
        world,
        &[
            departure(rebellion_core::manufacturing::BuildableKind::ManufacturingFacility(
                build(0x2800_0001),
            )),
            departure(rebellion_core::manufacturing::BuildableKind::ProductionFacility(
                build(0x2c00_0001),
            )),
            departure(rebellion_core::manufacturing::BuildableKind::ProductionFacility(
                build(0x2d00_0002),
            )),
        ],
    );
}

pub fn apply(
    request: FixtureRequest,
    world: &mut GameWorld,
    game_mode: &mut GameMode,
    player_faction: &mut MissionFaction,
    cockpit: &mut CockpitState,
    map: &mut GalaxyMapState,
    movement: &mut MovementState,
    manufacturing: &mut ManufacturingState,
    economy: &mut EconomyState,
    missions: &mut MissionState,
    blockade: &mut BlockadeState,
    sectors: &mut SectorWindowState,
    systems: &mut SystemWindowState,
    fleets: &mut FleetWindowState,
    defenses: &mut DefensesWindowState,
    troop_transport: &mut TroopTransportState,
    uprisings: &mut rebellion_core::uprising::UprisingState,
) {
    *game_mode = GameMode::Galaxy;
    *player_faction = match request.faction {
        CockpitFaction::Alliance => MissionFaction::Alliance,
        CockpitFaction::Empire => MissionFaction::Empire,
    };
    cockpit.faction = request.faction;
    cockpit.gid_mode = request.scenario.mode();
    // As chosen from the GID menu (FUN_00426d00): Display Off hides the
    // legend's button (A0 gid/d-display-off.png).
    cockpit.gid_legend.show_display(cockpit.gid_mode);
    cockpit.gid_ui.menu_open = false;
    cockpit.gid_ui.category = None;

    let system_keys: Vec<_> = world.systems.keys().take(10).collect();
    let Some(&primary) = system_keys.first() else {
        return;
    };
    let secondary = system_keys.get(1).copied().unwrap_or(primary);
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let player_control = if player_is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    let enemy_control = if player_is_alliance {
        Faction::Empire
    } else {
        Faction::Alliance
    };

    for (index, key) in system_keys.iter().copied().enumerate() {
        if let Some(system) = world.systems.get_mut(key) {
            system.exploration_status = ExplorationStatus::Explored;
            system.is_populated = true;
            system.is_destroyed = false;
            system.is_headquarters = false;
            system.popularity_alliance = [0.92, 0.72, 0.55, 0.28][index % 4];
            system.popularity_empire = 1.0 - system.popularity_alliance;
            system.control = ControlKind::Controlled(if index % 2 == 0 {
                player_control
            } else {
                enemy_control
            });
            system.total_energy = 12;
            system.raw_materials = 11;
        }
        economy.per_system.entry(key).or_default().energy_allocated = 3;
        economy
            .per_system
            .entry(key)
            .or_default()
            .raw_material_allocated = 2;
    }

    match request.scenario {
        Scenario::Unknown => {
            world.systems[primary].exploration_status = ExplorationStatus::Unexplored;
        }
        Scenario::Uninhabited => {
            world.systems[primary].is_populated = false;
        }
        Scenario::Headquarters => {
            world.systems[primary].is_headquarters = true;
            move_headquarters_object(world, primary);
        }
        Scenario::Uprising => {
            world.systems[primary].control = ControlKind::Uprising(player_control);
        }
        Scenario::CharacterMove => {
            world.systems[secondary].control = ControlKind::Controlled(player_control);
        }
        _ => {}
    }

    // The agent is the first character; for the Command scenario, the first
    // whose class may be an admiral (FUN_004ed1c0).
    let agent = world
        .characters
        .iter()
        .find(|(_, c)| {
            !matches!(
                request.scenario,
                Scenario::CharacterCommand
                    | Scenario::CharacterMove
                    | Scenario::CharacterBoard
                    | Scenario::CharacterAboard
            ) || c.can_be_admiral
        })
        .map(|(key, _)| key);
    if let Some((character_key, character)) =
        agent.and_then(|key| world.characters.get_mut(key).map(|c| (key, c)))
    {
        character.current_system = Some(primary);
        character.current_fleet = None;
        character.is_alliance = player_is_alliance;
        character.is_empire = !player_is_alliance;
        character.on_mission = matches!(
            request.scenario,
            Scenario::Mission | Scenario::ActivePersonnel
        );
        if matches!(
            request.scenario,
            Scenario::MissionTargeting
                | Scenario::ForceStatus
                | Scenario::ResearchTargeting
                | Scenario::TroopResearchTargeting
                | Scenario::ResearchDiplomacyTargeting
                | Scenario::UprisingTargeting
                | Scenario::CharacterCommand
                | Scenario::CharacterMove
                | Scenario::CharacterBoard
                | Scenario::CharacterAboard
        ) || request.scenario.targets_an_object()
        {
            // MissionState::mission_order_enabled: a free recruited member.
            character.recruited = true;
            character.is_captive = false;
            character.on_mandatory_mission = false;
        }
        if request.scenario == Scenario::ForceStatus {
            // FUN_004486f0's Force Ranking by +0x8c: 50 a Trainee, 120 a
            // Jedi Master.
            character.force_potential = true;
            character.force_aware = true;
            character.force_base = if player_is_alliance { 50 } else { 120 };
            character.force_training = 0;
        }

        if matches!(
            request.scenario,
            Scenario::Mission | Scenario::ActivePersonnel
        ) {
            let faction = if player_is_alliance {
                MissionFaction::Alliance
            } else {
                MissionFaction::Empire
            };
            missions.dispatch(rebellion_core::missions::MissionRequest::single(
                MissionKind::Espionage,
                faction,
                character_key,
                primary,
                None,
                0,
            ));
        }
    }

    if request.scenario == Scenario::SubdueSuccess {
        place_subdue(request, world, missions, uprisings, agent, primary);
    }

    let mut transit_fleet = None;
    if let Some((fleet_key, fleet)) = world.fleets.iter_mut().next() {
        fleet.location = primary;
        fleet.is_alliance = player_is_alliance;
        fleet.has_death_star = request.scenario == Scenario::DeathStarIntel;
        transit_fleet = Some(fleet_key);
    }
    if matches!(request.scenario, Scenario::FleetsEnroute) {
        if let Some(fleet_key) = transit_fleet {
            let _ = begin_fleet_transit(movement, world, fleet_key, secondary, 12);
        }
    }
    reconcile_fleet_orbits(movement, world);

    if request.scenario == Scenario::Blockade {
        world.systems[primary].control = ControlKind::Controlled(player_control);
        if let Some((_, fleet)) = world.fleets.iter_mut().next() {
            fleet.location = primary;
            fleet.is_alliance = !player_is_alliance;
        }
        reconcile_fleet_orbits(movement, world);
        let _ = BlockadeSystem::advance(blockade, world, &[TickEvent { tick: 1 }]);
    }

    if request.scenario == Scenario::Selection {
        map.selected_system = Some(primary);
    }
    if request.scenario == Scenario::Sector {
        sectors.open_for_system(world, primary, request.faction);
    }
    // The Build Selection gate: a shipyard, a training facility and a
    // construction yard of the player's at the primary system, whose
    // Manufacturing window opens on its overview.
    if request.scenario.opens_build_selection() {
        for class in [0x2800_0001, 0x2900_0002, 0x2a00_0003] {
            let yard = world.manufacturing_facilities.insert(
                rebellion_core::world::ManufacturingFacilityInstance {
                    class_dat_id: rebellion_core::ids::DatId::new(class),
                    side: rebellion_core::dat::Faction::of_alliance(player_is_alliance),
                    is_shipyard: class == 0x2800_0001,
                },
            );
            world.systems[primary].manufacturing_facilities.push(yard);
        }
    }
    if request.scenario == Scenario::BuildSelectionDestination {
        sectors.open_for_system(world, secondary, request.faction);
    }
    if request.scenario == Scenario::AdvancedBases {
        place_advanced_bases(world, primary, player_is_alliance);
    }
    if request.scenario == Scenario::BuildSelectionFleetDestination {
        let own = world
            .fleets
            .iter()
            .find(|(_, fleet)| fleet.is_alliance == player_is_alliance)
            .map(|(key, _)| key);
        if let Some(fleet) = own {
            let from = world.fleets[fleet].location;
            if let Some(system) = world.systems.get_mut(from) {
                system.fleets.retain(|&key| key != fleet);
            }
            world.fleets[fleet].location = secondary;
            world.systems[secondary].fleets.push(fleet);
            let layout = cockpit.layout_for(640.0, 480.0);
            let galaxy = layout.galaxy;
            fleets.open(
                world,
                secondary,
                (galaxy.x as i16 + 5, galaxy.y as i16 + 5),
                request.faction,
                layout,
            );
        }
    }
    if request.scenario == Scenario::BuildSelectionDefensesDestination {
        // The secondary is the player's, as a system that builds is
        // (FUN_00538220), and its window lists the holder's side.
        world.systems[secondary].control = ControlKind::Controlled(player_control);
        // An A-wing or TIE Fighter being built for the
        // secondary, its only squadron: A0 pending-facility/23 (Pzob). The
        // primary's yards build it, at a cost no game day here completes.
        let name = if player_is_alliance {
            "A-wing"
        } else {
            "TIE Fighter"
        };
        let class = world
            .fighter_classes
            .iter()
            .find(|(_, class)| class.is_alliance == player_is_alliance && class.name == name)
            .map(|(key, _)| key);
        if let Some(class) = class {
            world.systems[secondary].fighters.clear();
            manufacturing.enqueue(
                primary,
                rebellion_core::manufacturing::QueueItem::new(
                    rebellion_core::manufacturing::BuildableKind::Fighter(class),
                    100_000,
                )
                .delivered_to(secondary),
            );
        }
        // And an Alliance Army (TROOPSD 2) or Stormtrooper (6) regiment in
        // training for it, its only regiment.
        let troop = rebellion_core::ids::DatId::new(if player_is_alliance {
            0x1000_0002
        } else {
            0x1000_0006
        });
        world.systems[secondary].ground_units.clear();
        manufacturing.enqueue(
            primary,
            rebellion_core::manufacturing::QueueItem::new(
                rebellion_core::manufacturing::BuildableKind::Troop(troop),
                100_000,
            )
            .delivered_to(secondary),
        );
        let layout = cockpit.layout_for(640.0, 480.0);
        let galaxy = layout.galaxy;
        defenses.open(
            world,
            secondary,
            (galaxy.x as i16 + 5, galaxy.y as i16 + 5),
            request.faction,
            layout,
        );
    }
    if request.scenario == Scenario::BuildSelectionOverMaintenance {
        remove_production_facilities(world, rebellion_core::dat::Faction::of_alliance(player_is_alliance));
    }
    if request.scenario == Scenario::BuildSelection {
        // A built mine and refinery for the Mines and Refineries pages'
        // pictures (FUN_00458fe0: 9001, 9030).
        let side = rebellion_core::dat::Faction::of_alliance(player_is_alliance);
        for (raw, is_mine) in [(0x2c00_0001, true), (0x2d00_0002, false)] {
            let key = world
                .production_facilities
                .insert(rebellion_core::world::ProductionFacilityInstance {
                    class_dat_id: rebellion_core::ids::DatId::new(raw),
                    side,
                    is_mine,
                });
            world.systems[primary].production_facilities.push(key);
        }
        // One of each page's kinds under construction for the primary,
        // queued at the secondary so the primary's bands stay empty
        // (FUN_00458fe0: base + 2 for side 1, + 1 otherwise; A0
        // personnel-states/35, Xyquine's Advanced Training Facility 9020).
        use rebellion_core::manufacturing::{BuildableKind, FacilityBuild, QueueItem};
        for (raw, manufacturing_kind) in [
            (0x2800_0001, true),
            (0x2900_0005, true),
            (0x2c00_0001, false),
            (0x2d00_0002, false),
        ] {
            let build = FacilityBuild {
                class: rebellion_core::ids::DatId::new(raw),
                is_alliance: player_is_alliance,
            };
            let kind = if manufacturing_kind {
                BuildableKind::ManufacturingFacility(build)
            } else {
                BuildableKind::ProductionFacility(build)
            };
            manufacturing.enqueue(secondary, QueueItem::new(kind, 10).delivered_to(primary));
        }
        // build-selection.mjs reads every facility class at number 1, so
        // the primary keeps a free energy slot past its facilities, the
        // four queued above and the three `place_deliveries` sends, and a
        // raw deposit past its mines (FUN_00507750, the room
        // FUN_005385f0's order needs).
        let system = &mut world.systems[primary];
        let mines = system
            .production_facilities
            .iter()
            .filter(|key| world.production_facilities.get(**key).is_some_and(|value| value.is_mine))
            .count()
            + 2;
        let facilities = system.defense_facilities.len()
            + system.manufacturing_facilities.len()
            + system.production_facilities.len()
            + 7;
        let room = |taken: usize| u8::try_from(taken + 1).unwrap_or(u8::MAX);
        system.total_energy = system.total_energy.max(room(facilities));
        system.raw_materials = system.raw_materials.max(room(mines));
    }
    if matches!(
        request.scenario,
        Scenario::System
            | Scenario::BuildSelection
            | Scenario::BuildSelectionTraining
            | Scenario::BuildSelectionOverMaintenance
            | Scenario::BuildSelectionDestination
            | Scenario::BuildSelectionDefensesDestination
            | Scenario::BuildSelectionFleetDestination
            | Scenario::ProductionDelivery
            | Scenario::AdvancedBases
    ) {
        systems.open(
            world,
            primary,
            // Beside the sector window, whose planets the Destination
            // gate releases on.
            if matches!(
                request.scenario,
                Scenario::BuildSelectionDestination
                    | Scenario::BuildSelectionDefensesDestination
                    | Scenario::BuildSelectionFleetDestination
            ) {
                (360, 76)
            } else {
                (225, 76)
            },
            request.faction,
            cockpit.layout_for(640.0, 480.0),
        );
    }
    if request.scenario.moves_a_fleet() {
        place_moving_fleet(
            request,
            world,
            cockpit,
            movement,
            blockade,
            sectors,
            fleets,
            (
                primary,
                secondary,
                *system_keys.last().unwrap_or(&secondary),
            ),
        );
    }
    if request.scenario.uses_the_fleet_window() {
        place_loading_fleet(
            request,
            world,
            cockpit,
            movement,
            sectors,
            defenses,
            troop_transport,
            primary,
        );
        if request.scenario == Scenario::FleetArrivals {
            if let Some(fleet) = world.fleets.keys().next() {
                send_member_aboard(request, world, missions, fleet);
            }
        }
    }
    if request.scenario == Scenario::FleetWindowA0 {
        place_kimanan(request, world, cockpit, movement, troop_transport, fleets);
    }
    if request.scenario == Scenario::FleetWindowVjun {
        place_vjun(request, world, cockpit, movement, fleets);
    }
    if request.scenario == Scenario::BuildSelectionTraining {
        place_training(request, world, manufacturing, primary);
    }
    if request.scenario == Scenario::ModdellUninhabited {
        place_moddell_uninhabited(world);
    }
    if request.scenario == Scenario::GidA0 {
        place_gid_a0(world, missions);
    }
    if request.scenario == Scenario::SpecForcesYavin {
        place_specforces_yavin(world);
    }
    if request.scenario == Scenario::FleetWindowCoruscant {
        place_coruscant(request, world, cockpit, movement, troop_transport, fleets);
    }
    if request.scenario == Scenario::FleetWindowUmgul {
        place_umgul(request, world, cockpit, movement, troop_transport, fleets);
    }
    if request.scenario == Scenario::FleetWindowSelonia {
        place_selonia(request, world, cockpit, movement, fleets);
    }
    if request.scenario == Scenario::FleetWindowYavin {
        place_yavin(request, world, cockpit, movement, troop_transport, fleets);
    }
    if request.scenario == Scenario::EnemyAssaultTaken {
        place_enemy_assault(world, movement, troop_transport);
    }
    if request.scenario == Scenario::CoruscantSeized {
        place_coruscant_seized(request, world, movement, sectors);
    }
    if request.scenario == Scenario::CoruscantRegiments {
        place_coruscant_seized(request, world, movement, sectors);
        // A0 31's "Alliance Army Regiment" (TROOPSD 2) twice.
        if let Some(coruscant) = coruscant(world) {
            let army = rebellion_core::ids::DatId::new(0x1000_0002);
            for key in world.systems[coruscant].ground_units.clone() {
                if let Some(unit) = world.troops.get_mut(key) {
                    unit.class_dat_id = army;
                }
            }
            let regiment = world.troops.insert(TroopUnit {
                class_dat_id: army,
                is_alliance: true,
                regiment_strength: 100,
            });
            world.systems[coruscant].ground_units.push(regiment);
        }
    }
    if request.scenario == Scenario::CoruscantSquadrons {
        place_coruscant_squadrons(request, world, sectors);
    }
    if request.scenario == Scenario::FleetWindowBlockade {
        place_yavin(request, world, cockpit, movement, troop_transport, fleets);
        blockade_bortras(request, world, cockpit, movement, fleets);
    }
    if request.scenario == Scenario::FleetWindowDestination {
        place_yavin(request, world, cockpit, movement, troop_transport, fleets);
        send_to_qat(request, world, cockpit, movement, fleets);
    }
    if request.scenario == Scenario::FleetWindowEmpty {
        if let Some(umgul) = clear_for_a0(world, "Umgul") {
            let layout = cockpit.layout_for(640.0, 480.0);
            fleets.open(world, umgul, (305, 86), request.faction, layout);
        }
    }
    if request.scenario == Scenario::ProductionDelivery {
        place_delivery(request, world, manufacturing, primary);
    }
    if request.scenario == Scenario::UprisingIncident {
        place_incidents(request, world, uprisings, primary);
    }
    if request.scenario == Scenario::BlockadeArrival {
        place_blockade_arrival(request, world, movement, primary);
    }
    if request.scenario == Scenario::SupportFlip {
        place_support_flip(request, world, primary);
    }
    if request.scenario == Scenario::EndingDefeat {
        place_win(request.faction != CockpitFaction::Alliance, world);
    }
    if request.scenario == Scenario::EndingVictory {
        place_win(request.faction == CockpitFaction::Alliance, world);
    }
    if request.scenario == Scenario::Quadrants {
        place_quadrant_contents(
            request, world, movement, missions, uprisings, sectors, primary,
        );
    }
    if request.scenario == Scenario::FleetFinder {
        place_finder_fleets(request, world, movement, &system_keys);
        map.selected_system = Some(primary);
        sectors.open_for_system(world, primary, request.faction);
    }
    if matches!(
        request.scenario,
        Scenario::RescueTargeting | Scenario::EnemyCharacterTargeting
    ) {
        place_prisoner(request, world, defenses, cockpit, primary, secondary);
    }
    if matches!(
        request.scenario,
        Scenario::CharacterBoard | Scenario::CharacterAboard
    ) {
        place_boarding_fleet(request, world, defenses, fleets, cockpit, primary, secondary);
    }
    if matches!(
        request.scenario,
        Scenario::SabotageTargeting
            | Scenario::DeathStarTargeting
            | Scenario::ShipTargeting
            | Scenario::FleetTargeting
    ) {
        place_sabotage_targets(
            request, world, defenses, systems, fleets, cockpit, primary, secondary,
        );
    }
    if matches!(
        request.scenario,
        Scenario::ReconTargeting | Scenario::SpecialForceEnroute
    ) {
        // The recon special force takes the agent's place: SPECFCSD class 3
        // (mission mask 0x4) for the Alliance, class 5 (0x100) for the
        // Empire, both in Reconnaissance's mask 0x104.
        if let Some(agent) = world.characters.keys().next() {
            world.characters[agent].current_system = None;
        }
        let enroute = request.scenario == Scenario::SpecialForceEnroute;
        // En route: SPECFCSD class 2 (Infiltrators, A0 sfstatus/03) for the
        // Alliance, class 7 (Imperial Commandos) for the Empire; neither is
        // a class FUN_0042c3b0 gives the GOKRES en route mark.
        let class = rebellion_core::ids::DatId::new(match (enroute, player_is_alliance) {
            (false, true) => 0x3c00_0003,
            (false, false) => 0x3c00_0005,
            (true, true) => 0x3c00_0002,
            (true, false) => 0x3c00_0007,
        });
        let unit = world
            .special_forces
            .insert(rebellion_core::world::SpecialForceUnit {
                class_dat_id: class,
                is_alliance: player_is_alliance,
                skills: [0; 8],
                on_mission: false,
            });
        if enroute {
            // A member on its way is listed in its destination's Defenses
            // window (A0 specforces/03) and its Status reads Enroute.
            world.systems[primary].special_forces.clear();
            missions.push_transit(rebellion_core::missions::MemberTransit {
                member: MissionMember::SpecialForce(unit),
                mission_id: 0,
                to: primary,
                arrival: 249,
                abode: None,
            });
        } else {
            world.systems[primary].special_forces = vec![unit];
        }
    }
    if request.scenario == Scenario::PersonnelStates {
        let own: Vec<_> = world
            .characters
            .iter()
            .filter(|(_, character)| character.is_alliance == player_is_alliance && character.recruited)
            .map(|(key, _)| key)
            .take(6)
            .collect();
        // A sixth character, where the side has one, is dead: its row reads
        // "Name - Killed" (FUN_00465bb0, destroyed bit +0x50 bit 3, word
        // 0x8798; Steam guide guide-023: "Ackbar - Killed"). It stands
        // nowhere, so no window lists it.
        if let Some(&dead) = own.get(5) {
            let character = &mut world.characters[dead];
            character.mark_killed();
            character.current_abode = None;
            character.ship = None;
            for fleet in world.fleets.values_mut() {
                fleet.characters.retain(|&key| key != dead);
            }
        }
        // A fifth character, where the side has one, is injured and free
        // where it stands (A0 health-2026-11-09/07-personnel-finder-wedge-
        // injured-day954: "Wedge Antilles - Averam ( Injured )").
        if let Some(&injured) = own.get(4) {
            world.characters[injured].injury = 40;
        }
        // A fourth character, where the side has one, is a captive aboard
        // a ship of the player's first fleet (A0 personnel-captured/
        // finder-day628: "Jan Dodonna - Punisher ( Captured )").
        if let (Some(&captive), Some(fleet)) = (
            own.get(3),
            world
                .fleets
                .iter()
                .find(|(_, fleet)| {
                    fleet.is_alliance == player_is_alliance && !fleet.capital_ships.is_empty()
                })
                .map(|(key, _)| key),
        ) {
            let character = &mut world.characters[captive];
            character.current_system = None;
            character.current_fleet = Some(fleet);
            character.ship = None;
            character.on_mission = false;
            character.is_captive = true;
            character.captured_by = Some(if player_is_alliance {
                Faction::Empire
            } else {
                Faction::Alliance
            });
            character.command = rebellion_core::character_command::CommandKind::None;
            for other in world.fleets.values_mut() {
                other.characters.retain(|&key| key != captive);
            }
            world.fleets[fleet].characters.push(captive);
            world.number_ships();
        }
        if let [busy, travelling, commander, ..] = own[..] {
            let busy_character = &mut world.characters[busy];
            busy_character.current_system = Some(primary);
            busy_character.current_fleet = None;
            busy_character.on_mission = true;
            let walker = &mut world.characters[travelling];
            walker.current_system = None;
            walker.current_fleet = None;
            walker.current_abode = None;
            walker.on_mission = false;
            walker.is_captive = false;
            walker.injury = 0;
            missions.push_transit(rebellion_core::missions::MemberTransit {
                member: MissionMember::Character(travelling),
                mission_id: 0,
                to: secondary,
                arrival: 99,
                abode: None,
            });
            world.characters[commander].command =
                rebellion_core::character_command::CommandKind::General;
        }
    }
    if request.scenario == Scenario::BatteryStatus {
        let class_dat_id = rebellion_core::ids::DatId::new(0x2200_0001);
        let battery = world.defense_facilities.insert(rebellion_core::world::DefenseFacilityInstance {
            class_dat_id,
            side: player_control,
        });
        world.systems[primary].defense_facilities.insert(0, battery);
        let layout = cockpit.layout_for(640.0, 480.0);
        let galaxy = layout.galaxy;
        defenses.open(
            world,
            primary,
            (
                (galaxy.x + galaxy.width - DEFENSES_WINDOW_WIDTH) as i16 - 5,
                galaxy.y as i16 + 5,
            ),
            request.faction,
            layout,
        );
        defenses.show_page(primary, rebellion_render::defenses_window::DefensesPage::Batteries);
    }
    if request.scenario == Scenario::UprisingTargeting {
        // The target revolts against the player's side (ControlKind::
        // Uprising, an active uprising), which Subdue Uprising takes.
        let side = player_control;
        world.systems[secondary].control = ControlKind::Uprising(side);
        uprisings.active_uprisings.insert(
            secondary,
            rebellion_core::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
                next_support_tick: None,
                side: Some(side),
            },
        );
        // A0 kinds2/61..63 offer no Recruitment (the pool was empty), so
        // Subdue Uprising heads FUN_0046cab0's order (key 3, Recruitment 1).
        world.set_recruit_pool_empty(player_control);
    }
    if matches!(
        request.scenario,
        Scenario::ResearchTargeting
            | Scenario::TroopResearchTargeting
            | Scenario::ResearchDiplomacyTargeting
    ) {
        // FUN_0056c510: an own-side target with a working yard of the tree's
        // area, and a member with the tree's skill. Full support ends
        // Diplomacy (FUN_00573ee0) and the empty pool Recruitment
        // (FUN_0056b370), so the dialog lists research alone; the Diplomacy
        // case keeps the side's support below full.
        let full = if request.scenario == Scenario::ResearchDiplomacyTargeting {
            0.72
        } else {
            1.0
        };
        let target = &mut world.systems[secondary];
        target.control = ControlKind::Controlled(player_control);
        target.popularity_alliance = if player_is_alliance { full } else { 1.0 - full };
        target.popularity_empire = 1.0 - target.popularity_alliance;
        // The troop case holds one training facility (MANFACSD family 0x29,
        // area_family) as A0's Averam does, and no shipyard.
        let troop = request.scenario == Scenario::TroopResearchTargeting;
        let yard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(if troop {
                    0x2900_0001
                } else {
                    0x2800_0001
                }),
                side: player_control,
                is_shipyard: !troop,
            },
        );
        world.systems[secondary].manufacturing_facilities = vec![yard];
        world.set_recruit_pool_empty(player_control);
        if let Some(agent) = world.characters.values_mut().next() {
            let skill = |base| rebellion_core::world::SkillPair { base, variance: 0 };
            agent.ship_design = skill(if troop { 0 } else { 40 });
            agent.troop_training = skill(if troop { 40 } else { 0 });
            agent.facility_design = skill(0);
        }
    }
    if matches!(
        request.scenario,
        Scenario::MissionTargeting
            | Scenario::ForceStatus
            | Scenario::ReconTargeting
            | Scenario::SpecialForceEnroute
            | Scenario::RegimentEnroute
            | Scenario::SquadronEnroute
            | Scenario::ResearchTargeting
            | Scenario::TroopResearchTargeting
            | Scenario::ResearchDiplomacyTargeting
            | Scenario::UprisingTargeting
            | Scenario::CharacterCommand
            | Scenario::CharacterMove
    ) {
        // The primary system's Defenses window at the galaxy view's right
        // edge holds the agent first on its personnel page; the second
        // system's planet in its sector window, in the first column on the
        // left, is the target (FUN_0045c830).
        let layout = cockpit.layout_for(640.0, 480.0);
        let galaxy = layout.galaxy;
        defenses.open(
            world,
            primary,
            (
                (galaxy.x + galaxy.width - DEFENSES_WINDOW_WIDTH) as i16 - 5,
                galaxy.y as i16 + 5,
            ),
            request.faction,
            layout,
        );
        sectors.open_for_system(world, secondary, request.faction);
    }
    if request.scenario == Scenario::RegimentEnroute {
        place_regiment_enroute(
            request,
            world,
            movement,
            manufacturing,
            troop_transport,
            defenses,
            primary,
            system_keys.get(2).copied(),
        );
    }
    if request.scenario == Scenario::SquadronEnroute {
        place_squadron_enroute(
            request,
            world,
            troop_transport,
            defenses,
            (primary, secondary),
            system_keys.get(2).copied(),
        );
    }

    let _ = manufacturing;
}

/// The `SquadronEnroute` scenario: the side's FIGHTSD 1 (A-wing) or 5 (TIE
/// Fighter) squadron sent by Move from `origin` to `primary`'s planet,
/// whose Defenses window shows its Fighter Squadrons page with the
/// travelling squadron alone (the planet's own of the side leave it), and
/// another to `secondary`, the player's with nothing the defenses icon
/// counts.
fn place_squadron_enroute(
    request: FixtureRequest,
    world: &mut GameWorld,
    troop_transport: &mut TroopTransportState,
    defenses: &mut DefensesWindowState,
    (primary, secondary): (SystemKey, SystemKey),
    origin: Option<SystemKey>,
) {
    let alliance = request.faction == CockpitFaction::Alliance;
    let side = rebellion_core::dat::Faction::of_alliance(alliance);
    let index = if alliance { 1 } else { 5 };
    let Some(class) = world
        .fighter_classes
        .iter()
        .find(|(_, class)| class.dat_id.index() == index && class.is_alliance == alliance)
        .map(|(key, _)| key)
    else {
        return;
    };
    let classes = &world.fighter_classes;
    world.systems[primary].fighters.retain(|entry| {
        classes
            .get(entry.class)
            .is_none_or(|value| value.is_alliance != alliance)
    });
    let empty = (secondary != primary).then_some(secondary);
    if let Some(secondary) = empty {
        let value = &mut world.systems[secondary];
        value.control = ControlKind::Controlled(side);
        value.ground_units.clear();
        value.fighters.clear();
        value.defense_facilities.clear();
        value.special_forces.clear();
        for character in world.characters.values_mut() {
            if character.current_system == Some(secondary) {
                character.current_system = Some(primary);
            }
        }
    }
    if let Some(origin) = origin.filter(|&origin| origin != primary && origin != secondary) {
        world.systems[origin].control = ControlKind::Controlled(side);
        rebellion_core::squadrons::add(&mut world.systems[origin].fighters, class, 2);
        for to in std::iter::once(primary).chain(empty) {
            let _ = troop_transport.squadrons.move_squadrons(
                world,
                alliance,
                class,
                1,
                rebellion_core::squadrons::SquadronPlace::Planet(origin),
                rebellion_core::squadrons::SquadronPlace::Planet(to),
                0,
            );
        }
    }
    defenses.show_page(
        primary,
        rebellion_render::defenses_window::DefensesPage::Squadrons,
    );
}

/// The `RegimentEnroute` scenario: TROOPSD 2 (the Alliance Army Regiment)
/// or 8 (the Imperial Army Regiment), both classes `FUN_0042c3b0` gives
/// their own GOKRES en route mark, one sent by Move from `origin` to
/// `primary` and one in training at `primary`, whose Defenses window shows
/// its Trooper Regiments page.
#[allow(clippy::too_many_arguments)]
fn place_regiment_enroute(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &MovementState,
    manufacturing: &mut ManufacturingState,
    troop_transport: &mut TroopTransportState,
    defenses: &mut DefensesWindowState,
    primary: SystemKey,
    origin: Option<SystemKey>,
) {
    let alliance = request.faction == CockpitFaction::Alliance;
    let side = rebellion_core::dat::Faction::of_alliance(alliance);
    let class = rebellion_core::ids::DatId::new(if alliance { 0x1000_0002 } else { 0x1000_0008 });
    // The page lists only these two: the primary's own regiments of the
    // side leave its list.
    let troops = &world.troops;
    world.systems[primary].ground_units.retain(|key| {
        troops
            .get(*key)
            .is_none_or(|troop| troop.is_alliance != alliance)
    });
    if let Some(origin) = origin.filter(|&origin| origin != primary) {
        world.systems[origin].control = ControlKind::Controlled(side);
        let troop = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: class,
            is_alliance: alliance,
            regiment_strength: 100,
        });
        world.systems[origin].ground_units.push(troop);
        let _ = troop_transport.move_regiment(
            world,
            movement,
            &std::collections::HashSet::new(),
            troop,
            rebellion_core::troop_transport::RegimentTarget::System(primary),
            0,
        );
    }
    manufacturing.enqueue(
        primary,
        rebellion_core::manufacturing::QueueItem::new(
            rebellion_core::manufacturing::BuildableKind::Troop(class),
            100_000,
        )
        .delivered_to(primary),
    );
    defenses.show_page(
        primary,
        rebellion_render::defenses_window::DefensesPage::Regiments,
    );
}

/// The Rescue scenario: the second character becomes a prisoner of the
/// player's side held by the enemy at `secondary`, the only character
/// there, so it fills the first cell of that system's personnel page
/// (`FUN_004eec60` gives it the captor's side). The agent's Defenses window
/// opens at the galaxy view's right edge, the prison's at its left edge.
fn place_prisoner(
    request: FixtureRequest,
    world: &mut GameWorld,
    defenses: &mut DefensesWindowState,
    cockpit: &CockpitState,
    primary: SystemKey,
    secondary: SystemKey,
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let Some(prisoner) = world.characters.keys().nth(1) else {
        return;
    };
    for (key, character) in &mut world.characters {
        if key != prisoner && character.current_system == Some(secondary) {
            character.current_system = Some(primary);
        }
    }
    for fleet in world.fleets.values_mut() {
        fleet.characters.retain(|&key| key != prisoner);
    }
    // EnemyCharacterTargeting: the other side's character, free.
    let enemy = request.scenario == Scenario::EnemyCharacterTargeting;
    let character = &mut world.characters[prisoner];
    character.current_system = Some(secondary);
    character.current_fleet = None;
    character.is_alliance = player_is_alliance != enemy;
    character.is_empire = player_is_alliance == enemy;
    character.is_captive = !enemy;
    character.captured_by = (!enemy).then_some(if player_is_alliance {
        Faction::Empire
    } else {
        Faction::Alliance
    });
    character.is_killed = false;
    character.on_mission = false;
    character.on_mandatory_mission = false;
    character.recruited = true;

    let layout = cockpit.layout_for(640.0, 480.0);
    let galaxy = layout.galaxy;
    defenses.open(
        world,
        primary,
        (
            (galaxy.x + galaxy.width - DEFENSES_WINDOW_WIDTH) as i16 - 5,
            galaxy.y as i16 + 5,
        ),
        request.faction,
        layout,
    );
    defenses.open(
        world,
        secondary,
        (galaxy.x as i16 + 5, galaxy.y as i16 + 5),
        request.faction,
        layout,
    );
}

/// The Sabotage scenarios: the agent's Defenses window at the galaxy
/// view's right edge, and at its left edge either the enemy secondary
/// system's System window on its Shipyards page, holding one enemy shipyard,
/// or its Fleet window with an enemy fleet carrying the Death Star selected.
#[expect(
    clippy::too_many_arguments,
    reason = "The scenario opens three kinds of window over the world."
)]
fn place_sabotage_targets(
    request: FixtureRequest,
    world: &mut GameWorld,
    defenses: &mut DefensesWindowState,
    systems: &mut SystemWindowState,
    fleets: &mut FleetWindowState,
    cockpit: &CockpitState,
    primary: SystemKey,
    secondary: SystemKey,
) {
    let enemy = if request.faction == CockpitFaction::Alliance {
        Faction::Empire
    } else {
        Faction::Alliance
    };
    let layout = cockpit.layout_for(640.0, 480.0);
    let galaxy = layout.galaxy;
    defenses.open(
        world,
        primary,
        (
            (galaxy.x + galaxy.width - DEFENSES_WINDOW_WIDTH) as i16 - 5,
            galaxy.y as i16 + 5,
        ),
        request.faction,
        layout,
    );
    let left = (galaxy.x as i16 + 5, galaxy.y as i16 + 5);
    if request.scenario == Scenario::SabotageTargeting {
        let class_dat_id = world
            .manufacturing_facilities
            .values()
            .find(|facility| facility.is_shipyard)
            .map_or(rebellion_core::ids::DatId::new(0x2800_0001), |facility| {
                facility.class_dat_id
            });
        let yard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id,
                side: enemy,
                is_shipyard: true,
            },
        );
        world.systems[secondary].manufacturing_facilities = vec![yard];
        systems.open_tab(
            world,
            secondary,
            rebellion_render::SystemWindowTab::Shipyards,
            left,
            request.faction,
            layout,
        );
        return;
    }
    let wants_death_star = request.scenario == Scenario::DeathStarTargeting;
    let Some(class) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| {
            class.is_death_star() == wants_death_star
                && (wants_death_star || class.is_alliance == (enemy == Faction::Alliance))
        })
        .map(|(key, _)| key)
    else {
        return;
    };
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: secondary,
        capital_ships: vec![ShipInstance::new(class, 100, true)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: enemy == Faction::Alliance,
        has_death_star: wants_death_star,
        special_forces: Vec::new(),
    });
    if wants_death_star {
        world.systems[secondary].fleets.push(fleet);
    } else {
        world.systems[secondary].fleets = vec![fleet];
    }
    fleets.open(world, secondary, left, request.faction, layout);
    fleets.select(secondary, FleetWindowEntry::Fleet(fleet));
}

/// The `FleetWindowA0` scenario: A0's fleets at Kimanan, named as A0 names
/// them, and its Fleet window open where A0's lay.
fn place_kimanan(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    troop_transport: &mut TroopTransportState,
    fleets: &mut FleetWindowState,
) {
    let system = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let class = |world: &GameWorld, name: &str| {
        world
            .capital_ship_classes
            .iter()
            .find(|(_, value)| value.is_alliance && value.name == name)
            .map(|(key, _)| key)
    };
    let (Some(kimanan), Some(corvette), Some(transport)) = (
        system(world, "Kimanan"),
        class(world, "Corellian Corvette"),
        class(world, "Medium Transport"),
    ) else {
        return;
    };
    // Fleet 4 sets out from another system of Kimanan's sector (Orus).
    let sector = world.systems[kimanan].sector;
    let Some(origin) = world
        .systems
        .iter()
        .find(|(key, value)| *key != kimanan && value.sector == sector)
        .map(|(key, _)| key)
    else {
        return;
    };
    for value in world.fleets.values_mut() {
        if value.location == kimanan {
            value.location = origin;
        }
    }
    let value = &mut world.systems[kimanan];
    value.control = ControlKind::Controlled(Faction::Alliance);
    value.exploration_status = ExplorationStatus::Explored;
    value.ground_units.clear();
    value.special_forces.clear();
    // At full hull: A0's ships carry no damage mark.
    let ship = |world: &GameWorld, class| {
        ShipInstance::new(class, world.capital_ship_classes[class].hull as i32, true)
    };
    let fleet = |capital_ships| rebellion_core::world::Fleet {
        location: kimanan,
        capital_ships,
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    };
    let first = world.insert_fleet(fleet(vec![ship(world, corvette), ship(world, transport)]));
    let mut fourth = fleet(vec![ship(world, corvette)]);
    fourth.location = origin;
    let fourth = world.insert_fleet(fourth);
    world.rename_fleet(first, "Fleet 1");
    world.rename_fleet(fourth, "Fleet 4");
    world.rename_ship(first, 0, "Corellian Corvette 1");
    world.rename_ship(first, 1, "Medium Transport 1");
    world.rename_ship(fourth, 0, "Corellian Corvette 2");
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let regiments: Vec<_> = (0..2)
        .map(|_| {
            world.troops.insert(TroopUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x1000_0001),
                is_alliance: true,
                regiment_strength: 100,
            })
        })
        .collect();
    world.systems[kimanan].ground_units = regiments.clone();
    let _ = troop_transport.embark(world, first, &regiments);
    let ticks = rebellion_core::movement::fleet_transit_ticks(&world.fleets[fourth], world, origin, kimanan)
        .unwrap_or(10);
    movement.order(fourth, origin, kimanan, ticks);
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, kimanan, (305, 86), request.faction, layout);
    fleets.select(kimanan, FleetWindowEntry::Fleet(first));
}

/// The `FleetWindowCoruscant` scenario: A0's Fleet 1 at Coruscant in a new
/// Empire game, and its Fleet window open where A0's lay.
fn place_coruscant(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    troop_transport: &mut TroopTransportState,
    fleets: &mut FleetWindowState,
) {
    let coruscant = world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Coruscant")
        .map(|(key, _)| key);
    let destroyer = world
        .capital_ship_classes
        .iter()
        .find(|(_, value)| value.is_empire && value.name == "Imperial Star Destroyer")
        .map(|(key, _)| key);
    let tie = world
        .fighter_classes
        .iter()
        .find(|(_, value)| !value.is_alliance && value.name == "TIE Fighter")
        .map(|(key, _)| key);
    let veers = world
        .characters
        .iter()
        .find(|(_, value)| value.name == "Veers")
        .map(|(key, _)| key);
    let stormtroopers = world
        .buildable_classes
        .iter()
        .find(|(class, value)| class.family() == 0x10 && value.name == "Stormtrooper Regiment")
        .map(|(class, _)| *class);
    let (Some(coruscant), Some(destroyer), Some(tie), Some(veers), Some(stormtroopers)) =
        (coruscant, destroyer, tie, veers, stormtroopers)
    else {
        return;
    };
    let sector = world.systems[coruscant].sector;
    let Some(elsewhere) = world
        .systems
        .iter()
        .find(|(key, value)| *key != coruscant && value.sector == sector)
        .map(|(key, _)| key)
    else {
        return;
    };
    for value in world.fleets.values_mut() {
        if value.location == coruscant {
            value.location = elsewhere;
        }
    }
    let value = &mut world.systems[coruscant];
    value.control = ControlKind::Controlled(Faction::Empire);
    value.exploration_status = ExplorationStatus::Explored;
    value.ground_units.clear();
    value.special_forces.clear();
    let ship = ShipInstance::new(destroyer, world.capital_ship_classes[destroyer].hull as i32, true);
    // A0 lists six TIE Fighter squadrons, one item each.
    let squadrons = (0..6)
        .map(|_| rebellion_core::world::FighterEntry {
            class: tie,
            count: 1,
            carrier: None,
        })
        .collect();
    let first = world.insert_fleet(rebellion_core::world::Fleet {
        location: coruscant,
        capital_ships: vec![ship],
        fighters: squadrons,
        characters: Vec::new(),
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.rename_fleet(first, "Fleet 1");
    world.rename_ship(first, 0, "Imperial Star Destroyer 1");
    world.number_ships();
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let regiments: Vec<_> = (0..3)
        .map(|_| {
            world.troops.insert(TroopUnit {
                class_dat_id: stormtroopers,
                is_alliance: false,
                regiment_strength: 100,
            })
        })
        .collect();
    world.systems[coruscant].ground_units = regiments.clone();
    let _ = troop_transport.embark(world, first, &regiments);
    world.systems[coruscant].ground_units.clear();
    rebellion_core::missions::board_fleet(world, rebellion_core::missions::MissionMember::Character(veers), first);
    rebellion_core::missions::seat_characters(world, first);
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, coruscant, (225, 92), request.faction, layout);
    fleets.select(coruscant, FleetWindowEntry::Fleet(first));
    // A0 opened it with a double click on the entry, which expands it.
    fleets.expand(coruscant, first);
}

/// The MANFACSD classes of the advanced bases: the Advanced Shipyard,
/// Advanced Training Facility and Advanced Construction Yard (records 4, 5
/// and 6, families 0x28, 0x29 and 0x2a).
const ADVANCED_BASES: [u32; 3] = [0x2800_0004, 0x2900_0005, 0x2a00_0006];

/// The `AdvancedBases` scenario: the primary's yards replaced by one built
/// advanced base of each kind, the player's, so each is its page's first
/// cell.
fn place_advanced_bases(world: &mut GameWorld, primary: SystemKey, player_is_alliance: bool) {
    world.systems[primary].manufacturing_facilities.clear();
    for class in ADVANCED_BASES {
        let base = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(class),
                side: rebellion_core::dat::Faction::of_alliance(player_is_alliance),
                is_shipyard: class >> 24 == 0x28,
            },
        );
        world.systems[primary].manufacturing_facilities.push(base);
    }
}

/// The world's Coruscant.
fn coruscant(world: &GameWorld) -> Option<SystemKey> {
    world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Coruscant")
        .map(|(key, _)| key)
}

/// The `CoruscantSeized` scenario: Coruscant the Alliance's, as A0
/// coruscant-676/14 shows its Defenses window after the seizure. Its tabs
/// there: Personnel pressed over an empty list, Trooper Regiments full, the
/// rest empty (`FUN_004a9ce0`'s art for side 1). So Coruscant keeps one
/// Alliance Army Regiment (`0x1000_0001`) and nothing else; the rest of
/// what stood there goes to a system of another sector.
fn place_coruscant_seized(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &mut MovementState,
    sectors: &mut SectorWindowState,
) {
    let Some(coruscant) = coruscant(world) else {
        return;
    };
    let sector = world.systems[coruscant].sector;
    let Some(elsewhere) = world
        .systems
        .iter()
        .find(|(_, value)| value.sector != sector)
        .map(|(key, _)| key)
    else {
        return;
    };
    let value = &mut world.systems[coruscant];
    value.control = ControlKind::Controlled(Faction::Alliance);
    value.exploration_status = ExplorationStatus::Explored;
    value.is_populated = true;
    value.is_destroyed = false;
    value.defense_facilities.clear();
    let troops = std::mem::take(&mut value.ground_units);
    let forces = std::mem::take(&mut value.special_forces);
    world.systems[elsewhere].ground_units.extend(troops);
    world.systems[elsewhere].special_forces.extend(forces);
    for fleet in world.fleets.values_mut() {
        if fleet.location == coruscant {
            fleet.location = elsewhere;
        }
    }
    reconcile_fleet_orbits(movement, world);
    for character in world.characters.values_mut() {
        if character.current_system == Some(coruscant) {
            character.current_system = Some(elsewhere);
        }
    }
    let regiment = world.troops.insert(TroopUnit {
        class_dat_id: rebellion_core::ids::DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[coruscant].ground_units.push(regiment);
    sectors.open_for_system(world, coruscant, request.faction);
}

/// The `CoruscantSquadrons` scenario: Coruscant the Empire's, its
/// squadrons there ten TIE Fighter squadrons, as A0
/// 2026-11-11-icon-menu-topic/05 lists on its Fighter Squadrons page: four
/// lines of the list's three, past its 210 pixels, so the list shows its
/// bar (`FUN_004a8790`: `FUN_0060a490` with RCDATA 10700).
fn place_coruscant_squadrons(
    request: FixtureRequest,
    world: &mut GameWorld,
    sectors: &mut SectorWindowState,
) {
    let Some(coruscant) = coruscant(world) else {
        return;
    };
    let Some(tie) = world
        .fighter_classes
        .iter()
        .find(|(_, value)| !value.is_alliance && value.name == "TIE Fighter")
        .map(|(key, _)| key)
    else {
        return;
    };
    let value = &mut world.systems[coruscant];
    value.control = ControlKind::Controlled(Faction::Empire);
    value.exploration_status = ExplorationStatus::Explored;
    value.fighters.retain(|entry| entry.carrier.is_some());
    rebellion_core::squadrons::add(&mut value.fighters, tie, 10);
    sectors.open_for_system(world, coruscant, request.faction);
}

/// The quadrant gate's two systems: in `Quadrants` the primary and its
/// sector's next system, in `CoruscantSeized` and `CoruscantSquadrons` the
/// sector's first other system and Coruscant.
fn quadrant_systems(request: FixtureRequest, world: &GameWorld) -> Option<(SystemKey, SystemKey)> {
    match request.scenario {
        Scenario::Quadrants => {
            let primary = world.systems.keys().next()?;
            Some((primary, loading_target(world, primary)?))
        }
        Scenario::CoruscantSeized
        | Scenario::CoruscantSquadrons
        | Scenario::CoruscantRegiments => {
            let coruscant = coruscant(world)?;
            Some((loading_target(world, coruscant)?, coruscant))
        }
        // The second system, in the open sector window, both times.
        Scenario::SquadronEnroute => {
            let second = world.systems.keys().nth(1)?;
            Some((second, second))
        }
        _ => None,
    }
}

/// The `FleetWindowVjun` scenario: A0's Fleet 5 at Vjun with Garm Bel
/// Iblis aboard as its Admiral, and its Fleet window open where A0's lay.
fn place_vjun(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    fleets: &mut FleetWindowState,
) {
    let vjun = world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Vjun")
        .map(|(key, _)| key);
    let corvette = world
        .capital_ship_classes
        .iter()
        .find(|(_, value)| value.is_alliance && value.name == "Corellian Corvette")
        .map(|(key, _)| key);
    let garm = world
        .characters
        .iter()
        .find(|(_, value)| value.name == "Garm Bel Iblis")
        .map(|(key, _)| key);
    let (Some(vjun), Some(corvette), Some(garm)) = (vjun, corvette, garm) else {
        return;
    };
    let sector = world.systems[vjun].sector;
    let Some(elsewhere) = world
        .systems
        .iter()
        .find(|(key, value)| *key != vjun && value.sector == sector)
        .map(|(key, _)| key)
    else {
        return;
    };
    for value in world.fleets.values_mut() {
        if value.location == vjun {
            value.location = elsewhere;
        }
    }
    let value = &mut world.systems[vjun];
    value.control = ControlKind::Controlled(Faction::Alliance);
    value.exploration_status = ExplorationStatus::Explored;
    // At full hull: A0's corvette carries no damage mark.
    let ship = ShipInstance::new(corvette, world.capital_ship_classes[corvette].hull as i32, true);
    let fifth = world.insert_fleet(rebellion_core::world::Fleet {
        location: vjun,
        capital_ships: vec![ship],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.rename_fleet(fifth, "Fleet 5");
    world.rename_ship(fifth, 0, "Corellian Corvette 2");
    world.number_ships();
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    rebellion_core::missions::board_fleet(world, rebellion_core::missions::MissionMember::Character(garm), fifth);
    rebellion_core::missions::seat_characters(world, fifth);
    world.characters[garm].set_command(rebellion_core::character_command::CommandKind::Admiral);
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, vjun, (305, 86), request.faction, layout);
    fleets.select(vjun, FleetWindowEntry::Fleet(fifth));
    // A0 opened it with a double click on the entry, which expands it.
    fleets.expand(vjun, fifth);
}

/// The `FleetWindowUmgul` scenario: A0's Fleets 1 and 3 at Umgul after
/// the battle there, and its Fleet window open where A0's lay.
fn place_umgul(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    troop_transport: &mut TroopTransportState,
    fleets: &mut FleetWindowState,
) {
    let umgul = world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Umgul")
        .map(|(key, _)| key);
    let class = |world: &GameWorld, name: &str| {
        world
            .capital_ship_classes
            .iter()
            .find(|(_, value)| value.is_alliance && value.name == name)
            .map(|(key, _)| key)
    };
    let (Some(umgul), Some(corvette), Some(transport)) = (
        umgul,
        class(world, "Corellian Corvette"),
        class(world, "Medium Transport"),
    ) else {
        return;
    };
    let sector = world.systems[umgul].sector;
    let Some(elsewhere) = world
        .systems
        .iter()
        .find(|(key, value)| *key != umgul && value.sector == sector)
        .map(|(key, _)| key)
    else {
        return;
    };
    for value in world.fleets.values_mut() {
        if value.location == umgul {
            value.location = elsewhere;
        }
    }
    let value = &mut world.systems[umgul];
    value.control = ControlKind::Controlled(Faction::Alliance);
    value.exploration_status = ExplorationStatus::Explored;
    value.ground_units.clear();
    value.special_forces.clear();
    // A0's Capital Ship Status windows (fleet2/05, 06): hull 240:500 and
    // 317:360; Medium Transport 1's Hyperdrive Rating 0:100, its primary
    // hyperdrive hit (CAPSHPSD hyperdrive_if_damaged 0).
    let ship = |world: &GameWorld, class, hull: Option<i32>| {
        let mut ship = ShipInstance::new(class, world.capital_ship_classes[class].hull as i32, true);
        if let Some(hull) = hull {
            ship.hull_current = hull;
        }
        ship
    };
    let fleet = |capital_ships| rebellion_core::world::Fleet {
        location: umgul,
        capital_ships,
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    };
    let mut hit = ship(world, transport, Some(317));
    hit.set_hits(rebellion_core::world::Subsystem::PrimaryHyperdrive, 1);
    let first = world.insert_fleet(fleet(vec![ship(world, corvette, Some(240)), hit]));
    // A0 marks Fleet 3's entry damaged too (umgul-fleet1-damaged-ships);
    // its hull was not read, and any damage shows the same mark.
    let third_hull = world.capital_ship_classes[transport].hull as i32 - 1;
    let third = world.insert_fleet(fleet(vec![ship(world, transport, Some(third_hull))]));
    world.rename_fleet(first, "Fleet 1");
    world.rename_fleet(third, "Fleet 3");
    world.rename_ship(first, 0, "Corellian Corvette 1");
    world.rename_ship(first, 1, "Medium Transport 1");
    world.rename_ship(third, 0, "Medium Transport 2");
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    for (fleet, class) in [(first, 0x1000_0001), (third, 0x1000_0002)] {
        let regiments: Vec<_> = (0..2)
            .map(|_| {
                world.troops.insert(TroopUnit {
                    class_dat_id: rebellion_core::ids::DatId::new(class),
                    is_alliance: true,
                    regiment_strength: 100,
                })
            })
            .collect();
        world.systems[umgul].ground_units = regiments.clone();
        let _ = troop_transport.embark(world, fleet, &regiments);
    }
    world.systems[umgul].ground_units.clear();
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, umgul, (305, 86), request.faction, layout);
    fleets.select(umgul, FleetWindowEntry::Fleet(first));
    fleets.expand(umgul, first);
}

/// The system named `name`, emptied for an A0 copy: its fleets moved to
/// another system of its sector, held by the Alliance, explored, with no
/// troops or special forces on it.
fn clear_for_a0(world: &mut GameWorld, name: &str) -> Option<SystemKey> {
    let here = world
        .systems
        .iter()
        .find(|(_, value)| value.name == name)
        .map(|(key, _)| key)?;
    let sector = world.systems[here].sector;
    let elsewhere = world
        .systems
        .iter()
        .find(|(key, value)| *key != here && value.sector == sector)
        .map(|(key, _)| key)?;
    for value in world.fleets.values_mut() {
        if value.location == here {
            value.location = elsewhere;
        }
    }
    let fleets = std::mem::take(&mut world.systems[here].fleets);
    world.systems[elsewhere].fleets.extend(fleets);
    let value = &mut world.systems[here];
    value.control = ControlKind::Controlled(Faction::Alliance);
    value.exploration_status = ExplorationStatus::Explored;
    value.ground_units.clear();
    value.special_forces.clear();
    Some(here)
}

/// The `FleetWindowSelonia` scenario: A0's Fleets 1 and 4 at Selonia, and
/// its Fleet window open where A0's lay.
fn place_selonia(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    fleets: &mut FleetWindowState,
) {
    let class = |world: &GameWorld, name: &str| {
        world
            .capital_ship_classes
            .iter()
            .find(|(_, value)| value.is_alliance && value.name == name)
            .map(|(key, _)| key)
    };
    let fighter = |world: &GameWorld, name: &str| {
        world
            .fighter_classes
            .iter()
            .find(|(_, value)| value.is_alliance && value.name == name)
            .map(|(key, _)| key)
    };
    let (Some(corvette), Some(carrier), Some(xwing), Some(ywing)) = (
        class(world, "Corellian Corvette"),
        class(world, "Alliance Escort Carrier"),
        fighter(world, "X-wing"),
        fighter(world, "Y-wing"),
    ) else {
        return;
    };
    let Some(selonia) = clear_for_a0(world, "Selonia") else {
        return;
    };
    // Fleet 1 withdrew from Umgul and is still in hyperspace to Selonia:
    // A0 draws its engine glow, and its orders are grey (fleet2/08).
    let sector = world.systems[selonia].sector;
    let Some(origin) = world
        .systems
        .iter()
        .find(|(key, value)| *key != selonia && value.sector == sector)
        .map(|(key, _)| key)
    else {
        return;
    };
    let ship = |world: &GameWorld, class| {
        ShipInstance::new(class, world.capital_ship_classes[class].hull as i32, true)
    };
    // A0 shows Corellian Corvette 1 damaged (its hull 240:500 on day 728).
    let mut damaged = ship(world, corvette);
    damaged.hull_current = 240;
    let fleet = |capital_ships, fighters| rebellion_core::world::Fleet {
        location: selonia,
        capital_ships,
        fighters,
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    };
    let squadron = |class| rebellion_core::world::FighterEntry {
        class,
        count: 1,
        carrier: None,
    };
    let mut withdrawing = fleet(vec![damaged], Vec::new());
    withdrawing.location = origin;
    let first = world.insert_fleet(withdrawing);
    let fourth = world.insert_fleet(fleet(
        vec![ship(world, carrier)],
        vec![squadron(xwing), squadron(ywing)],
    ));
    world.rename_fleet(first, "Fleet 1");
    world.rename_fleet(fourth, "Fleet 4");
    world.rename_ship(first, 0, "Corellian Corvette 1");
    world.rename_ship(fourth, 0, "Alliance Escort Carrier 2");
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let ticks = rebellion_core::movement::fleet_transit_ticks(&world.fleets[first], world, origin, selonia)
        .unwrap_or(10);
    movement.order(first, origin, selonia, ticks);
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, selonia, (305, 86), request.faction, layout);
    fleets.select(selonia, FleetWindowEntry::Fleet(fourth));
    fleets.expand(selonia, first);
}

/// The `FleetWindowYavin` scenario: A0's Fleet 1 at Yavin on day 9, and
/// its Fleet window open where A0's lay.
fn place_yavin(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    troop_transport: &mut TroopTransportState,
    fleets: &mut FleetWindowState,
) {
    let class = |world: &GameWorld, name: &str| {
        world
            .capital_ship_classes
            .iter()
            .find(|(_, value)| value.is_alliance && value.name == name)
            .map(|(key, _)| key)
    };
    let (Some(corvette), Some(transport)) = (
        class(world, "Corellian Corvette"),
        class(world, "Medium Transport"),
    ) else {
        return;
    };
    let Some(yavin) = clear_for_a0(world, "Yavin") else {
        return;
    };
    let ship = |world: &GameWorld, class| {
        ShipInstance::new(class, world.capital_ship_classes[class].hull as i32, true)
    };
    let first = world.insert_fleet(rebellion_core::world::Fleet {
        location: yavin,
        capital_ships: vec![ship(world, corvette), ship(world, transport)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: true,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.rename_fleet(first, "Fleet 1");
    world.rename_ship(first, 0, "Corellian Corvette 1");
    world.rename_ship(first, 1, "Medium Transport 1");
    world.number_ships();
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let regiments: Vec<_> = (0..2)
        .map(|_| {
            world.troops.insert(TroopUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x1000_0001),
                is_alliance: true,
                regiment_strength: 100,
            })
        })
        .collect();
    world.systems[yavin].ground_units = regiments.clone();
    let _ = troop_transport.embark(world, first, &regiments);
    world.systems[yavin].ground_units.clear();
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, yavin, (305, 86), request.faction, layout);
    fleets.select(yavin, FleetWindowEntry::Fleet(first));
    fleets.expand(yavin, first);
}

/// The Stormtrooper Regiments aboard the `EnemyAssaultTaken` fleet: A0
/// assault-clean/02's Imperial Forces list overflows with a 38-pixel thumb
/// on its scroll bar (`FUN_0060a490(.., 7, 0x29fc, ..)`), nine 70-pixel
/// rows.
const ENEMY_ASSAULT_REGIMENTS: usize = 9;

/// The `EnemyAssaultTaken` scenario: Yavin the Alliance's with one
/// Alliance Army Regiment and no defense facility, and an Imperial fleet
/// of Imperial Star Destroyers in orbit carrying
/// [`ENEMY_ASSAULT_REGIMENTS`] Stormtrooper Regiments (A0 assault-clean:
/// the Empire's Fleet 13 took Yavin, its System Forces lost an Alliance
/// Army Regiment).
fn place_enemy_assault(
    world: &mut GameWorld,
    movement: &mut MovementState,
    troop_transport: &mut TroopTransportState,
) {
    let troop_class = |world: &GameWorld, name: &str| {
        world
            .buildable_classes
            .iter()
            .find(|(class, value)| class.family() == 0x10 && value.name == name)
            .map(|(class, _)| *class)
    };
    let destroyer = world
        .capital_ship_classes
        .iter()
        .find(|(_, value)| value.is_empire && value.name == "Imperial Star Destroyer")
        .map(|(key, _)| key);
    let (Some(stormtroopers), Some(army), Some(destroyer)) = (
        troop_class(world, "Stormtrooper Regiment"),
        troop_class(world, "Alliance Army Regiment"),
        destroyer,
    ) else {
        return;
    };
    let Some(yavin) = clear_for_a0(world, "Yavin") else {
        return;
    };
    world.systems[yavin].defense_facilities.clear();
    let capacity = world.capital_ship_classes[destroyer].troop_capacity.max(1) as usize;
    let hull = world.capital_ship_classes[destroyer].hull as i32;
    let ships = ENEMY_ASSAULT_REGIMENTS.div_ceil(capacity);
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: yavin,
        capital_ships: (0..ships)
            .map(|_| ShipInstance::new(destroyer, hull, false))
            .collect(),
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.rename_fleet(fleet, "Fleet 13");
    world.number_ships();
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let regiments: Vec<_> = (0..ENEMY_ASSAULT_REGIMENTS)
        .map(|_| {
            world.troops.insert(TroopUnit {
                class_dat_id: stormtroopers,
                is_alliance: false,
                regiment_strength: 100,
            })
        })
        .collect();
    world.systems[yavin].ground_units = regiments.clone();
    let _ = troop_transport.embark(world, fleet, &regiments);
    let defender = world.troops.insert(TroopUnit {
        class_dat_id: army,
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[yavin].ground_units = vec![defender];
}

/// The `EnemyAssaultTaken` scenario's assault: the Empire's fleets at
/// Yavin assault it (`FUN_0058c120`) with the first seed whose draws give
/// A0 assault-clean's outcome: the defending regiment destroyed, no
/// attacker lost, and the system taken (TEXTSTRA `0x7091`). Returns the
/// reports for the app to present.
pub fn enemy_assault(
    request: FixtureRequest,
    world: &mut GameWorld,
    troop_transport: &mut TroopTransportState,
) -> Vec<rebellion_core::assault_report::AssaultReport> {
    if request.scenario != Scenario::EnemyAssaultTaken {
        return Vec::new();
    }
    let Some(yavin) = world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Yavin")
        .map(|(key, _)| key)
    else {
        return Vec::new();
    };
    let fleets: Vec<FleetKey> = world
        .fleets
        .iter()
        .filter(|(_, fleet)| fleet.location == yavin && !fleet.is_alliance)
        .map(|(key, _)| key)
        .collect();
    for seed in 0..256 {
        let mut trial_world = world.clone();
        let mut trial_transport = troop_transport.clone();
        let mut env = rebellion_core::bombardment::BombardEnv::new(seed, None);
        let outcome = env
            .assault(
                &mut trial_world,
                &mut trial_transport,
                Faction::Empire,
                &fleets,
                yavin,
            )
            .clone();
        let as_a0 = outcome.lost_attackers.is_empty()
            && outcome.lost_defenders.len() == 1
            && env.reports.iter().all(|report| report.taken);
        if as_a0 && !env.reports.is_empty() {
            *world = trial_world;
            *troop_transport = trial_transport;
            return env.reports;
        }
    }
    Vec::new()
}

/// The `FleetWindowDestination` scenario: `place_yavin`'s fleet, renamed
/// as A0's "Fleet 8", ordered from Yavin to Qat Chrystac; Yavin's window
/// closed and the destination's opened where A0's lay, the fleet selected.
fn send_to_qat(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    fleets: &mut FleetWindowState,
) {
    let system = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let (Some(yavin), Some(qat)) = (system(world, "Yavin"), system(world, "Qat Chrystac")) else {
        return;
    };
    let Some(&fleet) = world.systems[yavin].fleets.first() else {
        return;
    };
    for value in world.fleets.values_mut() {
        if value.location == qat {
            value.location = yavin;
        }
    }
    let moved = std::mem::take(&mut world.systems[qat].fleets);
    world.systems[yavin].fleets.extend(moved);
    world.systems[qat].control = ControlKind::Controlled(Faction::Alliance);
    world.systems[qat].exploration_status = ExplorationStatus::Explored;
    world.rename_fleet(fleet, "Fleet 8");
    let ticks = rebellion_core::movement::fleet_transit_ticks(&world.fleets[fleet], world, yavin, qat)
        .unwrap_or(10);
    movement.order(fleet, yavin, qat, ticks);
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);

    fleets.clear();
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, qat, (305, 86), request.faction, layout);
    fleets.select(qat, FleetWindowEntry::Fleet(fleet));
}

/// Remove `side`'s mines and refineries, leaving it no maintenance
/// capacity.
fn remove_production_facilities(world: &mut GameWorld, side: rebellion_core::dat::Faction) {
    let removed: Vec<_> = world
        .production_facilities
        .iter()
        .filter(|(_, facility)| facility.side == side)
        .map(|(key, _)| key)
        .collect();
    for key in &removed {
        world.production_facilities.remove(*key);
    }
    for system in world.systems.values_mut() {
        system.production_facilities.retain(|key| !removed.contains(key));
    }
}

/// The `FleetWindowBlockade` scenario: `place_yavin`'s fleet, as A0's
/// "Fleet 8", moved to Imperial Bortras with its corvette damaged (A0
/// shows its flames; its hull was not read), the Bortras window opened
/// where A0's lay with the fleet selected.
fn blockade_bortras(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    fleets: &mut FleetWindowState,
) {
    let system = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let (Some(yavin), Some(bortras)) = (system(world, "Yavin"), system(world, "Bortras")) else {
        return;
    };
    let Some(&fleet) = world.systems[yavin].fleets.first() else {
        return;
    };
    for value in world.fleets.values_mut() {
        if value.location == bortras {
            value.location = yavin;
        }
    }
    let others = std::mem::take(&mut world.systems[bortras].fleets);
    world.systems[yavin].fleets.extend(others);
    world.systems[yavin].fleets.retain(|&key| key != fleet);
    world.systems[bortras].fleets.push(fleet);
    world.fleets[fleet].location = bortras;
    let value = &mut world.systems[bortras];
    value.control = ControlKind::Controlled(Faction::Empire);
    value.exploration_status = ExplorationStatus::Explored;
    value.ground_units.clear();
    value.special_forces.clear();
    world.rename_fleet(fleet, "Fleet 8");
    let ship = &mut world.fleets[fleet].capital_ships[0];
    ship.hull_current -= ship.hull_current / 2;
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    fleets.clear();
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, bortras, (305, 86), request.faction, layout);
    fleets.select(bortras, FleetWindowEntry::Fleet(fleet));
}

/// Whether the scenario runs the clock Fast from the start.
#[must_use]
pub fn runs_the_clock(request: FixtureRequest) -> bool {
    matches!(
        request.scenario,
        Scenario::SubdueSuccess
            | Scenario::ProductionDelivery
            | Scenario::UprisingIncident
            | Scenario::BlockadeArrival
            | Scenario::SupportFlip
            | Scenario::StoryMovie
            | Scenario::EndingDefeat
            | Scenario::EndingVictory
    )
}

/// The `SubdueSuccess` scenario: the primary system, the player's, revolts
/// against it; eight of the player's regiments garrison it; the agent, a
/// free recruited character there, is on a Subdue Uprising mission.
fn place_subdue(
    request: FixtureRequest,
    world: &mut GameWorld,
    missions: &mut MissionState,
    uprisings: &mut rebellion_core::uprising::UprisingState,
    agent: Option<rebellion_core::ids::CharacterKey>,
    primary: SystemKey,
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let side = if player_is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    world.systems[primary].control = ControlKind::Uprising(side);
    uprisings.active_uprisings.insert(
        primary,
        rebellion_core::uprising::ActiveUprising {
            started_tick: 0,
            next_incident_tick: None,
            next_support_tick: None,
            side: Some(side),
        },
    );
    let class_dat_id = world
        .troops
        .values()
        .find(|troop| troop.is_alliance == player_is_alliance)
        .map_or_else(
            || {
                rebellion_core::ids::DatId::new(if player_is_alliance {
                    0x1000_0001
                } else {
                    0x1000_0006
                })
            },
            |troop| troop.class_dat_id,
        );
    for _ in 0..8 {
        let troop = world.troops.insert(TroopUnit {
            class_dat_id,
            is_alliance: player_is_alliance,
            regiment_strength: 100,
        });
        world.systems[primary].ground_units.push(troop);
    }
    let Some(agent) = agent else {
        return;
    };
    let character = &mut world.characters[agent];
    character.recruited = true;
    character.is_captive = false;
    character.on_mandatory_mission = false;
    let faction = if player_is_alliance {
        MissionFaction::Alliance
    } else {
        MissionFaction::Empire
    };
    let _ = missions.dispatch_guarded(
        rebellion_core::missions::MissionRequest::single(
            MissionKind::SubdueUprising,
            faction,
            agent,
            primary,
            None,
            0,
        ),
        world,
    );
}

/// The `CharacterBoard` scenario: the agent in the primary system's
/// Defenses window at the galaxy view's right edge, and a fleet of the
/// player's side at the enemy secondary system, selected in its Fleet
/// window at the left edge.
fn place_boarding_fleet(
    request: FixtureRequest,
    world: &mut GameWorld,
    defenses: &mut DefensesWindowState,
    fleets: &mut FleetWindowState,
    cockpit: &CockpitState,
    primary: SystemKey,
    secondary: SystemKey,
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let layout = cockpit.layout_for(640.0, 480.0);
    let galaxy = layout.galaxy;
    defenses.open(
        world,
        primary,
        (
            (galaxy.x + galaxy.width - DEFENSES_WINDOW_WIDTH) as i16 - 5,
            galaxy.y as i16 + 5,
        ),
        request.faction,
        layout,
    );
    let Some(class) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| !class.is_death_star() && class.is_alliance == player_is_alliance)
        .map(|(key, _)| key)
    else {
        return;
    };
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: secondary,
        capital_ships: vec![ShipInstance::new(class, 100, true)],
        fighters: Vec::new(),
        characters: Vec::new(),
        is_alliance: player_is_alliance,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.systems[secondary].fleets = vec![fleet];
    // CharacterAboard: the agent (the first who may be an admiral) is
    // aboard, on the Personnel tab.
    let aboard = (request.scenario == Scenario::CharacterAboard)
        .then(|| {
            world
                .characters
                .iter()
                .find(|(_, c)| c.can_be_admiral)
                .map(|(key, _)| key)
        })
        .flatten();
    if let Some(agent) = aboard {
        world.characters[agent].move_to(Some(fleet), Some(secondary));
        world.fleets[fleet].characters.push(agent);
    }
    let left = (galaxy.x as i16 + 5, galaxy.y as i16 + 5);
    fleets.open(world, secondary, left, request.faction, layout);
    fleets.select(secondary, FleetWindowEntry::Fleet(fleet));
    if aboard.is_some() {
        fleets.show_tab(secondary, FleetWindowTab::Personnel);
    }
}

/// The Personnel Finder's Characters rows while it is open, in the
/// PersonnelStates scenario.
pub fn emit_personnel_finder(
    request: FixtureRequest,
    world: &GameWorld,
    fog: &FogState,
    finder: &rebellion_render::personnel_finder::PersonnelFinderState,
) {
    if request.scenario == Scenario::SpecForcesYavin {
        emit_finder_list(request, world, fog, finder);
        return;
    }
    if request.scenario != Scenario::PersonnelStates {
        return;
    }
    let Some(rows) = finder.shown_character_rows(world, fog) else {
        return;
    };
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let Some((alliance_tab, tabs)) = finder.shown_side_tabs(layout) else {
        return;
    };
    #[derive(Serialize)]
    struct Report {
        status: &'static str,
        rows: Vec<String>,
        /// The side tab shown, and each side tab's centre (Alliance,
        /// Imperial).
        tab: &'static str,
        tabs: [(f32, f32); 2],
    }
    let bytes = serde_json::to_vec(&Report {
        status: "personnel-finder",
        rows,
        tab: if alliance_tab { "alliance" } else { "imperial" },
        tabs: tabs.map(|point| (point.x, point.y)),
    })
    .expect("serialize the personnel finder report");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// The Personnel Finder's shown rows in either view, its chosen row and its
/// name box's text, in the SpecForcesYavin scenario.
fn emit_finder_list(
    request: FixtureRequest,
    world: &GameWorld,
    fog: &FogState,
    finder: &rebellion_render::personnel_finder::PersonnelFinderState,
) {
    let Some((rows, chosen, name)) = finder.shown_list(world, fog) else {
        return;
    };
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let Some((alliance_tab, tabs)) = finder.shown_side_tabs(layout) else {
        return;
    };
    #[derive(Serialize)]
    struct Report {
        status: &'static str,
        rows: Vec<String>,
        chosen: Option<usize>,
        name: String,
        tab: &'static str,
        tabs: [(f32, f32); 2],
    }
    let bytes = serde_json::to_vec(&Report {
        status: "personnel-finder",
        rows,
        chosen,
        name,
        tab: if alliance_tab { "alliance" } else { "imperial" },
        tabs: tabs.map(|point| (point.x, point.y)),
    })
    .expect("serialize the personnel finder list");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// The open Status window's title, name and rows: a Character Status in the
/// ForceStatus and FleetArrivals scenarios, a facility's (the headquarters') in Quadrants, the
/// travelling special force's in SpecialForceEnroute, regiment's in
/// RegimentEnroute and the receiving fleet's or ship's in FleetArrivals
/// (with the picture's marks), and an unfinished unit's or a producer's in any.
pub fn emit_status_rows(
    request: FixtureRequest,
    sources: rebellion_render::status_rows::StatusSources<'_>,
    status: &rebellion_render::status_window::StatusWindowState,
) {
    use rebellion_render::status_rows::StatusObject;
    let mut marks = Vec::new();
    let (title, name, rows) = match (request.scenario, status.object()) {
        (
            Scenario::ForceStatus | Scenario::FleetArrivals,
            Some(StatusObject::Character(key)),
        ) => {
            let Some(rows) = rebellion_render::status_window::character_rows(
                sources.world,
                sources.missions.en_route(),
                &rebellion_render::quadrant_icons::fleet_arrivals(sources.missions),
                key,
            ) else {
                return;
            };
            ("Character Status", None, rows)
        }
        (Scenario::SpecialForceEnroute, Some(object @ StatusObject::SpecialForce(_)))
        | (Scenario::RegimentEnroute, Some(object @ StatusObject::Troop(_)))
        | (
            Scenario::FleetArrivals,
            Some(object @ (StatusObject::Ship { .. } | StatusObject::Fleet(_))),
        ) => {
            let Some(view) = rebellion_render::status_rows::status_view(sources, object) else {
                return;
            };
            marks = view
                .marks
                .iter()
                .map(|&(source, id)| (format!("{source:?}"), id))
                .collect();
            (view.title, Some(view.name), view.rows)
        }
        (Scenario::Quadrants, Some(object @ StatusObject::ManufacturingFacility(_)))
        | (
            _,
            Some(
                object @ (StatusObject::PendingFacility { .. }
                | StatusObject::PendingFighter { .. }
                | StatusObject::TravellingFighter { .. }
                | StatusObject::PendingTroop { .. }
                | StatusObject::Producer { .. }),
            ),
        ) => {
            let Some(view) = rebellion_render::status_rows::status_view(sources, object) else {
                return;
            };
            marks = view
                .marks
                .iter()
                .map(|&(source, id)| (format!("{source:?}"), id))
                .collect();
            (view.title, Some(view.name), view.rows)
        }
        _ => return,
    };
    #[derive(Serialize)]
    struct Report {
        status: &'static str,
        title: &'static str,
        name: Option<String>,
        rows: Vec<(String, String)>,
        marks: Vec<(String, u32)>,
    }
    let bytes = serde_json::to_vec(&Report {
        status: "status-rows",
        title,
        name,
        rows: rows.into_iter().map(|row| (row.label, row.value)).collect(),
        marks,
    })
    .expect("serialize the status rows report");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// The Encyclopedia while it shows, in the PersonnelStates and ForceStatus
/// scenarios: the index's category label, first eight rows and chosen row,
/// or the topic's name and arrows.
pub fn emit_encyclopedia(
    request: FixtureRequest,
    state: &rebellion_render::EncyclopediaState,
    catalog: &rebellion_render::encyclopedia::OriginalEncyclopediaCatalog,
    topics: Option<&std::collections::HashMap<u32, rebellion_render::OriginalEncyclopediaTopic>>,
) {
    if !matches!(
        request.scenario,
        Scenario::PersonnelStates | Scenario::ForceStatus
    ) || !state.open
    {
        return;
    }
    if state.original_topic_mode {
        // The topic: its name, whether Previous and Next are enabled
        // (FUN_0045fd20), and the title and body the window draws
        // (FUN_0045fa60), for an exact text comparison with A0.
        #[derive(Serialize)]
        struct Topic {
            status: &'static str,
            topic: String,
            previous: bool,
            next: bool,
            title: Option<String>,
            body: Option<String>,
        }
        let shown = state
            .original_selected_object_id
            .and_then(|id| topics.and_then(|topics| topics.get(&id)));
        let (previous, next) = rebellion_render::encyclopedia::original_topic_neighbours(state, catalog);
        let topic = state
            .original_selected_object_id
            .and_then(|id| catalog.entries_for(0x6f).into_iter().find(|entry| entry.object_id == id))
            .map(|entry| entry.name.clone())
            .unwrap_or_default();
        let bytes = serde_json::to_vec(&Topic {
            status: "encyclopedia-topic",
            topic,
            previous: previous.is_some(),
            next: next.is_some(),
            title: shown.map(|topic| topic.title.clone()),
            body: shown.and_then(|topic| topic.body.clone()),
        })
        .expect("serialize the encyclopedia topic report");
        unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
        return;
    }
    let command = state.original_category_command;
    let entries = catalog.entries_for(command);
    #[derive(Serialize)]
    struct Report {
        status: &'static str,
        command: u16,
        label: String,
        rows: Vec<String>,
        chosen: Option<usize>,
    }
    let bytes = serde_json::to_vec(&Report {
        status: "encyclopedia",
        command,
        label: catalog.category_label(command).unwrap_or_default().to_owned(),
        rows: entries.iter().take(8).map(|entry| entry.name.clone()).collect(),
        chosen: state
            .original_selected_object_id
            .and_then(|id| entries.iter().position(|entry| entry.object_id == id)),
    })
    .expect("serialize the encyclopedia report");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// The Rescue scenario sees into the enemy prison, and `CoruscantSeized`
/// into Coruscant (`opposing_contents_visible`).
pub fn reveal(
    request: FixtureRequest,
    world: &GameWorld,
    fog_alliance: &mut FogState,
    fog_empire: &mut FogState,
) {
    if request.scenario == Scenario::CoruscantSeized {
        // A0 coruscant-676/14 shows the Alliance's regiment there to the
        // Empire (its Trooper Regiments tab full).
        if let Some(coruscant) = coruscant(world) {
            fog_alliance.reveal(coruscant);
            fog_empire.reveal(coruscant);
        }
        return;
    }
    if !request.scenario.targets_an_object() {
        return;
    }
    if let Some(secondary) = world.systems.keys().nth(1) {
        fog_alliance.reveal(secondary);
        fog_empire.reveal(secondary);
    }
}

/// The mission dialog and the active missions, each frame, for the
/// targeting scenarios.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureMissionFlow {
    status: &'static str,
    code: u32,
    dialog_open: bool,
    kinds: Vec<String>,
    target_character: Option<String>,
    /// The object target's variant, such as `DeathStar`.
    target_object: Option<String>,
    /// Each active mission's kind, target character and target object.
    missions: Vec<(String, Option<String>, Option<String>)>,
}

pub fn emit_mission_flow(
    request: FixtureRequest,
    world: &GameWorld,
    dialog: &MissionDialogState,
    missions: &MissionState,
) {
    if !request.scenario.targets_an_object()
        && !matches!(
            request.scenario,
            Scenario::MissionTargeting
                | Scenario::ReconTargeting
                | Scenario::ResearchTargeting
                | Scenario::TroopResearchTargeting
                | Scenario::ResearchDiplomacyTargeting
                | Scenario::UprisingTargeting
        )
    {
        return;
    }
    let name = |key| world.characters.get(key).map(|c| c.name.clone());
    let variant = |object: rebellion_core::missions::MissionTarget| {
        format!("{object:?}")
            .split(['(', ' '])
            .next()
            .unwrap_or_default()
            .to_owned()
    };
    let report = FixtureMissionFlow {
        status: "mission-flow",
        code: request.code,
        dialog_open: dialog.is_open(),
        kinds: dialog
            .dialog()
            .map(|d| d.kinds().iter().map(|kind| format!("{kind:?}")).collect())
            .unwrap_or_default(),
        target_character: dialog
            .dialog()
            .and_then(|d| d.target().character)
            .and_then(name),
        target_object: dialog.dialog().and_then(|d| d.target().object).map(variant),
        missions: missions
            .missions()
            .iter()
            .map(|m| {
                (
                    format!("{:?}", m.kind),
                    m.target_character.and_then(name),
                    m.target_object.map(variant),
                )
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&report).expect("serialize the mission flow report");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// The Fleet Finder scenario: the player's first fleet at the primary
/// system, a copy of it at the third system (the player's), and a copy for
/// the other side at the primary system, which the player holds and so sees
/// into (`opposing_contents_visible`).
fn place_finder_fleets(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &MovementState,
    system_keys: &[SystemKey],
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let (Some(&primary), Some(&third), Some(first)) = (
        system_keys.first(),
        system_keys.get(2),
        world.fleets.keys().next(),
    ) else {
        return;
    };
    let mut second = world.fleets[first].clone();
    second.location = third;
    second.is_alliance = player_is_alliance;
    let mut enemy = world.fleets[first].clone();
    enemy.location = primary;
    enemy.is_alliance = !player_is_alliance;
    enemy.has_death_star = false;
    world.fleets.insert(second);
    world.fleets.insert(enemy);
    reconcile_fleet_orbits(movement, world);
}

/// The fleet-move scenarios: the player's first fleet, alone in the primary
/// system and able to enter hyperspace, in that system's window at the galaxy
/// view's right edge, with the second system's sector window open as the
/// target. In the blockade variant the enemy holds the primary system, so the
/// player's fleet is alone there and blockades it
/// (`BlockadeSystem::system_is_blockaded`).
#[allow(clippy::too_many_arguments)]
fn place_moving_fleet(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    blockade: &mut BlockadeState,
    sectors: &mut SectorWindowState,
    fleets: &mut FleetWindowState,
    (primary, secondary, elsewhere): (
        rebellion_core::ids::SystemKey,
        rebellion_core::ids::SystemKey,
        rebellion_core::ids::SystemKey,
    ),
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let Some(player) = world.fleets.keys().next() else {
        return;
    };
    for (key, fleet) in &mut world.fleets {
        if key != player && fleet.location == primary {
            fleet.location = elsewhere;
        }
    }
    if let Some(class) = world.capital_ship_classes.keys().next() {
        if fleet_speed(&world.fleets[player], world).is_none() {
            world.fleets[player].capital_ships.push(ShipInstance::new(
                class,
                100,
                player_is_alliance,
            ));
        }
    }
    world.fleets[player].location = primary;
    world.fleets[player].is_alliance = player_is_alliance;
    reconcile_fleet_orbits(movement, world);
    if request.scenario == Scenario::FleetMoveBlockade {
        world.systems[primary].control = ControlKind::Controlled(if player_is_alliance {
            Faction::Empire
        } else {
            Faction::Alliance
        });
        let _ = BlockadeSystem::advance(blockade, world, &[TickEvent { tick: 1 }]);
    }
    let layout = cockpit.layout_for(640.0, 480.0);
    let galaxy = layout.galaxy;
    fleets.open(
        world,
        primary,
        (
            (galaxy.x + galaxy.width - FLEET_WINDOW_WIDTH) as i16 - 5,
            galaxy.y as i16 + 5,
        ),
        request.faction,
        layout,
    );
    sectors.open_for_system(world, secondary, request.faction);
}

/// The bombardment variant: the primary system becomes the other side's,
/// its facilities move elsewhere (no battery, ion cannon or shield answers
/// the fleet, `FUN_0058dc10`), the player's regiment there turns the other
/// side's and a second joins it, and the fleet's one ship is of the
/// player's first class that bombards and carries regiments, at full hull
/// (`FUN_0058df60`).
fn place_bombarding_fleet(
    world: &mut GameWorld,
    fleet: FleetKey,
    primary: SystemKey,
    elsewhere: SystemKey,
    player_is_alliance: bool,
) {
    let enemy = if player_is_alliance {
        Faction::Empire
    } else {
        Faction::Alliance
    };
    let system = &mut world.systems[primary];
    system.control = ControlKind::Controlled(enemy);
    system.is_populated = true;
    let defense = std::mem::take(&mut system.defense_facilities);
    let manufacturing = std::mem::take(&mut system.manufacturing_facilities);
    let production = std::mem::take(&mut system.production_facilities);
    let other = &mut world.systems[elsewhere];
    other.defense_facilities.extend(defense);
    other.manufacturing_facilities.extend(manufacturing);
    other.production_facilities.extend(production);
    let regiments = world.systems[primary].ground_units.clone();
    for &key in &regiments {
        world.troops[key].is_alliance = !player_is_alliance;
    }
    if let Some(&first) = regiments.first() {
        let copy = world.troops[first].clone();
        let second = world.troops.insert(copy);
        world.systems[primary].ground_units.push(second);
    }
    if let Some((class, hull)) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| {
            class.is_alliance == player_is_alliance
                && class.bombardment_modifier > 0
                && class.troop_capacity > 0
                && class.hyperdrive > 0
        })
        .map(|(key, class)| (key, class.hull))
    {
        world.fleets[fleet].capital_ships = vec![ShipInstance::new(
            class,
            i32::try_from(hull).unwrap_or(i32::MAX),
            player_is_alliance,
        )];
    }
}

/// The Destroy System variant: the fleet's ship is the player's Death Star
/// when its side has one (the Empire), and one of the other side's free
/// characters stands on the planet, to go adrift when it is destroyed.
fn place_death_star(
    world: &mut GameWorld,
    fleet: FleetKey,
    primary: SystemKey,
    player_is_alliance: bool,
) {
    if let Some((class, hull)) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| class.is_alliance == player_is_alliance && class.is_death_star())
        .map(|(key, class)| (key, class.hull))
    {
        let value = &mut world.fleets[fleet];
        value.capital_ships = vec![ShipInstance::new(
            class,
            i32::try_from(hull).unwrap_or(i32::MAX),
            player_is_alliance,
        )];
        value.has_death_star = true;
    }
    if let Some(key) = world
        .characters
        .iter()
        .find(|(_, c)| {
            c.is_alliance != player_is_alliance
                && !c.is_killed
                && !c.on_mission
                && !c.is_captive
                && c.current_fleet.is_none()
        })
        .map(|(key, _)| key)
    {
        for (_, value) in &mut world.fleets {
            value.characters.retain(|&k| k != key);
        }
        world.characters[key].move_to(None, Some(primary));
        world.characters[key].current_abode = None;
    }
}

/// The delivery variant: one capital ship of the player's side, one point
/// from done in the primary shipyard's queue, bound for the loading target.
fn place_delivery(
    request: FixtureRequest,
    world: &GameWorld,
    manufacturing: &mut ManufacturingState,
    primary: SystemKey,
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let Some(target) = loading_target(world, primary) else {
        return;
    };
    let Some((class, cost)) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| {
            class.is_alliance == player_is_alliance
                && class.hyperdrive > 0
                && class.refined_material_cost > 1
        })
        .map(|(key, class)| (key, class.refined_material_cost))
    else {
        return;
    };
    manufacturing.enqueue(
        primary,
        rebellion_core::manufacturing::QueueItem::new(
            rebellion_core::manufacturing::BuildableKind::CapitalShip(class),
            cost,
        )
        .delivered_to(target),
    );
    let queue = manufacturing.queue_mut(
        primary,
        rebellion_core::manufacturing::ProductionArea::Shipyard,
    );
    for _ in 1..cost {
        let _ = queue.add_point();
    }
}

/// The `BuildSelectionTraining` scenario: the side's army regiments in the
/// primary's training queue, as a confirmed order makes them
/// (`order_items`, `FUN_0052a8f0`), part made.
fn place_training(
    request: FixtureRequest,
    world: &GameWorld,
    manufacturing: &mut ManufacturingState,
    primary: SystemKey,
) {
    use rebellion_core::manufacturing::{BuildableKind, ProductionArea};
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    // TROOPSD 2 Alliance Army Regiment, 8 Imperial Army Regiment
    // (`system_window::troop_mini`); A0's units and points made.
    let (raw, count, points) = if player_is_alliance {
        (0x1000_0002, 1, 1)
    } else {
        (0x1000_0008, 3, 3)
    };
    let kind = BuildableKind::Troop(rebellion_core::ids::DatId::new(raw));
    let Some(items) = rebellion_core::build_selection::order_items(
        world,
        primary,
        ProductionArea::TrainingFacility,
        kind,
        count,
        player_is_alliance,
    ) else {
        return;
    };
    for item in items {
        manufacturing.enqueue(primary, item);
    }
    let queue = manufacturing.queue_mut(primary, ProductionArea::TrainingFacility);
    for _ in 0..points {
        let _ = queue.add_point();
    }
}

/// A0 headquarters/00-moddell's uninhabited systems (a new Alliance game):
/// name, energy slots, raw-material slots, read from the capture's tracks.
/// Adega is uninhabited there too, but A0's cursor hides its tracks' ends.
const MODDELL_UNINHABITED: [(&str, u8, u8); 4] = [
    ("Agrilat", 5, 5),
    ("Khuiumin", 4, 3),
    ("Endor", 4, 4),
    ("Annaj", 8, 4),
];

/// The `ModdellUninhabited` scenario (`MODDELL_UNINHABITED`).
fn place_moddell_uninhabited(world: &mut GameWorld) {
    for (name, energy, raw) in MODDELL_UNINHABITED {
        let Some(system) = world.systems.values_mut().find(|value| value.name == name) else {
            continue;
        };
        system.exploration_status = ExplorationStatus::Explored;
        system.is_populated = false;
        system.is_destroyed = false;
        system.control = ControlKind::Uncontrolled;
        system.total_energy = energy;
        system.raw_materials = raw;
        system.defense_facilities.clear();
        system.manufacturing_facilities.clear();
        system.production_facilities.clear();
    }
}

/// The `GidA0` scenario, as A0 2026-11-13-gid shows the Alliance's game:
/// Kubindi explored and uninhabited, held by no side (B1z: two tracks, no
/// support track; B2: its no-side mark), and Adar Tallon alone at Averam
/// on a Troop Training Research mission there (C0, C1), Averam the
/// Alliance's.
fn place_gid_a0(world: &mut GameWorld, missions: &mut MissionState) {
    let find = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, system)| system.name == name)
            .map(|(key, _)| key)
    };
    if let Some(kubindi) = find(world, "Kubindi") {
        // Its sector (Calaron) unexplored but for it and Jerijador (B0,
        // B2), held by no side: A6 shows their no-side "None" marks.
        let sector = world.systems[kubindi].sector;
        for (key, system) in &mut world.systems {
            if key != kubindi && system.sector == sector && system.name != "Jerijador" {
                system.exploration_status = ExplorationStatus::Unexplored;
                system.control = ControlKind::Uncontrolled;
            }
        }
        let system = &mut world.systems[kubindi];
        system.exploration_status = ExplorationStatus::Explored;
        system.is_populated = false;
        system.is_destroyed = false;
        system.control = ControlKind::Uncontrolled;
    }
    let Some(averam) = find(world, "Averam") else {
        return;
    };
    world.systems[averam].exploration_status = ExplorationStatus::Explored;
    world.systems[averam].control = ControlKind::Controlled(Faction::Alliance);
    let Some(tallon) = world
        .characters
        .iter()
        .find(|(_, character)| character.name == "Adar Tallon")
        .map(|(key, _)| key)
    else {
        return;
    };
    // Alone at Averam: every other character there leaves for nowhere.
    for (key, character) in &mut world.characters {
        if key != tallon && character.current_system == Some(averam) {
            character.current_system = None;
        }
    }
    let character = &mut world.characters[tallon];
    character.current_system = Some(averam);
    character.current_fleet = None;
    character.is_alliance = true;
    character.is_empire = false;
    character.recruited = true;
    character.on_mission = true;
    missions.dispatch(rebellion_core::missions::MissionRequest::single(
        MissionKind::TroopTrainingResearch,
        MissionFaction::Alliance,
        tallon,
        averam,
        None,
        0,
    ));
}

/// The `SpecForcesYavin` scenario: one Alliance special force of the
/// SpecForces view's first column (class `0x3c000004`) at Yavin and no
/// other of the Alliance's on any system or fleet (A0 2026-11-13-gid/E1:
/// one row, "Yavin", 1 in the first column). The Empire's stay.
fn place_specforces_yavin(world: &mut GameWorld) {
    let alliance: Vec<_> = world
        .special_forces
        .iter()
        .filter(|(_, unit)| unit.is_alliance)
        .map(|(key, _)| key)
        .collect();
    for system in world.systems.values_mut() {
        system.special_forces.retain(|key| !alliance.contains(key));
    }
    for fleet in world.fleets.values_mut() {
        fleet.special_forces.retain(|key| !alliance.contains(key));
    }
    let unit = alliance.iter().copied().find(|&key| {
        world.special_forces[key].class_dat_id == rebellion_core::ids::DatId::new(0x3c00_0004)
    });
    let yavin = world
        .systems
        .iter()
        .find(|(_, system)| system.name == "Yavin")
        .map(|(key, _)| key);
    if let (Some(unit), Some(yavin)) = (unit, yavin) {
        world.special_forces[unit].on_mission = false;
        world.systems[yavin].special_forces.push(unit);
    }
}

/// The `BlockadeArrival` scenario: the loading target becomes the other
/// side's, held by its support (`FUN_0050b610`), and the player's fleet
/// sets out for it.
fn place_blockade_arrival(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &mut MovementState,
    primary: SystemKey,
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let Some(target) = loading_target(world, primary) else {
        return;
    };
    let enemy = if player_is_alliance {
        Faction::Empire
    } else {
        Faction::Alliance
    };
    let system = &mut world.systems[target];
    system.control = ControlKind::Controlled(enemy);
    system.popularity_alliance = if player_is_alliance { 0.0 } else { 1.0 };
    system.popularity_empire = 1.0 - system.popularity_alliance;
    if let Some(fleet) = world.fleets.keys().next() {
        let _ = begin_fleet_transit(movement, world, fleet, target, 3);
    }
}

/// The `SupportFlip` scenario: the loading target's support goes all to the
/// other side; with no regiment there, the economy hands it over.
fn place_support_flip(request: FixtureRequest, world: &mut GameWorld, primary: SystemKey) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let Some(target) = loading_target(world, primary) else {
        return;
    };
    let system = &mut world.systems[target];
    system.popularity_alliance = if player_is_alliance { 0.0 } else { 1.0 };
    system.popularity_empire = 1.0 - system.popularity_alliance;
}

/// The `EndingDefeat` and `EndingVictory` scenarios: the win conditions of
/// `alliance_wins`'s side hold (`victory::conditions_word`, game type 0).
/// An Alliance win: Coruscant (`0x90000109`) the Alliance's, with none of
/// the Empire's regiments and all support the Alliance's, and the Emperor
/// (`0x280`) and Vader (`0x281`) captive. An Empire win: Luke (`0x242`) and
/// Mon Mothma (`0x240`) captive; the fixtures' galaxies hold no Alliance
/// headquarters.
fn place_win(alliance_wins: bool, world: &mut GameWorld) {
    let captives: [u32; 2] = if alliance_wins {
        [0x280, 0x281]
    } else {
        [0x242, 0x240]
    };
    for character in world.characters.values_mut() {
        if captives.contains(&character.dat_id.index()) {
            character.is_captive = true;
            character.captured_by = Some(if alliance_wins {
                Faction::Alliance
            } else {
                Faction::Empire
            });
        }
    }
    if alliance_wins {
        if let Some(coruscant) = world
            .systems
            .values_mut()
            .find(|system| system.dat_id.index() == 0x109)
        {
            // The economy keeps the holder its regiments and support give
            // (FUN_0050b610): none of the Empire's stay, all support is the
            // Alliance's.
            coruscant.control = ControlKind::Controlled(Faction::Alliance);
            coruscant.ground_units.clear();
            coruscant.popularity_alliance = 1.0;
            coruscant.popularity_empire = 0.0;
        }
    }
}

/// The `UprisingIncident` scenario: the primary system revolts against the
/// player, its incident due on day 5 and the disaster on day 8. Every
/// system the disaster may pick holds energy or raw materials.
fn place_incidents(
    request: FixtureRequest,
    world: &mut GameWorld,
    uprisings: &mut rebellion_core::uprising::UprisingState,
    primary: SystemKey,
) {
    let side = if request.faction == CockpitFaction::Alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    world.systems[primary].control = ControlKind::Uprising(side);
    uprisings.active_uprisings.insert(
        primary,
        rebellion_core::uprising::ActiveUprising {
            started_tick: 0,
            next_incident_tick: Some(5),
            next_support_tick: None,
            side: Some(side),
        },
    );
    uprisings.next_disaster_tick = Some(8);
    for system in world.systems.values_mut() {
        if system.total_energy == 0 && system.raw_materials == 0 {
            system.total_energy = 1;
        }
    }
}

/// The regiment-loading target: another system in the primary system's
/// sector, so the sector window that shows the fleet icon shows it too.
fn loading_target(world: &GameWorld, primary: SystemKey) -> Option<SystemKey> {
    let sector = world.systems.get(primary)?.sector;
    world
        .sectors
        .get(sector)?
        .systems
        .iter()
        .copied()
        .find(|&key| key != primary)
}

/// The system the loading scenarios' Defenses window shows: the target in
/// the refused variant (the drop it refuses), the primary system otherwise.
fn defenses_window_subject(
    request: FixtureRequest,
    world: &GameWorld,
    primary: SystemKey,
) -> Option<SystemKey> {
    if request.scenario == Scenario::RegimentUnloadRefused {
        loading_target(world, primary)
    } else {
        Some(primary)
    }
}

/// The joining scenario's second fleet: the world's second, or a copy of
/// the first when the world has one.
fn joining_fleet(
    request: FixtureRequest,
    world: &mut GameWorld,
    first: FleetKey,
) -> Option<FleetKey> {
    if !matches!(request.scenario, Scenario::FleetJoin | Scenario::FleetJoinEnroute) {
        return None;
    }
    if let Some(second) = world.fleets.keys().nth(1) {
        return Some(second);
    }
    let copy = world.fleets[first].clone();
    Some(world.fleets.insert(copy))
}

/// The fixture's regiment: the first on the primary system's surface.
fn loading_regiment(world: &GameWorld, primary: SystemKey) -> Option<TroopKey> {
    world.systems.get(primary)?.ground_units.first().copied()
}

/// The `FleetArrivals` variant: a regiment and the side's FIGHTSD 1 (5)
/// squadron of the player's at `origin`, in another sector, sent by Move
/// into `fleet`.
fn place_fleet_arrivals(
    world: &mut GameWorld,
    movement: &MovementState,
    troop_transport: &mut TroopTransportState,
    fleet: rebellion_core::ids::FleetKey,
    origin: SystemKey,
    alliance: bool,
) {
    let side = rebellion_core::dat::Faction::of_alliance(alliance);
    world.systems[origin].control = ControlKind::Controlled(side);
    world.systems[origin].exploration_status = ExplorationStatus::Explored;
    let class_dat_id =
        rebellion_core::ids::DatId::new(if alliance { 0x1000_0001 } else { 0x1000_0006 });
    let troop = world.troops.insert(TroopUnit {
        class_dat_id,
        is_alliance: alliance,
        regiment_strength: 100,
    });
    world.systems[origin].ground_units.push(troop);
    let _ = troop_transport.move_regiment(
        world,
        movement,
        &std::collections::HashSet::new(),
        troop,
        rebellion_core::troop_transport::RegimentTarget::Fleet(fleet),
        0,
    );
    let index = if alliance { 1 } else { 5 };
    if let Some(class) = world
        .fighter_classes
        .iter()
        .find(|(_, class)| class.dat_id.index() == index && class.is_alliance == alliance)
        .map(|(key, _)| key)
    {
        rebellion_core::squadrons::add(&mut world.systems[origin].fighters, class, 1);
        let _ = troop_transport.squadrons.move_squadrons(
            world,
            alliance,
            class,
            1,
            rebellion_core::squadrons::SquadronPlace::Planet(origin),
            rebellion_core::squadrons::SquadronPlace::Fleet(fleet),
            0,
        );
    }
}

/// The `FleetArrivals` variant's character: the player's first recruited
/// one off the map, on a Move whose target container is `fleet`
/// (`MissionState::set_target_fleet`), arriving at its system on day 99;
/// after it on the same Move, a new special force of the player's side
/// (SPECFCSD class 2, Infiltrators, for the Alliance; class 7, Imperial
/// Commandos, for the Empire), which the Fleet Status's Personnel counts
/// and the Capital Ship Status's does not (`FUN_00536da0`, `FUN_004f25c0`).
fn send_member_aboard(
    request: FixtureRequest,
    world: &mut GameWorld,
    missions: &mut MissionState,
    fleet: rebellion_core::ids::FleetKey,
) {
    let alliance = request.faction == CockpitFaction::Alliance;
    let Some(to) = world.fleets.get(fleet).map(|value| value.location) else {
        return;
    };
    let Some(rider) = world
        .characters
        .iter()
        .find(|(_, character)| {
            character.is_alliance == alliance && character.recruited && !character.is_killed
        })
        .map(|(key, _)| key)
    else {
        return;
    };
    world.characters[rider].current_system = None;
    for value in world.fleets.values_mut() {
        value.characters.retain(|&key| key != rider);
    }
    let faction = if alliance {
        rebellion_core::missions::MissionFaction::Alliance
    } else {
        rebellion_core::missions::MissionFaction::Empire
    };
    let id = missions.dispatch(rebellion_core::missions::MissionRequest::single(
        rebellion_core::missions::MissionKind::Diplomacy,
        faction,
        rider,
        to,
        None,
        0,
    ));
    missions.set_target_fleet(id, fleet);
    missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::Character(rider),
        mission_id: id,
        to,
        arrival: 99,
        abode: None,
    });
    let force = world
        .special_forces
        .insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: rebellion_core::ids::DatId::new(if alliance {
                0x3c00_0002
            } else {
                0x3c00_0007
            }),
            is_alliance: alliance,
            skills: [0; 8],
            on_mission: false,
        });
    missions.push_transit(rebellion_core::missions::MemberTransit {
        member: MissionMember::SpecialForce(force),
        mission_id: id,
        to,
        arrival: 99,
        abode: None,
    });
}

/// The regiment-loading scenarios: the player's first fleet, alone at the
/// player's primary system with one capital ship that carries regiments, and
/// one of the player's regiments first on the surface. The Defenses window
/// on the galaxy view's left shows its regiment page; the primary system's
/// sector window takes the right column (a window for another sector opens
/// first), so the Fleet window its icon opens lands clear of it. The
/// target system, in the same sector, is the player's and empty, so the
/// regiment lands there unopposed. In the full variant the fleet's room is
/// taken by regiments loaded beforehand (`FUN_00500b40`); in the refused
/// variant the target is the other side's and populated, the regiment
/// starts aboard and held, and the Defenses window shows the target, so a
/// drop on it is the refused move (the Fleet window covers the target's
/// planet in the sector window). In the squadron variant the carrier also
/// carries fighters, two of the player's squadrons stand on the primary
/// planet, and the Defenses window shows its squadron page.
#[allow(clippy::too_many_arguments)]
fn place_loading_fleet(
    request: FixtureRequest,
    world: &mut GameWorld,
    cockpit: &CockpitState,
    movement: &mut MovementState,
    sectors: &mut SectorWindowState,
    defenses: &mut DefensesWindowState,
    troop_transport: &mut TroopTransportState,
    primary: SystemKey,
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let player = if player_is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    let (Some(target), Some(fleet)) = (loading_target(world, primary), world.fleets.keys().next())
    else {
        return;
    };
    let second = joining_fleet(request, world, fleet);
    let Some(elsewhere) = world
        .systems
        .keys()
        .find(|&key| world.systems[key].sector != world.systems[primary].sector)
    else {
        return;
    };
    // Both stay the player's without regiments: a side holds such a system
    // with its support at GNPRTB 7760 (60; economy::resolve_system_control,
    // FUN_0050b610, FUN_0055a080).
    let support = if player == Faction::Alliance {
        0.9
    } else {
        0.1
    };
    for system in [primary, target] {
        let value = &mut world.systems[system];
        value.control = ControlKind::Controlled(player);
        value.exploration_status = ExplorationStatus::Explored;
        (value.popularity_alliance, value.popularity_empire) = (support, 1.0 - support);
    }
    for (key, value) in &mut world.fleets {
        if key != fleet
            && Some(key) != second
            && (value.location == primary || value.location == target)
        {
            value.location = elsewhere;
        }
    }
    // One carrier of the player's side that can enter hyperspace, of
    // fighters in the squadron variant.
    let squadrons = matches!(
        request.scenario,
        Scenario::SquadronLoad | Scenario::FleetArrivals
    );
    let carrier = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| {
            class.is_alliance == player_is_alliance
                && class.troop_capacity > 0
                && (!squadrons || class.fighter_capacity > 0)
                && class.hyperdrive > 0
        })
        .map(|(key, _)| key)
        .unwrap_or_else(|| {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: "Transport".into(),
                    is_alliance: player_is_alliance,
                    hull: 100,
                    hyperdrive: 80,
                    troop_capacity: 2,
                    fighter_capacity: 2,
                    ..Default::default()
                })
        });
    let value = &mut world.fleets[fleet];
    value.location = primary;
    value.is_alliance = player_is_alliance;
    value.has_death_star = false;
    value.characters.clear();
    value.fighters.clear();
    value.capital_ships = vec![ShipInstance::new(carrier, 100, player_is_alliance)];
    // The joining variant: the first fleet holds two ships and a second
    // fleet of the player's, with one, orbits beside it.
    if let Some(second) = second {
        value
            .capital_ships
            .push(ShipInstance::new(carrier, 100, player_is_alliance));
        let other = &mut world.fleets[second];
        other.location = primary;
        other.is_alliance = player_is_alliance;
        other.has_death_star = false;
        other.characters.clear();
        other.fighters.clear();
        other.capital_ships = vec![ShipInstance::new(carrier, 100, player_is_alliance)];
        // The en route variant: the second fleet's ship left `elsewhere`
        // to join the first (`fleet_join::join_fleet` across systems).
        if request.scenario == Scenario::FleetJoinEnroute {
            other.location = elsewhere;
            movement.order(second, elsewhere, primary, 30);
            movement.set_join(second, fleet);
        }
    }
    reconcile_fleet_orbits(movement, world);

    let class_dat_id = world
        .troops
        .values()
        .find(|troop| troop.is_alliance == player_is_alliance)
        .map_or_else(
            || {
                rebellion_core::ids::DatId::new(if player_is_alliance {
                    0x1000_0001
                } else {
                    0x1000_0006
                })
            },
            |troop| troop.class_dat_id,
        );
    let regiment = || TroopUnit {
        class_dat_id,
        is_alliance: player_is_alliance,
        regiment_strength: 100,
    };
    for system in [primary, target] {
        let displaced = std::mem::take(&mut world.systems[system].ground_units);
        world.systems[elsewhere].ground_units.extend(displaced);
        world.systems[system].special_forces.clear();
    }
    let troop = world.troops.insert(regiment());
    world.systems[primary].ground_units.push(troop);
    // The refused variant: the target is the other side's and populated,
    // which FUN_0053d430 refuses a regiment (1/0x28).
    if request.scenario == Scenario::RegimentUnloadRefused {
        let enemy = if player_is_alliance {
            Faction::Empire
        } else {
            Faction::Alliance
        };
        world.systems[target].control = ControlKind::Controlled(enemy);
        world.systems[target].is_populated = true;
        let _ = troop_transport.embark(world, fleet, &[troop]);
    }
    // The destination variant: a shipyard of the player's, so the facility
    // icon's menu offers Destination (0x214, FUN_00512700 kind 4).
    if matches!(
        request.scenario,
        Scenario::ProductionDestination | Scenario::ProductionDelivery
    ) {
        let shipyard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
                side: rebellion_core::dat::Faction::of_alliance(player_is_alliance),
                is_shipyard: true,
            },
        );
        world.systems[primary]
            .manufacturing_facilities
            .push(shipyard);
    }
    if request.scenario == Scenario::FleetLoadFull {
        let room = world.capital_ship_classes[carrier].troop_capacity;
        let aboard: Vec<_> = (0..room)
            .map(|_| {
                let key = world.troops.insert(regiment());
                world.systems[primary].ground_units.push(key);
                key
            })
            .collect();
        let _ = troop_transport.embark(world, fleet, &aboard);
    }

    if request.scenario == Scenario::FleetArrivals {
        place_fleet_arrivals(
            world,
            movement,
            troop_transport,
            fleet,
            elsewhere,
            player_is_alliance,
        );
    }
    // The squadron variant: two of the player's squadrons, of its side's
    // first fighter class, on the primary planet.
    if request.scenario == Scenario::SquadronLoad {
        for system in [primary, target] {
            world.systems[system].fighters.clear();
        }
        if let Some(class) = world
            .fighter_classes
            .iter()
            .find(|(_, class)| class.is_alliance == player_is_alliance)
            .map(|(key, _)| key)
        {
            rebellion_core::squadrons::add(&mut world.systems[primary].fighters, class, 2);
        }
    }
    // The battle variant: two of the other side's squadrons there instead.
    if request.scenario == Scenario::SquadronBattle {
        for system in [primary, target] {
            world.systems[system].fighters.clear();
        }
        if let Some(class) = world
            .fighter_classes
            .iter()
            .find(|(_, class)| class.is_alliance != player_is_alliance)
            .map(|(key, _)| key)
        {
            rebellion_core::squadrons::add(&mut world.systems[primary].fighters, class, 2);
        }
    }
    if matches!(
        request.scenario,
        Scenario::FleetBombardment
            | Scenario::FleetAssault
            | Scenario::FleetAssaultTaken
            | Scenario::FleetDestroySystem
    ) {
        place_bombarding_fleet(world, fleet, primary, elsewhere, player_is_alliance);
    }
    if request.scenario == Scenario::FleetDestroySystem {
        place_death_star(world, fleet, primary, player_is_alliance);
    }
    // The assault variant: one of the player's regiments aboard the fleet.
    if matches!(
        request.scenario,
        Scenario::FleetAssault | Scenario::FleetAssaultTaken
    ) {
        let key = world.troops.insert(regiment());
        world.systems[primary].ground_units.push(key);
        let _ = troop_transport.embark(world, fleet, &[key]);
    }
    // The taken variant: six of the player's regiments aboard, as A0
    // messages-day633's Alliance Forces list them (its bar's 57-pixel thumb
    // is six 70-pixel rows), on copies of the fleet's ship enough to carry
    // them; the other side's regiments leave the planet.
    if request.scenario == Scenario::FleetAssaultTaken {
        let ship = world.fleets[fleet].capital_ships.first().cloned();
        if let Some(ship) = ship {
            let capacity = world
                .capital_ship_classes
                .get(ship.class)
                .map_or(0, |class| class.troop_capacity);
            while capacity > 0 && world.fleets[fleet].capital_ships.len() * (capacity as usize) < 6
            {
                world.fleets[fleet].capital_ships.push(ship.clone());
            }
        }
        // The other side's general rides along as the fleet's captive, as
        // A0 messages-day633/assault-coruscant-alliance-tab4 lists Veers
        // under the Alliance's Captured (FUN_00496770, +0x90).
        let captive_name = if player_is_alliance { "Veers" } else { "Jan Dodonna" };
        if let Some(captive) = world
            .characters
            .iter()
            .find(|(_, character)| character.name == captive_name)
            .map(|(key, _)| key)
        {
            let character = &mut world.characters[captive];
            character.is_captive = true;
            // Hurt, as A0's Veers shows the damage mark.
            character.injury = character.injury.max(1);
            character.captured_by = Some(if player_is_alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            });
            character.current_system = None;
            character.current_fleet = Some(fleet);
            character.on_mission = false;
            for other in world.fleets.values_mut() {
                other.characters.retain(|&key| key != captive);
            }
            world.fleets[fleet].characters.push(captive);
        }
        let more: Vec<_> = (0..5).map(|_| world.troops.insert(regiment())).collect();
        world.systems[primary].ground_units.extend(&more);
        let _ = troop_transport.embark(world, fleet, &more);
        let player_side = player_is_alliance;
        let troops = &world.troops;
        world.systems[primary].ground_units.retain(|key| {
            troops
                .get(*key)
                .is_none_or(|troop| troop.is_alliance == player_side)
        });
    }

    let layout = cockpit.layout_for(640.0, 480.0);
    let galaxy = layout.galaxy;
    let subject = defenses_window_subject(request, world, primary).unwrap_or(primary);
    defenses.open(
        world,
        subject,
        (galaxy.x as i16 + 5, galaxy.y as i16 + 5),
        request.faction,
        layout,
    );
    defenses.show_page(
        subject,
        if squadrons {
            DefensesPage::Squadrons
        } else {
            DefensesPage::Regiments
        },
    );
    sectors.open_for_system(world, elsewhere, request.faction);
    sectors.open_for_system(world, primary, request.faction);
}

/// The quadrant-icon scenario. The primary system (the player's) and a
/// second system in its sector (the other side's) are emptied, then stocked
/// so each icon the gate checks has one cause:
///
/// - primary: a player mine (kind 4), a player regiment and KDY-150
///   (kind 8, and rows on the Defenses window's regiment and battery
///   pages), the player's first fleet when it has one (kind `0x10`), and
///   one agent of each side on a Diplomacy mission there (kind `0x40`, the
///   other side's art, `FUN_004a1f60`);
/// - second: a player regiment (kind 8, the system's side art) and a player
///   agent on a mission there (kind `0x40`, the player's art), still in
///   hyperspace to it.
///
/// Only the primary system's sector window is open.
fn place_quadrant_contents(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &mut MovementState,
    missions: &mut MissionState,
    uprisings: &mut rebellion_core::uprising::UprisingState,
    sectors: &mut SectorWindowState,
    primary: SystemKey,
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let (player, enemy) = if player_is_alliance {
        (Faction::Alliance, Faction::Empire)
    } else {
        (Faction::Empire, Faction::Alliance)
    };
    let Some(second) = loading_target(world, primary) else {
        return;
    };
    let Some(elsewhere) = world
        .systems
        .keys()
        .find(|&key| world.systems[key].sector != world.systems[primary].sector)
    else {
        return;
    };
    let fleet = world
        .fleets
        .iter()
        .find(|(_, value)| value.is_alliance == player_is_alliance)
        .map(|(key, _)| key);
    world.systems[primary].control = ControlKind::Controlled(player);
    world.systems[second].control = ControlKind::Controlled(enemy);
    // The sector's next two systems: the Alliance's headquarters, and a
    // system of the player's in uprising (the sector window's marks).
    if let [headquarters, revolt, ..] = mark_systems(world, primary)[..] {
        for system in [headquarters, revolt] {
            world.systems[system].exploration_status = ExplorationStatus::Explored;
            world.systems[system].is_destroyed = false;
        }
        world.systems[headquarters].is_headquarters = true;
        world.systems[headquarters].control = ControlKind::Controlled(Faction::Alliance);
        move_headquarters_object(world, headquarters);
        world.systems[revolt].is_headquarters = false;
        world.systems[revolt].control = ControlKind::Controlled(player);
        uprisings.active_uprisings.insert(
            revolt,
            rebellion_core::uprising::ActiveUprising {
                started_tick: 0,
                next_incident_tick: None,
                next_support_tick: None,
                side: Some(player),
            },
        );
    }
    // The sector's third such system: explored, held by neither side, with
    // a mine (FUN_0045cdc0 passes side 0 through; FUN_0045ca80 gives the
    // neutral art 10787, which A0 sector-b shows by every neutral planet).
    if let Some(&neutral) = mark_systems(world, primary).get(2) {
        let value = &mut world.systems[neutral];
        value.exploration_status = ExplorationStatus::Explored;
        value.is_destroyed = false;
        value.is_headquarters = false;
        value.control = ControlKind::Uncontrolled;
        value.defense_facilities.clear();
        value.manufacturing_facilities.clear();
        // The player's own mine, which the player always sees
        // (quadrant_icons: a facility of another side needs intel); the
        // icon takes the system's side either way.
        let mine =
            world
                .production_facilities
                .insert(rebellion_core::world::ProductionFacilityInstance {
                    class_dat_id: rebellion_core::ids::DatId::new(0x2c00_0001),
                    side: player,
                    is_mine: true,
                });
        world.systems[neutral].production_facilities = vec![mine];
        // And the player's battery, so its defenses icon shows with the
        // system's side too: the neutral pair 10789/10790 (FUN_0045ce80,
        // FUN_0045ca80 state 0; A0 modal/07-after shows 10790).
        let battery = world
            .defense_facilities
            .insert(rebellion_core::world::DefenseFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2200_0001),
                side: player,
            });
        world.systems[neutral].defense_facilities = vec![battery];
        uprisings.active_uprisings.remove(&neutral);
    }
    // And a fourth, destroyed (FUN_0045bbb0: the asteroid field 10240 in
    // place of its planet, its name white, no tracks; A0 menus3/05 shows
    // Averam so after the Death Star).
    if let Some(&ruin) = mark_systems(world, primary).get(3) {
        let value = &mut world.systems[ruin];
        value.exploration_status = ExplorationStatus::Explored;
        value.is_destroyed = true;
        value.is_headquarters = false;
        uprisings.active_uprisings.remove(&ruin);
    }
    for system in [primary, second] {
        let value = &mut world.systems[system];
        value.exploration_status = ExplorationStatus::Explored;
        value.defense_facilities.clear();
        value.manufacturing_facilities.clear();
        value.production_facilities.clear();
        let troops = std::mem::take(&mut value.ground_units);
        let forces = std::mem::take(&mut value.special_forces);
        world.systems[elsewhere].ground_units.extend(troops);
        world.systems[elsewhere].special_forces.extend(forces);
    }
    for (key, value) in &mut world.fleets {
        if Some(key) != fleet && (value.location == primary || value.location == second) {
            value.location = elsewhere;
        }
    }
    if let Some(fleet) = fleet {
        world.fleets[fleet].location = primary;
    }
    reconcile_fleet_orbits(movement, world);
    for character in world.characters.values_mut() {
        if character.current_system == Some(primary) || character.current_system == Some(second) {
            character.current_system = Some(elsewhere);
        }
    }

    let mine =
        world
            .production_facilities
            .insert(rebellion_core::world::ProductionFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2c00_0001),
                side: rebellion_core::dat::Faction::of_alliance(player_is_alliance),
                is_mine: true,
            });
    world.systems[primary].production_facilities.push(mine);
    let battery = world
        .defense_facilities
        .insert(rebellion_core::world::DefenseFacilityInstance {
            class_dat_id: rebellion_core::ids::DatId::new(0x2200_0001),
            side: rebellion_core::dat::Faction::of_alliance(player_is_alliance),
        });
    world.systems[primary].defense_facilities.push(battery);
    let class_dat_id = rebellion_core::ids::DatId::new(if player_is_alliance {
        0x1000_0001
    } else {
        0x1000_0006
    });
    for system in [primary, second] {
        let troop = world.troops.insert(TroopUnit {
            class_dat_id,
            is_alliance: player_is_alliance,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(troop);
    }

    let aboard: std::collections::HashSet<_> = world
        .fleets
        .values()
        .flat_map(|value| value.characters.iter().copied())
        .collect();
    let agents = |alliance: bool| {
        world
            .characters
            .iter()
            .filter(|(key, character)| {
                character.is_alliance == alliance
                    && character.is_empire != alliance
                    && !character.is_killed
                    && !character.is_captive
                    && !aboard.contains(key)
            })
            .map(|(key, _)| key)
            .collect::<Vec<_>>()
            .into_iter()
    };
    let mut own: Vec<_> = agents(player_is_alliance).collect();
    // The Empire's traveller is Jerjerrod, as A0 mission-reports/missions-
    // window-enroute-day276 shows him in hyperspace to Chandrila.
    if let Some(at) = own
        .iter()
        .position(|&key| world.characters[key].name == "Jerjerrod")
        .filter(|_| !player_is_alliance && own.len() > 1)
    {
        let jerjerrod = own.remove(at);
        own.insert(1, jerjerrod);
    }
    let mut own = own.into_iter();
    let mut theirs = agents(!player_is_alliance);
    let placements = [
        (own.next(), primary, player_is_alliance),
        (theirs.next(), primary, !player_is_alliance),
        (own.next(), second, player_is_alliance),
    ];
    for (agent, system, alliance) in placements {
        let Some(agent) = agent else {
            continue;
        };
        world.characters[agent].current_system = Some(system);
        let id = missions.dispatch(rebellion_core::missions::MissionRequest::single(
            MissionKind::Diplomacy,
            if alliance {
                MissionFaction::Alliance
            } else {
                MissionFaction::Empire
            },
            agent,
            system,
            None,
            0,
        ));
        rebellion_core::missions::set_on_mission(world, MissionMember::Character(agent), true);
        if system == second {
            // The second system's agent is still in hyperspace to it
            // (FUN_00556430: listed at its target with the en route mark).
            world.characters[agent].current_system = None;
            missions.push_transit(rebellion_core::missions::MemberTransit {
                member: MissionMember::Character(agent),
                mission_id: id,
                to: second,
                arrival: 99,
                abode: None,
            });
        }
    }
    sectors.open_for_system(world, primary, request.faction);
}

#[derive(Serialize)]
struct FixtureReady<'a> {
    schema_version: u32,
    status: &'static str,
    code: u32,
    scenario: u8,
    faction: &'static str,
    mode: &'a str,
    state_fingerprint: String,
    stable_frames: u32,
    primary_system_x: u16,
    primary_system_y: u16,
    probe_system_dat_id: u32,
    probe_system_name: &'a str,
    probe_screen_x: f32,
    probe_screen_y: f32,
    /// The primary system's point and the GID bitmaps drawn there, in
    /// order (the headquarters burst under the mark, `FUN_0042cde0`).
    primary_screen_x: f32,
    primary_screen_y: f32,
    primary_gid_marks: Vec<u32>,
}

pub fn emit_ready(request: FixtureRequest, world: &GameWorld, map: &GalaxyMapState) {
    let faction = match request.faction {
        CockpitFaction::Alliance => "alliance",
        CockpitFaction::Empire => "empire",
    };
    let fingerprint_input = serde_json::to_vec(&(request.code, world, map.selected_system))
    .expect("serialize deterministic interface fixture state");
    let aperture = CockpitState::new(request.faction)
        .layout_for(640.0, 480.0)
        .galaxy;
    let probe = world
        .systems
        .iter()
        .map(|(_, system)| {
            let (x, y) = screen_point(system, request.faction);
            (system, x, y)
        })
        .filter(|(_, x, y)| {
            *x > aperture.x + 20.0
                && *x < aperture.x + aperture.width - 20.0
                && *y > aperture.y + 20.0
                && *y < aperture.y + aperture.height - 20.0
        })
        .min_by(|a, b| {
            let center_x = aperture.x + aperture.width / 2.0;
            let center_y = aperture.y + aperture.height / 2.0;
            let distance = |x: f32, y: f32| (x - center_x).powi(2) + (y - center_y).powi(2);
            distance(a.1, a.2).total_cmp(&distance(b.1, b.2))
        })
        .expect("fixture has a visible system for hover probe");
    let primary = world
        .systems
        .values()
        .next()
        .map_or((0.0, 0.0), |system| screen_point(system, request.faction));
    let report = FixtureReady {
        schema_version: 1,
        status: "ready",
        code: request.code,
        scenario: request.scenario as u8,
        faction,
        mode: request.scenario.mode().label(),
        state_fingerprint: format!("fnv1a64:{:016x}", fnv1a64(&fingerprint_input)),
        stable_frames: 2,
        primary_system_x: world.systems.iter().next().map_or(0, |(_, value)| value.x),
        primary_system_y: world.systems.iter().next().map_or(0, |(_, value)| value.y),
        probe_system_dat_id: probe.0.dat_id.raw(),
        probe_system_name: &probe.0.name,
        probe_screen_x: probe.1,
        probe_screen_y: probe.2,
        primary_screen_x: primary.0,
        primary_screen_y: primary.1,
        primary_gid_marks: map
            .drawn_gid_markers
            .iter()
            .filter(|(_, [x, y])| (x - primary.0).abs() < 0.5 && (y - primary.1).abs() < 0.5)
            .map(|(id, _)| *id)
            .collect(),
    };
    let bytes = serde_json::to_vec(&report).expect("serialize interface fixture report");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// Where `system` is drawn in the 640 by 480 fixture canvas.
fn screen_point(system: &rebellion_core::world::System, faction: CockpitFaction) -> (f32, f32) {
    let aperture = CockpitState::new(faction).layout_for(640.0, 480.0).galaxy;
    rebellion_render::galaxy_camera(
        (aperture.x, aperture.y, aperture.width, aperture.height),
        1.0,
        faction,
    )
    .to_screen(f32::from(system.x), f32::from(system.y))
}

/// The open object pop-up menu and the targeting scenario's target planet,
/// so the browser gate can choose Mission and release over it.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureObjectMenu<'a> {
    status: &'static str,
    code: u32,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    rows: usize,
    /// Each row's label and whether it is enabled, top to bottom.
    row_states: Vec<(&'static str, bool)>,
    reserved_row: Option<usize>,
    /// Whether the Reserved row carries its check mark.
    reserved_checked: bool,
    mission_row: Option<usize>,
    move_row: Option<usize>,
    confirmed_move_row: Option<usize>,
    confirmed_move_enabled: bool,
    create_fleet_row: Option<usize>,
    rename_row: Option<usize>,
    rename_enabled: bool,
    destination_row: Option<usize>,
    status_window_row: Option<usize>,
    build_row: Option<usize>,
    stop_row: Option<usize>,
    bombardment_row: Option<usize>,
    assault_row: Option<usize>,
    command_row: Option<usize>,
    /// Retire (`0x242`) and a mission icon's Abort (`0x250`), each with
    /// whether its row is enabled.
    retire_row: Option<usize>,
    retire_enabled: bool,
    abort_row: Option<usize>,
    abort_enabled: bool,
    /// Scrap (`0x200`) and whether its row is enabled.
    scrap_row: Option<usize>,
    scrap_enabled: bool,
    /// The Planetary Bombardment or Command submenu while it is shown.
    submenu: Option<FixtureSubmenu>,
    target_dat_id: u32,
    target_name: &'a str,
    target_screen_x: f32,
    target_screen_y: f32,
}

/// The shown submenu's window and General Bombardment's row in it.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureSubmenu {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    rows: usize,
    general_row: Option<usize>,
    /// Target Military Facilities' (`0x220`) and Target Civilian
    /// Facilities' (`0x221`) rows.
    military_row: Option<usize>,
    civilian_row: Option<usize>,
    /// Destroy System's row (`0x223`) and whether it is enabled.
    destroy_row: Option<usize>,
    destroy_enabled: bool,
    /// The Command submenu's rows `0x260..0x263` (None, Commander,
    /// Admiral, General).
    command_rows: [Option<usize>; 4],
}

fn object_menu_report<'a>(
    request: FixtureRequest,
    (rect, submenu): (
        egui_macroquad::egui::Rect,
        Option<egui_macroquad::egui::Rect>,
    ),
    menu: &ObjectMenuState,
    world: &'a GameWorld,
    sectors: &SectorWindowState,
    windows: (&DefensesWindowState, &SystemWindowState, &FleetWindowState),
) -> Option<FixtureObjectMenu<'a>> {
    let (defenses, systems, fleets) = windows;
    let key = if request.scenario.uses_the_fleet_window() {
        loading_target(world, world.systems.keys().next()?)?
    } else {
        world.systems.keys().nth(1)?
    };
    let target = world.systems.get(key)?;
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    // The Rescue scenario targets the prisoner in the prison's first cell.
    let prison_cell = match request.scenario {
        Scenario::RescueTargeting | Scenario::EnemyCharacterTargeting => {
            defenses.cell_screen_rect(layout, key, 0)
        }
        Scenario::SabotageTargeting => systems.cell_screen_rect(layout, key, 0),
        Scenario::DeathStarTargeting | Scenario::ShipTargeting => {
            fleets.item_screen_rect(layout, key, 0)
        }
        Scenario::FleetTargeting | Scenario::CharacterBoard => {
            fleets.entry_screen_rect(layout, key, 0)
        }
        // The agent's Personnel item, the drag's source.
        Scenario::CharacterAboard => fleets.item_screen_rect(layout, key, 0),
        _ => None,
    };
    let planet = match prison_cell.or_else(|| sectors.planet_screen_rect(world, layout, key)) {
        Some(rect) => rect.center(),
        // The Build Selection, mission-icon and A0 Yavin gates target
        // nothing; their menus report anyway.
        None if matches!(
            request.scenario,
            Scenario::BuildSelection
                | Scenario::BuildSelectionTraining
                | Scenario::BuildSelectionOverMaintenance
                | Scenario::Quadrants
                | Scenario::FleetWindowYavin
                | Scenario::FleetWindowA0
                | Scenario::BatteryStatus
                | Scenario::PersonnelStates
                | Scenario::BuildSelectionDefensesDestination
                | Scenario::BuildSelectionFleetDestination
        ) =>
        {
            egui_macroquad::egui::Pos2::ZERO
        }
        None => return None,
    };
    let (target_screen_x, target_screen_y) = (planet.x, planet.y);
    Some(FixtureObjectMenu {
        status: "object-menu",
        code: request.code,
        left: rect.min.x,
        top: rect.min.y,
        width: rect.width(),
        height: rect.height(),
        rows: menu.row_count(),
        row_states: menu.row_states().collect(),
        reserved_row: menu.row_of(ObjectMenuCommand::Reserved),
        reserved_checked: menu.row_checked(ObjectMenuCommand::Reserved),
        mission_row: menu.row_of(ObjectMenuCommand::Mission),
        move_row: menu.row_of(ObjectMenuCommand::Move),
        confirmed_move_row: menu.row_of(ObjectMenuCommand::ConfirmedMove),
        confirmed_move_enabled: menu
            .row_of(ObjectMenuCommand::ConfirmedMove)
            .is_some_and(|row| menu.row_enabled(row)),
        create_fleet_row: menu.row_of(ObjectMenuCommand::CreateFleet),
        rename_row: menu.row_of(ObjectMenuCommand::Rename),
        rename_enabled: menu
            .row_of(ObjectMenuCommand::Rename)
            .is_some_and(|row| menu.row_enabled(row)),
        destination_row: menu.row_of(ObjectMenuCommand::Destination),
        status_window_row: menu.row_of(ObjectMenuCommand::Status),
        build_row: menu.row_of(ObjectMenuCommand::Build),
        stop_row: menu.row_of(ObjectMenuCommand::Stop),
        bombardment_row: menu.row_of(ObjectMenuCommand::Bombardment),
        command_row: menu.row_of(ObjectMenuCommand::Command),
        retire_row: menu.row_of(ObjectMenuCommand::Retire),
        retire_enabled: menu
            .row_of(ObjectMenuCommand::Retire)
            .is_some_and(|row| menu.row_enabled(row)),
        abort_row: menu.row_of(ObjectMenuCommand::Abort),
        abort_enabled: menu
            .row_of(ObjectMenuCommand::Abort)
            .is_some_and(|row| menu.row_enabled(row)),
        scrap_row: menu.row_of(ObjectMenuCommand::Scrap),
        scrap_enabled: menu
            .row_of(ObjectMenuCommand::Scrap)
            .is_some_and(|row| menu.row_enabled(row)),
        assault_row: menu
            .row_of(ObjectMenuCommand::Assault)
            .filter(|&row| menu.row_enabled(row)),
        submenu: submenu
            .filter(|_| menu.submenu_shown())
            .map(|sub| FixtureSubmenu {
                left: sub.min.x,
                top: sub.min.y,
                width: sub.width(),
                height: sub.height(),
                rows: menu.submenu_row_count(),
                general_row: menu.submenu_row_of(ObjectMenuCommand::BombardmentTarget(
                    rebellion_core::bombardment::GENERAL,
                )),
                military_row: menu.submenu_row_of(ObjectMenuCommand::BombardmentTarget(
                    rebellion_core::bombardment::TARGET_MILITARY,
                )),
                civilian_row: menu.submenu_row_of(ObjectMenuCommand::BombardmentTarget(
                    rebellion_core::bombardment::TARGET_CIVILIAN,
                )),
                destroy_row: menu.submenu_row_of(ObjectMenuCommand::BombardmentTarget(
                    rebellion_core::bombardment::DESTROY_SYSTEM,
                )),
                destroy_enabled: menu
                    .submenu_row_of(ObjectMenuCommand::BombardmentTarget(
                        rebellion_core::bombardment::DESTROY_SYSTEM,
                    ))
                    .is_some_and(|row| menu.submenu_row_enabled(row)),
                command_rows: [0x260, 0x261, 0x262, 0x263]
                    .map(|order| menu.submenu_row_of(ObjectMenuCommand::GiveCommand(order))),
            }),
        target_dat_id: target.dat_id.raw(),
        target_name: &target.name,
        target_screen_x,
        target_screen_y,
    })
}

pub fn emit_object_menu(
    request: FixtureRequest,
    rect: (
        egui_macroquad::egui::Rect,
        Option<egui_macroquad::egui::Rect>,
    ),
    menu: &ObjectMenuState,
    world: &GameWorld,
    sectors: &SectorWindowState,
    windows: (&DefensesWindowState, &SystemWindowState, &FleetWindowState),
) {
    if !request.scenario.reports_object_menu() {
        return;
    }
    let Some(report) = object_menu_report(request, rect, menu, world, sectors, windows) else {
        return;
    };
    let bytes = serde_json::to_vec(&report).expect("serialize the object menu report");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// One Manufacturing window band as the Build Selection gate sees it.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureBand {
    area: &'static str,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    queued: usize,
    product: String,
    /// The name of the area's Destination system (0x214).
    destination: String,
    /// The band's drawn lines (`manufacturing_window::band`): the product
    /// or idle text, "Building: N" or empty, and "Destination: ...".
    lines: [String; 3],
}

/// The primary system's bands and the Build Selection window, each frame.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureProduction {
    status: &'static str,
    code: u32,
    bands: Vec<FixtureBand>,
    build_selection_open: bool,
    build_selection_left: f32,
    build_selection_top: f32,
    /// The open window's number to build and its four values as drawn
    /// (refined material, maintenance, completion, deployment).
    build_selection_quantity: Option<u32>,
    build_selection_values: Option<[String; 4]>,
    build_selection_index: Option<usize>,
    /// The shown class's name (`FUN_00437880`).
    build_selection_class: Option<String>,
    /// The Training Facilities tab and its page's first cell, as
    /// `[left, top, width, height]`.
    training_tab: Option<[f32; 4]>,
    first_cell: Option<[f32; 4]>,
}

fn production_report(
    request: FixtureRequest,
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    en_route: &[rebellion_core::delivery::Delivery],
    systems: &SystemWindowState,
    build_selection: &rebellion_render::build_selection::BuildSelectionState,
) -> Option<FixtureProduction> {
    let primary = world.systems.keys().next()?;
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let bands = rebellion_core::manufacturing::ProductionArea::ALL
        .iter()
        .filter_map(|&area| {
            let rect = systems.band_screen_rect(layout, primary, area)?;
            let queue = manufacturing.queue(primary, area);
            Some(FixtureBand {
                area: match area {
                    rebellion_core::manufacturing::ProductionArea::Shipyard => "ships",
                    rebellion_core::manufacturing::ProductionArea::TrainingFacility => "troops",
                    rebellion_core::manufacturing::ProductionArea::ConstructionYard => "facilities",
                },
                left: rect.min.x,
                top: rect.min.y,
                width: rect.width(),
                height: rect.height(),
                queued: queue.map_or(0, rebellion_core::manufacturing::ProductionQueue::len),
                product: queue
                    .and_then(rebellion_core::manufacturing::ProductionQueue::active)
                    .map(|item| rebellion_render::manufacturing_window::product(world, item.kind).0)
                    .unwrap_or_default(),
                destination: world
                    .systems
                    .get(manufacturing.destination(primary, area).unwrap_or(primary))
                    .map_or_else(String::new, |value| value.name.clone()),
                lines: {
                    let band = rebellion_render::manufacturing_window::band(
                        world,
                        manufacturing,
                        true,
                        primary,
                        area,
                    );
                    [
                        band.status,
                        band.units.unwrap_or_default(),
                        band.destination,
                    ]
                },
            })
        })
        .collect();
    let window = rebellion_render::build_selection::window_rect(layout);
    let ltwh = |rect: egui_macroquad::egui::Rect| [rect.min.x, rect.min.y, rect.width(), rect.height()];
    Some(FixtureProduction {
        status: "production",
        code: request.code,
        bands,
        build_selection_open: build_selection.is_open(),
        build_selection_quantity: build_selection.dialog().map(|dialog| dialog.quantity()),
        build_selection_index: build_selection.dialog().map(|dialog| dialog.selected_index()),
        build_selection_class: build_selection
            .dialog()
            .map(|dialog| rebellion_render::manufacturing_window::product(world, dialog.kind()).0),
        build_selection_values: build_selection.dialog().map(|dialog| {
            use rebellion_render::build_selection::{cost_text, days_text};
            // The gate opens it on the primary system's band, for the player.
            let is_alliance = request.faction == CockpitFaction::Alliance;
            let side = rebellion_core::dat::Faction::of_alliance(is_alliance);
            let unallocated = rebellion_core::side_resources::maintenance_totals(
                world,
                manufacturing,
                side,
                |_| false,
            )
            .unallocated();
            let estimate = rebellion_core::build_selection::estimate(
                world,
                manufacturing,
                en_route,
                primary,
                dialog.area(),
                dialog.kind(),
                dialog.quantity(),
                is_alliance,
                unallocated,
            );
            let valid = estimate.completion.is_some();
            [
                cost_text(estimate.refined_material, valid),
                cost_text(estimate.maintenance, valid),
                days_text(estimate.completion.unwrap_or(0), valid),
                days_text(estimate.deployment, valid),
            ]
        }),
        build_selection_left: window.min.x,
        build_selection_top: window.min.y,
        training_tab: systems
            .tab_screen_rect(
                layout,
                primary,
                rebellion_render::system_window::SystemWindowTab::TrainingFacilities,
            )
            .map(ltwh),
        first_cell: systems.cell_screen_rect(layout, primary, 0).map(ltwh),
    })
}

pub fn emit_production(
    request: FixtureRequest,
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    en_route: &[rebellion_core::delivery::Delivery],
    systems: &SystemWindowState,
    build_selection: &rebellion_render::build_selection::BuildSelectionState,
) {
    // ProductionDelivery's shipyard band shows its product one point short.
    // AdvancedBases reports the Training Facilities tab and first cell.
    if !request.scenario.opens_build_selection()
        && !matches!(
            request.scenario,
            Scenario::ProductionDelivery | Scenario::AdvancedBases
        )
    {
        return;
    }
    let Some(report) = production_report(request, world, manufacturing, en_route, systems, build_selection)
    else {
        return;
    };
    let bytes = serde_json::to_vec(&report).expect("serialize the production report");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// The story-message scenario's messages, posted as the game posts them:
/// timer `0x389`'s two discoveries
/// (`story_messages::force_user_discovered`). They are filed unread under
/// the Mission rail.
pub fn post_story_messages(
    request: FixtureRequest,
    world: &GameWorld,
    log: &mut rebellion_render::MessageLog,
    texts: &std::collections::HashMap<u16, String>,
) {
    if request.scenario == Scenario::StoryMovie {
        if let Some(movie) =
            rebellion_render::story_messages::death_star_sabotaged_movie("Endor", texts)
        {
            log.queue_story_movie(movie);
        }
        // The victory conditions word's Coruscant bit (0x100, FUN_0048f790):
        // MDATA.103 under "The Alliance has seized the Imperial capital
        // system of Coruscant.", as A0 empire-clean/61 shows it.
        for movie in rebellion_render::story_messages::victory_conditions_movies(0x100, texts) {
            log.queue_story_movie(movie);
        }
        return;
    }
    if request.scenario != Scenario::StoryMessages {
        return;
    }
    let named = |needle: &str| {
        world
            .characters
            .iter()
            .find(|(_, character)| character.name.to_lowercase().contains(needle))
            .map(|(key, _)| key)
    };
    let (Some(leia), Some(chewbacca)) = (named("leia"), named("chewbacca")) else {
        return;
    };
    // Notification 0x362 (FUN_0048f1e0) as timer 0x389 posts it: Vader
    // finds an Empire minor character (the Empire reader sees Future Jedi
    // Discovered), Luke finds Leia (the Alliance reader sees Leia Uses
    // Force).
    let empire_minor = world
        .characters
        .iter()
        .find(|(_, character)| character.is_empire && !character.is_major)
        .map(|(key, _)| key);
    // Character Discovers Traitor (FUN_0048c9c0): Leia learns Chewbacca's
    // betrayal.
    if let Some(message) = rebellion_render::story_messages::discovers_traitor_message(
        &world.characters[leia],
        &world.characters[chewbacca],
        0,
        texts,
    ) {
        log.push(message);
    }
    // Rescue Attempt (FUN_0048c4c0): Chewbacca sets out for the palace.
    if let Some(message) = rebellion_render::story_messages::rescue_attempt_message(
        &world.characters[chewbacca],
        0,
        texts,
    ) {
        log.push(message);
    }
    // Emperor Arrives at Coruscant (FUN_0048ce80), the Empire's.
    if let Some(message) = rebellion_render::story_messages::emperor_arrives_message(0, texts) {
        log.push(message);
    }
    // Killed by Imperial Assassins (FUN_0048b050).
    if let Some(message) = rebellion_render::story_messages::assassins_message(
        &world.characters[chewbacca],
        "Yavin",
        0,
        texts,
    ) {
        log.push(message);
    }
    // Jabba Delivers Prisoners (FUN_0048b6e0), then Jabba Captures
    // (FUN_00490340 case 5): Chewbacca, a failed rescuer.
    if let Some(message) = rebellion_render::story_messages::jabba_delivers_message(0, texts) {
        log.push(message);
    }
    if let Some(message) = rebellion_render::story_messages::jabba_captures_message(
        &world.characters[chewbacca],
        0,
        texts,
    ) {
        log.push(message);
    }
    let discoveries = [
        (named("vader"), empire_minor),
        (named("luke"), Some(leia)),
    ];
    for (trainer, found) in discoveries {
        if let (Some(trainer), Some(found)) = (trainer, found) {
            if let Some(message) = rebellion_render::story_messages::force_user_discovered(
                world, trainer, found, 0, texts,
            ) {
                log.push(message);
            }
        }
    }
    // The encounter class (vtable 0x65b408): an Alliance minor character
    // detected by Vader (Force user Detects Enemy, one per side), Luke
    // confronting Vader, and Luke learning his heritage from him.
    {
        use rebellion_core::missions::MissionEffect;
        let alliance_minor = world
            .characters
            .iter()
            .find(|(_, character)| character.is_alliance && !character.is_major)
            .map(|(key, _)| key);
        let pair = |member, defender| {
            [
                MissionEffect::EncounterBegan {
                    member,
                    defender,
                    day: 0,
                },
                MissionEffect::EncounterEnded {
                    member,
                    partner: defender,
                },
            ]
        };
        if let (Some(luke), Some(vader)) = (named("luke"), named("vader")) {
            let mut days: Vec<Vec<MissionEffect>> = Vec::new();
            if let Some(minor) = alliance_minor {
                days.push(pair(minor, vader).to_vec());
            }
            days.push(pair(luke, vader).to_vec());
            let mut heritage = pair(luke, vader).to_vec();
            heritage.push(MissionEffect::HeritageLearned { character: luke });
            days.push(heritage);
            for effects in days {
                for message in
                    rebellion_render::story_messages::EncounterNotes::gather(world, &effects)
                        .messages(
                            world,
                            &rebellion_core::missions::MissionState::new(),
                            0,
                            texts,
                        )
                {
                    log.push(message);
                }
            }
        }
    }
    // Notification 0x212 (FUN_0048fa00) with Han free: the Alliance reads
    // that he was attacked, the Empire where the hunters found him.
    if let Some(han) = named("han solo") {
        for message in rebellion_render::story_messages::bounty_messages(world, han, 0, texts) {
            log.push(message);
        }
    }
    // The Final Battle (FUN_0048e3c0), with Luke free: he overcame the
    // Emperor.
    if let Some(luke) = named("luke") {
        if let Some(message) =
            rebellion_render::story_messages::final_battle_message(world, luke, 0, texts)
        {
            log.push(message);
        }
    }
    // Luke's Force Growth (0x1e1, FUN_0048ed80) after Dagobah, under the
    // Jedi Knight's portrait (A0 alliance-game2/luke-force-growth, day 643).
    // Filed before Luke Goes to Dagobah, so it lists under it.
    if request.faction == CockpitFaction::Alliance {
        if let Some(luke) = named("luke") {
            let mut trained = world.clone();
            trained.characters[luke].dagobah_completed = true;
            log.push(rebellion_render::story_messages::story_message(
                &trained,
                rebellion_core::events::EVT_CHARACTER_FORCE,
                &[],
                String::new(),
                0,
                request.faction,
                texts,
                rebellion_render::MessageCategory::Event,
            ));
        }
    }
    // Luke's Dagobah messages (0x221, 0x210), for the Alliance reader.
    if let Some(message) = rebellion_render::story_messages::luke_goes_to_dagobah(0, texts) {
        log.push(message);
    }
    if let Some(luke) = named("luke") {
        if let Some(message) =
            rebellion_render::story_messages::luke_leaves_dagobah(world, luke, 0, texts)
        {
            log.push(message);
        }
    }
}

/// The event-message scenario's capture of the reader's own character
/// (`FUN_00490340`, 0x7158 / 0x7159), filed before the Emperor's so it
/// lists below: Han Solo taken by the Empire at Yavin for the Alliance (A0
/// alliance-game2/han-solo-captured), Ozzel taken by the Alliance at Tralus
/// for the Empire (A0 empire-game3/ozzel-captured).
pub fn post_own_capture_message(
    request: FixtureRequest,
    world: &GameWorld,
    log: &mut rebellion_render::MessageLog,
    texts: &std::collections::HashMap<u16, String>,
) {
    if request.scenario != Scenario::EventMessages {
        return;
    }
    let (name, system, captor_is_alliance) = match request.faction {
        CockpitFaction::Alliance => ("Han Solo", "Yavin", false),
        CockpitFaction::Empire => ("Ozzel", "Tralus", true),
    };
    let Some(character) = world
        .characters
        .values()
        .find(|character| character.name == name)
    else {
        return;
    };
    let at = world
        .systems
        .iter()
        .find(|(_, value)| value.name == system)
        .map(|(key, _)| key);
    if let Some(message) = rebellion_render::story_messages::captured_message(
        at,
        character,
        captor_is_alliance,
        system,
        request.faction,
        0,
        texts,
    ) {
        log.push(message);
    }
}

/// The EventMessages scenario's Character Captured message (`FUN_00490340`,
/// notification `0x1e0`): the Emperor taken by the Alliance at Corfai, as
/// A0 messages-day633 shows it, for each side's reader.
pub fn post_capture_message(
    request: FixtureRequest,
    world: &GameWorld,
    log: &mut rebellion_render::MessageLog,
    texts: &std::collections::HashMap<u16, String>,
) {
    if request.scenario != Scenario::EventMessages {
        return;
    }
    let Some(emperor) = world
        .characters
        .values()
        .find(|character| character.is_empire && character.dat_id.index() == 0x280)
    else {
        return;
    };
    let corfai = world
        .systems
        .iter()
        .find(|(_, system)| system.name == "Corfai")
        .map(|(key, _)| key);
    if let Some(message) = rebellion_render::story_messages::captured_message(
        corfai,
        emperor,
        true,
        "Corfai",
        request.faction,
        0,
        texts,
    ) {
        log.push(message);
    }
}

/// The event-message scenario's Character Killed for the Alliance reader
/// (`FUN_0048fdf0`, notification `0x106` on a destroyed character): Jan
/// Dodonna, as A0 alliance-game2/jan-dodonna-killed shows "Jan Dodonna has
/// been killed." over the Alliance's coffin in an Alliance game.
#[must_use]
pub fn killed_character(
    request: FixtureRequest,
    world: &GameWorld,
) -> Option<rebellion_core::ids::CharacterKey> {
    if request.scenario != Scenario::EventMessages {
        return None;
    }
    // The Empire's reader: Jerjerrod over the Empire's coffin 0x433 (A0
    // empire-game3/jerjerrod-killed, a new Empire game, day 682).
    let name = match request.faction {
        CockpitFaction::Alliance => "Jan Dodonna",
        CockpitFaction::Empire => "Jerjerrod",
    };
    world
        .characters
        .iter()
        .find(|(_, character)| character.name.contains(name))
        .map(|(key, _)| key)
}

/// The event-message scenario's Character Injured for the Alliance reader
/// (`FUN_0048fdf0`, notification `0x106` with the injury `+0x94` not 0):
/// Leia Organa, as A0 health-2026-11-09/01-leia-injured-day270 shows
/// "Leia Organa has been injured." over STRATEGY 1050 in an Alliance game
/// (after her Incite Uprising at Bortras was foiled), and 03-leia-healed-
/// day372 "Leia Organa has been healed of all injuries." over 0x41b with
/// her portrait. No Empire capture.
#[must_use]
pub fn injured_character(
    request: FixtureRequest,
    world: &GameWorld,
) -> Option<rebellion_core::ids::CharacterKey> {
    if request.scenario != Scenario::EventMessages || request.faction != CockpitFaction::Alliance {
        return None;
    }
    world
        .characters
        .iter()
        .find(|(_, character)| character.name.contains("Leia"))
        .map(|(key, _)| key)
}

/// The event-message scenario's smuggling for the Alliance reader
/// (`FUN_00499130`): Yavin, as A0 alliance-game2/smuggling-benefits shows
/// "Smugglers from Yavin are providing us with additional resources." in
/// an Alliance game after the Empire took it.
#[must_use]
pub fn smuggled_system(
    request: FixtureRequest,
    world: &GameWorld,
) -> Option<rebellion_core::ids::SystemKey> {
    if request.scenario != Scenario::EventMessages {
        return None;
    }
    // The Empire's reader: Coruscant, held by the Alliance (A0
    // empire-game3/smuggling-benefits, a new Empire game, day 681).
    let name = match request.faction {
        CockpitFaction::Alliance => "Yavin",
        CockpitFaction::Empire => "Coruscant",
    };
    world
        .systems
        .iter()
        .find(|(_, system)| system.name == name)
        .map(|(key, _)| key)
}

/// The event-message scenario's Research Complete for the Alliance reader
/// (`FUN_00499aa0`): the ship tree's level that opens the Nebulon-B
/// Frigate, as A0 research-complete/01 shows "Nebulon-B Frigate Research
/// Complete" in an Alliance game (day 204).
#[must_use]
pub fn researched_class(
    request: FixtureRequest,
    world: &GameWorld,
) -> Option<(rebellion_core::research::TechType, u32)> {
    if request.scenario != Scenario::EventMessages || request.faction != CockpitFaction::Alliance {
        return None;
    }
    let nebulon = world
        .capital_ship_classes
        .values()
        .find(|class| class.is_alliance && class.name == "Nebulon-B Frigate")?;
    let level = nebulon.research_order;
    let tech = rebellion_core::research::TechType::Ship;
    (rebellion_render::story_messages::research_complete_class(world, true, tech, level)
        == Some(nebulon.dat_id))
    .then_some((tech, level))
}

/// The event-message scenario's fleet arrival for the Alliance reader
/// (`FUN_004981c0`, case 1): the Alliance's Fleet 1 at Sluis Van, as A0
/// events-day673/04 shows "Fleet Arrives at Sluis Van" in an Alliance game.
/// The app files it on the Fleet rail.
#[must_use]
pub fn arrived_fleet(
    request: FixtureRequest,
    world: &GameWorld,
) -> Option<(
    rebellion_core::ids::FleetKey,
    rebellion_core::ids::SystemKey,
)> {
    if request.scenario != Scenario::EventMessages || request.faction != CockpitFaction::Alliance {
        return None;
    }
    let fleet = world
        .fleets
        .iter()
        .find(|&(key, fleet)| fleet.is_alliance && world.fleet_name(key) == Some("Fleet 1"))
        .map(|(key, _)| key)?;
    let system = world
        .systems
        .iter()
        .find(|(_, system)| system.name == "Sluis Van")
        .map(|(key, _)| key)?;
    Some((fleet, system))
}

/// The event-message scenario's blockade for the Alliance reader
/// (`FUN_004960f0`, notification `0x14e` with no battle there): Yavin, held
/// by the Alliance with a fleet there, as A0 events-day673/02 shows "Yavin
/// Under Blockade" in an Alliance game. The app files it before Saboteurs
/// Strike, so it lists last on the Conflict rail.
#[must_use]
pub fn blockaded_system(
    request: FixtureRequest,
    world: &GameWorld,
) -> Option<rebellion_core::ids::SystemKey> {
    if request.scenario != Scenario::EventMessages || request.faction != CockpitFaction::Alliance {
        return None;
    }
    world
        .systems
        .iter()
        .find(|(_, system)| system.name == "Yavin")
        .map(|(key, _)| key)
}

/// The event-message scenario's sabotaged regiment: the first Alliance
/// regiment on a planet and its system. The app posts Saboteurs Strike
/// (`FUN_0048b9a0`) for it before the battle reports, so it lists last on
/// the Alliance's Conflict rail (A0 saboteurs-clean/test-day651-saboteurs-
/// yavin, an unedited Alliance game).
#[must_use]
pub fn sabotaged_regiment(
    request: FixtureRequest,
    world: &GameWorld,
) -> Option<(
    rebellion_core::ids::SystemKey,
    rebellion_core::ids::TroopKey,
)> {
    if request.scenario != Scenario::EventMessages {
        return None;
    }
    world.systems.iter().find_map(|(key, system)| {
        system
            .ground_units
            .iter()
            .copied()
            .find(|&troop| {
                world
                    .troops
                    .get(troop)
                    .is_some_and(|troop| troop.is_alliance)
            })
            .map(|troop| (key, troop))
    })
}

/// The event-message scenario's battle reports on its reader's Conflict rail
/// (`FUN_0048bc60`, category `0x100`), as A0 messages-day633/index-tab7's
/// "Assault on Coruscant" over the older "Assault on Umgul", and a
/// construction yard's idle report on the Manufacturing rail, as A0
/// manufacturing-messages/.
pub fn post_conflict_report(request: FixtureRequest, log: &mut rebellion_render::MessageLog) {
    if request.scenario != Scenario::EventMessages {
        return;
    }
    // A0 manufacturing-messages/q01..q05: a construction yard's idle report
    // on the Manufacturing rail (Facility Idle, 0x71f8).
    log.push(
        rebellion_render::GameMessage::new(
            0,
            "The Construction Yard on Womrik is idle.",
            rebellion_render::MessageCategory::Manufacturing,
        )
        .with_title("Construction Yard Idle on Womrik")
        .on_rail(
            rebellion_render::MessageRail::Manufacturing,
            rebellion_render::message_log::RailAudience::side(
                request.faction == CockpitFaction::Alliance,
            ),
        ),
    );
    for system in ["Umgul", "Coruscant"] {
        log.push(
            rebellion_render::GameMessage::new(
                0,
                format!("The Alliance has assaulted {system}."),
                rebellion_render::MessageCategory::Combat,
            )
            .with_title(format!("Assault on {system}"))
            .on_rail(
                rebellion_render::MessageRail::Conflict,
                rebellion_render::message_log::RailAudience::side(
                    request.faction == CockpitFaction::Alliance,
                ),
            ),
        );
    }
}

/// The event-message scenario's systems joining the Empire, for the
/// Alliance reader, in the order they join, each with its day: Corellia,
/// then Commenor, as A0 message-index/category-1-day228 lists them, newest
/// first.
pub fn systems_joining_the_empire(request: FixtureRequest) -> Vec<(&'static str, u64)> {
    if request.scenario != Scenario::EventMessages || request.faction != CockpitFaction::Alliance {
        return Vec::new();
    }
    vec![("Corellia", 0), ("Commenor", 0)]
}

/// The event-message scenario's systems joining the Alliance, for the
/// Empire reader: Commenor, as A0 empire-game3/joins-enemy shows
/// "Commenor Joins Enemy" in a new Empire game (day 220).
pub fn systems_joining_the_alliance(request: FixtureRequest) -> Vec<(&'static str, u64)> {
    if request.scenario != Scenario::EventMessages || request.faction != CockpitFaction::Empire {
        return Vec::new();
    }
    vec![("Commenor", 0)]
}

/// The event-message scenario's Informants Provide Information
/// (`FUN_00491060`): Selonia, as A0 informants-clean/test-day1457-selonia
/// shows it to the Alliance, and Uvena, as A0 empire-clean/
/// 04-day188-informants-uvena shows it to the Empire.
pub fn informant_reports(request: FixtureRequest) -> Vec<&'static str> {
    if request.scenario != Scenario::EventMessages {
        return Vec::new();
    }
    if request.faction == CockpitFaction::Alliance {
        vec!["Selonia"]
    } else {
        vec!["Uvena"]
    }
}

/// The event-message scenario's Force discovery (`FUN_0048f1e0`, timer
/// `0x389`), "Future Jedi Discovered" as A0 shows it: for the Empire Vader
/// finds Ozzel's Force (research-display/08-future-jedi, a new Easy Empire
/// game, day 107); for the Alliance Luke, not yet a Jedi Knight, finds Jan
/// Dodonna's (future-jedi-alliance/01, an Alliance game, day 607).
pub fn force_discoveries(request: FixtureRequest) -> Vec<(&'static str, &'static str)> {
    if request.scenario != Scenario::EventMessages {
        return Vec::new();
    }
    match request.faction {
        CockpitFaction::Empire => vec![("Vader", "Ozzel")],
        CockpitFaction::Alliance => vec![("Luke", "Dodonna")],
    }
}

/// The trainer and the found character of each of [`force_discoveries`],
/// found by name in `world`. Luke has finished Dagobah, as in A0's day 607
/// (his picture the Jedi Knight's, `FUN_0056f550`).
pub fn place_force_discoveries(
    request: FixtureRequest,
    world: &mut GameWorld,
) -> Vec<(
    rebellion_core::ids::CharacterKey,
    rebellion_core::ids::CharacterKey,
)> {
    let named = |world: &GameWorld, needle: &str| {
        world
            .characters
            .iter()
            .find(|(_, character)| character.name.contains(needle))
            .map(|(key, _)| key)
    };
    let mut placed = Vec::new();
    for (trainer, found) in force_discoveries(request) {
        if let (Some(trainer), Some(found)) = (named(world, trainer), named(world, found)) {
            placed.push((trainer, found));
        }
    }
    for &(trainer, _) in &placed {
        let character = &mut world.characters[trainer];
        if character.name.contains("Luke") {
            character.dagobah_completed = true;
        }
    }
    placed
}

/// The event-message scenario's Maintenance Shortfall (`FUN_00498970`): a
/// corvette scrapped in hyperspace, so at no system, as A0
/// maintenance-shortfall/test-day1224-corvette-no-system shows the
/// Alliance "Corellian Corvette 2".
pub fn maintenance_shortfalls(request: FixtureRequest) -> Vec<&'static str> {
    if request.scenario != Scenario::EventMessages {
        return Vec::new();
    }
    vec!["Corellian Corvette 2"]
}

/// The event-message scenario's Near Uprising (`FUN_00498cb0`): the
/// reader's own Umgul, garrisoned, its troop surplus fallen to 0, as A0
/// uprising-clean/test-day795-umgul-near-uprising shows it to the
/// Alliance.
pub fn near_uprisings(request: FixtureRequest) -> Vec<&'static str> {
    if request.scenario != Scenario::EventMessages {
        return Vec::new();
    }
    vec!["Umgul"]
}

/// The event-message scenario's Personnel Arrive: Mon Mothma's Move landing
/// at Klatooine, as A0 mission-reports/index-day914-top lists "Mon Mothma
/// Arrives at Klatooine". The app posts it through
/// `push_personnel_arrivals`.
pub fn personnel_arrivals(
    request: FixtureRequest,
    world: &GameWorld,
) -> Vec<rebellion_core::missions::PersonnelArrival> {
    if request.scenario != Scenario::EventMessages {
        return Vec::new();
    }
    let klatooine = world
        .systems
        .iter()
        .find(|(_, system)| system.name == "Klatooine")
        .map(|(key, _)| key);
    let mothma = world
        .characters
        .iter()
        .find(|(_, character)| character.name == "Mon Mothma")
        .map(|(key, _)| key);
    let (Some(system), Some(mothma)) = (klatooine, mothma) else {
        return Vec::new();
    };
    vec![rebellion_core::missions::PersonnelArrival {
        mission_id: u64::MAX - 3,
        faction: MissionFaction::Alliance,
        system,
        members: vec![MissionMember::Character(mothma)],
        tick: 0,
    }]
}

/// The event-message scenario's mission results: the Emperor recruits
/// Thrawn at Coruscant, then fails his Espionage at Chandrila, his team
/// going home to Coruscant, as A0 mission-reports/ reports them to the
/// Empire. The app files it through `apply_mission_result`.
pub fn mission_results(
    request: FixtureRequest,
    world: &GameWorld,
) -> Vec<rebellion_core::missions::MissionResult> {
    if request.scenario != Scenario::EventMessages {
        return Vec::new();
    }
    let system = |name: &str| {
        world
            .systems
            .iter()
            .find(|(_, system)| system.name == name)
            .map(|(key, _)| key)
    };
    let emperor = world
        .characters
        .iter()
        .find(|(_, character)| character.name == "Emperor Palpatine")
        .map(|(key, _)| key);
    let thrawn = world
        .characters
        .iter()
        .find(|(_, character)| character.name == "Thrawn")
        .map(|(key, _)| key);
    let (Some(chandrila), Some(coruscant), Some(emperor), Some(thrawn)) =
        (system("Chandrila"), system("Coruscant"), emperor, thrawn)
    else {
        return Vec::new();
    };
    // A0 mission-reports/emperor-recruits-thrawn-day43: "Emperor Palpatine
    // Recruits Thrawn".
    let recruited = rebellion_core::missions::MissionResult {
        mission_id: u64::MAX - 2,
        tick: 0,
        kind: MissionKind::Recruitment,
        faction: MissionFaction::Empire,
        character: Some(emperor),
        target_system: coruscant,
        target_character: None,
        target_object: None,
        returning_to: None,
        outcome: rebellion_core::missions::MissionOutcome::Success,
        effects: vec![rebellion_core::missions::MissionEffect::CharacterRecruited {
            character: thrawn,
            system: coruscant,
            faction: MissionFaction::Empire,
            pool_emptied: false,
        }],
    };
    vec![recruited, rebellion_core::missions::MissionResult {
        mission_id: u64::MAX - 1,
        tick: 0,
        kind: MissionKind::Espionage,
        faction: MissionFaction::Empire,
        character: Some(emperor),
        target_system: chandrila,
        target_character: None,
        target_object: None,
        returning_to: Some(rebellion_core::missions::ReturnPlace::System(coruscant)),
        outcome: rebellion_core::missions::MissionOutcome::Failure,
        effects: Vec::new(),
    }]
}

/// The event-message scenario's foiled mission: an Imperial Reconnaissance
/// mission to Chandrila ended by a battle (code 7), as A0
/// mission-reports/enemy-foiled-chandrila-day739 reports it to the
/// Alliance. The app posts it through `push_foiled_missions`.
pub fn foiled_missions(
    request: FixtureRequest,
    world: &GameWorld,
) -> Vec<rebellion_core::missions::MissionEnded> {
    if request.scenario != Scenario::EventMessages {
        return Vec::new();
    }
    let Some(chandrila) = world
        .systems
        .iter()
        .find(|(_, system)| system.name == "Chandrila")
        .map(|(system, _)| system)
    else {
        return Vec::new();
    };
    vec![
        rebellion_core::missions::MissionEnded {
            mission_id: u64::MAX,
            kind: MissionKind::Reconnaissance,
            faction: MissionFaction::Empire,
            character: None,
            target_system: chandrila,
            end_code: 7,
            tick: 0,
        },
        // A0 empire-clean/02-day83-enemy-foiled-chandrila (a new Easy
        // Empire game): the Alliance's Espionage at Chandrila, foiled, as
        // the Empire reads it.
        rebellion_core::missions::MissionEnded {
            mission_id: u64::MAX - 4,
            kind: MissionKind::Espionage,
            faction: MissionFaction::Alliance,
            character: None,
            target_system: chandrila,
            end_code: 7,
            tick: 0,
        },
    ]
}

/// The event-message scenario's messages: one per report class whose
/// picture the port attaches (`message_pictures`), titled by its class and
/// filed under the Mission rail so one category lists them all.
pub fn post_event_messages(
    request: FixtureRequest,
    world: &mut GameWorld,
    missions: &mut MissionState,
    log: &mut rebellion_render::MessageLog,
) {
    if request.scenario != Scenario::EventMessages {
        return;
    }
    // A player agent on a Diplomacy mission at the first system, which the
    // Mission Report asks to continue (FUN_004927c0 case 0x51: 0x70db,
    // kind 5, the Abort it holds). It has not reached phase 8, so its X is
    // refused (FUN_00521600, 0x41).
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let asked = world.systems.keys().next().and_then(|system| {
        let agent = world
            .characters
            .iter()
            .find(|(_, character)| {
                character.is_alliance == player_is_alliance
                    && character.is_empire != player_is_alliance
                    && !character.is_killed
                    && !character.is_captive
            })
            .map(|(key, _)| key)?;
        world.characters[agent].current_system = Some(system);
        let id = missions.dispatch(rebellion_core::missions::MissionRequest::single(
            MissionKind::Diplomacy,
            if player_is_alliance {
                MissionFaction::Alliance
            } else {
                MissionFaction::Empire
            },
            agent,
            system,
            None,
            0,
        ));
        rebellion_core::missions::set_on_mission(world, MissionMember::Character(agent), true);
        Some(id)
    });
    let world: &GameWorld = world;
    use rebellion_core::manufacturing::BuildableKind;
    use rebellion_render::message_pictures::{self as pictures, Arrival, ReportedMission};
    let leia = world
        .characters
        .iter()
        .find(|(_, character)| character.name.to_lowercase().contains("leia"))
        .map(|(_, character)| (character.dat_id, character.is_major));
    let reader = request.faction;
    let side = match reader {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    };
    let posts = [
        (
            "Mission Report",
            Some(pictures::mission_report(
                ReportedMission::Diplomacy,
                true,
                reader,
                leia,
            )),
        ),
        ("Uprising Begins", Some(pictures::uprising_begins())),
        ("Uprising Ends", pictures::uprising_ends(side)),
        ("Blockade", pictures::blockade(side)),
        ("Fleet Arrives", pictures::arrival(Arrival::Fleet, reader)),
        ("Units Arrive", pictures::arrival(Arrival::Units, reader)),
        (
            "Shipyard Deployed",
            pictures::deployed(
                world,
                BuildableKind::ManufacturingFacility(
                    rebellion_core::manufacturing::FacilityBuild {
                        class: rebellion_core::ids::DatId::new(0x2800_0001),
                        is_alliance: reader == CockpitFaction::Alliance,
                    },
                ),
            ),
        ),
        (
            "Regiment Deployed",
            pictures::deployed(
                world,
                BuildableKind::Troop(rebellion_core::ids::DatId::new(
                    if reader == CockpitFaction::Alliance {
                        0x1000_0002
                    } else {
                        0x1000_0006
                    },
                )),
            ),
        ),
        ("Character Killed", Some(pictures::killed(side))),
        ("Research Complete", Some(pictures::research())),
    ];
    // The arrival names its system, which Display (0x67) goes to
    // (FUN_004981c0 sets the message's object).
    let arrival_system = world.systems.keys().next();
    for (title, picture) in posts {
        let message = match arrival_system.filter(|_| title == "Fleet Arrives") {
            Some(system) => rebellion_render::GameMessage::at_system(
                0,
                format!("{title}."),
                rebellion_render::MessageCategory::Event,
                system,
            ),
            None => rebellion_render::GameMessage::new(
                0,
                format!("{title}."),
                rebellion_render::MessageCategory::Event,
            ),
        };
        // The report names its mission at the first system, which Display
        // goes to (A0 mission-reports/report-display-missions-window-day235).
        let message = match asked.filter(|_| title == "Mission Report") {
            Some(id) => match arrival_system {
                Some(system) => rebellion_render::GameMessage::at_system(
                    0,
                    format!("{title}."),
                    rebellion_render::MessageCategory::Event,
                    system,
                ),
                None => message,
            }
            .with_continue_question(id)
            .with_mission(id),
            None => message,
        };
        // Research Complete is kind 4: Display opens the Encyclopedia at
        // the class (class 0x0065bb40, FUN_00467f10 -> FUN_0041d6b0).
        let message = match rebellion_render::story_messages::research_complete_encyclopedia_object(
            world,
            reader == CockpitFaction::Alliance,
            rebellion_core::research::TechType::Ship,
            1,
        )
        .filter(|_| title == "Research Complete")
        {
            Some(object) => message.with_encyclopedia(object),
            None => message,
        };
        log.push(
            message
            .with_title(title)
            .with_picture_if(picture)
            .on_rail(
                rebellion_render::story_messages::STORY_RAIL,
                rebellion_render::message_log::RailAudience::Both,
            ),
        );
    }
}

/// The story-message gate's view: where the Mission rail button is, where
/// the Message window opens, and what it shows.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureStoryMessages {
    status: &'static str,
    code: u32,
    rail_x: f32,
    rail_y: f32,
    window_left: f32,
    window_top: f32,
    index_open: bool,
    mode: &'static str,
    rows: Vec<String>,
    viewing_title: Option<String>,
    viewing_text: Option<String>,
    viewing_background: Option<u32>,
    viewing_foreground: Option<u32>,
    /// The viewed message's open question: the mission it asks about.
    viewing_question: Option<u64>,
}

fn story_messages_report(
    request: FixtureRequest,
    log: &rebellion_render::MessageLog,
    index: Option<&rebellion_render::message_index::MessageIndexState>,
    player_is_alliance: bool,
) -> Option<FixtureStoryMessages> {
    if !matches!(
        request.scenario,
        Scenario::StoryMessages | Scenario::EventMessages
    ) {
        return None;
    }
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let rail = rebellion_render::cockpit::strategic_message_index_controls(request.faction)
        .iter()
        .zip(rebellion_render::MessageRail::RAIL_ORDER)
        .find(|(_, rail)| *rail == rebellion_render::story_messages::STORY_RAIL)
        .map(|(control, _)| {
            let rect = control.rect;
            egui_macroquad::egui::pos2(
                layout.canvas.x + (rect.x + rect.width / 2.0) * layout.scale,
                layout.canvas.y + (rect.y + rect.height / 2.0) * layout.scale,
            )
        })?;
    let window = rebellion_render::message_index::window_rect(layout).min;
    let report = index.map(|index| index.report(log, player_is_alliance));
    let viewing = report
        .as_ref()
        .and_then(|report| report.viewing)
        .and_then(|id| log.messages().iter().find(|message| message.id == id));
    Some(FixtureStoryMessages {
        status: "story-messages",
        code: request.code,
        rail_x: rail.x,
        rail_y: rail.y,
        window_left: window.x,
        window_top: window.y,
        index_open: report.is_some(),
        mode: match report.as_ref().map(|report| report.mode) {
            Some(rebellion_render::message_index::IndexMode::SingleMessage) => "single",
            Some(rebellion_render::message_index::IndexMode::List) => "list",
            None => "closed",
        },
        rows: report
            .map(|report| report.rows.into_iter().map(|row| row.text).collect())
            .unwrap_or_default(),
        viewing_title: viewing.and_then(|message| message.title.clone()),
        viewing_text: viewing.map(|message| message.text.clone()),
        viewing_background: viewing.and_then(|message| message.picture).map(|picture| picture.background),
        viewing_foreground: viewing
            .and_then(|message| message.picture)
            .and_then(|picture| picture.foreground),
        viewing_question: viewing.and_then(|message| message.continue_question),
    })
}

pub fn emit_story_messages(
    request: FixtureRequest,
    log: &rebellion_render::MessageLog,
    index: Option<&rebellion_render::message_index::MessageIndexState>,
    player_is_alliance: bool,
) {
    let Some(report) = story_messages_report(request, log, index, player_is_alliance) else {
        return;
    };
    let bytes = serde_json::to_vec(&report).expect("serialize the story-message report");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

#[derive(Debug, Serialize, PartialEq)]
struct FixtureSystemPoint {
    dat_id: u32,
    x: f32,
    y: f32,
}

/// Where the fleet-move gate presses and releases: the moving fleet's cell,
/// the target planet in the sector window, every system on the map, and
/// each planet the open sector windows show.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureFleetMoveSetup {
    status: &'static str,
    code: u32,
    primary_dat_id: u32,
    fleet_item_x: f32,
    fleet_item_y: f32,
    target_dat_id: u32,
    target_screen_x: f32,
    target_screen_y: f32,
    /// The target planet's fleet icon, which opens its Fleet window.
    target_fleet_icon_x: f32,
    target_fleet_icon_y: f32,
    systems: Vec<FixtureSystemPoint>,
    sector_planets: Vec<FixtureSystemPoint>,
}

fn fleet_move_setup(
    request: FixtureRequest,
    world: &GameWorld,
    sectors: &SectorWindowState,
    fleets: &FleetWindowState,
) -> Option<FixtureFleetMoveSetup> {
    if !request.scenario.moves_a_fleet() {
        return None;
    }
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let mut keys = world.systems.keys();
    let (primary, target) = (keys.next()?, keys.next()?);
    // The fleet is the only one at its system: the Fleet window's first entry.
    let item = fleets.entry_screen_rect(layout, primary, 0)?.center();
    let planet = sectors.planet_screen_rect(world, layout, target)?.center();
    let icon = sectors.fleet_icon_screen_rect(world, layout, target)?.center();
    Some(FixtureFleetMoveSetup {
        status: "fleet-move-setup",
        code: request.code,
        primary_dat_id: world.systems[primary].dat_id.raw(),
        fleet_item_x: item.x,
        fleet_item_y: item.y,
        target_dat_id: world.systems[target].dat_id.raw(),
        target_screen_x: planet.x,
        target_screen_y: planet.y,
        target_fleet_icon_x: icon.x,
        target_fleet_icon_y: icon.y,
        systems: world
            .systems
            .values()
            .map(|system| {
                let (x, y) = screen_point(system, request.faction);
                FixtureSystemPoint {
                    dat_id: system.dat_id.raw(),
                    x,
                    y,
                }
            })
            .collect(),
        sector_planets: world
            .systems
            .iter()
            .filter_map(|(key, system)| {
                let center = sectors.planet_screen_rect(world, layout, key)?.center();
                // A planet placed past its window's edge is clipped away.
                if !sectors.contains_screen_point(layout, (center.x, center.y)) {
                    return None;
                }
                Some(FixtureSystemPoint {
                    dat_id: system.dat_id.raw(),
                    x: center.x,
                    y: center.y,
                })
            })
            .collect(),
    })
}

pub fn emit_fleet_move_setup(
    request: FixtureRequest,
    world: &GameWorld,
    sectors: &SectorWindowState,
    fleets: &FleetWindowState,
) {
    let Some(report) = fleet_move_setup(request, world, sectors, fleets) else {
        return;
    };
    let bytes = serde_json::to_vec(&report).expect("serialize the fleet move setup");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// What the fleet-move gate checks after each step: the fleet's transit
/// order, whether the confirmation window is open, the map's selection, and
/// the last message (a refusal's text).
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FleetMoveObservation {
    status: &'static str,
    code: u32,
    in_transit: bool,
    destination_dat_id: Option<u32>,
    confirmation_open: bool,
    selected_system_dat_id: Option<u32>,
    last_message: Option<String>,
    /// The fleet's entry in its destination's Fleet window, once that
    /// window is open and lists it.
    destination_entry: Option<[f32; 2]>,
    /// The first right-list item of that window while the fleet is its
    /// selection: the fleet's first ship on the Capital Ships tab.
    destination_item: Option<[f32; 2]>,
}

fn fleet_move_observation(
    request: FixtureRequest,
    world: &GameWorld,
    movement: &MovementState,
    confirmation_open: bool,
    map: &GalaxyMapState,
    (fleets, fog, transport): (&FleetWindowState, &FogState, &TroopTransportState),
    messages: &[GameMessage],
) -> Option<FleetMoveObservation> {
    if !request.scenario.moves_a_fleet() {
        return None;
    }
    let fleet = world.fleets.keys().next()?;
    let dat_id = |key| world.systems.get(key).map(|system| system.dat_id.raw());
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let destination_entry = movement.get(fleet).and_then(|order| {
        let report = fleets.report(world, movement, fog, transport, order.destination)?;
        let row = report
            .fleet_rows
            .iter()
            .find_map(|&(listed, row)| (listed == fleet).then_some(row))?;
        let center = fleets
            .entry_screen_rect(layout, order.destination, row)?
            .center();
        Some([center.x, center.y])
    });
    let destination_item = movement.get(fleet).and_then(|order| {
        let (selected, tab) = fleets.selection(order.destination)?;
        if selected != Some(FleetWindowEntry::Fleet(fleet)) || tab != FleetWindowTab::CapitalShips
        {
            return None;
        }
        let center = fleets.item_screen_rect(layout, order.destination, 0)?.center();
        Some([center.x, center.y])
    });
    Some(FleetMoveObservation {
        status: "fleet-move",
        code: request.code,
        in_transit: movement.is_in_transit(fleet),
        destination_dat_id: movement
            .get(fleet)
            .and_then(|order| dat_id(order.destination)),
        confirmation_open,
        selected_system_dat_id: map.selected_system.and_then(dat_id),
        last_message: messages.last().map(|message| message.text.clone()),
        destination_entry,
        destination_item,
    })
}

/// Emits a [`FleetMoveObservation`] whenever it changes.
#[derive(Debug, Default)]
pub struct FleetMoveWatch {
    last: Option<FleetMoveObservation>,
}

impl FleetMoveWatch {
    fn changed(&mut self, now: Option<FleetMoveObservation>) -> Option<&FleetMoveObservation> {
        if now.is_none() || now == self.last {
            return None;
        }
        self.last = now;
        self.last.as_ref()
    }

    pub fn observe(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        movement: &MovementState,
        confirmation_open: bool,
        map: &GalaxyMapState,
        windows: (&FleetWindowState, &FogState, &TroopTransportState),
        messages: &[GameMessage],
    ) {
        let now = fleet_move_observation(
            request,
            world,
            movement,
            confirmation_open,
            map,
            windows,
            messages,
        );
        let Some(report) = self.changed(now) else {
            return;
        };
        let bytes = serde_json::to_vec(report).expect("serialize the fleet move observation");
        unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
    }
}

fn screen_center(rect: egui_macroquad::egui::Rect) -> (f32, f32) {
    (rect.center().x, rect.center().y)
}

/// One quadrant overlay's screen rect, as `[left, top, width, height]`.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureQuadrant {
    system_dat_id: u32,
    quadrant: &'static str,
    rect: [f32; 4],
}

/// Where the quadrant gate looks: every quadrant overlay rect of the primary
/// system and the second system, and the layout's scale.
#[derive(Debug, Serialize, PartialEq)]
struct FixtureQuadrantSetup {
    status: &'static str,
    code: u32,
    primary_dat_id: u32,
    second_dat_id: u32,
    scale: f32,
    quadrants: Vec<FixtureQuadrant>,
    /// The Alliance headquarters' planet rect, and the planet rect of the
    /// player's system in uprising.
    headquarters: Option<[f32; 4]>,
    uprising: Option<[f32; 4]>,
    /// An explored neutral system with a mine: its planet rect and its
    /// system icon's rect (A0 sector-b: neutral names white, the icon
    /// STRATEGY 10787).
    neutral: Option<[[f32; 4]; 2]>,
    /// The neutral system's defenses icon rect (its player battery).
    neutral_defenses: Option<[f32; 4]>,
    /// A destroyed system's rect (A0 menus3/05: Averam's asteroid field
    /// STRATEGY 10240 and its white name).
    destroyed: Option<[f32; 4]>,
    /// The day readout's centre, which opens the Game Speed menu.
    day_readout: (f32, f32),
}

/// Move the headquarters object (ALLFACSD, family `0x20`) to `to`, with the
/// flag: the sector window's mark and menu stand for that object
/// (`FUN_00526cf0`, `FUN_0052bed0`).
fn move_headquarters_object(world: &mut GameWorld, to: SystemKey) {
    let is_headquarters =
        |world: &GameWorld, key: &rebellion_core::ids::ManufacturingFacilityKey| {
            world
                .manufacturing_facilities
                .get(*key)
                .is_some_and(|facility| facility.class_dat_id.family() == 0x20)
        };
    let held: Vec<_> = world
        .manufacturing_facilities
        .iter()
        .filter(|(_, facility)| facility.class_dat_id.family() == 0x20)
        .map(|(key, _)| key)
        .collect();
    let keys: Vec<SystemKey> = world.systems.keys().collect();
    for system in keys {
        let mut list = std::mem::take(&mut world.systems[system].manufacturing_facilities);
        list.retain(|key| !is_headquarters(world, key));
        world.systems[system].manufacturing_facilities = list;
    }
    let hq = held.first().copied().unwrap_or_else(|| {
        world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2000_0001),
                side: Faction::Alliance,
                is_shipyard: false,
            },
        )
    });
    for extra in held.iter().skip(1) {
        world.manufacturing_facilities.remove(*extra);
    }
    world.systems[to].manufacturing_facilities.push(hq);
}

/// The primary's sector's first four systems other than the primary and
/// the second system: the headquarters, the system in uprising, a neutral
/// one and a destroyed one.
fn mark_systems(world: &GameWorld, primary: SystemKey) -> Vec<SystemKey> {
    let Some(second) = loading_target(world, primary) else {
        return Vec::new();
    };
    world
        .sectors
        .get(world.systems[primary].sector)
        .map(|sector| {
            sector
                .systems
                .iter()
                .copied()
                .filter(|&key| key != primary && key != second)
                .take(4)
                .collect()
        })
        .unwrap_or_default()
}

fn quadrant_setup(
    request: FixtureRequest,
    world: &GameWorld,
    sectors: &SectorWindowState,
) -> Option<FixtureQuadrantSetup> {
    let (primary, second) = quadrant_systems(request, world)?;
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let mut quadrants = Vec::new();
    for system in [primary, second] {
        for (quadrant, name) in [
            (Quadrant::System, "system"),
            (Quadrant::Defenses, "defenses"),
            (Quadrant::Fleets, "fleets"),
            (Quadrant::Missions, "missions"),
        ] {
            let rect = sectors.quadrant_screen_rect(world, layout, system, quadrant)?;
            quadrants.push(FixtureQuadrant {
                system_dat_id: world.systems[system].dat_id.raw(),
                quadrant: name,
                rect: [rect.min.x, rect.min.y, rect.width(), rect.height()],
            });
        }
    }
    let planet = |system: Option<&SystemKey>| {
        system
            .and_then(|&system| sectors.planet_screen_rect(world, layout, system))
            .map(|rect| [rect.min.x, rect.min.y, rect.width(), rect.height()])
    };
    let marks = if request.scenario == Scenario::Quadrants {
        mark_systems(world, primary)
    } else {
        Vec::new()
    };
    Some(FixtureQuadrantSetup {
        status: "quadrant-setup",
        code: request.code,
        primary_dat_id: world.systems[primary].dat_id.raw(),
        second_dat_id: world.systems[second].dat_id.raw(),
        scale: layout.scale,
        quadrants,
        headquarters: planet(marks.first()),
        uprising: planet(marks.get(1)),
        neutral: marks.get(2).and_then(|&system| {
            let icon = sectors.quadrant_screen_rect(world, layout, system, Quadrant::System)?;
            Some([
                planet(Some(&system))?,
                [icon.min.x, icon.min.y, icon.width(), icon.height()],
            ])
        }),
        neutral_defenses: marks.get(2).and_then(|&system| {
            let icon = sectors.quadrant_screen_rect(world, layout, system, Quadrant::Defenses)?;
            Some([icon.min.x, icon.min.y, icon.width(), icon.height()])
        }),
        destroyed: planet(marks.get(3)),
        day_readout: day_readout_centre(request.faction, layout),
    })
}

pub fn emit_quadrant_setup(
    request: FixtureRequest,
    world: &GameWorld,
    sectors: &SectorWindowState,
) {
    let Some(report) = quadrant_setup(request, world, sectors) else {
        return;
    };
    let bytes = serde_json::to_vec(&report).expect("serialize the quadrant setup");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// One open Defenses window, for the quadrant gate: what it shows and where
/// its tab buttons and listed cells are, as `[left, top, width, height]`.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FixtureDefensesWindow {
    system_dat_id: u32,
    origin: (i16, i16),
    side: u8,
    page: &'static str,
    counts: [usize; 5],
    rows: Vec<String>,
    selected: Option<usize>,
    /// Every selected row.
    selection: Vec<usize>,
    garrison: Option<String>,
    tabs: Vec<(&'static str, [f32; 4])>,
    cells: Vec<[f32; 4]>,
}

/// One open Missions window, for the quadrant gate: its rows (name, side,
/// GOKRES mini), selection, tab, members and target, and the rects of its
/// mission rows, tabs and member rows, as `[left, top, width, height]`.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FixtureMissionsWindow {
    system_dat_id: u32,
    origin: (i16, i16),
    side: u8,
    rows: Vec<(String, u8, u32)>,
    selected: Option<usize>,
    tab: &'static str,
    tab_side: Option<u8>,
    members: Vec<String>,
    member_art: Vec<(Option<u32>, Vec<u32>)>,
    target: Option<String>,
    row_rects: Vec<[f32; 4]>,
    tabs: Vec<(&'static str, [f32; 4])>,
    member_rects: Vec<[f32; 4]>,
}

/// The open System, Defenses and Missions windows and the rail, for the
/// quadrant gate.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct QuadrantObservation {
    status: &'static str,
    code: u32,
    system_windows: Vec<(u32, (i16, i16))>,
    defenses_windows: Vec<FixtureDefensesWindow>,
    missions_windows: Vec<FixtureMissionsWindow>,
    rail: Vec<(u32, &'static str, [f32; 4])>,
}

/// The windows and pages the quadrant observation reads.
pub struct QuadrantWindows<'a> {
    pub systems: &'a SystemWindowState,
    pub defenses: &'a DefensesWindowState,
    pub missions_windows: &'a MissionsWindowState,
    pub fog: &'a FogState,
    pub missions: &'a MissionState,
    pub production: rebellion_render::DefensesProduction<'a>,
    pub economy: &'a EconomyState,
}

fn defenses_page_name(page: DefensesPage) -> &'static str {
    match page {
        DefensesPage::Personnel => "personnel",
        DefensesPage::Regiments => "regiments",
        DefensesPage::Squadrons => "squadrons",
        DefensesPage::Shields => "shields",
        DefensesPage::Batteries => "batteries",
    }
}

fn missions_tab_name(tab: MissionsTab) -> &'static str {
    match tab {
        MissionsTab::Agents => "agents",
        MissionsTab::Decoys => "decoys",
    }
}

fn rect_array(rect: egui_macroquad::egui::Rect) -> [f32; 4] {
    [rect.min.x, rect.min.y, rect.width(), rect.height()]
}

/// Emits the open windows and the rail each time they change.
#[derive(Debug, Default)]
pub struct QuadrantWatch {
    last: Option<QuadrantObservation>,
}

impl QuadrantWatch {
    fn next(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        windows: &QuadrantWindows<'_>,
    ) -> Option<QuadrantObservation> {
        quadrant_systems(request, world)?;
        let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
        let defenses_windows = world
            .systems
            .iter()
            .filter_map(|(system, value)| {
                let report = windows.defenses.report(
                    world,
                    windows.fog,
                    windows.missions,
                    windows.production,
                    windows.economy,
                    system,
                )?;
                Some(FixtureDefensesWindow {
                    system_dat_id: value.dat_id.raw(),
                    origin: report.origin,
                    side: report.side,
                    page: defenses_page_name(report.page),
                    counts: report.counts,
                    cells: (0..report.rows.len())
                        .filter_map(|index| {
                            windows.defenses.cell_screen_rect(layout, system, index)
                        })
                        .map(rect_array)
                        .collect(),
                    rows: report.rows,
                    selected: report.selected,
                    selection: report.selection,
                    garrison: report.garrison,
                    tabs: DefensesPage::ALL
                        .into_iter()
                        .filter_map(|page| {
                            let rect = windows.defenses.tab_screen_rect(layout, system, page)?;
                            Some((defenses_page_name(page), rect_array(rect)))
                        })
                        .collect(),
                })
            })
            .collect();
        let missions_windows = world
            .systems
            .iter()
            .filter_map(|(system, value)| {
                let shown = windows.missions_windows;
                let report = shown.report(world, windows.fog, windows.missions, system)?;
                Some(FixtureMissionsWindow {
                    system_dat_id: value.dat_id.raw(),
                    origin: report.origin,
                    side: report.side,
                    row_rects: (0..report.rows.len())
                        .filter_map(|index| shown.mission_row_screen_rect(layout, system, index))
                        .map(rect_array)
                        .collect(),
                    member_rects: (0..report.members.len())
                        .filter_map(|index| shown.member_row_screen_rect(layout, system, index))
                        .map(rect_array)
                        .collect(),
                    tabs: MissionsTab::ALL
                        .into_iter()
                        .filter_map(|tab| {
                            let rect = shown.tab_screen_rect(layout, system, tab)?;
                            Some((missions_tab_name(tab), rect_array(rect)))
                        })
                        .collect(),
                    rows: report.rows,
                    selected: report.selected,
                    tab: missions_tab_name(report.tab),
                    tab_side: report.tab_side,
                    members: report.members,
                    member_art: report.member_art,
                    target: report.target,
                })
            })
            .collect();
        let now = QuadrantObservation {
            status: "quadrant-observation",
            code: request.code,
            system_windows: windows
                .systems
                .open_windows()
                .filter_map(|(system, at)| Some((world.systems.get(system)?.dat_id.raw(), at)))
                .collect(),
            defenses_windows,
            missions_windows,
            rail: windows
                .systems
                .rail_entries()
                .filter_map(|(index, system, kind)| {
                    let rect = windows.systems.rail_slot_screen_rect(layout, index)?;
                    Some((
                        world.systems.get(system)?.dat_id.raw(),
                        kind,
                        rect_array(rect),
                    ))
                })
                .collect(),
        };
        if self.last.as_ref() == Some(&now) {
            return None;
        }
        self.last = Some(now.clone());
        Some(now)
    }

    pub fn observe(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        windows: &QuadrantWindows<'_>,
    ) {
        let Some(report) = self.next(request, world, windows) else {
            return;
        };
        let bytes = serde_json::to_vec(&report).expect("serialize the quadrant observation");
        unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
    }
}

/// Where the fleet-load gate presses: the fleet icon in the primary system's
/// sector window (and the target's name, for the arrival message), the
/// regiment's cell (none when it starts aboard) and the
/// Fleets tab in the system window, the primary and target planets, and the
/// day readout (the Game Speed control).
#[derive(Debug, Serialize, PartialEq)]
struct FixtureFleetLoadSetup {
    status: &'static str,
    code: u32,
    primary_dat_id: u32,
    target_dat_id: u32,
    target_name: String,
    icon: (f32, f32),
    /// The primary system's facility (System quadrant) icon, when shown.
    system_icon: Option<(f32, f32)>,
    /// The first cell on the Defenses window's page: the regiment, or in
    /// the squadron variant the first squadron.
    troop_item: Option<(f32, f32)>,
    /// The Defenses window's centre: a move released anywhere on it targets
    /// its system (`+0x70`, `FUN_004aa470`).
    defenses_window: (f32, f32),
    primary_planet: (f32, f32),
    target_planet: (f32, f32),
    day_readout: (f32, f32),
    capacity: Option<u32>,
    /// The fleet's room for squadrons (`FUN_00500b40`).
    fighter_room: u32,
}

fn fleet_load_setup(
    request: FixtureRequest,
    world: &GameWorld,
    sectors: &SectorWindowState,
    defenses: &DefensesWindowState,
) -> Option<FixtureFleetLoadSetup> {
    if !request.scenario.uses_the_fleet_window() {
        return None;
    }
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let primary = world.systems.keys().next()?;
    let target = loading_target(world, primary)?;
    let fleet = world.fleets.keys().next()?;
    let subject = defenses_window_subject(request, world, primary)?;
    Some(FixtureFleetLoadSetup {
        status: "fleet-load-setup",
        code: request.code,
        primary_dat_id: world.systems[primary].dat_id.raw(),
        target_dat_id: world.systems[target].dat_id.raw(),
        target_name: world.systems[target].name.clone(),
        icon: screen_center(sectors.fleet_icon_screen_rect(world, layout, primary)?),
        system_icon: sectors
            .quadrant_screen_rect(world, layout, primary, Quadrant::System)
            .map(screen_center),
        troop_item: defenses
            .cell_screen_rect(layout, primary, 0)
            .map(screen_center),
        defenses_window: screen_center(defenses.screen_rect(layout, subject)?),
        primary_planet: screen_center(sectors.planet_screen_rect(world, layout, primary)?),
        target_planet: screen_center(sectors.planet_screen_rect(world, layout, target)?),
        day_readout: day_readout_centre(request.faction, layout),
        capacity: TroopTransportState::fleet_capacity(world, fleet),
        fighter_room: rebellion_core::squadrons::fleet_room(world, fleet),
    })
}

/// The day readout's centre on the 640 by 480 canvas (`day_readout_rect`).
fn day_readout_centre(
    faction: CockpitFaction,
    layout: rebellion_render::CockpitLayout,
) -> (f32, f32) {
    let day = day_readout_rect(faction);
    (
        layout.canvas.x + (day.x + day.width / 2.0) * layout.scale,
        layout.canvas.y + (day.y + day.height / 2.0) * layout.scale,
    )
}

pub fn emit_fleet_load_setup(
    request: FixtureRequest,
    world: &GameWorld,
    sectors: &SectorWindowState,
    defenses: &DefensesWindowState,
) {
    let Some(report) = fleet_load_setup(request, world, sectors, defenses) else {
        return;
    };
    let bytes = serde_json::to_vec(&report).expect("serialize the fleet load setup");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// One fleet the primary system's Fleet window lists: its living capital
/// ships and where its entry lies.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FleetEntryObservation {
    ships: usize,
    entry: (f32, f32),
}

/// What the fleet-load gate checks after each step: the primary system's
/// Fleet window (its list, tabs and contents, and where its first entry,
/// each listed fleet, the Troops tab and the first right-list item lie),
/// where the regiment is and whether it travels on its own, the fleet's
/// cargo and hold, its transit, whether the move confirmation is open, the
/// last message (a refusal's text), and the last regiment line.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FleetLoadObservation {
    status: &'static str,
    code: u32,
    window_open: bool,
    origin: Option<(i16, i16)>,
    entries: usize,
    selected: Option<&'static str>,
    tab: Option<String>,
    enabled: [bool; 4],
    items: Vec<String>,
    /// The right list's selected rows.
    selected_items: Vec<usize>,
    counts: Option<(u32, u32)>,
    fleet_entry: Option<(f32, f32)>,
    fleets: Vec<FleetEntryObservation>,
    troops_tab: Option<(f32, f32)>,
    first_item: Option<(f32, f32)>,
    aboard: bool,
    cargo: usize,
    troop_system_dat_id: Option<u32>,
    /// The system the fleet stands at.
    fleet_system_dat_id: Option<u32>,
    in_transit: bool,
    regiment_travelling: bool,
    confirmation_open: bool,
    last_message: Option<String>,
    last_regiment_message: Option<String>,
    /// The player's squadrons on the primary planet and aboard the fleet.
    planet_squadrons: u32,
    fleet_squadrons: u32,
    fighters_tab: Option<(f32, f32)>,
    /// The other side's squadrons on the primary planet and its fleets
    /// orbiting there (a battle's group of the planet's squadrons).
    enemy_planet_squadrons: u32,
    enemy_fleets: usize,
    /// The other side's regiments on the primary planet, and the last
    /// bombardment line (the bombardment variant).
    enemy_regiments: usize,
    last_bombardment_message: Option<String>,
    /// The last Planetary Assault line (the assault variant).
    last_assault_message: Option<String>,
    /// The Destroy System variant: the primary system destroyed, the
    /// player's fleet still in the world, the characters standing on the
    /// planet that went adrift, and the destruction's line.
    primary_destroyed: bool,
    fleet_present: bool,
    adrift_characters: usize,
    last_destroy_message: Option<String>,
    /// The delivery variant: the departure's Units Deployed title (`0x107`,
    /// `FUN_00497940`, naming the destination), the arrival's Unit Arrival
    /// title (`0x105`, `FUN_004981c0`) and the player's capital ships at the
    /// loading target.
    last_departure_message: Option<String>,
    last_arrival_message: Option<String>,
    target_ships: usize,
    /// The player's last Facility Idle title (`0x160`, `FUN_0048be60`).
    last_idle_title: Option<String>,
    /// The incident variant: the primary system in revolt, the player's
    /// last uprising incident line, and the last disaster line (any
    /// audience: the disaster may strike a neutral system).
    primary_in_revolt: bool,
    last_incident_message: Option<String>,
    last_disaster_message: Option<String>,
    /// The disaster line's title: Natural Disaster (TEXTSTRA `0x7190`) on a
    /// held system, none on a neutral one.
    last_disaster_title: Option<String>,
    /// The player's last Blockade title (`0x14e`, `FUN_004960f0`).
    last_blockade_title: Option<String>,
    /// The player's System Joins title (`0x100`, `FUN_00499760`) for the
    /// loading target.
    last_joins_title: Option<String>,
    /// The top Assault Summary window (`FUN_0044c410`).
    assault_summary: Option<FixtureAssaultSummary>,
}

/// What the top Assault Summary window shows, and where it sits on the 640
/// by 480 canvas.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FixtureAssaultSummary {
    windows: usize,
    origin: (f32, f32),
    title: String,
    body: String,
    page: String,
    category: String,
    picture: u32,
    columns: Vec<Vec<String>>,
}

impl FixtureAssaultSummary {
    #[must_use]
    pub fn of(
        state: &rebellion_render::assault_summary::AssaultSummaryState,
        faction: CockpitFaction,
    ) -> Option<Self> {
        let report = state.report()?;
        let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
        let rect = rebellion_render::assault_summary::window_rect(layout);
        Some(Self {
            windows: state.window_count(),
            origin: (rect.min.x, rect.min.y),
            title: report.title,
            body: report.body,
            page: format!("{:?}", report.page),
            category: format!("{:?}", report.category),
            picture: report.picture,
            columns: report.columns,
        })
    }
}

/// The open Assault Summary of the `EnemyAssaultTaken` and
/// `FleetWindowA0` scenarios (the Empire's assault on Yavin, Fleet 1's
/// bombardment of Kimanan), on the fleet-load channel the fleet-window
/// gate reads.
#[derive(Debug, Default)]
pub struct EnemyAssaultWatch {
    last: Option<Option<FixtureAssaultSummary>>,
}

#[derive(Debug, Serialize)]
struct EnemyAssaultObservation {
    status: &'static str,
    code: u32,
    window_open: bool,
    assault_summary: Option<FixtureAssaultSummary>,
    /// Yavin's holder after the assault (`economy::system_holder`).
    yavin_holder: Option<String>,
}

impl EnemyAssaultWatch {
    pub fn observe(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        assault_summary: Option<FixtureAssaultSummary>,
    ) {
        if !matches!(
            request.scenario,
            Scenario::EnemyAssaultTaken | Scenario::FleetWindowA0
        ) || self.last.as_ref() == Some(&assault_summary)
        {
            return;
        }
        self.last = Some(assault_summary.clone());
        let yavin_holder = world
            .systems
            .values()
            .find(|value| value.name == "Yavin")
            .map(|value| format!("{:?}", rebellion_core::economy::system_holder(world, value)));
        let report = EnemyAssaultObservation {
            status: "fleet-load",
            code: request.code,
            window_open: false,
            assault_summary,
            yavin_holder,
        };
        let bytes = serde_json::to_vec(&report).expect("serialize the enemy assault observation");
        unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
    }
}

#[allow(clippy::too_many_arguments)]
fn fleet_load_observation(
    request: FixtureRequest,
    regiment: TroopKey,
    world: &GameWorld,
    fog: &FogState,
    movement: &MovementState,
    transport: &TroopTransportState,
    fleets: &FleetWindowState,
    confirmation_open: bool,
    messages: &[GameMessage],
) -> Option<FleetLoadObservation> {
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let primary = world.systems.keys().next()?;
    let fleet = world.fleets.keys().next()?;
    let report = fleets.report(world, movement, fog, transport, primary);
    Some(FleetLoadObservation {
        status: "fleet-load",
        code: request.code,
        window_open: report.is_some(),
        origin: report.as_ref().map(|report| report.origin),
        entries: report.as_ref().map_or(0, |report| report.entries),
        selected: report
            .as_ref()
            .and_then(|report| report.selected)
            .map(|entry| match entry {
                FleetWindowEntry::Fleet(_) => "fleet",
                FleetWindowEntry::Ship { .. } => "ship",
            }),
        tab: report.as_ref().map(|report| format!("{:?}", report.tab)),
        enabled: report.as_ref().map_or([false; 4], |report| report.enabled),
        items: report
            .as_ref()
            .map(|report| report.items.clone())
            .unwrap_or_default(),
        selected_items: report
            .as_ref()
            .map(|report| report.selected_items.clone())
            .unwrap_or_default(),
        counts: report.as_ref().and_then(|report| report.counts),
        fleet_entry: fleets
            .entry_screen_rect(layout, primary, 0)
            .map(screen_center),
        fleets: report
            .as_ref()
            .map(|report| {
                report
                    .fleet_rows
                    .iter()
                    .filter_map(|&(listed, row)| {
                        Some(FleetEntryObservation {
                            ships: world
                                .fleets
                                .get(listed)?
                                .capital_ships
                                .iter()
                                .filter(|ship| ship.alive)
                                .count(),
                            entry: screen_center(fleets.entry_screen_rect(layout, primary, row)?),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        troops_tab: fleets
            .tab_screen_rect(layout, primary, FleetWindowTab::Troops)
            .map(screen_center),
        first_item: fleets
            .item_screen_rect(layout, primary, 0)
            .map(screen_center),
        aboard: transport.cargo(fleet).contains(&regiment),
        cargo: transport.carried_count(fleet),
        troop_system_dat_id: regiment_system(world, regiment)
            .and_then(|system| world.systems.get(system))
            .map(|system| system.dat_id.raw()),
        fleet_system_dat_id: world
            .fleets
            .get(fleet)
            .and_then(|value| world.systems.get(value.location))
            .map(|system| system.dat_id.raw()),
        in_transit: movement.is_in_transit(fleet),
        regiment_travelling: transport
            .transits()
            .iter()
            .any(|transit| transit.troop == regiment),
        confirmation_open,
        last_message: messages.last().map(|message| message.text.clone()),
        // One frame can run several days, so a regiment's line may not be
        // the last. Only the player's: the AI's regiments post their own
        // arrivals to the other side's index (Unit Arrival, 0xd).
        last_regiment_message: messages
            .iter()
            .rev()
            .filter(|message| {
                message
                    .audience
                    .includes(request.faction == CockpitFaction::Alliance)
            })
            // FUN_004981c0 case 3 titles it "Units Arrive at {system}";
            // without the texts the port's line stays.
            .find(|message| {
                message.text.starts_with("Regiment ")
                    || message
                        .title
                        .as_deref()
                        .is_some_and(|title| title.starts_with("Units Arrive"))
            })
            .map(|message| message.title.clone().unwrap_or_else(|| message.text.clone())),
        planet_squadrons: rebellion_core::squadrons::on_planet(
            world,
            primary,
            request.faction == CockpitFaction::Alliance,
        ),
        fleet_squadrons: world
            .fleets
            .get(fleet)
            .map_or(0, |value| rebellion_core::squadrons::total(&value.fighters)),
        fighters_tab: fleets
            .tab_screen_rect(layout, primary, FleetWindowTab::Fighters)
            .map(screen_center),
        enemy_planet_squadrons: rebellion_core::squadrons::on_planet(
            world,
            primary,
            request.faction != CockpitFaction::Alliance,
        ),
        enemy_fleets: world.systems[primary]
            .fleets
            .iter()
            .filter(|&&key| {
                world.fleets.get(key).is_some_and(|value| {
                    value.is_alliance != (request.faction == CockpitFaction::Alliance)
                })
            })
            .count(),
        enemy_regiments: world.systems[primary]
            .ground_units
            .iter()
            .filter(|&&key| {
                world.troops.get(key).is_some_and(|troop| {
                    troop.is_alliance != (request.faction == CockpitFaction::Alliance)
                })
            })
            .count(),
        last_bombardment_message: messages
            .iter()
            .rev()
            .find(|message| message.text.contains("bombardment"))
            .map(|message| message.text.clone()),
        last_assault_message: messages
            .iter()
            .rev()
            .find(|message| message.text.contains("assault"))
            .map(|message| message.text.clone()),
        primary_destroyed: world.systems[primary].is_destroyed,
        fleet_present: world.fleets.contains_key(fleet),
        adrift_characters: world
            .characters
            .values()
            .filter(|c| c.adrift || (c.on_mission && c.current_system == Some(primary)))
            .count(),
        last_destroy_message: messages
            .iter()
            .rev()
            // System Destroyed by the Death Star (0x182, FUN_004950d0).
            .find(|message| {
                message
                    .text
                    .starts_with("The Death Star has destroyed the ")
            })
            .map(|message| message.text.clone()),
        // Only the player's: the AI's deliveries post to the other side's
        // index.
        last_departure_message: messages
            .iter()
            .rev()
            .filter(|message| {
                message
                    .audience
                    .includes(request.faction == CockpitFaction::Alliance)
            })
            .filter_map(|message| message.title.as_deref())
            .find(|title| title.contains(" Deployed at "))
            .map(str::to_owned),
        last_arrival_message: messages
            .iter()
            .rev()
            .filter(|message| {
                message
                    .audience
                    .includes(request.faction == CockpitFaction::Alliance)
            })
            .filter_map(|message| message.title.as_deref())
            .find(|title| title.starts_with("Ships arrive at "))
            .map(str::to_owned),
        target_ships: loading_target(world, primary).map_or(0, |target| {
            world.systems[target]
                .fleets
                .iter()
                .filter_map(|&key| world.fleets.get(key))
                .filter(|value| value.is_alliance == (request.faction == CockpitFaction::Alliance))
                .map(|value| value.capital_ships.len())
                .sum()
        }),
        last_idle_title: messages
            .iter()
            .rev()
            .filter(|message| {
                message
                    .audience
                    .includes(request.faction == CockpitFaction::Alliance)
            })
            .filter_map(|message| message.title.as_deref())
            .find(|title| title.contains(" Idle on "))
            .map(str::to_owned),
        primary_in_revolt: matches!(world.systems[primary].control, ControlKind::Uprising(_)),
        last_incident_message: messages
            .iter()
            .rev()
            .filter(|message| {
                message
                    .audience
                    .includes(request.faction == CockpitFaction::Alliance)
            })
            .find(|message| message.text.starts_with("Uprising incident at "))
            .map(|message| message.text.clone()),
        last_disaster_message: last_disaster(messages).map(|message| message.text.clone()),
        last_disaster_title: last_disaster(messages).and_then(|message| message.title.clone()),
        last_blockade_title: messages
            .iter()
            .rev()
            .filter(|message| {
                message
                    .audience
                    .includes(request.faction == CockpitFaction::Alliance)
            })
            .filter_map(|message| message.title.as_deref())
            .find(|title| title.contains("Blockade"))
            .map(str::to_owned),
        last_joins_title: messages
            .iter()
            .rev()
            .filter(|message| {
                message
                    .audience
                    .includes(request.faction == CockpitFaction::Alliance)
            })
            .filter_map(|message| message.title.as_deref())
            .find(|title| {
                loading_target(world, primary).is_some_and(|target| {
                    let name = &world.systems[target].name;
                    title
                        .strip_prefix(name.as_str())
                        .is_some_and(|rest| rest == " Joins Enemy" || rest == " Joins")
                })
            })
            .map(str::to_owned),
        assault_summary: None,
    })
}

/// The last disaster line: Natural Disaster's (`FUN_0048e940`) on a held
/// system, the port's own line on a neutral one.
fn last_disaster(messages: &[GameMessage]) -> Option<&GameMessage> {
    messages.iter().rev().find(|message| {
        message
            .title
            .as_deref()
            .is_some_and(|title| title.starts_with("Natural Disaster on "))
            || message.text.starts_with("Natural disaster strikes ")
    })
}

/// Emits a [`FleetLoadObservation`] whenever it changes. The regiment it
/// follows is the one first on the primary system's surface when the watch
/// starts, or else the first aboard the player's fleet (the refused
/// variant).
#[derive(Debug, Default)]
pub struct FleetLoadWatch {
    regiment: Option<TroopKey>,
    last: Option<FleetLoadObservation>,
}

impl FleetLoadWatch {
    /// The observation when it differs from the last one.
    #[allow(clippy::too_many_arguments)]
    fn next(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        fog: &FogState,
        movement: &MovementState,
        transport: &TroopTransportState,
        fleets: &FleetWindowState,
        confirmation_open: bool,
        messages: &[GameMessage],
        assault_summary: Option<FixtureAssaultSummary>,
        destroy_movie: Option<String>,
    ) -> Option<FleetLoadObservation> {
        if !request.scenario.uses_the_fleet_window() {
            return None;
        }
        if self.regiment.is_none() {
            self.regiment = world
                .systems
                .keys()
                .next()
                .and_then(|primary| loading_regiment(world, primary))
                .or_else(|| {
                    let fleet = world.fleets.keys().next()?;
                    transport.cargo(fleet).first().copied()
                });
        }
        let now = fleet_load_observation(
            request,
            self.regiment?,
            world,
            fog,
            movement,
            transport,
            fleets,
            confirmation_open,
            messages,
        )
        .map(|observation| FleetLoadObservation {
            assault_summary,
            // A story movie is no message (FUN_0048a060 kind 6): the
            // Death Star's movie 101 carries the line as its caption.
            last_destroy_message: observation.last_destroy_message.clone().or(destroy_movie),
            ..observation
        });
        if now.is_none() || now == self.last {
            return None;
        }
        self.last.clone_from(&now);
        now
    }

    #[allow(clippy::too_many_arguments)]
    pub fn observe(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        fog: &FogState,
        movement: &MovementState,
        transport: &TroopTransportState,
        fleets: &FleetWindowState,
        confirmation_open: bool,
        messages: &[GameMessage],
        assault_summary: Option<FixtureAssaultSummary>,
        destroy_movie: Option<String>,
    ) {
        let Some(report) = self.next(
            request,
            world,
            fog,
            movement,
            transport,
            fleets,
            confirmation_open,
            messages,
            assault_summary,
            destroy_movie,
        ) else {
            return;
        };
        let bytes = serde_json::to_vec(&report).expect("serialize the fleet load observation");
        unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
    }
}

/// One Fleet Finder row: its name, what it stands for (a fleet by its place
/// in the world's fleets, and a ship by its index there), the side, and the
/// system the Finder would open.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FinderRowObservation {
    name: String,
    kind: &'static str,
    fleet: Option<usize>,
    ship: Option<usize>,
    is_alliance: bool,
    system_dat_id: Option<u32>,
}

/// Where the Finder's controls lie on screen, at their centers.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FinderControlsObservation {
    origin: (f32, f32),
    tabs: Vec<(f32, f32)>,
    close: (f32, f32),
    display: (f32, f32),
    ship_finder: (f32, f32),
    fleet_finder: (f32, f32),
    name_box: (f32, f32),
    /// Each row's center while the list shows it.
    rows: Vec<Option<(f32, f32)>>,
}

/// An open Fleet window and its selected entry.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FinderFleetWindowObservation {
    system_dat_id: u32,
    selected: Option<&'static str>,
    fleet: Option<usize>,
    ship: Option<usize>,
    /// The window's screen rect, `(x, y, width, height)`, so the gate can
    /// click a Finder control the window covers.
    rect: Option<(f32, f32, f32, f32)>,
}

/// A planet currently exposed by an open sector window. The browser gate uses
/// these exact screen coordinates to prove that a Finder click cannot fall
/// through to the covered original window.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FinderSectorPlanetObservation {
    system_dat_id: u32,
    center: (f32, f32),
}

/// What the fleet-finder gate checks after each step: the Finder (its mode,
/// tab, name box, rows, choice and controls), the cockpit's Fleet Finder
/// control, the open Fleet windows and the systems whose sector window is
/// open, and the galaxy view's selected system.
#[derive(Debug, Clone, Serialize, PartialEq)]
struct FleetFinderObservation {
    status: &'static str,
    code: u32,
    open: bool,
    player_is_alliance: bool,
    mode: Option<&'static str>,
    tab: Option<&'static str>,
    name: Option<String>,
    rows: Vec<FinderRowObservation>,
    chosen: Option<usize>,
    first_row: usize,
    controls: Option<FinderControlsObservation>,
    cockpit_button: (f32, f32),
    fleet_windows: Vec<FinderFleetWindowObservation>,
    sector_open: Vec<u32>,
    sector_planets: Vec<FinderSectorPlanetObservation>,
    selected_system_dat_id: Option<u32>,
    left_panel_open: bool,
}

fn fleet_ordinal(world: &GameWorld, fleet: FleetKey) -> Option<usize> {
    world.fleets.keys().position(|key| key == fleet)
}

fn finder_entry(
    world: &GameWorld,
    entry: FleetWindowEntry,
) -> (&'static str, Option<usize>, Option<usize>) {
    match entry {
        FleetWindowEntry::Fleet(fleet) => ("fleet", fleet_ordinal(world, fleet), None),
        FleetWindowEntry::Ship { fleet, index } => {
            ("ship", fleet_ordinal(world, fleet), Some(index))
        }
    }
}

pub struct FinderWindows<'a> {
    pub fog: &'a FogState,
    pub movement: &'a MovementState,
    pub finder: &'a FleetFinderState,
    pub fleets: &'a FleetWindowState,
    pub sectors: &'a SectorWindowState,
    pub map: &'a GalaxyMapState,
    /// Whether a left panel the galaxy view's letter keys toggle is open.
    pub left_panel_open: bool,
}

fn fleet_finder_observation(
    request: FixtureRequest,
    world: &GameWorld,
    windows: &FinderWindows<'_>,
) -> Option<FleetFinderObservation> {
    // PersonnelStates' galaxy shows no fleet of the other side, for the
    // empty tab.
    if !matches!(
        request.scenario,
        Scenario::FleetFinder | Scenario::PersonnelStates
    ) {
        return None;
    }
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let player = if request.faction == CockpitFaction::Alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    let report = windows
        .finder
        .report(world, windows.movement, windows.fog, player);
    let rows = report.as_ref().map_or_else(Vec::new, |report| {
        report
            .rows
            .iter()
            .map(|row| {
                let (kind, fleet, ship) = finder_entry(world, row.object);
                let value = world.fleets.get(row.object.fleet());
                FinderRowObservation {
                    name: row.name.clone(),
                    kind,
                    fleet,
                    ship,
                    is_alliance: value.is_some_and(|value| value.is_alliance),
                    system_dat_id: value
                        .and_then(|value| world.systems.get(value.location))
                        .map(|system| system.dat_id.raw()),
                }
            })
            .collect()
    });
    let center = |control| {
        windows
            .finder
            .control_screen_rect(layout, control)
            .map(screen_center)
    };
    let controls = windows.finder.screen_rect(layout).and_then(|rect| {
        Some(FinderControlsObservation {
            origin: (rect.min.x, rect.min.y),
            tabs: [FinderTab::All, FinderTab::Alliance, FinderTab::Imperial]
                .into_iter()
                .map(|tab| center(FinderControl::Tab(tab)))
                .collect::<Option<_>>()?,
            close: center(FinderControl::Close)?,
            display: center(FinderControl::Display)?,
            ship_finder: center(FinderControl::ShipFinder)?,
            fleet_finder: center(FinderControl::FleetFinder)?,
            name_box: center(FinderControl::NameBox)?,
            rows: (0..rows.len())
                .map(|row| center(FinderControl::Row(row)))
                .collect(),
        })
    });
    let button = strategic_primary_controls(request.faction)
        .iter()
        .find(|control| control.button == CockpitButton::FleetFinder)?
        .rect;
    let fleet_windows = world
        .systems
        .iter()
        .filter_map(|(key, system)| {
            let (selected, _) = windows.fleets.selection(key)?;
            let (kind, fleet, ship) = selected.map_or((None, None, None), |entry| {
                let (kind, fleet, ship) = finder_entry(world, entry);
                (Some(kind), fleet, ship)
            });
            Some(FinderFleetWindowObservation {
                system_dat_id: system.dat_id.raw(),
                selected: kind,
                fleet,
                ship,
                rect: windows
                    .fleets
                    .screen_rect(layout, key)
                    .map(|rect| (rect.min.x, rect.min.y, rect.width(), rect.height())),
            })
        })
        .collect();
    let sector_planets: Vec<_> = world
        .systems
        .iter()
        .filter_map(|(key, system)| {
            let rect = windows.sectors.planet_screen_rect(world, layout, key)?;
            let center = rect.center();
            Some(FinderSectorPlanetObservation {
                system_dat_id: system.dat_id.raw(),
                center: (center.x, center.y),
            })
        })
        .collect();
    let sector_open = sector_planets
        .iter()
        .map(|planet| planet.system_dat_id)
        .collect();
    Some(FleetFinderObservation {
        status: "fleet-finder",
        code: request.code,
        open: report.is_some(),
        player_is_alliance: player == Faction::Alliance,
        mode: report.as_ref().map(|report| match report.mode {
            FinderMode::Fleets => "fleets",
            FinderMode::Ships => "ships",
        }),
        tab: report.as_ref().map(|report| match report.tab {
            FinderTab::All => "all",
            FinderTab::Alliance => "alliance",
            FinderTab::Imperial => "imperial",
        }),
        name: report.as_ref().map(|report| report.name.clone()),
        rows,
        chosen: report.as_ref().and_then(|report| report.chosen),
        first_row: report.as_ref().map_or(0, |report| report.first_row),
        controls,
        cockpit_button: (
            layout.canvas.x + (button.x + button.width / 2.0) * layout.scale,
            layout.canvas.y + (button.y + button.height / 2.0) * layout.scale,
        ),
        fleet_windows,
        sector_open,
        sector_planets,
        selected_system_dat_id: windows
            .map
            .selected_system
            .and_then(|system| world.systems.get(system))
            .map(|system| system.dat_id.raw()),
        left_panel_open: windows.left_panel_open,
    })
}

/// The System Finder's state, for `system-finder.mjs` (the Fleet Finder
/// scenario's galaxy).
#[derive(Debug, Clone, Serialize, PartialEq)]
struct SystemFinderObservation {
    status: &'static str,
    code: u32,
    open: bool,
    tab: Option<&'static str>,
    name: String,
    rows: Vec<String>,
    chosen: Option<usize>,
    origin: Option<(f32, f32)>,
    tabs: Vec<(f32, f32)>,
    close: Option<(f32, f32)>,
    display: Option<(f32, f32)>,
    row_centres: Vec<Option<(f32, f32)>>,
    /// The cockpit's System Finder control (0x12d).
    button: (f32, f32),
    /// The systems whose sector windows are open.
    sector_open: Vec<String>,
}

fn system_finder_observation(
    request: FixtureRequest,
    world: &GameWorld,
    finder: &rebellion_render::system_finder::SystemFinderState,
    sectors: &SectorWindowState,
) -> Option<SystemFinderObservation> {
    use rebellion_render::system_finder::{SystemFinderControl, SystemTab};
    if request.scenario != Scenario::FleetFinder {
        return None;
    }
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let report = finder.report(world);
    let center = |control| finder.control_screen_rect(layout, control).map(screen_center);
    let rows: Vec<String> = report
        .as_ref()
        .map_or_else(Vec::new, |report| report.rows.iter().map(|row| row.name.clone()).collect());
    let button = strategic_primary_controls(request.faction)
        .iter()
        .find(|control| control.button == CockpitButton::SystemFinder)?
        .rect;
    Some(SystemFinderObservation {
        status: "system-finder",
        code: request.code,
        open: report.is_some(),
        tab: report.as_ref().map(|report| report.tab.label()),
        name: report.as_ref().map_or_else(String::new, |report| report.name.clone()),
        chosen: report.as_ref().and_then(|report| report.chosen),
        origin: finder.screen_rect(layout).map(|rect| (rect.min.x, rect.min.y)),
        tabs: SystemTab::ALL
            .into_iter()
            .filter_map(|tab| center(SystemFinderControl::Tab(tab)))
            .collect(),
        close: center(SystemFinderControl::Close),
        display: center(SystemFinderControl::Display),
        row_centres: (0..rows.len()).map(|row| center(SystemFinderControl::Row(row))).collect(),
        rows,
        button: (button.x + button.width / 2.0, button.y + button.height / 2.0),
        sector_open: world
            .systems
            .iter()
            .filter(|(key, _)| sectors.planet_screen_rect(world, layout, *key).is_some())
            .map(|(_, system)| system.name.clone())
            .collect(),
    })
}

/// Emits the System Finder's observation when it changes.
#[derive(Debug, Default)]
pub struct SystemFinderWatch {
    last: Option<SystemFinderObservation>,
}

impl SystemFinderWatch {
    pub fn observe(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        finder: &rebellion_render::system_finder::SystemFinderState,
        sectors: &SectorWindowState,
    ) {
        let now = system_finder_observation(request, world, finder, sectors);
        if now.is_none() || now == self.last {
            return;
        }
        self.last.clone_from(&now);
        let Some(report) = now else {
            return;
        };
        let bytes = serde_json::to_vec(&report).expect("serialize the system finder observation");
        unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
    }
}

#[derive(Debug, Default)]
pub struct FleetFinderWatch {
    last: Option<FleetFinderObservation>,
}

impl FleetFinderWatch {
    /// The observation when it differs from the last one.
    fn next(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        windows: &FinderWindows<'_>,
    ) -> Option<FleetFinderObservation> {
        let now = fleet_finder_observation(request, world, windows);
        if now.is_none() || now == self.last {
            return None;
        }
        self.last.clone_from(&now);
        now
    }

    pub fn observe(
        &mut self,
        request: FixtureRequest,
        world: &GameWorld,
        windows: &FinderWindows<'_>,
    ) {
        let Some(report) = self.next(request, world, windows) else {
            return;
        };
        let bytes = serde_json::to_vec(&report).expect("serialize the fleet finder observation");
        unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
    }
}

/// Where the open Game Speed menu lies, so the fleet-load gate can unpause
/// the clock: the browser has no keypad `+` (`gl.js`).
#[derive(Debug, Serialize, PartialEq)]
struct FixtureSpeedMenu {
    status: &'static str,
    code: u32,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    rows: usize,
}

/// Emits the speed menu's rect each time it opens or moves.
#[derive(Debug, Default)]
pub struct SpeedMenuWatch {
    last: Option<egui_macroquad::egui::Rect>,
}

impl SpeedMenuWatch {
    /// The report when the menu has opened or moved since the last one.
    fn next(
        &mut self,
        request: FixtureRequest,
        rect: Option<egui_macroquad::egui::Rect>,
    ) -> Option<FixtureSpeedMenu> {
        let watched =
            request.scenario.uses_the_fleet_window() || request.scenario == Scenario::Quadrants;
        if !watched || rect == self.last {
            return None;
        }
        self.last = rect;
        let rect = rect?;
        Some(FixtureSpeedMenu {
            status: "speed-menu",
            code: request.code,
            left: rect.min.x,
            top: rect.min.y,
            width: rect.width(),
            height: rect.height(),
            rows: 5,
        })
    }

    pub fn observe(&mut self, request: FixtureRequest, rect: Option<egui_macroquad::egui::Rect>) {
        let Some(report) = self.next(request, rect) else {
            return;
        };
        let bytes = serde_json::to_vec(&report).expect("serialize the speed menu report");
        unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
    }
}

#[derive(Serialize)]
struct FixtureSelected<'a> {
    status: &'static str,
    code: u32,
    mode: &'a str,
    command_id: u8,
}

pub fn emit_selected(request: FixtureRequest, mode: GidMode) {
    let report = FixtureSelected {
        status: "selected",
        code: request.code,
        mode: mode.label(),
        command_id: mode.command_id(),
    };
    let bytes = serde_json::to_vec(&report).expect("serialize selected GID mode");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

#[derive(Serialize)]
struct FixtureHover<'a> {
    status: &'static str,
    code: u32,
    system_dat_id: u32,
    system_name: &'a str,
}

pub fn emit_hover(request: FixtureRequest, world: &GameWorld, map: &GalaxyMapState) {
    let Some(system) = map.hovered_system.and_then(|key| world.systems.get(key)) else {
        return;
    };
    let report = FixtureHover {
        status: "hovered",
        code: request.code,
        system_dat_id: system.dat_id.raw(),
        system_name: &system.name,
    };
    let bytes = serde_json::to_vec(&report).expect("serialize hovered GID system");
    unsafe { open_rebellion_interface_fixture_emit(bytes.as_ptr(), bytes.len()) };
}

/// Opens the mission dialog for the mission-dialog scenarios, as a drop of the
/// fixture's first character onto its primary system would (`FUN_0042a320`).
/// Returns false for every other scenario, or when no kind is available.
pub fn open_mission_dialog(
    request: FixtureRequest,
    world: &GameWorld,
    uprisings: &UprisingState,
    research: &ResearchState,
    dialog: &mut MissionDialogState,
) -> bool {
    let page = match request.scenario {
        Scenario::MissionDialogMission => MissionDialogPage::Mission,
        Scenario::MissionDialogAgents => MissionDialogPage::Agents,
        _ => return false,
    };
    let (Some(character), Some(target)) =
        (world.characters.keys().next(), world.systems.keys().next())
    else {
        return false;
    };
    let faction = match request.faction {
        CockpitFaction::Alliance => MissionFaction::Alliance,
        CockpitFaction::Empire => MissionFaction::Empire,
    };
    let team = vec![MissionMember::Character(character)];
    let kinds = available_kinds(world, uprisings, research, faction, &team, &[], target);
    if !dialog.open(
        faction,
        target,
        team,
        rebellion_render::mission_dialog::box_order(kinds),
    ) {
        return false;
    }
    if let Some(open) = dialog.dialog_mut() {
        open.show_page(page);
    }
    true
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_render::object_menu::{MenuObject, OrderGates};

    // A0 personnel-states/15, 17, 18 (Alliance, the Troop Finder's
    // Display): Pzob's Defenses window at (118, 86), Mon Calamari's and
    // Umgul's at (305, 86). FUN_0045c8e0 opens at the icon's top left,
    // clamped into the galaxy view.
    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_finders_defenses_window_opens_where_a0_puts_it() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        let world = rebellion_data::load_game_data(&data).expect("load original game data");
        let layout = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
        for (name, a0) in [("Pzob", (118.0, 86.0)), ("Mon Calamari", (305.0, 86.0)), ("Umgul", (305.0, 86.0))] {
            let key = world.systems.iter().find(|(_, s)| s.name == name).map(|(k, _)| k).unwrap();
            let mut sectors = SectorWindowState::default();
            sectors.open_for_system(&world, key, CockpitFaction::Alliance);
            let point = sectors.quadrant_window_point(&world, layout, key, Quadrant::Defenses).unwrap();
            let mut defenses = DefensesWindowState::default();
            defenses.open(&world, key, point, CockpitFaction::Alliance, layout);
            let origin = defenses.screen_rect(layout, key).unwrap().min;
            assert_eq!((origin.x, origin.y), a0, "{name}");
        }
    }

    // A0 status/03-ship: Corellian Corvette 1 in Fleet 1, undamaged. Its
    // first page reads Awaiting Orders, Maintenance Cost 23, no capacity,
    // nothing embarked, Ship Damaged No, Hyperdrive 80:80, Hull 500:500
    // (FUN_00446fd0's rows over CAPSHPSD's Corellian Corvette).
    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn an_undamaged_corellian_corvettes_status_reads_as_a0() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        let mut world = rebellion_data::load_game_data(&data).expect("load original game data");
        let (class, hull) = world
            .capital_ship_classes
            .iter()
            .find(|(_, class)| class.name == "Corellian Corvette")
            .map(|(key, class)| (key, class.hull))
            .expect("CAPSHPSD lists the Corellian Corvette");
        let here = world.systems.keys().next().unwrap();
        let fleet = world.fleets.insert(rebellion_core::world::Fleet {
            location: here,
            capital_ships: vec![ShipInstance::new(class, hull as i32, true)],
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.rename_fleet(fleet, "Fleet 1");
        let missions = MissionState::default();
        let movement = MovementState::default();
        let transport = TroopTransportState::default();
        let manufacturing = ManufacturingState::default();
        let sources = rebellion_render::status_rows::StatusSources {
            world: &world,
            missions: &missions,
            movement: &movement,
            transport: &transport,
            manufacturing: &manufacturing,
            today: 180,
            objectives: rebellion_render::status_rows::ObjectiveSource {
                alliance: true,
                conditions_word: 0,
                standard: true,
            },
        };
        let rows: Vec<(String, String)> = rebellion_render::status_rows::ship_rows(sources, fleet, 0)
            .expect("rows")
            .into_iter()
            .map(|row| (row.label, row.value))
            .take(14)
            .collect();
        let a0: Vec<(String, String)> = [
            ("Class:", "Corellian Corvette"),
            ("Fleet:", "Fleet 1"),
            ("Status:", "Awaiting Orders"),
            ("Maintenance Cost:", "23"),
            ("Capacity:", ""),
            ("Fighter Squadrons:", "0"),
            ("Trooper Regiments:", "0"),
            ("Embarked:", ""),
            ("Fighter Squadrons:", "0"),
            ("Trooper Regiments:", "0"),
            ("Personnel:", "0"),
            ("Ship Damaged:", "No"),
            ("Hyperdrive Rating:", "80:80"),
            ("Hull Value:", "500:500"),
        ]
        .into_iter()
        .map(|(label, value)| (label.to_owned(), value.to_owned()))
        .collect();
        assert_eq!(rows, a0);
    }

    // A0 fleet2/05-medium-transport-1-status (day 714, after the battle at
    // Umgul): Medium Transport 1 in Fleet 1, two regiments aboard, Ship
    // Damaged Yes, Hyperdrive Rating 0:100 (both hyperdrives gone) and
    // Hull Value 317:360 (FUN_00446fd0's rows; ship_condition's
    // FUN_00500f60, FUN_00501780, FUN_00501820).
    #[test]
    #[ignore = "requires original data/base DAT files"]
    fn a_damaged_medium_transports_status_reads_as_a0() {
        use rebellion_core::world::Subsystem;
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        let mut world = rebellion_data::load_game_data(&data).expect("load original game data");
        let class = world
            .capital_ship_classes
            .iter()
            .find(|(_, class)| class.name == "Medium Transport")
            .map(|(key, _)| key)
            .expect("CAPSHPSD lists the Medium Transport");
        let here = world.systems.keys().next().unwrap();
        let mut ship = ShipInstance::new(class, 317, true);
        ship.set_hits(Subsystem::PrimaryHyperdrive, 1);
        ship.set_hits(Subsystem::BackupHyperdrive, 1);
        let fleet = world.fleets.insert(rebellion_core::world::Fleet {
            location: here,
            capital_ships: vec![ship],
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.rename_fleet(fleet, "Fleet 1");
        let regiment_class = world
            .buildable_classes
            .keys()
            .copied()
            .find(|id| id.family() == 0x10)
            .expect("TROOPSD lists a regiment");
        let regiments: Vec<_> = (0..2)
            .map(|_| {
                let key = world.troops.insert(rebellion_core::world::TroopUnit {
                    class_dat_id: regiment_class,
                    is_alliance: true,
                    regiment_strength: 1,
                });
                world.systems[here].ground_units.push(key);
                key
            })
            .collect();
        let mut transport = TroopTransportState::default();
        transport
            .embark(&mut world, fleet, &regiments)
            .expect("the transport carries two regiments");
        let missions = MissionState::default();
        let movement = MovementState::default();
        let manufacturing = ManufacturingState::default();
        let sources = rebellion_render::status_rows::StatusSources {
            world: &world,
            missions: &missions,
            movement: &movement,
            transport: &transport,
            manufacturing: &manufacturing,
            today: 714,
            objectives: rebellion_render::status_rows::ObjectiveSource {
                alliance: true,
                conditions_word: 0,
                standard: true,
            },
        };
        let rows: Vec<(String, String)> = rebellion_render::status_rows::ship_rows(sources, fleet, 0)
            .expect("rows")
            .into_iter()
            .map(|row| (row.label, row.value))
            .take(14)
            .collect();
        let a0: Vec<(String, String)> = [
            ("Class:", "Medium Transport"),
            ("Fleet:", "Fleet 1"),
            ("Status:", "Awaiting Orders"),
            ("Maintenance Cost:", "12"),
            ("Capacity:", ""),
            ("Fighter Squadrons:", "0"),
            ("Trooper Regiments:", "2"),
            ("Embarked:", ""),
            ("Fighter Squadrons:", "0"),
            ("Trooper Regiments:", "2"),
            ("Personnel:", "0"),
            ("Ship Damaged:", "Yes"),
            ("Hyperdrive Rating:", "0:100"),
            ("Hull Value:", "317:360"),
        ]
        .into_iter()
        .map(|(label, value)| (label.to_owned(), value.to_owned()))
        .collect();
        assert_eq!(rows, a0);

        // A0 fleet2/06-corellian-corvette-1-status (day 728): a hull hit
        // alone reads Ship Damaged Yes, Hyperdrive 80:80, Hull 240:500.
        let corvette = world
            .capital_ship_classes
            .iter()
            .find(|(_, class)| class.name == "Corellian Corvette")
            .map(|(key, _)| key)
            .expect("CAPSHPSD lists the Corellian Corvette");
        world.fleets[fleet]
            .capital_ships
            .push(ShipInstance::new(corvette, 240, true));
        let sources = rebellion_render::status_rows::StatusSources {
            world: &world,
            missions: &missions,
            movement: &movement,
            transport: &transport,
            manufacturing: &manufacturing,
            today: 728,
            objectives: rebellion_render::status_rows::ObjectiveSource {
                alliance: true,
                conditions_word: 0,
                standard: true,
            },
        };
        let rows: Vec<(String, String)> = rebellion_render::status_rows::ship_rows(sources, fleet, 1)
            .expect("rows")
            .into_iter()
            .map(|row| (row.label, row.value))
            .skip(11)
            .take(3)
            .collect();
        assert_eq!(
            rows,
            [
                ("Ship Damaged:".to_owned(), "Yes".to_owned()),
                ("Hyperdrive Rating:".to_owned(), "80:80".to_owned()),
                ("Hull Value:".to_owned(), "240:500".to_owned()),
            ]
        );
    }

    #[test]
    fn scenario_table_is_stable_and_complete() {
        for value in 0..SCENARIO_COUNT {
            assert_eq!(Scenario::decode(value).unwrap() as u8, value);
        }
        assert!(Scenario::decode(SCENARIO_COUNT).is_none());
    }

    /// One system held by `side` with one recruited character of that side
    /// on it, and the shipped MISSNSD Diplomacy record `0x51000010`.
    fn diplomacy_world(side: Faction) -> GameWorld {
        use rebellion_core::world::{
            Character, MissionMemberRules, MissionRecord, MissionTargetRules, SkillPair, System,
        };
        let mut world = GameWorld::default();
        let here = world.systems.insert(System {
            last_arrival: None,
            dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
            name: "System".into(),
            sector: rebellion_core::ids::SectorKey::default(),
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
            control: ControlKind::Controlled(side),
        });
        world.characters.insert(Character {
            name: "Agent".into(),
            is_alliance: side == Faction::Alliance,
            is_empire: side == Faction::Empire,
            loyalty: SkillPair {
                base: 100,
                variance: 0,
            },
            current_system: Some(here),
            recruited: true,
            ..Default::default()
        });
        world.mission_records = vec![MissionRecord {
            dat_id: rebellion_core::ids::DatId::new(0x5100_0010),
            timer_min_days: 5,
            timer_spread_days: 10,
            repeats: true,
            hidden: false,
            detection_phases: true,
            can_resign: true,
            // MISSNSD column 10 of the shipped Diplomacy record.
            leave_rule: 1,
            rules: MissionTargetRules {
                container_loss_ends: true,
                explored_container: true,
                needs_populated_container: true,
                target_loss_ends: true,
                own_side_target: true,
                other_side_target: true,
                opponent_target: false,
                revolting_target: false,
                calm_target: true,
                prisoner_target: false,
                free_target: false,
            },
            members: MissionMemberRules {
                alliance: true,
                empire: true,
                special_force_mask: 0,
                character_mask: 0x1_0000,
            },
        }];
        world
    }

    fn request(scenario: Scenario, faction: CockpitFaction) -> FixtureRequest {
        FixtureRequest {
            scenario,
            faction,
            code: 0,
        }
    }

    #[test]
    fn the_agents_scenario_opens_the_dialog_on_its_second_page() {
        let world = diplomacy_world(Faction::Alliance);
        let mut dialog = MissionDialogState::default();

        let opened = open_mission_dialog(
            request(Scenario::MissionDialogAgents, CockpitFaction::Alliance),
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &mut dialog,
        );

        assert!(opened);
        let open = dialog.dialog().expect("the dialog is open");
        assert_eq!(open.page(), MissionDialogPage::Agents);
        assert_eq!(open.kind(), MissionKind::Diplomacy);
        assert_eq!(open.agents().len(), 1);
    }

    #[test]
    fn the_mission_scenario_opens_the_empire_dialog_on_its_first_page() {
        let world = diplomacy_world(Faction::Empire);
        let mut dialog = MissionDialogState::default();

        let opened = open_mission_dialog(
            request(Scenario::MissionDialogMission, CockpitFaction::Empire),
            &world,
            &UprisingState::default(),
            &ResearchState::default(),
            &mut dialog,
        );

        assert!(opened);
        let open = dialog.dialog().expect("the dialog is open");
        assert_eq!(open.page(), MissionDialogPage::Mission);
        let Some(rebellion_render::mission_dialog::MissionDialogAction::Begin { faction, .. }) =
            dialog.begin()
        else {
            panic!("an open dialog begins its mission");
        };
        assert_eq!(faction, MissionFaction::Empire);
    }

    #[test]
    fn other_scenarios_and_refused_teams_open_no_dialog() {
        // FUN_0042a320: with no legal kind nothing opens. An Alliance agent
        // cannot run an Empire mission (FUN_00583320).
        let world = diplomacy_world(Faction::Alliance);
        for request in [
            request(Scenario::Galaxy, CockpitFaction::Alliance),
            request(Scenario::MissionDialogAgents, CockpitFaction::Empire),
        ] {
            let mut dialog = MissionDialogState::default();

            let opened = open_mission_dialog(
                request,
                &world,
                &UprisingState::default(),
                &ResearchState::default(),
                &mut dialog,
            );

            assert!(!opened, "{:?}", request.scenario);
            assert!(!dialog.is_open());
        }
        let mut dialog = MissionDialogState::default();
        assert!(!open_mission_dialog(
            request(Scenario::MissionDialogAgents, CockpitFaction::Alliance),
            &GameWorld::default(),
            &UprisingState::default(),
            &ResearchState::default(),
            &mut dialog,
        ));
    }

    #[test]
    fn the_enemy_character_scenario_frees_the_target_on_the_other_side_in_its_system() {
        // A0 kinds2/50: Veers' Mission released on Leia, free at Yavin.
        let mut world = diplomacy_world(Faction::Alliance);
        let here = world.systems.keys().next().unwrap();
        let mut second = world.systems[here].clone();
        second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
        let there = world.systems.insert(second);
        world.characters.insert(rebellion_core::world::Character {
            name: "Leia".into(),
            current_system: Some(there),
            ..Default::default()
        });
        let request = request(Scenario::EnemyCharacterTargeting, CockpitFaction::Alliance);
        let mut defenses = DefensesWindowState::default();
        apply(
            request,
            &mut world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut CockpitState::new(CockpitFaction::Alliance),
            &mut GalaxyMapState::default(),
            &mut MovementState::default(),
            &mut ManufacturingState::default(),
            &mut EconomyState::default(),
            &mut MissionState::default(),
            &mut BlockadeState::default(),
            &mut SectorWindowState::default(),
            &mut SystemWindowState::default(),
            &mut FleetWindowState::default(),
            &mut defenses,
            &mut TroopTransportState::default(),
            &mut rebellion_core::uprising::UprisingState::new(),
        );
        let target = &world.characters[world.characters.keys().nth(1).unwrap()];
        assert!(!target.is_captive && target.is_empire && !target.is_alliance);
        assert_eq!(target.captured_by, None);
        assert_eq!(target.current_system, Some(there));
        assert!(defenses.is_open(here) && defenses.is_open(there));
    }

    #[test]
    fn the_rescue_scenario_holds_a_prisoner_alone_in_the_enemy_prison_in_sight() {
        // The prisoner carries its captor's side (FUN_004eec60), so it lists
        // first on the prison's personnel page; the menu report aims at
        // that cell.
        let mut world = diplomacy_world(Faction::Alliance);
        let here = world.systems.keys().next().unwrap();
        let mut second = world.systems[here].clone();
        second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
        let prison = world.systems.insert(second);
        let bystander = world.characters.insert(rebellion_core::world::Character {
            name: "Bystander".into(),
            current_system: Some(prison),
            ..Default::default()
        });
        let request = request(Scenario::RescueTargeting, CockpitFaction::Alliance);
        let mut cockpit = CockpitState::new(CockpitFaction::Alliance);
        let mut defenses = DefensesWindowState::default();
        apply(
            request,
            &mut world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut cockpit,
            &mut GalaxyMapState::default(),
            &mut MovementState::default(),
            &mut ManufacturingState::default(),
            &mut EconomyState::default(),
            &mut MissionState::default(),
            &mut BlockadeState::default(),
            &mut SectorWindowState::default(),
            &mut SystemWindowState::default(),
            &mut FleetWindowState::default(),
            &mut defenses,
            &mut TroopTransportState::default(),
            &mut rebellion_core::uprising::UprisingState::new(),
        );

        let prisoner = world.characters.keys().nth(1).unwrap();
        assert_eq!(prisoner, bystander);
        let held = &world.characters[prisoner];
        assert!(held.is_captive && held.is_alliance);
        assert_eq!(held.captured_by, Some(Faction::Empire));
        assert_eq!(held.current_system, Some(prison));
        assert_eq!(
            world.systems[prison].control,
            ControlKind::Controlled(Faction::Empire)
        );
        assert!(defenses.is_open(here) && defenses.is_open(prison));

        let (mut alliance, mut empire) = (
            FogState::new(Faction::Alliance),
            FogState::new(Faction::Empire),
        );
        reveal(request, &world, &mut alliance, &mut empire);
        assert!(alliance.is_visible(prison));

        let layout = cockpit.layout_for(640.0, 480.0);
        let menu = ObjectMenuState::new(
            Some(MenuObject::Character(
                world.characters.keys().next().unwrap(),
            )),
            OrderGates {
                mission: true,
                ..OrderGates::default()
            },
            (0, 0),
        );
        let rect = egui_macroquad::egui::Rect::from_min_size(
            egui_macroquad::egui::Pos2::ZERO,
            egui_macroquad::egui::vec2(10.0, 10.0),
        );
        let report = object_menu_report(
            request,
            (rect, None),
            &menu,
            &world,
            &SectorWindowState::default(),
            (
                &defenses,
                &SystemWindowState::default(),
                &FleetWindowState::default(),
            ),
        )
        .unwrap();
        let cell = defenses
            .cell_screen_rect(layout, prison, 0)
            .unwrap()
            .center();
        assert_eq!(
            (report.target_screen_x, report.target_screen_y),
            (cell.x, cell.y)
        );
    }

    #[test]
    fn a_mission_icons_menu_reports_abort_without_a_target_planet() {
        // The Abort gate's menu has no target: no sector window shows the
        // second system, which the targeting scenarios need.
        let mut world = diplomacy_world(Faction::Alliance);
        let here = world.systems.keys().next().unwrap();
        let second = world.systems[here].clone();
        world.systems.insert(second);
        let rect = egui_macroquad::egui::Rect::from_min_size(
            egui_macroquad::egui::Pos2::ZERO,
            egui_macroquad::egui::vec2(10.0, 10.0),
        );
        let report = |scenario, abort| {
            let menu = ObjectMenuState::new(
                Some(MenuObject::SystemIcon {
                    system: here,
                    quadrant: Quadrant::Missions,
                }),
                OrderGates {
                    abort,
                    ..OrderGates::default()
                },
                (0, 0),
            );
            object_menu_report(
                request(scenario, CockpitFaction::Alliance),
                (rect, None),
                &menu,
                &world,
                &SectorWindowState::default(),
                (
                    &DefensesWindowState::default(),
                    &SystemWindowState::default(),
                    &FleetWindowState::default(),
                ),
            )
            .map(|report| (report.abort_row, report.abort_enabled, report.retire_row))
        };

        assert_eq!(
            report(Scenario::Quadrants, true),
            Some((Some(2), true, None))
        );
        assert_eq!(
            report(Scenario::Quadrants, false),
            Some((Some(2), false, None))
        );
        assert_eq!(report(Scenario::MissionTargeting, true), None);
    }

    #[test]
    fn the_battery_scenario_opens_the_batteries_page_on_the_players_kdy_150() {
        // A0 status/22: Coruscant's Planetary Batteries page lists the
        // KDY-150 first, its menu Encyclopedia, Status, Scrap.
        let request = request(Scenario::BatteryStatus, CockpitFaction::Empire);
        let applied = apply_to_loading_world_with(request, |_| {});
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let first = world.systems[primary].defense_facilities[0];
        assert_eq!(
            world.defense_facilities[first].class_dat_id,
            rebellion_core::ids::DatId::new(0x2200_0001)
        );
        assert_eq!(world.defense_facilities[first].side, Faction::Empire);
        let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
        assert!(applied.defenses.cell_screen_rect(layout, primary, 0).is_some());
    }

    #[test]
    fn the_uprising_scenario_makes_the_target_the_players_and_in_uprising() {
        // A0 kinds2/61: Subdue Uprising is offered on Selonia, the
        // Empire's, in uprising against it.
        let mut world = diplomacy_world(Faction::Empire);
        let here = world.systems.keys().next().unwrap();
        let mut second = world.systems[here].clone();
        second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
        let target = world.systems.insert(second);
        let mut uprisings = rebellion_core::uprising::UprisingState::new();
        apply(
            request(Scenario::UprisingTargeting, CockpitFaction::Empire),
            &mut world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Empire,
            &mut CockpitState::new(CockpitFaction::Empire),
            &mut GalaxyMapState::default(),
            &mut MovementState::default(),
            &mut ManufacturingState::default(),
            &mut EconomyState::default(),
            &mut MissionState::default(),
            &mut BlockadeState::default(),
            &mut SectorWindowState::default(),
            &mut SystemWindowState::default(),
            &mut FleetWindowState::default(),
            &mut DefensesWindowState::default(),
            &mut TroopTransportState::default(),
            &mut uprisings,
        );
        assert_eq!(world.systems[target].control, ControlKind::Uprising(Faction::Empire));
        assert!(uprisings.is_uprising(target));
    }

    #[test]
    fn the_research_scenarios_list_their_own_tree_alone_of_the_trees() {
        // FUN_0056c510: an own-side target with a working yard of the tree's
        // area, and a member with the tree's skill (FUN_0056c820). A0
        // 2026-11-11-mission-kinds/10: Tallon on Averam, one Training
        // Facility, is offered Troop Training Research.
        for (scenario, tree, cockpit_side, side) in [
            (
                Scenario::ResearchTargeting,
                MissionKind::ShipDesignResearch,
                CockpitFaction::Alliance,
                Faction::Alliance,
            ),
            (
                Scenario::ResearchTargeting,
                MissionKind::ShipDesignResearch,
                CockpitFaction::Empire,
                Faction::Empire,
            ),
            (
                Scenario::TroopResearchTargeting,
                MissionKind::TroopTrainingResearch,
                CockpitFaction::Alliance,
                Faction::Alliance,
            ),
            (
                Scenario::TroopResearchTargeting,
                MissionKind::TroopTrainingResearch,
                CockpitFaction::Empire,
                Faction::Empire,
            ),
        ] {
            let mut world = diplomacy_world(side);
            let here = world.systems.keys().next().unwrap();
            let mut second = world.systems[here].clone();
            second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
            let target = world.systems.insert(second);
            let template = world.mission_records[0];
            world.mission_records = [0x5100_0010, 0x5300_0020, 0x5300_0021, 0x5300_0022]
                .into_iter()
                .map(|id| rebellion_core::world::MissionRecord {
                    dat_id: rebellion_core::ids::DatId::new(id),
                    ..template
                })
                .collect();
            let mut missions = MissionState::default();
            apply(
                request(scenario, cockpit_side),
                &mut world,
                &mut GameMode::MainMenu,
                &mut MissionFaction::Alliance,
                &mut CockpitState::new(cockpit_side),
                &mut GalaxyMapState::default(),
                &mut MovementState::default(),
                &mut ManufacturingState::default(),
                &mut EconomyState::default(),
                &mut missions,
                &mut BlockadeState::default(),
                &mut SectorWindowState::default(),
                &mut SystemWindowState::default(),
                &mut FleetWindowState::default(),
                &mut DefensesWindowState::default(),
                &mut TroopTransportState::default(),
                &mut rebellion_core::uprising::UprisingState::new(),
            );
            let agent = MissionMember::Character(world.characters.keys().next().unwrap());
            let faction = if side == Faction::Alliance {
                MissionFaction::Alliance
            } else {
                MissionFaction::Empire
            };

            let kinds = available_kinds(
                &world,
                &UprisingState::default(),
                &ResearchState::default(),
                faction,
                &[agent],
                &[],
                target,
            );

            assert_eq!(world.systems[target].control, ControlKind::Controlled(side));
            let (own, other) = if side == Faction::Alliance {
                (
                    world.systems[target].popularity_alliance,
                    world.systems[target].popularity_empire,
                )
            } else {
                (
                    world.systems[target].popularity_empire,
                    world.systems[target].popularity_alliance,
                )
            };
            assert_eq!((own, other), (1.0, 0.0));
            assert!(world.recruit_pool_empty(side));
            assert_eq!(kinds, vec![tree], "{scenario:?} {side:?}");
        }
    }

    #[test]
    fn the_research_diplomacy_scenario_boxes_research_then_diplomacy_then_espionage() {
        // A0 2026-11-13-mission-dialogs/40..43: Wedge Antilles on his own
        // Drall (a shipyard, support below full) lists Ship Design Research,
        // Diplomacy, Espionage: FUN_0046cab0's keys 4, 7, 9, not MISSNSD's
        // 0x10, 0x13, 0x20 order.
        for (cockpit_side, side, faction) in [
            (
                CockpitFaction::Alliance,
                Faction::Alliance,
                MissionFaction::Alliance,
            ),
            (
                CockpitFaction::Empire,
                Faction::Empire,
                MissionFaction::Empire,
            ),
        ] {
            let mut world = diplomacy_world(side);
            let here = world.systems.keys().next().unwrap();
            let mut second = world.systems[here].clone();
            second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
            let target = world.systems.insert(second);
            let template = world.mission_records[0];
            world.mission_records = [0x5100_0010, 0x5200_0013, 0x5300_0020]
                .into_iter()
                .map(|id| rebellion_core::world::MissionRecord {
                    dat_id: rebellion_core::ids::DatId::new(id),
                    ..template
                })
                .collect();
            let mut missions = MissionState::default();
            apply(
                request(Scenario::ResearchDiplomacyTargeting, cockpit_side),
                &mut world,
                &mut GameMode::MainMenu,
                &mut MissionFaction::Alliance,
                &mut CockpitState::new(cockpit_side),
                &mut GalaxyMapState::default(),
                &mut MovementState::default(),
                &mut ManufacturingState::default(),
                &mut EconomyState::default(),
                &mut missions,
                &mut BlockadeState::default(),
                &mut SectorWindowState::default(),
                &mut SystemWindowState::default(),
                &mut FleetWindowState::default(),
                &mut DefensesWindowState::default(),
                &mut TroopTransportState::default(),
                &mut rebellion_core::uprising::UprisingState::new(),
            );
            let agent = MissionMember::Character(world.characters.keys().next().unwrap());

            let kinds = available_kinds(
                &world,
                &UprisingState::default(),
                &ResearchState::default(),
                faction,
                &[agent],
                &[],
                target,
            );

            assert_eq!(
                rebellion_render::mission_dialog::box_order(kinds),
                [
                    MissionKind::ShipDesignResearch,
                    MissionKind::Diplomacy,
                    MissionKind::Espionage
                ],
                "{side:?}"
            );
        }
    }

    #[test]
    fn the_targeting_scenario_frees_the_agent_and_shows_the_target_planet_clear_of_its_window() {
        let mut world = diplomacy_world(Faction::Alliance);
        let sector = world.sectors.insert(rebellion_core::world::Sector {
            dat_id: rebellion_core::ids::DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: rebellion_core::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: Vec::new(),
        });
        let here = world.systems.keys().next().unwrap();
        world.systems[here].sector = sector;
        let mut second = world.systems[here].clone();
        second.dat_id = rebellion_core::ids::DatId::new(0x9000_0002);
        second.name = "Target".into();
        second.x = 26;
        second.y = 20;
        let target = world.systems.insert(second);
        world.sectors[sector].systems = vec![here, target];
        let agent = world.characters.keys().next().unwrap();
        world.characters[agent].recruited = false;
        world.characters[agent].is_captive = true;
        let request = request(Scenario::MissionTargeting, CockpitFaction::Alliance);
        let mut cockpit = CockpitState::new(CockpitFaction::Alliance);
        let mut missions = MissionState::default();
        let mut sectors = SectorWindowState::default();
        let mut defenses = DefensesWindowState::default();

        apply(
            request,
            &mut world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut cockpit,
            &mut GalaxyMapState::default(),
            &mut MovementState::default(),
            &mut ManufacturingState::default(),
            &mut EconomyState::default(),
            &mut missions,
            &mut BlockadeState::default(),
            &mut sectors,
            &mut SystemWindowState::default(),
            &mut FleetWindowState::default(),
            &mut defenses,
            &mut TroopTransportState::default(),
            &mut rebellion_core::uprising::UprisingState::new(),
        );

        // MissionState::mission_order_enabled enables the menu's Mission.
        assert!(missions.mission_order_enabled(
            &world,
            MissionFaction::Alliance,
            &[MissionMember::Character(agent)],
        ));
        let layout = cockpit.layout_for(640.0, 480.0);
        let galaxy = layout.galaxy;
        // The Defenses window's top-left corner, 5 pixels in from the
        // view's top-right.
        let corner = (
            galaxy.x + galaxy.width - DEFENSES_WINDOW_WIDTH - 5.0,
            galaxy.y + 5.0,
        );
        assert!(defenses.contains_screen_point(layout, corner));
        assert!(!defenses.contains_screen_point(layout, (corner.0 - 1.0, corner.1)));
        assert!(!defenses.contains_screen_point(layout, (corner.0, corner.1 - 1.0)));
        assert_eq!(sectors.window_count(), 1);

        let menu = ObjectMenuState::new(
            Some(MenuObject::Character(agent)),
            OrderGates {
                mission: true,
                ..OrderGates::default()
            },
            (0, 0),
        );
        let rect = egui_macroquad::egui::Rect::from_min_size(
            egui_macroquad::egui::pos2(100.0, 150.0),
            egui_macroquad::egui::vec2(120.0, 142.0),
        );
        let report = object_menu_report(
            request,
            (rect, None),
            &menu,
            &world,
            &sectors,
            (
                &DefensesWindowState::default(),
                &SystemWindowState::default(),
                &FleetWindowState::default(),
            ),
        )
        .unwrap();
        assert_eq!(
            (report.left, report.top, report.width, report.height),
            (100.0, 150.0, 120.0, 142.0)
        );
        assert_eq!((report.rows, report.mission_row), (7, Some(2)));
        assert_eq!(
            (report.target_dat_id, report.target_name),
            (0x9000_0002, "Target")
        );
        // The Alliance's first sector window sits at (60, 35); the target's
        // planet, at sector offset (26, 20), sits at (77, 70) in it
        // (FUN_00459e30 at 0x45a3d4) and is 37 by 37.
        let target = (report.target_screen_x, report.target_screen_y);
        assert_eq!(target, (60.0 + 77.0 + 18.5, 35.0 + 70.0 + 18.5));
        assert!(sectors.contains_screen_point(layout, target));
        assert!(!defenses.contains_screen_point(layout, target));
    }

    /// Three systems in one sector, the second the target in the targeting
    /// layout; three fleets of the other side with no capital ship, two at the
    /// primary system and one at the target; and one capital ship class.
    fn fleet_world() -> GameWorld {
        use rebellion_core::world::{CapitalShipClass, Fleet};
        let mut world = diplomacy_world(Faction::Alliance);
        let sector = world.sectors.insert(rebellion_core::world::Sector {
            dat_id: rebellion_core::ids::DatId::new(0x8000_0001),
            name: "Sector".into(),
            group: rebellion_core::dat::SectorGroup::Core,
            x: 0,
            y: 0,
            systems: Vec::new(),
        });
        let here = world.systems.keys().next().unwrap();
        world.systems[here].sector = sector;
        let mut others = Vec::new();
        for (raw, x) in [(0x9000_0002, 26), (0x9000_0003, 300)] {
            let mut system = world.systems[here].clone();
            system.dat_id = rebellion_core::ids::DatId::new(raw);
            system.x = x;
            system.y = 20;
            others.push(world.systems.insert(system));
        }
        world.sectors[sector].systems = vec![here, others[0], others[1]];
        world.capital_ship_classes.insert(CapitalShipClass {
            hyperdrive: 80,
            ..CapitalShipClass::default()
        });
        for location in [here, here, others[0]] {
            world.fleets.insert(Fleet {
                location,
                capital_ships: Vec::new(),
                fighters: Vec::new(),
                characters: Vec::new(),
                is_alliance: false,
                has_death_star: false,
                special_forces: Vec::new(),
            });
        }
        world
    }

    struct Applied {
        world: GameWorld,
        movement: MovementState,
        blockade: BlockadeState,
        map: GalaxyMapState,
        sectors: SectorWindowState,
        systems: SystemWindowState,
        fleets: FleetWindowState,
        defenses: DefensesWindowState,
        transport: TroopTransportState,
        missions: MissionState,
        manufacturing: ManufacturingState,
    }

    fn apply_to_fleet_world(request: FixtureRequest) -> Applied {
        let mut applied = Applied {
            world: fleet_world(),
            movement: MovementState::default(),
            blockade: BlockadeState::default(),
            map: GalaxyMapState::default(),
            sectors: SectorWindowState::default(),
            systems: SystemWindowState::default(),
            fleets: FleetWindowState::default(),
            defenses: DefensesWindowState::default(),
            transport: TroopTransportState::default(),
            missions: MissionState::default(),
            manufacturing: ManufacturingState::default(),
        };
        apply(
            request,
            &mut applied.world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut CockpitState::new(request.faction),
            &mut applied.map,
            &mut applied.movement,
            &mut applied.manufacturing,
            &mut EconomyState::default(),
            &mut applied.missions,
            &mut applied.blockade,
            &mut applied.sectors,
            &mut applied.systems,
            &mut applied.fleets,
            &mut applied.defenses,
            &mut applied.transport,
            &mut rebellion_core::uprising::UprisingState::new(),
        );
        applied
    }

    /// [`fleet_world`] with its third system moved into a sector of its own,
    /// where the loading scenarios send what they clear away.
    fn apply_to_loading_world(request: FixtureRequest) -> Applied {
        apply_to_loading_world_with(request, |_| {})
    }

    fn apply_to_loading_world_with(
        request: FixtureRequest,
        prepare: impl FnOnce(&mut GameWorld),
    ) -> Applied {
        let mut world = fleet_world();
        let mut keys = world.systems.keys();
        let (here, elsewhere) = (keys.next().unwrap(), keys.nth(1).unwrap());
        let home = world.systems[here].sector;
        let mut far = world.sectors[home].clone();
        far.dat_id = rebellion_core::ids::DatId::new(0x8000_0002);
        far.systems = vec![elsewhere];
        let far = world.sectors.insert(far);
        world.sectors[home].systems.retain(|&key| key != elsewhere);
        world.systems[elsewhere].sector = far;
        prepare(&mut world);
        let mut applied = Applied {
            world,
            movement: MovementState::default(),
            blockade: BlockadeState::default(),
            map: GalaxyMapState::default(),
            sectors: SectorWindowState::default(),
            systems: SystemWindowState::default(),
            fleets: FleetWindowState::default(),
            defenses: DefensesWindowState::default(),
            transport: TroopTransportState::default(),
            missions: MissionState::default(),
            manufacturing: ManufacturingState::default(),
        };
        apply(
            request,
            &mut applied.world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut CockpitState::new(request.faction),
            &mut applied.map,
            &mut applied.movement,
            &mut applied.manufacturing,
            &mut EconomyState::default(),
            &mut applied.missions,
            &mut applied.blockade,
            &mut applied.sectors,
            &mut applied.systems,
            &mut applied.fleets,
            &mut applied.defenses,
            &mut applied.transport,
            &mut rebellion_core::uprising::UprisingState::new(),
        );
        applied
    }

    // port: the fixture's own layout, copying A0 menus/23-fleet1-*.png.
    #[test]
    fn the_a0_kimanan_scenario_lists_fleet_1_loaded_and_fleet_4_en_route() {
        let request = request(Scenario::FleetWindowA0, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Kimanan".into();
            world.systems[elsewhere].sector = world.systems[here].sector;
            for name in ["Corellian Corvette", "Medium Transport"] {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        name: name.into(),
                        is_alliance: true,
                        hull: 100,
                        hyperdrive: 80,
                        troop_capacity: 1,
                        ..Default::default()
                    });
            }
        });
        let world = &applied.world;
        let kimanan = world.systems.keys().next().unwrap();
        let listed = rebellion_core::movement::listed_fleets(&applied.movement, world, kimanan);
        let names: Vec<_> = listed.iter().map(|&fleet| world.fleet_name(fleet).unwrap()).collect();
        assert_eq!(names, ["Fleet 1", "Fleet 4"]);
        let (first, fourth) = (listed[0], listed[1]);
        assert_eq!(world.systems[kimanan].fleets, [first]);
        assert!(applied.movement.is_in_transit(fourth));
        assert_eq!(applied.transport.cargo(first).len(), 2);
        assert_eq!(world.ship_name(first, 1), Some("Medium Transport 1"));
        assert_eq!(world.ship_name(fourth, 0), Some("Corellian Corvette 2"));
        assert!(applied.fleets.is_open(kimanan));
        assert_eq!(
            applied.fleets.screen_rect(CockpitState::new(request.faction).layout_for(640.0, 480.0), kimanan).map(|rect| rect.min),
            Some(egui_macroquad::egui::pos2(305.0, 86.0))
        );
    }

    #[test]
    fn the_enemy_assault_scenario_takes_yavin_from_its_one_regiment_as_a0_shows() {
        // A0 assault-clean/01..03 (an unedited Alliance game, day 720): the
        // Empire took the Alliance's Yavin; Imperial Forces list
        // Stormtrooper Regiments, "No Casualties"; System Forces lost an
        // Alliance Army Regiment, Operational "None". The Alliance reads it
        // from its Conflict rail (FUN_00497530 kind 3).
        use rebellion_core::assault_report::ReportCategory;
        let enemy = request(Scenario::EnemyAssaultTaken, CockpitFaction::Alliance);
        let mut applied = apply_to_loading_world_with(enemy, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Yavin".into();
            world.systems[elsewhere].sector = world.systems[here].sector;
            // GNPRTB as shipped (ghidra/notes/planetary-assault.md):
            // 0x200..0x205, and the garrison 0x1e51 / 0x1e52 that bounds
            // the landing (FUN_0058c810).
            let entry = |id: u32, value: i32| rebellion_core::world::GnprtbEntry {
                parameter_id: id,
                development: value,
                alliance_sp_easy: value,
                alliance_sp_medium: value,
                alliance_sp_hard: value,
                empire_sp_easy: value,
                empire_sp_medium: value,
                empire_sp_hard: value,
                multiplayer: value,
            };
            world.gnprtb = rebellion_core::world::GnprtbParams::new(vec![
                entry(0x200, 5),
                entry(0x201, 30),
                entry(0x202, 10),
                entry(0x203, 4),
                entry(0x204, 6),
                entry(0x205, 8),
                entry(0x1e51, 60),
                entry(0x1e52, -10),
            ]);
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: "Imperial Star Destroyer".into(),
                    is_empire: true,
                    hull: 100,
                    hyperdrive: 80,
                    troop_capacity: 3,
                    ..Default::default()
                });
            for (id, name, alliance) in [
                (0x1000_0007, "Stormtrooper Regiment", false),
                (0x1000_0001, "Alliance Army Regiment", true),
            ] {
                world.buildable_classes.insert(
                    rebellion_core::ids::DatId::new(id),
                    rebellion_core::world::BuildableClass {
                        name: name.into(),
                        is_alliance: alliance,
                        is_empire: !alliance,
                        ..Default::default()
                    },
                );
            }
        });
        let reports = enemy_assault(enemy, &mut applied.world, &mut applied.transport);
        let [report] = &reports[..] else {
            panic!("one report: {reports:?}");
        };
        assert_eq!(report.attacker, Faction::Empire);
        assert_eq!(report.defender, Some(Faction::Alliance));
        assert!(report.taken);
        assert!(!report.opens_at_once(Faction::Alliance));
        let name = |units: &[&rebellion_core::assault_report::ReportUnit]| {
            units
                .iter()
                .map(|unit| unit.name.clone())
                .collect::<Vec<_>>()
        };
        let [operational, _, destroyed] = report.attackers.columns(ReportCategory::Troops);
        assert_eq!(
            name(&operational),
            vec!["Stormtrooper Regiment".to_string(); ENEMY_ASSAULT_REGIMENTS]
        );
        assert!(destroyed.is_empty());
        let [operational, _, destroyed] = report.system_forces.columns(ReportCategory::Troops);
        assert!(operational.is_empty());
        assert_eq!(name(&destroyed), vec!["Alliance Army Regiment".to_string()]);
        // Any other scenario assaults nothing.
        let other = request(Scenario::FleetAssaultTaken, CockpitFaction::Alliance);
        assert!(enemy_assault(other, &mut applied.world, &mut applied.transport).is_empty());
    }

    #[test]
    fn the_a0_coruscant_scenario_loads_fleet_1_as_a0_shows_it() {
        // A0 fleet-aboard/00..12 (a new Empire game, day 10): Fleet 1's one
        // Imperial Star Destroyer carries six TIE Fighter squadrons, three
        // Stormtrooper Regiments and Veers.
        let request = request(Scenario::FleetWindowCoruscant, CockpitFaction::Empire);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Coruscant".into();
            world.systems[elsewhere].sector = world.systems[here].sector;
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: "Imperial Star Destroyer".into(),
                    is_empire: true,
                    hull: 100,
                    hyperdrive: 80,
                    fighter_capacity: 6,
                    troop_capacity: 3,
                    ..Default::default()
                });
            world.fighter_classes.insert(rebellion_core::world::FighterClass {
                name: "TIE Fighter".into(),
                is_empire: true,
                ..Default::default()
            });
            world.buildable_classes.insert(
                rebellion_core::ids::DatId::new(0x1000_0007),
                rebellion_core::world::BuildableClass {
                    name: "Stormtrooper Regiment".into(),
                    is_empire: true,
                    ..Default::default()
                },
            );
            world.characters.insert(rebellion_core::world::Character {
                name: "Veers".into(),
                ..Default::default()
            });
        });
        let world = &applied.world;
        let coruscant = world.systems.keys().next().unwrap();
        let [first] = world.systems[coruscant].fleets[..] else {
            panic!("one fleet at Coruscant: {:?}", world.systems[coruscant].fleets);
        };
        let fleet = &world.fleets[first];
        assert_eq!(world.fleet_name(first), Some("Fleet 1"));
        assert_eq!(world.ship_name(first, 0), Some("Imperial Star Destroyer 1"));
        assert!(!fleet.is_alliance);
        assert_eq!(fleet.fighters.iter().map(|entry| entry.count).sum::<u32>(), 6);
        let [veers] = fleet.characters[..] else {
            panic!("one character aboard");
        };
        // Seated on the numbered destroyer, so its entry shows the
        // personnel mark 10456 as A0's does.
        assert_eq!(world.characters[veers].ship, Some(fleet.capital_ships[0].id));
        assert_ne!(world.characters[veers].ship, Some(0));
        assert_eq!(world.systems[coruscant].ground_units.len(), 0);
        assert!(applied.fleets.is_open(coruscant));
    }

    #[test]
    fn the_a0_vjun_scenario_seats_admiral_bel_iblis_aboard_fleet_5() {
        // A0 fleet2/vjun-fleet5-admiral-personnel: the Admiral order renames
        // him "Admiral Bel Iblis" (FUN_004f17f0, TEXTSTRA name + 0x6c00).
        let request = request(Scenario::FleetWindowVjun, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Vjun".into();
            world.systems[elsewhere].sector = world.systems[here].sector;
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    name: "Corellian Corvette".into(),
                    is_alliance: true,
                    hull: 100,
                    hyperdrive: 80,
                    ..Default::default()
                });
            world.characters.insert(rebellion_core::world::Character {
                name: "Garm Bel Iblis".into(),
                command_names: [
                    String::new(),
                    "Commander Bel Iblis".into(),
                    "Admiral Bel Iblis".into(),
                    "General Bel Iblis".into(),
                ],
                ..Default::default()
            });
        });
        let world = &applied.world;
        let vjun = world.systems.keys().next().unwrap();
        let [fifth] = world.systems[vjun].fleets[..] else {
            panic!("one fleet at Vjun: {:?}", world.systems[vjun].fleets);
        };
        assert_eq!(world.fleet_name(fifth), Some("Fleet 5"));
        assert_eq!(world.ship_name(fifth, 0), Some("Corellian Corvette 2"));
        let [garm] = world.fleets[fifth].characters[..] else {
            panic!("one character aboard");
        };
        let garm = &world.characters[garm];
        assert_eq!(garm.name, "Admiral Bel Iblis");
        assert_eq!(garm.ship, Some(world.fleets[fifth].capital_ships[0].id));
        assert_ne!(garm.ship, Some(0));
        assert!(applied.fleets.is_open(vjun));
        assert!(applied.fleets.is_expanded(vjun, fifth));
        assert_eq!(
            applied.fleets.screen_rect(CockpitState::new(request.faction).layout_for(640.0, 480.0), vjun).map(|rect| rect.min),
            Some(egui_macroquad::egui::pos2(305.0, 86.0))
        );
    }

    #[test]
    fn the_a0_umgul_scenario_lists_fleet_1_damaged_and_fleet_3_loaded() {
        let request = request(Scenario::FleetWindowUmgul, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Umgul".into();
            world.systems[elsewhere].sector = world.systems[here].sector;
            for (name, hull) in [("Corellian Corvette", 500), ("Medium Transport", 360)] {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        name: name.into(),
                        is_alliance: true,
                        hull,
                        hyperdrive: 100,
                        troop_capacity: 2,
                        ..Default::default()
                    });
            }
        });
        let world = &applied.world;
        let umgul = world.systems.keys().next().unwrap();
        let names: Vec<_> = world.systems[umgul]
            .fleets
            .iter()
            .map(|&fleet| world.fleet_name(fleet).unwrap())
            .collect();
        assert_eq!(names, ["Fleet 1", "Fleet 3"]);
        let (first, third) = (world.systems[umgul].fleets[0], world.systems[umgul].fleets[1]);
        let hulls: Vec<_> = world.fleets[first].capital_ships.iter().map(|ship| ship.hull_current).collect();
        assert_eq!(hulls, [240, 317]);
        let transport = &world.fleets[first].capital_ships[1];
        assert_eq!(
            rebellion_core::ship_condition::hyperdrive(&world.capital_ship_classes[transport.class], transport),
            0
        );
        assert_eq!(world.fleets[third].capital_ships[0].hull_current, 359);
        assert_eq!(applied.transport.cargo(first).len(), 2);
        assert_eq!(applied.transport.cargo(third).len(), 2);
        assert!(world.systems[umgul].ground_units.is_empty());
        assert!(applied.fleets.is_expanded(umgul, first));
        assert_eq!(
            applied.fleets.screen_rect(CockpitState::new(request.faction).layout_for(640.0, 480.0), umgul).map(|rect| rect.min),
            Some(egui_macroquad::egui::pos2(305.0, 86.0))
        );
    }

    #[test]
    fn the_a0_selonia_scenario_selects_fleet_4_with_two_squadrons() {
        let request = request(Scenario::FleetWindowSelonia, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Selonia".into();
            world.systems[elsewhere].sector = world.systems[here].sector;
            for (name, fighters) in [("Corellian Corvette", 0), ("Alliance Escort Carrier", 6)] {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        name: name.into(),
                        is_alliance: true,
                        hull: 500,
                        hyperdrive: 100,
                        fighter_capacity: fighters,
                        ..Default::default()
                    });
            }
            for name in ["X-wing", "Y-wing"] {
                world.fighter_classes.insert(rebellion_core::world::FighterClass {
                    name: name.into(),
                    is_alliance: true,
                    ..Default::default()
                });
            }
        });
        let world = &applied.world;
        let selonia = world.systems.keys().next().unwrap();
        let listed = rebellion_core::movement::listed_fleets(&applied.movement, world, selonia);
        let names: Vec<_> = listed.iter().map(|&fleet| world.fleet_name(fleet).unwrap()).collect();
        assert_eq!(names, ["Fleet 1", "Fleet 4"]);
        let (first, fourth) = (listed[0], listed[1]);
        assert!(applied.movement.is_in_transit(first));
        assert_eq!(world.systems[selonia].fleets, [fourth]);
        assert_eq!(world.fleets[first].capital_ships[0].hull_current, 240);
        let squadrons: Vec<_> = world.fleets[fourth]
            .fighters
            .iter()
            .map(|entry| (world.fighter_classes[entry.class].name.as_str(), entry.count))
            .collect();
        assert_eq!(squadrons, [("X-wing", 1), ("Y-wing", 1)]);
        assert_eq!(world.ship_name(fourth, 0), Some("Alliance Escort Carrier 2"));
        assert!(applied.fleets.is_expanded(selonia, first));
        assert!(!applied.fleets.is_expanded(selonia, fourth));
    }

    #[test]
    fn the_a0_yavin_scenario_selects_fleet_1_with_two_ships_and_two_regiments() {
        let request = request(Scenario::FleetWindowYavin, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Yavin".into();
            world.systems[elsewhere].sector = world.systems[here].sector;
            for name in ["Corellian Corvette", "Medium Transport"] {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        name: name.into(),
                        is_alliance: true,
                        hull: 100,
                        hyperdrive: 100,
                        troop_capacity: 2,
                        ..Default::default()
                    });
            }
        });
        let world = &applied.world;
        let yavin = world.systems.keys().next().unwrap();
        let [first] = world.systems[yavin].fleets[..] else {
            panic!("one fleet at Yavin: {:?}", world.systems[yavin].fleets);
        };
        assert_eq!(world.fleet_name(first), Some("Fleet 1"));
        assert_eq!(world.ship_name(first, 1), Some("Medium Transport 1"));
        assert_eq!(applied.transport.cargo(first).len(), 2);
        assert!(world.systems[yavin].ground_units.is_empty());
        assert!(applied.fleets.is_expanded(yavin, first));
    }

    #[test]
    fn the_a0_destination_scenario_lists_fleet_8_en_route_at_qat_chrystac() {
        let request = request(Scenario::FleetWindowDestination, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, there) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Yavin".into();
            world.systems[there].name = "Qat Chrystac".into();
            world.systems[there].sector = world.systems[here].sector;
            for name in ["Corellian Corvette", "Medium Transport"] {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        name: name.into(),
                        is_alliance: true,
                        hull: 100,
                        hyperdrive: 100,
                        troop_capacity: 2,
                        ..Default::default()
                    });
            }
        });
        let world = &applied.world;
        let mut keys = world.systems.keys();
        let (yavin, qat) = (keys.next().unwrap(), keys.next().unwrap());
        let listed = rebellion_core::movement::listed_fleets(&applied.movement, world, qat);
        assert_eq!(listed.len(), 1);
        let fleet = listed[0];
        assert_eq!(world.fleet_name(fleet), Some("Fleet 8"));
        assert!(applied.movement.is_in_transit(fleet));
        assert_eq!(applied.transport.cargo(fleet).len(), 2);
        assert!(applied.fleets.is_open(qat));
        assert!(!applied.fleets.is_open(yavin));
    }

    #[test]
    fn the_a0_blockade_scenario_puts_fleet_8_damaged_over_imperial_bortras() {
        let request = request(Scenario::FleetWindowBlockade, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, there) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Yavin".into();
            world.systems[there].name = "Bortras".into();
            world.systems[there].sector = world.systems[here].sector;
            for name in ["Corellian Corvette", "Medium Transport"] {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        name: name.into(),
                        is_alliance: true,
                        hull: 100,
                        hyperdrive: 100,
                        troop_capacity: 2,
                        ..Default::default()
                    });
            }
        });
        let world = &applied.world;
        let mut keys = world.systems.keys();
        let (_, bortras) = (keys.next().unwrap(), keys.next().unwrap());
        let [fleet] = world.systems[bortras].fleets[..] else {
            panic!("one fleet at Bortras");
        };
        assert_eq!(world.fleet_name(fleet), Some("Fleet 8"));
        assert_eq!(world.systems[bortras].control, ControlKind::Controlled(Faction::Empire));
        assert_eq!(world.fleets[fleet].capital_ships[0].hull_current, 50);
        assert!(!applied.movement.is_in_transit(fleet));
        assert_eq!(applied.transport.cargo(fleet).len(), 2);
        assert!(applied.fleets.is_open(bortras));
        // The player's fleet alone over the other side's system blockades
        // it (A0 fleet4/05: "Bortras is now under blockade by Alliance
        // forces"); the sector-window case shows it so.
        assert!(BlockadeSystem::system_is_blockaded(
            world,
            &world.systems[bortras]
        ));
    }

    #[test]
    fn the_training_scenario_puts_a0s_regiments_part_made_in_the_primarys_training_queue() {
        use rebellion_core::manufacturing::{BuildableKind, ProductionArea};
        for (faction, raw, count, points) in [
            (CockpitFaction::Alliance, 0x1000_0002, 1, 1),
            (CockpitFaction::Empire, 0x1000_0008, 3, 3),
        ] {
            let is_alliance = faction == CockpitFaction::Alliance;
            let mut world = fleet_world();
            let primary = world.systems.keys().next().unwrap();
            for (class, rate) in [(raw, 0), (0x2900_0002, 4)] {
                world.buildable_classes.insert(
                    rebellion_core::ids::DatId::new(class),
                    rebellion_core::world::BuildableClass {
                        refined_material_cost: 6,
                        processing_rate: rate,
                        ..Default::default()
                    },
                );
            }
            let yard = world.manufacturing_facilities.insert(
                rebellion_core::world::ManufacturingFacilityInstance {
                    class_dat_id: rebellion_core::ids::DatId::new(0x2900_0002),
                    side: Faction::of_alliance(is_alliance),
                    is_shipyard: false,
                },
            );
            world.systems[primary].manufacturing_facilities.push(yard);
            let mut manufacturing = ManufacturingState::new();
            place_training(
                request(Scenario::BuildSelectionTraining, faction),
                &world,
                &mut manufacturing,
                primary,
            );
            let queue = manufacturing
                .queue(primary, ProductionArea::TrainingFacility)
                .expect("a training queue");
            assert_eq!(queue.len(), count, "{faction:?}");
            assert_eq!(queue.completed_points(), points, "{faction:?}");
            assert_eq!(
                queue.active().map(|item| (item.kind, item.cost)),
                Some((
                    BuildableKind::Troop(rebellion_core::ids::DatId::new(raw)),
                    6
                ))
            );
        }
    }

    #[test]
    fn the_moddell_scenario_leaves_a0s_four_systems_explored_uninhabited_and_unheld() {
        let mut world = fleet_world();
        let keys: Vec<_> = world.systems.keys().collect();
        world.systems[keys[0]].name = "Agrilat".into();
        world.systems[keys[1]].name = "Annaj".into();
        world.systems[keys[0]].is_populated = true;
        world.systems[keys[0]].control = ControlKind::Controlled(Faction::Alliance);
        world.systems[keys[2]].is_populated = true;
        let other = world.systems[keys[2]].control;
        place_moddell_uninhabited(&mut world);
        for (key, energy, raw) in [(keys[0], 5, 5), (keys[1], 8, 4)] {
            let system = &world.systems[key];
            assert_eq!(system.exploration_status, ExplorationStatus::Explored);
            assert!(!system.is_populated);
            assert_eq!(system.control, ControlKind::Uncontrolled);
            assert_eq!((system.total_energy, system.raw_materials), (energy, raw));
            assert!(
                system.defense_facilities.is_empty() && system.production_facilities.is_empty()
            );
        }
        // A system A0 does not show uninhabited keeps its state.
        assert!(world.systems[keys[2]].is_populated);
        assert_eq!(world.systems[keys[2]].control, other);
    }

    #[test]
    fn the_a0_empty_scenario_opens_umgul_with_no_fleet_there() {
        let request = request(Scenario::FleetWindowEmpty, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            let mut keys = world.systems.keys().collect::<Vec<_>>().into_iter();
            let (here, elsewhere) = (keys.next().unwrap(), keys.next().unwrap());
            world.systems[here].name = "Umgul".into();
            world.systems[elsewhere].sector = world.systems[here].sector;
        });
        let world = &applied.world;
        let umgul = world.systems.keys().next().unwrap();
        assert!(world.systems[umgul].fleets.is_empty());
        assert!(world.fleets.values().all(|fleet| fleet.location != umgul));
        assert!(applied.fleets.is_open(umgul));
        assert_eq!(
            applied.fleets.screen_rect(CockpitState::new(request.faction).layout_for(640.0, 480.0), umgul).map(|rect| rect.min),
            Some(egui_macroquad::egui::pos2(305.0, 86.0))
        );
    }

    // A0 manufacturing/11: the Empire at -402 sees "n/a" and a grey
    // Confirm; FUN_0052e970 refuses what the side's unallocated
    // maintenance cannot take.
    #[test]
    fn the_over_maintenance_scenario_leaves_the_side_no_capacity_and_its_yards() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let side = rebellion_core::dat::Faction::of_alliance(faction == CockpitFaction::Alliance);
            let request = request(Scenario::BuildSelectionOverMaintenance, faction);
            let applied = apply_to_loading_world_with(request, |world| {
                let here = world.systems.keys().next().unwrap();
                let other = rebellion_core::dat::Faction::of_alliance(faction != CockpitFaction::Alliance);
                for (is_mine, holder) in [(true, side), (false, side), (true, other)] {
                    let key = world.production_facilities.insert(
                        rebellion_core::world::ProductionFacilityInstance {
                            class_dat_id: rebellion_core::ids::DatId::new(0x2c00_0001),
                            side: holder,
                            is_mine,
                        },
                    );
                    world.systems[here].production_facilities.push(key);
                }
            });
            let world = &applied.world;
            assert!(world.production_facilities.values().all(|facility| facility.side != side));
            assert_eq!(world.production_facilities.len(), 1);
            assert!(world
                .systems
                .values()
                .flat_map(|system| &system.production_facilities)
                .all(|key| world.production_facilities.contains_key(*key)));
            let totals = rebellion_core::side_resources::maintenance_totals(
                world,
                &ManufacturingState::new(),
                side,
                |_| false,
            );
            assert_eq!(totals.unallocated(), 0);
            let yards = world
                .manufacturing_facilities
                .values()
                .filter(|yard| yard.side == side)
                .count();
            assert!(yards >= 3);
        }
    }

    // A0 build-selection2/21: a band's Destination is released on a planet
    // in a sector window; the gate needs the secondary's planet on screen.
    #[test]
    fn the_destination_scenario_shows_the_secondary_planet_beside_the_open_window() {
        let request = request(Scenario::BuildSelectionDestination, CockpitFaction::Empire);
        let applied = apply_to_loading_world_with(request, |_| {});
        let world = &applied.world;
        let mut keys = world.systems.keys();
        let (primary, secondary) = (keys.next().unwrap(), keys.next().unwrap());
        let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
        let planet = applied.sectors.planet_screen_rect(world, layout, secondary).unwrap();
        let band = applied
            .systems
            .band_screen_rect(layout, primary, rebellion_core::manufacturing::ProductionArea::Shipyard)
            .unwrap();
        // The planet is not under the window.
        assert!(planet.center().x < band.min.x - 55.0, "{planet:?} {band:?}");
        assert!(applied.systems.band_screen_rect(layout, primary, rebellion_core::manufacturing::ProductionArea::TrainingFacility).is_some());
    }

    // FUN_0048bc60: the EventMessages scenario files a battle report on its
    // reader's Conflict rail (A0 messages-day633/index-tab7).
    #[test]
    fn the_event_messages_scenario_files_a_conflict_report_for_its_reader() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let mut log = rebellion_render::MessageLog::new();
            post_conflict_report(request(Scenario::EventMessages, faction), &mut log);
            let [idle, older, newer] = log.messages() else {
                panic!("{:?}", log.messages());
            };
            assert_eq!(idle.title.as_deref(), Some("Construction Yard Idle on Womrik"));
            assert_eq!(idle.rail, Some(rebellion_render::MessageRail::Manufacturing));
            assert_eq!(older.title.as_deref(), Some("Assault on Umgul"));
            assert_eq!(newer.title.as_deref(), Some("Assault on Coruscant"));
            for message in [older, newer] {
                assert_eq!(message.rail, Some(rebellion_render::MessageRail::Conflict));
            }
            for message in [idle, older, newer] {
                assert!(message.unread);
                assert_eq!(
                    message.audience,
                    rebellion_render::message_log::RailAudience::side(
                        faction == CockpitFaction::Alliance
                    )
                );
            }
            let mut none = rebellion_render::MessageLog::new();
            post_conflict_report(request(Scenario::StoryMessages, faction), &mut none);
            assert!(none.messages().is_empty());
        }
    }

    // FUN_004981c0 case 1: A0 events-day673/04 shows the Alliance "Fleet
    // Arrives at Sluis Van" for Fleet 1; the scenario names that pair for the
    // Alliance reader only.
    #[test]
    fn the_event_messages_scenario_brings_fleet_1_to_sluis_van_for_the_alliance() {
        let mut world = fleet_world();
        let sluis = world.systems.keys().nth(1).unwrap();
        world.systems[sluis].name = "Sluis Van".into();
        let alliance = request(Scenario::EventMessages, CockpitFaction::Alliance);
        assert_eq!(arrived_fleet(alliance, &world), None, "no Fleet 1 yet");
        let fleet = world.insert_fleet(rebellion_core::world::Fleet {
            location: sluis,
            capital_ships: Vec::new(),
            fighters: Vec::new(),
            characters: Vec::new(),
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        assert!(world.rename_fleet(fleet, "Fleet 1"));
        assert_eq!(arrived_fleet(alliance, &world), Some((fleet, sluis)));
        assert_eq!(
            arrived_fleet(
                request(Scenario::EventMessages, CockpitFaction::Empire),
                &world
            ),
            None
        );
        assert_eq!(
            arrived_fleet(
                request(Scenario::StoryMessages, CockpitFaction::Alliance),
                &world
            ),
            None
        );
    }

    // FUN_00490340: A0 alliance-game2/han-solo-captured and
    // empire-game3/ozzel-captured show each side's own character captured
    // (0x7158 / 0x7159); the scenario files that for its reader, under the
    // Emperor's capture.
    #[test]
    fn the_event_messages_scenario_files_the_readers_own_capture() {
        let texts: std::collections::HashMap<u16, String> = [
            (0x7158, "{1:1} Captured"),
            (0x7159, "{1:1} was captured by the {3:1} at {2:1}."),
            (0x715a, "{1:1} Captured"),
            (0x715b, "{1:1} was captured at {2:1}."),
            (0x4d50, "Alliance"),
            (0x4d51, "Empire"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        for (faction, name, alliance, text) in [
            (
                CockpitFaction::Alliance,
                "Han Solo",
                true,
                "Han Solo was captured by the Empire at Yavin.",
            ),
            (
                CockpitFaction::Empire,
                "Ozzel",
                false,
                "Ozzel was captured by the Alliance at Tralus.",
            ),
        ] {
            let request = request(Scenario::EventMessages, faction);
            let applied = apply_to_loading_world_with(request, |world| {
                world.characters.insert(rebellion_core::world::Character {
                    name: name.into(),
                    is_alliance: alliance,
                    is_empire: !alliance,
                    ..Default::default()
                });
            });
            let mut log = rebellion_render::MessageLog::new();
            post_own_capture_message(request, &applied.world, &mut log, &texts);
            let [message] = log.messages() else {
                panic!("{:?}", log.messages());
            };
            assert_eq!(message.text, text);
            let mut none = rebellion_render::MessageLog::new();
            post_own_capture_message(
                super::tests::request(Scenario::StoryMessages, faction),
                &applied.world,
                &mut none,
                &texts,
            );
            assert!(none.messages().is_empty());
        }
    }

    // FUN_00499760: A0 message-index/category-1-day228 lists the Alliance's
    // systems joining the Empire; A0 empire-game3/joins-enemy shows the
    // Empire's Commenor joining the Alliance. Each reader gets its own.
    #[test]
    fn the_event_messages_scenario_joins_systems_to_the_readers_enemy() {
        let alliance = request(Scenario::EventMessages, CockpitFaction::Alliance);
        let empire = request(Scenario::EventMessages, CockpitFaction::Empire);
        assert_eq!(
            systems_joining_the_empire(alliance),
            vec![("Corellia", 0), ("Commenor", 0)]
        );
        assert!(systems_joining_the_empire(empire).is_empty());
        assert_eq!(systems_joining_the_alliance(empire), vec![("Commenor", 0)]);
        assert!(systems_joining_the_alliance(alliance).is_empty());
        assert!(systems_joining_the_alliance(request(
            Scenario::StoryMessages,
            CockpitFaction::Empire
        ))
        .is_empty());
    }

    // FUN_0048fdf0: A0 alliance-game2/jan-dodonna-killed shows the
    // Alliance's "Jan Dodonna Killed"; the scenario names him for that
    // reader only.
    #[test]
    fn the_event_messages_scenario_reports_jan_dodonna_killed_for_the_alliance() {
        let mut world = fleet_world();
        let dodonna = world.characters.insert(rebellion_core::world::Character {
            name: "Jan Dodonna".into(),
            is_alliance: true,
            ..Default::default()
        });
        assert_eq!(
            killed_character(
                request(Scenario::EventMessages, CockpitFaction::Alliance),
                &world
            ),
            Some(dodonna)
        );
        // The Empire's reader takes Jerjerrod, whom this world lacks.
        assert_eq!(
            killed_character(
                request(Scenario::EventMessages, CockpitFaction::Empire),
                &world
            ),
            None
        );
        let jerjerrod = world.characters.insert(rebellion_core::world::Character {
            name: "Jerjerrod".into(),
            is_empire: true,
            ..Default::default()
        });
        assert_eq!(
            killed_character(
                request(Scenario::EventMessages, CockpitFaction::Empire),
                &world
            ),
            Some(jerjerrod)
        );
        assert_eq!(
            killed_character(
                request(Scenario::StoryMessages, CockpitFaction::Alliance),
                &world
            ),
            None
        );
    }

    // FUN_00499130: A0 alliance-game2/smuggling-benefits shows the
    // Alliance's "Smuggling Benefits" from Yavin; the scenario names Yavin
    // for that reader only.
    #[test]
    fn the_event_messages_scenario_smuggles_from_yavin_for_the_alliance() {
        let mut world = fleet_world();
        let yavin = world.systems.keys().nth(1).unwrap();
        world.systems[yavin].name = "Yavin".into();
        assert_eq!(
            smuggled_system(
                request(Scenario::EventMessages, CockpitFaction::Alliance),
                &world
            ),
            Some(yavin)
        );
        // The Empire's reader takes Coruscant, which this world lacks.
        assert_eq!(
            smuggled_system(
                request(Scenario::EventMessages, CockpitFaction::Empire),
                &world
            ),
            None
        );
        assert_eq!(
            smuggled_system(
                request(Scenario::StoryMessages, CockpitFaction::Alliance),
                &world
            ),
            None
        );
    }

    // FUN_004960f0: A0 events-day673/02 shows the Alliance "Yavin Under
    // Blockade"; the scenario blockades Yavin for that reader only.
    #[test]
    fn the_event_messages_scenario_blockades_yavin_for_the_alliance() {
        let mut world = fleet_world();
        let yavin = world.systems.keys().nth(1).unwrap();
        world.systems[yavin].name = "Yavin".into();
        assert_eq!(
            blockaded_system(request(Scenario::EventMessages, CockpitFaction::Alliance), &world),
            Some(yavin)
        );
        assert_eq!(
            blockaded_system(request(Scenario::EventMessages, CockpitFaction::Empire), &world),
            None
        );
        assert_eq!(
            blockaded_system(request(Scenario::StoryMessages, CockpitFaction::Alliance), &world),
            None
        );
    }

    // FUN_0048f1e0 (timer 0x389): A0 research-display/08-future-jedi shows
    // the Empire Vader's "Future Jedi Discovered" for Ozzel, and
    // future-jedi-alliance/01 the Alliance Luke's for Jan Dodonna.
    #[test]
    fn the_event_messages_scenario_has_each_sides_trainer_find_a_force_user() {
        assert_eq!(
            force_discoveries(request(Scenario::EventMessages, CockpitFaction::Empire)),
            vec![("Vader", "Ozzel")]
        );
        assert_eq!(
            force_discoveries(request(Scenario::EventMessages, CockpitFaction::Alliance)),
            vec![("Luke", "Dodonna")]
        );
        assert!(
            force_discoveries(request(Scenario::StoryMessages, CockpitFaction::Empire)).is_empty()
        );
    }

    // FUN_00490340: the EventMessages scenario files the Emperor's capture
    // at Corfai for its reader (A0 messages-day633, read by the Empire).
    #[test]
    fn the_event_messages_scenario_files_the_emperors_capture() {
        let texts: std::collections::HashMap<u16, String> = [
            (0x7158, "{1:1} Captured"),
            (0x7159, "{1:1} was captured by the {3:1} at {2:1}."),
            (0x715a, "{1:1} Captured"),
            (0x715b, "{1:1} was captured at {2:1}."),
            (0x4d50, "Alliance"),
        ]
        .into_iter()
        .map(|(id, text)| (id, text.to_owned()))
        .collect();
        let request = request(Scenario::EventMessages, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            world.characters.insert(rebellion_core::world::Character {
                name: "Emperor Palpatine".into(),
                dat_id: rebellion_core::ids::DatId::new(0x3000_0280),
                is_major: true,
                is_empire: true,
                ..Default::default()
            });
        });
        let mut log = rebellion_render::MessageLog::new();
        post_capture_message(request, &applied.world, &mut log, &texts);
        let [message] = log.messages() else {
            panic!("{:?}", log.messages());
        };
        assert_eq!(message.title.as_deref(), Some("Emperor Palpatine Captured"));
        // The Alliance reads the enemy's captive by case 4 (A0
        // empire-clean/31-needa-captured gives the own side case 3).
        assert_eq!(message.text, "Emperor Palpatine was captured at Corfai.");
        let mut none = rebellion_render::MessageLog::new();
        post_capture_message(
            super::tests::request(Scenario::StoryMessages, CockpitFaction::Alliance),
            &applied.world,
            &mut none,
            &texts,
        );
        assert!(none.messages().is_empty());
    }

    // FUN_00458fe0: a built facility (`+0x50` bit 2 set, bit 4 clear) takes
    // its type's base whatever its side; FUN_00539f50's records 4, 5 and 6
    // have 0x2332, 0x233a and 0x2342 (9010, 9018, 9026).
    #[test]
    fn the_advanced_bases_scenario_shows_each_built_advanced_base_first_on_its_page() {
        use rebellion_render::manufacturing_window::{page_cells, FacilityPage};
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let applied =
                apply_to_loading_world_with(request(Scenario::AdvancedBases, faction), |_| {});
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let pictures = |page| -> Vec<u32> {
                page_cells(
                    world,
                    &ManufacturingState::default(),
                    &rebellion_core::delivery::DeliveryState::default(),
                    true,
                    primary,
                    page,
                )
                .iter()
                .map(|cell| cell.picture)
                .collect()
            };
            assert_eq!(pictures(FacilityPage::Shipyards), [9_010], "{faction:?}");
            assert_eq!(
                pictures(FacilityPage::TrainingFacilities),
                [9_018],
                "{faction:?}"
            );
            assert_eq!(
                pictures(FacilityPage::ConstructionYards),
                [9_026],
                "{faction:?}"
            );
            // Only the Advanced Shipyard builds ships.
            let shipyards: Vec<bool> = world.systems[primary]
                .manufacturing_facilities
                .iter()
                .map(|&key| world.manufacturing_facilities[key].is_shipyard)
                .collect();
            assert_eq!(shipyards, [true, false, false], "{faction:?}");
        }
    }

    // FUN_00458fe0: an object en route takes its base + 3; the Build
    // Selection scenario sends a shipyard, a mine and a refinery to the
    // primary (A0 personnel-states/34, 9009; build-selection4/26, 9004;
    // build-selection4/29, 9033).
    #[test]
    fn the_build_selection_scenario_sends_a_shipyard_a_mine_and_a_refinery_to_the_primary() {
        use rebellion_render::manufacturing_window::{page_cells, FacilityPage};
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let request = request(Scenario::BuildSelection, faction);
            let applied = apply_to_loading_world_with(request, |_| {});
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let mut deliveries = rebellion_core::delivery::DeliveryState::default();
            place_deliveries(request, world, &mut deliveries);
            let [shipyard, mine, refinery] = deliveries.en_route() else {
                panic!("{:?}", deliveries.en_route());
            };
            for delivery in [shipyard, mine, refinery] {
                assert_eq!(delivery.destination, primary);
                assert!(delivery.arrival_tick > 100_000);
            }
            let cells = |page| {
                page_cells(
                    world,
                    &ManufacturingState::default(),
                    &deliveries,
                    true,
                    primary,
                    page,
                )
            };
            assert_eq!(cells(FacilityPage::Shipyards).last().map(|cell| cell.picture), Some(9_009));
            // The mine en route follows the built mine, before the empty
            // deposits (A0 build-selection4/26: Yavin's two built mines,
            // then the one en route, then the deposits).
            let mines: Vec<u32> = cells(FacilityPage::Mines).iter().map(|cell| cell.picture).collect();
            assert_eq!(mines[..3], [9_001, 9_004, 9_005], "{mines:?}");
            // A0 build-selection4/29: Yavin's refinery en route, 9033.
            let refineries: Vec<u32> =
                cells(FacilityPage::Refineries).iter().map(|cell| cell.picture).collect();
            assert_eq!(refineries, [9_030, 9_033], "{refineries:?}");
            let mut none = rebellion_core::delivery::DeliveryState::default();
            place_deliveries(
                super::tests::request(Scenario::BuildSelectionDestination, faction),
                world,
                &mut none,
            );
            assert!(none.en_route().is_empty());
        }
    }

    // build-selection.mjs reads every facility class's values at the
    // primary at number 1 (A0 build-selection4, build-selection-empire2),
    // so the primary keeps a free energy slot and raw deposit for each
    // (FUN_00507750, the room FUN_005385f0's order needs) with the three
    // products sent to it on their way.
    #[test]
    fn the_build_selection_scenarios_primary_has_room_for_one_more_facility_of_each_class() {
        use rebellion_core::manufacturing::{BuildableKind, FacilityBuild};
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let request = request(Scenario::BuildSelection, faction);
            let applied = apply_to_loading_world_with(request, |_| {});
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let mut deliveries = rebellion_core::delivery::DeliveryState::default();
            place_deliveries(request, world, &mut deliveries);
            let is_alliance = faction == CockpitFaction::Alliance;
            // A defense, a yard of each kind, a mine and a refinery.
            let classes = [0x2200_0001, 0x2800_0001, 0x2900_0005, 0x2a00_0001, 0x2c00_0001, 0x2d00_0002]
                .map(|raw| {
                    let class = rebellion_core::ids::DatId::new(raw);
                    let build = FacilityBuild { class, is_alliance };
                    match class.family() {
                        0x22 => BuildableKind::DefenseFacility(build),
                        0x2c | 0x2d => BuildableKind::ProductionFacility(build),
                        _ => BuildableKind::ManufacturingFacility(build),
                    }
                });
            for kind in classes {
                assert!(
                    !rebellion_core::build_selection::lacks_facility_room(
                        world,
                        &applied.manufacturing,
                        deliveries.en_route(),
                        primary,
                        kind,
                        1,
                    ),
                    "{faction:?} {kind:?}"
                );
            }
        }
    }

    // FUN_00509710: the other side's squadrons make it present beside the
    // player's fleet, so a battle is due.
    #[test]
    fn the_squadron_battle_scenario_puts_two_enemy_squadrons_under_the_fleet() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let alliance = faction == CockpitFaction::Alliance;
            let request = request(Scenario::SquadronBattle, faction);
            let applied = apply_to_loading_world_with(request, |world| {
                for side in [true, false] {
                    world
                        .fighter_classes
                        .insert(rebellion_core::world::FighterClass {
                            is_alliance: side,
                            is_empire: !side,
                            ..Default::default()
                        });
                }
            });
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            assert_eq!(
                rebellion_core::squadrons::on_planet(world, primary, !alliance),
                2
            );
            assert_eq!(
                rebellion_core::squadrons::on_planet(world, primary, alliance),
                0
            );
            assert_eq!(
                rebellion_core::squadrons::sides_present(world, primary),
                (true, true)
            );
        }
    }

    // port: the fixture's own layout. A product one point from its cost
    // completes on the next yard cycle (FUN_0052b960) and departs for its
    // destination (FUN_0052bee0).
    #[test]
    fn the_delivery_scenario_queues_a_ship_one_point_short_bound_for_the_target() {
        let request = request(Scenario::ProductionDelivery, CockpitFaction::Alliance);
        let applied = apply_to_loading_world_with(request, |world| {
            world
                .capital_ship_classes
                .insert(rebellion_core::world::CapitalShipClass {
                    is_alliance: true,
                    hull: 300,
                    hyperdrive: 80,
                    troop_capacity: 2,
                    refined_material_cost: 6,
                    ..Default::default()
                });
        });
        let world = &applied.world;
        let primary = world.systems.keys().next().unwrap();
        let target = loading_target(world, primary).unwrap();
        let mut manufacturing = ManufacturingState::default();
        place_delivery(request, world, &mut manufacturing, primary);
        let queue = manufacturing
            .queue(primary, rebellion_core::manufacturing::ProductionArea::Shipyard)
            .unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.completed_points(), 5);
        assert_eq!(queue.active().unwrap().destination, Some(target));
        assert!(runs_the_clock(request));
        assert!(world.systems[primary]
            .manufacturing_facilities
            .iter()
            .any(|&key| world.manufacturing_facilities[key].is_shipyard));
    }

    // port: the fixture's own layout. A revolt's incident timer (event
    // 0x38d, FUN_0050d030) and the disaster timer (event 0x38f,
    // FUN_00511930) fire when due; the disaster spares a system with
    // neither energy nor raw materials.
    // port: the fixture's own layout. FUN_0050b610 gives a system with no
    // regiment the side its support favours, and System Joins
    // (FUN_00499760) reports the change.
    #[test]
    fn the_support_flip_scenario_gives_the_target_s_support_to_the_other_side() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let alliance = faction == CockpitFaction::Alliance;
            let request = request(Scenario::SupportFlip, faction);
            let mut applied = apply_to_loading_world_with(request, |_| {});
            let primary = applied.world.systems.keys().next().unwrap();
            let target = loading_target(&applied.world, primary).unwrap();
            let target_name = applied.world.systems[target].name.clone();
            let system = &applied.world.systems[target];
            assert_eq!(system.popularity_alliance, if alliance { 0.0 } else { 1.0 });
            assert_eq!(system.popularity_empire, if alliance { 1.0 } else { 0.0 });
            assert!(system.ground_units.is_empty());
            assert!(runs_the_clock(request));
            let line = |title: &str, player: bool| {
                GameMessage::new(
                    1,
                    "text".to_string(),
                    rebellion_render::message_log::MessageCategory::Diplomacy,
                )
                .with_title(title.to_string())
                .on_rail(
                    rebellion_render::message_log::MessageRail::PopularSupport,
                    rebellion_render::message_log::RailAudience::side(alliance == player),
                )
            };
            let observed = fleet_load_observation(
                request,
                loading_regiment(&applied.world, primary).unwrap(),
                &applied.world,
                &FogState::new(Faction::of_alliance(alliance)),
                &applied.movement,
                &applied.transport,
                &FleetWindowState::default(),
                false,
                &[
                    line(&format!("{target_name} Joins Enemy"), true),
                    line("Elsewhere Joins Enemy", true),
                    line(&format!("{target_name} Joins"), false),
                ],
            )
            .unwrap();
            assert_eq!(
                observed.last_joins_title,
                Some(format!("{target_name} Joins Enemy"))
            );
            // A system joining the player's own side reads "Joins".
            let observed = fleet_load_observation(
                request,
                loading_regiment(&applied.world, primary).unwrap(),
                &applied.world,
                &FogState::new(Faction::of_alliance(alliance)),
                &applied.movement,
                &applied.transport,
                &FleetWindowState::default(),
                false,
                &[line(&format!("{target_name} Joins"), true)],
            )
            .unwrap();
            assert_eq!(
                observed.last_joins_title,
                Some(format!("{target_name} Joins"))
            );
        }
    }

    // port: the fixture's own layout. A fleet arriving at the other side's
    // system blockades it (blockade::system_is_blockaded); Blockade
    // (FUN_004960f0) then speaks to the player.
    #[test]
    fn the_blockade_scenario_sends_the_fleet_to_a_system_the_other_side_holds() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let alliance = faction == CockpitFaction::Alliance;
            let request = request(Scenario::BlockadeArrival, faction);
            let mut applied = apply_to_loading_world_with(request, |_| {});
            let primary = applied.world.systems.keys().next().unwrap();
            let target = loading_target(&applied.world, primary).unwrap();
            let fleet = applied.world.fleets.keys().next().unwrap();
            place_blockade_arrival(request, &mut applied.world, &mut applied.movement, primary);
            let enemy = if alliance {
                Faction::Empire
            } else {
                Faction::Alliance
            };
            let system = &applied.world.systems[target];
            assert_eq!(system.control, ControlKind::Controlled(enemy));
            assert_eq!(system.popularity_alliance, if alliance { 0.0 } else { 1.0 });
            assert_eq!(system.popularity_empire, if alliance { 1.0 } else { 0.0 });
            assert_eq!(
                applied.movement.get(fleet).map(|order| order.destination),
                Some(target)
            );
            assert!(runs_the_clock(request));

            // The report keeps the player's Blockade title only.
            let line = |title: &str, player: bool| {
                GameMessage::new(
                    1,
                    "text".to_string(),
                    rebellion_render::message_log::MessageCategory::Combat,
                )
                .with_title(title.to_string())
                .on_rail(
                    rebellion_render::message_log::MessageRail::Conflict,
                    rebellion_render::message_log::RailAudience::side(alliance == player),
                )
            };
            let observed = fleet_load_observation(
                request,
                loading_regiment(&applied.world, primary).unwrap(),
                &applied.world,
                &FogState::new(Faction::of_alliance(alliance)),
                &applied.movement,
                &applied.transport,
                &FleetWindowState::default(),
                false,
                &[
                    line("Fleet Initiates Blockade of Mine", true),
                    line("Theirs Under Blockade", false),
                ],
            )
            .unwrap();
            assert_eq!(
                observed.last_blockade_title.as_deref(),
                Some("Fleet Initiates Blockade of Mine")
            );
        }
    }

    #[test]
    fn the_incident_scenario_revolts_the_primary_system_with_both_timers_due_soon() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let request = request(Scenario::UprisingIncident, faction);
            let mut applied = apply_to_loading_world_with(request, |_| {});
            let world = &mut applied.world;
            let primary = world.systems.keys().next().unwrap();
            let empty = world.systems.keys().nth(1).unwrap();
            world.systems[empty].total_energy = 0;
            world.systems[empty].raw_materials = 0;
            let mined = world.systems.keys().nth(2).unwrap();
            world.systems[mined].total_energy = 0;
            world.systems[mined].raw_materials = 3;
            let mut uprisings = rebellion_core::uprising::UprisingState::default();
            place_incidents(request, world, &mut uprisings, primary);
            let side = if faction == CockpitFaction::Alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            };
            assert_eq!(world.systems[primary].control, ControlKind::Uprising(side));
            let uprising = &uprisings.active_uprisings[&primary];
            assert_eq!(uprising.next_incident_tick, Some(5));
            assert_eq!(uprising.side, Some(side));
            assert_eq!(uprisings.next_disaster_tick, Some(8));
            assert!(world
                .systems
                .values()
                .all(|system| system.total_energy > 0 || system.raw_materials > 0));
            assert_eq!(world.systems[empty].total_energy, 1);
            assert_eq!(world.systems[mined].total_energy, 0);
            assert!(runs_the_clock(request));

            // The report keeps the player's incident line only, and any
            // disaster line.
            let alliance = faction == CockpitFaction::Alliance;
            let line = |text: &str, player: bool| {
                GameMessage::new(
                    1,
                    text.to_string(),
                    rebellion_render::message_log::MessageCategory::Diplomacy,
                )
                .on_rail(
                    rebellion_render::message_log::MessageRail::PopularSupport,
                    rebellion_render::message_log::RailAudience::side(alliance == player),
                )
            };
            let observed = fleet_load_observation(
                request,
                loading_regiment(world, primary).unwrap(),
                world,
                &FogState::new(Faction::of_alliance(alliance)),
                &applied.movement,
                &applied.transport,
                &FleetWindowState::default(),
                false,
                &[
                    line("Uprising incident at Mine", true),
                    line("Uprising incident at Theirs", false),
                    line("Natural disaster strikes There", false),
                    line("There has been a natural disaster on Held.", false)
                        .with_title("Natural Disaster on Held".to_string()),
                ],
            )
            .unwrap();
            assert!(observed.primary_in_revolt);
            assert_eq!(
                observed.last_incident_message.as_deref(),
                Some("Uprising incident at Mine")
            );
            assert_eq!(
                observed.last_disaster_message.as_deref(),
                Some("There has been a natural disaster on Held.")
            );
            assert_eq!(
                observed.last_disaster_title.as_deref(),
                Some("Natural Disaster on Held")
            );
        }
    }

    // port: the fixture's own layout. Destroy System needs a Death Star in
    // the fleet (FUN_00582440, 8/8 without one); CAPSHPSD 136 is the
    // Empire's only.
    #[test]
    fn the_destroy_system_scenario_gives_only_the_empire_s_fleet_a_death_star() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let alliance = faction == CockpitFaction::Alliance;
            let request = request(Scenario::FleetDestroySystem, faction);
            let applied = apply_to_loading_world_with(request, |world| {
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        is_alliance: alliance,
                        hull: 300,
                        hyperdrive: 80,
                        troop_capacity: 2,
                        bombardment_modifier: 5,
                        ..Default::default()
                    });
                world
                    .capital_ship_classes
                    .insert(rebellion_core::world::CapitalShipClass {
                        dat_id: rebellion_core::ids::DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
                        is_alliance: false,
                        is_empire: true,
                        hull: 25_000,
                        hyperdrive: 80,
                        bombardment_modifier: 15,
                        ..Default::default()
                    });
                world.characters.insert(rebellion_core::world::Character {
                    is_alliance: !alliance,
                    is_empire: alliance,
                    ..Default::default()
                });
            });
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let fleet = world.fleets.keys().next().unwrap();
            assert_eq!(world.fleets[fleet].has_death_star, !alliance);
            assert_eq!(
                rebellion_core::bombardment::order_enabled(
                    world,
                    &applied.movement,
                    alliance,
                    &[fleet],
                    primary,
                    rebellion_core::bombardment::DESTROY_SYSTEM,
                ),
                !alliance
            );
            assert!(world.characters.values().any(|c| c.is_alliance != alliance
                && rebellion_core::death_star::stands_in(c, primary)));
        }
    }

    // port: the fixture's own layout; the order's gate is
    // bombardment::order_enabled (FUN_00553770, FUN_00582440).
    #[test]
    fn the_bombardment_scenario_lets_the_fleet_bombard_two_enemy_regiments() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let alliance = faction == CockpitFaction::Alliance;
            let request = request(Scenario::FleetBombardment, faction);
            let applied = apply_to_loading_world_with(request, |world| {
                // Before each side's class that fits, one without room for
                // regiments (the gate's setup needs it), one that does not
                // bombard and one without a hyperdrive; the other side's
                // fitting class comes first.
                let other = !alliance;
                for (side, room, bombard, hyperdrive) in [
                    (other, 2, 5, 80),
                    (alliance, 0, 5, 80),
                    (alliance, 2, 0, 80),
                    (alliance, 2, 5, 0),
                    (alliance, 2, 5, 80),
                ] {
                    world
                        .capital_ship_classes
                        .insert(rebellion_core::world::CapitalShipClass {
                            is_alliance: side,
                            hull: 300,
                            hyperdrive,
                            troop_capacity: room,
                            bombardment_modifier: bombard,
                            ..Default::default()
                        });
                }
            });
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let fleet = world.fleets.keys().next().unwrap();
            let system = &world.systems[primary];
            assert_eq!(
                GameWorld::facility_holder(system.control),
                Some(Faction::of_alliance(!alliance))
            );
            assert_eq!(system.defense_facilities.len(), 0);
            assert_eq!(system.ground_units.len(), 2);
            assert!(system
                .ground_units
                .iter()
                .all(|&key| world.troops[key].is_alliance != alliance));
            assert_eq!(world.fleets[fleet].capital_ships.len(), 1);
            assert_eq!(world.fleets[fleet].capital_ships[0].hull_current, 300);
            let class = &world.capital_ship_classes[world.fleets[fleet].capital_ships[0].class];
            assert_eq!(
                (
                    class.is_alliance,
                    class.troop_capacity,
                    class.bombardment_modifier,
                    class.hyperdrive
                ),
                (alliance, 2, 5, 80)
            );
            assert!(rebellion_core::bombardment::fleet_bombardment(world, fleet) > 0);
            assert!(rebellion_core::bombardment::order_enabled(
                world,
                &applied.movement,
                alliance,
                &[fleet],
                primary,
                rebellion_core::bombardment::GENERAL,
            ));
            // port: the fixture's own report counts the other side's
            // regiments and keeps the last bombardment line.
            let line = GameMessage::new(
                1,
                "Orbital bombardment at X".to_string(),
                rebellion_render::message_log::MessageCategory::Combat,
            );
            let observed = fleet_load_observation(
                request,
                loading_regiment(world, primary).unwrap(),
                world,
                &FogState::new(Faction::of_alliance(alliance)),
                &applied.movement,
                &applied.transport,
                &FleetWindowState::default(),
                false,
                &[line],
            )
            .unwrap();
            assert_eq!(observed.enemy_regiments, 2);
            assert_eq!(
                observed.last_bombardment_message.as_deref(),
                Some("Orbital bombardment at X")
            );
        }
    }

    // port: the fixture's own layout; the order's gate is
    // assault::order_enabled (FUN_00581ae0, FUN_00553770).
    #[test]
    fn the_assault_scenario_lets_the_fleet_assault_with_a_regiment_aboard() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let alliance = faction == CockpitFaction::Alliance;
            let assault = |scenario| {
                let applied = apply_to_loading_world_with(request(scenario, faction), |world| {
                    world
                        .capital_ship_classes
                        .insert(rebellion_core::world::CapitalShipClass {
                            is_alliance: alliance,
                            hull: 300,
                            hyperdrive: 80,
                            troop_capacity: 2,
                            bombardment_modifier: 5,
                            ..Default::default()
                        });
                });
                let world = &applied.world;
                let primary = world.systems.keys().next().unwrap();
                let fleet = world.fleets.keys().next().unwrap();
                let aboard = applied.transport.cargo(fleet).to_vec();
                let enabled = rebellion_core::assault::order_enabled(
                    world,
                    &applied.movement,
                    &applied.transport,
                    alliance,
                    &[fleet],
                    primary,
                );
                (applied, aboard, enabled)
            };
            let (applied, aboard, enabled) = assault(Scenario::FleetAssault);
            assert_eq!(aboard.len(), 1);
            assert_eq!(applied.world.troops[aboard[0]].is_alliance, alliance);
            assert!(enabled);
            // The taken variant leaves the planet no regiment of the other
            // side's, so the assault takes it (0x7091).
            // It carries six, as A0 messages-day633's Alliance Forces.
            let (taken, aboard, enabled) = assault(Scenario::FleetAssaultTaken);
            assert_eq!(aboard.len(), 6);
            assert!(enabled);
            let target = taken.world.systems.keys().next().unwrap();
            assert!(taken.world.systems[target]
                .ground_units
                .iter()
                .all(|key| taken.world.troops[*key].is_alliance == alliance));
            // The bombardment variant carries none, so the order is off.
            let (_, aboard, enabled) = assault(Scenario::FleetBombardment);
            assert!(aboard.is_empty());
            assert!(!enabled);
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let line = GameMessage::new(
                1,
                "Planetary assault at X".to_string(),
                rebellion_render::message_log::MessageCategory::Combat,
            );
            let observed = fleet_load_observation(
                request(Scenario::FleetAssault, faction),
                loading_regiment(world, primary).unwrap(),
                world,
                &FogState::new(Faction::of_alliance(alliance)),
                &applied.movement,
                &applied.transport,
                &FleetWindowState::default(),
                false,
                &[line],
            )
            .unwrap();
            assert_eq!(observed.enemy_regiments, 2);
            assert_eq!(
                observed.last_assault_message.as_deref(),
                Some("Planetary assault at X")
            );
            assert_eq!(observed.last_bombardment_message, None);
        }
    }

    // port: the fixture's own layout; the room follows FUN_00500b40.
    #[test]
    fn the_squadron_load_scenario_puts_two_squadrons_beside_a_fighter_carrier() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let alliance = faction == CockpitFaction::Alliance;
            let request = request(Scenario::SquadronLoad, faction);
            let applied = apply_to_loading_world_with(request, |world| {
                for side in [true, false] {
                    world
                        .fighter_classes
                        .insert(rebellion_core::world::FighterClass {
                            is_alliance: side,
                            is_empire: !side,
                            ..Default::default()
                        });
                }
            });
            let world = &applied.world;
            let primary = world.systems.keys().next().unwrap();
            let fleet = world.fleets.keys().next().unwrap();
            assert_eq!(
                rebellion_core::squadrons::on_planet(world, primary, alliance),
                2
            );
            assert_eq!(
                rebellion_core::squadrons::on_planet(world, primary, !alliance),
                0
            );
            assert!(rebellion_core::squadrons::fleet_room(world, fleet) >= 1);
            assert!(world.fleets[fleet].fighters.is_empty());
            let setup =
                fleet_load_setup(request, world, &applied.sectors, &applied.defenses).unwrap();
            assert_eq!(
                setup.fighter_room,
                rebellion_core::squadrons::fleet_room(world, fleet)
            );
        }
    }

    // port: the fixture's own layout; the load itself follows FUN_00500b40.
    #[test]
    fn the_fleet_load_scenario_leaves_one_carrier_and_one_movable_regiment_at_the_players_system() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let request = request(Scenario::FleetLoad, faction);
            let mut applied = apply_to_loading_world(request);
            let world = &applied.world;
            let mut keys = world.systems.keys();
            let (here, target) = (keys.next().unwrap(), keys.next().unwrap());
            let fleet = world.fleets.keys().next().unwrap();
            let is_alliance = faction == CockpitFaction::Alliance;
            let player = if is_alliance {
                Faction::Alliance
            } else {
                Faction::Empire
            };

            assert_eq!(world.systems[here].fleets, [fleet], "{faction:?}");
            assert!(world.systems[target].fleets.is_empty(), "{faction:?}");
            assert_eq!(world.fleets[fleet].is_alliance, is_alliance);
            let [ship] = world.fleets[fleet].capital_ships.as_slice() else {
                panic!("{faction:?}: the fleet holds one capital ship");
            };
            let class = &world.capital_ship_classes[ship.class];
            assert!(
                class.troop_capacity > 0 && class.hyperdrive > 0,
                "{faction:?}"
            );
            assert_eq!(
                world.systems[target].control,
                ControlKind::Controlled(player)
            );
            assert!(world.systems[target].ground_units.is_empty());
            let regiment = loading_regiment(world, here).unwrap();
            assert_eq!(world.systems[here].ground_units, [regiment]);
            assert_eq!(world.troops[regiment].is_alliance, is_alliance);
            assert!(applied.transport.is_empty());
            assert!(applied
                .transport
                .regiment_move_enabled(world, regiment, is_alliance));

            let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
            let setup =
                fleet_load_setup(request, world, &applied.sectors, &applied.defenses).unwrap();
            assert_eq!(setup.target_dat_id, world.systems[target].dat_id.raw());
            assert_eq!(setup.target_name, world.systems[target].name);
            assert_eq!(setup.capacity, Some(class.troop_capacity));
            // The Defenses window lies on the galaxy view's left, the
            // primary system's sector window in the right column.
            assert!(applied
                .defenses
                .contains_screen_point(layout, setup.troop_item.unwrap()));
            assert!(!applied.defenses.contains_screen_point(layout, setup.icon));
            assert!(applied.sectors.contains_screen_point(layout, setup.icon));
            assert!(applied
                .sectors
                .contains_screen_point(layout, setup.target_planet));

            applied
                .transport
                .embark(&mut applied.world, fleet, &[regiment])
                .unwrap();
        }
    }

    #[test]
    fn the_destination_scenario_adds_a_player_shipyard_and_shows_its_facility_icon() {
        // port: the fixture's own layout for the Destination gate
        // (ghidra/notes/production-destination.md).
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let player_shipyards = |scenario| {
                let applied = apply_to_loading_world(request(scenario, faction));
                let world = &applied.world;
                let primary = world.systems.keys().next().unwrap();
                let setup = fleet_load_setup(
                    request(scenario, faction),
                    world,
                    &applied.sectors,
                    &applied.defenses,
                )
                .unwrap();
                let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
                if let Some(icon) = setup.system_icon {
                    assert!(
                        applied.sectors.contains_screen_point(layout, icon),
                        "{faction:?}"
                    );
                }
                let shipyards = world.systems[primary]
                    .manufacturing_facilities
                    .iter()
                    .filter(|key| {
                        let facility = &world.manufacturing_facilities[**key];
                        facility.is_shipyard
                            && (facility.side == rebellion_core::dat::Faction::Alliance)
                                == (faction == CockpitFaction::Alliance)
                    })
                    .count();
                (shipyards, setup.system_icon.is_some())
            };
            let (before, _) = player_shipyards(Scenario::FleetLoad);
            assert_eq!(
                player_shipyards(Scenario::ProductionDestination),
                (before + 1, true),
                "{faction:?}"
            );
        }
    }

    #[test]
    fn the_joining_scenario_puts_a_player_fleet_of_one_ship_beside_one_of_two() {
        // port: the fixture's own layout for the join gate
        // (ghidra/notes/fleet-join-split.md).
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let applied = apply_to_loading_world(request(Scenario::FleetJoin, faction));
            let world = &applied.world;
            let here = world.systems.keys().next().unwrap();
            let first = world.fleets.keys().next().unwrap();
            let is_alliance = faction == CockpitFaction::Alliance;
            let [a, b] = world.systems[here].fleets.as_slice() else {
                panic!("{faction:?}: two fleets orbit the primary system");
            };
            assert_eq!(*a, first, "{faction:?}");
            for (fleet, ships) in [(*a, 2), (*b, 1)] {
                let value = &world.fleets[fleet];
                assert_eq!(value.capital_ships.len(), ships, "{faction:?}");
                assert!(value.capital_ships.iter().all(|ship| ship.alive));
                assert_eq!(value.is_alliance, is_alliance);
                assert!(!value.has_death_star && value.characters.is_empty());
                assert!(!applied.movement.is_in_transit(fleet));
            }
        }
    }

    #[test]
    fn the_en_route_joining_scenario_sends_the_one_ship_fleet_onto_the_other() {
        // port: the fixture's own layout for the en route join case.
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let applied = apply_to_loading_world(request(Scenario::FleetJoinEnroute, faction));
            let world = &applied.world;
            let here = world.systems.keys().next().unwrap();
            let first = world.fleets.keys().next().unwrap();
            assert_eq!(world.systems[here].fleets, [first], "{faction:?}");
            let order = applied
                .movement
                .orders()
                .values()
                .find(|order| order.join == Some(first))
                .expect("a fleet on its way to join the first");
            assert_eq!(order.destination, here);
            assert_ne!(world.fleets[order.fleet].location, here);
            assert_eq!(world.fleets[order.fleet].capital_ships.len(), 1);
            assert_eq!(world.fleets[first].capital_ships.len(), 2);
            assert!(rebellion_core::fleet_join::joins_on_arrival(
                world,
                &applied.movement,
                first,
                here,
                faction == CockpitFaction::Alliance
            ));
        }
    }

    #[test]
    fn the_fleet_load_observation_lists_each_fleet_with_its_ships_and_entry() {
        // port: the fixture's own report.
        let request = request(Scenario::FleetJoin, CockpitFaction::Alliance);
        let applied = apply_to_loading_world(request);
        let here = applied.world.systems.keys().next().unwrap();
        let regiment = loading_regiment(&applied.world, here).unwrap();
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(here);
        let layout = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
        let mut fleets = FleetWindowState::default();
        fleets.open(
            &applied.world,
            here,
            (300, 80),
            CockpitFaction::Alliance,
            layout,
        );
        let observe = |confirmation_open| {
            fleet_load_observation(
                request,
                regiment,
                &applied.world,
                &fog,
                &applied.movement,
                &applied.transport,
                &fleets,
                confirmation_open,
                &[],
            )
            .unwrap()
        };
        let row = |row| screen_center(fleets.entry_screen_rect(layout, here, row).unwrap());

        let observed = observe(false);
        assert_eq!(observed.entries, 2);
        assert_eq!(
            observed.fleets,
            [
                FleetEntryObservation {
                    ships: 2,
                    entry: row(0)
                },
                FleetEntryObservation {
                    ships: 1,
                    entry: row(1)
                },
            ]
        );
        assert_ne!(row(0), row(1));
        assert!(!observed.confirmation_open);
        assert!(observe(true).confirmation_open);
    }

    #[test]
    fn the_fleet_load_scenario_picks_a_carrier_of_the_players_side_that_can_jump() {
        // port: the fixture's own choice of a regiment carrier.
        use rebellion_core::world::CapitalShipClass;
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let is_alliance = faction == CockpitFaction::Alliance;
            let class = |is_alliance, troop_capacity, hyperdrive| CapitalShipClass {
                is_alliance,
                troop_capacity,
                hyperdrive,
                hull: 100,
                ..CapitalShipClass::default()
            };
            let mut carrier = None;
            let applied =
                apply_to_loading_world_with(request(Scenario::FleetLoad, faction), |world| {
                    world.capital_ship_classes.insert(class(is_alliance, 0, 80));
                    world.capital_ship_classes.insert(class(is_alliance, 2, 0));
                    world
                        .capital_ship_classes
                        .insert(class(!is_alliance, 2, 80));
                    carrier = Some(world.capital_ship_classes.insert(class(is_alliance, 3, 80)));
                });
            let fleet = applied.world.fleets.keys().next().unwrap();
            assert_eq!(
                applied.world.fleets[fleet].capital_ships[0].class,
                carrier.unwrap(),
                "{faction:?}"
            );
        }
    }

    #[test]
    fn without_a_carrier_the_fleet_load_scenario_builds_one_for_the_players_side() {
        // port: the fixture's fallback class.
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let applied = apply_to_loading_world(request(Scenario::FleetLoad, faction));
            let fleet = applied.world.fleets.keys().next().unwrap();
            let class = &applied.world.capital_ship_classes
                [applied.world.fleets[fleet].capital_ships[0].class];
            assert_eq!(class.name, "Transport");
            assert_eq!(class.is_alliance, faction == CockpitFaction::Alliance);
            assert_eq!(
                (class.hull, class.hyperdrive, class.troop_capacity),
                (100, 80, 2)
            );
        }
    }

    #[test]
    fn the_refused_unload_scenario_starts_with_the_regiment_held_aboard_and_the_target_in_the_system_window(
    ) {
        // port: the fixture's own layout; FUN_0053d430 refuses the drop
        // (other side, populated).
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let request = request(Scenario::RegimentUnloadRefused, faction);
            let applied = apply_to_loading_world(request);
            let world = &applied.world;
            let mut keys = world.systems.keys();
            let (here, target) = (keys.next().unwrap(), keys.next().unwrap());
            let fleet = world.fleets.keys().next().unwrap();
            let is_alliance = faction == CockpitFaction::Alliance;
            let enemy = if is_alliance {
                Faction::Empire
            } else {
                Faction::Alliance
            };

            assert_eq!(
                world.systems[target].control,
                ControlKind::Controlled(enemy),
                "{faction:?}"
            );
            assert!(world.systems[target].is_populated);
            assert!(world.systems[here].ground_units.is_empty());
            let [regiment] = applied.transport.cargo(fleet) else {
                panic!("{faction:?}: one regiment starts aboard");
            };
            assert_eq!(world.troops[*regiment].is_alliance, is_alliance);
            assert_eq!(applied.defenses.window_count(), 1);
            assert!(applied.defenses.is_open(target));

            let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
            let setup =
                fleet_load_setup(request, world, &applied.sectors, &applied.defenses).unwrap();
            assert_eq!(setup.troop_item, None);
            assert!(applied
                .defenses
                .contains_screen_point(layout, setup.defenses_window));

            let mut watch = FleetLoadWatch::default();
            let start = watch
                .next(
                    request,
                    world,
                    &FogState::new(enemy),
                    &applied.movement,
                    &applied.transport,
                    &FleetWindowState::default(),
                    false,
                    &[],
                    None,
                    None,
                )
                .unwrap();
            assert!(start.aboard, "{faction:?}");
            assert_eq!((start.cargo, start.troop_system_dat_id), (1, None));
        }
    }

    #[test]
    fn the_fleet_load_scenario_leaves_fleets_elsewhere_where_they_are() {
        // port: only fleets at the loading and target systems are moved.
        let mut kept = None;
        let applied = apply_to_loading_world_with(
            request(Scenario::FleetLoad, CockpitFaction::Alliance),
            |world| {
                let copy = world.systems.values().nth(2).unwrap().clone();
                let fourth = world.systems.insert(copy);
                let fleet = world
                    .fleets
                    .insert(world.fleets.values().next().unwrap().clone());
                world.fleets[fleet].location = fourth;
                kept = Some((fleet, fourth));
            },
        );
        let (fleet, fourth) = kept.unwrap();
        assert_eq!(applied.world.fleets[fleet].location, fourth);
    }

    #[test]
    fn the_fleet_load_scenarios_regiment_is_of_the_players_troop_class() {
        // port: the fixture copies the class of a regiment of the player's side.
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let is_alliance = faction == CockpitFaction::Alliance;
            let applied =
                apply_to_loading_world_with(request(Scenario::FleetLoad, faction), |world| {
                    for (side, raw) in [(!is_alliance, 0x1000_0010), (is_alliance, 0x1000_0020)] {
                        world.troops.insert(TroopUnit {
                            class_dat_id: rebellion_core::ids::DatId::new(raw),
                            is_alliance: side,
                            regiment_strength: 100,
                        });
                    }
                });
            let here = applied.world.systems.keys().next().unwrap();
            let regiment = loading_regiment(&applied.world, here).unwrap();
            assert_eq!(
                applied.world.troops[regiment].class_dat_id.raw(),
                0x1000_0020,
                "{faction:?}"
            );
        }
    }

    #[test]
    fn the_fleet_load_scenarios_defenses_window_sits_five_pixels_into_the_galaxy_view() {
        // port: the fixture's own placement.
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let applied = apply_to_loading_world(request(Scenario::FleetLoad, faction));
            let galaxy = CockpitState::new(faction).layout_for(640.0, 480.0).galaxy;
            let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
            let inside = |x: f32, y: f32| applied.defenses.contains_screen_point(layout, (x, y));
            let (x, y) = (galaxy.x + 5.0, galaxy.y + 5.0);
            assert!(inside(x, y), "{faction:?}");
            assert!(!inside(x - 0.5, y) && !inside(x, y - 0.5), "{faction:?}");
        }
    }

    #[test]
    fn the_fleet_load_watch_reports_only_its_own_regiment_as_travelling() {
        // port: the fixture's own report of TroopTransportState::transits.
        use rebellion_core::troop_transport::RegimentTarget;
        let request = request(Scenario::FleetLoad, CockpitFaction::Alliance);
        let mut applied = apply_to_loading_world(request);
        let mut keys = applied.world.systems.keys();
        let (here, target) = (keys.next().unwrap(), keys.next().unwrap());
        let regiment = loading_regiment(&applied.world, here).unwrap();
        let other = applied
            .world
            .troops
            .insert(applied.world.troops[regiment].clone());
        applied.world.systems[here].ground_units.push(other);
        let fog = FogState::new(Faction::Alliance);
        let fleets = FleetWindowState::default();
        let observe = |applied: &Applied| {
            fleet_load_observation(
                request,
                regiment,
                &applied.world,
                &fog,
                &applied.movement,
                &applied.transport,
                &fleets,
                false,
                &[],
            )
            .unwrap()
        };

        let blockaded = std::collections::HashSet::new();
        applied
            .transport
            .move_regiment(
                &mut applied.world,
                &applied.movement,
                &blockaded,
                other,
                RegimentTarget::System(target),
                0,
            )
            .unwrap();
        assert!(!observe(&applied).regiment_travelling);

        applied
            .transport
            .move_regiment(
                &mut applied.world,
                &applied.movement,
                &blockaded,
                regiment,
                RegimentTarget::System(target),
                0,
            )
            .unwrap();
        let travelling = observe(&applied);
        assert!(travelling.regiment_travelling);
        assert_eq!(travelling.troop_system_dat_id, None);

        use rebellion_render::MessageCategory;
        let line = |text: &str| GameMessage::new(0, text.to_string(), MessageCategory::Event);
        let messages = [
            line("Regiment move rejected: the destination is blockaded"),
            line("Regiment arrived at Cathar"),
            // The AI's own arrival goes to its side's index only.
            line("Regiment arrived at Svivren").on_rail(
                rebellion_render::MessageRail::Mission,
                rebellion_render::RailAudience::Empire,
            ),
            line("Luke Skywalker has reached Jedi Knight tier"),
        ];
        let later = fleet_load_observation(
            request,
            regiment,
            &applied.world,
            &fog,
            &applied.movement,
            &applied.transport,
            &fleets,
            false,
            &messages,
        )
        .unwrap();
        assert_eq!(
            later.last_regiment_message.as_deref(),
            Some("Regiment arrived at Cathar")
        );
    }

    #[test]
    fn the_fleet_load_watch_reports_each_change_once() {
        // port: the fixture's own report.
        let request = request(Scenario::FleetLoad, CockpitFaction::Alliance);
        let mut applied = apply_to_loading_world(request);
        let here = applied.world.systems.keys().next().unwrap();
        let fleet = applied.world.fleets.keys().next().unwrap();
        let regiment = loading_regiment(&applied.world, here).unwrap();
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(here);
        let mut fleets = FleetWindowState::default();
        let mut watch = FleetLoadWatch::default();
        let layout = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
        let mut next = |applied: &Applied, fleets: &FleetWindowState| {
            watch.next(
                request,
                &applied.world,
                &fog,
                &applied.movement,
                &applied.transport,
                fleets,
                false,
                &[],
                None,
                None,
            )
        };

        let start = next(&applied, &fleets).unwrap();
        assert_eq!(start.status, "fleet-load");
        assert!(!start.window_open && !start.aboard);
        assert_eq!(
            start.troop_system_dat_id,
            Some(applied.world.systems[here].dat_id.raw())
        );
        assert_eq!(next(&applied, &fleets), None);

        fleets.open(
            &applied.world,
            here,
            (300, 80),
            CockpitFaction::Alliance,
            layout,
        );
        let opened = next(&applied, &fleets).unwrap();
        assert!(opened.window_open);
        assert_eq!((opened.origin, opened.entries), (Some((300, 80)), 1));
        assert_eq!(
            opened.fleet_entry,
            Some((300.0 + 4.0 + 45.5, 80.0 + 29.0 + 25.0))
        );
        assert_eq!(
            opened.troops_tab,
            Some((300.0 + 99.0 + 65.0 + 15.5, 80.0 + 96.0 + 14.0))
        );

        applied
            .transport
            .embark(&mut applied.world, fleet, &[regiment])
            .unwrap();
        let loaded = next(&applied, &fleets).unwrap();
        assert!(loaded.aboard);
        assert_eq!((loaded.cargo, loaded.troop_system_dat_id), (1, None));

        let moving = FixtureRequest {
            scenario: Scenario::FleetMove,
            ..request
        };
        assert_eq!(
            FleetLoadWatch::default().next(
                moving,
                &applied.world,
                &fog,
                &applied.movement,
                &applied.transport,
                &fleets,
                false,
                &[],
                None,
                None,
            ),
            None
        );
    }

    #[test]
    fn the_speed_menu_watch_reports_the_menu_when_it_opens_or_moves() {
        // port: the fixture's own report; the menu lists five speeds
        // (FUN_0042d190).
        use egui_macroquad::egui;
        let request = request(Scenario::FleetLoad, CockpitFaction::Alliance);
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 30.0), egui::vec2(90.0, 72.0));
        let mut watch = SpeedMenuWatch::default();

        assert_eq!(watch.next(request, None), None);
        assert_eq!(
            watch.next(request, Some(rect)),
            Some(FixtureSpeedMenu {
                status: "speed-menu",
                code: request.code,
                left: 100.0,
                top: 30.0,
                width: 90.0,
                height: 72.0,
                rows: 5,
            })
        );
        assert_eq!(watch.next(request, Some(rect)), None);
        assert_eq!(watch.next(request, None), None);
        assert!(watch.next(request, Some(rect)).is_some());
        let moving = FixtureRequest {
            scenario: Scenario::FleetMove,
            ..request
        };
        assert_eq!(SpeedMenuWatch::default().next(moving, Some(rect)), None);
        // The mission icon's gate runs the clock to start its missions.
        let quadrants = FixtureRequest {
            scenario: Scenario::Quadrants,
            ..request
        };
        assert!(SpeedMenuWatch::default()
            .next(quadrants, Some(rect))
            .is_some());
    }

    #[test]
    fn the_fleet_load_setup_reports_the_day_readouts_centre() {
        // GameSpeed: the readout is the Game Speed control (day_readout_rect).
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let loading = request(Scenario::FleetLoad, faction);
            let applied = apply_to_loading_world(loading);
            let setup =
                fleet_load_setup(loading, &applied.world, &applied.sectors, &applied.defenses)
                    .unwrap();
            let day = day_readout_rect(faction);
            assert_eq!(
                setup.day_readout,
                (day.x + day.width / 2.0, day.y + day.height / 2.0),
                "{faction:?}"
            );
            assert_eq!(setup.status, "fleet-load-setup");
            let moving = request(Scenario::FleetMove, faction);
            assert_eq!(
                fleet_load_setup(moving, &applied.world, &applied.sectors, &applied.defenses),
                None
            );
        }
    }

    #[test]
    fn the_full_fleet_load_scenario_leaves_no_room_for_the_regiment() {
        // FUN_00500b40: the room is the capacity less the regiments aboard.
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let mut applied = apply_to_loading_world(request(Scenario::FleetLoadFull, faction));
            let here = applied.world.systems.keys().next().unwrap();
            let fleet = applied.world.fleets.keys().next().unwrap();
            let capacity = TroopTransportState::fleet_capacity(&applied.world, fleet).unwrap();
            assert_eq!(applied.transport.carried_count(fleet), capacity as usize);
            let regiment = loading_regiment(&applied.world, here).unwrap();
            assert_eq!(applied.world.systems[here].ground_units, [regiment]);
            assert_eq!(
                applied
                    .transport
                    .embark(&mut applied.world, fleet, &[regiment]),
                Err(
                    rebellion_core::troop_transport::TroopTransportError::CapacityExceeded {
                        capacity,
                        requested: capacity + 1,
                    }
                )
            );
        }
    }

    #[test]
    fn the_fleet_move_scenario_leaves_the_players_fleet_alone_and_movable_beside_its_target() {
        use rebellion_core::movement::{fleet_move_confirms, validate_fleet_dispatch};
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let request = request(Scenario::FleetMove, faction);
            let applied = apply_to_fleet_world(request);
            let world = &applied.world;
            let mut systems = world.systems.keys();
            let (here, target, elsewhere) = (
                systems.next().unwrap(),
                systems.next().unwrap(),
                systems.next().unwrap(),
            );
            let fleets: Vec<_> = world.fleets.keys().collect();
            let (fleet, other, away) = (fleets[0], fleets[1], fleets[2]);
            let is_alliance = faction == CockpitFaction::Alliance;

            assert_eq!(world.systems[here].fleets, [fleet], "{faction:?}");
            assert_eq!(world.fleets[other].location, elsewhere);
            assert_eq!(world.fleets[away].location, target);
            assert_eq!(world.fleets[fleet].is_alliance, is_alliance);
            assert_eq!(
                validate_fleet_dispatch(&applied.movement, world, fleet, target, is_alliance),
                Ok(())
            );
            assert!(!fleet_move_confirms(
                world,
                applied.blockade.blockaded_systems(),
                fleet,
                false
            ));

            let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
            // The Fleet window's top-left corner, 5 pixels in from the
            // galaxy view's top-right.
            let galaxy = layout.galaxy;
            let corner = (
                galaxy.x + galaxy.width - FLEET_WINDOW_WIDTH - 5.0,
                galaxy.y + 5.0,
            );
            assert!(applied.fleets.contains_screen_point(layout, corner));
            assert!(!applied
                .fleets
                .contains_screen_point(layout, (corner.0 - 1.0, corner.1)));
            assert!(!applied
                .fleets
                .contains_screen_point(layout, (corner.0, corner.1 - 1.0)));
            let setup = fleet_move_setup(request, world, &applied.sectors, &applied.fleets)
                .expect("the scenario reports its points");
            let item = (setup.fleet_item_x, setup.fleet_item_y);
            let planet = (setup.target_screen_x, setup.target_screen_y);
            assert!(applied.fleets.contains_screen_point(layout, item));
            assert!(applied.sectors.contains_screen_point(layout, planet));
            assert!(!applied.fleets.contains_screen_point(layout, planet));
            assert_eq!(
                (setup.primary_dat_id, setup.target_dat_id),
                (0x9000_0001, 0x9000_0002)
            );
            assert_eq!(setup.systems.len(), 3);
            // The sector window's planets, the target's among them, give the
            // gate points that select a system where no bare map is left.
            assert!(!setup.sector_planets.is_empty());
            assert!(setup.sector_planets.iter().all(|point| applied
                .sectors
                .contains_screen_point(layout, (point.x, point.y))));
            assert!(setup
                .sector_planets
                .iter()
                .any(|point| point.dat_id == setup.target_dat_id && (point.x, point.y) == planet));

            let observed =
                fleet_move_observation(
                    request,
                    world,
                    &applied.movement,
                    false,
                    &applied.map,
                    (
                        &applied.fleets,
                        &FogState::new(Faction::Alliance),
                        &applied.transport,
                    ),
                    &[],
                )
                .unwrap();
            assert_eq!(
                (
                    observed.in_transit,
                    observed.destination_dat_id,
                    observed.selected_system_dat_id,
                    observed.last_message,
                ),
                (false, None, None, None)
            );
        }
    }

    #[test]
    fn the_blockade_variant_leaves_the_fleet_blockading_an_enemy_system() {
        use rebellion_core::movement::{
            fleet_move_confirms, validate_fleet_destination, FleetDispatchError,
        };
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let applied = apply_to_fleet_world(request(Scenario::FleetMoveBlockade, faction));
            let world = &applied.world;
            let mut systems = world.systems.keys();
            let (here, target) = (systems.next().unwrap(), systems.next().unwrap());
            let fleet = world.fleets.keys().next().unwrap();
            let blockaded = applied.blockade.blockaded_systems();
            let is_alliance = faction == CockpitFaction::Alliance;

            assert!(blockaded.contains(&here), "{faction:?}");
            assert_eq!(
                validate_fleet_destination(
                    &applied.movement,
                    world,
                    blockaded,
                    fleet,
                    target,
                    is_alliance
                ),
                Err(FleetDispatchError::Blockaded)
            );
            // The enemy holds the system, so a menu Move does not ask.
            assert!(!fleet_move_confirms(world, blockaded, fleet, false));
        }
    }

    #[test]
    fn a_dispatched_fleet_reports_its_transit_and_the_last_message() {
        let request = request(Scenario::FleetMove, CockpitFaction::Alliance);
        let mut applied = apply_to_fleet_world(request);
        let fleet = applied.world.fleets.keys().next().unwrap();
        let target = applied.world.systems.keys().nth(1).unwrap();
        assert!(begin_fleet_transit(
            &mut applied.movement,
            &mut applied.world,
            fleet,
            target,
            5
        ));
        applied.map.selected_system = Some(target);
        let message = GameMessage::new(
            0,
            "Fleet move rejected",
            rebellion_render::MessageCategory::Event,
        );

        let fog = FogState::new(Faction::Alliance);
        let observe = |applied: &Applied, message: &GameMessage| {
            fleet_move_observation(
                request,
                &applied.world,
                &applied.movement,
                true,
                &applied.map,
                (&applied.fleets, &fog, &applied.transport),
                std::slice::from_ref(message),
            )
            .unwrap()
        };
        let observed = observe(&applied, &message);
        // No window at the destination is open yet.
        assert_eq!(observed.destination_entry, None);

        assert_eq!(
            (
                observed.in_transit,
                observed.destination_dat_id,
                observed.confirmation_open,
                observed.selected_system_dat_id,
                observed.last_message.as_deref(),
            ),
            (
                true,
                Some(0x9000_0002),
                true,
                Some(0x9000_0002),
                Some("Fleet move rejected")
            )
        );

        // move-order.md: the destination's Fleet window lists the fleet
        // en route (a0-vm/menus/18-fleet4.png), so the gate can open its
        // menu there.
        let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
        let point = applied
            .sectors
            .fleet_window_point(&applied.world, layout, target)
            .unwrap();
        assert!(applied.fleets.open(
            &applied.world,
            target,
            point,
            request.faction,
            layout
        ));
        let opened = observe(&applied, &message);
        let [x, y] = opened.destination_entry.unwrap();
        let window = applied.fleets.screen_rect(layout, target).unwrap();
        assert!(window.contains(egui_macroquad::egui::pos2(x, y)));
        // Its ships are on the right list once the fleet is the selection.
        assert_eq!(opened.destination_item, None);
        assert!(applied
            .fleets
            .select(target, FleetWindowEntry::Fleet(fleet)));
        let [x, y] = observe(&applied, &message).destination_item.unwrap();
        let item = applied.fleets.item_screen_rect(layout, target, 0).unwrap();
        assert_eq!(egui_macroquad::egui::pos2(x, y), item.center());
    }

    #[test]
    fn only_the_fleet_move_scenarios_report_fleet_moves() {
        let request = request(Scenario::MissionTargeting, CockpitFaction::Alliance);
        let applied = apply_to_fleet_world(request);

        assert_eq!(
            fleet_move_setup(request, &applied.world, &applied.sectors, &applied.fleets),
            None
        );
        assert_eq!(
            fleet_move_observation(
                request,
                &applied.world,
                &applied.movement,
                false,
                &applied.map,
                (
                    &applied.fleets,
                    &FogState::new(Faction::Alliance),
                    &applied.transport
                ),
                &[]
            ),
            None
        );
    }

    /// The quadrant scenario on the loading world (three systems, the third
    /// in its own sector) plus a fourth in that sector, with one Alliance
    /// fleet that carries the world's first agent, a bystander fleet and
    /// agent at the fourth system, an agent waiting at the second system,
    /// and three more agents of each side nowhere.
    fn apply_quadrants(faction: CockpitFaction) -> Applied {
        apply_to_loading_world_with(request(Scenario::Quadrants, faction), |world| {
            use rebellion_core::world::{Character, Fleet};
            let carried = world.characters.keys().next().unwrap();
            let mut keys = world.systems.keys();
            let (second, third) = (keys.nth(1).unwrap(), keys.next().unwrap());
            let mut fourth = world.systems[third].clone();
            fourth.dat_id = rebellion_core::ids::DatId::new(0x9000_0004);
            let fourth = world.systems.insert(fourth);
            let far = world.systems[third].sector;
            world.sectors[far].systems.push(fourth);
            world.characters[carried].current_system = Some(third);
            for (location, is_alliance) in [(fourth, false), (fourth, true)] {
                world.fleets.insert(Fleet {
                    location,
                    capital_ships: Vec::new(),
                    fighters: Vec::new(),
                    characters: Vec::new(),
                    is_alliance,
                    has_death_star: false,
                    special_forces: Vec::new(),
                });
            }
            for (name, location) in [("Bystander", fourth), ("Waiting", second)] {
                world.characters.insert(Character {
                    name: name.into(),
                    is_alliance: false,
                    is_empire: false,
                    current_system: Some(location),
                    ..Default::default()
                });
            }
            world.fleets.insert(Fleet {
                location: third,
                capital_ships: Vec::new(),
                fighters: Vec::new(),
                characters: vec![carried],
                is_alliance: true,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            for (index, is_alliance) in [true, true, true, false, false, false]
                .into_iter()
                .enumerate()
            {
                world.characters.insert(Character {
                    name: format!("Agent {index}"),
                    is_alliance,
                    is_empire: !is_alliance,
                    recruited: true,
                    ..Default::default()
                });
            }
        })
    }

    #[test]
    fn the_quadrant_scenario_gives_each_checked_icon_its_side() {
        // port: the fixture's own layout; the sides follow FUN_0045cdc0,
        // FUN_0045ce80, FUN_0045ccc0 and FUN_004a1f60.
        use rebellion_render::quadrant_icons::quadrant_side;
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let applied = apply_quadrants(faction);
            let world = &applied.world;
            let mut keys = world.systems.keys();
            let (primary, second) = (keys.next().unwrap(), keys.next().unwrap());
            let (player, own, other) = match faction {
                CockpitFaction::Alliance => (Faction::Alliance, 1, 2),
                CockpitFaction::Empire => (Faction::Empire, 2, 1),
            };
            let fog = FogState::new(player);
            let side = |system, quadrant| {
                quadrant_side(
                    world,
                    &fog,
                    &applied.missions,
                    &applied.movement,
                    rebellion_render::quadrant_icons::Travellers::default(),
                    player,
                    system,
                    quadrant,
                )
            };
            assert_eq!(side(primary, Quadrant::System), Some(own), "{faction:?}");
            assert_eq!(side(primary, Quadrant::Defenses), Some(own), "{faction:?}");
            assert_eq!(side(primary, Quadrant::Fleets), Some(own), "{faction:?}");
            assert_eq!(
                side(primary, Quadrant::Missions),
                Some(other),
                "{faction:?}"
            );
            assert_eq!(side(second, Quadrant::System), None, "{faction:?}");
            assert_eq!(side(second, Quadrant::Defenses), Some(other), "{faction:?}");
            assert_eq!(side(second, Quadrant::Fleets), None, "{faction:?}");
            assert_eq!(side(second, Quadrant::Missions), Some(own), "{faction:?}");
            assert_eq!(applied.missions.missions().len(), 3, "{faction:?}");
            assert_eq!(applied.sectors.window_count(), 1, "{faction:?}");

            // Only what the scenario placed lies at the two systems.
            let is_alliance = faction == CockpitFaction::Alliance;
            let own_fleets: Vec<_> = world
                .fleets
                .iter()
                .filter(|(_, fleet)| fleet.location == primary)
                .map(|(_, fleet)| fleet.is_alliance)
                .collect();
            assert_eq!(own_fleets, [is_alliance], "{faction:?}");
            assert!(world.fleets.values().all(|fleet| fleet.location != second));
            let at = |system| {
                world
                    .characters
                    .values()
                    .filter(|character| character.current_system == Some(system))
                    .count()
            };
            // The second system's agent is in hyperspace to it.
            assert_eq!((at(primary), at(second)), (2, 0), "{faction:?}");
            let bound: Vec<_> = applied.missions.en_route().iter().map(|transit| transit.to).collect();
            assert_eq!(bound, [second], "{faction:?}");
            // What lies at the fourth system stays there.
            let fourth = world.systems.keys().nth(3).unwrap();
            assert_eq!(at(fourth), 1, "{faction:?}");
            assert_eq!(
                world
                    .fleets
                    .values()
                    .filter(|fleet| fleet.location == fourth)
                    .count(),
                2,
                "{faction:?}"
            );
            // Each mission is sent by its agent's side.
            for mission in applied.missions.missions() {
                let MissionMember::Character(agent) = mission.team[0] else {
                    panic!("the scenario sends characters");
                };
                let sent_by_alliance = mission.faction == MissionFaction::Alliance;
                assert_eq!(
                    sent_by_alliance, world.characters[agent].is_alliance,
                    "{faction:?}"
                );
            }
        }
    }

    #[test]
    fn the_quadrant_scenario_leaves_an_agent_aboard_a_fleet_where_it_is() {
        let applied = apply_quadrants(CockpitFaction::Alliance);
        let world = &applied.world;
        let carried = world.characters.keys().next().unwrap();
        let third = world.systems.keys().nth(2).unwrap();
        assert_eq!(world.characters[carried].current_system, Some(third));
        assert!(!world.characters[carried].on_mission);
        assert!(applied
            .missions
            .missions()
            .iter()
            .all(|mission| !mission.team.contains(&MissionMember::Character(carried))));
    }

    #[test]
    fn the_quadrant_setup_reports_both_systems_four_rects_in_kind_order() {
        let applied = apply_quadrants(CockpitFaction::Empire);
        let request = request(Scenario::Quadrants, CockpitFaction::Empire);
        let report = quadrant_setup(request, &applied.world, &applied.sectors).unwrap();
        let world = &applied.world;
        let mut keys = world.systems.keys();
        let (primary, second) = (keys.next().unwrap(), keys.next().unwrap());
        let layout = CockpitState::new(CockpitFaction::Empire).layout_for(640.0, 480.0);
        assert_eq!(report.status, "quadrant-setup");
        assert_eq!(report.primary_dat_id, world.systems[primary].dat_id.raw());
        assert_eq!(report.second_dat_id, world.systems[second].dat_id.raw());
        assert_eq!(report.scale, layout.scale);
        let names = ["system", "defenses", "fleets", "missions"];
        let kinds = [
            Quadrant::System,
            Quadrant::Defenses,
            Quadrant::Fleets,
            Quadrant::Missions,
        ];
        let mut expected = Vec::new();
        for system in [primary, second] {
            for (quadrant, name) in kinds.into_iter().zip(names) {
                let rect = applied
                    .sectors
                    .quadrant_screen_rect(world, layout, system, quadrant)
                    .unwrap();
                expected.push(FixtureQuadrant {
                    system_dat_id: world.systems[system].dat_id.raw(),
                    quadrant: name,
                    rect: [rect.min.x, rect.min.y, rect.width(), rect.height()],
                });
            }
        }
        assert_eq!(report.quadrants, expected);
        // The sector's third and fourth systems, when it has them: neutral
        // with a mine, and destroyed.
        let marks = mark_systems(world, primary);
        assert_eq!(report.neutral.is_some(), marks.len() > 2);
        assert_eq!(report.destroyed.is_some(), marks.len() > 3);
        if let [_, _, neutral, ruin] = marks[..] {
            assert_eq!(world.systems[neutral].control, ControlKind::Uncontrolled);
            assert!(!world.systems[neutral].production_facilities.is_empty());
            assert!(world.systems[ruin].is_destroyed);
        }
        assert!(quadrant_setup(
            super::tests::request(Scenario::FleetLoad, CockpitFaction::Empire),
            world,
            &applied.sectors
        )
        .is_none());
    }

    /// A0 coruscant-676/14 (the VM's Empire game, day 679, after the
    /// Alliance seized Coruscant): the Empire's Defenses window of Coruscant
    /// shows the Alliance's side, Personnel pressed over an empty list,
    /// Trooper Regiments full and the other tabs empty.
    #[test]
    fn the_seized_coruscant_scenario_leaves_the_alliance_one_regiment_there_in_the_empires_sight() {
        let request = request(Scenario::CoruscantSeized, CockpitFaction::Empire);
        let applied = apply_to_loading_world_with(request, |world| {
            let held = world.systems.keys().nth(1).unwrap();
            world.systems[held].name = "Coruscant".into();
            world.systems[held].control = ControlKind::Controlled(Faction::Empire);
            // A third system in the sector, which the quadrant scenario
            // would mark; this one marks nothing.
            let mut third = world.systems[held].clone();
            third.name = "Third".into();
            third.dat_id = rebellion_core::ids::DatId::new(0x9000_0005);
            let third = world.systems.insert(third);
            let sector = world.systems[held].sector;
            world.sectors[sector].systems.push(third);
            // `apply` moves the world's first character to the primary
            // system, so the one at Coruscant is a second.
            let mut agent = world.characters.values().next().unwrap().clone();
            agent.name = "Bystander".into();
            agent.current_system = Some(held);
            world.characters.insert(agent);
            let stormtroopers = world.troops.insert(TroopUnit {
                class_dat_id: rebellion_core::ids::DatId::new(0x1000_0006),
                is_alliance: false,
                regiment_strength: 100,
            });
            world.systems[held].ground_units.push(stormtroopers);
        });
        let world = &applied.world;
        let here = world.systems.keys().next().unwrap();
        let held = super::coruscant(world).unwrap();
        assert_eq!(
            world.systems[held].control,
            ControlKind::Controlled(Faction::Alliance)
        );
        let [regiment] = world.systems[held].ground_units[..] else {
            panic!("{:?}", world.systems[held].ground_units);
        };
        assert!(world.troops[regiment].is_alliance);
        assert!(world.fleets.values().all(|fleet| fleet.location != held));
        // What stood there went to another sector.
        let agent = world
            .characters
            .values()
            .find(|character| character.name == "Bystander")
            .unwrap();
        let moved_to = agent.current_system.unwrap();
        assert_ne!(world.systems[moved_to].sector, world.systems[held].sector);
        assert!(world
            .characters
            .values()
            .all(|character| character.current_system != Some(held)));

        // The quadrant gate's second system is Coruscant.
        let setup = quadrant_setup(request, world, &applied.sectors).unwrap();
        assert_eq!(
            (setup.primary_dat_id, setup.second_dat_id),
            (
                world.systems[here].dat_id.raw(),
                world.systems[held].dat_id.raw()
            )
        );
        assert_eq!(setup.quadrants.len(), 8);
        assert_eq!((setup.headquarters, setup.uprising), (None, None));

        let mut fog_alliance = FogState::new(Faction::Alliance);
        let mut fog = FogState::new(Faction::Empire);
        reveal(request, world, &mut fog_alliance, &mut fog);
        let economy = EconomyState::default();
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::default();
        let deliveries = rebellion_core::delivery::DeliveryState::default();
        let missions_windows = MissionsWindowState::default();
        let mut defenses = DefensesWindowState::default();
        let layout = CockpitState::new(CockpitFaction::Empire).layout_for(640.0, 480.0);
        assert!(defenses.open(world, held, (200, 91), CockpitFaction::Empire, layout));
        let shown = QuadrantWatch::default()
            .next(
                request,
                world,
                &QuadrantWindows {
                    systems: &applied.systems,
                    defenses: &defenses,
                    missions_windows: &missions_windows,
                    fog: &fog,
                    missions: &applied.missions,
                    production: rebellion_render::DefensesProduction {
                        manufacturing: &manufacturing,
                        deliveries: &deliveries,
                        regiments: &[],
                        squadrons: &[],
                    },
                    economy: &economy,
                },
            )
            .unwrap();
        let [window] = &shown.defenses_windows[..] else {
            panic!("{:?}", shown.defenses_windows);
        };
        assert_eq!((window.side, window.page), (1, "personnel"));
        assert_eq!(window.counts, [0, 1, 0, 0, 0]);
    }

    /// A0 2026-11-11-icon-menu-topic/05 (the VM's Empire game): Coruscant's
    /// Fighter Squadrons page lists ten TIE Fighter squadrons, past the
    /// list's three lines, and the fixture reports every cell's rect.
    #[test]
    fn the_coruscant_squadrons_scenario_lists_ten_tie_fighters_there_with_every_cell() {
        let request = request(Scenario::CoruscantSquadrons, CockpitFaction::Empire);
        let applied = apply_to_loading_world_with(request, |world| {
            let held = world.systems.keys().nth(1).unwrap();
            world.systems[held].name = "Coruscant".into();
            // The Empire's other classes and the Alliance's own TIE-named
            // one are not the squadrons placed.
            for (raw, name, is_alliance) in [
                (0x1c00_0006, "TIE Interceptor", false),
                (0x1c00_0001, "TIE Fighter", true),
            ] {
                world
                    .fighter_classes
                    .insert(rebellion_core::world::FighterClass {
                        dat_id: rebellion_core::ids::DatId::new(raw),
                        name: name.into(),
                        is_alliance,
                        is_empire: !is_alliance,
                        ..Default::default()
                    });
            }
            world
                .fighter_classes
                .insert(rebellion_core::world::FighterClass {
                    dat_id: rebellion_core::ids::DatId::new(0x1c00_0005),
                    name: "TIE Fighter".into(),
                    is_empire: true,
                    ..Default::default()
                });
        });
        let world = &applied.world;
        let held = super::coruscant(world).unwrap();
        assert_eq!(
            world.systems[held].control,
            ControlKind::Controlled(Faction::Empire)
        );
        let [entry] = &world.systems[held].fighters[..] else {
            panic!("{:?}", world.systems[held].fighters);
        };
        assert_eq!((entry.count, entry.carrier), (10, None));
        assert_eq!(world.fighter_classes[entry.class].dat_id.raw(), 0x1c00_0005);
        let setup = quadrant_setup(request, world, &applied.sectors).unwrap();
        assert_eq!(setup.second_dat_id, world.systems[held].dat_id.raw());

        let mut fog_alliance = FogState::new(Faction::Alliance);
        let mut fog = FogState::new(Faction::Empire);
        fog.reveal(held);
        reveal(request, world, &mut fog_alliance, &mut fog);
        let economy = EconomyState::default();
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::default();
        let deliveries = rebellion_core::delivery::DeliveryState::default();
        let missions_windows = MissionsWindowState::default();
        let mut defenses = DefensesWindowState::default();
        let layout = CockpitState::new(CockpitFaction::Empire).layout_for(640.0, 480.0);
        assert!(defenses.open(world, held, (203, 91), CockpitFaction::Empire, layout));
        assert!(defenses.show_page(held, DefensesPage::Squadrons));
        let shown = QuadrantWatch::default()
            .next(
                request,
                world,
                &QuadrantWindows {
                    systems: &applied.systems,
                    defenses: &defenses,
                    missions_windows: &missions_windows,
                    fog: &fog,
                    missions: &applied.missions,
                    production: rebellion_render::DefensesProduction {
                        manufacturing: &manufacturing,
                        deliveries: &deliveries,
                        regiments: &[],
                        squadrons: &[],
                    },
                    economy: &economy,
                },
            )
            .unwrap();
        let [window] = &shown.defenses_windows[..] else {
            panic!("{:?}", shown.defenses_windows);
        };
        assert_eq!((window.side, window.page), (2, "squadrons"));
        assert_eq!(window.rows.len(), 10);
        // The tenth cell, the fourth line's first, starts below the list.
        assert_eq!(window.cells.len(), 10);
        assert_eq!(window.cells[9][1] - window.cells[0][1], 210.0);
    }

    #[test]
    fn the_quadrant_watch_reports_the_open_windows_and_the_rail_when_they_change() {
        let mut applied = apply_quadrants(CockpitFaction::Alliance);
        let request = request(Scenario::Quadrants, CockpitFaction::Alliance);
        let primary = applied.world.systems.keys().next().unwrap();
        let mut fog = FogState::new(Faction::Alliance);
        fog.reveal(primary);
        let economy = EconomyState::default();
        let manufacturing = rebellion_core::manufacturing::ManufacturingState::default();
        let deliveries = rebellion_core::delivery::DeliveryState::default();
        let mut defenses = DefensesWindowState::default();
        let mut missions_windows = MissionsWindowState::default();
        let mut watch = QuadrantWatch::default();
        macro_rules! next {
            ($watch:expr) => {
                $watch.next(
                    request,
                    &applied.world,
                    &QuadrantWindows {
                        systems: &applied.systems,
                        defenses: &defenses,
                        missions_windows: &missions_windows,
                        fog: &fog,
                        missions: &applied.missions,
                        production: rebellion_render::DefensesProduction {
                            manufacturing: &manufacturing,
                            deliveries: &deliveries,
                            regiments: &[],
                            squadrons: &[],
                        },
                        economy: &economy,
                    },
                )
            };
        }
        let first = next!(watch).unwrap();
        assert_eq!(first.status, "quadrant-observation");
        assert!(first.system_windows.is_empty() && first.defenses_windows.is_empty());
        assert!(first.missions_windows.is_empty());
        assert!(first.rail.is_empty());
        assert!(next!(watch).is_none());

        let layout = CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
        let dat = applied.world.systems[primary].dat_id.raw();
        assert!(applied.systems.open(
            &applied.world,
            primary,
            (100, 50),
            CockpitFaction::Alliance,
            layout
        ));
        let opened = next!(watch).unwrap();
        assert_eq!(opened.system_windows, [(dat, (100, 50))]);

        // The primary system's Defenses window: the player's regiment and
        // KDY-150, the garrison line on the regiment page, and each tab's
        // and listed cell's rect.
        assert!(defenses.open(
            &applied.world,
            primary,
            (200, 60),
            CockpitFaction::Alliance,
            layout
        ));
        let shown = next!(watch).unwrap();
        let [window] = &shown.defenses_windows[..] else {
            panic!("{:?}", shown.defenses_windows);
        };
        assert_eq!(window.system_dat_id, dat);
        assert_eq!(window.origin, (200, 60));
        assert_eq!((window.side, window.page), (1, "personnel"));
        assert_eq!(window.counts, [0, 1, 0, 0, 1]);
        assert!(window.rows.is_empty() && window.cells.is_empty());
        assert_eq!((window.selected, window.garrison.as_deref()), (None, None));
        let tab = |name: &str| {
            window
                .tabs
                .iter()
                .find(|(tab, _)| *tab == name)
                .map(|(_, rect)| *rect)
        };
        let origin = |x: f32, y: f32| {
            [
                layout.canvas.x + (200.0 + x) * layout.scale,
                layout.canvas.y + (60.0 + y) * layout.scale,
            ]
        };
        let [left, top] = origin(64.0, 20.0);
        assert_eq!(
            tab("regiments"),
            Some([left, top, 36.0 * layout.scale, 33.0 * layout.scale])
        );
        assert_eq!(window.tabs.len(), 5);

        applied.systems.minimize_defenses_window(
            primary,
            (200, 60),
            rebellion_render::defenses_window::DefensesPage::Personnel,
        );
        let railed = next!(watch).unwrap();
        let slot = applied.systems.rail_slot_screen_rect(layout, 0).unwrap();
        assert_eq!(railed.rail, [(dat, "defenses", super::rect_array(slot))]);

        // The primary system's Missions window: the player's Diplomacy
        // mission first (selected, its agent listed), then the other
        // side's; both target the system.
        assert!(missions_windows.open(
            &applied.world,
            &fog,
            &applied.missions,
            primary,
            (150, 40),
            CockpitFaction::Alliance,
            layout
        ));
        let shown = next!(watch).unwrap();
        let [window] = &shown.missions_windows[..] else {
            panic!("{:?}", shown.missions_windows);
        };
        assert_eq!((window.system_dat_id, window.origin), (dat, (150, 40)));
        assert_eq!(
            window.rows,
            [
                ("Diplomacy".to_string(), 1, 0x4c10),
                ("Diplomacy".to_string(), 2, 0x5c10)
            ]
        );
        assert_eq!(window.side, 2);
        assert_eq!(
            (window.selected, window.tab, window.tab_side),
            (Some(0), "agents", Some(1))
        );
        assert_eq!(window.members.len(), 1);
        assert_eq!(window.member_rects.len(), 1);
        assert_eq!(
            window.target.as_deref(),
            Some(applied.world.systems[primary].name.as_str())
        );
        let [left, top] = [
            layout.canvas.x + (150.0 + 5.0) * layout.scale,
            layout.canvas.y + (40.0 + 74.0) * layout.scale,
        ];
        assert_eq!(
            window.row_rects[1],
            [left, top, 90.0 * layout.scale, 50.0 * layout.scale]
        );
        assert_eq!(window.tabs.len(), 2);

        applied.systems.minimize_missions_window(primary, (150, 40));
        let railed = next!(watch).unwrap();
        assert_eq!(railed.rail.last().map(|entry| entry.1), Some("missions"));

        let other = super::tests::request(Scenario::FleetLoad, CockpitFaction::Alliance);
        assert!(QuadrantWatch::default()
            .next(
                other,
                &applied.world,
                &QuadrantWindows {
                    systems: &applied.systems,
                    defenses: &defenses,
                    missions_windows: &missions_windows,
                    fog: &fog,
                    missions: &applied.missions,
                    production: rebellion_render::DefensesProduction {
                        manufacturing: &manufacturing,
                        deliveries: &deliveries,
                        regiments: &[],
                        squadrons: &[],
                    },
                    economy: &economy,
                },
            )
            .is_none());
    }

    #[test]
    fn the_subdue_scenario_revolts_the_primary_and_sends_the_agent_to_subdue_it() {
        // Our fixture: a revolt the player's garrison may end, an agent on
        // Subdue Uprising (MISSNSD 0x57000080) at it, and the clock Fast.
        let mut world = diplomacy_world(Faction::Alliance);
        let here = world.systems.keys().next().unwrap();
        world.characters.insert(rebellion_core::world::Character {
            name: "Agent".into(),
            is_alliance: true,
            current_system: Some(here),
            ..Default::default()
        });
        let request = request(Scenario::SubdueSuccess, CockpitFaction::Alliance);
        let mut missions = MissionState::default();
        let mut uprisings = rebellion_core::uprising::UprisingState::new();
        apply(
            request,
            &mut world,
            &mut GameMode::MainMenu,
            &mut MissionFaction::Alliance,
            &mut CockpitState::new(CockpitFaction::Alliance),
            &mut GalaxyMapState::default(),
            &mut MovementState::default(),
            &mut ManufacturingState::default(),
            &mut EconomyState::default(),
            &mut missions,
            &mut BlockadeState::default(),
            &mut SectorWindowState::default(),
            &mut SystemWindowState::default(),
            &mut FleetWindowState::default(),
            &mut DefensesWindowState::default(),
            &mut TroopTransportState::default(),
            &mut uprisings,
        );

        assert!(runs_the_clock(request));
        assert!(!runs_the_clock(super::tests::request(
            Scenario::CharacterMove,
            CockpitFaction::Alliance
        )));
        assert!(uprisings.is_uprising(here));
        assert_eq!(
            world.systems[here].control,
            ControlKind::Uprising(Faction::Alliance)
        );
        let regiments = world.systems[here]
            .ground_units
            .iter()
            .filter(|&&key| world.troops[key].is_alliance)
            .count();
        assert!(regiments >= 8, "{regiments}");
        let kinds: Vec<_> = missions.missions().iter().map(|m| m.kind).collect();
        assert_eq!(kinds, [MissionKind::SubdueUprising]);
    }

    // A0 sfstatus/03: the Alliance's Infiltrators en route to Yavin, listed
    // there; FUN_0042c3b0 gives class 2 the STRATEGY 11520 mark.
    #[test]
    fn the_special_force_enroute_scenario_sends_a_team_to_the_primary_system() {
        for (faction, class, alliance) in [
            (CockpitFaction::Alliance, 0x3c00_0002, true),
            (CockpitFaction::Empire, 0x3c00_0007, false),
        ] {
            let applied = apply_to_fleet_world(request(Scenario::SpecialForceEnroute, faction));
            let primary = applied.world.systems.keys().next().unwrap();
            let [transit] = applied.missions.en_route() else {
                panic!("{:?}", applied.missions.en_route());
            };
            let MissionMember::SpecialForce(unit) = transit.member else {
                panic!("{transit:?}");
            };
            assert_eq!(transit.to, primary);
            assert_eq!(transit.abode, None);
            let unit = &applied.world.special_forces[unit];
            assert_eq!(
                (unit.class_dat_id.raw(), unit.is_alliance),
                (class, alliance)
            );
            assert!(applied.world.systems[primary].special_forces.is_empty());
        }
        let recon =
            apply_to_fleet_world(request(Scenario::ReconTargeting, CockpitFaction::Alliance));
        assert!(recon.missions.en_route().is_empty());
    }

    // A0 2026-11-11-regiment-en-route/00: Averam lists the Alliance Army
    // Regiment moving there from Yavin.
    #[test]
    fn the_regiment_enroute_scenario_moves_a_regiment_to_the_primary_system() {
        for (faction, class, alliance) in [
            (CockpitFaction::Alliance, 0x1000_0002, true),
            (CockpitFaction::Empire, 0x1000_0008, false),
        ] {
            // The primary's own regiment leaves its list; the other side's
            // stays.
            let mut kept = None;
            let applied =
                apply_to_loading_world_with(request(Scenario::RegimentEnroute, faction), |world| {
                    let primary = world.systems.keys().next().unwrap();
                    for is_alliance in [alliance, !alliance] {
                        let troop = world.troops.insert(rebellion_core::world::TroopUnit {
                            class_dat_id: rebellion_core::ids::DatId::new(class),
                            is_alliance,
                            regiment_strength: 100,
                        });
                        world.systems[primary].ground_units.push(troop);
                        kept = Some(troop);
                    }
                });
            let primary = applied.world.systems.keys().next().unwrap();
            assert_eq!(applied.world.systems[primary].ground_units, [kept.unwrap()]);
            let [transit] = applied.transport.transits() else {
                panic!("{:?}", applied.transport.transits());
            };
            assert_eq!(
                transit.leg,
                rebellion_core::troop_transport::RegimentLeg::Surface(primary)
            );
            assert_ne!(transit.origin, primary);
            let troop = &applied.world.troops[transit.troop];
            assert_eq!(
                (troop.class_dat_id.raw(), troop.is_alliance),
                (class, alliance)
            );
        }
        let recon =
            apply_to_fleet_world(request(Scenario::ReconTargeting, CockpitFaction::Alliance));
        assert!(recon.transport.transits().is_empty());
    }

    // FUN_00556390 puts a squadron sent by Move in its destination at once.
    #[test]
    fn the_squadron_enroute_scenario_moves_a_squadron_to_the_primary_planet() {
        for (faction, index, alliance) in [
            (CockpitFaction::Alliance, 1, true),
            (CockpitFaction::Empire, 5, false),
        ] {
            let applied =
                apply_to_loading_world_with(request(Scenario::SquadronEnroute, faction), |world| {
                    for (raw, alliance) in [(0x1c00_0001, true), (0x1c00_0005, false)] {
                        world
                            .fighter_classes
                            .insert(rebellion_core::world::FighterClass {
                                dat_id: rebellion_core::ids::DatId::new(raw),
                                is_alliance: alliance,
                                is_empire: !alliance,
                                ..Default::default()
                            });
                    }
                });
            let mut keys = applied.world.systems.keys();
            let (primary, secondary) = (keys.next().unwrap(), keys.next().unwrap());
            let [first, second] = applied.transport.squadrons.transits() else {
                panic!("{:?}", applied.transport.squadrons.transits());
            };
            for (transit, to) in [(first, primary), (second, secondary)] {
                assert_eq!(
                    transit.to,
                    rebellion_core::squadrons::SquadronPlace::Planet(to)
                );
                assert_ne!(transit.origin, to);
                assert!(transit.arrival_tick > 0);
                let class = &applied.world.fighter_classes[transit.class];
                assert_eq!((class.dat_id.index(), class.is_alliance), (index, alliance));
            }
            let empty = &applied.world.systems[secondary];
            assert!(empty.ground_units.is_empty() && empty.fighters.is_empty());
            assert!(empty.defense_facilities.is_empty() && empty.special_forces.is_empty());
        }
    }

    // FUN_00556390 puts each object sent by Move into its target container,
    // here the fleet, at once (regiment-unload.md).
    #[test]
    fn the_fleet_arrivals_scenario_sends_a_regiment_squadron_character_and_special_force_aboard() {
        for (faction, index) in [(CockpitFaction::Alliance, 1), (CockpitFaction::Empire, 5)] {
            let alliance = faction == CockpitFaction::Alliance;
            let applied =
                apply_to_loading_world_with(request(Scenario::FleetArrivals, faction), |world| {
                    for (raw, alliance) in [(0x1c00_0001, true), (0x1c00_0005, false)] {
                        world
                            .fighter_classes
                            .insert(rebellion_core::world::FighterClass {
                                dat_id: rebellion_core::ids::DatId::new(raw),
                                is_alliance: alliance,
                                is_empire: !alliance,
                                hyperdrive: 1,
                                ..Default::default()
                            });
                    }
                    let here = world.systems.keys().next().unwrap();
                    world.characters.insert(rebellion_core::world::Character {
                        name: "Rider".into(),
                        is_alliance: alliance,
                        is_empire: !alliance,
                        recruited: true,
                        current_system: Some(here),
                        ..Default::default()
                    });
                });
            let fleet = applied.world.fleets.keys().next().unwrap();
            let [regiment] = applied.transport.transits() else {
                panic!("{faction:?} {:?}", applied.transport.transits());
            };
            assert_eq!(
                regiment.leg,
                rebellion_core::troop_transport::RegimentLeg::Fleet(fleet)
            );
            let [squadron] = applied.transport.squadrons.transits() else {
                panic!("{faction:?} {:?}", applied.transport.squadrons.transits());
            };
            assert_eq!(
                squadron.to,
                rebellion_core::squadrons::SquadronPlace::Fleet(fleet)
            );
            assert_eq!(
                applied.world.fighter_classes[squadron.class].dat_id.index(),
                index
            );
            let arrivals = rebellion_render::quadrant_icons::fleet_arrivals(&applied.missions);
            assert_eq!(arrivals.len(), 2, "{faction:?}");
            assert!(arrivals.iter().all(|&(_, to)| to == fleet));
            assert!(matches!(arrivals[0].0, MissionMember::Character(_)));
            let MissionMember::SpecialForce(force) = arrivals[1].0 else {
                panic!("{faction:?} {arrivals:?}");
            };
            assert_eq!(applied.world.special_forces[force].is_alliance, alliance);
            assert!(applied.transport.cargo(fleet).is_empty());
            assert!(applied.world.fleets[fleet].fighters.is_empty());
        }
    }

    // FUN_00465bb0's destroyed branch reads "Name - Killed" (0x8798); Steam
    // guide guide-023 lists "Ackbar - Killed" in the Personnel Finder.
    #[test]
    fn the_personnel_states_scenario_lists_its_sixth_character_as_killed() {
        let names = ["One", "Two", "Three", "Four", "Five", "Six"];
        let applied = apply_to_loading_world_with(
            request(Scenario::PersonnelStates, CockpitFaction::Alliance),
            |world| {
                world.characters.clear();
                let here = world.systems.keys().next().unwrap();
                let mut last = None;
                for name in names {
                    last = Some(world.characters.insert(rebellion_core::world::Character {
                        name: name.into(),
                        is_alliance: true,
                        recruited: true,
                        current_system: Some(here),
                        ..Default::default()
                    }));
                }
                // The sixth rides a fleet, which must drop it when it dies.
                let fleet = world.fleets.keys().next().expect("a fleet");
                world.fleets[fleet].characters.push(last.unwrap());
            },
        );
        let world = &applied.world;
        let killed: Vec<_> = world
            .characters
            .values()
            .filter(|character| character.is_killed)
            .map(|character| character.name.as_str())
            .collect();
        assert_eq!(killed, ["Six"]);
        let six = world.characters.values().find(|c| c.name == "Six").unwrap();
        assert_eq!((six.current_system, six.current_fleet), (None, None));
        assert!(world.fleets.values().all(|fleet| fleet
            .characters
            .iter()
            .all(|&key| world.characters[key].name != "Six")));
        let fog = rebellion_core::fog::FogState::new(Faction::Alliance);
        let mut finder = rebellion_render::personnel_finder::PersonnelFinderState::default();
        finder.open(CockpitFaction::Alliance);
        let texts = finder.shown_character_rows(world, &fog).unwrap();
        assert!(texts.iter().any(|text| text == "Six - Killed"), "{texts:?}");
    }

    #[test]
    fn the_targeting_fleet_move_and_fleet_window_scenarios_report_the_object_menu() {
        let reporting: Vec<_> = (0..SCENARIO_COUNT)
            .filter_map(Scenario::decode)
            .filter(|scenario| scenario.reports_object_menu())
            .collect();

        assert_eq!(
            reporting,
            [
                Scenario::MissionTargeting,
                Scenario::FleetMove,
                Scenario::FleetMoveBlockade,
                Scenario::FleetLoad,
                Scenario::FleetLoadFull,
                Scenario::Quadrants,
                Scenario::RegimentUnloadRefused,
                Scenario::FleetJoin,
                Scenario::ProductionDestination,
                Scenario::BuildSelection,
                Scenario::RescueTargeting,
                Scenario::SabotageTargeting,
                Scenario::DeathStarTargeting,
                Scenario::ShipTargeting,
                Scenario::FleetTargeting,
                Scenario::ReconTargeting,
                Scenario::ResearchTargeting,
                Scenario::SquadronLoad,
                Scenario::SquadronBattle,
                Scenario::FleetBombardment,
                Scenario::FleetAssault,
                Scenario::CharacterCommand,
                Scenario::CharacterMove,
                Scenario::CharacterBoard,
                Scenario::CharacterAboard,
                Scenario::FleetDestroySystem,
                Scenario::ProductionDelivery,
                Scenario::UprisingIncident,
                Scenario::BlockadeArrival,
                Scenario::SupportFlip,
                Scenario::FleetWindowA0,
                Scenario::FleetWindowYavin,
                Scenario::BuildSelectionOverMaintenance,
                Scenario::BuildSelectionDestination,
                Scenario::EnemyCharacterTargeting,
                Scenario::UprisingTargeting,
                Scenario::BatteryStatus,
                Scenario::PersonnelStates,
                Scenario::ForceStatus,
                Scenario::BuildSelectionDefensesDestination,
                Scenario::BuildSelectionFleetDestination,
                Scenario::FleetAssaultTaken,
                Scenario::BuildSelectionTraining,
                Scenario::TroopResearchTargeting,
                Scenario::SpecialForceEnroute,
                Scenario::RegimentEnroute,
                Scenario::SquadronEnroute,
                Scenario::FleetArrivals,
                Scenario::FleetJoinEnroute,
                Scenario::ResearchDiplomacyTargeting,
            ]
        );
    }

    fn finder_observation(
        applied: &Applied,
        finder: &FleetFinderState,
        faction: CockpitFaction,
    ) -> FleetFinderObservation {
        let player = if faction == CockpitFaction::Alliance {
            Faction::Alliance
        } else {
            Faction::Empire
        };
        fleet_finder_observation(
            request(Scenario::FleetFinder, faction),
            &applied.world,
            &FinderWindows {
                fog: &FogState::new(player),
                movement: &applied.movement,
                finder,
                fleets: &FleetWindowState::default(),
                sectors: &applied.sectors,
                map: &applied.map,
                left_panel_open: false,
            },
        )
        .unwrap()
    }

    // port: the fixture's own layout; the rows follow FUN_00462be0.
    #[test]
    fn the_fleet_finder_scenario_lists_a_second_own_fleet_and_the_other_sides_fleet_at_home() {
        for faction in [CockpitFaction::Alliance, CockpitFaction::Empire] {
            let applied = apply_to_fleet_world(request(Scenario::FleetFinder, faction));
            let world = &applied.world;
            let is_alliance = faction == CockpitFaction::Alliance;
            let keys: Vec<_> = world.systems.keys().take(10).collect();
            let (home, third) = (
                world.systems[keys[0]].dat_id.raw(),
                world.systems[keys[2]].dat_id.raw(),
            );
            assert_eq!(world.fleets.len(), 5);
            let mut finder = FleetFinderState::default();
            assert!(finder_observation(&applied, &finder, faction)
                .rows
                .is_empty());
            finder.open(faction);

            let observed = finder_observation(&applied, &finder, faction);

            assert!(observed.open);
            assert_eq!(observed.mode, Some("fleets"));
            assert_eq!(observed.tab, Some("all"));
            let row = |side: bool, system: u32| {
                observed
                    .rows
                    .iter()
                    .any(|row| row.is_alliance == side && row.system_dat_id == Some(system))
            };
            assert!(row(!is_alliance, home), "{faction:?}: {:?}", observed.rows);
            assert!(row(is_alliance, third), "{faction:?}: {:?}", observed.rows);
            let controls = observed.controls.unwrap();
            assert_eq!(controls.rows.len(), observed.rows.len());
            assert!(controls.rows.iter().all(Option::is_some));
            assert_eq!(observed.player_is_alliance, is_alliance);
            // FUN_00427270: the control's rectangle, (157, 407) 27 by 15 for
            // the Alliance and (199, 434) 37 by 24 for the Empire.
            let button = if is_alliance {
                (170.5, 414.5)
            } else {
                (217.5, 446.0)
            };
            assert_eq!(observed.cockpit_button, button);
            // The first fleet stays first; the other side's copy is the last.
            let fleet = |ordinal| observed.rows.iter().find(|row| row.fleet == Some(ordinal));
            let first = fleet(0).unwrap();
            assert_eq!(
                (first.kind, first.ship, first.is_alliance),
                ("fleet", None, is_alliance)
            );
            assert_eq!(fleet(4).map(|row| row.is_alliance), Some(!is_alliance));
        }
    }

    #[test]
    fn the_finder_observation_names_ships_and_the_fleet_windows_selection() {
        let faction = CockpitFaction::Alliance;
        let mut applied = apply_to_fleet_world(request(Scenario::FleetFinder, faction));
        let class = applied.world.capital_ship_classes.keys().next().unwrap();
        let first = applied.world.fleets.keys().next().unwrap();
        let home = applied.world.fleets[first].location;
        applied.world.fleets[first].capital_ships = vec![
            ShipInstance::new(class, 100, true),
            ShipInstance::new(class, 100, true),
        ];
        let mut finder = FleetFinderState::default();
        finder.open(faction);
        let layout = CockpitState::new(faction).layout_for(640.0, 480.0);
        let mut fleets = FleetWindowState::default();
        assert!(fleets.open(&applied.world, home, (100, 100), faction, layout));
        assert!(fleets.select(
            home,
            FleetWindowEntry::Ship {
                fleet: first,
                index: 1
            }
        ));
        let observe = |finder: &FleetFinderState| {
            fleet_finder_observation(
                request(Scenario::FleetFinder, faction),
                &applied.world,
                &FinderWindows {
                    fog: &FogState::new(Faction::Alliance),
                    movement: &applied.movement,
                    finder,
                    fleets: &fleets,
                    sectors: &applied.sectors,
                    map: &applied.map,
                    left_panel_open: true,
                },
            )
            .unwrap()
        };
        let observed = observe(&finder);

        let window = observed.fleet_windows.first().unwrap();
        assert_eq!(observed.fleet_windows.len(), 1);
        assert_eq!(
            window.system_dat_id,
            applied.world.systems[home].dat_id.raw()
        );
        assert_eq!(
            (window.selected, window.fleet, window.ship),
            (Some("ship"), Some(0), Some(1))
        );
        // FUN_004a2630: the Fleet window is 235 by 304, where it opened.
        let rect = fleets.screen_rect(layout, home).unwrap();
        assert_eq!(window.rect, Some((rect.min.x, rect.min.y, 235.0, 304.0)));
        assert!(observed.left_panel_open);

        finder.set_mode(FinderMode::Ships);
        let observed = observe(&finder);
        let row = observed
            .rows
            .iter()
            .find(|row| row.fleet == Some(0) && row.ship == Some(1));
        assert_eq!(row.map(|row| row.kind), Some("ship"));
    }

    #[test]
    fn the_finder_watch_reports_only_a_changed_finder() {
        let faction = CockpitFaction::Empire;
        let applied = apply_to_fleet_world(request(Scenario::FleetFinder, faction));
        let fog = FogState::new(Faction::Empire);
        let fleets = FleetWindowState::default();
        let mut finder = FleetFinderState::default();
        let mut watch = FleetFinderWatch::default();
        let next = |watch: &mut FleetFinderWatch, scenario, finder: &FleetFinderState| {
            watch.next(
                request(scenario, faction),
                &applied.world,
                &FinderWindows {
                    fog: &fog,
                    movement: &applied.movement,
                    finder,
                    fleets: &fleets,
                    sectors: &applied.sectors,
                    map: &applied.map,
                    left_panel_open: false,
                },
            )
        };

        let finder_scenario = Scenario::FleetFinder;
        assert!(next(&mut watch, finder_scenario, &finder).is_some_and(|observed| !observed.open));
        assert!(next(&mut watch, finder_scenario, &finder).is_none());
        finder.open(faction);
        assert!(next(&mut watch, finder_scenario, &finder).is_some_and(|observed| observed.open));
        assert!(next(&mut watch, finder_scenario, &finder).is_none());
        assert!(next(&mut watch, Scenario::FleetJoin, &finder).is_none());
    }

    #[test]
    fn the_watch_passes_each_new_observation_once() {
        let mut watch = FleetMoveWatch::default();
        let first = FleetMoveObservation {
            status: "fleet-move",
            code: 0,
            in_transit: false,
            destination_dat_id: None,
            confirmation_open: false,
            selected_system_dat_id: None,
            last_message: None,
            destination_entry: None,
            destination_item: None,
        };
        let second = FleetMoveObservation {
            in_transit: true,
            ..first.clone()
        };

        assert_eq!(watch.changed(Some(first.clone())), Some(&first));
        assert_eq!(watch.changed(Some(first.clone())), None);
        assert_eq!(watch.changed(None), None);
        assert_eq!(watch.changed(Some(second.clone())), Some(&second));
    }

    #[test]
    fn a_system_point_matches_the_original_imperial_frame() {
        // FUN_00422ce0 neither zooms nor pans: the map keeps one framing.
        // Squakenet frame 0712 (Imperial cockpit) shows system 0x121,
        // DAT (747, 490), at canvas (535.5, 244.5) within 2 px.
        let world = diplomacy_world(Faction::Alliance);
        let mut system = world.systems.values().next().unwrap().clone();
        system.x = 747;
        system.y = 490;

        let (x, y) = screen_point(&system, CockpitFaction::Empire);
        assert!((x - 535.5).abs() < 2.0, "x = {x}");
        assert!((y - 244.5).abs() < 2.0, "y = {y}");
    }

    #[test]
    fn fixture_fingerprint_is_stable() {
        assert_eq!(
            fnv1a64(b"gid/alliance/popular-support"),
            0x432f_ad5e_fe03_453d
        );
    }
}
