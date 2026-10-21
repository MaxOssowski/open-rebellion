//! The picture each original message class shows above its text
//! (`ghidra/notes/message-pictures.md`).
//!
//! A message builder writes its background to `+0x2e` and, for a
//! character, a keyed portrait to `+0x2c` (`FUN_0046a320`). A background
//! is a STRATEGY bitmap or, where STRATEGY has none, the object's
//! Encyclopedia artwork (`FUN_0046a280` falls back to `FUN_0045f970`).
//!
//! `reader` is the side a message is built for (`+0x20`).

use rebellion_core::dat::Faction;
use rebellion_core::ids::{CharacterKey, DatId, SystemKey};
use rebellion_core::manufacturing::BuildableKind;
use rebellion_core::world::GameWorld;

use crate::cockpit::CockpitFaction;
use crate::message_log::MessagePicture;
use crate::story_messages::character_portrait;

/// A picture with no portrait.
const fn background(background: u32) -> MessagePicture {
    MessagePicture {
        background,
        foreground: None,
    }
}

/// `first` for the Alliance and the next bitmap for the Empire, the
/// builders' `(side != 1) + first`.
const fn by_reader(reader: CockpitFaction, first: u32) -> MessagePicture {
    match reader {
        CockpitFaction::Alliance => background(first),
        CockpitFaction::Empire => background(first + 1),
    }
}

/// The reader's own background (`0x412`, `0x413`) with a character's
/// portrait on it: mission reports and failures (`FUN_004927c0`,
/// `FUN_00491500`), personnel arriving (`FUN_004955b0`) and the story
/// classes about one character.
#[must_use]
pub fn side(reader: CockpitFaction, character: Option<(DatId, bool)>) -> MessagePicture {
    MessagePicture {
        foreground: character.and_then(|(dat_id, major)| character_portrait(dat_id, major)),
        ..by_reader(reader, 0x412)
    }
}

/// An object's own picture (`FUN_004c5000`): its class picture, the list
/// mini less `0x4000`, plus `0x1000`. New units deployed (`FUN_00497940`),
/// an idle yard (`FUN_0048be60`), a repaired ship or refilled squadron
/// (`FUN_00499de0`) and a mission led by special forces show it.
#[must_use]
pub fn object(mini: u32) -> Option<MessagePicture> {
    mini.checked_sub(0x3000).map(background)
}

/// The EDATA file number of an object picture, from ENCYBMAP's string
/// table: `FUN_0045f970` loads `EDATA.NNN` by the picture id when STRATEGY
/// has no such bitmap. Facilities, regiments, special forces, fighters,
/// capital ships and characters, each a run of consecutive files.
#[must_use]
pub fn artwork_edata(picture: u32) -> Option<u16> {
    const RUNS: [(u32, u32, u16); 16] = [
        (0x1001, 2, 1),
        (0x1100, 6, 3),
        (0x1200, 5, 9),
        (0x1280, 1, 14),
        (0x1440, 5, 15),
        (0x1480, 5, 20),
        (0x1540, 4, 25),
        (0x1580, 5, 29),
        (0x1640, 4, 34),
        (0x1680, 4, 38),
        (0x1740, 15, 42),
        (0x1780, 15, 57),
        (0x1840, 4, 72),
        (0x1880, 2, 76),
        (0x1a40, 26, 78),
        (0x1a80, 28, 104),
    ];
    RUNS.iter()
        .find(|&&(first, count, _)| (first..first + count).contains(&picture))
        .and_then(|&(first, _, file)| u16::try_from(picture - first).ok().map(|at| file + at))
}

/// Uprising Begins (`FUN_00499460`, notification `0x14b`).
#[must_use]
pub const fn uprising_begins() -> MessagePicture {
    background(0x3f2)
}

/// Uprising Ends (`FUN_00499460`), by the system's side; a neutral
/// system's is not shown.
#[must_use]
pub const fn uprising_ends(system: Faction) -> Option<MessagePicture> {
    match system {
        Faction::Alliance => Some(background(0x3f3)),
        Faction::Empire => Some(background(0x3f4)),
        Faction::Neutral => None,
    }
}

/// Near Uprising (`FUN_00498cb0`, notification `0x148`), by the system's
/// side.
#[must_use]
pub const fn near_uprising(system: Faction) -> Option<MessagePicture> {
    match system {
        Faction::Alliance => Some(background(0x3f0)),
        Faction::Empire => Some(background(0x3f1)),
        Faction::Neutral => None,
    }
}

