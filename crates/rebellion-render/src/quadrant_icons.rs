//! The four icons a sector window draws around each planet (`FUN_00459e30`),
//! which rule shows each one, and its art. Recovery notes:
//! `ghidra/notes/sector-quadrants.md`.
//!
//! Each rule ends in `FUN_0045d140(item, state, enabled)`. The state is a
//! side, which picks the art (`FUN_0045ca80`), and a zero `enabled` removes
//! the item, so an icon with nothing to show is not drawn.

use std::collections::{HashMap, HashSet};

use rebellion_core::dat::Faction;
use rebellion_core::fog::FogState;
use rebellion_core::ids::SystemKey;
use rebellion_core::missions::{MissionMember, MissionState};
use rebellion_core::movement::MovementState;
use rebellion_core::squadrons::{SquadronPlace, SquadronTransit};
use rebellion_core::troop_transport::{RegimentLeg, RegimentTransit};
use rebellion_core::world::GameWorld;

use crate::fleet_window::{control_side, faction_side, fleet_side, icon_side};
use crate::system_window::opposing_contents_visible;

/// The regiments and squadrons travelling by Move
/// (`TroopTransportState::transits`, `SquadronTransits::transits`).
/// `FUN_00556390` puts each in its destination at once, so a walk of the
/// destination's regiments or squadrons finds it there.
#[derive(Debug, Clone, Copy, Default)]
pub struct Travellers<'a> {
    pub regiments: &'a [RegimentTransit],
    pub squadrons: &'a [SquadronTransit],
}

/// One of the four overlay items, by its kind (`flag >> 16`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quadrant {
    /// Kind 4, top-left: the System window (type 9).
    System,
    /// Kind 8, bottom-left: the System Defenses window (type 10).
    Defenses,
    /// Kind `0x10`, top-right: the Fleet window (type 4).
    Fleets,
    /// Kind `0x40`, bottom-right: the Missions window (type 11).
    Missions,
}

impl Quadrant {
    /// The order `FUN_00459e30` creates the items in.
    pub const ALL: [Self; 4] = [Self::System, Self::Defenses, Self::Fleets, Self::Missions];

    /// The item's kind (`flag >> 16`), which `FUN_00512700` resolves an
    /// order's team by.
    #[must_use]
    pub const fn kind(self) -> u8 {
        match self {
            Self::System => 4,
            Self::Defenses => 8,
            Self::Fleets => 0x10,
            Self::Missions => 0x40,
        }
    }
}

/// `FUN_0045ca80(kind, state, 0/1)`: the item's two bitmaps for a side.
/// Kinds 4 and 8 have art for sides 0 and 3 as well; the fleet and mission
/// kinds have none.
#[must_use]
pub const fn quadrant_art(quadrant: Quadrant, side: u8) -> Option<(u32, u32)> {
    match (quadrant, side) {
        (Quadrant::System, 1) => Some((10771, 10772)),
        (Quadrant::System, 2) => Some((10779, 10780)),
        (Quadrant::System, 0 | 3) => Some((10787, 10788)),
        (Quadrant::Defenses, 1) => Some((10773, 10774)),
        (Quadrant::Defenses, 2) => Some((10781, 10782)),
        (Quadrant::Defenses, 0 | 3) => Some((10789, 10790)),
        (Quadrant::Fleets, 1) => Some((10775, 10776)),
        (Quadrant::Fleets, 2) => Some((10783, 10784)),
        (Quadrant::Missions, 1) => Some((10777, 10778)),
        (Quadrant::Missions, 2) => Some((10785, 10786)),
        _ => None,
    }
}

