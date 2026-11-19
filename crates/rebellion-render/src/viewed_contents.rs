//! What a window shows of an object: the live object, or the viewing
//! side's copy of it (`rebellion_core::object_views`,
//! `ghidra/notes/per-side-object-views.md`).
//!
//! The original's windows read the viewing side's copy of every object
//! (`FUN_004f2d10(side, id)` -> `FUN_0053ef00` -> `FUN_0053efd0`). A side's
//! own objects are always live; the other side's are its views, live or
//! frozen. port: so far only regiments read views; the debug "see
//! everything" mode (`ObjectViews::see_everything`) shows every live object
//! instead.

use rebellion_core::dat::Faction;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{DatId, SystemKey, TroopKey};
use rebellion_core::object_views::{ObjectView, RegimentView, SystemView, ViewPlace};
use rebellion_core::world::GameWorld;

/// An object of the player's frozen copy of a system that the port keeps no
/// live key for: a special force, a squadron or a facility, with its class
/// and side, and while it was seen in hyperspace its arrival day
/// (`FUN_004f9a60`, `+0x50` bit 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewedUnit {
    SpecialForce {
        class: DatId,
        is_alliance: bool,
        arrival: Option<u64>,
    },
    Squadron {
        class: DatId,
        is_alliance: bool,
        arrival: Option<u64>,
    },
    /// A defense, manufacturing or production facility; `owner` is its
    /// side, 1 or 2.
    Facility { class: DatId, owner: u8 },
}

impl ViewedUnit {
    /// Its arrival day while it was seen en route.
    #[must_use]
    pub const fn arrival(self) -> Option<u64> {
        match self {
            Self::SpecialForce { arrival, .. } | Self::Squadron { arrival, .. } => arrival,
            Self::Facility { .. } => None,
        }
    }
}

/// A regiment a window lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewedRegiment {
    /// The live regiment.
    Live(TroopKey),
    /// The viewing side's frozen copy, by the regiment's id.
    View(DatId),
}

/// Whether the debug "see everything" mode is on: every window shows live
/// objects of both sides, past the views and the fog.
#[must_use]
pub fn sees_everything(world: &GameWorld) -> bool {
    world.object_views.see_everything
}

/// How the player sees the other side's objects at a system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemSight<'a> {
    /// The live objects: the player holds the system, its copy is live
    /// (`FUN_004f9a60`), or the debug mode shows everything.
    Live,
    /// The player's frozen copy of the system (`FUN_005404d0`).
    Frozen(&'a SystemView),
    /// Nothing of the other side's.
    Hidden,
}

/// How `player` sees `system`'s other-side objects (`FUN_004f2d10` reads
/// the side's copy). A system the side holds no copy of shows nothing of
/// the other side: the original has no sight outside the copies
/// (`ghidra/notes/fog-of-war.md`). port: `_fog` is no longer read.
#[must_use]
pub fn system_sight<'a>(
    world: &'a GameWorld,
    _fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> SystemSight<'a> {
    if sees_everything(world) {
        return SystemSight::Live;
    }
    let Some(value) = world.systems.get(system) else {
        return SystemSight::Hidden;
    };
    if value.control.is_controlled_by(player) {
        return SystemSight::Live;
    }
    match world.object_views.system(player, value.dat_id) {
        Some(view) if view.live => SystemSight::Live,
        Some(view) => SystemSight::Frozen(view),
        None => SystemSight::Hidden,
    }
}

/// Gives `side` a live copy of `system` (`FUN_004f9a60` with its seen
/// bit), as a test's stand-in for a fleet of the side there.
#[cfg(test)]
pub(crate) fn see_live(world: &mut GameWorld, side: Faction, system: SystemKey) {
    let Some(value) = world.systems.get(system) else {
        return;
    };
    let id = value.dat_id;
    let view = SystemView::of(world, value, true);
    let views = &mut world.object_views;
    match side {
        Faction::Alliance => views.alliance_systems.insert(id, view),
        Faction::Empire => views.empire_systems.insert(id, view),
        Faction::Neutral => None,
    };
}

