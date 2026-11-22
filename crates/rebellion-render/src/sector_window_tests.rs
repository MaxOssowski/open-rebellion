use super::*;

#[test]
fn each_track_tips_its_label_and_count() {
    // FUN_0045c240 / FUN_0045c450: label, 0x1824 " ", used, 0x1825 "/",
    // slots (A0 tooltip/132, 133); 0x1821 for support (134).
    crate::win_tooltip::set_strings(
        crate::win_tooltip::tests::TEST_STRINGS
            .iter()
            .map(|(id, text)| (*id, (*text).to_owned()))
            .collect(),
    );
    let values = StatusTrackValues {
        energy: (10, 8),
        raw_materials: (8, 2),
        support: 40,
    };
    assert_eq!(
        track_tooltips(&values),
        vec![
            (
                39.0,
                (0.0, 39.0, 30.0, 3.0),
                "Energy Consumption 8/10".to_owned()
            ),
            (43.0, (0.0, 43.0, 24.0, 3.0), "Raw Materials 2/8".to_owned()),
            (47.0, (0.0, 47.0, 37.0, 3.0), "Popular Support".to_owned()),
        ]
    );
    // A track with no slots has no tool.
    let empty = StatusTrackValues {
        energy: (0, 0),
        raw_materials: (3, 0),
        support: 0,
    };
    assert_eq!(track_tooltips(&empty).len(), 2);
}
use crate::cockpit::CockpitViewport;
use rebellion_core::dat::{ExplorationStatus, SectorGroup};
use rebellion_core::world::{ControlKind, Sector, System};

// FUN_0045bbb0: side 1's names red (0x20000ff), side 2's green
// (0x200ff00); any other side keeps the item's white (FUN_0060ba00), as
// the A0 Sesswenna capture shows; a revolt does not change the colour.
// port: an unexplored system's name stays white (A0 Moddell).
#[test]
fn a_name_takes_its_sides_colour_and_a_neutral_or_unexplored_one_stays_white() {
    let (mut world, system, _) = fixture_world();
    let mut colour = |control, status| {
        world.systems[system].control = control;
        world.systems[system].exploration_status = status;
        system_name_color(&world.systems[system])
    };
    let explored = ExplorationStatus::Explored;
    let red = egui::Color32::from_rgb(255, 0, 0);
    let green = egui::Color32::from_rgb(0, 255, 0);
    assert_eq!(
        colour(ControlKind::Controlled(Faction::Alliance), explored),
        red
    );
    assert_eq!(
        colour(ControlKind::Controlled(Faction::Empire), explored),
        green
    );
    assert_eq!(
        colour(ControlKind::Uprising(Faction::Empire), explored),
        green
    );
    assert_eq!(
        colour(ControlKind::Uncontrolled, explored),
        egui::Color32::WHITE
    );
    assert_eq!(
        colour(ControlKind::Contested, explored),
        egui::Color32::WHITE
    );
    assert_eq!(
        colour(
            ControlKind::Controlled(Faction::Empire),
            ExplorationStatus::Unexplored
        ),
        egui::Color32::WHITE
    );
}

// FUN_00459e30 creates the tracks hidden; FUN_0045c240 and FUN_0045c450
// show energy and raw materials for an explored system, FUN_0045baf0
// support for a populated one with a side; a destroyed system shows none.
#[test]
fn the_tracks_show_only_where_their_refresh_shows_them() {
    let (mut world, system, _) = fixture_world();
    let mut shown = |explored, populated, destroyed| {
        let value = &mut world.systems[system];
        value.exploration_status = if explored {
            ExplorationStatus::Explored
        } else {
            ExplorationStatus::Unexplored
        };
        value.is_populated = populated;
        value.is_destroyed = destroyed;
        [39.0, 43.0, 47.0].map(|row| track_shown(&world.systems[system], row))
    };
    assert_eq!(shown(true, true, false), [true, true, true]);
    assert_eq!(shown(true, false, false), [true, true, false]);
    assert_eq!(shown(false, true, false), [false, false, false]);
    assert_eq!(shown(true, true, true), [false, false, false]);
}

// The drawn window keeps an explored, unpopulated system's energy and
// raw-material tracks and drops its support track.
#[test]
fn an_unpopulated_planet_draws_its_slots_but_no_support_track() {
    let (mut world, system, _) = fixture_world();
    world.systems[system].is_populated = false;
    let layout = layout(2.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    let planet = state.planet_screen_rect(&world, layout, system).unwrap();
    let fog = FogState::new(Faction::Alliance);
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let missions = rebellion_core::missions::MissionState::new();
    let mut rows = Vec::new();
    for index in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(f64::from(index) * 0.05),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            let _ = draw_sector_windows(
                ctx,
                &world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            );
        });
        rows = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect)
                    if rect.rect.height() == 6.0 && rect.rect.min.x >= planet.min.x =>
                {
                    Some(((rect.rect.min.y - planet.min.y) / 2.0).round())
                }
                _ => None,
            })
            .collect();
    }
    assert!(rows.contains(&39.0), "{rows:?}");
    assert!(rows.contains(&43.0), "{rows:?}");
    assert!(!rows.contains(&47.0), "{rows:?}");
}

// FUN_0045bbb0: a system in uprising (+0x88 bit 2) takes its side's mark,
// 0x389 for side 1 and 0x38a for side 2, and none while neutral or
// calm.
#[test]
fn a_system_in_uprising_takes_its_sides_mark() {
    let (mut world, system, _) = fixture_world();
    let mut uprisings = rebellion_core::uprising::UprisingState::default();
    world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
    assert_eq!(uprising_mark(&world, Faction::Alliance, &uprisings, system), None);
    uprisings.active_uprisings.insert(
        system,
        rebellion_core::uprising::ActiveUprising {
            started_tick: 1,
            next_incident_tick: None,
            next_support_tick: None,
            side: None,
        },
    );
    assert_eq!(uprising_mark(&world, Faction::Alliance, &uprisings, system), Some(905));
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    assert_eq!(uprising_mark(&world, Faction::Alliance, &uprisings, system), Some(906));
    world.systems[system].control = ControlKind::Uncontrolled;
    assert_eq!(uprising_mark(&world, Faction::Alliance, &uprisings, system), None);
}

// FUN_0045bbb0 reads the player's copy (FUN_004f3220): a frozen copy
// synced while the system was calm shows no uprising mark
// (ghidra/notes/fog-of-war.md).
#[test]
fn a_frozen_copy_synced_calm_shows_no_uprising_mark() {
    let (mut world, system, _) = fixture_world();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    let view =
        rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    let id = world.systems[system].dat_id;
    world.object_views.alliance_systems.insert(id, view);
    let mut uprisings = rebellion_core::uprising::UprisingState::default();
    uprisings.active_uprisings.insert(
        system,
        rebellion_core::uprising::ActiveUprising {
            started_tick: 1,
            next_incident_tick: None,
            next_support_tick: None,
            side: None,
        },
    );
    assert_eq!(uprising_mark(&world, Faction::Alliance, &uprisings, system), None);
    assert_eq!(uprising_mark(&world, Faction::Empire, &uprisings, system), Some(906));
    world.object_views.system_uprisings.alliance.insert(id, true);
    assert_eq!(uprising_mark(&world, Faction::Alliance, &uprisings, system), Some(906));
}

// FUN_0045bbb0 state bit 8: the headquarters (families 0x20..0x21, the
// Alliance's only) shows on its system; the Empire sees it only with the
// Alliance's contents there.
#[test]
fn the_alliance_headquarters_shows_to_the_alliance_and_to_an_empire_that_sees_it() {
    let (mut world, system, _) = fixture_world();
    world.systems[system].is_headquarters = true;
    world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
    let fog = FogState::new(Faction::Empire);
    assert!(shows_alliance_headquarters(
        &world,
        &fog,
        Faction::Alliance,
        system
    ));
    assert!(!shows_alliance_headquarters(
        &world,
        &fog,
        Faction::Empire,
        system
    ));
    // A live Empire copy (FUN_004f9a60: an Imperial fleet in orbit,
    // FUN_0050ac80) shows it: A0 2026-11-16-edited-saves/x-39, Tokmia
    // under the Empire's blockade on day 45.
    let live =
        rebellion_core::object_views::SystemView::of(&world, &world.systems[system], true);
    let id = world.systems[system].dat_id;
    world.object_views.empire_systems.insert(id, live);
    assert!(shows_alliance_headquarters(
        &world,
        &fog,
        Faction::Empire,
        system
    ));
    world.object_views.empire_systems.remove(&id);
    // Coruscant, the Empire's: no mark.
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    assert!(!shows_alliance_headquarters(
        &world,
        &fog,
        Faction::Empire,
        system
    ));
    assert!(!shows_alliance_headquarters(
        &world,
        &fog,
        Faction::Alliance,
        system
    ));
}

// FUN_0045bbb0 walks the viewer's copy of the system (FUN_004f3220).
// The Empire's copies start at setup (FUN_005136d0 stage 0x18 ->
// FUN_0051b990) with the systems (0x90..0x98) and their 0xa0..0xaf
// children only, no facility (0x20..0x2f); the HQ object's copy turns
// live only with the system's Empire seen bit (FUN_004f7c70 ->
// FUN_0050ac80: an Imperial fleet there or the Empire holds it). A
// system the port's fog shows without either (a passing fleet's sensor
// reach, a half-way reveal) holds no headquarters for the Empire. Manual
// p. 67: "The Empire does not know the location of Alliance
// headquarters."
#[test]
fn an_empire_whose_fog_alone_shows_the_headquarters_system_sees_no_mark() {
    let (mut world, system, _) = fixture_world();
    world.systems[system].is_headquarters = true;
    world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
    let mut revealed = FogState::new(Faction::Empire);
    revealed.reveal(system);
    assert!(!shows_alliance_headquarters(
        &world,
        &revealed,
        Faction::Empire,
        system
    ));
    // The debug see-everything mode still shows it.
    world.object_views.see_everything = true;
    assert!(shows_alliance_headquarters(
        &world,
        &revealed,
        Faction::Empire,
        system
    ));
}

// FUN_0045bbb0 reads the viewer's copy (FUN_004f3220): a frozen copy
// that holds the headquarters object keeps the mark after the Empire
// has left (A0 2026-11-16-edited-saves/x-29, Duros on day 527); one
// made before the headquarters came shows none, whatever the live
// system holds.
#[test]
fn an_empire_copy_frozen_with_the_headquarters_keeps_its_mark() {
    let (mut world, system, _) = fixture_world();
    world.systems[system].control = ControlKind::Controlled(Faction::Alliance);
    let fog = FogState::new(Faction::Empire);
    let freeze = |world: &mut GameWorld| {
        let view =
            rebellion_core::object_views::SystemView::of(world, &world.systems[system], false);
        let id = world.systems[system].dat_id;
        world.object_views.empire_systems.insert(id, view);
    };
    // Frozen before the headquarters came.
    freeze(&mut world);
    let hq = world.manufacturing_facilities.insert(
        rebellion_core::world::ManufacturingFacilityInstance {
            class_dat_id: rebellion_core::ids::DatId::new(0x2000_0001),
            side: Faction::Alliance,
            is_shipyard: false,
        },
    );
    world.systems[system].manufacturing_facilities.push(hq);
    world.systems[system].is_headquarters = true;
    assert!(!shows_alliance_headquarters(
        &world,
        &fog,
        Faction::Empire,
        system
    ));
    // Frozen with it: the mark stays, here even after it has moved on.
    freeze(&mut world);
    world.systems[system]
        .manufacturing_facilities
        .retain(|key| *key != hq);
    world.systems[system].is_headquarters = false;
    assert!(shows_alliance_headquarters(
        &world,
        &fog,
        Faction::Empire,
        system
    ));
    // A destroyed system shows none.
    world.systems[system].is_destroyed = true;
    assert!(!shows_alliance_headquarters(
        &world,
        &fog,
        Faction::Empire,
        system
    ));
}