/// The side a quadrant item shows, or `None` when it is hidden.
#[expect(
    clippy::too_many_arguments,
    reason = "Each rule reads its own state: the missions, fleets and travellers."
)]
#[must_use]
pub fn quadrant_side(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    movement: &MovementState,
    travellers: Travellers<'_>,
    player: Faction,
    system: SystemKey,
    quadrant: Quadrant,
) -> Option<u8> {
    let contents = SystemContents::new(world, fog, player, system)?;
    match quadrant {
        // FUN_0045cdc0: the system's side bits while it has manufacturing or
        // production facilities (families 0x28..0x30, FUN_0053b6e0).
        Quadrant::System => (contents.manufacturing_and_production() > 0).then(|| contents.side()),
        // FUN_0045ce80: the system's side bits while it has defense
        // facilities, regiments, squadrons, or personnel off a visible
        // mission, those on their way there included.
        Quadrant::Defenses => {
            (contents.defenses(missions, travellers) > 0).then(|| contents.side())
        }
        // FUN_0045ccc0, ported in `fleet_window`.
        Quadrant::Fleets => match icon_side(world, movement, fog, player, system) {
            (_, 0) => None,
            (side, _) => Some(side),
        },
        // FUN_0045d090: FUN_004a1f60's side; disabled for 0 and 3.
        Quadrant::Missions => match mission_side(&contents, missions) {
            0 | 3 => None,
            side => Some(side),
        },
    }
}

/// The characters and special forces at `system` the player sees, each with
/// its side and whether it is on a hidden mission; none at an unexplored
/// system.
pub(crate) fn visible_members(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> Vec<(MissionMember, u8, bool)> {
    SystemContents::new(world, fog, player, system)
        .map(|contents| contents.members().collect())
        .unwrap_or_default()
}

/// The player's visible missions at `system`, which a mission icon's Abort
/// (`0x250`) acts on: `FUN_00512700` kind `0x40` gives the members on a
/// visible mission (`+0x78` bit 8 clear), `FUN_00553350` keeps the order's
/// side, and event `0x261` resolves each to its mission (`FUN_00505a10`).
/// Distinct, in mission order.
#[must_use]
pub fn own_visible_missions(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    player: Faction,
    system: SystemKey,
) -> Vec<u64> {
    let Some(contents) = SystemContents::new(world, fog, player, system) else {
        return Vec::new();
    };
    let keys = mission_keys(missions);
    let own = contents.player_side;
    let mut ids: Vec<u64> = contents
        .members_with_arrivals(missions)
        .filter(|&(_, side, hidden)| side == own && !hidden)
        .filter_map(|(member, _, _)| keys.get(&member).copied())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Whether a facility icon's Destination (`0x214`) and Reserved (`0x216`)
/// have a team at `system` for `side`: `FUN_00512700` kind 4 gives the
/// system's managers (`FUN_0052c170`, families `0xa0..0xaf`) of the order's
/// side (`FUN_00553350`), and `FUN_00553770` refuses an empty team (`0x16`).
/// port: the managers are not modelled; a system has the side's when it has
/// a manufacturing facility of the side.
#[must_use]
pub fn facility_icon_has_managers(world: &GameWorld, system: SystemKey, side: Faction) -> bool {
    world.systems.get(system).is_some_and(|value| {
        value.manufacturing_facilities.iter().any(|key| {
            world
                .manufacturing_facilities
                .get(*key)
                .is_some_and(|facility| facility.side == side)
        })
    })
}

/// The team of a defenses icon's Move and Confirmed Move (`0x201`, `0x202`):
/// `FUN_00512700` kind 8 gives the system's regiments (`FUN_00504c40`,
/// families `0x10..0x13`), fighter squadrons (`FUN_005039d0`, `0x1c..0x1f`),
/// then characters and special forces (`FUN_00536da0`, `0x30..0x3f`), each
/// list filtered to the order's side (`FUN_00553350`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefensesTeam {
    /// The side's regiments on the planet.
    pub troops: Vec<rebellion_core::ids::TroopKey>,
    /// The side's squadrons on the planet, one class entry per squadron.
    pub squadrons: Vec<rebellion_core::ids::FighterKey>,
    /// The side's characters and special forces in the system, those on
    /// their way there included.
    pub members: Vec<MissionMember>,
}

impl DefensesTeam {
    /// `FUN_0053c4b0` refuses a move with no member (`1`/`0x16`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.troops.is_empty() && self.squadrons.is_empty() && self.members.is_empty()
    }
}