/// A system joins a side or declares neutrality (`FUN_00499760`,
/// notification `0x100`).
#[must_use]
pub const fn loyalty_change() -> MessagePicture {
    background(0x3ed)
}

/// A blockade begins (`FUN_004960f0`, notification `0x14e`), by the
/// blockaded system's side.
#[must_use]
pub const fn blockade(system: Faction) -> Option<MessagePicture> {
    match system {
        Faction::Alliance => Some(background(0x404)),
        Faction::Empire => Some(background(0x403)),
        Faction::Neutral => None,
    }
}

/// Evacuation Losses, units lost running a blockade (`FUN_00495e90`,
/// notification `0x340`).
#[must_use]
pub const fn evacuation_losses(reader: CockpitFaction) -> MessagePicture {
    match reader {
        CockpitFaction::Alliance => background(0x407),
        CockpitFaction::Empire => background(0x406),
    }
}

/// Assault on a system (`FUN_00496770`), by the attacker's side.
///
/// port: not attached yet. This picture and the bombardment's are 400 by
/// 310 and do not fit the Message window's 400 by 200 picture area; how
/// the original shows them is not traced.
#[must_use]
pub const fn assault(attacker: Faction) -> MessagePicture {
    match attacker {
        Faction::Alliance => background(0x2b98),
        Faction::Empire | Faction::Neutral => background(0x2b99),
    }
}

/// Orbital bombardment (`FUN_00496770`): the scorched ground when the
/// strike destroyed something at the system (message `+0x80`), the
/// planet's guns answering when only units elsewhere were lost
/// (`+0x88`), and the untouched ground otherwise. The two lists are
/// filled by `FUN_00497380` on notification `0x302`.
#[must_use]
pub const fn bombardment(lost_at_system: bool, lost_elsewhere: bool) -> MessagePicture {
    if lost_at_system {
        background(0x2b9b)
    } else if lost_elsewhere {
        background(0x2b9c)
    } else {
        background(0x2b9a)
    }
}

/// What a Unit Arrival message reports (`FUN_004981c0`, notification
/// `0x105`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// Ships or a fleet (`0x7076`, `0x7078`).
    Fleet,
    /// Other units (`0x707a`).
    Units,
    /// The Alliance Headquarters (`0x707c`).
    Headquarters,
}

/// Unit Arrival (`FUN_004981c0`). The Headquarters arriving is shown to
/// the Alliance only.
#[must_use]
pub const fn arrival(what: Arrival, reader: CockpitFaction) -> Option<MessagePicture> {
    match (what, reader) {
        (Arrival::Fleet, _) => Some(by_reader(reader, 0x3fa)),
        (Arrival::Units, _) => Some(by_reader(reader, 0x3fc)),
        (Arrival::Headquarters, CockpitFaction::Alliance) => Some(background(0x3fe)),
        (Arrival::Headquarters, CockpitFaction::Empire) => None,
    }
}

/// Research Complete (`FUN_00499aa0`), Research Exhausted
/// (`FUN_0048abc0`) and a Research mission's report (`FUN_004927c0`).
#[must_use]
pub const fn research() -> MessagePicture {
    background(0x3f9)
}

/// Units listed as destroyed: Maintenance Shortfall (`FUN_00498970`) and
/// Saboteurs Strike (`FUN_0048b9a0`) share slot `+0x40`, `FUN_0048bbf0`.
#[must_use]
pub const fn units_destroyed(reader: CockpitFaction) -> MessagePicture {
    by_reader(reader, 0x3f5)
}

/// Facility Lost, a new facility with nowhere to deploy (`FUN_00497690`,
/// notification `0x303`).
#[must_use]
pub const fn facility_lost(reader: CockpitFaction) -> MessagePicture {
    by_reader(reader, 0x3f7)
}

/// Natural Disaster (`0x154`) or a change of resources (`0x155`),
/// `FUN_0048e940`.
#[must_use]
pub const fn system_incident(disaster: bool) -> MessagePicture {
    if disaster {
        background(0x3eb)
    } else {
        background(0x3ea)
    }
}

/// Informants Provide Information (`FUN_00491060`, notification `0x153`).
#[must_use]
pub const fn informants() -> MessagePicture {
    background(0x3e8)
}

/// Character Injured (`FUN_0048fdf0`, notification `0x106`).
#[must_use]
pub const fn injured() -> MessagePicture {
    background(0x41a)
}