fn fixture_world() -> (GameWorld, SystemKey, SystemKey) {
    let mut world = GameWorld::default();
    let sector_a = world.sectors.insert(Sector {
        dat_id: DatId::new(36),
        name: "Sesswenna".into(),
        group: SectorGroup::Core,
        x: 317,
        y: 248,
        systems: Vec::new(),
    });
    let system_a = world.systems.insert(System {
        dat_id: DatId::new(263),
        name: "Chandrila".into(),
        sector: sector_a,
        x: 322,
        y: 260,
        exploration_status: ExplorationStatus::Explored,
        popularity_alliance: 1.0,
        is_populated: true,
        total_energy: 8,
        raw_materials: 6,
        control: ControlKind::Controlled(Faction::Alliance),
        ..Default::default()
    });
    world.sectors[sector_a].systems.push(system_a);

    let sector_b = world.sectors.insert(Sector {
        dat_id: DatId::new(37),
        name: "Sluis".into(),
        group: SectorGroup::RimInner,
        x: 526,
        y: 532,
        systems: Vec::new(),
    });
    let system_b = world.systems.insert(System {
        dat_id: DatId::new(270),
        name: "Bothawui".into(),
        sector: sector_b,
        x: 560,
        y: 548,
        exploration_status: ExplorationStatus::Explored,
        popularity_empire: 1.0,
        is_populated: true,
        total_energy: 5,
        raw_materials: 9,
        control: ControlKind::Controlled(Faction::Empire),
        ..Default::default()
    });
    world.sectors[sector_b].systems.push(system_b);
    (world, system_a, system_b)
}

fn layout(scale: f32) -> CockpitLayout {
    CockpitLayout {
        canvas: CockpitViewport {
            x: 10.0,
            y: 20.0,
            width: 640.0 * scale,
            height: 480.0 * scale,
        },
        galaxy: CockpitViewport {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        },
        scale,
    }
}

#[test]
fn a_destroyed_system_shows_the_asteroid_field() {
    // FUN_0045bbb0: an item whose system is destroyed (+0x50 bit 3)
    // takes state 0x40, STRATEGY bitmap 0x2800 (10240) and the name
    // colour 0x2ffff00.
    let (mut world, system, _) = fixture_world();
    assert_eq!(planet_art(&world.systems[system]), 10224);
    world.systems[system].is_destroyed = true;
    assert_eq!(planet_art(&world.systems[system]), 10240);
    // A0 menus3/05 draws the 0x2ffff00 name white, as the unheld ones.
    assert_eq!(DESTROYED_NAME_COLOR, egui::Color32::WHITE);
}

#[test]
fn original_planet_mapping_covers_all_special_ids() {
    // Source: SYSTEMSD.DAT picture_id values and STRATEGY.DLL bitmap resources 10212-10239.
    assert_eq!(planet_resource_id(DatId::new(100)), 10212);
    assert_eq!(planet_resource_id(DatId::new(263)), 10224);
    assert_eq!(planet_resource_id(DatId::new(269)), 10215);
    assert_eq!(planet_resource_id(DatId::new(271)), 10239);
    assert_eq!(planet_resource_id(DatId::new(279)), 10237);
    assert_eq!(planet_resource_id(DatId::new(282)), 10238);
    assert_eq!(planet_resource_id(DatId::new(99)), 10212);
}

#[test]
fn the_status_tracks_stack_energy_raw_and_support_under_the_planet() {
    // FUN_00459e30: tracks 3 tall at the planet's bottom + 2, then 1
    // apart; FUN_0045c240 / FUN_0045c450 size energy and raw to 3 per
    // slot; manual p. 33 (Fig. 2.18): white energy in use, blue free,
    // yellow mined, red-orange unmined. Frame 0480 shows each slot as
    // a 2-pixel block after a 1-pixel gap and Vagran's support bar
    // 15 green and 22 red for the Empire player.
    let values = StatusTrackValues {
        energy: (3, 1),
        raw_materials: (2, 2),
        support: 40,
    };
    let white = egui::Color32::from_rgb(255, 255, 255);
    let blue = egui::Color32::from_rgb(0, 0, 255);
    let yellow = egui::Color32::from_rgb(255, 255, 0);
    let green = egui::Color32::from_rgb(0, 255, 0);
    let red = egui::Color32::from_rgb(255, 0, 0);
    // Each energy and raw-material slot starts with a 1-pixel tick in
    // the track's pen 0x808080 (FUN_00459e30 -> FUN_0060e3b0; tick step
    // 1 from FUN_0045c240 / FUN_0045c450 -> FUN_0060e400), drawn over
    // the fill at x = slot * width / slots (Class_0065c058_vt14,
    // FUN_004ad210): A0 headquarters/00-moddell and sluis-van-768/07
    // show (128, 128, 128) before every 2-pixel slot.
    let grey = egui::Color32::from_rgb(128, 128, 128);
    assert_eq!(
        status_tracks(10.0, 100.0, &values, Faction::Empire),
        [
            ((11.0, 139.0, 2.0, 3.0), white),
            ((14.0, 139.0, 2.0, 3.0), blue),
            ((17.0, 139.0, 2.0, 3.0), blue),
            ((10.0, 139.0, 1.0, 3.0), grey),
            ((13.0, 139.0, 1.0, 3.0), grey),
            ((16.0, 139.0, 1.0, 3.0), grey),
            ((11.0, 143.0, 2.0, 3.0), yellow),
            ((14.0, 143.0, 2.0, 3.0), yellow),
            ((10.0, 143.0, 1.0, 3.0), grey),
            ((13.0, 143.0, 1.0, 3.0), grey),
            ((10.0, 147.0, 15.0, 3.0), green),
            ((25.0, 147.0, 22.0, 3.0), red),
        ]
    );
    let alliance = status_tracks(10.0, 100.0, &values, Faction::Alliance);
    assert_eq!(alliance[10], ((10.0, 147.0, 15.0, 3.0), red));
    assert_eq!(alliance[11], ((25.0, 147.0, 22.0, 3.0), green));
    // The name starts below the tracks, at the label item's offset.
    assert_eq!(NAME_OFFSET, 37.0 + 11.0);
}

#[test]
fn the_tracks_count_facilities_as_energy_mines_as_raw_and_the_players_support() {
    // Manual p. 33 (Fig. 2.18): one energy slot per facility; mines take
    // the raw-material slots. FUN_0045b770 passes the player's support.
    use rebellion_core::world::{
        DefenseFacilityInstance, ManufacturingFacilityInstance, ProductionFacilityInstance,
    };
    let (mut world, system, _) = fixture_world();
    add_mine(&mut world, system);
    add_mine(&mut world, system);
    let refinery = world
        .production_facilities
        .insert(ProductionFacilityInstance {
            class_dat_id: DatId::new(0x2d00_0001),
            side: Faction::Alliance,
            is_mine: false,
        });
    world.systems[system].production_facilities.push(refinery);
    let yard = world
        .manufacturing_facilities
        .insert(ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2800_0001),
            side: Faction::Alliance,
            is_shipyard: true,
        });
    world.systems[system].manufacturing_facilities.push(yard);
    let shield = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(0x2200_0001),
        side: Faction::Alliance,
    });
    world.systems[system].defense_facilities.push(shield);
    world.systems[system].popularity_alliance = 0.75;
    world.systems[system].popularity_empire = 0.25;

    let of = |player| StatusTrackValues::of(&world, &world.systems[system], player);
    // A frozen copy keeps the values it was seen with (FUN_004f9a60).
    let copy = rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    assert_eq!(StatusTrackValues::of_copy(&copy, Faction::Alliance), of(Faction::Alliance));
    assert_eq!(StatusTrackValues::of_copy(&copy, Faction::Empire), of(Faction::Empire));
    let mut frozen = world.clone();
    frozen
        .object_views
        .empire_systems
        .insert(frozen.systems[system].dat_id, copy);
    frozen.systems[system].popularity_empire = 0.9;
    frozen.systems[system].production_facilities.clear();
    assert_eq!(
        StatusTrackValues::shown(
            &frozen,
            &FogState::new(Faction::Empire),
            &frozen.systems[system],
            system,
            Faction::Empire
        ),
        of(Faction::Empire)
    );
    assert_eq!(
        of(Faction::Alliance),
        StatusTrackValues {
            energy: (8, 5),
            raw_materials: (6, 2),
            support: 75,
        }
    );
    assert_eq!(of(Faction::Empire).support, 25);

    // Full support leaves no empty share to draw.
    let full = StatusTrackValues {
        energy: (0, 0),
        raw_materials: (0, 0),
        support: 100,
    };
    assert_eq!(
        status_tracks(0.0, 0.0, &full, Faction::Empire),
        [((0.0, 47.0, 37.0, 3.0), egui::Color32::from_rgb(0, 255, 0))]
    );
}

#[test]
fn a_planet_sits_at_its_sector_offset_scaled_to_the_window() {
    // FUN_00459e30 at 0x45a3d4: offset / 1024 * 13 * 235 across and
    // offset / 1024 * 10 * 360 down, truncated. The Corellian sector at
    // SECTORSD (258, 394) in frame 0480 of the failed multiplayer
    // recording places Xyquine, Corellia, Corfai, Tralus and Vagran at
    // these points, one pixel left of and two below the window's frame
    // line (Tralus three below).
    let at = |x, y| sector_planet_position(258, 394, x, y);
    assert_eq!(at(272, 401), (41.0, 24.0));
    assert_eq!(at(314, 419), (167.0, 87.0));
    assert_eq!(at(270, 466), (35.0, 253.0));
    assert_eq!(at(307, 471), (146.0, 270.0));
    assert_eq!(at(287, 478), (86.0, 295.0));
}

#[test]
fn modeless_windows_alternate_columns_and_raise_without_duplicates() {
    let (world, first, second) = fixture_world();
    let mut state = SectorWindowState::default();
    assert!(state.open_for_system(&world, first, CockpitFaction::Alliance));
    // FUN_00429ce0 tells the agent of a created window only.
    assert!(state.take_created());
    assert!(!state.take_created());
    assert!(state.open_for_system(&world, second, CockpitFaction::Alliance));
    assert_eq!(state.window_count(), 2);
    assert_eq!(state.windows[0].column, WindowColumn::Primary);
    assert_eq!(state.windows[1].column, WindowColumn::Secondary);
    assert!(state.take_created());
    assert!(state.open_for_system(&world, first, CockpitFaction::Alliance));
    assert!(!state.take_created());
    assert_eq!(state.window_count(), 2);
    assert_eq!(
        state.windows.last().unwrap().sector,
        world.systems[first].sector
    );
}

/// Add a one-system sector at map x `x` and return its system.
fn add_sector(world: &mut GameWorld, x: u16) -> SystemKey {
    let template = world.systems.values().next().unwrap().clone();
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(38),
        name: "Added".into(),
        group: SectorGroup::Core,
        x,
        y: 300,
        systems: Vec::new(),
    });
    let system = world.systems.insert(System { sector, ..template });
    world.sectors[sector].systems.push(system);
    system
}

#[test]
fn a_first_window_takes_the_column_of_its_sectors_half_of_the_galaxy() {
    // FUN_00429ce0: with both columns free, a sector at x >= 1023 / 2
    // (DAT_00658bd8) opens in the second column.
    let (world, left, right) = fixture_world();
    for (system, column) in [
        (left, WindowColumn::Primary),
        (right, WindowColumn::Secondary),
    ] {
        let mut state = SectorWindowState::default();
        assert!(state.open_for_system(&world, system, CockpitFaction::Empire));
        assert_eq!(state.windows[0].column, column);
    }
    // The other column is the free one, whatever the half.
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, right, CockpitFaction::Alliance);
    let mut world = world;
    let also_right = add_sector(&mut world, 900);
    state.open_for_system(&world, also_right, CockpitFaction::Alliance);
    assert_eq!(state.windows[1].column, WindowColumn::Primary);
}

#[test]
fn a_third_sector_window_replaces_the_one_on_its_half_of_the_galaxy() {
    // FUN_00429ce0 → FUN_00600f90: with both columns taken, the window
    // in the new sector's half's column is destroyed and replaced.
    let (mut world, left, right) = fixture_world();
    let far_left = add_sector(&mut world, 100);
    let far_right = add_sector(&mut world, 1000);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, left, CockpitFaction::Alliance);
    state.open_for_system(&world, right, CockpitFaction::Alliance);

    state.open_for_system(&world, far_left, CockpitFaction::Alliance);
    assert_eq!(state.window_count(), 2);
    let sectors = |state: &SectorWindowState| {
        let mut open: Vec<_> = state
            .windows
            .iter()
            .map(|window| (window.column == WindowColumn::Primary, window.sector))
            .collect();
        open.sort_by_key(|(primary, _)| !primary);
        open
    };
    assert_eq!(
        sectors(&state),
        [
            (true, world.systems[far_left].sector),
            (false, world.systems[right].sector)
        ]
    );

    state.open_for_system(&world, far_right, CockpitFaction::Alliance);
    assert_eq!(
        sectors(&state),
        [
            (true, world.systems[far_left].sector),
            (false, world.systems[far_right].sector)
        ]
    );
}

