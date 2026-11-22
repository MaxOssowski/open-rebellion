//! Test-only deterministic interface fixture bridge.
//!
//! This module is compiled for the dedicated WASM acceptance artifact and
//! native unit tests. Production builds contain no fixture bridge.

use rebellion_core::blockade::{BlockadeState, BlockadeSystem};
use rebellion_core::dat::{ExplorationStatus, Faction};
use rebellion_core::economy::EconomyState;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{CharacterKey, FleetKey, SystemKey, TroopKey};
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
const SCENARIO_COUNT: u8 = 161;


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
    /// window: the ships band's Destination released on the fleet takes it,
    /// the facilities band's is refused (`FUN_00537180`: only a
    /// construction manager, `0xa0..0xa1`, wants a system).
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
    /// As `Quadrants` with the player's fleet at rest at the explored
    /// neutral system, its one ship of the player's first class that
    /// bombards, and one of the player's squadrons on the primary planet:
    /// the fleet icon's drag and its menu at a neutral system, as A0
    /// 2026-11-13-icon-drag-move/20..22 and 2026-11-13-hyperspace-icons/00
    /// (Kubindi), and a squadron's Move onto that neutral planet, refused
    /// (A0 icon-drag-move/04, 05, 08, 09).
    QuadrantsNeutralFleet = 116,
    /// As `Quadrants` with the player's agent at the primary system free,
    /// not on a mission: the defenses icon's team (the agent and the
    /// regiment, `FUN_00512700` kind 8) may move, and the battery stays
    /// behind it, as A0 2026-11-13-defenses-missions-drop/00..06 and 21
    /// drop Jerijador's defenses icon on Yavin.
    QuadrantsFreeTeam = 117,
    /// As `Quadrants` with a shipyard of the player's at the primary
    /// system, so its facility icon has managers (`FUN_00512700` kind 4):
    /// the icon's drop and its menu's Destination, as A0
    /// 2026-11-13-facility-icon-drop drops Jerijador's facility icon on
    /// its own planet, on Yavin's HQ mark and on Yavin's planet edge.
    QuadrantsFacility = 118,
    /// `BuildSelection`'s yards with the primary system's energy taken:
    /// its facilities and one Construction Yard queued on its own
    /// facilities band fill every slot, and its mines every raw deposit.
    /// An order on that band gets the queued unit's slot back
    /// (`FUN_00552210`), so one non-mine facility fits and two do not
    /// (`FUN_00507750`, `{0x90, 1}`); a mine finds no deposit (`{0x90, 2}`),
    /// as A0 2026-11-13-build-confirm/21, 23 grey Confirm at Jerijador.
    BuildSelectionFacilityRoom = 119,
    /// As `ResearchTargeting` with a facility designer for the agent, one
    /// construction yard the target's only yard, and the target below full
    /// support: Facility Design Research, Diplomacy and Espionage list, in
    /// `FUN_0046cab0`'s key order (A0 2026-11-14-missions-transfer/11..14:
    /// Adar Tallon on his own Jerijador, a construction yard alone, shows
    /// Facility Design Research first of three kinds).
    FacilityResearchTargeting = 120,
    /// The Alliance's own failed Planetary Assault, as A0
    /// 2026-11-14-bombard-assault/09..12 shows it (a new Alliance game,
    /// day 148): Fleet 1 with one Alliance Fleet Regiment aboard lands on
    /// Imperial Commenor, held by five Imperial Army Regiments; the
    /// regiment is destroyed, none of the defenders, and the summary opens
    /// at once (`FUN_00497530` kind 7) on "Imperial Troops have defended
    /// Commenor from an Alliance assault." (`0x7092`).
    OwnAssaultRepulsed = 121,
    /// OBJ-02-C010, as A0 2026-11-14-quadrants-status-known/18 shows it
    /// (an Alliance game, day 576): the second system, the other side's,
    /// on its Defenses window's Trooper Regiments page, lists the player's
    /// frozen copy of the other side's Army Regiment, en route with ETA day
    /// 171 (`FUN_004f9a60` froze it in hyperspace). The live regiment is
    /// gone from the player's sight.
    EnemyRegimentFrozen = 122,
    /// OBJ-02-C010, as A0 2026-11-14-stale-views shows Kothlis (an
    /// Alliance game; the Alliance saw the Empire's Kothlis on day 371 and
    /// left): `Quadrants` with the second system, the other side's, frozen
    /// in the player's copies as seen then (07-kothlis-def-d374-montage,
    /// 07-kothlis-fac-d374-montage): two Stormtrooper and one Army
    /// regiment, six TIE Fighter squadrons, one shipyard, five refineries
    /// and five mines; and a fleet made there since, copied with its name
    /// only (06-kothlis-fleet16-d374-montage). The live system has since
    /// lost them all.
    QuadrantsStale = 123,
    /// `QuadrantsStale` with the debug "see everything" mode on: the
    /// windows show the live system.
    QuadrantsStaleSeeEverything = 124,
    /// `EnemyRegimentFrozen` with the copy at rest on its planet: A0
    /// 2026-11-14-stale-views/07-kothlis-reg1-status-d375 reads
    /// "Attached Kothlis, Awaiting Orders" with no ETA.
    EnemyRegimentAtRest = 125,
    /// The Personnel Finder's copied characters, as A0
    /// 2026-11-14-stale-views (second run) shows them on the Alliance's
    /// Imperial tab: one of the other side's characters aboard a ship of a
    /// fleet only the player's frozen copy of a system holds reads "Name -
    /// <ship>" (1x-personnelfinder-empire-d85, sha256 d57ed845); another,
    /// whose copy a live rebuild left in no place, "Name - Location
    /// Unknown" (d91 1b8818d8). The live objects are out of the player's
    /// sight.
    PersonnelCopied = 126,
    /// A0 2026-11-15 "m-1" (d31, sha256 d59ff334): the Empire's Fleet
    /// window over Corfai, the yard's spare holding only the unfinished
    /// Carrack Light Cruiser, its mini over the side's grid.
    UnfinishedShipMini = 127,
    /// A0 2026-11-15 "m-4" (d115, sha256 0d4174bd): the Carrack finished
    /// at Corfai while it stood in Talus's Fleet 3, on its trip there: the
    /// en route mark, no grid.
    UnfinishedShipEnroute = 128,
    /// As `FleetLoad`, with a regiment of the regiment's class also
    /// ordered at the primary system: it waits unfinished on the planet,
    /// the Defenses window's second row (A0 2026-11-15 "m-5").
    FleetLoadUnfinished = 129,
    /// As `FleetLoad`, the fleet's ship whole but for one hit against its
    /// shield recharge: for the Empire, Victory Destroyer 2, as A0
    /// 2026-11-15-partial-cells e-vd2 (8bce5f1cbf27) reads it, "Ship
    /// Damaged: Yes", Hull 1800:1800, Shield Recharge Rate 8:10.
    ShipShieldHit = 130,
    /// A0 2026-11-16-partial-cells/f-1..f-5 (an Empire game, day 304..318,
    /// OBJ-10-C006): Coruscant's Fleet 1, Imperial Star Destroyer 1 and
    /// Victory Destroyer 1, each carrying TIE Fighter squadrons and
    /// Stormtrooper Regiments, with the Imperial Probe Droid aboard the
    /// Star Destroyer; its Fleet window open at A0's (365, 92), nothing
    /// selected.
    SpecialForceAboard = 131,
    /// As `EnemyAssaultTaken` with Yavin the Alliance's headquarters (the
    /// HQ object, ALLFACSD `0x20000001`, on it) and Sumitra's sector
    /// window open: A0 2026-11-16-partial-cells/v-17..v-20 (an Alliance
    /// game, day 1268..1716, OBJ-13-C007). The report is filed on the
    /// Conflict rail and not opened, as A0 read it from the index (v-18).
    HeadquartersTaken = 132,
    /// The Empire's Popular Support rail with three Uprising messages
    /// (`FUN_00499460`, notification `0x14b`) filed, as A0 2026-11-16-
    /// empire-hunt (slot 5, an Empire game, day 153..368) lists them:
    /// "Uprising Begins on Selonia" on Imperial Selonia (h-30, h-37, h-38),
    /// "Uprising Ends on Selonia" after Selonia left the Empire (h-33, h-34)
    /// and "Uprising Begins on Yavin" on Imperial Yavin (h-49, h-50). None
    /// is opened (EVT-01-C005).
    UprisingMessages = 133,
    /// As `ShipShieldHit` with an Orbital Shipyard of the player's side at
    /// the fleet's system, the days run until the port's repair
    /// (`FUN_00501c50`, FastRepair `FUN_0050c580` / `FUN_0050bb70`) has
    /// mended the hit: A0 2026-11-16-empire-hunt h-28 / h-29 (an Empire
    /// game, day 634): Victory Destroyer 2 at Coruscant reads "Ship
    /// Damaged: No", Shield Recharge Rate 10:10 (OBJ-06-C012).
    ShipRepaired = 134,
    /// OBJ-11-C010, as A0 2026-11-16-edited-saves/x-54..x-57 shows it (an
    /// Alliance game, day 813, Luke's Force edited to 100): Luke Skywalker,
    /// a Jedi Knight (MJCHARSD `is_jedi_trainer`, Force at GNPRTB 2581),
    /// and Jan Dodonna, with some Force below his, both free on Yavin; the
    /// mission dialog opened for the two on Yavin, as their Mission order
    /// released on it would (`FUN_0042a320`), its first kind Jedi Training.
    JediTrainingDialog = 135,
    /// EVT-01-C015, as A0 2026-11-16-edited-saves/x-58..x-60 reads it: the
    /// world of `JediTrainingDialog` with Luke's Jedi Training on Yavin
    /// ended a success, Dodonna's Force grown from Novice to Trainee
    /// (`FUN_005712b0`): "Jan Dodonna Force Growth" (0x1e1) and "Luke
    /// Skywalker Mission Report" filed on the Mission rail, none opened.
    JediTrainingMessages = 136,
    /// OBJ-13-C002, as A0 2026-11-16-edited-saves/x-29 (an Empire game, day
    /// 527: the HQ mark over Duros) and x-39 (Tokmia under the Empire's
    /// blockade) show it: `Quadrants` with the Empire's copy of the
    /// Alliance's headquarters system frozen while the headquarters stood
    /// there (`FUN_004f9a60`); `FUN_0045bbb0` marks it from that copy
    /// (`FUN_004f3220`, `FUN_00526cf0`). For the Alliance it is `Quadrants`.
    QuadrantsHeadquartersFrozen = 137,
    /// EVT-02-C008: as `FleetLoad` with a fleet of the other side at the
    /// primary system holding one capital ship whose class projects a
    /// gravity well (CAPSHPSD `gravity_well_projector`: the Empire's
    /// Interdictor Cruiser `0x82`, the Alliance's `0x4a`). A running clock
    /// starts a battle there, and the Battle Alert's Retreat is disabled
    /// (`FUN_00516730`, `retreat_offered`), as A0
    /// 2026-11-17-edited-saves-4/d-01 shows it for the Alliance.
    GravityWellBattle = 138,
    /// EVT-01-C019: the Empire player's own General Bombardment of the
    /// Alliance's Ghorman, as A0 2026-11-17-edited-saves-5/i-01..i-05
    /// shows it (a new, unedited Empire game, day 33): Fleet 1, an
    /// Imperial Star Destroyer and an Imperial Dreadnaught, strikes; no
    /// side loses a unit, and the summary opens at once (`FUN_00497530`
    /// kind 7) on "Orbital bombardment of Ghorman" over `0x2b9a`, filed on
    /// the Conflict rail. Sesswenna's sector window is open behind it.
    GhormanBombardment = 139,
    /// EVT-01-C019: the same Fleet 1, carrying a Stormtrooper Regiment,
    /// assaults the undefended Ghorman and takes it, as A0
    /// 2026-11-17-edited-saves-5/i-06 and i-07 show it: the Empire's own
    /// Assault Summary opens at once on "Assault on Ghorman" / "Imperial
    /// troops have taken control of the Alliance system Ghorman"
    /// (TEXTSTRA `0x7090` / `0x7091`, `FUN_00496770`), the System Forces'
    /// regiments "None" and "No Casualties".
    GhormanAssault = 140,
    /// OBJ-03-C011, as A0 2026-11-17-edited-saves-6 shows it. For the
    /// Alliance, d-01 (day 368, an Alliance game): the side's eight
    /// characters, "Chewbacca - Yavin", "Han Solo - Sluis Van ( Captured
    /// )", "Huoba Neva - Corsin", "Jan Dodonna - Yavin", "Leia Organa -
    /// Yavin", "Luke Skywalker - Dagobah" (in the Dagobah abode
    /// `0x98000441`), "Mon Mothma - Chazwa", "Wedge Antilles - Yavin". For
    /// the Empire, b-01 (day 411): Luke in the abode and the Empire's
    /// copies of the eight Alliance characters b-01 lists, Luke's in no
    /// place ("Luke Skywalker - Location Unknown"). Every other character
    /// of the Alliance is not recruited.
    PersonnelAbode = 141,
    /// EVT-01-C019: a character capture as both sides read it, as A0
    /// 2026-11-17-edited-saves-6 shows it. The Alliance's Mission
    /// messages hold d-07's newest rows (Corfai and Vagran Joins Enemy,
    /// Luke Goes to Dagobah, Enemy Mission Foiled, Han Solo Captured, Han
    /// Solo Injured, Enemy Mission Foiled; d-08..d-11); the Empire's hold
    /// Leia Organa taken by the Empire at Yavin (e-03, e-04). See
    /// [`post_capture_messages`].
    CaptureMessages = 142,
    /// CMD-02-C024: Idle Fleets with one fleet of each side in orbit at
    /// the primary system, which the player sees through its own fleet:
    /// the mark is `0x27b0`, STRATEGY 10160 (`FUN_0042b330` case `0x21`;
    /// `ghidra/notes/gid-fleet-marks.md`).
    FleetsBothSides = 143,
    /// EVT-01-C018: the Alliance's Personnel Finder after Jabba delivered
    /// his prisoners, as A0 2026-11-18-edited-saves-7 b-19 (e4f53d49b065,
    /// day 31) shows it: "Chewbacca", "Han Solo", "Leia Organa" and "Luke
    /// Skywalker - Jabba's Palace" (the Alliance's copies, frozen in the
    /// palace; `FUN_004eec60`, `FUN_00465bb0`'s abode rule) while the
    /// prisoners stand at Coruscant, beside "Huoba Neva - Corsin", "Jan
    /// Dodonna - Yavin", "Mon Mothma - Chazwa", "Wedge Antilles - Yavin".
    /// See [`place_palace_prisoners`].
    PersonnelPalace = 144,
    /// OBJ-03-C011: the Empire's Personnel Finder with a retired
    /// character, as A0 2026-11-18-edited-saves-9 d-03 (b9ea91b80dbb, an
    /// unedited Empire game, day 132) shows it: "Ozzel - Retired" (0x8799,
    /// `FUN_00465bb0`) sixth of eight, beside "Darth Vader - Yavin ( On
    /// Mission ) ", "Emperor Palpatine - Coruscant", "Menndo - Corfai" and
    /// "Veers - Coruscant". port: Jerjerrod, Needa and Piett, aboard ships
    /// in A0, stand at Coruscant here. See [`place_retired_roster`].
    PersonnelRetired = 145,
    /// EVT-01-C018: the Empire's Personnel Finder, Alliance tab, right
    /// after Jabba delivered his prisoners, as A0
    /// 2026-11-18-edited-saves-9 c-14 (88e1928e1f1a, day 531) shows it:
    /// "Han Solo", "Leia Organa" and "Luke Skywalker - Victory Destroyer 1
    /// ( Captured ) " (aboard the ship Vader joined the Pickup from,
    /// `FUN_0056da80`), "Chewbacca - Yavin" (killed, read from the
    /// Empire's frozen copy: `FUN_005445d0` accepts him destroyed), "Jan
    /// Dodonna - Location Unknown", "Ma'w'shiye - Yavin ( Enroute ) ",
    /// "Tura Raftican - Yavin" and "Wedge Antilles - Yavin". See
    /// [`place_pickup_roster`].
    PersonnelPickup = 146,
    /// OBJ-13-C001, the hidden headquarters: `Quadrants` with the Empire's
    /// fog showing the Alliance's headquarters system (as a passing fleet's
    /// sensor reach or a half-way reveal would) but no copy of it. The
    /// Empire's setup copies hold no facility (`FUN_005136d0` stage `0x18`
    /// -> `FUN_0051b990`), so `FUN_0045bbb0` finds no family `0x20` child
    /// and draws no 904 (manual p. 67: "The Empire does not know the
    /// location of Alliance headquarters"). For the Alliance it is
    /// `Quadrants`.
    QuadrantsHeadquartersHidden = 147,
    /// OBJ-13-C001, the headquarters found: `Quadrants` with the Empire's
    /// copy of the headquarters system live, as an Imperial fleet in orbit
    /// makes it (`FUN_004f7c70` -> `FUN_0050ac80`, `FUN_004f9a60`); 904 over
    /// the planet as A0 2026-11-16-edited-saves/x-39 shows it over Tokmia
    /// under the Empire's blockade. port: the live copy alone, no fleet.
    /// For the Alliance it is `Quadrants`.
    QuadrantsHeadquartersLive = 148,
    /// CMD-03-C018: `QuadrantsFreeTeam` with a regiment of the player's
    /// queued at a yard in another sector for the primary: an unfinished product
    /// in the primary itself (`FUN_0052a140` -> `FUN_00507750`), which
    /// leaves the defenses icon's Move white and greys its Scrap
    /// (`FUN_004f9860` status 2), as A0 2026-11-19-edited-saves-10 c-01.
    QuadrantsRegimentQueued = 149,
    /// CMD-03-C018: as `QuadrantsRegimentQueued` with the regiment trained
    /// and on its way in (`FUN_0052bee0`, status 4 en route): Move,
    /// Confirmed Move and Scrap grey, as A0 2026-11-19-edited-saves-10 c-05.
    QuadrantsRegimentEnRoute = 150,
    /// EVT-01-C018: Jabba Delivers Prisoners (`FUN_0048b6e0`) alone under
    /// the Mission rail, its background chosen by Luke's (`0x32000242`)
    /// DagobahCompleted as the Pickup reads it. The Empire's Luke has
    /// left Dagobah (0x42b, as A0 2026-11-18-edited-saves-9 c-13 shows
    /// it); the Alliance's has not (0x42a, A0 2026-11-18-edited-saves-7
    /// b-12). See [`post_jabba_delivers`].
    JabbaDelivers = 151,
    /// Running the blockade: `FleetLoad` with the primary system the other
    /// side's and the player's fleet blockading it, two regiments aboard,
    /// each holding the primary's withdraw percent, which the fleet's ships
    /// bring to 0 (`FUN_0055a020`, no KDY-150 there, `FUN_0050b310`). A
    /// regiment that leaves on its own is lost (`FUN_00504990`, event
    /// `0x340`, Evacuation Losses `FUN_00495e90`); one aboard the departing
    /// fleet is not rolled. See [`place_blockade_running`].
    BlockadeRunning = 152,
    /// `CharacterMove` with the other side's fleet alone in orbit of the
    /// player's primary system, which it blockades: the agent's Confirmed
    /// Move asks with the evacuation warning (`FUN_0049a350`, picture 1031
    /// or 1030).
    CharacterEvacuation = 153,
    /// `SquadronLoad` with the loading target, the player's, blockaded by
    /// the other side's fleet alone in its orbit: a planet squadron's Move
    /// there is refused (`FUN_005073d0`, `0x90`/4).
    SquadronBlockaded = 154,
    /// The fog of war at a new game: the seeded galaxy as a new game leaves
    /// it, with the setup push (`FUN_0051b990`): full copies of the core
    /// systems, bare ones of the Rim systems, and the Empire's level-10 push
    /// of Yavin (`Event319_Handle(10, 2)`), on Popular Support, as A0
    /// 2026-11-19-fog-13 e-02 (the Empire's) and fog-14 a-01 (the
    /// Alliance's) show the galaxy. See [`place_new_game`].
    NewGame = 155,
    /// `NewGame` with the player's first fleet in hyperspace to Triton
    /// (a Rim system in Mayagil): not usable in flight, so it sees nothing
    /// there (`FUN_0050ac80`); A0 fog-14 e-05.
    TritonEnroute = 156,
    /// `NewGame` with the player's first fleet in orbit of Triton: its copy
    /// is live (`FUN_004f9a60`) and explored; A0 fog-14 e-06.
    TritonArrived = 157,
    /// `TritonArrived`, then the fleet gone on to Kabal and Triton's own
    /// energy and raw materials changed: the frozen copy keeps the values
    /// it had when the fleet left; A0 fog-14 e-13, e-28.
    TritonStale = 158,
    /// `FleetAssaultTaken` against Coruscant (`0x90000109`) in a
    /// Headquarters Victory game (galaxy `+0x60` = 1) whose Alliance
    /// headquarters stands elsewhere, with the player's advisor: the
    /// assault takes Coruscant, so the victory conditions word
    /// (`FUN_005180e0`) ends the game, as in A0 2026-11-17-edited-saves-5
    /// run 2 (caption film 103, then 105 and the main menu).
    EndingAssaultCoruscant = 159,
    /// `CharacterMove` with a character of the other side held captive by
    /// the player's side at the primary system, after the agent on the
    /// personnel page. Moved alone its Move is grey (`FUN_0054bb90`
    /// `0x40`/`0x91`); selected with the agent, its jailer, the pair moves
    /// (manual p. 96). See [`place_captive`].
    CaptiveTransport = 160,
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
            116 => Self::QuadrantsNeutralFleet,
            117 => Self::QuadrantsFreeTeam,
            118 => Self::QuadrantsFacility,
            119 => Self::BuildSelectionFacilityRoom,
            120 => Self::FacilityResearchTargeting,
            121 => Self::OwnAssaultRepulsed,
            122 => Self::EnemyRegimentFrozen,
            123 => Self::QuadrantsStale,
            124 => Self::QuadrantsStaleSeeEverything,
            125 => Self::EnemyRegimentAtRest,
            126 => Self::PersonnelCopied,
            127 => Self::UnfinishedShipMini,
            128 => Self::UnfinishedShipEnroute,
            129 => Self::FleetLoadUnfinished,
            130 => Self::ShipShieldHit,
            131 => Self::SpecialForceAboard,
            132 => Self::HeadquartersTaken,
            133 => Self::UprisingMessages,
            134 => Self::ShipRepaired,
            135 => Self::JediTrainingDialog,
            136 => Self::JediTrainingMessages,
            137 => Self::QuadrantsHeadquartersFrozen,
            138 => Self::GravityWellBattle,
            139 => Self::GhormanBombardment,
            140 => Self::GhormanAssault,
            141 => Self::PersonnelAbode,
            142 => Self::CaptureMessages,
            143 => Self::FleetsBothSides,
            144 => Self::PersonnelPalace,
            145 => Self::PersonnelRetired,
            146 => Self::PersonnelPickup,
            147 => Self::QuadrantsHeadquartersHidden,
            148 => Self::QuadrantsHeadquartersLive,
            149 => Self::QuadrantsRegimentQueued,
            150 => Self::QuadrantsRegimentEnRoute,
            151 => Self::JabbaDelivers,
            152 => Self::BlockadeRunning,
            153 => Self::CharacterEvacuation,
            154 => Self::SquadronBlockaded,
            155 => Self::NewGame,
            156 => Self::TritonEnroute,
            157 => Self::TritonArrived,
            158 => Self::TritonStale,
            159 => Self::EndingAssaultCoruscant,
            160 => Self::CaptiveTransport,
            _ => return None,
        })
    }

    /// The scenarios that keep the seeded galaxy as a new game leaves it
    /// ([`place_new_game`]).
    fn is_new_game(self) -> bool {
        matches!(
            self,
            Self::NewGame | Self::TritonEnroute | Self::TritonArrived | Self::TritonStale
        )
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
                | Self::GravityWellBattle
                | Self::FleetBombardment
                | Self::FleetAssault
                | Self::FleetAssaultTaken
                | Self::EndingAssaultCoruscant
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
                | Self::FleetLoadUnfinished
                | Self::ShipShieldHit
                | Self::ShipRepaired
                | Self::BlockadeRunning
                | Self::SquadronBlockaded
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
                | Self::BuildSelectionFacilityRoom
        )
    }

    /// The scenarios whose gate opens the object pop-up menu.
    fn reports_object_menu(self) -> bool {
        self == Self::MissionTargeting
            || self == Self::ForceStatus
            || self == Self::CharacterCommand
            || self == Self::CharacterMove
            || self == Self::CharacterEvacuation
            || self == Self::CaptiveTransport
            || self == Self::CharacterBoard
            || self == Self::CharacterAboard
            || self == Self::ReconTargeting
            || self == Self::SpecialForceEnroute
            || self == Self::RegimentEnroute
            || self == Self::SquadronEnroute
            || self == Self::ResearchTargeting
            || self == Self::TroopResearchTargeting
            || self == Self::FacilityResearchTargeting
            || self == Self::ResearchDiplomacyTargeting
            || self == Self::UprisingTargeting
            || self == Self::BatteryStatus
            || self == Self::PersonnelStates
            || self == Self::PersonnelAbode
            || self == Self::PersonnelRetired
            || self.targets_an_object()
            || self.opens_build_selection()
            || self == Self::Quadrants
            || self == Self::QuadrantsNeutralFleet
            || self == Self::QuadrantsFreeTeam
            || self == Self::QuadrantsRegimentQueued
            || self == Self::QuadrantsRegimentEnRoute
            || self == Self::QuadrantsFacility
            || self.moves_a_fleet()
            || self.uses_the_fleet_window()
            || self == Self::FleetWindowYavin
            || self == Self::FleetWindowA0
            || self == Self::SpecialForceAboard
            || self == Self::EnemyRegimentFrozen
            || self == Self::EnemyRegimentAtRest
            || self == Self::QuadrantsStale
            || self == Self::QuadrantsStaleSeeEverything
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
            Self::Fleets | Self::FleetsBothSides => GidMode::IdleFleets,
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
        name: None,
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
    if request.scenario.is_new_game() {
        return;
    }

    // The Coruscant assault's primary system (the first, which every
    // observer reads) stands as Coruscant: it takes Coruscant's key
    // 0x90000109, by which FUN_005180e0 reads it, and its name, and gives
    // its own to the system that held them.
    if request.scenario == Scenario::EndingAssaultCoruscant {
        let first = world.systems.keys().next();
        if let (Some(first), Some(coruscant)) = (first, coruscant_key(world)) {
            if first != coruscant {
                let (id, name) = (
                    world.systems[coruscant].dat_id,
                    world.systems[coruscant].name.clone(),
                );
                let (own_id, own_name) = (
                    world.systems[first].dat_id,
                    world.systems[first].name.clone(),
                );
                (world.systems[first].dat_id, world.systems[first].name) = (id, name);
                (
                    world.systems[coruscant].dat_id,
                    world.systems[coruscant].name,
                ) = (own_id, own_name);
            }
        }
    }
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
        Scenario::CharacterMove | Scenario::CharacterEvacuation | Scenario::CaptiveTransport => {
            world.systems[secondary].control = ControlKind::Controlled(player_control);
        }
        _ => {}
    }

    // The Alliance's Rescue matches A0 2026-11-15-partial-cells/rescue-
    // dialog-han-on-leia-fleet90-d1428 (sha256 44c8c052b951): Han Solo the
    // agent, Leia Organa the prisoner. The scenario takes the first
    // character as its agent and the second as its prisoner, so the two
    // trade places with them. The Alliance's Death Star Sabotage matches
    // A0 2026-11-16-edited-saves/x-45..x-50 (Han Solo sent from Yavin), so
    // Han is its agent too.
    let death_star = request.scenario == Scenario::DeathStarTargeting;
    if (request.scenario == Scenario::RescueTargeting || death_star) && player_is_alliance {
        let named = |world: &GameWorld, name: &str| {
            world
                .characters
                .iter()
                .find(|(_, character)| character.name == name)
                .map(|(key, _)| key)
        };
        let cast: &[(usize, &str)] = if death_star {
            &[(0, "Han Solo")]
        } else {
            &[(0, "Han Solo"), (1, "Leia Organa")]
        };
        for &(index, name) in cast {
            let slot = world.characters.keys().nth(index);
            if let (Some(slot), Some(found)) = (slot, named(world, name)) {
                swap_characters(world, slot, found);
            }
        }
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
                    | Scenario::CharacterEvacuation
                    | Scenario::CaptiveTransport
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
                | Scenario::FacilityResearchTargeting
                | Scenario::ResearchDiplomacyTargeting
                | Scenario::UprisingTargeting
                | Scenario::CharacterCommand
                | Scenario::CharacterMove
                | Scenario::CharacterEvacuation
                | Scenario::CaptiveTransport
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
    if request.scenario == Scenario::CaptiveTransport {
        place_captive(world, agent, primary, player_is_alliance);
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

    // The evacuation variant: the first fleet turns the other side's and,
    // alone at the primary system with no squadron on its planet, blockades
    // it (`FUN_0050b8e0`; a squadron would make it a battle, `FUN_00509710`).
    if request.scenario == Scenario::CharacterEvacuation {
        let away = *system_keys.last().unwrap_or(&secondary);
        for (key, fleet) in &mut world.fleets {
            if Some(key) == transit_fleet {
                fleet.is_alliance = !player_is_alliance;
            } else if fleet.location == primary {
                fleet.location = away;
            }
        }
        world.systems[primary].fighters.clear();
        world.systems[primary].control = ControlKind::Controlled(player_control);
        reconcile_fleet_orbits(movement, world);
        let _ = BlockadeSystem::advance(blockade, world, &[TickEvent { tick: 1 }]);
    }

    if request.scenario == Scenario::FleetsBothSides {
        // An enemy fleet in orbit beside the player's own (gid-fleet-marks.md):
        // the player's fleet makes the system seen (FUN_0050ac80), so its
        // copy of the enemy fleet is live and both count.
        if let Some(mut enemy) = transit_fleet.and_then(|key| world.fleets.get(key)).cloned() {
            enemy.is_alliance = !player_is_alliance;
            enemy.location = primary;
            enemy.characters.clear();
            enemy.has_death_star = false;
            world.fleets.insert(enemy);
        }
        reconcile_fleet_orbits(movement, world);
    }

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
    if matches!(
        request.scenario,
        Scenario::BuildSelectionOverMaintenance | Scenario::BuildSelectionFacilityRoom
    ) {
        place_facility_room(request, world, manufacturing, primary);
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
            | Scenario::BuildSelectionFacilityRoom
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
        if matches!(
            request.scenario,
            Scenario::ShipShieldHit | Scenario::ShipRepaired
        ) {
            hit_the_shield_recharge(request, world);
        }
        if request.scenario == Scenario::ShipRepaired {
            repair_at_a_shipyard(request, world);
        }
        if request.scenario == Scenario::FleetLoadUnfinished {
            if let Some(regiment) = loading_regiment(world, primary) {
                let class = world.troops[regiment].class_dat_id;
                manufacturing.enqueue(
                    primary,
                    rebellion_core::manufacturing::QueueItem::new(
                        rebellion_core::manufacturing::BuildableKind::Troop(class),
                        12,
                    ),
                );
                rebellion_core::products::reconcile(world, manufacturing);
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
    if request.scenario == Scenario::SpecialForceAboard {
        place_probe_droid_aboard(request, world, cockpit, movement, troop_transport, fleets);
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
    if matches!(
        request.scenario,
        Scenario::EnemyAssaultTaken | Scenario::HeadquartersTaken
    ) {
        place_enemy_assault(world, movement, troop_transport);
    }
    if request.scenario == Scenario::HeadquartersTaken {
        place_headquarters_at_yavin(request, world, sectors);
    }
    if matches!(
        request.scenario,
        Scenario::GhormanBombardment | Scenario::GhormanAssault
    ) {
        place_ghorman_bombardment(request, world, movement, sectors);
    }
    if request.scenario == Scenario::GhormanAssault {
        embark_ghorman_regiment(world, troop_transport);
    }
    if request.scenario == Scenario::OwnAssaultRepulsed {
        place_own_assault(world, movement, troop_transport);
    }
    if request.scenario == Scenario::EnemyRegimentFrozen {
        place_frozen_enemy_regiment(request, world, defenses, cockpit, secondary, Some(FROZEN_ARRIVAL));
    }
    if request.scenario == Scenario::EnemyRegimentAtRest {
        place_frozen_enemy_regiment(request, world, defenses, cockpit, secondary, None);
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
    if matches!(
        request.scenario,
        Scenario::UnfinishedShipMini | Scenario::UnfinishedShipEnroute
    ) {
        place_unfinished_ship(
            request,
            world,
            manufacturing,
            cockpit,
            fleets,
            request.scenario == Scenario::UnfinishedShipEnroute,
        );
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
    if request.scenario == Scenario::BlockadeRunning {
        place_blockade_running(request, world, blockade, troop_transport, primary);
    }
    if request.scenario == Scenario::SquadronBlockaded {
        place_blockaded_target(request, world, movement, blockade, primary);
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
    if matches!(
        request.scenario,
        Scenario::Quadrants
            | Scenario::QuadrantsNeutralFleet
            | Scenario::QuadrantsFreeTeam
            | Scenario::QuadrantsRegimentQueued
            | Scenario::QuadrantsRegimentEnRoute
            | Scenario::QuadrantsHeadquartersFrozen
            | Scenario::QuadrantsHeadquartersHidden
            | Scenario::QuadrantsHeadquartersLive
            | Scenario::QuadrantsFacility
            | Scenario::QuadrantsStale
            | Scenario::QuadrantsStaleSeeEverything
    ) {
        place_quadrant_contents(
            request, world, movement, missions, uprisings, sectors, primary,
        );
    }
    if matches!(
        request.scenario,
        Scenario::QuadrantsRegimentQueued | Scenario::QuadrantsRegimentEnRoute
    ) {
        place_regiment_for_primary(
            request,
            world,
            manufacturing,
            primary,
            request.scenario == Scenario::QuadrantsRegimentEnRoute,
        );
    }
    if matches!(
        request.scenario,
        Scenario::QuadrantsHeadquartersFrozen | Scenario::QuadrantsHeadquartersLive
    ) && !player_is_alliance
    {
        // The Empire's copy of the headquarters system with the
        // headquarters object in it (FUN_004f9a60): frozen, or live.
        if let Some(&headquarters) = mark_systems(world, primary).first() {
            let view = rebellion_core::object_views::SystemView::of(
                world,
                &world.systems[headquarters],
                request.scenario == Scenario::QuadrantsHeadquartersLive,
            );
            let id = world.systems[headquarters].dat_id;
            world.object_views.empire_systems.insert(id, view);
        }
    }
    if matches!(
        request.scenario,
        Scenario::QuadrantsStale | Scenario::QuadrantsStaleSeeEverything
    ) {
        if let Some(second) = loading_target(world, primary) {
            place_stale_second(request, world, movement, missions, troop_transport, second);
        }
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
        // The side's recruited characters first, then others it recruits
        // here, up to eight: the Empire's galaxy starts with three.
        let mut ranked: Vec<_> = world
            .characters
            .iter()
            .filter(|(_, character)| {
                character.is_alliance == player_is_alliance && !character.is_killed
            })
            .map(|(key, character)| (!character.recruited, key))
            .collect();
        ranked.sort();
        let own: Vec<_> = ranked.into_iter().map(|(_, key)| key).take(8).collect();
        // One recruited here and standing nowhere stands on a system of
        // the side's, as a recruit does.
        let home = world
            .systems
            .iter()
            .find(|(_, value)| value.control.is_controlled_by(player_control))
            .map_or(primary, |(key, _)| key);
        for &key in &own {
            let character = &mut world.characters[key];
            character.recruited = true;
            if character.current_system.is_none()
                && character.current_fleet.is_none()
                && character.current_abode.is_none()
            {
                character.current_system = Some(home);
            }
        }
        // An eighth character, where the side has one, rests in an abode:
        // its row names the abode alone, with no state word
        // (FUN_00465bb0, container family 0x98..0x9f).
        if let (Some(&resting), Some(abode)) = (own.get(7), world.abodes.keys().next()) {
            let character = &mut world.characters[resting];
            character.current_abode = Some(abode);
            character.current_system = None;
            character.current_fleet = None;
            character.ship = None;
            character.on_mission = false;
            character.is_captive = false;
            character.injury = 0;
            character.command = rebellion_core::character_command::CommandKind::None;
            for fleet in world.fleets.values_mut() {
                fleet.characters.retain(|&key| key != resting);
            }
        }
        // A seventh character, where the side has one, retired: its row
        // reads "Name - Retired" (FUN_00465bb0, destruction reason
        // 0x14..0x16, word 0x8799; FUN_00534c00).
        if let Some(&retired) = own.get(6) {
            let character = &mut world.characters[retired];
            character.mark_killed();
            character.is_retired = true;
            character.current_abode = None;
            character.ship = None;
            for fleet in world.fleets.values_mut() {
                fleet.characters.retain(|&key| key != retired);
            }
        }
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
    if request.scenario == Scenario::PersonnelCopied {
        place_copied_characters(request, world, movement, missions, troop_transport, primary);
    }
    if request.scenario == Scenario::PersonnelAbode {
        place_abode_roster(request, world);
    }
    if request.scenario == Scenario::PersonnelPalace {
        place_palace_prisoners(world, movement, missions, troop_transport);
    }
    if request.scenario == Scenario::PersonnelRetired {
        place_retired_roster(world);
    }
    if request.scenario == Scenario::PersonnelPickup {
        place_pickup_roster(world);
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
            | Scenario::FacilityResearchTargeting
            | Scenario::ResearchDiplomacyTargeting
    ) {
        // FUN_0056c510: an own-side target with a working yard of the tree's
        // area, and a member with the tree's skill. Full support ends
        // Diplomacy (FUN_00573ee0) and the empty pool Recruitment
        // (FUN_0056b370), so the dialog lists research alone; the Diplomacy
        // case keeps the side's support below full.
        let full = if matches!(
            request.scenario,
            Scenario::ResearchDiplomacyTargeting | Scenario::FacilityResearchTargeting
        ) {
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
        // The facility case holds one construction yard (family 0x2a) as
        // A0's Jerijador does.
        let troop = request.scenario == Scenario::TroopResearchTargeting;
        let facility = request.scenario == Scenario::FacilityResearchTargeting;
        let yard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(if troop {
                    0x2900_0001
                } else if facility {
                    0x2a00_0001
                } else {
                    0x2800_0001
                }),
                side: player_control,
                is_shipyard: !troop && !facility,
            },
        );
        world.systems[secondary].manufacturing_facilities = vec![yard];
        world.set_recruit_pool_empty(player_control);
        if let Some(agent) = world.characters.values_mut().next() {
            let skill = |base| rebellion_core::world::SkillPair { base, variance: 0 };
            agent.ship_design = skill(if troop || facility { 0 } else { 40 });
            agent.troop_training = skill(if troop { 40 } else { 0 });
            agent.facility_design = skill(if facility { 40 } else { 0 });
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
            | Scenario::FacilityResearchTargeting
            | Scenario::ResearchDiplomacyTargeting
            | Scenario::UprisingTargeting
            | Scenario::CharacterCommand
            | Scenario::CharacterMove
            | Scenario::CharacterEvacuation
            | Scenario::CaptiveTransport
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
    if matches!(
        request.scenario,
        Scenario::JediTrainingDialog | Scenario::JediTrainingMessages
    ) && player_is_alliance
    {
        place_jedi_training(world);
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
        // The other side's fleets leave it, for another system (the
        // squadrons' origin in a world of three): alone there they would
        // blockade it, and FUN_005073d0 refuses a squadron a blockaded
        // planet (0x90/4).
        let away = world
            .systems
            .keys()
            .find(|&key| key != primary && key != secondary && Some(key) != origin)
            .or(origin);
        if let Some(away) = away {
            let enemies: Vec<_> = world
                .fleets
                .iter()
                .filter(|(_, fleet)| fleet.location == secondary && fleet.is_alliance != alliance)
                .map(|(key, _)| key)
                .collect();
            for key in enemies {
                world.fleets[key].location = away;
                world.systems[secondary]
                    .fleets
                    .retain(|&listed| listed != key);
                world.systems[away].fleets.push(key);
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

/// The `QuadrantsStale` scenarios' second system: the other side's
/// Kothlis-like contents (A0 2026-11-14-stale-views/07-kothlis-def-d374-
/// montage, 07-kothlis-fac-d374-montage) copied frozen into the player's
/// views (`FUN_004f9a60`), then taken from the live system; and a fleet of
/// the other side's made there since, copied with its name only
/// (06-kothlis-fleet16-d374-montage: four empty tabs). In
/// `QuadrantsStaleSeeEverything` the debug mode is on.
fn place_stale_second(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &MissionState,
    troop_transport: &TroopTransportState,
    second: SystemKey,
) {
    use rebellion_core::object_views::{
        apply, FleetDetail, ObjectView, RegimentView, SystemDetails, SystemExtras, SystemView,
        ViewPlace, ViewUpdate,
    };
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let viewer = Faction::of_alliance(player_is_alliance);
    let other = Faction::of_alliance(!player_is_alliance);
    let value = &mut world.systems[second];
    value.control = ControlKind::Controlled(other);
    value.exploration_status = ExplorationStatus::Explored;
    value.raw_materials = 6;
    value.total_energy = 11;
    value.ground_units.clear();
    value.fighters.clear();
    value.special_forces.clear();
    value.defense_facilities.clear();
    value.manufacturing_facilities.clear();
    value.production_facilities.clear();
    // The other side's regiments: two Stormtrooper and one Imperial Army
    // (two Alliance Army and one Fleet regiment for the Empire's view).
    let classes: [u32; 3] = if player_is_alliance {
        [0x1000_0006, 0x1000_0008, 0x1000_0006]
    } else {
        [0x1000_0002, 0x1000_0001, 0x1000_0002]
    };
    for class in classes {
        let troop = world.troops.insert(TroopUnit {
            class_dat_id: rebellion_core::ids::DatId::new(class),
            is_alliance: !player_is_alliance,
            regiment_strength: 100,
        });
        world.systems[second].ground_units.push(troop);
    }
    if let Some(fighter) = world
        .fighter_classes
        .iter()
        .find(|(_, class)| class.is_alliance != player_is_alliance)
        .map(|(key, _)| key)
    {
        world.systems[second]
            .fighters
            .push(rebellion_core::world::FighterEntry {
                class: fighter,
                count: 6,
                carrier: None,
                damage: None,
            });
    }
    let yard = world.manufacturing_facilities.insert(
        rebellion_core::world::ManufacturingFacilityInstance {
            class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
            side: other,
            is_shipyard: true,
        },
    );
    world.systems[second].manufacturing_facilities.push(yard);
    for (class, is_mine) in [(0x2d00_0002, false), (0x2c00_0001, true)] {
        for _ in 0..5 {
            let key = world.production_facilities.insert(
                rebellion_core::world::ProductionFacilityInstance {
                    class_dat_id: rebellion_core::ids::DatId::new(class),
                    side: other,
                    is_mine,
                },
            );
            world.systems[second].production_facilities.push(key);
        }
    }
    // The player's copies, taken while it saw the system, now frozen.
    rebellion_core::object_views::number_regiments(world);
    let id = world.systems[second].dat_id;
    let copy = SystemView::of(world, &world.systems[second], false);
    let extras = SystemExtras::of(world, movement, missions, second, false);
    let details = SystemDetails::of(world, movement, missions, troop_transport, second);
    let place = ViewPlace::Surface(id);
    let regiments: Vec<ViewUpdate> = world.systems[second]
        .ground_units
        .iter()
        .filter_map(|&key| {
            let troop = &world.troops[key];
            Some(ViewUpdate::Set {
                side: viewer,
                id: world.object_views.regiment_id(key)?,
                view: ObjectView::Regiment(RegimentView {
                    class: troop.class_dat_id,
                    is_alliance: troop.is_alliance,
                    strength: troop.regiment_strength,
                    place: place.clone(),
                    arrival: None,
                    live: false,
                }),
            })
        })
        .collect();
    apply(world, &regiments);
    let views = &mut world.object_views;
    let (systems, extras_map, details_map) = if player_is_alliance {
        (&mut views.alliance_systems, &mut views.alliance_extras, &mut views.alliance_details)
    } else {
        (&mut views.empire_systems, &mut views.empire_extras, &mut views.empire_details)
    };
    systems.insert(id, copy);
    extras_map.insert(id, extras);
    details_map.insert(id, details);
    // The live system has since lost them all but one regiment and one
    // refinery, so the debug mode has something of the side to show.
    let value = &mut world.systems[second];
    for key in value.ground_units.split_off(1) {
        world.troops.remove(key);
    }
    // One refinery stays, so the live system still shows its facility
    // icon and the debug mode has a page to show.
    let value = &mut world.systems[second];
    value.fighters.clear();
    value.manufacturing_facilities.clear();
    value.production_facilities.truncate(1);
    // A fleet made there since: copied with its name only.
    let made = world.insert_fleet(rebellion_core::world::Fleet {
        location: second,
        is_alliance: !player_is_alliance,
        ..Default::default()
    });
    world.systems[second].fleets.push(made);
    let serial = world.fleet_serial(made);
    let name = world.fleet_name(made).unwrap_or_default().to_owned();
    let views = &mut world.object_views;
    let details_map = if player_is_alliance {
        &mut views.alliance_details
    } else {
        &mut views.empire_details
    };
    if let Some(details) = details_map.get_mut(&id) {
        details.fleets.push(FleetDetail {
            serial,
            name,
            is_alliance: !player_is_alliance,
            ships: Vec::new(),
            fighters: Vec::new(),
            troops: Vec::new(),
            characters: Vec::new(),
        });
    }
    world.object_views.see_everything = request.scenario == Scenario::QuadrantsStaleSeeEverything;
}

/// The `PersonnelAbode` scenario (A0 2026-11-17-edited-saves-6 d-01 and
/// b-01): only the named Alliance characters are recruited, each free in
/// no fleet; Luke rests in the Dagobah abode (ABODESD `0x98000441`), Han
/// is held at Sluis Van for the Alliance. For the Empire, Han and Garm (a
/// General) stand at Fwillsving, the others nowhere, and the Empire's
/// copies name the places b-01 reads, Luke's in no place (`FUN_00465bb0`,
/// `0x1895`). port: the live places back the copies, as the view sync
/// (`object_views::record_characters`) leaves in no place a copy whose
/// system it sees without the character.
fn place_abode_roster(request: FixtureRequest, world: &mut GameWorld) {
    use rebellion_core::object_views::CharacterCopy;
    let alliance = request.faction == CockpitFaction::Alliance;
    let system = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, value)| (key, value.dat_id))
    };
    let dagobah = world
        .abodes
        .iter()
        // ABODESD's entry id 0x441, the game's 0x98000441.
        .find(|(_, abode)| abode.dat_id == rebellion_core::ids::DatId::new(0x441))
        .map(|(key, _)| key);
    // (name, system, held) live for the Alliance (d-01); the Empire's copy
    // of each (system, command) for b-01.
    let roster: &[(&str, &str, bool)] = &[
        ("Chewbacca", "Yavin", false),
        ("Han Solo", "Sluis Van", true),
        ("Huoba Neva", "Corsin", false),
        ("Jan Dodonna", "Yavin", false),
        ("Leia Organa", "Yavin", false),
        ("Luke Skywalker", "", false),
        ("Mon Mothma", "Chazwa", false),
        ("Wedge Antilles", "Yavin", false),
    ];
    let copies: &[(&str, &str, bool)] = &[
        ("Chewbacca", "", false),
        ("Garm Bel Iblis", "Fwillsving", true),
        ("Han Solo", "Fwillsving", false),
        ("Jan Dodonna", "", false),
        ("Leia Organa", "", false),
        ("Luke Skywalker", "", false),
        ("Syub Snunb", "", false),
        ("Wedge Antilles", "", false),
    ];
    let listed: Vec<&str> = if alliance {
        roster.iter().map(|(name, _, _)| *name).collect()
    } else {
        copies.iter().map(|(name, _, _)| *name).collect()
    };
    let keys: Vec<_> = world.characters.keys().collect();
    for key in keys {
        let character = &world.characters[key];
        let name = character.name.clone();
        let named = listed.contains(&name.as_str());
        // The scenarios' agent, the first character, took the player's
        // side above; a named one is the Alliance's again.
        if !character.is_alliance && !named {
            continue;
        }
        let place = roster
            .iter()
            .find(|(listed, _, _)| *listed == name)
            .and_then(|(_, at, held)| system(world, at).map(|(key, _)| (key, *held)));
        let character = &mut world.characters[key];
        character.is_alliance = true;
        character.is_empire = false;
        character.recruited = named;
        character.current_fleet = None;
        character.ship = None;
        character.on_mission = false;
        character.injury = 0;
        character.command = rebellion_core::character_command::CommandKind::None;
        character.is_captive = false;
        character.current_abode = None;
        character.current_system = None;
        if name == "Luke Skywalker" {
            character.current_abode = dagobah;
        } else if alliance {
            if let Some((at, held)) = place {
                character.current_system = Some(at);
                character.is_captive = held;
            }
        } else if let Some((at, general)) = copies
            .iter()
            .find(|(listed, _, _)| *listed == name)
            .and_then(|(_, at, general)| system(world, at).map(|(key, _)| (key, *general)))
        {
            let character = &mut world.characters[key];
            character.current_system = Some(at);
            if general {
                character.command = rebellion_core::character_command::CommandKind::General;
            }
        }
        for fleet in world.fleets.values_mut() {
            fleet.characters.retain(|&seated| seated != key);
        }
    }
    if alliance {
        return;
    }
    for (name, at, general) in copies {
        let Some(id) = world
            .characters
            .values()
            .find(|character| character.name == *name)
            .map(|character| character.dat_id)
        else {
            continue;
        };
        let copy = CharacterCopy {
            system: system(world, at).map(|(_, id)| id),
            ship: None,
        };
        world.object_views.empire_characters.insert(id, copy);
        if *general {
            world
                .object_views
                .empire_character_commands
                .insert(id, rebellion_core::character_command::CommandKind::General);
        }
    }
}

/// The `PersonnelRetired` scenario (A0 2026-11-18-edited-saves-9 d-03):
/// only d-03's eight Imperial characters are recruited, free in no fleet.
/// Vader is on a mission at Yavin, Palpatine and Veers stand at Coruscant,
/// Menndo at Corfai, and Ozzel is retired (destroyed with a Retire's
/// reason, `FUN_00534c00`). port: Jerjerrod, Needa and Piett stand at
/// Coruscant, not aboard d-03's ships.
fn place_retired_roster(world: &mut GameWorld) {
    let system = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let roster: &[(&str, &str)] = &[
        ("Darth Vader", "Yavin"),
        ("Emperor Palpatine", "Coruscant"),
        ("Jerjerrod", "Coruscant"),
        ("Menndo", "Corfai"),
        ("Needa", "Coruscant"),
        ("Ozzel", ""),
        ("Piett", "Coruscant"),
        ("Veers", "Coruscant"),
    ];
    let keys: Vec<_> = world.characters.keys().collect();
    for key in keys {
        let name = world.characters[key].name.clone();
        let place = roster.iter().find(|(listed, _)| *listed == name);
        if place.is_none() {
            // Every other character of the Empire's stays off its list.
            if !world.characters[key].is_alliance {
                world.characters[key].recruited = false;
            }
            continue;
        }
        let at = place.and_then(|(_, at)| system(world, at));
        for fleet in world.fleets.values_mut() {
            fleet.characters.retain(|&seated| seated != key);
        }
        let character = &mut world.characters[key];
        character.is_alliance = false;
        character.is_empire = true;
        character.recruited = true;
        character.current_fleet = None;
        character.ship = None;
        character.current_abode = None;
        character.on_mission = name == "Darth Vader";
        character.injury = 0;
        character.command = rebellion_core::character_command::CommandKind::None;
        character.is_captive = false;
        character.captured_by = None;
        character.current_system = at;
        if name == "Ozzel" {
            character.mark_killed();
            character.is_retired = true;
        }
    }
}

/// The `PersonnelPickup` scenario (A0 2026-11-18-edited-saves-9 c-14):
/// only c-14's eight Alliance characters are recruited.
/// - Han, Leia and Luke are the Empire's prisoners aboard "Victory
///   Destroyer 1" in an Imperial fleet at Coruscant, which the Empire
///   sees, so its Finder reads them live with the Captured word
///   (`FUN_0056da80` gives them Vader's `+0x70`, his ship).
/// - Chewbacca is killed: `DarthPickup` (`FUN_005445d0`) ran, so he was
///   captive or destroyed, and a capture would have synced the Captured
///   bit and the captor's side into the Empire's copy
///   (`Character_SetCaptured` `FUN_004ee800`, `FUN_004eec60`). The
///   Empire's copy of him stands frozen at Yavin, where it last synced: a
///   frozen copy takes the destroyed bit only when it next syncs
///   (`per-side-object-views.md`). hyp: he died of a failed rescue's
///   injury (slot `+0x2e0` of the minor class before `FUN_004ef480`'s
///   capture); A0 does not show how.
/// - The Empire's copies of the other four, frozen too: Dodonna's in no
///   place, Ma'w'shiye's at Yavin with the en route bit (`+0x50` bit 4),
///   Tura Raftican's and Wedge's at Yavin.
/// - Coruscant is the Empire's (seen live), Yavin the Alliance's, seen
///   by the Empire through a frozen copy.
///
/// port: the copies are written here, as `place_copied_characters` does,
/// not taken by the view step; the ship is named as A0 shows it.
fn place_pickup_roster(world: &mut GameWorld) {
    use rebellion_core::object_views::{CharacterCopy, CharacterCopyState, SystemView};
    let system = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    const PRISONERS: [&str; 3] = ["Han Solo", "Leia Organa", "Luke Skywalker"];
    // Each copied character and its copy's system ("" for none) and en
    // route bit.
    const COPIED: [(&str, &str, bool); 5] = [
        ("Chewbacca", "Yavin", false),
        ("Jan Dodonna", "", false),
        ("Ma'w'shiye", "Yavin", true),
        ("Tura Raftican", "Yavin", false),
        ("Wedge Antilles", "Yavin", false),
    ];
    let (Some(coruscant), Some(yavin)) = (system(world, "Coruscant"), system(world, "Yavin"))
    else {
        return;
    };
    // Coruscant the Empire's, which it sees live; Yavin the Alliance's,
    // seen by the Empire only through its frozen copy.
    world.systems[coruscant].control = ControlKind::Controlled(Faction::Empire);
    world.systems[yavin].control = ControlKind::Controlled(Faction::Alliance);
    let Some(class) = world
        .capital_ship_classes
        .iter()
        .find(|(_, value)| !value.is_alliance && value.name == "Victory Destroyer")
        .map(|(key, value)| (key, i32::try_from(value.hull).unwrap_or(i32::MAX)))
    else {
        return;
    };
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: coruscant,
        capital_ships: vec![rebellion_core::world::ShipInstance::new(class.0, class.1, false)],
        ..Default::default()
    });
    world.systems[coruscant].fleets.push(fleet);
    world.number_ships();
    let ship = {
        let ship = &mut world.fleets[fleet].capital_ships[0];
        ship.name = Some("Victory Destroyer 1".into());
        ship.id
    };
    let keys: Vec<_> = world.characters.keys().collect();
    for key in keys {
        let name = world.characters[key].name.clone();
        let prisoner = PRISONERS.contains(&name.as_str());
        let copied = COPIED.iter().any(|(listed, _, _)| *listed == name);
        if !world.characters[key].is_alliance && !prisoner && !copied {
            continue;
        }
        for value in world.fleets.values_mut() {
            value.characters.retain(|&seated| seated != key);
        }
        let character = &mut world.characters[key];
        character.is_alliance = true;
        character.is_empire = false;
        character.recruited = prisoner || copied;
        character.current_fleet = None;
        character.ship = None;
        character.current_abode = None;
        character.current_system = Some(yavin);
        character.on_mission = false;
        character.injury = 0;
        character.command = rebellion_core::character_command::CommandKind::None;
        character.is_captive = false;
        character.captured_by = None;
        if prisoner {
            character.is_captive = true;
            character.captured_by = Some(Faction::Empire);
            character.escape_barred = true;
            character.current_system = None;
            character.current_fleet = Some(fleet);
            character.ship = Some(ship);
            world.fleets[fleet].characters.push(key);
        } else if name == "Chewbacca" {
            character.mark_killed();
        }
    }
    let frozen = SystemView::of(world, &world.systems[yavin], false);
    let yavin_id = world.systems[yavin].dat_id;
    world.object_views.empire_systems.insert(yavin_id, frozen);
    for (name, at, enroute) in COPIED {
        let Some(id) = world
            .characters
            .values()
            .find(|character| character.name == name)
            .map(|character| character.dat_id)
        else {
            continue;
        };
        let copy = CharacterCopy {
            system: system(world, at).map(|key| world.systems[key].dat_id),
            ship: None,
        };
        let views = &mut world.object_views;
        views.empire_characters.insert(id, copy);
        views.empire_character_commands.remove(&id);
        views.empire_character_states.insert(
            id,
            CharacterCopyState {
                on_mission: false,
                hidden_mission: false,
                enroute,
            },
        );
    }
}

/// The `PersonnelPalace` scenario (A0 2026-11-18-edited-saves-7 b-19): only
/// b-19's eight Alliance characters are recruited. Huoba Neva, Jan
/// Dodonna, Mon Mothma and Wedge Antilles stand free at their systems.
/// Chewbacca, Han, Leia and Luke are the Empire's captives in Jabba's
/// Palace when the day's view step runs (`object_views::advance_extras`:
/// the Alliance's copies take them there, `record_own_captives`), then
/// stand at Coruscant, as the Pickup leaves them (`jabba.rs`), and a
/// second step keeps the copies in the palace: the Alliance does not see
/// Coruscant. port: the copies come from the port's own view step, not
/// from data written here.
fn place_palace_prisoners(
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &MissionState,
    troop_transport: &TroopTransportState,
) {
    let system = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let palace = world
        .abodes
        .iter()
        .find(|(_, abode)| abode.name == "Jabba's Palace")
        .map(|(key, _)| key);
    let free: &[(&str, &str)] = &[
        ("Huoba Neva", "Corsin"),
        ("Jan Dodonna", "Yavin"),
        ("Mon Mothma", "Chazwa"),
        ("Wedge Antilles", "Yavin"),
    ];
    let held = ["Chewbacca", "Han Solo", "Leia Organa", "Luke Skywalker"];
    let keys: Vec<_> = world.characters.keys().collect();
    let mut prisoners = Vec::new();
    for key in keys {
        let name = world.characters[key].name.clone();
        let named = held.contains(&name.as_str()) || free.iter().any(|(n, _)| *n == name);
        // The scenarios' agent, the first character, took the player's
        // side above; a named one is the Alliance's again.
        if !world.characters[key].is_alliance && !named {
            continue;
        }
        let at = free
            .iter()
            .find(|(listed, _)| *listed == name)
            .and_then(|(_, at)| system(world, at));
        for fleet in world.fleets.values_mut() {
            fleet.characters.retain(|&seated| seated != key);
        }
        let character = &mut world.characters[key];
        character.is_alliance = true;
        character.is_empire = false;
        character.recruited = named;
        character.current_fleet = None;
        character.ship = None;
        character.on_mission = false;
        character.injury = 0;
        character.command = rebellion_core::character_command::CommandKind::None;
        character.is_captive = false;
        character.captured_by = None;
        character.current_abode = None;
        character.current_system = at;
        if held.contains(&name.as_str()) {
            character.is_captive = true;
            character.captured_by = Some(Faction::Empire);
            character.current_abode = palace;
            prisoners.push(key);
        }
    }
    rebellion_core::object_views::advance_extras(world, movement, missions, troop_transport);
    let coruscant = system(world, "Coruscant");
    for key in prisoners {
        let character = &mut world.characters[key];
        character.current_abode = None;
        character.current_system = coruscant;
    }
    rebellion_core::object_views::advance_extras(world, movement, missions, troop_transport);
}

/// The `PersonnelCopied` scenario: a system other than `primary` with no
/// fleet of the player's becomes the other side's, with a new fleet of the
/// other side's first capital ship class carrying its first character. The
/// player's side takes its copies of that system there (`SystemView`,
/// `SystemExtras`, `SystemDetails`, `CharacterCopy`), frozen; its second
/// character takes a copy in no place ("Location Unknown"), its third one
/// en route to the system and its fourth one on a mission there
/// (`CharacterCopyState`). All four stand on the system and are recruited.
fn place_copied_characters(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &MissionState,
    troop_transport: &TroopTransportState,
    primary: SystemKey,
) {
    use rebellion_core::object_views::{
        character_copies, CharacterCopy, CharacterCopyState, SystemDetails, SystemExtras,
        SystemView,
    };
    let alliance = request.faction == CockpitFaction::Alliance;
    let viewer = Faction::of_alliance(alliance);
    let other = Faction::of_alliance(!alliance);
    let Some(at) = world.systems.keys().find(|&key| {
        key != primary
            && !world.systems[key].control.is_controlled_by(viewer)
            && !world
                .fleets
                .values()
                .any(|fleet| fleet.location == key && fleet.is_alliance == alliance)
    }) else {
        return;
    };
    // The other side's living characters, its recruited ones first; those
    // taken here are recruited (the Empire's galaxy recruits few Alliance
    // characters).
    let mut others: Vec<_> = world
        .characters
        .iter()
        .filter(|(_, value)| value.is_alliance != alliance && !value.is_killed)
        .map(|(key, value)| (!value.recruited, key))
        .collect();
    others.sort();
    let chosen: Vec<_> = others.into_iter().map(|(_, key)| key).take(4).collect();
    let [aboard, lost, travelling, busy] = chosen[..] else {
        return;
    };
    let Some(class) = world
        .capital_ship_classes
        .iter()
        .find(|(_, value)| value.is_alliance != alliance && !value.is_death_star())
        .map(|(key, value)| (key, i32::try_from(value.hull).unwrap_or(i32::MAX)))
    else {
        return;
    };
    world.systems[at].control = ControlKind::Controlled(other);
    for key in [aboard, lost, travelling, busy] {
        let character = &mut world.characters[key];
        character.recruited = true;
        character.current_system = None;
        character.current_fleet = None;
        character.current_abode = None;
        character.ship = None;
        character.on_mission = false;
        character.is_captive = false;
        character.injury = 0;
        character.command = rebellion_core::character_command::CommandKind::None;
        for fleet in world.fleets.values_mut() {
            fleet.characters.retain(|&seated| seated != key);
        }
    }
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: at,
        capital_ships: vec![rebellion_core::world::ShipInstance::new(
            class.0, class.1, !alliance,
        )],
        characters: vec![aboard],
        is_alliance: !alliance,
        ..Default::default()
    });
    world.systems[at].fleets.push(fleet);
    world.characters[aboard].current_fleet = Some(fleet);
    world.number_ships();
    // The player's copies, taken while it saw the system, now frozen.
    let id = world.systems[at].dat_id;
    let copy = SystemView::of(world, &world.systems[at], false);
    let extras = SystemExtras::of(world, movement, missions, at, false);
    let details = SystemDetails::of(world, movement, missions, troop_transport, at);
    let copied = character_copies(world, movement, viewer, at);
    let lost_id = world.characters[lost].dat_id;
    let aboard_id = world.characters[aboard].dat_id;
    let travelling_id = world.characters[travelling].dat_id;
    let busy_id = world.characters[busy].dat_id;
    for key in [lost, travelling, busy] {
        world.characters[key].current_system = Some(at);
    }
    // A traveller seen leaving for the system, frozen en route there
    // (FUN_00556390, +0x50 bit 4), and one on a mission there (+0x68).
    let placed = CharacterCopy {
        system: Some(id),
        ship: None,
    };
    let state = |on_mission, enroute| CharacterCopyState {
        on_mission,
        hidden_mission: false,
        enroute,
    };
    let views = &mut world.object_views;
    let (systems, extras_map, details_map, characters_map) = if alliance {
        (
            &mut views.alliance_systems,
            &mut views.alliance_extras,
            &mut views.alliance_details,
            &mut views.alliance_characters,
        )
    } else {
        (
            &mut views.empire_systems,
            &mut views.empire_extras,
            &mut views.empire_details,
            &mut views.empire_characters,
        )
    };
    systems.insert(id, copy);
    extras_map.insert(id, extras);
    details_map.insert(id, details);
    characters_map.extend(copied);
    characters_map.insert(lost_id, CharacterCopy::default());
    characters_map.insert(travelling_id, placed.clone());
    characters_map.insert(busy_id, placed);
    let states = if alliance {
        &mut views.alliance_character_states
    } else {
        &mut views.empire_character_states
    };
    states.insert(travelling_id, state(false, true));
    states.insert(busy_id, state(true, false));
    // The copy aboard froze holding an Admiral's command (+0x96, kept by
    // FUN_004f04f0 while frozen), which the live character has since lost:
    // "Name - <ship> ( Admiral ) " (FUN_00465bb0, 0x8801).
    let commands = if alliance {
        &mut views.alliance_character_commands
    } else {
        &mut views.empire_character_commands
    };
    commands.insert(
        aboard_id,
        rebellion_core::character_command::CommandKind::Admiral,
    );
}

/// The arrival day of the frozen copy in `EnemyRegimentFrozen`: A0
/// 2026-11-14-quadrants-status-known/18 reads "ETA Destination: Day 171".
const FROZEN_ARRIVAL: u64 = 171;

/// The `EnemyRegimentFrozen` scenario: `secondary` becomes the other
/// side's, without its regiments, and the player's side holds a frozen
/// copy of the other side's Army Regiment (TROOPSD 8 for the Empire, as A0
/// 18's Imperial Army Regiment; TROOPSD 2 for the Alliance) on its surface,
/// en route until day 171 (`FUN_004f9a60`, `object_views`). Its Defenses
/// window opens at the galaxy view's right edge on the Trooper Regiments
/// page.
fn place_frozen_enemy_regiment(
    request: FixtureRequest,
    world: &mut GameWorld,
    defenses: &mut DefensesWindowState,
    cockpit: &CockpitState,
    secondary: SystemKey,
    arrival: Option<u64>,
) {
    use rebellion_core::object_views::{apply, ObjectView, RegimentView, ViewPlace, ViewUpdate};
    let alliance = request.faction == CockpitFaction::Alliance;
    let viewer = rebellion_core::dat::Faction::of_alliance(alliance);
    let other = rebellion_core::dat::Faction::of_alliance(!alliance);
    world.systems[secondary].control = ControlKind::Controlled(other);
    let troops = &world.troops;
    world.systems[secondary]
        .ground_units
        .retain(|key| troops.get(*key).is_none_or(|troop| troop.is_alliance == alliance));
    let class = rebellion_core::ids::DatId::new(if alliance { 0x1000_0008 } else { 0x1000_0002 });
    let views = &mut world.object_views;
    views.last_id += 1;
    let id = rebellion_core::ids::DatId::new(0x1000_0000 | views.last_id);
    let place = ViewPlace::Surface(world.systems[secondary].dat_id);
    apply(
        world,
        &[ViewUpdate::Set {
            side: viewer,
            id,
            view: ObjectView::Regiment(RegimentView {
                class,
                is_alliance: !alliance,
                strength: 100,
                place,
                arrival,
                live: false,
            }),
        }],
    );
    let layout = cockpit.layout_for(640.0, 480.0);
    let galaxy = layout.galaxy;
    defenses.open(
        world,
        secondary,
        (
            (galaxy.x + galaxy.width - DEFENSES_WINDOW_WIDTH) as i16 - 5,
            galaxy.y as i16 + 5,
        ),
        request.faction,
        layout,
    );
    defenses.show_page(
        secondary,
        rebellion_render::defenses_window::DefensesPage::Regiments,
    );
}

/// The Rescue scenario: the second character becomes a prisoner of the
/// player's side held by the enemy at `secondary`, the only character
/// there, so it fills the first cell of that system's personnel page
/// (`FUN_004eec60` gives it the captor's side). The agent's Defenses window
/// opens at the galaxy view's right edge, the prison's at its left edge.
/// Trades the records of two characters, and their places in the fleets.
fn swap_characters(
    world: &mut GameWorld,
    a: rebellion_core::ids::CharacterKey,
    b: rebellion_core::ids::CharacterKey,
) {
    if a == b {
        return;
    }
    let first = world.characters[a].clone();
    let second = std::mem::replace(&mut world.characters[b], first);
    world.characters[a] = second;
    for fleet in world.fleets.values_mut() {
        for key in &mut fleet.characters {
            if *key == a {
                *key = b;
            } else if *key == b {
                *key = a;
            }
        }
    }
}

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
        is_alliance: enemy == Faction::Alliance,
        has_death_star: wants_death_star,
        ..Default::default()
    });
    if wants_death_star {
        world.systems[secondary].fleets.push(fleet);
    } else {
        world.systems[secondary].fleets = vec![fleet];
    }
    fleets.open(world, secondary, left, request.faction, layout);
    fleets.select(secondary, FleetWindowEntry::Fleet(fleet));
}

/// The Jedi Training scenarios' world, as A0 2026-11-16-edited-saves
/// x-53..x-56 had it: Luke a Jedi Knight after Dagobah (x-53's Force
/// Ranking, Force edited to 100; his Jedi mini, x-56) and Jan Dodonna with
/// some Force, a Novice (10), both free and recruited on the Alliance's
/// Yavin. The teacher is the member that trains Jedi at Force GNPRTB 2581
/// (100) or more (`FUN_00570e40`, `FUN_004ed310`), the pupil any other at
/// Force above 0 and below the teacher's (`FUN_00570690`). The recruit pool
/// is empty, as A0 lists Jedi Training first (x-54, x-55) where Recruitment
/// (`FUN_0046cab0` key 1) would head it.
fn place_jedi_training(world: &mut GameWorld) {
    let Some((luke, dodonna, yavin)) = jedi_training_cast(world) else {
        return;
    };
    world.systems[yavin].control = ControlKind::Controlled(Faction::Alliance);
    for (key, force) in [(luke, 100), (dodonna, 10)] {
        for fleet in world.fleets.values_mut() {
            fleet.characters.retain(|&member| member != key);
        }
        let value = &mut world.characters[key];
        value.current_system = Some(yavin);
        value.current_fleet = None;
        value.is_alliance = true;
        value.is_empire = false;
        value.recruited = true;
        value.is_captive = false;
        value.captured_by = None;
        value.on_mission = false;
        value.on_mandatory_mission = false;
        value.injury = 0;
        value.force_potential = true;
        value.force_aware = true;
        value.force_base = force;
        value.force_training = 0;
    }
    world.characters[luke].dagobah_completed = true;
    world.set_recruit_pool_empty(Faction::Alliance);
}

/// The Jedi Training scenarios' Luke, Dodonna and Yavin.
fn jedi_training_cast(world: &GameWorld) -> Option<(CharacterKey, CharacterKey, SystemKey)> {
    let character = |name: &str| {
        world
            .characters
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let yavin = world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Yavin")
        .map(|(key, _)| key)?;
    Some((
        character("Luke Skywalker")?,
        character("Jan Dodonna")?,
        yavin,
    ))
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
        is_alliance: true,
        ..Default::default()
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
            damage: None,
        })
        .collect();
    let first = world.insert_fleet(rebellion_core::world::Fleet {
        location: coruscant,
        capital_ships: vec![ship],
        fighters: squadrons,
        ..Default::default()
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

/// The `SpecialForceAboard` scenario: A0 2026-11-16-partial-cells/f-1
/// (sha256 1b446c19e7e7): Coruscant's Fleet 1 after the Imperial Probe
/// Droid was dropped on it, its two ships each marked with squadrons and
/// regiments (f-2, ce8c644c6e6a) and the droid aboard Imperial Star
/// Destroyer 1 (f-5, 472c151b475c), the Fleet window at (365, 92).
fn place_probe_droid_aboard(
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
    let class = |world: &GameWorld, name: &str| {
        world
            .capital_ship_classes
            .iter()
            .find(|(_, value)| value.is_empire && value.name == name)
            .map(|(key, _)| key)
    };
    let destroyer = class(world, "Imperial Star Destroyer");
    let victory = class(world, "Victory Destroyer");
    let tie = world
        .fighter_classes
        .iter()
        .find(|(_, value)| !value.is_alliance && value.name == "TIE Fighter")
        .map(|(key, _)| key);
    let stormtroopers = world
        .buildable_classes
        .iter()
        .find(|(class, value)| class.family() == 0x10 && value.name == "Stormtrooper Regiment")
        .map(|(class, _)| *class);
    let (Some(coruscant), Some(destroyer), Some(victory), Some(tie), Some(stormtroopers)) =
        (coruscant, destroyer, victory, tie, stormtroopers)
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
    let ships = [destroyer, victory]
        .map(|class| ShipInstance::new(class, world.capital_ship_classes[class].hull as i32, true));
    let capacity = |class| &world.capital_ship_classes[class];
    // Room for one more than the Star Destroyer holds, so the Victory
    // Destroyer carries some too (squadrons go where most room is left,
    // regiments fill the ships in fleet order).
    let squadrons = capacity(destroyer).fighter_capacity + capacity(victory).fighter_capacity;
    let regiments = (capacity(destroyer).troop_capacity + 1)
        .min(capacity(destroyer).troop_capacity + capacity(victory).troop_capacity);
    let first = world.insert_fleet(rebellion_core::world::Fleet {
        location: coruscant,
        capital_ships: ships.to_vec(),
        fighters: (0..squadrons)
            .map(|_| rebellion_core::world::FighterEntry {
                class: tie,
                count: 1,
                carrier: None,
                damage: None,
            })
            .collect(),
        ..Default::default()
    });
    world.rename_fleet(first, "Fleet 1");
    world.rename_ship(first, 0, "Imperial Star Destroyer 1");
    world.rename_ship(first, 1, "Victory Destroyer 1");
    world.number_ships();
    rebellion_core::squadrons::seat_squadrons(world, first);
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let regiments: Vec<_> = (0..regiments)
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
    // SPECFCSD class 5, the Imperial Probe Droid, with f-5's ratings:
    // Espionage 30 and Combat 10 ([`Skill`] order).
    let droid = world
        .special_forces
        .insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0005),
            is_alliance: false,
            skills: [0, 30, 0, 0, 0, 10, 0, 0],
            on_mission: false,
        });
    rebellion_core::missions::board_fleet(
        world,
        rebellion_core::missions::MissionMember::SpecialForce(droid),
        first,
    );
    let layout = cockpit.layout_for(640.0, 480.0);
    fleets.open(world, coruscant, (365, 92), request.faction, layout);
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
        Scenario::Quadrants
        | Scenario::QuadrantsNeutralFleet
        | Scenario::QuadrantsFreeTeam
        | Scenario::QuadrantsRegimentQueued
        | Scenario::QuadrantsRegimentEnRoute
        | Scenario::QuadrantsHeadquartersFrozen
        | Scenario::QuadrantsHeadquartersHidden
        | Scenario::QuadrantsHeadquartersLive
        | Scenario::QuadrantsFacility
        | Scenario::QuadrantsStale
        | Scenario::QuadrantsStaleSeeEverything => {
            let primary = world.systems.keys().next()?;
            Some((primary, loading_target(world, primary)?))
        }
        Scenario::CoruscantSeized
        | Scenario::CoruscantSquadrons
        | Scenario::CoruscantRegiments => {
            let coruscant = coruscant(world)?;
            Some((loading_target(world, coruscant)?, coruscant))
        }
        // Yavin, in Sumitra's open sector window, both times.
        Scenario::HeadquartersTaken => {
            let yavin = world
                .systems
                .iter()
                .find(|(_, value)| value.name == "Yavin")
                .map(|(key, _)| key)?;
            Some((yavin, yavin))
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
        is_alliance: true,
        ..Default::default()
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
        is_alliance: true,
        ..Default::default()
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

/// The `UnfinishedShipMini` and `UnfinishedShipEnroute` scenarios: A0
/// 2026-11-15 run "m-" (Empire, Small galaxy). A Carrack Light Cruiser is
/// ordered at Corfai, so it waits unfinished in the yard's spare
/// (`rebellion_core::products`), and the Fleet window opens over Corfai
/// where A0's lay (174, 91). With `enroute`, the Carrack is moved into a
/// fleet of two Imperial ships at Talus and finishes at Corfai: it stays
/// there on its trip (`FUN_0052bee0`), and the window opens over Talus at
/// (180, 91).
fn place_unfinished_ship(
    request: FixtureRequest,
    world: &mut GameWorld,
    manufacturing: &mut ManufacturingState,
    cockpit: &CockpitState,
    fleets: &mut FleetWindowState,
    enroute: bool,
) {
    use rebellion_core::manufacturing::{BuildableKind, QueueItem};
    let Some(corfai) = clear_for_a0(world, "Corfai") else {
        return;
    };
    world.systems[corfai].control = ControlKind::Controlled(Faction::Empire);
    let class = |world: &GameWorld, name: &str| {
        world
            .capital_ship_classes
            .iter()
            .find(|(_, value)| !value.is_alliance && value.name == name)
            .map(|(key, _)| key)
    };
    let Some(carrack) = class(world, "Carrack Light Cruiser") else {
        return;
    };
    manufacturing.enqueue(
        corfai,
        QueueItem::new(BuildableKind::CapitalShip(carrack), 104).ordered(world),
    );
    rebellion_core::products::reconcile(world, manufacturing);
    rebellion_core::products::number_new_spares(world);
    let layout = cockpit.layout_for(640.0, 480.0);
    let Some(product) = world.products.all().first().cloned() else {
        return;
    };
    if !enroute {
        let rebellion_core::products::Container::Fleet(spare) = product.container else {
            return;
        };
        fleets.open(world, corfai, (174, 91), request.faction, layout);
        fleets.select(corfai, FleetWindowEntry::Fleet(spare));
        return;
    }
    let Some(talus) = clear_for_a0(world, "Talus") else {
        return;
    };
    world.systems[talus].control = ControlKind::Controlled(Faction::Empire);
    let ships: Vec<ShipInstance> = ["Imperial Dreadnaught", "Galleon"]
        .iter()
        .filter_map(|name| class(world, name))
        .map(|key| ShipInstance::new(key, world.capital_ship_classes[key].hull.cast_signed(), false))
        .collect();
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: talus,
        capital_ships: ships,
        ..Default::default()
    });
    world.systems[talus].fleets.push(fleet);
    rebellion_core::products::move_to_target(world, product.id, talus, Some(fleet), 0);
    // The order's last point at Corfai: finished away from its container.
    *manufacturing = ManufacturingState::new();
    rebellion_core::products::settle(
        world,
        vec![rebellion_core::manufacturing::CompletionEvent {
            system: corfai,
            // Far ahead, so the trip outlasts the gate whatever the clock.
            tick: 100_000,
            kind: BuildableKind::CapitalShip(carrack),
            name: product.name.clone(),
            fleet: None,
        }],
        Vec::new(),
    );
    fleets.open(world, talus, (180, 91), request.faction, layout);
    fleets.select(talus, FleetWindowEntry::Fleet(fleet));
}

/// `QuadrantsRegimentQueued` and `QuadrantsRegimentEnRoute`: a regiment of
/// the player's ordered at a yard of the player's with the primary as its
/// destination, so its product stands in the primary (`products::reconcile`;
/// A0 2026-11-19-edited-saves-10: Coruscant's Troops band, Destination
/// Corfai). With `en_route` it is trained and travelling in, its arrival
/// far ahead so the trip outlasts the gate (`products::settle`).
fn place_regiment_for_primary(
    request: FixtureRequest,
    world: &mut GameWorld,
    manufacturing: &mut ManufacturingState,
    primary: SystemKey,
    en_route: bool,
) {
    use rebellion_core::manufacturing::{BuildableKind, CompletionEvent, QueueItem};
    // The yard: a system of the player's in another sector, off the
    // sector window (A0's Coruscant, Sesswenna, for Corfai, Corellian).
    let Some(yard) = world
        .systems
        .keys()
        .find(|&key| world.systems[key].sector != world.systems[primary].sector)
    else {
        return;
    };
    world.systems[yard].control = ControlKind::Controlled(Faction::of_alliance(
        request.faction == CockpitFaction::Alliance,
    ));
    let class = rebellion_core::ids::DatId::new(if request.faction == CockpitFaction::Alliance {
        0x1000_0001
    } else {
        0x1000_0006
    });
    let kind = BuildableKind::Troop(class);
    manufacturing.enqueue(yard, QueueItem::new(kind, 24).delivered_to(primary));
    rebellion_core::products::reconcile(world, manufacturing);
    if !en_route {
        return;
    }
    *manufacturing = ManufacturingState::new();
    rebellion_core::products::settle(
        world,
        vec![CompletionEvent {
            system: yard,
            tick: 100_000,
            kind,
            name: None,
            fleet: None,
        }],
        Vec::new(),
    );
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
        is_alliance: true,
        ..Default::default()
    };
    let squadron = |class| rebellion_core::world::FighterEntry {
        class,
        count: 1,
        carrier: None,
        damage: None,
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
        is_alliance: true,
        ..Default::default()
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
        ..Default::default()
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

/// The regiments on Commenor in the `OwnAssaultRepulsed` scenario: A0
/// 2026-11-14-bombard-assault/05 lists five Imperial Army Regiments.
const OWN_ASSAULT_DEFENDERS: usize = 5;

/// The `OwnAssaultRepulsed` scenario: Commenor the Empire's with
/// [`OWN_ASSAULT_DEFENDERS`] Imperial Army Regiments and no defense
/// facility (A0 05..07), and the Alliance's Fleet 1, a Medium Transport
/// carrying one Alliance Fleet Regiment, in orbit (A0 00, 08).
fn place_own_assault(
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
    let transport = world
        .capital_ship_classes
        .iter()
        .find(|(_, value)| value.is_alliance && value.name == "Medium Transport")
        .map(|(key, _)| key);
    let (Some(fleet_regiment), Some(army), Some(transport)) = (
        troop_class(world, "Alliance Fleet Regiment"),
        troop_class(world, "Imperial Army Regiment"),
        transport,
    ) else {
        return;
    };
    let Some(commenor) = clear_for_a0(world, "Commenor") else {
        return;
    };
    let value = &mut world.systems[commenor];
    value.control = ControlKind::Controlled(Faction::Empire);
    value.is_populated = true;
    value.defense_facilities.clear();
    let hull = world.capital_ship_classes[transport].hull as i32;
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: commenor,
        capital_ships: vec![ShipInstance::new(transport, hull, false)],
        is_alliance: true,
        ..Default::default()
    });
    world.rename_fleet(fleet, "Fleet 1");
    world.number_ships();
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    let attacker = world.troops.insert(TroopUnit {
        class_dat_id: fleet_regiment,
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[commenor].ground_units = vec![attacker];
    let _ = troop_transport.embark(world, fleet, &[attacker]);
    let defenders: Vec<_> = (0..OWN_ASSAULT_DEFENDERS)
        .map(|_| {
            world.troops.insert(TroopUnit {
                class_dat_id: army,
                is_alliance: false,
                regiment_strength: 100,
            })
        })
        .collect();
    world.systems[commenor].ground_units = defenders;
}

/// The `HeadquartersTaken` scenario: Yavin the Alliance's headquarters
/// alone, the HQ object (ALLFACSD `0x20000001`, the Alliance's) on it, as
/// A0 2026-11-16-partial-cells/v-17 (sha256 2d0316e7) marks it, and
/// Sumitra's sector window open on it.
fn place_headquarters_at_yavin(
    request: FixtureRequest,
    world: &mut GameWorld,
    sectors: &mut SectorWindowState,
) {
    let Some(yavin) = world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Yavin")
        .map(|(key, _)| key)
    else {
        return;
    };
    for value in world.systems.values_mut() {
        value.is_headquarters = false;
    }
    world.systems[yavin].is_headquarters = true;
    let class = rebellion_core::ids::DatId::new(0x2000_0001);
    if !world.systems[yavin]
        .manufacturing_facilities
        .iter()
        .any(|&key| world.manufacturing_facilities.get(key).is_some_and(|f| f.class_dat_id == class))
    {
        let hq = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: class,
                side: Faction::Alliance,
                is_shipyard: false,
            },
        );
        world.systems[yavin].manufacturing_facilities.push(hq);
    }
    // A0 v-17 and v-20 show a facility icon at Yavin besides the HQ, which
    // the takeover hands to the Empire (v-20). Since the seed rolls every
    // system's resources (471b20e1) its Yavin holds none, so one mine
    // stands in for it.
    let other_facility = world.systems[yavin].production_facilities.len()
        + world.systems[yavin]
            .manufacturing_facilities
            .iter()
            .filter(|&&key| world.manufacturing_facilities.get(key).is_some_and(|f| f.class_dat_id != class))
            .count();
    if other_facility == 0 {
        let mine = world.production_facilities.insert(rebellion_core::world::ProductionFacilityInstance {
            class_dat_id: rebellion_core::ids::DatId::new(0x2C00_0001),
            side: Faction::Alliance,
            is_mine: true,
        });
        world.systems[yavin].production_facilities.push(mine);
    }
    sectors.open_for_system(world, yavin, request.faction);
}

/// The `GhormanBombardment` scenario: Ghorman the Alliance's, explored,
/// with no fleet in orbit, and the Empire's Fleet 1 there: an Imperial
/// Star Destroyer and an Imperial Dreadnaught at full hull (A0
/// 2026-11-17-edited-saves-5/i-04 lists them). Sesswenna's sector window
/// is open, as behind A0's summary (i-02).
fn place_ghorman_bombardment(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &mut MovementState,
    sectors: &mut SectorWindowState,
) {
    let Some(ghorman) = clear_for_a0(world, "Ghorman") else {
        return;
    };
    // A0's Fleet 1 holds the side's first ship of each class ("Imperial
    // Star Destroyer 1", "Imperial Dreadnaught 1"). The number is the
    // side's count of the class plus one (FUN_00517760 -> FUN_005302c0),
    // so a ship made here after the seeded ones would take the next
    // number: the seeded first ship joins Fleet 1 instead, and only a
    // class the seed gave none of is made new (and numbered 1).
    let ships: Vec<ShipInstance> = ["Imperial Star Destroyer", "Imperial Dreadnaught"]
        .into_iter()
        .filter_map(|name| {
            let (class, hull) = world
                .capital_ship_classes
                .iter()
                .find(|(_, value)| value.is_empire && value.name == name)
                .map(|(key, value)| (key, i32::try_from(value.hull).unwrap_or(i32::MAX)))?;
            let first = format!("{name} 1");
            let seeded = world
                .fleets
                .values_mut()
                .filter(|fleet| !fleet.is_alliance)
                .find_map(|fleet| {
                    let at = fleet
                        .capital_ships
                        .iter()
                        .position(|ship| ship.name.as_deref() == Some(first.as_str()))?;
                    Some(fleet.capital_ships.remove(at))
                });
            Some(seeded.unwrap_or_else(|| ShipInstance::new(class, hull, false)))
        })
        .collect();
    if ships.is_empty() {
        return;
    }
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        location: ghorman,
        capital_ships: ships,
        ..Default::default()
    });
    world.rename_fleet(fleet, "Fleet 1");
    world.number_ships();
    rebellion_core::movement::reconcile_fleet_orbits(movement, world);
    sectors.open_for_system(world, ghorman, request.faction);
}