/// Character Healed (`FUN_0048fdf0`): the empty chair with the
/// character's portrait.
#[must_use]
pub fn healed(character: (DatId, bool)) -> MessagePicture {
    MessagePicture {
        background: 0x41b,
        foreground: character_portrait(character.0, character.1),
    }
}

/// Character Killed (`FUN_0048fdf0`), by the character's side
/// (`FUN_004edc80`): `0x42e`, or `0x433` when it is not the Alliance's.
#[must_use]
pub const fn killed(character: Faction) -> MessagePicture {
    match character {
        Faction::Alliance => background(0x42e),
        Faction::Empire | Faction::Neutral => background(0x433),
    }
}

/// A character captured (`FUN_00490340`, notification `0x1e0`), by the
/// captive's side (`FUN_004edc80`).
#[must_use]
pub const fn captured(captive: Faction) -> MessagePicture {
    match captive {
        Faction::Alliance => background(0x418),
        Faction::Empire | Faction::Neutral => background(0x419),
    }
}

/// A character escaped (`FUN_00490340`).
#[must_use]
pub const fn escaped() -> MessagePicture {
    background(0x405)
}

/// An enemy mission foiled (`FUN_00491500`, notification `0x1f1`).
#[must_use]
pub const fn enemy_mission_foiled(reader: CockpitFaction) -> MessagePicture {
    by_reader(reader, 0x431)
}

/// The mission types whose reports open on their own picture
/// (`FUN_004927c0`, notification `0x1f0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportedMission {
    /// Family `0x51`.
    Diplomacy,
    /// Family `0x52`.
    Espionage,
    /// Family `0x53`.
    Research,
    /// Family `0x56`.
    InciteUprising,
    /// Family `0x57`.
    SubdueUprising,
    /// Every other family.
    Other,
}

/// A Mission Report (`FUN_004927c0`): the mission type's picture, or the
/// reader's own background, with the leading character's portrait.
///
/// Subdue Uprising opens on the riot and ends on the reader's troops when
/// it succeeds (case `0x57`, outcome 3); Incite Uprising shows the crowd
/// when it succeeds (case `0x56`, outcome 3).
#[must_use]
pub fn mission_report(
    mission: ReportedMission,
    succeeded: bool,
    reader: CockpitFaction,
    leader: Option<(DatId, bool)>,
) -> MessagePicture {
    let scene = match (mission, succeeded) {
        (ReportedMission::Diplomacy, _) => background(0x414),
        (ReportedMission::Espionage, _) => background(0x415),
        (ReportedMission::Research, _) => research(),
        (ReportedMission::SubdueUprising, true) => by_reader(reader, 0x3f3),
        (ReportedMission::SubdueUprising, false) => uprising_begins(),
        (ReportedMission::InciteUprising, true) => loyalty_change(),
        (ReportedMission::InciteUprising, false) | (ReportedMission::Other, _) => {
            by_reader(reader, 0x412)
        }
    };
    MessagePicture {
        foreground: leader.and_then(|(dat_id, major)| character_portrait(dat_id, major)),
        ..scene
    }
}

/// The side a message for the Alliance, or for the Empire, is built for.
#[must_use]
pub const fn reader(is_alliance: bool) -> CockpitFaction {
    if is_alliance {
        CockpitFaction::Alliance
    } else {
        CockpitFaction::Empire
    }
}

/// The side holding `system` (system `+0x24` bits 6..7), neutral when no
/// one does.
#[must_use]
pub fn system_side(world: &GameWorld, system: SystemKey) -> Faction {
    world
        .systems
        .get(system)
        .and_then(|value| value.control.faction())
        .unwrap_or(Faction::Neutral)
}

/// A character's side.
#[must_use]
pub fn character_side(world: &GameWorld, character: CharacterKey) -> Faction {
    match world.characters.get(character) {
        Some(value) if value.is_alliance => Faction::Alliance,
        Some(value) if value.is_empire => Faction::Empire,
        _ => Faction::Neutral,
    }
}

/// What a character's portrait is looked up by.
#[must_use]
pub fn portrait_key(world: &GameWorld, character: CharacterKey) -> Option<(DatId, bool)> {
    world
        .characters
        .get(character)
        .map(|value| (value.dat_id, value.is_major))
}

/// A new unit deployed (`FUN_00497940`): the unit's own picture.
#[must_use]
pub fn deployed(world: &GameWorld, kind: BuildableKind) -> Option<MessagePicture> {
    crate::manufacturing_window::product(world, kind)
        .1
        .and_then(object)
}