#[test]
fn faction_change_clears_stale_windows() {
    let (world, first, _) = fixture_world();
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, first, CockpitFaction::Alliance);
    state.prepare_faction(CockpitFaction::Empire);
    assert_eq!(state.window_count(), 0);
}

#[test]
fn occlusion_uses_scaled_exclusive_rect_edges() {
    let (world, first, _) = fixture_world();
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, first, CockpitFaction::Alliance);
    let layout = layout(2.0);
    assert!(state.contains_screen_point(layout, (130.0, 90.0)));
    assert!(state.contains_screen_point(layout, (599.9, 809.9)));
    assert!(!state.contains_screen_point(layout, (600.0, 90.0)));
    assert!(!state.contains_screen_point(layout, (130.0, 810.0)));
}

#[test]
fn original_control_rects_exclude_right_and_bottom_edges() {
    let rect = egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(24.0, 34.0));
    assert!(rect_contains(rect, egui::pos2(10.0, 20.0)));
    assert!(rect_contains(rect, egui::pos2(23.999, 33.999)));
    assert!(!rect_contains(rect, egui::pos2(24.0, 20.0)));
    assert!(!rect_contains(rect, egui::pos2(10.0, 34.0)));
}

thread_local! {
    /// The bitmap folder the click helpers' caches read, if any.
    static CLICK_BASE: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

fn click_cache() -> BmpCache {
    let mut cache = BmpCache::new();
    if let Some(base) = CLICK_BASE.with(|base| base.borrow().clone()) {
        cache.set_base_path(base);
    }
    cache
}

/// Draw `system`'s sector window and double-click at the canvas point
/// `at`, returning every action.
fn double_click_at(
    world: &GameWorld,
    system: SystemKey,
    at: (f32, f32),
) -> Vec<SectorWindowAction> {
    double_click_with_missions(
        world,
        system,
        &rebellion_core::missions::MissionState::new(),
        at,
    )
}

fn double_click_with_missions(
    world: &GameWorld,
    system: SystemKey,
    missions: &rebellion_core::missions::MissionState,
    at: (f32, f32),
) -> Vec<SectorWindowAction> {
    let layout = layout(1.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(world, system, CockpitFaction::Alliance);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    let ctx = egui::Context::default();
    let mut cache = click_cache();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let point = egui::pos2(layout.canvas.x + at.0, layout.canvas.y + at.1);
    let button = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let mut actions = Vec::new();
    let frames = [
        vec![egui::Event::PointerMoved(point)],
        vec![egui::Event::PointerMoved(point)],
        vec![button(true)],
        vec![button(false)],
        vec![button(true)],
        vec![button(false)],
        vec![],
    ];
    for (index, events) in frames.into_iter().enumerate() {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            time: Some(index as f64 * 0.05),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            actions.extend(draw_sector_windows(
                ctx,
                world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                missions,
            ));
        });
    }
    actions
}

/// Draw `system`'s sector window and click each `(button, point)` in
/// turn, a press and a release apiece, in one window state.
fn clicks_at(
    world: &GameWorld,
    system: SystemKey,
    clicks: &[(egui::PointerButton, (f32, f32))],
) -> Vec<SectorWindowAction> {
    let layout = layout(1.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(world, system, CockpitFaction::Alliance);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    let missions = rebellion_core::missions::MissionState::new();
    let ctx = egui::Context::default();
    let mut cache = click_cache();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let mut frames = Vec::new();
    for &(button, at) in clicks {
        let pos = egui::pos2(layout.canvas.x + at.0, layout.canvas.y + at.1);
        let event = |pressed| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        frames.push(vec![egui::Event::PointerMoved(pos)]);
        frames.push(vec![event(true)]);
        frames.push(vec![event(false)]);
        frames.push(vec![]);
    }
    let mut actions = Vec::new();
    for (index, events) in frames.into_iter().enumerate() {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            // Far enough apart that no two clicks make a double click.
            time: Some(index as f64 * 0.5),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            actions.extend(draw_sector_windows(
                ctx,
                world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            ));
        });
    }
    actions
}

fn menus(actions: &[SectorWindowAction]) -> Vec<(Option<MenuObject>, (i16, i16))> {
    actions
        .iter()
        .filter_map(|action| match *action {
            SectorWindowAction::OpenObjectMenu { selection, point } => Some((selection, point)),
            _ => None,
        })
        .collect()
}

const RIGHT: egui::PointerButton = egui::PointerButton::Secondary;
const LEFT: egui::PointerButton = egui::PointerButton::Primary;

#[test]
fn a_right_click_on_the_fleet_icon_opens_its_menu_at_the_cursor() {
    // FUN_004593e0 case 0x204: FUN_0045cc10 finds the shown overlay and
    // FUN_0045b1b0 makes it the window's selection (FUN_004f5b10: the
    // system and the kind). WM_RBUTTONUP reaches slot 7, FUN_004ac5c0,
    // with the cursor. (115, 90) is on the icon, past the planet.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let fleets = Some(MenuObject::SystemIcon {
        system,
        quadrant: Quadrant::Fleets,
    });
    assert_eq!(
        menus(&clicks_at(&world, system, &[(RIGHT, (115.0, 90.0))])),
        [(fleets, (115, 90))]
    );
    // On the planet's own picture, where the icon overlaps it.
    assert_eq!(
        menus(&clicks_at(&world, system, &[(RIGHT, (100.0, 85.0))])),
        [(fleets, (100, 85))]
    );
}

/// Make `system` the Alliance headquarters, holding the HQ object, and
/// give the click helpers a 904 whose pixels are opaque in x 10..30 and
/// y 5..20, in column 0 and in row 0, and keyed (the first DIB pixel's
/// index) elsewhere.
fn with_headquarters(
    world: &mut GameWorld,
    system: SystemKey,
) -> (
    rebellion_core::ids::ManufacturingFacilityKey,
    std::path::PathBuf,
) {
    world.systems[system].is_headquarters = true;
    let hq = world.manufacturing_facilities.insert(
        rebellion_core::world::ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2000_0001),
            side: Faction::Alliance,
            is_shipyard: false,
        },
    );
    world.systems[system].manufacturing_facilities.push(hq);
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let base = std::env::temp_dir().join(format!(
        "open-rebellion-sector-headquarters-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let folder = base.join("strategy-dll").join("BMP");
    std::fs::create_dir_all(&folder).unwrap();
    let (width, height) = (37_usize, 37_usize);
    let stride = 40;
    let mut bytes = Vec::new();
    let offset = 14 + 40 + 4 * 2;
    bytes.extend(b"BM");
    bytes.extend(
        u32::try_from(offset + stride * height)
            .unwrap()
            .to_le_bytes(),
    );
    bytes.extend([0; 4]);
    bytes.extend(u32::try_from(offset).unwrap().to_le_bytes());
    bytes.extend(40u32.to_le_bytes());
    bytes.extend(i32::try_from(width).unwrap().to_le_bytes());
    bytes.extend(i32::try_from(height).unwrap().to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8u16.to_le_bytes());
    bytes.extend([0; 4]);
    bytes.extend(u32::try_from(stride * height).unwrap().to_le_bytes());
    bytes.extend([0; 8]);
    bytes.extend(2u32.to_le_bytes());
    bytes.extend([0; 4]);
    bytes.extend([255, 0, 0, 0, 255, 255, 255, 0]);
    // Bottom-up rows; the first stored pixel, (0, 36), is the key.
    for stored in 0..height {
        let y = height - 1 - stored;
        for x in 0..stride {
            let opaque = x < width
                && ((x == 0 && y != 36)
                    || y == 0
                    || ((10..30).contains(&x) && (5..20).contains(&y)));
            bytes.push(u8::from(opaque));
        }
    }
    std::fs::write(folder.join("904.bmp"), bytes).unwrap();
    CLICK_BASE.with(|cell| *cell.borrow_mut() = Some(base.clone()));
    (hq, base)
}

fn without_headquarters_art(base: std::path::PathBuf) {
    CLICK_BASE.with(|cell| *cell.borrow_mut() = None);
    std::fs::remove_dir_all(base).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_headquarters_pixel_test_wants_an_opaque_pixel_above_offset_0_at_any_scale() {
    // FUN_005fca00: the offset into the overlay's bitmap must be above 0
    // in x and y and its pixel not the key.
    let (mut world, system, _) = fixture_world();
    let (_, base) = with_headquarters(&mut world, system);
    let mut cache = click_cache();
    let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(74.0, 74.0));
    let hit = |cache: &mut BmpCache, x: f32, y: f32| {
        headquarters_hit(cache, rect, 2.0, egui::pos2(10.0 + 2.0 * x, 20.0 + 2.0 * y))
    };
    let results = [
        hit(&mut cache, 15.0, 6.0),
        hit(&mut cache, 0.0, 10.0),
        hit(&mut cache, 15.0, 0.0),
        hit(&mut cache, 33.0, 2.0),
        hit(&mut cache, 29.5, 19.5),
    ];
    without_headquarters_art(base);

    // Opaque; column 0 and row 0, opaque but at offset 0; keyed; the
    // last opaque pixel, half a logical pixel in.
    assert_eq!(results, [true, false, false, false, true]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_headquarters_takes_a_press_on_its_opaque_pixels_before_the_icons() {
    // FUN_0045cc10 walks the overlay list sorted by id (FUN_0060a790(1),
    // Class_0066e148_vt4), so the headquarters overlay 0x10000
    // (FUN_0045bbb0) comes before the fleet icon (sys_id | 0x100000).
    // FUN_005fca00 takes it only where 904 is opaque, at an offset above
    // 0. Its object is the HQ (FUN_0052bed0), whose menu FUN_00526c00
    // lists. Chandrila's planet sits at (74, 79); the fleet icon spans
    // (93, 78) to (121, 97).
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let (hq, base) = with_headquarters(&mut world, system);
    let fleets = Some(MenuObject::SystemIcon {
        system,
        quadrant: Quadrant::Fleets,
    });

    // (100, 85) is 904's (26, 6), opaque, under the fleet icon.
    let on_mark = menus(&clicks_at(&world, system, &[(RIGHT, (100.0, 85.0))]));
    // (108, 81) is its (34, 2), keyed.
    let keyed = menus(&clicks_at(&world, system, &[(RIGHT, (108.0, 81.0))]));
    // (74, 89) is its (0, 10): opaque, but FUN_005fca00 wants x above 0.
    // No icon lies there: the HQ lights none (FUN_0045cdc0).
    let left_edge = menus(&clicks_at(&world, system, &[(RIGHT, (74.0, 89.0))]));
    // A double click on the mark opens no Fleet window (FUN_0045aac0
    // has no window for kind 1).
    let double = double_click_at(&world, system, (100.0, 85.0));
    // A press on the mark, then on the fleet icon, moves the selection.
    let then_icon = menus(&clicks_at(
        &world,
        system,
        &[(LEFT, (100.0, 85.0)), (RIGHT, (108.0, 81.0))],
    ));
    without_headquarters_art(base);

    assert_eq!(on_mark, [(Some(MenuObject::Headquarters(hq)), (100, 85))]);
    assert_eq!(keyed, [(fleets, (108, 81))]);
    assert_eq!(left_edge, [(None, (74, 89))]);
    assert!(
        !double
            .iter()
            .any(|action| matches!(action, SectorWindowAction::OpenFleetWindow { .. })),
        "{double:?}"
    );
    assert_eq!(then_icon, [(fleets, (108, 81))]);
}

#[test]
fn a_drag_from_the_headquarters_mark_drops_the_headquarters() {
    // The manual (p. 134, Fig. 3.82): "drag the HQ icon onto a new
    // system". FUN_004593e0 drags the selected overlay as an icon; its
    // object is the HQ (FUN_0052bed0).
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let (hq, base) = with_headquarters(&mut world, system);
    let none = egui::Modifiers::default();
    let dropped = drops(&drag_at(
        &world,
        system,
        (100.0, 85.0),
        (200.0, 150.0),
        none,
    ));
    let ctrl = drops(&drag_at(
        &world,
        system,
        (100.0, 85.0),
        (200.0, 150.0),
        egui::Modifiers::CTRL,
    ));
    without_headquarters_art(base);

    let at = (10.0 + 200.0, 20.0 + 150.0);
    assert_eq!(dropped, [(MenuObject::Headquarters(hq), at, false)]);
    assert_eq!(ctrl, [(MenuObject::Headquarters(hq), at, true)]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_point_on_the_headquarters_mark_names_the_headquarters_not_its_planet() {
    // FUN_0045cc10 walks the overlay list first, and the headquarters
    // overlay (0x10000) takes a point wherever its bitmap is opaque, so
    // the window's +0x68 there names the HQ (family 0x20), an object
    // that FUN_00537180 refuses as a Destination (1/0x25; FUN_00422ce0
    // 0x214). A0 2026-11-13-facility-icon-drop/03..07: Jerijador's
    // facility icon dropped on the HQ dome over Yavin's planet centre
    // sets nothing; 15, 16: dropped on the planet's edge, off the dome,
    // it sets Yavin.
    let (mut world, system, _) = fixture_world();
    let (_, base) = with_headquarters(&mut world, system);
    let layout = layout(1.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    let ctx = egui::Context::default();
    let mut cache = click_cache();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(700.0, 520.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        let _ = draw_sector_windows(
            ctx,
            &world,
            &rebellion_core::movement::MovementState::default(),
            crate::quadrant_icons::Travellers::default(),
            &fog,
            &mut state,
            CockpitFaction::Alliance,
            layout,
            &mut cache,
            &rebellion_core::uprising::UprisingState::default(),
            &rebellion_core::missions::MissionState::new(),
        );
    });
    let layer = egui::LayerId::new(egui::Order::Middle, area_id(world.systems[system].sector));
    let at = |x: f32, y: f32| egui::pos2(layout.canvas.x + x, layout.canvas.y + y);
    // The mark's opaque middle, and the planet's corner where the mark
    // is keyed out.
    let on_mark = state.headquarters_at(&mut cache, layout, layer, at(100.0, 85.0));
    let off_mark = state.headquarters_at(&mut cache, layout, layer, at(76.0, 110.0));
    let planet = state.release_target(&world, layout, layer, at(76.0, 110.0));
    without_headquarters_art(base);

    assert_eq!(on_mark, Some(system));
    assert_eq!(off_mark, None);
    assert_eq!(planet, Some(Some(system)));
}

#[test]
fn each_shown_icon_is_its_own_selection() {
    // FUN_00459e30 gives each quadrant its own item and kind.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    add_mine(&mut world, system);
    let troop = world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(0x1000_0002),
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(troop);
    for (quadrant, at) in [
        (Quadrant::System, (66.0, 80.0)),
        (Quadrant::Defenses, (66.0, 110.0)),
        (Quadrant::Fleets, (115.0, 80.0)),
    ] {
        assert_eq!(
            menus(&clicks_at(&world, system, &[(RIGHT, at)])),
            [(
                Some(MenuObject::SystemIcon { system, quadrant }),
                (at.0 as i16, at.1 as i16)
            )],
            "{quadrant:?}"
        );
    }
}

#[test]
fn a_press_off_every_icon_keeps_the_windows_selection() {
    // FUN_004593e0 cases 0x201 and 0x204 return before FUN_0045afc0 when
    // FUN_0045cc10 finds nothing, so the last icon chosen stays selected
    // and the next right release (FUN_004ac5c0) opens its menu.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let fleets = Some(MenuObject::SystemIcon {
        system,
        quadrant: Quadrant::Fleets,
    });
    // (200, 250) is empty window space, far from any planet.
    assert_eq!(
        menus(&clicks_at(
            &world,
            system,
            &[(LEFT, (115.0, 90.0)), (RIGHT, (200.0, 250.0))]
        )),
        [(fleets, (200, 250))]
    );
    // With nothing chosen yet, the menu opens for an empty selection.
    assert_eq!(
        menus(&clicks_at(&world, system, &[(RIGHT, (200.0, 250.0))])),
        [(None, (200, 250))]
    );
}

#[test]
fn a_right_release_outside_the_window_opens_no_menu() {
    // WM_RBUTTONUP reaches the window under the cursor; the galaxy view
    // beyond the window's right edge (60 + 235) is not this window.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    assert!(menus(&clicks_at(
        &world,
        system,
        &[(LEFT, (115.0, 90.0)), (RIGHT, (400.0, 200.0))]
    ))
    .is_empty());
}