/// The `GhormanAssault` scenario's regiment: one Stormtrooper Regiment
/// aboard Fleet 1 at Ghorman (hyp: A0 i-01's Fleet 1 carried "troops";
/// which and how many is not on any capture, and the Summary page shows
/// none of them), Ghorman's own ground left empty (A0 i-07: "None").
fn embark_ghorman_regiment(world: &mut GameWorld, troop_transport: &mut TroopTransportState) {
    let Some(stormtroopers) = world
        .buildable_classes
        .iter()
        .find(|(class, value)| class.family() == 0x10 && value.name == "Stormtrooper Regiment")
        .map(|(class, _)| *class)
    else {
        return;
    };
    let Some(ghorman) = world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Ghorman")
        .map(|(key, _)| key)
    else {
        return;
    };
    let Some(&fleet) = world.systems[ghorman]
        .fleets
        .iter()
        .find(|&&fleet| !world.fleets[fleet].is_alliance)
    else {
        return;
    };
    let regiment = world.troops.insert(TroopUnit {
        class_dat_id: stormtroopers,
        is_alliance: false,
        regiment_strength: 100,
    });
    world.systems[ghorman].ground_units = vec![regiment];
    let _ = troop_transport.embark(world, fleet, &[regiment]);
    world.systems[ghorman].ground_units.clear();
    // A0 i-07: the Empire takes Ghorman undefended. Since the seed rolls
    // every system's resources (471b20e1) its Ghorman holds defense
    // facilities, which A0's did not show.
    world.systems[ghorman].defense_facilities.clear();
}