/// [`DefensesTeam`] at `system` for `side`. The walk is the system's own
/// (`FUN_00513050` over its children), so it takes no fog, and a member on
/// its way in counts, as `FUN_0045ce80`'s does: a transit puts the member
/// in its target's container at once (`mission-lifecycle.md`).
#[must_use]
pub fn defenses_move_team(
    world: &GameWorld,
    missions: &MissionState,
    side: Faction,
    system: SystemKey,
) -> DefensesTeam {
    let Some(value) = world.systems.get(system) else {
        return DefensesTeam::default();
    };
    let own = faction_side(side);
    let troops = value
        .ground_units
        .iter()
        .copied()
        .filter(|key| {
            world
                .troops
                .get(*key)
                .is_some_and(|troop| fleet_side(troop.is_alliance) == own)
        })
        .collect();
    let squadrons = value
        .fighters
        .iter()
        .filter(|entry| {
            world
                .fighter_classes
                .get(entry.class)
                .is_some_and(|class| fleet_side(class.is_alliance) == own)
        })
        .flat_map(|entry| std::iter::repeat_n(entry.class, entry.count as usize))
        .collect();
    let arrivals = missions
        .en_route()
        .iter()
        .filter(|transit| transit.abode.is_none() && bound_fleet(missions, transit).is_none());
    let mut members: Vec<MissionMember> = Vec::new();
    for (member, member_side, _) in
        system_members(world, system).chain(transit_members(world, arrivals, system))
    {
        if member_side == own && !members.contains(&member) {
            members.push(member);
        }
    }
    DefensesTeam {
        troops,
        squadrons,
        members,
    }
}

/// `FUN_004a1f60(system, player side)` over what the player sees: the other
/// side's missions win, then the player's, else 3.
pub(crate) fn missions_side(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    player: Faction,
    system: SystemKey,
) -> u8 {
    SystemContents::new(world, fog, player, system)
        .map_or(3, |contents| mission_side(&contents, missions))
}

/// The quadrant item's two bitmaps, or `None` when it is not drawn.
#[expect(
    clippy::too_many_arguments,
    reason = "Each rule reads its own state: the missions, fleets and travellers."
)]
#[must_use]
pub fn quadrant_icon(
    world: &GameWorld,
    fog: &FogState,
    missions: &MissionState,
    movement: &MovementState,
    travellers: Travellers<'_>,
    player: Faction,
    system: SystemKey,
    quadrant: Quadrant,
) -> Option<(u32, u32)> {
    quadrant_side(
        world, fog, missions, movement, travellers, player, system, quadrant,
    )
    .and_then(|side| quadrant_art(quadrant, side))
}

/// What the player sees at a system: every side's objects where the System
/// window shows opposing contents, otherwise only the player's own.
///
/// port: the original reads the galaxy view's system (hyp: the player
/// side's view, `FUN_00539fd0`); the port uses the System window's
/// visibility so an icon never reveals what that window hides.
struct SystemContents<'a> {
    world: &'a GameWorld,
    system: SystemKey,
    player: Faction,
    player_side: u8,
    opposing_visible: bool,
    /// The player's frozen copy of the system, whose other-side objects
    /// the icons count instead of the live ones (`FUN_004f2d10` reads the
    /// side's copy; `FUN_004f9a60`).
    frozen: Option<&'a rebellion_core::object_views::SystemView>,
    /// The player's frozen copy of the system's missions.
    frozen_extras: Option<&'a rebellion_core::object_views::SystemExtras>,
}

impl<'a> SystemContents<'a> {
    fn new(
        world: &'a GameWorld,
        fog: &FogState,
        player: Faction,
        system: SystemKey,
    ) -> Option<Self> {
        // The player's copy's explored bit (ghidra/notes/fog-of-war.md).
        if !rebellion_core::object_views::side_explored(world, player, system) {
            return None;
        }
        let frozen = match crate::viewed_contents::system_sight(world, fog, player, system) {
            crate::viewed_contents::SystemSight::Frozen(view) => Some(view),
            _ => None,
        };
        Some(Self {
            world,
            system,
            player,
            player_side: faction_side(player),
            opposing_visible: opposing_contents_visible(world, fog, player, system),
            frozen,
            frozen_extras: crate::viewed_contents::frozen_extras(world, fog, player, system),
        })
    }