/// Press the left button at canvas point `from`, move to `to` and
/// release there, with `modifiers` held.
fn drag_at(
    world: &GameWorld,
    system: SystemKey,
    from: (f32, f32),
    to: (f32, f32),
    modifiers: egui::Modifiers,
) -> Vec<SectorWindowAction> {
    drags_at(world, system, &[(from, to)], modifiers)
}

/// A left-button drag's canvas points: where it presses, where it
/// releases.
type Drag = ((f32, f32), (f32, f32));

/// Each drag of `drags` in turn, in one window state.
fn drags_at(
    world: &GameWorld,
    system: SystemKey,
    drags: &[Drag],
    modifiers: egui::Modifiers,
) -> Vec<SectorWindowAction> {
    let frames = drags.iter().flat_map(|&(from, to)| {
        [
            vec![Step::Move(from)],
            vec![Step::Press(from)],
            vec![Step::Move(to)],
            vec![Step::Release(to)],
            vec![],
        ]
    });
    steps_at(world, system, frames, modifiers)
}

/// One pointer event of a frame, at a canvas point.
#[derive(Clone, Copy)]
enum Step {
    Move((f32, f32)),
    Press((f32, f32)),
    Release((f32, f32)),
}

/// One frame per entry of `frames`, each with its steps in order, in
/// one window state.
fn steps_at(
    world: &GameWorld,
    system: SystemKey,
    frames: impl IntoIterator<Item = Vec<Step>>,
    modifiers: egui::Modifiers,
) -> Vec<SectorWindowAction> {
    let layout = layout(1.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(world, system, CockpitFaction::Alliance);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    let missions = rebellion_core::missions::MissionState::new();
    let ctx = egui::Context::default();
    let mut cache = click_cache();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let at =
        |point: (f32, f32)| egui::pos2(layout.canvas.x + point.0, layout.canvas.y + point.1);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };
    let frames = frames.into_iter().map(|steps| {
        steps
            .into_iter()
            .map(|step| match step {
                Step::Move(point) => egui::Event::PointerMoved(at(point)),
                Step::Press(point) => button(at(point), true),
                Step::Release(point) => button(at(point), false),
            })
            .collect::<Vec<_>>()
    });
    let mut actions = Vec::new();
    for (index, events) in frames.into_iter().enumerate() {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            time: Some(index as f64 * 0.05),
            modifiers,
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            actions.extend(draw_sector_windows(
                ctx,
                world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            ));
        });
    }
    actions
}

fn drops(actions: &[SectorWindowAction]) -> Vec<(MenuObject, (f32, f32), bool)> {
    actions
        .iter()
        .filter_map(|action| match *action {
            SectorWindowAction::DropSelection {
                selection,
                point,
                confirmed,
            } => Some((selection, (point.x, point.y), confirmed)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_drag_from_the_fleet_icon_drops_its_selection_where_released() {
    // FUN_004593e0: case 0x201 selects the icon and keeps the press
    // point; case 0x202 past four pixels posts 0x29a with the cursor.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let fleets = MenuObject::SystemIcon {
        system,
        quadrant: Quadrant::Fleets,
    };
    // Released at (200, 250) on the canvas, which sits at (10, 20).
    assert_eq!(
        drops(&drag_at(
            &world,
            system,
            (115.0, 90.0),
            (200.0, 250.0),
            egui::Modifiers::NONE
        )),
        [(fleets, (210.0, 270.0), false)]
    );
    // Ctrl held: FUN_00422ce0 issues 0x202 (GetAsyncKeyState(0x11)).
    assert_eq!(
        drops(&drag_at(
            &world,
            system,
            (115.0, 90.0),
            (200.0, 250.0),
            egui::Modifiers::CTRL
        )),
        [(fleets, (210.0, 270.0), true)]
    );
}

#[test]
fn a_release_within_four_pixels_of_the_press_drops_nothing() {
    // FUN_004593e0 case 0x202: 4 < |dx| or 4 < |dy|.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let none = egui::Modifiers::NONE;
    assert!(drops(&drag_at(&world, system, (110.0, 90.0), (114.0, 86.0), none)).is_empty());
    assert_eq!(
        drops(&drag_at(&world, system, (110.0, 90.0), (110.0, 95.0), none)).len(),
        1
    );
    assert_eq!(
        drops(&drag_at(&world, system, (110.0, 90.0), (115.0, 90.0), none)).len(),
        1
    );
}

#[test]
fn a_dropped_selection_does_not_drop_again_on_the_next_release() {
    // FUN_004593e0 case 0x202 ends the drag it posts 0x29a for; a later
    // press off every icon (case 0x201 returns early) starts none.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    assert_eq!(
        drops(&drags_at(
            &world,
            system,
            &[
                ((115.0, 90.0), (200.0, 250.0)),
                ((200.0, 250.0), (150.0, 200.0))
            ],
            egui::Modifiers::NONE
        ))
        .len(),
        1
    );
}

/// Press the left button at canvas point `from` and move, held, to
/// `to`: the window state's drag image and what the last frame painted.
fn held_drag(
    world: &GameWorld,
    system: SystemKey,
    from: (f32, f32),
    to: (f32, f32),
    scale: f32,
) -> (Option<(u32, egui::Rect)>, Vec<(u32, egui::Pos2)>) {
    let layout = layout(scale);
    let mut state = SectorWindowState::default();
    state.open_for_system(world, system, CockpitFaction::Alliance);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    let missions = rebellion_core::missions::MissionState::new();
    let ctx = egui::Context::default();
    let mut cache = click_cache();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let at = |point: (f32, f32)| {
        egui::pos2(
            layout.canvas.x + point.0 * scale,
            layout.canvas.y + point.1 * scale,
        )
    };
    let press = egui::Event::PointerButton {
        pos: at(from),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    };
    let frames = [
        vec![egui::Event::PointerMoved(at(from))],
        vec![press],
        vec![egui::Event::PointerMoved(at(to))],
        vec![],
    ];
    let mut painted = Vec::new();
    for (index, events) in frames.into_iter().enumerate() {
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(index as f64 * 0.05),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            draw_sector_windows(
                ctx,
                world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            );
        });
        painted = crate::fleet_window::tests::PAINTED.with(|painted| painted.take());
    }
    (state.drag_image(), painted)
}

#[test]
fn a_held_icon_drag_paints_the_icons_art_at_the_pointers_offset() {
    // FUN_004593e0 case 0x200: the first move with the button held
    // starts the drag image (FUN_0060dbb0 copies the window-sized
    // bitmap FUN_0045b1b0 blitted the icon's first bitmap, +0x20, into
    // at its rect; FUN_0060dcb0 anchors it at the press; FUN_0060dce0
    // and FUN_0060df00 draw it at the pointer). A0
    // 2026-11-13-icon-drag-move/21: the fleet icon pressed at (266, 214)
    // and moved by about (-49, -55) shows 10775 there, no white
    // outline, while the origin keeps the pressed 10776. Chandrila's
    // fleet icon is drawn from (93, 76) on the canvas at (10, 20); its
    // mine lights the system icon beside it, which is not dragged.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    add_mine(&mut world, system);
    let (image, painted) = held_drag(&world, system, (115.0, 90.0), (66.0, 35.0), 1.0);
    let (art, rect) = image.expect("the held drag paints an image");
    assert_eq!(art, 10_775);
    assert_eq!(
        rect,
        egui::Rect::from_min_size(
            egui::pos2(103.0 - 49.0, 96.0 - 55.0),
            egui::vec2(28.0, 19.0)
        )
    );
    let origin = egui::pos2(103.0, 96.0);
    assert!(painted.contains(&(10_775, rect.min)), "{painted:?}");
    assert!(painted.contains(&(10_776, origin)), "{painted:?}");
    assert!(!painted.contains(&(10_775, origin)), "{painted:?}");
    // At twice the scale the offset is the pointer's, in screen pixels.
    let (image, _) = held_drag(&world, system, (115.0, 90.0), (66.0, 35.0), 2.0);
    let (_, rect) = image.expect("the held drag paints an image");
    assert_eq!(
        rect.min,
        egui::pos2(10.0 + 93.0 * 2.0 - 98.0, 20.0 + 76.0 * 2.0 - 110.0)
    );
    // A press that has not moved draws no image yet.
    let (still, _) = held_drag(&world, system, (115.0, 90.0), (115.0, 90.0), 1.0);
    assert_eq!(still, None);
}