/// `system` as `player` sees it on the galaxy map and in a sector window:
/// the GID item (`FUN_0042b330`) and the planet (`FUN_0045bbb0`) read the
/// viewer's copy (`FUN_004f2d10`, `FUN_004f3220`;
/// `ghidra/notes/fog-of-war.md`). A live copy, the player's own system and
/// the debug mode show the system itself; a frozen copy shows its side,
/// support, population, resources, explored and destroyed bits and whether
/// it holds the headquarters (families `0x20..0x21`). port: the facility
/// lists stay the system's own.
#[must_use]
pub fn apparent_system<'a>(
    world: &'a GameWorld,
    player: Faction,
    system: SystemKey,
) -> Option<std::borrow::Cow<'a, rebellion_core::world::System>> {
    use std::borrow::Cow;
    let value = world.systems.get(system)?;
    if sees_everything(world) || value.control.is_controlled_by(player) {
        return Some(Cow::Borrowed(value));
    }
    let explored = |yes: bool| {
        if yes {
            rebellion_core::dat::ExplorationStatus::Explored
        } else {
            rebellion_core::dat::ExplorationStatus::Unexplored
        }
    };
    match world.object_views.system(player, value.dat_id) {
        Some(view) if !view.live => {
            let mut shown = value.clone();
            shown.control = view.control;
            shown.popularity_alliance = view.popularity_alliance;
            shown.popularity_empire = view.popularity_empire;
            shown.is_populated = view.is_populated;
            shown.total_energy = view.total_energy;
            shown.raw_materials = view.raw_materials;
            shown.exploration_status = explored(rebellion_core::object_views::side_explored(
                world, player, system,
            ));
            shown.is_destroyed =
                rebellion_core::object_views::side_destroyed(world, player, system);
            shown.is_headquarters = view
                .manufacturing_facilities
                .iter()
                .chain(&view.defense_facilities)
                .any(|facility| matches!(facility.class.family(), 0x20..=0x21));
            Some(Cow::Owned(shown))
        }
        // A live copy is synced from the system, explored (hyp: the master
        // is explored for every system, ghidra/notes/fog-of-war.md).
        Some(_) if value.exploration_status != explored(true) => {
            let mut shown = value.clone();
            shown.exploration_status = explored(true);
            Some(Cow::Owned(shown))
        }
        _ => Some(Cow::Borrowed(value)),
    }
}

/// The player's frozen copy of `system`'s fleets, missions and
/// travellers, when it sees the system through a frozen copy.
#[must_use]
pub fn frozen_extras<'a>(
    world: &'a GameWorld,
    fog: &FogState,
    player: Faction,
    system: SystemKey,
) -> Option<&'a rebellion_core::object_views::SystemExtras> {
    let SystemSight::Frozen(_) = system_sight(world, fog, player, system) else {
        return None;
    };
    let id = world.systems.get(system)?.dat_id;
    world.object_views.system_extras(player, id)
}

/// The live character a copy names by its id.
#[must_use]
pub fn character_key(world: &GameWorld, id: DatId) -> Option<rebellion_core::ids::CharacterKey> {
    world
        .characters
        .iter()
        .find(|(_, character)| character.dat_id == id)
        .map(|(key, _)| key)
}

/// `viewer`'s copy of the regiment `id`.
#[must_use]
pub fn regiment_view(world: &GameWorld, viewer: Faction, id: DatId) -> Option<&RegimentView> {
    match world.object_views.view(viewer, id)? {
        ObjectView::Regiment(view) => Some(view),
    }
}

/// The other side's regiments `viewer` holds copies of on `system`'s
/// surface, in id order: a live copy as its live regiment, a frozen one as
/// the copy.
#[must_use]
pub fn viewed_surface_regiments(
    world: &GameWorld,
    viewer: Faction,
    system: SystemKey,
) -> Vec<ViewedRegiment> {
    let Some(system_id) = world.systems.get(system).map(|value| value.dat_id) else {
        return Vec::new();
    };
    let Some(views) = world.object_views.side(viewer) else {
        return Vec::new();
    };
    let mut ids: Vec<(DatId, bool)> = views
        .iter()
        .filter_map(|(id, view)| match view {
            ObjectView::Regiment(view) if view.place == ViewPlace::Surface(system_id) => {
                Some((*id, view.live))
            }
            ObjectView::Regiment(_) => None,
        })
        .collect();
    ids.sort_by_key(|(id, _)| id.raw());
    ids.into_iter()
        .map(|(id, live)| {
            live.then(|| world.object_views.regiment_key(world, id))
                .flatten()
                .map_or(ViewedRegiment::View(id), ViewedRegiment::Live)
        })
        .collect()
}