    /// The system's side bits `(+0x24 >> 6) & 3`, as the player's frozen
    /// copy holds them when it has one.
    fn side(&self) -> u8 {
        if let Some(view) = self.frozen {
            return control_side(view.control);
        }
        self.world
            .systems
            .get(self.system)
            .map_or(0, |value| control_side(value.control))
    }

    /// The other side's yards, mines and refineries in the frozen copy.
    fn frozen_facilities(&self) -> usize {
        let Some(view) = self.frozen else {
            return 0;
        };
        let other = |facility: &&rebellion_core::object_views::FacilityView| {
            crate::fleet_window::faction_side(facility.side) != self.player_side
        };
        view.manufacturing_facilities
            .iter()
            .filter(other)
            .filter(|facility| (0x28..0x30).contains(&facility.class.family()))
            .count()
            + view.production_facilities.iter().filter(other).count()
    }

    /// The other side's defenses in the frozen copy: its defense
    /// facilities, squadrons, special forces and characters, and the
    /// player's copies of its regiments on the planet.
    fn frozen_defenses(&self) -> usize {
        let Some(view) = self.frozen else {
            return 0;
        };
        let other = |side: u8| side != self.player_side;
        let facilities = view
            .defense_facilities
            .iter()
            .filter(|facility| other(crate::fleet_window::faction_side(facility.side)))
            .count();
        let squadrons: usize = view
            .fighters
            .iter()
            .filter(|(class, _)| {
                self.world
                    .fighter_classes
                    .values()
                    .find(|value| value.dat_id == *class)
                    .is_some_and(|value| other(fleet_side(value.is_alliance)))
            })
            .map(|(_, count)| *count as usize)
            .sum();
        let forces = view
            .special_forces
            .iter()
            .filter(|(_, is_alliance)| other(fleet_side(*is_alliance)))
            .count();
        let characters = view
            .characters
            .iter()
            .filter_map(|id| crate::viewed_contents::character_key(self.world, *id))
            .filter(|key| {
                let side = match rebellion_core::missions::character_side(&self.world.characters[*key])
                {
                    Some(Faction::Alliance) => 1,
                    Some(Faction::Empire) => 2,
                    _ => 0,
                };
                other(side)
            })
            .count();
        let regiments =
            crate::viewed_contents::viewed_surface_regiments(self.world, self.player, self.system)
                .len();
        facilities + squadrons + forces + characters + regiments
    }

    fn seen(&self, side: u8) -> bool {
        self.opposing_visible || side == self.player_side
    }

    fn manufacturing_and_production(&self) -> usize {
        let Some(value) = self.world.systems.get(self.system) else {
            return 0;
        };
        // FUN_0053b6e0 walks families 0x28..0x30, so the headquarters
        // (0x20), which the port keeps with the yards, is left out.
        let manufacturing = value.manufacturing_facilities.iter().filter(|key| {
            self.world
                .manufacturing_facilities
                .get(**key)
                .is_some_and(|facility| {
                    (0x28..0x30).contains(&facility.class_dat_id.family())
                        && self.seen(crate::fleet_window::faction_side(facility.side))
                })
        });
        let production = value.production_facilities.iter().filter(|key| {
            self.world
                .production_facilities
                .get(**key)
                .is_some_and(|facility| self.seen(crate::fleet_window::faction_side(facility.side)))
        });
        manufacturing.count() + production.count() + self.frozen_facilities()
    }