#[test]
fn a_held_headquarters_drag_paints_the_mark_at_the_pointers_offset() {
    // FUN_004593e0 case 0x200 drags the headquarters overlay (id
    // 0x10000, FUN_0045bbb0) as it does a quadrant icon: its bitmap
    // 0x388 on the planet's rect, moved with the pointer.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let (_, base) = with_headquarters(&mut world, system);
    let (image, painted) = held_drag(&world, system, (100.0, 85.0), (150.0, 120.0), 1.0);
    without_headquarters_art(base);
    let (art, rect) = image.expect("the held drag paints the mark");
    assert_eq!(art, HEADQUARTERS_MARK);
    let origin = painted
        .iter()
        .find(|(id, at)| *id == HEADQUARTERS_MARK && *at != rect.min)
        .map(|&(_, at)| at)
        .expect("the mark stays on its planet");
    assert_eq!(rect.min, origin + egui::vec2(50.0, 35.0));
    assert!(
        painted.contains(&(HEADQUARTERS_MARK, rect.min)),
        "{painted:?}"
    );
}

#[test]
fn a_click_pressed_and_released_in_one_frame_leaves_no_drag_for_the_next_release() {
    // FUN_004593e0: case 0x201 takes the capture and keeps the press;
    // case 0x202 always ends the drag (ReleaseCapture, +0x148 = 0).
    // Both messages of a quick click can reach the port in one slow
    // frame (a double click on the fleet icon); the next press and
    // release elsewhere is a click of its own, no drop of the icon.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let icon = (115.0, 90.0);
    let elsewhere = (200.0, 250.0);
    let actions = steps_at(
        &world,
        system,
        [
            vec![Step::Move(icon)],
            vec![Step::Press(icon), Step::Release(icon)],
            vec![],
            vec![Step::Move(elsewhere)],
            vec![Step::Press(elsewhere)],
            vec![Step::Release(elsewhere)],
            vec![],
        ],
        egui::Modifiers::NONE,
    );
    assert!(drops(&actions).is_empty(), "{:?}", drops(&actions));
}

#[test]
fn a_drag_that_starts_off_every_icon_drops_nothing() {
    // FUN_004593e0 case 0x200 drags only an item the press chose
    // (+0x1a0); (200, 250) is empty window space.
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    assert!(drops(&drag_at(
        &world,
        system,
        (200.0, 250.0),
        (115.0, 90.0),
        egui::Modifiers::NONE
    ))
    .is_empty());
}

#[test]
fn a_hidden_icons_corner_selects_nothing() {
    // FUN_0045d140 removes an item with nothing to show, so a press on
    // its corner finds no overlay.
    let (world, system, _) = fixture_world();
    assert_eq!(
        menus(&clicks_at(&world, system, &[(RIGHT, (115.0, 90.0))])),
        [(None, (115, 90))]
    );
}

fn opened(actions: &[SectorWindowAction]) -> Vec<SectorWindowAction> {
    actions
        .iter()
        .copied()
        .filter(|action| !matches!(action, SectorWindowAction::SelectSystem(_)))
        .collect()
}

fn add_fleet(world: &mut GameWorld, system: SystemKey) {
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        location: system,
        is_alliance: true,
        ..Default::default()
    });
    world.systems[system].fleets.push(fleet);
}

#[test]
fn a_double_click_on_the_fleet_icon_opens_the_fleet_window() {
    // FUN_004593e0 case 0x203: the shown overlay under the point;
    // FUN_0045aac0 maps kind 0x10 to window type 4. Chandrila's planet
    // sits at (74, 79), so the icon spans (93, 78) to (121, 97).
    let (mut world, system, _) = fixture_world();
    add_fleet(&mut world, system);
    let actions = double_click_at(&world, system, (100.0, 85.0));

    assert_eq!(
        opened(&actions),
        [SectorWindowAction::OpenFleetWindow {
            system,
            logical_position: (100, 85),
        }]
    );
    // Past the planet's right edge, still on the icon.
    let actions = double_click_at(&world, system, (115.0, 90.0));
    assert!(matches!(
        opened(&actions)[..],
        [SectorWindowAction::OpenFleetWindow { .. }]
    ));
}

#[test]
fn the_fleet_window_opens_from_the_fleet_icons_point() {
    // FUN_0045c8e0 opens the Fleet window at the sector item's stored
    // point, its rect's top left: the icon at (93, 76) to (121, 95)
    // (A0 personnel-states/17: Pzob's window at its icon's left edge).
    let (world, system, _) = fixture_world();
    let mut state = SectorWindowState::default();
    assert_eq!(state.fleet_window_point(&world, layout(1.0), system), None);
    state.open_for_system(&world, system, CockpitFaction::Alliance);

    assert_eq!(
        state.fleet_window_point(&world, layout(1.0), system),
        Some((93, 76))
    );
    assert_eq!(
        state.fleet_window_point(&world, layout(2.0), system),
        Some((93, 76))
    );
}

#[test]
fn without_icons_a_double_click_on_the_planet_opens_nothing() {
    // FUN_0045d140 hides the overlay at a zero count, and
    // FUN_004593e0 case 0x203 hit-tests the overlays only
    // (FUN_0045cc10), so the planet answers nothing (A0 2026-11-09:
    // Adega, Flax, Akrit'tar; system-windows/04, 05 Yavin).
    let (world, system, _) = fixture_world();
    let actions = double_click_at(&world, system, (100.0, 85.0));

    assert!(opened(&actions).is_empty(), "{actions:?}");
}

#[test]
fn the_fleet_icons_rect_is_the_planets_top_right_quadrant() {
    // FUN_00459e30, flag 0x100000: (cx + 1, cy - 19) to (cx + 29, cy).
    let (world, system, _) = fixture_world();
    let layout = layout(1.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);

    assert_eq!(
        state.fleet_icon_screen_rect(&world, layout, system),
        Some(egui::Rect::from_min_size(
            egui::pos2(10.0 + 93.0, 20.0 + 76.0),
            egui::vec2(28.0, 19.0)
        ))
    );
    // Twice the size: the offsets and the size scale with the window.
    let doubled = CockpitLayout {
        scale: 2.0,
        ..layout
    };
    assert_eq!(
        state.fleet_icon_screen_rect(&world, doubled, system),
        Some(egui::Rect::from_min_size(
            egui::pos2(10.0 + 93.0 * 2.0, 20.0 + 76.0 * 2.0),
            egui::vec2(56.0, 38.0)
        ))
    );
}

#[test]
fn each_quadrant_icon_sits_at_its_corner_of_the_planet() {
    // FUN_00459e30:369-447: 28 by 19 each; the left ones end at cx, the
    // right ones start at cx + 1; the top ones end at cy, the bottom ones
    // start at cy + 1. Chandrila's planet sits at (74, 79), so cx = 92
    // and cy = 97.
    let (world, system, _) = fixture_world();
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    let corners = [
        (Quadrant::System, (64.0, 76.0)),
        (Quadrant::Defenses, (64.0, 96.0)),
        (Quadrant::Fleets, (93.0, 76.0)),
        (Quadrant::Missions, (93.0, 96.0)),
    ];
    for scale in [1.0, 2.0] {
        for (quadrant, (x, y)) in corners {
            assert_eq!(
                state.quadrant_screen_rect(&world, layout(scale), system, quadrant),
                Some(egui::Rect::from_min_size(
                    egui::pos2(10.0 + x * scale, 20.0 + y * scale),
                    egui::vec2(28.0 * scale, 19.0 * scale)
                )),
                "{quadrant:?} at scale {scale}"
            );
        }
    }
}

fn add_mine(world: &mut GameWorld, system: SystemKey) {
    let mine =
        world
            .production_facilities
            .insert(rebellion_core::world::ProductionFacilityInstance {
                class_dat_id: DatId::new(0x2c00_0001),
                side: rebellion_core::dat::Faction::Alliance,
                is_mine: true,
            });
    world.systems[system].production_facilities.push(mine);
}

#[test]
fn a_double_click_on_the_system_icon_opens_the_system_window() {
    // FUN_0045aac0 maps kind 4 to window type 9. (66, 80) lies on the
    // top-left icon, left of the planet's picture.
    let (mut world, system, _) = fixture_world();
    assert!(opened(&double_click_at(&world, system, (66.0, 80.0))).is_empty());

    add_mine(&mut world, system);
    assert_eq!(
        opened(&double_click_at(&world, system, (66.0, 80.0))),
        [SectorWindowAction::OpenSystemWindow {
            system,
            logical_position: (66, 80),
        }]
    );
}

#[test]
fn a_double_click_on_the_defenses_icon_opens_the_defenses_window() {
    // FUN_0045aac0 maps kind 8 to window type 10. (66, 110) lies on the
    // bottom-left icon, (64, 98) to (92, 117), left of the planet.
    let (mut world, system, _) = fixture_world();
    assert!(opened(&double_click_at(&world, system, (66.0, 110.0))).is_empty());

    let troop = world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(0x1000_0002),
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(troop);
    assert_eq!(
        opened(&double_click_at(&world, system, (66.0, 110.0))),
        [SectorWindowAction::OpenDefensesWindow {
            system,
            logical_position: (66, 110),
        }]
    );
}

#[test]
fn a_double_click_on_the_missions_icon_opens_the_missions_window() {
    // FUN_0045aac0 maps kind 0x40 to window type 11. (119, 110) lies on
    // the bottom-right icon, (93, 98) to (121, 117), right of the
    // planet.
    let (mut world, system, _) = fixture_world();
    let agent = world.characters.insert(rebellion_core::world::Character {
        dat_id: DatId::new(832),
        name: "Agent".into(),
        is_alliance: true,
        current_system: Some(system),
        recruited: true,
        ..Default::default()
    });
    let mut missions = rebellion_core::missions::MissionState::new();
    assert!(opened(&double_click_with_missions(
        &world,
        system,
        &missions,
        (119.0, 110.0)
    ))
    .is_empty());

    missions.dispatch(rebellion_core::missions::MissionRequest::single(
        rebellion_core::missions::MissionKind::Diplomacy,
        rebellion_core::missions::MissionFaction::Alliance,
        agent,
        system,
        None,
        0,
    ));
    assert_eq!(
        opened(&double_click_with_missions(
            &world,
            system,
            &missions,
            (119.0, 110.0)
        )),
        [SectorWindowAction::OpenMissionsWindow {
            system,
            logical_position: (119, 110),
        }]
    );
}

/// Draw `system`'s sector window at `scale` for two frames (an Area
/// paints nothing on its first) and return each quadrant bitmap painted
/// and where.
fn painted_quadrant_icons(
    world: &GameWorld,
    system: SystemKey,
    missions: &rebellion_core::missions::MissionState,
    scale: f32,
) -> Vec<(u32, egui::Pos2)> {
    let layout = layout(scale);
    let mut state = SectorWindowState::default();
    state.open_for_system(world, system, CockpitFaction::Alliance);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    for index in 0..2 {
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(f64::from(index) * 0.05),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            let _ = draw_sector_windows(
                ctx,
                world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                missions,
            );
        });
    }
    crate::fleet_window::tests::PAINTED
        .with(|painted| painted.take())
        .into_iter()
        .filter(|(id, _)| (10771..=10790).contains(id))
        .collect()
}