/// An idle production area (`FUN_0048be60`): the picture of the first
/// facility of the area's family its walk finds at the system.
#[must_use]
pub fn idle_yard(
    world: &GameWorld,
    system: SystemKey,
    area: rebellion_core::manufacturing::ProductionArea,
) -> Option<MessagePicture> {
    let family = rebellion_core::build_selection::area_family(area);
    let class = world
        .systems
        .get(system)?
        .manufacturing_facilities
        .iter()
        .filter_map(|&yard| world.manufacturing_facilities.get(yard))
        .map(|yard| yard.class_dat_id)
        .find(|class| class.family() == family)?;
    crate::manufacturing_window::class_mini(class).and_then(|(mini, _)| object(mini))
}

#[cfg(test)]
mod tests {
    use super::*;
    use CockpitFaction::{Alliance, Empire};

    fn ids(picture: MessagePicture) -> (u32, Option<u32>) {
        (picture.background, picture.foreground)
    }

    #[test]
    fn an_uprising_shows_the_riot_then_the_holders_troops() {
        // FUN_00499460: 0x3f2 on the begin branch; 0x3f3 for an Alliance
        // system and 0x3f4 for an Empire one; other sides are not shown.
        assert_eq!(ids(uprising_begins()), (1010, None));
        assert_eq!(
            uprising_ends(Faction::Alliance).map(ids),
            Some((1011, None))
        );
        assert_eq!(uprising_ends(Faction::Empire).map(ids), Some((1012, None)));
        assert_eq!(uprising_ends(Faction::Neutral), None);
        // FUN_00498cb0: 0x3f0 and 0x3f1.
        assert_eq!(
            near_uprising(Faction::Alliance).map(ids),
            Some((1008, None))
        );
        assert_eq!(near_uprising(Faction::Empire).map(ids), Some((1009, None)));
        assert_eq!(near_uprising(Faction::Neutral), None);
        // FUN_00499760: 0x3ed whatever the change.
        assert_eq!(ids(loyalty_change()), (1005, None));
    }

    #[test]
    fn a_blockade_shows_the_blockaders_ships_and_its_losses_the_readers() {
        // FUN_004960f0: system side 1 gives 0x404, side 2 gives 0x403.
        assert_eq!(blockade(Faction::Alliance).map(ids), Some((1028, None)));
        assert_eq!(blockade(Faction::Empire).map(ids), Some((1027, None)));
        assert_eq!(blockade(Faction::Neutral), None);
        // FUN_00495e90: reader 1 gives 0x407, otherwise 0x406.
        assert_eq!(ids(evacuation_losses(Alliance)), (1031, None));
        assert_eq!(ids(evacuation_losses(Empire)), (1030, None));
    }

    #[test]
    fn an_assault_shows_the_attacker_and_a_bombardment_what_it_cost() {
        // FUN_00496770: attacker side 1 gives 0x2b98, otherwise 0x2b99.
        assert_eq!(ids(assault(Faction::Alliance)), (11160, None));
        assert_eq!(ids(assault(Faction::Empire)), (11161, None));
        // The +0x80 list first, then +0x88, then neither.
        assert_eq!(ids(bombardment(true, false)), (11163, None));
        assert_eq!(ids(bombardment(true, true)), (11163, None));
        assert_eq!(ids(bombardment(false, true)), (11164, None));
        assert_eq!(ids(bombardment(false, false)), (11162, None));
    }

    #[test]
    fn an_arrival_shows_the_readers_fleet_or_hangar() {
        // FUN_004981c0: cases 1 and 2 give 0x3fa or 0x3fb, case 3 gives
        // 0x3fc or 0x3fd, case 4 gives 0x3fe to side 1 and hides the
        // message from the other.
        assert_eq!(
            arrival(Arrival::Fleet, Alliance).map(ids),
            Some((1018, None))
        );
        assert_eq!(arrival(Arrival::Fleet, Empire).map(ids), Some((1019, None)));
        assert_eq!(
            arrival(Arrival::Units, Alliance).map(ids),
            Some((1020, None))
        );
        assert_eq!(arrival(Arrival::Units, Empire).map(ids), Some((1021, None)));
        assert_eq!(
            arrival(Arrival::Headquarters, Alliance).map(ids),
            Some((1022, None))
        );
        assert_eq!(arrival(Arrival::Headquarters, Empire), None);
    }