    /// `FUN_0045ce80`'s sum: defense facilities (`FUN_00526fd0`), regiments
    /// (`FUN_00504c40`), squadrons (`FUN_005039d0`), and each member of
    /// families `0x30..0x40` with no mission key or on a hidden mission. A
    /// regiment or squadron on its way here by Move is already in the
    /// system (`FUN_00556390`; A0 2026-11-11-regiment-en-route/00 lists one
    /// in its destination's Defenses window), so it counts.
    fn defenses(&self, missions: &MissionState, travellers: Travellers<'_>) -> usize {
        let Some(value) = self.world.systems.get(self.system) else {
            return 0;
        };
        let defense = value.defense_facilities.iter().filter(|key| {
            self.world
                .defense_facilities
                .get(**key)
                .is_some_and(|facility| self.seen(crate::fleet_window::faction_side(facility.side)))
        });
        let travelling_regiments = travellers
            .regiments
            .iter()
            .filter(|transit| transit.leg == RegimentLeg::Surface(self.system))
            .map(|transit| transit.troop);
        let regiments = value
            .ground_units
            .iter()
            .copied()
            .chain(travelling_regiments)
            .filter(|key| {
                self.world
                    .troops
                    .get(*key)
                    .is_some_and(|troop| self.seen(fleet_side(troop.is_alliance)))
            });
        let travelling_squadrons = travellers
            .squadrons
            .iter()
            .filter(|transit| transit.to == SquadronPlace::Planet(self.system))
            .map(|transit| (transit.class, transit.count));
        let squadrons: usize = value
            .fighters
            .iter()
            .map(|entry| (entry.class, entry.count))
            .chain(travelling_squadrons)
            .filter(|(class, _)| {
                self.world
                    .fighter_classes
                    .get(*class)
                    .is_some_and(|class| self.seen(fleet_side(class.is_alliance)))
            })
            .map(|(_, count)| count as usize)
            .sum();
        let keys = mission_keys(missions);
        let idle = self
            .members_with_arrivals(missions)
            .filter(|(member, _, hidden)| *hidden || !keys.contains_key(member));
        defense.count() + regiments.count() + squadrons + idle.count() + self.frozen_defenses()
    }

    /// The system's characters and special forces the player sees, each with
    /// its side and whether it is on a hidden mission.
    fn members(&self) -> impl Iterator<Item = (MissionMember, u8, bool)> + '_ {
        system_members(self.world, self.system).filter(|(_, side, _)| self.seen(*side))
    }

    /// [`Self::members`], then those on their way here: a member's transit
    /// puts it in its target's container at once (`FUN_00556430`, then
    /// `FUN_00556390` to `+0x74`; `mission-lifecycle.md`), so the system's
    /// walks count it.
    fn members_with_arrivals<'b>(
        &'b self,
        missions: &'b MissionState,
    ) -> impl Iterator<Item = (MissionMember, u8, bool)> + 'b {
        // One bound for an abode or a fleet goes into it, not the system,
        // as the Defenses window lists it (`defenses_window`).
        let arrivals = missions
            .en_route()
            .iter()
            .filter(|transit| transit.abode.is_none() && bound_fleet(missions, transit).is_none());
        self.members().chain(
            transit_members(self.world, arrivals, self.system)
                .filter(|(_, side, _)| self.seen(*side)),
        )
    }
}

/// The fleet a member's Move lands it aboard (the mission's target
/// container `+0x74`, `MissionState::set_target_fleet`): `FUN_00556390`
/// puts the member in that fleet at once, so it is the fleet's child, not
/// its system's, while it travels (`object-state-flags.md`: a walk never
/// descends).
pub(crate) fn bound_fleet(
    missions: &MissionState,
    transit: &rebellion_core::missions::MemberTransit,
) -> Option<rebellion_core::ids::FleetKey> {
    if transit.abode.is_some() {
        return None;
    }
    missions
        .missions()
        .iter()
        .find(|mission| mission.id == transit.mission_id)?
        .target_fleet
}

/// The members on their way into a fleet ([`bound_fleet`]), each with the
/// fleet, in departure order: its Fleet window lists them.
#[must_use]
pub fn fleet_arrivals(
    missions: &MissionState,
) -> Vec<(MissionMember, rebellion_core::ids::FleetKey)> {
    missions
        .en_route()
        .iter()
        .filter_map(|transit| Some((transit.member, bound_fleet(missions, transit)?)))
        .collect()
}