// FUN_00459e30 writes the sector's name in yellow (0x200f0f0) whatever
// its systems' sides (A0 Moddell and Sesswenna); FUN_004593e0 writes
// each planet's name in its item's colour (FUN_0045bbb0).
#[test]
fn the_title_is_yellow_and_each_name_takes_its_items_colour() {
    let (mut world, system, _) = fixture_world();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    let layout = layout(2.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    let fog = FogState::new(Faction::Alliance);
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let missions = rebellion_core::missions::MissionState::new();
    let mut texts = Vec::new();
    for index in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(f64::from(index) * 0.05),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            let _ = draw_sector_windows(
                ctx,
                &world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            );
        });
        texts = output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) => Some((
                    text.galley.text().to_owned(),
                    text.galley.job.sections[0].format.color,
                    text.pos.y
                        + text
                            .galley
                            .rows
                            .first()
                            .and_then(|row| row.glyphs.first())
                            .map_or(0.0, |glyph| glyph.pos.y),
                )),
                _ => None,
            })
            .fold(
                Vec::<(String, egui::Color32, f32)>::new(),
                |mut lines, (text, colour, baseline)| {
                    // paint_gdi_text paints a line a character at a
                    // time; join each run of one colour and baseline.
                    match lines.last_mut() {
                        Some(line) if line.1 == colour && line.2 == baseline => {
                            line.0.push_str(&text);
                        }
                        _ => lines.push((text, colour, baseline)),
                    }
                    lines
                },
            );
    }
    let colour = |name: &str| {
        texts
            .iter()
            .find(|(text, _, _)| text == name)
            .map(|(_, colour, _)| *colour)
    };
    let sector = &world.sectors[world.systems[system].sector].name;
    // 0x200f0f0 in the 8-bit palette: A0 draws "Corellian" in
    // (255, 255, 0) (personnel-states/24).
    assert_eq!(colour(sector), Some(egui::Color32::from_rgb(255, 255, 0)), "{texts:?}");
    // DT_VCENTER in the title's 20 rows from y 2 with font 4 (tmHeight
    // 16, tmAscent 13): the baseline 2 + 2 + 13 down (A0
    // sector-windows/04-left's title matches at that line).
    let window = window_screen_rect(CockpitFaction::Alliance, state.windows[0].column, layout);
    let baseline = texts
        .iter()
        .find(|(text, _, _)| text == sector)
        .map(|(_, _, baseline)| *baseline);
    assert_eq!(baseline, Some(window.min.y + 17.0 * 2.0));
    assert_eq!(
        colour(&world.systems[system].name),
        Some(egui::Color32::from_rgb(0, 255, 0)),
        "{texts:?}"
    );
}

/// The bitmaps one frame of `state`'s windows paints, with their places.
fn painted(
    world: &GameWorld,
    state: &mut SectorWindowState,
    layout: CockpitLayout,
    base: Option<&std::path::Path>,
) -> Vec<(u32, egui::Pos2)> {
    let fog = FogState::new(Faction::Alliance);
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    if let Some(base) = base {
        cache.set_base_path(base);
    }
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let missions = rebellion_core::missions::MissionState::new();
    for index in 0..2 {
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(f64::from(index) * 0.05),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            let _ = draw_sector_windows(
                ctx,
                world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            );
        });
    }
    crate::fleet_window::tests::PAINTED.with(|painted| painted.take())
}

// FUN_0045bbb0 blits the system's galaxy-view marker (FUN_0042d860) at
// the bottom left of the planet's bitmap, which is the picture's 37 rows
// plus 4; a destroyed system's item skips it. The staged marker here is
// 2 by 1.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn quadrant_items_stand_where_a0_shows_them_around_their_planet() {
    // A0 sector-a: Vjun's planet at (230, 133); its System, Defenses and
    // Fleets items at (220, 132), (220, 152), (249, 132), 28 by 19.
    let planet = egui::Rect::from_min_size(egui::pos2(230.0, 133.0), egui::vec2(37.0, 37.0));
    for (quadrant, corner) in [
        (Quadrant::System, (220.0, 132.0)),
        (Quadrant::Defenses, (220.0, 152.0)),
        (Quadrant::Fleets, (249.0, 132.0)),
    ] {
        let rect = quadrant_rect(planet, 1.0, quadrant);
        assert_eq!((rect.min.x, rect.min.y), corner, "{quadrant:?}");
        assert_eq!((rect.width(), rect.height()), (28.0, 19.0));
    }
}

#[test]
fn a_planet_shows_its_galaxy_marker_at_its_bitmaps_bottom_left() {
    let base = std::env::temp_dir().join(format!(
        "open-rebellion-sector-marker-{}",
        std::process::id()
    ));
    let folder = base.join("strategy-dll").join("BMP");
    std::fs::create_dir_all(&folder).unwrap();
    let marker = crate::bmp_cache::resources::strategy::GID_UNEXPLORED;
    std::fs::write(
        folder.join(format!("{marker}.bmp")),
        crate::bmp_cache::tests::indexed_bmp(&[[255, 255, 255], [0, 0, 0]]),
    )
    .unwrap();
    let (mut world, system, _) = fixture_world();
    let layout = layout(2.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    state.set_gid_markers(&std::collections::HashMap::from([(system, marker)]));
    let planet = state.planet_screen_rect(&world, layout, system).unwrap();
    let at = planet.min + egui::vec2(0.0, (37.0 + 4.0 - 1.0) * 2.0);
    let shown = painted(&world, &mut state, layout, Some(&base));
    world.systems[system].is_destroyed = true;
    let destroyed = painted(&world, &mut state, layout, Some(&base));
    // With the display off the galaxy view draws no marker.
    state.set_gid_markers(&std::collections::HashMap::new());
    world.systems[system].is_destroyed = false;
    let off = painted(&world, &mut state, layout, Some(&base));
    std::fs::remove_dir_all(base).unwrap();
    assert!(shown.contains(&(marker, at)), "{shown:?}");
    assert!(!destroyed.iter().any(|&(id, _)| id == marker));
    assert!(!off.iter().any(|&(id, _)| id == marker));
}

// The window's copy of the galaxy view holds the markers the view drew
// under it, shaded (FUN_004593e0, FUN_005fe050).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_window_shows_the_views_markers_under_it_shaded() {
    let base = std::env::temp_dir().join(format!(
        "open-rebellion-sector-shaded-marker-{}",
        std::process::id()
    ));
    let folder = base.join("strategy-dll").join("BMP");
    std::fs::create_dir_all(&folder).unwrap();
    let marker = crate::bmp_cache::resources::strategy::GID_UNEXPLORED;
    std::fs::write(
        folder.join(format!("{marker}.bmp")),
        crate::bmp_cache::tests::indexed_bmp(&[[255, 255, 255], [0, 0, 0]]),
    )
    .unwrap();
    let (world, system, _) = fixture_world();
    let layout = layout(2.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    let window = window_screen_rect(CockpitFaction::Alliance, state.windows[0].column, layout);
    let inside = window.center();
    let outside = egui::pos2(window.max.x + 100.0, inside.y);
    state.set_drawn_gid_markers(&[
        (marker, [inside.x, inside.y]),
        (marker, [outside.x, outside.y]),
    ]);
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    cache.set_base_path(&base);
    let texture = cache
        .get_shaded(&ctx, DllSource::Strategy, marker)
        .map(egui::TextureHandle::id)
        .unwrap();
    let fog = FogState::new(Faction::Alliance);
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let missions = rebellion_core::missions::MissionState::new();
    let mut rects = Vec::new();
    for index in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(f64::from(index) * 0.05),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            let _ = draw_sector_windows(
                ctx,
                &world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            );
        });
        rects = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == texture => Some(
                    mesh.vertices
                        .iter()
                        .fold(egui::Rect::NOTHING, |rect, vertex| {
                            rect.union(egui::Rect::from_min_max(vertex.pos, vertex.pos))
                        }),
                ),
                _ => None,
            })
            .collect();
    }
    std::fs::remove_dir_all(base).unwrap();
    // The 2 by 1 marker, doubled, hung from its 17 by 17 cell's top left
    // as the view drew it (FUN_0042cde0); the one outside the window is
    // not drawn.
    assert_eq!(
        rects,
        [egui::Rect::from_min_size(
            inside - egui::vec2(17.0, 17.0),
            egui::vec2(4.0, 2.0)
        )]
    );
}

// The galaxy view's starfield is 903 under an active GID mode and 902
// with the display off (draw_galaxy_backdrop).
#[test]
fn the_windows_shade_the_starfield_the_galaxy_view_shows() {
    let mut state = SectorWindowState::default();
    assert_eq!(state.backdrop, 903);
    state.set_backdrop(crate::cockpit::GidMode::DisplayOff);
    assert_eq!(state.backdrop, 902);
    state.set_backdrop(crate::cockpit::GidMode::PopularSupport);
    assert_eq!(state.backdrop, 903);
}

// FUN_004593e0 copies the galaxy view's picture from under the window:
// the shaded starfield's texture coordinates are the window's place on
// the starfield.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_window_shows_the_shaded_starfield_from_under_itself() {
    let base = std::env::temp_dir().join(format!(
        "open-rebellion-sector-shade-{}",
        std::process::id()
    ));
    let folder = base.join("strategy-dll").join("BMP");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("903.bmp"),
        crate::bmp_cache::tests::indexed_bmp(&[[36, 37, 71], [59, 59, 59]]),
    )
    .unwrap();
    let (world, system, _) = fixture_world();
    let layout = layout(2.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    let fog = FogState::new(Faction::Alliance);
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    cache.set_base_path(&base);
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let missions = rebellion_core::missions::MissionState::new();
    let texture = cache
        .get_shaded(&ctx, DllSource::Strategy, 903)
        .map(egui::TextureHandle::id)
        .unwrap();
    let mut uv = None;
    for index in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(f64::from(index) * 0.05),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            let _ = draw_sector_windows(
                ctx,
                &world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            );
        });
        uv = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == texture => {
                    let xs = mesh.vertices.iter().map(|vertex| vertex.uv);
                    let min = xs
                        .clone()
                        .fold(egui::pos2(f32::MAX, f32::MAX), egui::Pos2::min);
                    let max = xs.fold(egui::pos2(f32::MIN, f32::MIN), egui::Pos2::max);
                    Some((min, max))
                }
                _ => None,
            });
    }
    std::fs::remove_dir_all(base).unwrap();
    // The 2 by 1 starfield at the Alliance's content origin (21, 25),
    // doubled; the window at its column's logical place, doubled.
    let window = window_screen_rect(CockpitFaction::Alliance, state.windows[0].column, layout);
    let origin = egui::pos2(10.0 + 21.0 * 2.0, 20.0 + 25.0 * 2.0);
    let extent = egui::vec2(2.0 * 2.0, 1.0 * 2.0);
    let (min, max) = uv.unwrap();
    let expected_min = ((window.min - origin) / extent).to_pos2();
    let expected_max = ((window.max - origin) / extent).to_pos2();
    assert!(
        (min - expected_min).length() < 1e-3,
        "{min:?} {expected_min:?}"
    );
    assert!(
        (max - expected_max).length() < 1e-3,
        "{max:?} {expected_max:?}"
    );
}

// FUN_004593e0's copy of the galaxy view holds the caption the view drew
// (FUN_00425d00), shaded: A0 sector-windows/02-second shows "Popular
// Support" (255, 0, 0) as (91, 67, 51) under both windows, the
// starfield palette's entry FUN_005fddd0's table maps red to.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_window_shows_the_views_caption_under_it_shaded() {
    let base = std::env::temp_dir().join(format!(
        "open-rebellion-sector-shaded-caption-{}",
        std::process::id()
    ));
    let folder = base.join("strategy-dll").join("BMP");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("903.bmp"),
        crate::bmp_cache::tests::indexed_bmp(&[[36, 37, 71], [255, 0, 0], [91, 67, 51]]),
    )
    .unwrap();
    let (world, system, _) = fixture_world();
    let layout = layout(2.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    let window = window_screen_rect(CockpitFaction::Alliance, state.windows[0].column, layout);
    state.set_gid_caption(Some(crate::GidCaption {
        text: "Popular Support".into(),
        rect: egui::Rect::from_min_size(window.min, egui::vec2(window.width(), 60.0)),
        scale: 2.0,
        color: egui::Color32::from_rgb(255, 0, 0),
    }));
    let fog = FogState::new(Faction::Alliance);
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    cache.set_base_path(&base);
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let missions = rebellion_core::missions::MissionState::new();
    let mut caption = String::new();
    for index in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1040.0),
            )),
            time: Some(f64::from(index) * 0.05),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            let _ = draw_sector_windows(
                ctx,
                &world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                &mut state,
                CockpitFaction::Alliance,
                layout,
                &mut cache,
                &uprisings,
                &missions,
            );
        });
        caption = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text)
                    if text.galley.job.sections[0].format.color
                        == egui::Color32::from_rgb(91, 67, 51) =>
                {
                    Some(text.galley.text().to_owned())
                }
                _ => None,
            })
            .collect();
    }
    std::fs::remove_dir_all(base).unwrap();
    assert_eq!(caption.replace(' ', ""), "PopularSupport");
}