/// The `GhormanBombardment` strike: the Empire's fleets at Ghorman carry
/// out General Bombardment (`FUN_00487740`) with the first seed whose
/// draws destroy nothing on either side, as A0 i-03 and i-04 list "No
/// Casualties": the report's picture is then `0x2b9a` (`FUN_00496770`).
/// Returns its report for the app to present.
fn ghorman_bombardment(
    world: &mut GameWorld,
    troop_transport: &mut TroopTransportState,
) -> Vec<rebellion_core::assault_report::AssaultReport> {
    let Some(ghorman) = world
        .systems
        .iter()
        .find(|(_, value)| value.name == "Ghorman")
        .map(|(key, _)| key)
    else {
        return Vec::new();
    };
    let fleets: Vec<FleetKey> = world
        .fleets
        .iter()
        .filter(|(_, fleet)| fleet.location == ghorman && !fleet.is_alliance)
        .map(|(key, _)| key)
        .collect();
    if fleets.is_empty() {
        return Vec::new();
    }
    for seed in 0..256 {
        let mut trial = world.clone();
        let mut env = rebellion_core::bombardment::BombardEnv::new(seed, None);
        let outcome = env
            .bombard_reported(
                &mut trial,
                troop_transport,
                Faction::Empire,
                &fleets,
                ghorman,
                rebellion_core::bombardment::GENERAL,
            )
            .clone();
        if outcome.destroyed.is_empty()
            && outcome.ship_losses.is_empty()
            && env.reports.iter().all(|report| report.picture() == 0x2b9a)
            && !env.reports.is_empty()
        {
            *world = trial;
            return env.reports;
        }
    }
    Vec::new()
}