/// The living members whose transit ends at `system`, each with its side
/// and whether it is on a hidden mission.
pub(crate) fn transit_members<'a>(
    world: &'a GameWorld,
    transits: impl IntoIterator<Item = &'a rebellion_core::missions::MemberTransit> + 'a,
    system: SystemKey,
) -> impl Iterator<Item = (MissionMember, u8, bool)> + 'a {
    transits
        .into_iter()
        .filter(move |transit| transit.to == system)
        .filter_map(move |transit| match transit.member {
            MissionMember::Character(key) => {
                let character = world.characters.get(key).filter(|value| !value.is_killed)?;
                let side = if character.is_alliance {
                    1
                } else if character.is_empire {
                    2
                } else {
                    0
                };
                Some((transit.member, side, character.on_hidden_mission))
            }
            MissionMember::SpecialForce(key) => {
                let force = world.special_forces.get(key)?;
                Some((transit.member, fleet_side(force.is_alliance), false))
            }
        })
}

/// The characters and special forces at `system`, each with its side and
/// whether it is on a hidden mission (role `+0x78` bit 8).
///
/// port: special forces carry no hidden-mission flag.
pub(crate) fn system_members(
    world: &GameWorld,
    system: SystemKey,
) -> impl Iterator<Item = (MissionMember, u8, bool)> + '_ {
    let characters = world
        .characters
        .iter()
        .filter(move |(_, character)| {
            !character.is_killed && character.current_system == Some(system)
        })
        .map(|(key, character)| {
            // A prisoner carries its captor's side (FUN_004eec60).
            let side = match rebellion_core::missions::character_side(character) {
                Some(rebellion_core::dat::Faction::Alliance) => 1,
                Some(rebellion_core::dat::Faction::Empire) => 2,
                _ => 0,
            };
            (
                MissionMember::Character(key),
                side,
                character.on_hidden_mission,
            )
        });
    let forces = world
        .systems
        .get(system)
        .into_iter()
        .flat_map(|value| value.special_forces.iter())
        .filter_map(|key| {
            let force = world.special_forces.get(*key)?;
            Some((
                MissionMember::SpecialForce(*key),
                fleet_side(force.is_alliance),
                false,
            ))
        });
    characters.chain(forces)
}

/// Each mission member's mission key (`+0x68`): the mission whose team,
/// decoys, or captured list holds it.
pub(crate) fn mission_keys(missions: &MissionState) -> HashMap<MissionMember, u64> {
    let mut keys = HashMap::new();
    for mission in missions.missions() {
        for member in mission
            .team
            .iter()
            .chain(&mission.decoys)
            .chain(&mission.captured)
        {
            keys.insert(*member, mission.id);
        }
    }
    keys
}

/// `FUN_004a1f60(system, player side)`: the distinct visible missions of
/// the system's members, split by side. The other side's missions win, then
/// the player's; with neither it returns 3.
fn mission_side(contents: &SystemContents<'_>, missions: &MissionState) -> u8 {
    let own = contents.player_side;
    let other = if own == 1 { 2 } else { 1 };
    let keys = mission_keys(missions);
    let mut own_missions = HashSet::new();
    let mut other_missions = HashSet::new();
    for (member, side, hidden) in contents.members_with_arrivals(missions) {
        let Some(&key) = keys.get(&member) else {
            continue;
        };
        if hidden {
            continue;
        }
        if side == own {
            own_missions.insert(key);
        } else if side == other {
            other_missions.insert(key);
        }
    }
    // The other side's missions of the player's frozen copy.
    if let Some(extras) = contents.frozen_extras {
        other_missions.extend(
            extras
                .missions
                .iter()
                // As seen then, though it may have ended since (the copy
                // stays until the side sees the system: FUN_004f82d0).
                .filter(|(_, side)| *side == other)
                .map(|(id, _)| *id),
        );
    }
    if !other_missions.is_empty() {
        other
    } else if !own_missions.is_empty() {
        own
    } else {
        3
    }
}

#[cfg(test)]
#[path = "quadrant_icons_tests.rs"]
mod tests;
