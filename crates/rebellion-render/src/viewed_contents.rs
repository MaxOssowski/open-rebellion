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
use rebellion_core::ids::{DatId, SystemKey, TroopKey};
use rebellion_core::object_views::{ObjectView, RegimentView, ViewPlace};
use rebellion_core::world::GameWorld;

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
}