    #[test]
    fn losses_and_incidents_carry_their_class_pictures() {
        // FUN_0048bbf0: 0x3f5 for side 1, else 0x3f6.
        assert_eq!(ids(units_destroyed(Alliance)), (1013, None));
        assert_eq!(ids(units_destroyed(Empire)), (1014, None));
        // FUN_00497690: (side != 1) + 0x3f7.
        assert_eq!(ids(facility_lost(Alliance)), (1015, None));
        assert_eq!(ids(facility_lost(Empire)), (1016, None));
        // FUN_0048e940: 0x3eb for a disaster, 0x3ea for resources.
        assert_eq!(ids(system_incident(true)), (1003, None));
        assert_eq!(ids(system_incident(false)), (1002, None));
        // FUN_00499aa0: 0x3f9.
        assert_eq!(ids(research()), (1017, None));
    }

    #[test]
    fn a_characters_fate_shows_its_scene() {
        // FUN_0048fdf0: 0x41a injured; 0x41b with the portrait healed;
        // killed is (side != 1 ? 5 : 0) + 0x42e.
        let luke = (DatId::new(578), true);
        assert_eq!(ids(injured()), (1050, None));
        assert_eq!(ids(healed(luke)), (1051, Some(6210)));
        assert_eq!(ids(killed(Faction::Alliance)), (1070, None));
        assert_eq!(ids(killed(Faction::Empire)), (1075, None));
        // FUN_00490340: (side != 1) + 0x418 captured, 0x405 escaped.
        assert_eq!(ids(captured(Faction::Alliance)), (1048, None));
        assert_eq!(ids(captured(Faction::Empire)), (1049, None));
        assert_eq!(ids(escaped()), (1029, None));
    }

    #[test]
    fn a_mission_report_opens_on_its_types_picture_with_the_leader() {
        // FUN_004927c0: 0x414 for family 0x51, 0x415 for 0x52, 0x3f9 for
        // 0x53, the side's background otherwise; the leading character's
        // portrait goes to +0x2c.
        let leia = Some((DatId::new(577), true));
        let report =
            |mission, succeeded, reader| ids(mission_report(mission, succeeded, reader, leia));
        assert_eq!(
            report(ReportedMission::Diplomacy, true, Alliance),
            (1044, Some(6209))
        );
        assert_eq!(
            report(ReportedMission::Espionage, false, Empire),
            (1045, Some(6209))
        );
        assert_eq!(
            report(ReportedMission::Research, true, Empire),
            (1017, Some(6209))
        );
        assert_eq!(
            report(ReportedMission::Other, true, Alliance),
            (1042, Some(6209))
        );
        assert_eq!(
            report(ReportedMission::Other, false, Empire),
            (1043, Some(6209))
        );
        // Case 0x57: (side != 1) + 0x3f3 on outcome 3, else 0x3f2.
        assert_eq!(
            report(ReportedMission::SubdueUprising, true, Alliance).0,
            1011
        );
        assert_eq!(
            report(ReportedMission::SubdueUprising, true, Empire).0,
            1012
        );
        assert_eq!(
            report(ReportedMission::SubdueUprising, false, Empire).0,
            1010
        );
        // Case 0x56: 0x3ed on outcome 3.
        assert_eq!(
            report(ReportedMission::InciteUprising, true, Empire).0,
            1005
        );
        assert_eq!(
            report(ReportedMission::InciteUprising, false, Empire).0,
            1043
        );
        // No character leads: no portrait.
        assert_eq!(
            ids(mission_report(
                ReportedMission::Diplomacy,
                true,
                Alliance,
                None
            )),
            (1044, None)
        );
        // FUN_00491500: (side != 1) + 0x431 for a foiled enemy mission.
        assert_eq!(ids(enemy_mission_foiled(Alliance)), (1073, None));
        assert_eq!(ids(enemy_mission_foiled(Empire)), (1074, None));
    }

    #[test]
    fn the_side_background_carries_a_characters_portrait() {
        // (side != 1) + 0x412, and FUN_004c5000 for the portrait.
        let han = Some((DatId::new(579), true));
        assert_eq!(ids(side(Alliance, han)), (1042, Some(6211)));
        assert_eq!(ids(side(Empire, None)), (1043, None));
    }