#[test]
fn a_planet_near_the_bottom_paints_nothing_outside_its_sector_window() {
    // The original sector window is a child window (FUN_00459e30 places
    // its items in it), so nothing it holds paints past its frame.
    // A system 88 DAT units below its sector's origin sits at y 326; its
    // label and status tracks reach past the window's 360 rows.
    let (mut world, system, _) = fixture_world();
    let sector = world.systems[system].sector;
    world.systems[system].y = world.sectors[sector].y + 88;
    let layout = layout(2.0);
    let mut state = SectorWindowState::default();
    state.open_for_system(&world, system, CockpitFaction::Alliance);
    let window = window_screen_rect(CockpitFaction::Alliance, state.windows[0].column, layout);
    let mut fog = FogState::new(Faction::Alliance);
    fog.reveal(system);
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let missions = rebellion_core::missions::MissionState::new();
    // egui hides a new area on its first frame while it sizes it.
    let mut frame = || {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1040.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let _ = draw_sector_windows(
                    ctx,
                    &world,
                    &rebellion_core::movement::MovementState::default(),
                    crate::quadrant_icons::Travellers::default(),
                    &fog,
                    &mut state,
                    CockpitFaction::Alliance,
                    layout,
                    &mut cache,
                    &uprisings,
                    &missions,
                );
            },
        )
    };
    let _ = frame();
    let output = frame();
    let visible: Vec<egui::Rect> = output
        .shapes
        .iter()
        .map(|clipped| {
            clipped
                .shape
                .visual_bounding_rect()
                .intersect(clipped.clip_rect)
        })
        .filter(egui::Rect::is_positive)
        .collect();
    assert!(visible.iter().any(|rect| rect.max.y > window.max.y - 60.0));
    for rect in visible {
        assert!(
            window.expand(0.5).contains_rect(rect),
            "{rect:?} outside {window:?}"
        );
    }
}

#[test]
fn the_sector_window_paints_each_shown_quadrant_icon_at_its_corner() {
    // FUN_0045d140 paints FUN_0045ca80's first bitmap for each enabled
    // item: a mine (kind 4), a regiment (kind 8), a fleet (kind 0x10)
    // and an agent on a mission (kind 0x40), all Alliance (side 1).
    let (mut world, system, _) = fixture_world();
    let empty = rebellion_core::missions::MissionState::new();
    assert!(painted_quadrant_icons(&world, system, &empty, 2.0).is_empty());

    add_mine(&mut world, system);
    add_fleet(&mut world, system);
    let regiment = world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(regiment);
    let agent = world.characters.insert(rebellion_core::world::Character {
        dat_id: DatId::new(832),
        name: "Agent".into(),
        is_alliance: true,
        current_system: Some(system),
        recruited: true,
        ..Default::default()
    });
    let mut missions = rebellion_core::missions::MissionState::new();
    missions.dispatch(rebellion_core::missions::MissionRequest::single(
        rebellion_core::missions::MissionKind::Diplomacy,
        rebellion_core::missions::MissionFaction::Alliance,
        agent,
        system,
        None,
        0,
    ));

    let at = |x: f32, y: f32| egui::pos2(10.0 + x * 2.0, 20.0 + y * 2.0);
    assert_eq!(
        painted_quadrant_icons(&world, system, &missions, 2.0),
        [
            (10771, at(64.0, 76.0)),
            (10773, at(64.0, 96.0)),
            (10775, at(93.0, 76.0)),
            (10777, at(93.0, 96.0)),
        ]
    );
}

/// `system`'s sector window, played as `owner`, with a mine and a
/// fleet of `owner`'s at the system so its system and fleet icons
/// show.
fn owned_icons_world(owner: Faction) -> (GameWorld, SystemKey) {
    let (mut world, system, _) = fixture_world();
    world.systems[system].control = ControlKind::Controlled(owner);
    add_mine(&mut world, system);
    add_fleet(&mut world, system);
    let alliance = owner == Faction::Alliance;
    for fleet in world.systems[system].fleets.clone() {
        world.fleets[fleet].is_alliance = alliance;
    }
    for mine in world.systems[system].production_facilities.clone() {
        world.production_facilities[mine].side = owner;
    }
    (world, system)
}

fn cockpit(owner: Faction) -> CockpitFaction {
    if owner == Faction::Alliance {
        CockpitFaction::Alliance
    } else {
        CockpitFaction::Empire
    }
}

/// Left-click each logical point in turn in `state`'s windows and
/// return the quadrant bitmaps the last frame painted.
fn icons_after_clicks(
    world: &GameWorld,
    owner: Faction,
    state: &mut SectorWindowState,
    clicks: &[(i16, i16)],
) -> Vec<u32> {
    painted_after_clicks(world, owner, state, clicks)
        .into_iter()
        .filter(|id| (10771..=10790).contains(id))
        .collect()
}

/// As [`icons_after_clicks`], every bitmap the last frame painted, in
/// paint order.
fn painted_after_clicks(
    world: &GameWorld,
    owner: Faction,
    state: &mut SectorWindowState,
    clicks: &[(i16, i16)],
) -> Vec<u32> {
    let layout = layout(1.0);
    let mut fog = FogState::new(owner);
    for system in world.systems.keys() {
        fog.reveal(system);
    }
    let missions = rebellion_core::missions::MissionState::new();
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    let uprisings = rebellion_core::uprising::UprisingState::default();
    let mut frames = vec![vec![], vec![]];
    for &(x, y) in clicks {
        let pos = egui::pos2(
            layout.canvas.x + f32::from(x),
            layout.canvas.y + f32::from(y),
        );
        let event = |pressed| egui::Event::PointerButton {
            pos,
            button: LEFT,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        frames.push(vec![egui::Event::PointerMoved(pos)]);
        frames.push(vec![event(true)]);
        frames.push(vec![event(false)]);
    }
    frames.push(vec![]);
    for (index, events) in frames.into_iter().enumerate() {
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            // Far enough apart that no two clicks make a double click.
            time: Some(index as f64 * 0.5),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            let _ = draw_sector_windows(
                ctx,
                world,
                &rebellion_core::movement::MovementState::default(),
                crate::quadrant_icons::Travellers::default(),
                &fog,
                state,
                cockpit(owner),
                layout,
                &mut cache,
                &uprisings,
                &missions,
            );
        });
    }
    crate::fleet_window::tests::PAINTED
        .with(|painted| painted.take())
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

/// A system in `system`'s sector at `(x, y)` with DAT id `dat_id`, held
/// by `control`, with nothing in it.
fn add_neighbour(
    world: &mut GameWorld,
    system: SystemKey,
    dat_id: u32,
    (x, y): (u16, u16),
    control: ControlKind,
) -> SystemKey {
    let mut neighbour = world.systems[system].clone();
    neighbour.dat_id = DatId::new(dat_id);
    neighbour.name = format!("Neighbour {dat_id}");
    (neighbour.x, neighbour.y) = (x, y);
    neighbour.control = control;
    neighbour.fleets.clear();
    neighbour.production_facilities.clear();
    let sector = neighbour.sector;
    let key = world.systems.insert(neighbour);
    world.sectors[sector].systems.push(key);
    key
}

#[test]
fn the_selected_icon_paints_its_second_bitmap_on_either_side() {
    // FUN_004593e0 WM_PAINT draws a shown overlay's `+0x24` bitmap
    // while `+0x3c` bit 0 is set, else its `+0x20`; FUN_0045b1b0 sets
    // the bit for the pressed overlay after FUN_0045afc0 clears the
    // rest. FUN_0045ca80 pairs kind 4 with 10771/10772 (side 1) and
    // 10779/10780 (side 2), kind 0x10 with 10775/10776 and 10783/10784.
    for (owner, (system_art, system_pressed), (fleet_art, fleet_pressed)) in [
        (Faction::Alliance, (10771, 10772), (10775, 10776)),
        (Faction::Empire, (10779, 10780), (10783, 10784)),
    ] {
        let (world, system) = owned_icons_world(owner);
        let mut state = SectorWindowState::default();
        state.open_for_system(&world, system, cockpit(owner));
        let layout = layout(1.0);
        let point = |quadrant| {
            state
                .quadrant_window_point(&world, layout, system, quadrant)
                .unwrap()
        };
        let (system_icon, fleet_icon) = (point(Quadrant::System), point(Quadrant::Fleets));

        let mut fresh = SectorWindowState::default();
        fresh.open_for_system(&world, system, cockpit(owner));
        assert_eq!(
            icons_after_clicks(&world, owner, &mut fresh, &[]),
            [system_art, fleet_art],
            "{owner:?}: nothing selected"
        );
        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[fleet_icon]),
            [system_art, fleet_pressed],
            "{owner:?}: the fleet icon"
        );
        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[system_icon]),
            [system_pressed, fleet_art],
            "{owner:?}: the system icon replaces it"
        );
        // A press that finds no overlay returns before FUN_0045afc0,
        // whether it lands on empty window space or on the planet.
        let planet = state
            .planet_screen_rect(&world, layout, system)
            .map(|rect| screen_to_logical(layout, rect.center()))
            .unwrap();
        for (press, what) in [((200, 250), "empty space"), (planet, "the planet")] {
            assert_eq!(
                icons_after_clicks(&world, owner, &mut state, &[press]),
                [system_pressed, fleet_art],
                "{owner:?}: a press on {what}"
            );
        }
    }
}

#[test]
fn selecting_one_systems_icon_leaves_a_neighbours_matching_icon_unselected() {
    // FUN_0045b1b0 sets `+0x3c` bit 0 on the pressed overlay alone, and
    // WM_PAINT draws the `+0x164` planet list before the `+0x174`
    // overlay list. FUN_0045ca80 pairs kind 0x10 with 10775/10776
    // (side 1) and 10783/10784 (side 2).
    for (owner, system_art, (fleet_art, fleet_pressed)) in [
        (Faction::Alliance, 10771, (10775, 10776)),
        (Faction::Empire, 10779, (10783, 10784)),
    ] {
        let (mut world, system) = owned_icons_world(owner);
        let neighbour = add_neighbour(
            &mut world,
            system,
            269,
            (380, 300),
            ControlKind::Controlled(owner),
        );
        add_fleet(&mut world, neighbour);
        for fleet in world.systems[neighbour].fleets.clone() {
            world.fleets[fleet].is_alliance = owner == Faction::Alliance;
        }
        // Last in the sector, with no icon to lapse the selection.
        let quiet = add_neighbour(&mut world, system, 271, (330, 330), ControlKind::Uncontrolled);
        let mut state = SectorWindowState::default();
        state.open_for_system(&world, system, cockpit(owner));
        let layout = layout(1.0);
        let planets: Vec<egui::Rect> = [system, neighbour, quiet]
            .map(|key| state.planet_screen_rect(&world, layout, key).unwrap())
            .to_vec();
        assert!(
            !planets[0].intersects(planets[1])
                && !planets[0].intersects(planets[2])
                && !planets[1].intersects(planets[2]),
            "the planets are apart: {planets:?}"
        );
        let fleet_icon = state
            .quadrant_window_point(&world, layout, system, Quadrant::Fleets)
            .unwrap();

        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[fleet_icon]),
            [system_art, fleet_pressed, fleet_art],
            "{owner:?}: only the pressed system's fleet icon"
        );
        let painted = painted_after_clicks(&world, owner, &mut state, &[]);
        assert_eq!(
            painted
                .iter()
                .filter(|id| (10771..=10790).contains(*id))
                .copied()
                .collect::<Vec<_>>(),
            [system_art, fleet_pressed, fleet_art],
            "{owner:?}: the selection holds"
        );
        let position = |id: u32| painted.iter().position(|painted| *painted == id).unwrap();
        let last_planet = [system, neighbour, quiet]
            .map(|key| position(planet_resource_id(world.systems[key].dat_id)))
            .into_iter()
            .max()
            .unwrap();
        assert!(
            last_planet < position(system_art),
            "{owner:?}: every planet before every icon: {painted:?}"
        );
    }
}