/// Whether the app opens the fixture's assault report at once, as Display
/// on it would (`FUN_00469de0`): not in `HeadquartersTaken`, whose gate
/// reads it from the index as A0 did (v-18).
pub fn opens_the_assault_report(request: FixtureRequest) -> bool {
    request.scenario != Scenario::HeadquartersTaken
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
    // OwnAssaultRepulsed: the Alliance's assault on Commenor with the first
    // seed whose draws give A0 2026-11-14-bombard-assault/09..11's outcome:
    // the attacking regiment destroyed, no defender lost, the system kept
    // (TEXTSTRA 0x7092).
    if request.scenario == Scenario::GhormanBombardment {
        return ghorman_bombardment(world, troop_transport);
    }
    let (name, side) = match request.scenario {
        Scenario::EnemyAssaultTaken | Scenario::HeadquartersTaken => ("Yavin", Faction::Empire),
        Scenario::GhormanAssault => ("Ghorman", Faction::Empire),
        Scenario::OwnAssaultRepulsed => ("Commenor", Faction::Alliance),
        _ => return Vec::new(),
    };
    let Some(yavin) = world
        .systems
        .iter()
        .find(|(_, value)| value.name == name)
        .map(|(key, _)| key)
    else {
        return Vec::new();
    };
    let alliance = side == Faction::Alliance;
    let fleets: Vec<FleetKey> = world
        .fleets
        .iter()
        .filter(|(_, fleet)| fleet.location == yavin && fleet.is_alliance == alliance)
        .map(|(key, _)| key)
        .collect();
    for seed in 0..256 {
        let mut trial_world = world.clone();
        let mut trial_transport = troop_transport.clone();
        let mut env = rebellion_core::bombardment::BombardEnv::new(seed, None);
        let outcome = env
            .assault(&mut trial_world, &mut trial_transport, side, &fleets, yavin)
            .clone();
        let as_a0 = if request.scenario == Scenario::GhormanAssault {
            // A0 i-07: no regiment lost on either side; taken.
            outcome.lost_attackers.is_empty()
                && outcome.lost_defenders.is_empty()
                && env.reports.iter().all(|report| report.taken)
        } else if alliance {
            outcome.lost_attackers.len() == 1
                && outcome.lost_defenders.is_empty()
                && env.reports.iter().all(|report| !report.taken)
        } else {
            outcome.lost_attackers.is_empty()
                && outcome.lost_defenders.len() == 1
                && env.reports.iter().all(|report| report.taken)
        };
        if as_a0 && !env.reports.is_empty() {
            *world = trial_world;
            *troop_transport = trial_transport;
            if matches!(
                request.scenario,
                Scenario::HeadquartersTaken | Scenario::GhormanAssault
            ) {
                // The occupation's control change (FUN_00510d70 ->
                // FUN_004f6f40, as `tactical_flow::occupy` makes it): the
                // HQ, which serves the Alliance alone, ends (FUN_004f8680).
                world.systems[yavin].control = ControlKind::Controlled(Faction::Empire);
                world.hand_over_facilities(yavin);
            }
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

/// The cockpit the ending scenarios' games were started from: Headquarters
/// Victory (A0 2026-11-16-edited-saves x-24), Expert and a Large galaxy.
/// The ending's film returns through app state 3, whose new menu window
/// (`FUN_00414a80` -> `FUN_00404c30`) shows none of them (x-43).
#[must_use]
pub fn starting_menu(request: FixtureRequest) -> Option<rebellion_render::MainMenuState> {
    matches!(
        request.scenario,
        Scenario::EndingDefeat | Scenario::EndingVictory
    )
    .then(|| {
        let mut menu = rebellion_render::MainMenuState::default();
        menu.difficulty = rebellion_render::Difficulty::Hard;
        // The lever reaches Large from Standard on its way to Huge.
        menu.galaxy_size = rebellion_core::dat::GalaxySize::Large;
        menu.lever_returning = true;
        menu.headquarters_only = true;
        menu
    })
}

/// Coruscant, `0x90000109` (`FUN_005180e0` reads it by that key).
fn coruscant_key(world: &GameWorld) -> Option<SystemKey> {
    world
        .systems
        .iter()
        .find(|(_, system)| system.dat_id.index() == 0x109)
        .map(|(key, _)| key)
}

/// The `CaptiveTransport` captive: the first character of the other side
/// after `agent` in the arena, held by the player's side at `primary`, so
/// the Defenses window's personnel page lists it right after the agent.
fn place_captive(
    world: &mut GameWorld,
    agent: Option<CharacterKey>,
    primary: SystemKey,
    player_is_alliance: bool,
) {
    let after_agent = world
        .characters
        .keys()
        .skip_while(|&key| Some(key) != agent)
        .skip(1);
    let captive = after_agent.clone().find(|&key| {
        let c = &world.characters[key];
        c.is_alliance != player_is_alliance && !c.is_killed
    });
    let Some(captive) = captive else {
        return;
    };
    // Nobody else stands between them on the page.
    let others: Vec<CharacterKey> = world
        .characters
        .iter()
        .filter(|&(key, c)| {
            Some(key) != agent && key != captive && c.current_system == Some(primary)
        })
        .map(|(key, _)| key)
        .collect();
    for key in others {
        world.characters[key].current_system = None;
    }
    for fleet in world.fleets.values_mut() {
        fleet.characters.retain(|&key| key != captive);
    }
    let character = &mut world.characters[captive];
    character.current_system = Some(primary);
    character.current_fleet = None;
    character.ship = None;
    character.on_mission = false;
    character.injury = 0;
    character.is_captive = true;
    character.captured_by = Some(if player_is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    });
    character.command = rebellion_core::character_command::CommandKind::None;
}

/// The `EndingAssaultCoruscant` scenario's game: the Alliance headquarters
/// (the HQ object, ALLFACSD `0x20000001`) at an Alliance system outside
/// Coruscant's sector with no fleet in orbit, so the word's `0x1` stays
/// clear. Returns the headquarters' system and Coruscant, for the app's
/// `VictoryState`; the game type is Headquarters Victory.
#[must_use]
pub fn place_headquarters_victory(
    request: FixtureRequest,
    world: &mut GameWorld,
) -> Option<(SystemKey, SystemKey)> {
    if request.scenario != Scenario::EndingAssaultCoruscant {
        return None;
    }
    let coruscant = coruscant_key(world)?;
    let sector = world.systems[coruscant].sector;
    let hq = world
        .systems
        .iter()
        .skip(10)
        .find(|&(key, system)| {
            system.sector != sector
                && system.ground_units.is_empty()
                && !world.fleets.values().any(|fleet| fleet.location == key)
        })
        .map(|(key, _)| key)?;
    for system in world.systems.values_mut() {
        system.is_headquarters = false;
    }
    let value = &mut world.systems[hq];
    value.is_headquarters = true;
    value.is_destroyed = false;
    // Populated, with no regiment and the Alliance's support, so the
    // economy keeps it the Alliance's (FUN_0050b610, FUN_0055a080).
    value.is_populated = true;
    value.exploration_status = ExplorationStatus::Explored;
    value.control = ControlKind::Controlled(Faction::Alliance);
    (value.popularity_alliance, value.popularity_empire) = (0.9, 0.1);
    move_headquarters_object(world, hq);
    Some((hq, coruscant))
}

/// Whether the scenario's player has the advisor's messages
/// (C3POACT.SPT / IMP22ACT.SPT), whose game-over clip starts the ending
/// (`FUN_004c3060` kind 3 -> `FUN_00425c90`).
#[must_use]
pub fn has_the_advisor(request: FixtureRequest) -> bool {
    runs_the_clock(request) || request.scenario == Scenario::EndingAssaultCoruscant
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
        is_alliance: player_is_alliance,
        ..Default::default()
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
    if matches!(
        request.scenario,
        Scenario::SpecForcesYavin
            | Scenario::PersonnelAbode
            | Scenario::PersonnelPalace
            | Scenario::PersonnelRetired
            | Scenario::PersonnelPickup
    ) {
        emit_finder_list(request, world, fog, finder);
        return;
    }
    if !matches!(
        request.scenario,
        Scenario::PersonnelStates | Scenario::PersonnelCopied
    ) {
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
        /// The galaxy's abode names, for a row that names one alone.
        abodes: Vec<String>,
    }
    let bytes = serde_json::to_vec(&Report {
        status: "personnel-finder",
        rows,
        abodes: world
            .abodes
            .values()
            .map(|abode| abode.name.clone())
            .collect(),
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
/// travelling special force's in SpecialForceEnroute, the one aboard a
/// fleet's in SpecialForceAboard, regiment's in
/// RegimentEnroute and the receiving fleet's or ship's in FleetArrivals
/// (with the picture's marks), the hit ship's in ShipShieldHit, and an
/// unfinished unit's or a producer's in any.
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
        (
            Scenario::SpecialForceEnroute | Scenario::SpecialForceAboard,
            Some(object @ StatusObject::SpecialForce(_)),
        )
        | (Scenario::RegimentEnroute, Some(object @ StatusObject::Troop(_)))
        | (
            Scenario::EnemyRegimentFrozen | Scenario::EnemyRegimentAtRest,
            Some(object @ StatusObject::ViewedTroop { .. }),
        )
        | (
            Scenario::FleetArrivals,
            Some(object @ (StatusObject::Ship { .. } | StatusObject::Fleet(_))),
        )
        | (
            Scenario::ShipShieldHit | Scenario::ShipRepaired,
            Some(object @ StatusObject::Ship { .. }),
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
        | (Scenario::QuadrantsNeutralFleet, Some(object @ StatusObject::PlanetFighter { .. }))
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
        /// The game day, to read an ETA against.
        today: u64,
    }
    let today = sources.today;
    let bytes = serde_json::to_vec(&Report {
        status: "status-rows",
        title,
        name,
        rows: rows.into_iter().map(|row| (row.label, row.value)).collect(),
        marks,
        today,
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
    if request.scenario.is_new_game() {
        // As a new game seeds each side's fog: where its fleets stand.
        rebellion_core::fog::FogSystem::seed(fog_alliance, world);
        rebellion_core::fog::FogSystem::seed(fog_empire, world);
        return;
    }
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

/// The new-game scenarios ([`Scenario::is_new_game`]), after the views
/// are seeded from the fog as a new game seeds them: the setup push
/// (`object_views::setup`, `FUN_0051b990`) and the day's view step. Then
/// the Triton scenarios move the player's first fleet:
/// - `TritonEnroute`: into hyperspace to Triton;
/// - `TritonArrived`: into Triton's orbit, and the views step (live copy);
/// - `TritonStale`: as `TritonArrived`, then on to Kabal, the views step
///   (Triton's copy freezes), and Triton's own energy and raw materials
///   change.
pub fn place_new_game(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &mut MovementState,
    transport: &TroopTransportState,
    missions: &MissionState,
) {
    if !request.scenario.is_new_game() {
        return;
    }
    let step = |world: &mut GameWorld, movement: &MovementState| {
        rebellion_core::object_views::advance_extras(world, movement, missions, transport);
        rebellion_core::object_views::advance(world, transport, movement);
    };
    rebellion_core::object_views::setup(world);
    step(world, movement);
    if request.scenario == Scenario::NewGame {
        return;
    }
    let named = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let (Some(triton), Some(kabal)) = (named(world, "Triton"), named(world, "Kabal")) else {
        return;
    };
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let Some(fleet) = world
        .fleets
        .iter()
        .find(|(_, value)| {
            value.is_alliance == player_is_alliance && !value.capital_ships.is_empty()
        })
        .map(|(key, _)| key)
    else {
        return;
    };
    if request.scenario == Scenario::TritonEnroute {
        begin_fleet_transit(movement, world, fleet, triton, 100_000);
        step(world, movement);
        return;
    }
    world.fleets[fleet].location = triton;
    reconcile_fleet_orbits(movement, world);
    step(world, movement);
    if request.scenario == Scenario::TritonStale {
        world.fleets[fleet].location = kabal;
        reconcile_fleet_orbits(movement, world);
        step(world, movement);
        let system = &mut world.systems[triton];
        system.total_energy = system.total_energy / 2 + 1;
        system.raw_materials = system.raw_materials / 2 + 1;
        step(world, movement);
    }
}

/// `QuadrantsHeadquartersHidden`: the Empire's fog takes the Alliance's
/// headquarters system after the views are seeded, so the Empire has no
/// copy of it, as a passing fleet's sensor reach reveals a system in play
/// (`fog::FogSystem::advance`). Returns the system revealed.
pub fn reveal_without_copy(
    request: FixtureRequest,
    world: &GameWorld,
    fog_empire: &mut FogState,
) -> Option<SystemKey> {
    if request.scenario != Scenario::QuadrantsHeadquartersHidden
        || request.faction == CockpitFaction::Alliance
    {
        return None;
    }
    let primary = world.systems.keys().next()?;
    let headquarters = *mark_systems(world, primary).first()?;
    fog_empire.reveal(headquarters);
    Some(headquarters)
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
                | Scenario::JediTrainingDialog
                | Scenario::ReconTargeting
                | Scenario::ResearchTargeting
                | Scenario::TroopResearchTargeting
                | Scenario::FacilityResearchTargeting
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
/// The primary system's room (`FUN_00507750`: energy less its facilities,
/// raw materials less its mines) for the Build Selection gates.
/// `BuildSelectionOverMaintenance` keeps one free slot and deposit, so its
/// mine and refinery, which cost no maintenance (`FUN_0052e970`), stay
/// buildable. `BuildSelectionFacilityRoom` queues one Construction Yard
/// on the primary's own facilities band and leaves no slot or deposit
/// past it.
fn place_facility_room(
    request: FixtureRequest,
    world: &mut GameWorld,
    manufacturing: &mut ManufacturingState,
    primary: SystemKey,
) {
    use rebellion_core::manufacturing::{BuildableKind, FacilityBuild, QueueItem};
    let full = request.scenario == Scenario::BuildSelectionFacilityRoom;
    if full {
        let build = FacilityBuild {
            class: rebellion_core::ids::DatId::new(0x2a00_0001),
            is_alliance: request.faction == CockpitFaction::Alliance,
        };
        manufacturing.enqueue(
            primary,
            QueueItem::new(BuildableKind::ManufacturingFacility(build), 10),
        );
    }
    let system = &mut world.systems[primary];
    let mines = system
        .production_facilities
        .iter()
        .filter(|key| {
            world
                .production_facilities
                .get(**key)
                .is_some_and(|value| value.is_mine)
        })
        .count();
    let facilities = system.defense_facilities.len()
        + system.manufacturing_facilities.len()
        + system.production_facilities.len()
        + usize::from(full);
    let free = usize::from(!full);
    system.total_energy = u8::try_from(facilities + free).unwrap_or(u8::MAX);
    system.raw_materials = u8::try_from(mines + free).unwrap_or(u8::MAX);
}

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
        // No fleet in the sector's unexplored systems: a side's fleet there
        // would see them explored (FUN_0050ac80, FUN_004f9a60;
        // ghidra/notes/fog-of-war.md), which A0 B2 does not show. They go to
        // Kubindi, which A0 shows explored.
        let unexplored: Vec<SystemKey> = world
            .systems
            .iter()
            .filter(|(key, system)| {
                *key != kubindi
                    && system.sector == sector
                    && system.exploration_status == ExplorationStatus::Unexplored
            })
            .map(|(key, _)| key)
            .collect();
        for from in unexplored {
            let fleets = std::mem::take(&mut world.systems[from].fleets);
            for fleet in &fleets {
                if let Some(value) = world.fleets.get_mut(*fleet) {
                    value.location = kubindi;
                }
            }
            world.systems[kubindi].fleets.extend(fleets);
        }
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

/// The `BlockadeRunning` scenario: the primary system becomes the other
/// side's, held by its support (`FUN_0050b610`), its defense facilities
/// moved to the loading target (an active KDY-150 keeps the withdraw percent
/// at 100, `FUN_0050b310`). The player's fleet, blockading it, holds copies
/// of its ship until `FUN_0055a020` leaves `100 - ships * GNPRTB[7684]` at 0,
/// and two of the player's regiments aboard. Each holds that 0, as a
/// regiment does whose own Move brought it onto the fleet there
/// (`FUN_00514a60` -> `FUN_00508660` -> `FUN_0050c540`).
fn place_blockade_running(
    request: FixtureRequest,
    world: &mut GameWorld,
    blockade: &mut BlockadeState,
    transport: &mut TroopTransportState,
    primary: SystemKey,
) {
    let player_is_alliance = request.faction == CockpitFaction::Alliance;
    let (Some(target), Some(fleet), Some(regiment)) = (
        loading_target(world, primary),
        world.fleets.keys().next(),
        loading_regiment(world, primary),
    ) else {
        return;
    };
    let enemy = if player_is_alliance {
        Faction::Empire
    } else {
        Faction::Alliance
    };
    let system = &mut world.systems[primary];
    system.control = ControlKind::Controlled(enemy);
    system.is_populated = true;
    system.popularity_alliance = if player_is_alliance { 0.0 } else { 1.0 };
    system.popularity_empire = 1.0 - system.popularity_alliance;
    let defenses = std::mem::take(&mut system.defense_facilities);
    world.systems[target].defense_facilities.extend(defenses);
    let per_ship = world.gnprtb.value(
        rebellion_core::blockade::GNPRTB_WITHDRAW_PENALTY_PER_CAPSHIP,
        world.difficulty_index,
    );
    let full = i32::from(rebellion_core::blockade::FULL_WITHDRAW_PERCENT);
    let ships = if per_ship > 0 {
        usize::try_from((full + per_ship - 1) / per_ship).unwrap_or(1)
    } else {
        1
    };
    if let Some(ship) = world.fleets[fleet].capital_ships.first().cloned() {
        world.fleets[fleet].capital_ships.resize(ships.max(1), ship);
    }
    let second = world.troops[regiment].clone();
    let second = world.troops.insert(second);
    world.systems[primary].ground_units.push(second);
    let _ = transport.embark(world, fleet, &[regiment, second]);
    let percent = BlockadeSystem::withdraw_percent(world, primary);
    for troop in [regiment, second] {
        blockade.set_withdraw_percent(troop, percent);
    }
    let _ = BlockadeSystem::advance(blockade, world, &[TickEvent { tick: 1 }]);
}

/// The `SquadronBlockaded` scenario: a copy of the player's fleet, turned
/// the other side's, alone in orbit of the loading target, the player's,
/// which it blockades (`FUN_0050b8e0`).
fn place_blockaded_target(
    request: FixtureRequest,
    world: &mut GameWorld,
    movement: &mut MovementState,
    blockade: &mut BlockadeState,
    primary: SystemKey,
) {
    let (Some(target), Some(fleet)) = (loading_target(world, primary), world.fleets.keys().next())
    else {
        return;
    };
    let mut enemy = world.fleets[fleet].clone();
    enemy.is_alliance = request.faction != CockpitFaction::Alliance;
    enemy.location = target;
    enemy.characters.clear();
    enemy.special_forces.clear();
    enemy.fighters.clear();
    world.fleets.insert(enemy);
    world.systems[target].fighters.clear();
    reconcile_fleet_orbits(movement, world);
    let _ = BlockadeSystem::advance(blockade, world, &[TickEvent { tick: 1 }]);
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
/// The `ShipShieldHit` scenario's ship (A0 2026-11-15-partial-cells e-vd2,
/// 8bce5f1cbf27): the loading fleet's ship at its class's full hull with
/// one hit against its shield recharge (`+0x64` bits 0..3). For the Empire
/// it is the Victory Destroyer class, named Victory Destroyer 2 as in A0.
fn hit_the_shield_recharge(request: FixtureRequest, world: &mut GameWorld) {
    let Some(fleet) = world.fleets.keys().next() else {
        return;
    };
    let victory = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| !class.is_alliance && class.name == "Victory Destroyer")
        .map(|(key, _)| key)
        .filter(|_| request.faction == CockpitFaction::Empire);
    let Some(ship) = world.fleets[fleet].capital_ships.first().cloned() else {
        return;
    };
    let class = victory.unwrap_or(ship.class);
    let hull = world.capital_ship_classes[class].hull as i32;
    let mut hit = ShipInstance::new(class, hull, request.faction == CockpitFaction::Alliance);
    hit.set_hits(rebellion_core::world::Subsystem::Shield, 1);
    if victory.is_some() {
        hit.name = Some("Victory Destroyer 2".into());
    }
    world.fleets[fleet].capital_ships[0] = hit;
}

/// The `ShipRepaired` scenario: an Orbital Shipyard (MANFACSD
/// `0x28000001`) of the player's side at the hit ship's system, which the
/// side holds, then the days the port's repair takes. Each day runs
/// `RepairSystem::advance` (timer `0x380`, FastRepair `FUN_0050c580` /
/// `FUN_0050bb70`; each fire `FUN_00501c50`) and the integrator's repair,
/// until the ship is whole. Returns the days run.
fn repair_at_a_shipyard(request: FixtureRequest, world: &mut GameWorld) -> u64 {
    let Some(fleet) = world.fleets.keys().next() else {
        return 0;
    };
    let side = if request.faction == CockpitFaction::Alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    let at = world.fleets[fleet].location;
    if world.fleets[fleet].capital_ships[0].id == 0 {
        world.last_ship_id += 1;
        world.fleets[fleet].capital_ships[0].id = world.last_ship_id;
    }
    world.systems[at].control = ControlKind::Controlled(side);
    let yard = world.manufacturing_facilities.insert(
        rebellion_core::world::ManufacturingFacilityInstance {
            class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
            side,
            is_shipyard: true,
        },
    );
    world.systems[at].manufacturing_facilities.push(yard);
    let mut state = rebellion_core::repair::RepairState::default();
    for day in 1..=600 {
        let ship = &world.fleets[fleet].capital_ships[0];
        let hull = world.capital_ship_classes[ship.class].hull as i32;
        if ship.subsystem_hits == 0 && ship.hull_current >= hull {
            return day - 1;
        }
        let events = rebellion_core::repair::RepairSystem::advance(
            &mut state,
            world,
            &[TickEvent { tick: day }],
        );
        rebellion_data::integrator::PerceptionIntegrator::new(day, 0)
            .apply_repair_events(world, &events);
    }
    600
}

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
        Scenario::SquadronLoad | Scenario::FleetArrivals | Scenario::SquadronBlockaded
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
    if matches!(
        request.scenario,
        Scenario::SquadronLoad | Scenario::SquadronBlockaded
    ) {
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
    if request.scenario == Scenario::GravityWellBattle {
        place_gravity_well_fleet(world, movement, primary, target, player_is_alliance);
    }
    if matches!(
        request.scenario,
        Scenario::FleetBombardment
            | Scenario::FleetAssault
            | Scenario::FleetAssaultTaken
            | Scenario::EndingAssaultCoruscant
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
        Scenario::FleetAssault
            | Scenario::FleetAssaultTaken
            | Scenario::EndingAssaultCoruscant
    ) {
        let key = world.troops.insert(regiment());
        world.systems[primary].ground_units.push(key);
        let _ = troop_transport.embark(world, fleet, &[key]);
    }
    // The taken variant: six of the player's regiments aboard, as A0
    // messages-day633's Alliance Forces list them (its bar's 57-pixel thumb
    // is six 70-pixel rows), on copies of the fleet's ship enough to carry
    // them; the other side's regiments leave the planet.
    if matches!(
        request.scenario,
        Scenario::FleetAssaultTaken | Scenario::EndingAssaultCoruscant
    ) {
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

/// The `GravityWellBattle` variant: the planets' squadrons gone, and a
/// fleet of the other side at `primary` with one ship of that side's first
/// class that projects a gravity well (`FUN_00516730` reads the class's
/// `+0xa4`, `ghidra/notes/abstract-battle.md`).
fn place_gravity_well_fleet(
    world: &mut GameWorld,
    movement: &mut MovementState,
    primary: SystemKey,
    target: SystemKey,
    player_is_alliance: bool,
) {
    for system in [primary, target] {
        world.systems[system].fighters.clear();
    }
    let Some((class, hull)) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| {
            class.is_alliance != player_is_alliance && class.gravity_well_projector != 0
        })
        .map(|(key, class)| (key, class.hull as i32))
    else {
        return;
    };
    world.insert_fleet(rebellion_core::world::Fleet {
        location: primary,
        capital_ships: vec![ShipInstance::new(class, hull, !player_is_alliance)],
        is_alliance: !player_is_alliance,
        ..Default::default()
    });
    world.number_ships();
    reconcile_fleet_orbits(movement, world);
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
    if request.scenario == Scenario::QuadrantsNeutralFleet {
        place_neutral_fleet(world, fleet, primary, elsewhere, player_is_alliance);
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
    if request.scenario == Scenario::QuadrantsFacility {
        let yard = world.manufacturing_facilities.insert(
            rebellion_core::world::ManufacturingFacilityInstance {
                class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
                side: player,
                is_shipyard: true,
            },
        );
        world.systems[primary].manufacturing_facilities.push(yard);
    }
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
        // QuadrantsFreeTeam: the player's agent at the primary stays free.
        if matches!(
            request.scenario,
            Scenario::QuadrantsFreeTeam
                | Scenario::QuadrantsRegimentQueued
                | Scenario::QuadrantsRegimentEnRoute
        ) && system == primary
            && alliance == player_is_alliance
        {
            continue;
        }
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
                | Scenario::BuildSelectionFacilityRoom
                | Scenario::Quadrants
                | Scenario::QuadrantsNeutralFleet
                | Scenario::QuadrantsFreeTeam
                | Scenario::QuadrantsRegimentQueued
                | Scenario::QuadrantsRegimentEnRoute
                | Scenario::QuadrantsHeadquartersFrozen
                | Scenario::QuadrantsHeadquartersHidden
                | Scenario::QuadrantsHeadquartersLive
                | Scenario::QuadrantsFacility
                | Scenario::QuadrantsStale
                | Scenario::QuadrantsStaleSeeEverything
                | Scenario::EnemyRegimentFrozen
                | Scenario::EnemyRegimentAtRest
                | Scenario::FleetWindowYavin
                | Scenario::FleetWindowA0
                | Scenario::SpecialForceAboard
                | Scenario::BatteryStatus
                | Scenario::PersonnelStates
                | Scenario::PersonnelAbode
                | Scenario::PersonnelRetired
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
    /// Whether Confirm is enabled: `FUN_00439160` disables it when
    /// `FUN_00538220` refuses the order, the four values then "n/a".
    build_selection_confirm_enabled: Option<bool>,
    /// Whether the fleet or ship the band's Destination names has no room
    /// for the order's squadrons or regiments
    /// (`build_selection::lacks_fleet_room`, the planner's `0x27`).
    build_selection_lacks_fleet_room: Option<bool>,
    build_selection_index: Option<usize>,
    /// The shown class's name (`FUN_00437880`).
    build_selection_class: Option<String>,
    /// The Training Facilities tab and its page's first cell, as
    /// `[left, top, width, height]`.
    training_tab: Option<[f32; 4]>,
    first_cell: Option<[f32; 4]>,
    /// The primary's System icon in the open sector window, whose double
    /// click opens its System window again.
    system_icon: Option<[f32; 4]>,
}

fn production_report(
    request: FixtureRequest,
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    en_route: &[rebellion_core::delivery::Delivery],
    systems: &SystemWindowState,
    sectors: &SectorWindowState,
    build_selection: &rebellion_render::build_selection::BuildSelectionState,
    carried: &dyn Fn(rebellion_core::ids::FleetKey) -> usize,
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
                    // The product line as the open window last built it.
                    [
                        systems
                            .band_status(primary, area)
                            .map_or(band.status, str::to_owned),
                        band.units.unwrap_or_default(),
                        band.destination,
                    ]
                },
            })
        })
        .collect();
    let window = rebellion_render::build_selection::window_rect(layout);
    let ltwh = |rect: egui_macroquad::egui::Rect| [rect.min.x, rect.min.y, rect.width(), rect.height()];
    // The gate opens it on the primary system's band, for the player.
    let build_selection_estimate = build_selection.dialog().map(|dialog| {
        let is_alliance = request.faction == CockpitFaction::Alliance;
        let side = rebellion_core::dat::Faction::of_alliance(is_alliance);
        let unallocated =
            rebellion_core::side_resources::maintenance_totals(world, manufacturing, en_route, side, |_| {
                false
            })
            .unallocated();
        rebellion_core::build_selection::estimate(
            world,
            manufacturing,
            en_route,
            primary,
            dialog.area(),
            dialog.kind(),
            dialog.quantity(),
            is_alliance,
            unallocated,
            carried,
        )
    });
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
        build_selection_confirm_enabled: build_selection_estimate.map(|e| e.completion.is_some()),
        build_selection_lacks_fleet_room: build_selection.dialog().map(|dialog| {
            rebellion_core::build_selection::lacks_fleet_room(
                world,
                manufacturing,
                (primary, dialog.area()),
                dialog.kind(),
                dialog.quantity(),
                carried,
            )
        }),
        build_selection_values: build_selection_estimate.map(|estimate| {
            use rebellion_render::build_selection::{cost_text, days_text};
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
        system_icon: sectors
            .quadrant_screen_rect(world, layout, primary, Quadrant::System)
            .map(ltwh),
    })
}

pub fn emit_production(
    request: FixtureRequest,
    world: &GameWorld,
    manufacturing: &ManufacturingState,
    en_route: &[rebellion_core::delivery::Delivery],
    systems: &SystemWindowState,
    sectors: &SectorWindowState,
    build_selection: &rebellion_render::build_selection::BuildSelectionState,
    carried: &dyn Fn(rebellion_core::ids::FleetKey) -> usize,
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
    let Some(report) = production_report(
        request,
        world,
        manufacturing,
        en_route,
        systems,
        sectors,
        build_selection,
        carried,
    )
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
    if let Some(message) = rebellion_render::story_messages::jabba_delivers_message(false, 0, texts) {
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

/// The `CaptureMessages` scenario's messages, oldest first, so the index
/// lists them as A0 2026-11-17-edited-saves-6 does (an Alliance game whose
/// Han Solo the Empire took at Yavin, day 368).
///
/// The Alliance reads, newest first as d-07's All Messages: Corfai and
/// Vagran Joins Enemy (`FUN_00499760`), Luke Goes to Dagobah
/// (`FUN_0048d190`, d-11), Enemy Mission Foiled (`FUN_00491500`, an
/// Imperial Espionage at Yavin, d-10), Han Solo Captured (`FUN_00490340`
/// case 3, d-08), Han Solo Injured (`FUN_0048fdf0`, d-09) and Enemy
/// Mission Foiled again.
///
/// The Empire reads Chewbacca taken at Yavin (`FUN_00490340` case 4,
/// TEXTSTRA `0x715a` / `0x715b`) as A0 2026-11-18-edited-saves-9 a-07
/// shows it in a genuine, unedited Empire game ("Chewbacca was captured
/// at Yavin.", after Vader's Abduction; a-06 lists the row).
pub fn post_capture_messages(
    request: FixtureRequest,
    world: &mut GameWorld,
    log: &mut rebellion_render::MessageLog,
    texts: &std::collections::HashMap<u16, String>,
) {
    if request.scenario != Scenario::CaptureMessages {
        return;
    }
    let system = |world: &GameWorld, name: &str| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let character = |world: &GameWorld, name: &str| {
        world
            .characters
            .iter()
            .find(|(_, value)| value.name == name)
            .map(|(key, _)| key)
    };
    let Some(yavin) = system(world, "Yavin") else {
        return;
    };
    if request.faction == CockpitFaction::Empire {
        if let Some(chewbacca) = character(world, "Chewbacca") {
            if let Some(message) = rebellion_render::story_messages::captured_message(
                Some(yavin),
                &world.characters[chewbacca],
                false,
                "Yavin",
                request.faction,
                0,
                texts,
            ) {
                log.push(message);
            }
        }
        return;
    }
    // push_foiled_missions reads no mission id.
    let foiled = rebellion_core::missions::MissionEnded {
        mission_id: u64::MAX,
        kind: MissionKind::Espionage,
        faction: MissionFaction::Empire,
        character: None,
        target_system: yavin,
        end_code: 7,
        tick: 0,
        returning_to: None,
    };
    crate::push_foiled_missions(log, world, std::slice::from_ref(&foiled), texts);
    if let Some(han) = character(world, "Han Solo") {
        world.characters[han].injury = 1;
        crate::push_character_health(log, world, han, 0, texts);
        world.characters[han].injury = 0;
        if let Some(message) = rebellion_render::story_messages::captured_message(
            Some(yavin),
            &world.characters[han],
            false,
            "Yavin",
            request.faction,
            0,
            texts,
        ) {
            log.push(message);
        }
    }
    crate::push_foiled_missions(log, world, &[foiled], texts);
    if let Some(message) = rebellion_render::story_messages::luke_goes_to_dagobah(0, texts) {
        log.push(message);
    }
    for name in ["Vagran", "Corfai"] {
        if let Some(at) = system(world, name) {
            if let Some(message) = crate::system_joins_message(
                world,
                at,
                Some(Faction::Alliance),
                Some(Faction::Empire),
                true,
                0,
                texts,
            ) {
                log.push(message);
            }
        }
    }
}

/// The `JabbaDelivers` scenario: Luke's (`0x32000242`) DagobahCompleted
/// set for the Empire's reader (A0 2026-11-18-edited-saves-9 c-13: an
/// Empire game whose Luke had left Dagobah, 0x42b) and clear for the
/// Alliance's (2026-11-18-edited-saves-7 b-12: 0x42a), then Jabba
/// Delivers Prisoners posted as the Pickup posts it (`FUN_0048b6e0`).
pub fn post_jabba_delivers(
    request: FixtureRequest,
    world: &mut GameWorld,
    log: &mut rebellion_render::MessageLog,
    texts: &std::collections::HashMap<u16, String>,
) {
    if request.scenario != Scenario::JabbaDelivers {
        return;
    }
    let trained = request.faction == CockpitFaction::Empire;
    for character in world.characters.values_mut() {
        if character.dat_id.index() == 0x242 {
            character.dagobah_completed = trained;
        }
    }
    if let Some(message) = crate::jabba_delivers_message(world, 0, texts) {
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

/// The event-message scenario's Natural Disaster for the Alliance reader
/// (`FUN_0048e940`, notification `0x154`, TEXTSTRA `0x7190` / `0x7191`,
/// EVT-01-C002): Yavin, held by the Alliance, losing resources only, as A0
/// 2026-11-15-partial-cells/r2-msg-natural-disaster-on-yavin-d824 (sha256
/// bc967b60eb32) shows "Natural Disaster on Yavin" over "There has been a
/// natural disaster on Yavin which has destroyed resources there.".
#[must_use]
pub fn disaster_system(
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

/// The `UprisingMessages` scenario's Uprising messages (`FUN_00499460`),
/// oldest first: the system, its holder when the message is filed (the
/// class reads the system's side, `+0x24` bits 6..7) and whether the
/// uprising begins. A0 2026-11-16-empire-hunt: Selonia's begins on day 153
/// while the Empire holds it (h-38, "the Imperial controlled system");
/// its end on day 299 shows the Alliance system's picture `0x3f3` (h-34),
/// Selonia then the enemy's (the Empire's informants report on it, h-36);
/// Yavin's begins after the Empire took the Alliance's HQ system (h-50).
pub fn uprising_messages(request: FixtureRequest) -> Vec<(&'static str, Faction, bool)> {
    if request.scenario != Scenario::UprisingMessages {
        return Vec::new();
    }
    vec![
        ("Selonia", Faction::Empire, true),
        ("Selonia", Faction::Alliance, false),
        ("Yavin", Faction::Empire, true),
    ]
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
/// Empire; then an Imperial Probe Droid's Reconnaissance of Chandrila
/// succeeds, as A0 2026-11-16-partial-cells/r-8, r-9 (OBJ-11-C011) report
/// it. The app files them through `apply_mission_result`.
pub fn mission_results(
    request: FixtureRequest,
    world: &mut GameWorld,
) -> Vec<rebellion_core::missions::MissionResult> {
    if request.scenario == Scenario::JediTrainingMessages {
        return jedi_training_result(world).into_iter().collect();
    }
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
        lead_force: None,
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
    // SPECFCSD class 5, the Imperial Probe Droid, on its mission; its
    // team has no character, so it leads the report (FUN_004927c0 +0x74).
    let droid = world
        .special_forces
        .insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0005),
            is_alliance: false,
            skills: [0; 8],
            on_mission: true,
        });
    let recon = rebellion_core::missions::MissionResult {
        mission_id: u64::MAX - 5,
        tick: 0,
        kind: MissionKind::Reconnaissance,
        faction: MissionFaction::Empire,
        character: None,
        lead_force: Some(droid),
        target_system: chandrila,
        target_character: None,
        target_object: None,
        returning_to: Some(rebellion_core::missions::ReturnPlace::System(coruscant)),
        outcome: rebellion_core::missions::MissionOutcome::Success,
        effects: Vec::new(),
    };
    vec![recon, recruited, rebellion_core::missions::MissionResult {
        mission_id: u64::MAX - 1,
        tick: 0,
        kind: MissionKind::Espionage,
        faction: MissionFaction::Empire,
        character: Some(emperor),
        lead_force: None,
        target_system: chandrila,
        target_character: None,
        target_object: None,
        returning_to: Some(rebellion_core::missions::ReturnPlace::System(coruscant)),
        outcome: rebellion_core::missions::MissionOutcome::Failure,
        effects: Vec::new(),
    }]
}

/// The `JediTrainingMessages` result: Luke's Jedi Training on Yavin ends a
/// success, Dodonna's Force raised by 15 into ForceTraining (`FUN_005712b0`
/// -> `FUN_004ee1d0`), from 10 (Novice) to 25, a Trainee as A0
/// 2026-11-16-edited-saves/x-60 reads it. The report names Luke, its
/// leader, and Yavin (x-59: "My Jedi Training mission on Yavin was a
/// success."); the members stay on Yavin, so nobody returns.
fn jedi_training_result(world: &GameWorld) -> Option<rebellion_core::missions::MissionResult> {
    let (luke, dodonna, yavin) = jedi_training_cast(world)?;
    Some(rebellion_core::missions::MissionResult {
        mission_id: u64::MAX - 6,
        tick: 0,
        kind: MissionKind::JediTraining,
        faction: MissionFaction::Alliance,
        character: Some(luke),
        lead_force: None,
        target_system: yavin,
        target_character: None,
        target_object: None,
        returning_to: None,
        outcome: rebellion_core::missions::MissionOutcome::Success,
        effects: vec![rebellion_core::missions::MissionEffect::ForceTrainingGained {
            character: dodonna,
            amount: 15,
        }],
    })
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
            returning_to: None,
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
            returning_to: None,
        },
    ]
    .into_iter()
    .chain(own_failed_mission(request, world, chandrila))
    .collect()
}

/// The player's own Espionage at Chandrila ended with code 6, its target
/// not found (`FUN_00491500`: Mission Failed from its leader): Leia for
/// the Alliance, Vader for the Empire.
fn own_failed_mission(
    request: FixtureRequest,
    world: &GameWorld,
    chandrila: SystemKey,
) -> Option<rebellion_core::missions::MissionEnded> {
    let alliance = request.faction == CockpitFaction::Alliance;
    let leader = world
        .characters
        .iter()
        .find(|(_, c)| c.dat_id.index() == if alliance { 0x241 } else { 0x281 })
        .map(|(key, _)| key)?;
    Some(rebellion_core::missions::MissionEnded {
        mission_id: u64::MAX - 5,
        kind: MissionKind::Espionage,
        faction: if alliance {
            MissionFaction::Alliance
        } else {
            MissionFaction::Empire
        },
        character: Some(leader),
        target_system: chandrila,
        end_code: 6,
        tick: 0,
        returning_to: None,
    })
}

/// The EventMessages scenario's Recruitment Done (`FUN_0048ae40`): the
/// player's side, Alliance true.
pub fn recruitment_done(request: FixtureRequest) -> Option<bool> {
    (request.scenario == Scenario::EventMessages).then_some(request.faction == CockpitFaction::Alliance)
}

/// The EventMessages scenario's Capital Ship Repaired (`FUN_00499de0`): the
/// player's first fleet with a ship, and that ship.
pub fn repaired_ship(request: FixtureRequest, world: &GameWorld) -> Option<(FleetKey, usize)> {
    if request.scenario != Scenario::EventMessages {
        return None;
    }
    let alliance = request.faction == CockpitFaction::Alliance;
    world
        .fleets
        .iter()
        .find(|(_, fleet)| fleet.is_alliance == alliance && !fleet.capital_ships.is_empty())
        .map(|(key, _)| (key, 0))
}

/// The EventMessages scenario's Squadron at Full Strength (`FUN_00499de0`
/// for a squadron, `+0x28` 9): the player's first squadron aboard a fleet,
/// else its first on a planet.
pub fn full_strength_squadron(
    request: FixtureRequest,
    world: &GameWorld,
) -> Option<(rebellion_core::squadrons::SquadronPlace, usize)> {
    use rebellion_core::squadrons::SquadronPlace;
    if request.scenario != Scenario::EventMessages {
        return None;
    }
    let alliance = request.faction == CockpitFaction::Alliance;
    let own = |entry: &rebellion_core::world::FighterEntry| {
        entry.count > 0
            && rebellion_core::squadrons::is_alliance(world, entry.class) == Some(alliance)
    };
    let aboard = world.fleets.iter().find_map(|(key, fleet)| {
        let index = fleet.fighters.iter().position(own)?;
        Some((SquadronPlace::Fleet(key), index))
    });
    aboard.or_else(|| {
        world.systems.iter().find_map(|(key, system)| {
            let index = system.fighters.iter().position(own)?;
            Some((SquadronPlace::Planet(key), index))
        })
    })
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
    let alliance = reader == CockpitFaction::Alliance;
    // Each post's display sounds (ghidra/notes/message-sounds.md), as its
    // class writes them.
    use rebellion_render::message_sound::MessageSounds;
    let sounds = |title: &str| match title {
        // FUN_004927c0: a Diplomacy success led by Leia.
        "Mission Report" => MessageSounds::mission_report(
            ReportedMission::Diplomacy,
            true,
            alliance,
            0x241,
            false,
        ),
        // FUN_00499460.
        "Uprising Begins" => MessageSounds::uprising(true),
        "Uprising Ends" => MessageSounds::uprising(false),
        // FUN_004960f0.
        "Blockade" => MessageSounds::blockade(),
        // FUN_004981c0.
        "Fleet Arrives" => MessageSounds::arrival(Arrival::Fleet, alliance),
        "Units Arrive" => MessageSounds::arrival(Arrival::Units, alliance),
        // FUN_0048fdf0.
        "Character Killed" => MessageSounds::character_health(
            rebellion_render::story_messages::CharacterHealth::Killed,
            0,
            false,
        ),
        // FUN_00499aa0.
        "Research Complete" => MessageSounds::research(),
        // FUN_00497940: neither a shipyard nor a regiment sounds.
        _ => MessageSounds::NONE,
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
            .with_sounds(sounds(title))
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
        Scenario::StoryMessages
            | Scenario::EventMessages
            | Scenario::HeadquartersTaken
            | Scenario::UprisingMessages
            | Scenario::JediTrainingMessages
            | Scenario::GhormanBombardment
            | Scenario::GhormanAssault
            | Scenario::CaptureMessages
            | Scenario::JabbaDelivers
    ) {
        return None;
    }
    // The assault report is filed on the Conflict rail (FUN_00497530), an
    // uprising on Popular Support (FUN_00499460's class, mask 0x001).
    let rail_of = match request.scenario {
        Scenario::HeadquartersTaken | Scenario::GhormanBombardment | Scenario::GhormanAssault => {
            rebellion_render::MessageRail::Conflict
        }
        Scenario::UprisingMessages => rebellion_render::MessageRail::PopularSupport,
        _ => rebellion_render::story_messages::STORY_RAIL,
    };
    let layout = CockpitState::new(request.faction).layout_for(640.0, 480.0);
    let rail = rebellion_render::cockpit::strategic_message_index_controls(request.faction)
        .iter()
        .zip(rebellion_render::MessageRail::RAIL_ORDER)
        .find(|(_, rail)| *rail == rail_of)
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
    /// The neutral system's fleet icon rect (`QuadrantsNeutralFleet`).
    neutral_fleets: Option<[f32; 4]>,
    /// A destroyed system's rect (A0 menus3/05: Averam's asteroid field
    /// STRATEGY 10240 and its white name).
    destroyed: Option<[f32; 4]>,
    /// The day readout's centre, which opens the Game Speed menu.
    day_readout: (f32, f32),
    /// The headquarters' trip in days to the system in uprising
    /// (`headquarters_move::plan`, `FUN_00555b30`), when its side may go.
    headquarters_trip_days: Option<u32>,
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

/// The `QuadrantsNeutralFleet` variant: the player's fleet alone at the
/// neutral mark system, its one ship of the player's first class that
/// bombards (no regiments aboard, so Planetary Assault stays grey), no
/// squadrons aboard; and one of the player's squadrons, of its side's first
/// fighter class, alone on the primary planet.
fn place_neutral_fleet(
    world: &mut GameWorld,
    fleet: Option<FleetKey>,
    primary: SystemKey,
    elsewhere: SystemKey,
    player_is_alliance: bool,
) {
    let (Some(fleet), Some(&neutral)) = (fleet, mark_systems(world, primary).get(2)) else {
        return;
    };
    for (key, value) in &mut world.fleets {
        if key != fleet && value.location == neutral {
            value.location = elsewhere;
        }
    }
    let value = &mut world.fleets[fleet];
    value.location = neutral;
    value.fighters.clear();
    if let Some((class, hull)) = world
        .capital_ship_classes
        .iter()
        .find(|(_, class)| {
            class.is_alliance == player_is_alliance
                && class.bombardment_modifier > 0
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
    world.systems[primary].fighters.clear();
    if let Some(class) = world
        .fighter_classes
        .iter()
        .find(|(_, class)| class.is_alliance == player_is_alliance)
        .map(|(key, _)| key)
    {
        rebellion_core::squadrons::add(&mut world.systems[primary].fighters, class, 1);
    }
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
    let marks = if matches!(
        request.scenario,
        Scenario::Quadrants
            | Scenario::QuadrantsNeutralFleet
            | Scenario::QuadrantsFreeTeam
            | Scenario::QuadrantsRegimentQueued
            | Scenario::QuadrantsRegimentEnRoute
            | Scenario::QuadrantsHeadquartersFrozen
            | Scenario::QuadrantsHeadquartersHidden
            | Scenario::QuadrantsHeadquartersLive
            | Scenario::QuadrantsFacility
            | Scenario::QuadrantsStale
            | Scenario::QuadrantsStaleSeeEverything
    ) {
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
        neutral_fleets: marks.get(2).and_then(|&system| {
            let icon = sectors.quadrant_screen_rect(world, layout, system, Quadrant::Fleets)?;
            Some([icon.min.x, icon.min.y, icon.width(), icon.height()])
        }),
        destroyed: planet(marks.get(3)),
        day_readout: day_readout_centre(request.faction, layout),
        headquarters_trip_days: marks.get(1).and_then(|&revolt| {
            rebellion_core::headquarters_move::plan(
                world,
                &rebellion_core::troop_transport::TroopTransportState::default(),
                true,
                revolt,
            )
            .ok()
            .map(|(_, days)| days)
        }),
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
    /// Each open System window's three bands' "Destination: ..." lines
    /// (`manufacturing_window::band`), Ship Construction first.
    system_destinations: Vec<(u32, Vec<String>)>,
    /// Each open System window's facility pages as the player sees them
    /// (`manufacturing_window::shown_page_cells`): the facilities listed on
    /// the shipyard, training, construction, refinery and mine pages, empty
    /// deposits left out.
    system_pages: Vec<(u32, [usize; 5])>,
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
            system_pages: windows
                .systems
                .open_windows()
                .filter_map(|(system, _)| {
                    use rebellion_render::manufacturing_window::{shown_page_cells, FacilityItem, FacilityPage};
                    let player = Faction::of_alliance(request.faction == CockpitFaction::Alliance);
                    let count = |page| {
                        shown_page_cells(
                            world,
                            windows.fog,
                            player,
                            windows.production.manufacturing,
                            windows.production.deliveries,
                            system,
                            page,
                        )
                        .iter()
                        .filter(|cell| !matches!(cell.item, FacilityItem::Deposit(_)))
                        .count()
                    };
                    Some((
                        world.systems.get(system)?.dat_id.raw(),
                        [
                            FacilityPage::Shipyards,
                            FacilityPage::TrainingFacilities,
                            FacilityPage::ConstructionYards,
                            FacilityPage::Refineries,
                            FacilityPage::Mines,
                        ]
                        .map(count),
                    ))
                })
                .collect(),
            system_destinations: windows
                .systems
                .open_windows()
                .filter_map(|(system, _)| {
                    let lines = rebellion_core::manufacturing::ProductionArea::ALL
                        .iter()
                        .map(|&area| {
                            rebellion_render::manufacturing_window::band(
                                world,
                                windows.production.manufacturing,
                                true,
                                system,
                                area,
                            )
                            .destination
                        })
                        .collect();
                    Some((world.systems.get(system)?.dat_id.raw(), lines))
                })
                .collect(),
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
    /// The page's second cell: the unfinished regiment in the
    /// `FleetLoadUnfinished` variant.
    pending_item: Option<(f32, f32)>,
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
        pending_item: defenses
            .cell_screen_rect(layout, primary, 1)
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
    /// Unfinished units (`rebellion_core::products`) aboard the fleet and
    /// on the primary planet.
    products_aboard: usize,
    products_on_planet: usize,
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
    /// The player's fleets holding capital ships at the loading target.
    target_fleets: usize,
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
    /// The player's last Evacuation Losses (`0x340`, `FUN_00495e90`): its
    /// title, body and background picture.
    last_loss_title: Option<String>,
    last_loss_text: Option<String>,
    last_loss_picture: Option<u32>,
    /// How many of those the player has.
    loss_count: usize,
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
    /// Commenor's holder (`OwnAssaultRepulsed`).
    commenor_holder: Option<String>,
    /// Whether Yavin is still the Alliance's headquarters
    /// (`HeadquartersTaken`).
    yavin_headquarters: Option<bool>,
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
            Scenario::EnemyAssaultTaken
                | Scenario::HeadquartersTaken
                | Scenario::GhormanBombardment
                | Scenario::GhormanAssault
                | Scenario::FleetWindowA0
                | Scenario::OwnAssaultRepulsed
        ) || self.last.as_ref() == Some(&assault_summary)
        {
            return;
        }
        self.last = Some(assault_summary.clone());
        let holder = |name: &str| {
            world
                .systems
                .values()
                .find(|value| value.name == name)
                .map(|value| format!("{:?}", rebellion_core::economy::system_holder(world, value)))
        };
        let yavin_holder = holder("Yavin");
        let commenor_holder = holder("Commenor");
        let report = EnemyAssaultObservation {
            status: "fleet-load",
            code: request.code,
            window_open: false,
            assault_summary,
            yavin_holder,
            commenor_holder,
            yavin_headquarters: world
                .systems
                .values()
                .find(|value| value.name == "Yavin")
                .map(|value| value.is_headquarters),
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
                FleetWindowEntry::Product { .. } => "product",
                FleetWindowEntry::Copy { .. } => "copy",
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
        products_aboard: world.products.in_fleet(fleet).count(),
        products_on_planet: world.products.in_system(primary).count(),
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
        // The port's log line, not the filed report (kind 7 files it too).
        last_assault_message: messages
            .iter()
            .rev()
            .find(|message| message.text.contains("assault") && message.assault_report.is_none())
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
        target_fleets: loading_target(world, primary).map_or(0, |target| {
            world.systems[target]
                .fleets
                .iter()
                .filter_map(|&key| world.fleets.get(key))
                .filter(|value| value.is_alliance == (request.faction == CockpitFaction::Alliance))
                .filter(|value| !value.capital_ships.is_empty())
                .count()
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
        last_loss_title: last_loss(request, messages).and_then(|message| message.title.clone()),
        last_loss_text: last_loss(request, messages).map(|message| message.text.clone()),
        loss_count: losses(request, messages).count(),
        last_loss_picture: last_loss(request, messages)
            .and_then(|message| message.picture)
            .map(|picture| picture.background),
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

/// The player's last line for a regiment lost running a blockade
/// (`FUN_00495e90`): TEXTSTRA `0x70a9`'s body, or the port's line without
/// the texts.
fn last_loss(request: FixtureRequest, messages: &[GameMessage]) -> Option<&GameMessage> {
    losses(request, messages).last()
}

/// The player's Evacuation Losses, oldest first.
fn losses(
    request: FixtureRequest,
    messages: &[GameMessage],
) -> impl DoubleEndedIterator<Item = &GameMessage> {
    messages
        .iter()
        .filter(move |message| {
            message
                .audience
                .includes(request.faction == CockpitFaction::Alliance)
        })
        .filter(|message| {
            message.text.contains("running an enemy blockade")
                || message.text.starts_with("Troops destroyed by blockade at ")
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
        FleetWindowEntry::Product { fleet, .. } => ("product", fleet_ordinal(world, fleet), None),
        FleetWindowEntry::Copy { fleet, .. } => ("copy", None, Some(fleet)),
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
        Scenario::MissionDialogMission | Scenario::JediTrainingDialog => MissionDialogPage::Mission,
        Scenario::MissionDialogAgents => MissionDialogPage::Agents,
        _ => return false,
    };
    let faction = match request.faction {
        CockpitFaction::Alliance => MissionFaction::Alliance,
        CockpitFaction::Empire => MissionFaction::Empire,
    };
    let (team, target) = if request.scenario == Scenario::JediTrainingDialog {
        // Luke and Dodonna, selected together, on Yavin (A0 x-54).
        let Some((luke, dodonna, yavin)) = jedi_training_cast(world) else {
            return false;
        };
        (
            vec![
                MissionMember::Character(luke),
                MissionMember::Character(dodonna),
            ],
            yavin,
        )
    } else {
        let (Some(character), Some(target)) =
            (world.characters.keys().next(), world.systems.keys().next())
        else {
            return false;
        };
        (vec![MissionMember::Character(character)], target)
    };
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
#[path = "interface_test_fixture_tests.rs"]
mod tests;