    #[test]
    fn an_objects_picture_is_its_mini_less_0x3000_and_an_edata_file() {
        // FUN_004c5000: (class picture & 0xfff) + 0x1000. The Orbital
        // Shipyard's mini is GOKRES 16640, so its picture is 0x1100, which
        // ENCYBMAP names EDATA.003; the Alliance Fleet Regiment (17472)
        // is 0x1440, EDATA.015.
        assert_eq!(object(16_640).map(ids), Some((0x1100, None)));
        assert_eq!(object(17_472).map(ids), Some((0x1440, None)));
        assert_eq!(object(12), None);
        assert_eq!(artwork_edata(0x1100), Some(3));
        assert_eq!(artwork_edata(0x1105), Some(8));
        assert_eq!(artwork_edata(0x1106), None);
        assert_eq!(artwork_edata(0x1440), Some(15));
        assert_eq!(artwork_edata(0x174e), Some(56));
        assert_eq!(artwork_edata(0x1780), Some(57));
        assert_eq!(artwork_edata(0x1842), Some(74));
        assert_eq!(artwork_edata(0x1a9b), Some(131));
        assert_eq!(artwork_edata(0x1000), None);
        assert_eq!(artwork_edata(0x3f2), None);
    }

    fn system(world: &mut GameWorld, control: rebellion_core::world::ControlKind) -> SystemKey {
        use rebellion_core::dat::ExplorationStatus;
        world.systems.insert(rebellion_core::world::System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Hoth".into(),
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

    #[test]
    fn a_messages_sides_come_from_the_world() {
        use rebellion_core::world::{Character, ControlKind};
        let mut world = GameWorld::default();
        let held = system(&mut world, ControlKind::Controlled(Faction::Empire));
        let rising = system(&mut world, ControlKind::Uprising(Faction::Alliance));
        let free = system(&mut world, ControlKind::Uncontrolled);
        assert_eq!(system_side(&world, held), Faction::Empire);
        assert_eq!(system_side(&world, rising), Faction::Alliance);
        assert_eq!(system_side(&world, free), Faction::Neutral);
        assert_eq!(system_side(&world, SystemKey::default()), Faction::Neutral);

        let mut person = |is_alliance, is_empire| {
            world.characters.insert(Character {
                dat_id: DatId::new(578),
                is_major: true,
                is_alliance,
                is_empire,
                ..Character::default()
            })
        };
        let (rebel, imperial, neither) = (
            person(true, false),
            person(false, true),
            person(false, false),
        );
        assert_eq!(character_side(&world, rebel), Faction::Alliance);
        assert_eq!(character_side(&world, imperial), Faction::Empire);
        assert_eq!(character_side(&world, neither), Faction::Neutral);
        assert_eq!(
            character_side(&world, CharacterKey::default()),
            Faction::Neutral
        );
        assert_eq!(portrait_key(&world, rebel), Some((DatId::new(578), true)));
        assert_eq!(portrait_key(&world, CharacterKey::default()), None);

        assert_eq!(reader(true), Alliance);
        assert_eq!(reader(false), Empire);
    }

    #[test]
    fn a_deployed_unit_and_an_idle_yard_show_their_own_pictures() {
        // FUN_00497940 and FUN_0048be60: FUN_004c5000 of the unit or the
        // facility. An Advanced Shipyard (0x28000004) has mini 16643, so
        // picture 0x1103; a Sullustan Regiment (0x10000003) has 17474, so
        // 0x1442.
        use rebellion_core::world::{ControlKind, ManufacturingFacilityInstance};
        let mut world = GameWorld::default();
        let here = system(&mut world, ControlKind::Controlled(Faction::Alliance));
        use rebellion_core::manufacturing::ProductionArea;
        assert_eq!(idle_yard(&world, here, ProductionArea::Shipyard), None);
        let yard = world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(0x2800_0004),
                side: Faction::Alliance,
                is_shipyard: true,
            });
        world.systems[here].manufacturing_facilities.push(yard);
        assert_eq!(
            idle_yard(&world, here, ProductionArea::Shipyard).map(ids),
            Some((0x1103, None))
        );
        // The walk looks for the area's own family (FUN_00520580).
        assert_eq!(
            idle_yard(&world, here, ProductionArea::TrainingFacility),
            None
        );
        assert_eq!(artwork_edata(0x1103), Some(6));

        let regiment = BuildableKind::Troop(DatId::new(0x1000_0003));
        assert_eq!(deployed(&world, regiment).map(ids), Some((0x1442, None)));
        let unknown = BuildableKind::Troop(DatId::new(0x1000_0063));
        assert_eq!(deployed(&world, unknown), None);
    }
}