#[test]
fn an_icon_that_hides_or_changes_art_is_no_longer_selected() {
    // FUN_0045b770 refreshes an icon on its system's change notice, and
    // FUN_0045d140 first deselects it (FUN_0045afc0). port: with no
    // change notices, the port deselects an icon that hides or changes
    // art, or whose system an object reaches or leaves. A side change
    // repaints it from the new side's pair; FUN_0045ca80 pairs each
    // first bitmap with the next id.
    for (owner, other, (system_art, other_art), (fleet_art, fleet_pressed)) in [
        (
            Faction::Alliance,
            Faction::Empire,
            (10771, 10779),
            (10775, 10776),
        ),
        (
            Faction::Empire,
            Faction::Alliance,
            (10779, 10771),
            (10783, 10784),
        ),
    ] {
        let (mut world, system) = owned_icons_world(owner);
        let mut state = SectorWindowState::default();
        state.open_for_system(&world, system, cockpit(owner));
        let layout = layout(1.0);
        let system_icon = state
            .quadrant_window_point(&world, layout, system, Quadrant::System)
            .unwrap();
        let fleet_icon = state
            .quadrant_window_point(&world, layout, system, Quadrant::Fleets)
            .unwrap();
        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[system_icon]),
            [system_art + 1, fleet_art],
            "{owner:?}: the system icon is selected"
        );
        world.systems[system].control = ControlKind::Controlled(other);
        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[])[0],
            other_art,
            "{owner:?}: the side change"
        );
        world.systems[system].control = ControlKind::Controlled(owner);
        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[])[0],
            system_art,
            "{owner:?}: back to its own side"
        );

        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[fleet_icon]),
            [system_art, fleet_pressed],
            "{owner:?}: the fleet icon is selected"
        );
        add_fleet(&mut world, system);
        for fleet in world.systems[system].fleets.clone() {
            world.fleets[fleet].is_alliance = owner == Faction::Alliance;
        }
        // The arrival is a change notice: the art stays, the selection
        // lapses (FUN_0045d140; A0 2026-11-13-defenses-missions-drop/05
        // for a team that leaves).
        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[]),
            [system_art, fleet_art],
            "{owner:?}: a second fleet leaves the art as it was, unselected"
        );
        let fleets = std::mem::take(&mut world.systems[system].fleets);
        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[]),
            [system_art],
            "{owner:?}: the fleet icon hides"
        );
        world.systems[system].fleets = fleets;
        assert_eq!(
            icons_after_clicks(&world, owner, &mut state, &[]),
            [system_art, fleet_art],
            "{owner:?}: the fleet icon hid and returned"
        );
    }
}

#[test]
fn a_selected_icon_lapses_when_its_systems_team_leaves_though_its_art_stays() {
    // FUN_0045b770 refreshes the system's icons on each change notice
    // for the system or an object at it, and FUN_0045d140 first
    // deselects (FUN_0045afc0), whether or not the art changes. A0
    // 2026-11-13-defenses-missions-drop/02, 05: Jerijador's defenses
    // icon drawn selected (10774) until its team (characters,
    // regiments, squadrons) is dropped on Yavin; the two shields keep
    // the icon, drawn unselected (10773). FUN_0045ca80 pairs kind 8
    // with 10773/10774 (side 1) and 10781/10782 (side 2).
    for (owner, (defenses_art, defenses_pressed)) in [
        (Faction::Alliance, (10773, 10774)),
        (Faction::Empire, (10781, 10782)),
    ] {
        let (mut world, system) = owned_icons_world(owner);
        let alliance = owner == Faction::Alliance;
        let shield =
            world
                .defense_facilities
                .insert(rebellion_core::world::DefenseFacilityInstance {
                    class_dat_id: DatId::new(0x2200_0001),
                    side: owner,
                });
        world.systems[system].defense_facilities.push(shield);
        let regiment = world.troops.insert(rebellion_core::world::TroopUnit {
            class_dat_id: DatId::new(0x1000_0001),
            is_alliance: alliance,
            regiment_strength: 100,
        });
        world.systems[system].ground_units.push(regiment);
        let mut state = SectorWindowState::default();
        state.open_for_system(&world, system, cockpit(owner));
        let defenses_icon = state
            .quadrant_window_point(&world, layout(1.0), system, Quadrant::Defenses)
            .unwrap();
        let defenses = |icons: Vec<u32>| {
            icons
                .into_iter()
                .filter(|id| [defenses_art, defenses_pressed].contains(id))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            defenses(icons_after_clicks(
                &world,
                owner,
                &mut state,
                &[defenses_icon]
            )),
            [defenses_pressed],
            "{owner:?}: the defenses icon is selected"
        );
        assert_eq!(
            defenses(icons_after_clicks(&world, owner, &mut state, &[])),
            [defenses_pressed],
            "{owner:?}: nothing changed, the selection holds"
        );
        // A notice for another system leaves this one's icons alone.
        let elsewhere = world
            .systems
            .keys()
            .find(|&key| key != system)
            .expect("the fixture holds a second system");
        world.characters.insert(rebellion_core::world::Character {
            name: "Elsewhere".into(),
            is_alliance: alliance,
            is_empire: !alliance,
            current_system: Some(elsewhere),
            ..Default::default()
        });
        assert_eq!(
            defenses(icons_after_clicks(&world, owner, &mut state, &[])),
            [defenses_pressed],
            "{owner:?}: a character at another system, the selection holds"
        );
        world.systems[system].ground_units.clear();
        assert_eq!(
            defenses(icons_after_clicks(&world, owner, &mut state, &[])),
            [defenses_art],
            "{owner:?}: the regiment left; the shield keeps the icon, unselected"
        );
    }
}

/// The interface command a script line `label` names for `world`.
fn palette_command(world: &GameWorld, label: &str) -> InterfaceCommand {
    let mut palette = crate::CommandPaletteState::new();
    palette.refresh_script(world);
    match palette.command_named(label).map(|item| item.action.clone()) {
        Some(crate::PaletteAction::Interface(command)) => command,
        other => panic!("{label}: {other:?}"),
    }
}

fn run_command(
    world: &GameWorld,
    owner: Faction,
    state: &mut SectorWindowState,
    command: InterfaceCommand,
) -> Result<Vec<SectorWindowAction>, &'static str> {
    let mut fog = FogState::new(owner);
    for system in world.systems.keys() {
        fog.reveal(system);
    }
    state.command_actions(
        world,
        &fog,
        &rebellion_core::missions::MissionState::new(),
        &rebellion_core::movement::MovementState::default(),
        crate::quadrant_icons::Travellers::default(),
        cockpit(owner),
        layout(1.0),
        command,
    )
}

#[test]
fn the_palette_names_each_systems_windows_and_menus_once_ignoring_case() {
    let (world, system, other) = fixture_world();
    let mut palette = crate::CommandPaletteState::new();
    palette.refresh_interface(&world);
    palette.refresh_interface(&world);
    let named = |label: &str| {
        palette
            .command_named(label)
            .map(|item| match item.action {
                crate::PaletteAction::Interface(command) => Some(command),
                crate::PaletteAction::Panel(_) => None,
            })
            .flatten()
    };
    assert_eq!(
        named("  open fleet WINDOW: chandrila "),
        Some(InterfaceCommand::OpenQuadrantWindow {
            system,
            quadrant: Quadrant::Fleets
        })
    );
    assert_eq!(
        named("Open defenses icon menu: Chandrila"),
        Some(InterfaceCommand::OpenIconMenu {
            system,
            quadrant: Quadrant::Defenses
        })
    );
    let other_name = world.systems[other].name.clone();
    assert_eq!(
        named(&format!("Open sector window: {other_name}")),
        Some(InterfaceCommand::OpenSector(other))
    );
    assert_eq!(named("Open Fleet window: Coruscant"), None);
    // The palette draws in the galaxy, where a start cannot run.
    assert_eq!(named("Start game: Empire"), None);
    // A second refresh replaces the interface commands, not adds them.
    let mut labels: Vec<&str> = palette
        .commands()
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    let total = labels.len();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), total);
    // A sector window, four windows and four menus a system.
    let interface = |palette: &crate::CommandPaletteState| {
        palette
            .commands()
            .iter()
            .filter(|item| matches!(item.action, crate::PaletteAction::Interface(_)))
            .count()
    };
    assert_eq!(interface(&palette), 9 * world.systems.len());

    // A script adds the two starts and reaches the simulation commands.
    palette.refresh_script(&world);
    assert_eq!(interface(&palette), 2 + 9 * world.systems.len());
    assert!(matches!(
        palette.command_named("start game: empire").map(|item| &item.action),
        Some(crate::PaletteAction::Interface(InterfaceCommand::StartGame(
            rebellion_core::missions::MissionFaction::Empire
        )))
    ));
    assert!(matches!(
        palette.command_named("Game Speed: Pause").map(|item| &item.action),
        Some(crate::PaletteAction::Panel(crate::PanelAction::SetGameSpeed(
            rebellion_core::tick::GameSpeed::Paused
        )))
    ));
}

#[test]
fn a_sector_command_opens_the_systems_sector_window() {
    let (world, system, other) = fixture_world();
    for owner in [Faction::Alliance, Faction::Empire] {
        let mut state = SectorWindowState::default();
        let name = world.systems[other].name.clone();
        assert_eq!(
            run_command(
                &world,
                owner,
                &mut state,
                palette_command(&world, &format!("Open sector window: {name}"))
            ),
            Ok(Vec::new()),
            "{owner:?}"
        );
        assert!(state.planet_screen_rect(&world, layout(1.0), other).is_some());
        assert!(state.planet_screen_rect(&world, layout(1.0), system).is_none());
    }
}

#[test]
fn a_window_command_opens_the_sector_and_the_window_at_its_icon() {
    // As a double click on the icon (FUN_004593e0 case 0x203,
    // FUN_0045aac0): the window opens at the icon's point.
    for owner in [Faction::Alliance, Faction::Empire] {
        let (world, system) = owned_icons_world(owner);
        let mut state = SectorWindowState::default();
        let actions = run_command(
            &world,
            owner,
            &mut state,
            palette_command(&world, "Open Fleet window: Chandrila"),
        )
        .unwrap();
        let point = state
            .quadrant_window_point(&world, layout(1.0), system, Quadrant::Fleets)
            .expect("the sector window is open");
        assert_eq!(
            actions,
            [SectorWindowAction::OpenFleetWindow {
                system,
                logical_position: point
            }],
            "{owner:?}"
        );
    }
}

#[test]
fn a_hidden_icon_refuses_its_window_and_menu() {
    // Every window and menu needs its icon shown, the System window
    // too: FUN_004593e0 case 0x203 hit-tests the icons only.
    let (world, _, _) = fixture_world();
    for owner in [Faction::Alliance, Faction::Empire] {
        let mut state = SectorWindowState::default();
        for label in [
            "Open System window: Chandrila",
            "Open Defenses window: Chandrila",
            "Open Fleet window: Chandrila",
            "Open Missions window: Chandrila",
            "Open system icon menu: Chandrila",
        ] {
            assert_eq!(
                run_command(&world, owner, &mut state, palette_command(&world, label)),
                Err("the icon is hidden"),
                "{owner:?}: {label}"
            );
        }
        assert_eq!(
            run_command(
                &world,
                owner,
                &mut state,
                palette_command(&world, "Start game: Alliance")
            ),
            Err("the app starts a game"),
            "{owner:?}"
        );
    }
}

#[test]
fn an_icon_menu_command_selects_the_icon_and_opens_its_menu_there() {
    // As a right click on the icon: FUN_0045b1b0 selects it, so it
    // paints its second bitmap, and the release opens its menu at the
    // icon (FUN_004ac5c0).
    for (owner, (fleet_art, fleet_pressed)) in [
        (Faction::Alliance, (10775, 10776)),
        (Faction::Empire, (10783, 10784)),
    ] {
        let (world, system) = owned_icons_world(owner);
        let mut state = SectorWindowState::default();
        let actions = run_command(
            &world,
            owner,
            &mut state,
            palette_command(&world, "Open fleet icon menu: Chandrila"),
        )
        .unwrap();
        let point = state
            .quadrant_window_point(&world, layout(1.0), system, Quadrant::Fleets)
            .unwrap();
        assert_eq!(
            actions,
            [SectorWindowAction::OpenObjectMenu {
                selection: Some(MenuObject::SystemIcon {
                    system,
                    quadrant: Quadrant::Fleets
                }),
                point
            }],
            "{owner:?}"
        );
        let painted = icons_after_clicks(&world, owner, &mut state, &[]);
        assert!(painted.contains(&fleet_pressed), "{owner:?}: {painted:?}");
        assert!(!painted.contains(&fleet_art), "{owner:?}: {painted:?}");
    }
}