/// The system name a regiment's copy is attached to: its planet, or the
/// fleet's name it was in.
#[must_use]
pub fn view_attached(world: &GameWorld, view: &RegimentView) -> String {
    match &view.place {
        ViewPlace::Surface(system) => world
            .systems
            .values()
            .find(|value| value.dat_id == *system)
            .map(|value| value.name.clone())
            .unwrap_or_default(),
        ViewPlace::Fleet { fleet_name, .. } => fleet_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rebellion_core::dat::{ExplorationStatus, SectorGroup};
    use rebellion_core::object_views::{apply, ViewUpdate};
    use rebellion_core::world::{ControlKind, Sector, System};

    /// port: test systems with SYSTEMSD's family 0x90 ids.
    fn system(world: &mut GameWorld, id: u32) -> SystemKey {
        let sector = world.sectors.insert(Sector {
            dat_id: DatId::new(0x9200_0000),
            name: "Sector".into(),
            group: SectorGroup::Core,
            x: 0,
            y: 0,
            systems: vec![],
        });
        world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0000 | id),
            name: format!("System {id}"),
            sector,
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
            control: ControlKind::Controlled(Faction::Empire),
        })
    }

    /// port: an Alliance copy of a test regiment.
    fn copy(id: u32, place: ViewPlace) -> ViewUpdate {
        ViewUpdate::Set {
            side: Faction::Alliance,
            id: DatId::new(id),
            view: ObjectView::Regiment(RegimentView {
                class: DatId::new(0x1000_0001),
                is_alliance: false,
                strength: 1,
                place,
                arrival: None,
                live: false,
            }),
        }
    }

    #[test]
    fn a_systems_viewed_regiments_are_the_copies_on_its_surface_in_id_order() {
        // FUN_004f2d10: the window reads the side's copies; port: in id order.
        let mut world = GameWorld::default();
        let here = system(&mut world, 1);
        system(&mut world, 2);
        let surface = ViewPlace::Surface(DatId::new(0x9000_0001));
        apply(
            &mut world,
            &[
                copy(0x1000_0009, surface.clone()),
                copy(0x1000_0003, surface),
                copy(0x1000_0004, ViewPlace::Surface(DatId::new(0x9000_0002))),
                copy(
                    0x1000_0005,
                    ViewPlace::Fleet {
                        system: DatId::new(0x9000_0001),
                        fleet_name: "Fleet 1".into(),
                    },
                ),
            ],
        );
        assert_eq!(
            viewed_surface_regiments(&world, Faction::Alliance, here),
            [
                ViewedRegiment::View(DatId::new(0x1000_0003)),
                ViewedRegiment::View(DatId::new(0x1000_0009)),
            ]
        );
        assert!(viewed_surface_regiments(&world, Faction::Empire, here).is_empty());
        let fleet_copy = regiment_view(&world, Faction::Alliance, DatId::new(0x1000_0005)).unwrap();
        assert_eq!(view_attached(&world, fleet_copy), "Fleet 1");
        let planet_copy =
            regiment_view(&world, Faction::Alliance, DatId::new(0x1000_0003)).unwrap();
        assert_eq!(view_attached(&world, planet_copy), "System 1");
    }

    /// A Rim system (SYSTEMSD family 0x92) of `id`, the Alliance's
    /// headquarters. port: test data.
    fn headquarters(world: &mut GameWorld, id: u32) -> SystemKey {
        let key = system(world, id);
        let value = &mut world.systems[key];
        value.dat_id = DatId::new(0x9200_0000 | id);
        value.control = ControlKind::Controlled(Faction::Alliance);
        value.is_headquarters = true;
        key
    }

    #[test]
    fn a_rim_system_the_side_never_saw_shows_unexplored_with_no_side_or_headquarters() {
        // FUN_0051b990 gives the Empire a bare copy of a Rim system
        // (mode 2), and FUN_0042b330 / FUN_0045bbb0 read that copy
        // (ghidra/notes/fog-of-war.md): manual p. 69, "The Empire does not
        // know the location of Alliance headquarters".
        let mut world = GameWorld::default();
        let hq = headquarters(&mut world, 7);
        rebellion_core::object_views::setup(&mut world);
        let shown = apparent_system(&world, Faction::Empire, hq).unwrap();
        assert_eq!(shown.exploration_status, ExplorationStatus::Unexplored);
        assert_eq!(shown.control, ControlKind::Uncontrolled);
        assert!(!shown.is_headquarters);
        // The Alliance sees its own system as it is.
        let own = apparent_system(&world, Faction::Alliance, hq).unwrap();
        assert!(own.is_headquarters);
        assert_eq!(own.control, ControlKind::Controlled(Faction::Alliance));
        // The debug mode sees everything.
        world.object_views.see_everything = true;
        let all = apparent_system(&world, Faction::Empire, hq).unwrap();
        assert!(all.is_headquarters);
        assert_eq!(all.exploration_status, ExplorationStatus::Explored);
    }

    #[test]
    fn a_frozen_copy_shows_its_side_support_and_destroyed_bit_as_last_seen() {
        // FUN_0045bbb0 reads +0x24 (side) and +0x50 bit 3 (destroyed) from
        // the viewer's copy, FUN_0042b330 its support (FUN_00507270).
        let mut world = GameWorld::default();
        let at = system(&mut world, 4);
        world.systems[at].control = ControlKind::Uncontrolled;
        rebellion_core::object_views::setup(&mut world);
        world.systems[at].control = ControlKind::Controlled(Faction::Alliance);
        world.systems[at].popularity_empire = 0.1;
        world.systems[at].is_destroyed = true;
        let shown = apparent_system(&world, Faction::Empire, at).unwrap();
        assert_eq!(shown.control, ControlKind::Uncontrolled);
        assert!((shown.popularity_empire - 0.5).abs() < f32::EPSILON);
        assert!(!shown.is_destroyed);
        assert_eq!(shown.exploration_status, ExplorationStatus::Explored);
    }

    #[test]
    fn a_live_copy_shows_the_system_as_it_is_and_explored() {
        // FUN_004f9a60: a side's fleet in the system keeps its copy live,
        // synced from the system (explored included, hyp in
        // ghidra/notes/fog-of-war.md).
        let mut world = GameWorld::default();
        let at = system(&mut world, 5);
        world.systems[at].dat_id = DatId::new(0x9200_0005);
        world.systems[at].exploration_status = ExplorationStatus::Unexplored;
        world.systems[at].control = ControlKind::Uncontrolled;
        rebellion_core::object_views::setup(&mut world);
        world.fleets.insert(rebellion_core::world::Fleet {
            location: at,
            capital_ships: vec![],
            fighters: vec![],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        rebellion_core::object_views::advance(
            &mut world,
            &rebellion_core::troop_transport::TroopTransportState::new(),
            &rebellion_core::movement::MovementState::new(),
        );
        world.systems[at].control = ControlKind::Controlled(Faction::Empire);
        let shown = apparent_system(&world, Faction::Alliance, at).unwrap();
        assert_eq!(shown.control, ControlKind::Controlled(Faction::Empire));
        assert_eq!(shown.exploration_status, ExplorationStatus::Explored);
        // With no copy at all, the system shows its own bits.
        let mut bare = GameWorld::default();
        let at = system(&mut bare, 6);
        bare.systems[at].exploration_status = ExplorationStatus::Unexplored;
        let shown = apparent_system(&bare, Faction::Alliance, at).unwrap();
        assert_eq!(shown.exploration_status, ExplorationStatus::Unexplored);
    }
}
