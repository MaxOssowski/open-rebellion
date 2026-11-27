use super::*;

#[test]
fn each_tab_has_its_tooltip_string() {
    // FUN_0049f540 L288, L305.
    assert_eq!(
        super::MissionsTab::ALL.map(super::MissionsTab::tooltip_string),
        [0x8500, 0x8501]
    );
}
use crate::cockpit::CockpitViewport;
use crate::fleet_window::en_route_mark;
use crate::fleet_window::tests::PAINTED;
use rebellion_core::dat::{ExplorationStatus, SectorGroup};
use rebellion_core::ids::DatId;
use rebellion_core::missions::{MissionKind, MissionRequest};
use rebellion_core::world::{Character, ControlKind, Sector, System};

fn world(control: ControlKind) -> (GameWorld, SystemKey) {
    let mut world = GameWorld::default();
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(36),
        name: "Sesswenna".into(),
        group: SectorGroup::Core,
        x: 317,
        y: 248,
        systems: Vec::new(),
    });
    let system = world.systems.insert(System {
        dat_id: DatId::new(100),
        name: "Sluis Van".into(),
        sector,
        x: 320,
        y: 250,
        exploration_status: ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control,
        ..Default::default()
    });
    (world, system)
}

fn character(
    world: &mut GameWorld,
    system: SystemKey,
    name: &str,
    alliance: bool,
) -> CharacterKey {
    world.characters.insert(Character {
        dat_id: DatId::new(832),
        name: name.into(),
        is_alliance: alliance,
        is_empire: !alliance,
        current_system: Some(system),
        recruited: true,
        ..Default::default()
    })
}

fn faction(alliance: bool) -> MissionFaction {
    if alliance {
        MissionFaction::Alliance
    } else {
        MissionFaction::Empire
    }
}

/// Send `team` and `decoys` on a `kind` mission at `system`.
fn send(
    missions: &mut MissionState,
    kind: MissionKind,
    alliance: bool,
    team: &[CharacterKey],
    decoys: &[CharacterKey],
    system: SystemKey,
    target_character: Option<CharacterKey>,
) -> u64 {
    missions.dispatch(MissionRequest {
        kind,
        faction: faction(alliance),
        team: team.iter().copied().map(MissionMember::Character).collect(),
        decoys: decoys
            .iter()
            .copied()
            .map(MissionMember::Character)
            .collect(),
        target_system: system,
        target_character,
        target_object: None,
        tick: 0,
    })
}

fn unseen() -> FogState {
    FogState::new(Faction::Alliance)
}

/// An Alliance system with an Alliance Diplomacy mission (two agents and
/// a decoy) and an Empire Espionage mission (one agent).
fn busy() -> (GameWorld, SystemKey, MissionState, [CharacterKey; 4]) {
    let (mut world, system) = world(ControlKind::Controlled(Faction::Alliance));
    let first = character(&mut world, system, "Leia", true);
    let second = character(&mut world, system, "Mon", true);
    let decoy = character(&mut world, system, "Decoy", true);
    let spy = character(&mut world, system, "Spy", false);
    let mut missions = MissionState::new();
    send(
        &mut missions,
        MissionKind::Diplomacy,
        true,
        &[first, second],
        &[decoy],
        system,
        None,
    );
    send(
        &mut missions,
        MissionKind::Espionage,
        false,
        &[spy],
        &[],
        system,
        None,
    );
    (world, system, missions, [first, second, decoy, spy])
}

#[test]
fn a_member_on_its_way_is_listed_at_its_target_with_the_en_route_mark() {
    // FUN_00556430 puts a travelling member in its target's container at
    // once (mission-lifecycle.md); FUN_0042c3b0 draws 11501 over the
    // mini of an en route (+0x50 bit 4) character.
    let (mut world, system, _, _) = busy();
    let traveller = world.characters.insert(Character {
        dat_id: DatId::new(832),
        name: "Traveller".into(),
        is_alliance: true,
        current_system: None,
        recruited: true,
        ..Default::default()
    });
    let member = MissionMember::Character(traveller);
    let transit = rebellion_core::missions::MemberTransit {
        member,
        mission_id: 0,
        to: system,
        arrival: 9,
        abode: None,
    };
    let listed = |transits: &[rebellion_core::missions::MemberTransit]| {
        listed_members(&world, &unseen(), transits, Faction::Alliance, system)
    };
    assert!(!listed(&[]).iter().any(|(m, ..)| *m == member));
    let with = listed(&[transit]);
    assert!(with.contains(&(member, 1, false, true)), "{with:?}");
    assert_eq!(
        mini_object(&world, member).map(|object| en_route_mark(object, 0)),
        Some((DllSource::Strategy, 11_501))
    );
}

#[test]
fn a_killed_or_unseen_traveller_is_not_listed_and_two_force_classes_carry_their_own_mark() {
    // port: a member bound for a system shows where the System window
    // shows its side's members, and a killed one not at all.
    // FUN_0042c3b0 keeps the GOKRES default (class & 0xfff) + 0x5000 for
    // classes 0x3c000003/0x3c000005 instead of 11501.
    let (mut world, system, _, _) = busy();
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    let mut traveller = |name: &str, is_alliance: bool, is_killed: bool| {
        MissionMember::Character(world.characters.insert(Character {
            dat_id: DatId::new(832),
            name: name.into(),
            is_alliance,
            is_empire: !is_alliance,
            is_killed,
            recruited: true,
            ..Default::default()
        }))
    };
    let own = traveller("Own", true, false);
    let killed = traveller("Killed", true, true);
    let enemy = traveller("Enemy", false, false);
    let transits: Vec<_> = [own, killed, enemy]
        .into_iter()
        .map(|member| rebellion_core::missions::MemberTransit {
            member,
            mission_id: 0,
            to: system,
            arrival: 9,
            abode: None,
        })
        .collect();
    let listed = |world: &GameWorld, fog: &FogState| {
        listed_members(world, fog, &transits, Faction::Alliance, system)
            .into_iter()
            .filter(|(.., en_route)| *en_route)
            .map(|(member, ..)| member)
            .collect::<Vec<_>>()
    };
    assert_eq!(listed(&world, &unseen()), [own]);
    let seen = unseen();
    crate::viewed_contents::see_live(&mut world, Faction::Alliance, system);
    assert_eq!(listed(&world, &seen), [own, enemy]);

    let mut force = |class: u32| {
        MissionMember::SpecialForce(world.special_forces.insert(
            rebellion_core::world::SpecialForceUnit {
                class_dat_id: DatId::new(class),
                is_alliance: true,
                skills: [0; 8],
                on_mission: false,
                on_hidden_mission: false,
            },
        ))
    };
    let forces: Vec<_> = [
        (0x3c00_0001, (DllSource::Strategy, 11_501)),
        (0x3c00_0003, (DllSource::Gokres, 21_826)),
        (0x3c00_0005, (DllSource::Gokres, 21_888)),
    ]
    .into_iter()
    .map(|(class, mark)| (class, force(class), mark))
    .collect();
    for (class, member, mark) in forces {
        let mini = member_mini(&world, member).and_then(|(mini, _)| mini);
        assert_eq!(
            mini.zip(mini_object(&world, member))
                .map(|(mini, object)| en_route_mark(object, mini)),
            Some(mark),
            "0x{class:x}"
        );
    }
}

fn labels(rows: &[MissionRow]) -> Vec<(&'static str, u8, u32)> {
    rows.iter()
        .map(|row| (row.label, row.side, row.picture))
        .collect()
}

#[test]
fn each_visible_mission_gets_one_row_with_its_sides_mini_and_name() {
    // FUN_004a1590: one row per mission key of the members on a visible
    // mission; GOKRES 0x4000 + (class +0x30 & 0xfff), 0x5000 + .. for
    // side 2: Diplomacy (TEXTSTRA 0x2c10) 0x4c10, Espionage (0x2c13)
    // 0x5c13.
    let (world, system, missions, _) = busy();
    assert_eq!(
        labels(&mission_rows(
            &world,
            &unseen(),
            &missions,
            Faction::Alliance,
            system
        )),
        [("Diplomacy", 1, 0x4c10), ("Espionage", 2, 0x5c13)]
    );
}

#[test]
fn hidden_missions_and_unseen_enemies_get_no_row() {
    // FUN_004a1590 skips role +0x78 bit 8 (a hidden mission). port: the
    // other side's members show only where the System window shows
    // them (an own or revealed system).
    let (mut world, system, missions, [first, second, decoy, _]) = busy();
    for member in [first, second, decoy] {
        world.characters[member].on_hidden_mission = true;
    }
    world.systems[system].control = ControlKind::Controlled(Faction::Empire);
    assert!(mission_rows(&world, &unseen(), &missions, Faction::Alliance, system).is_empty());

    let seen = unseen();
    crate::viewed_contents::see_live(&mut world, Faction::Alliance, system);
    assert_eq!(
        labels(&mission_rows(
            &world,
            &seen,
            &missions,
            Faction::Alliance,
            system
        )),
        [("Espionage", 2, 0x5c13)]
    );
}

#[test]
fn the_rail_icon_and_title_strip_prefer_the_other_sides_missions() {
    // FUN_004a21c0 and FUN_004a2200 read FUN_004a1f60: the other side's
    // missions win, then the player's, else side 3.
    let (mut world, system, mut missions, [.., spy]) = busy();
    let icon = |world: &GameWorld, missions: &MissionState| {
        rail_icon(world, &unseen(), missions, CockpitFaction::Alliance, system)
    };
    assert_eq!(icon(&world, &missions), 11540);
    world.characters[spy].current_system = None;
    assert_eq!(icon(&world, &missions), 11539);
    missions = MissionState::new();
    assert_eq!(icon(&world, &missions), 11541);
    assert_eq!(
        [1, 2, 3].map(|side| [title_resource(side, true), title_resource(side, false)]),
        [[10299, 10200], [10201, 10302], [10303, 10304]]
    );
}

#[test]
fn each_tab_shows_the_selected_missions_side_art() {
    // FUN_004a0ca0: 0x2d28/0x2d29 and 0x2d2a/0x2d2b for side 1,
    // 0x2d2d/0x2d2e and 0x2d2f/0x2d30 for side 2, none otherwise; the
    // selected tab shows its pressed bitmap (FUN_0060d700).
    let cases = [
        (MissionsTab::Agents, 1, Some([11560, 11561])),
        (MissionsTab::Decoys, 1, Some([11562, 11563])),
        (MissionsTab::Agents, 2, Some([11565, 11566])),
        (MissionsTab::Decoys, 2, Some([11567, 11568])),
        (MissionsTab::Agents, 3, None),
    ];
    for (tab, side, art) in cases {
        assert_eq!(
            tab_resource(tab, side, false).zip(tab_resource(tab, side, true)),
            art.map(|[normal, selected]| (normal, selected)),
            "{tab:?} side {side}"
        );
    }
    // TEXTSTRA 34048 and 34049.
    assert_eq!(
        MissionsTab::ALL.map(MissionsTab::name),
        ["Agents", "Decoys"]
    );
}

#[test]
fn opening_selects_the_first_mission_and_its_agents_then_its_decoys_on_their_tab() {
    // FUN_0049f540 selects the first row (FUN_00609500); FUN_004a0ca0
    // fills the Agents tab (FUN_004a1ba0(0x17)): members carrying the
    // key with +0x78 bit 0 clear, the decoys on tab 0x18.
    let (world, system, missions, _) = busy();
    let fog = unseen();
    let mut state = MissionsWindowState::default();
    assert!(state.open(
        &world,
        &fog,
        &missions,
        system,
        (20, 30),
        CockpitFaction::Alliance,
        scaled()
    ));
    let report = state.report(&world, &fog, &missions, system).unwrap();
    assert_eq!(report.selected, Some(0));
    assert_eq!(report.tab, MissionsTab::Agents);
    assert_eq!(report.tab_side, Some(1));
    assert_eq!(report.members, ["Leia", "Mon"]);
    assert_eq!(report.side, 2);

    state.windows[0].tab = MissionsTab::Decoys;
    let report = state.report(&world, &fog, &missions, system).unwrap();
    assert_eq!(report.members, ["Decoy"]);
}

#[test]
fn showing_a_member_selects_its_mission_and_the_tab_its_role_picks() {
    // FUN_004a1e10 (slot 27): the member's mission key selects its row;
    // role bit 0 picks the decoys' tab. FUN_00429440 sends a character
    // on a visible mission here (kind 11).
    let (world, system, missions, [_, _, decoy, spy]) = busy();
    let fog = unseen();
    let mut state = MissionsWindowState::default();
    assert!(!state.show_member(&missions, system, MissionMember::Character(spy)));
    assert!(state.open(
        &world,
        &fog,
        &missions,
        system,
        (20, 30),
        CockpitFaction::Alliance,
        scaled()
    ));

    assert!(state.show_member(&missions, system, MissionMember::Character(spy)));
    assert_eq!(state.windows[0].selected, Some(1));
    assert_eq!(state.windows[0].tab, MissionsTab::Agents);

    assert!(state.show_member(&missions, system, MissionMember::Character(decoy)));
    assert_eq!(state.windows[0].selected, Some(0));
    assert_eq!(state.windows[0].tab, MissionsTab::Decoys);

    let idle = CharacterKey::default();
    assert!(!state.show_member(&missions, system, MissionMember::Character(idle)));
    assert_eq!(state.windows[0].selected, Some(0));
}

#[test]
fn the_kind_claims_the_shared_focus_only_while_a_window_is_open() {
    // window_focus.rs: a kind with no window open makes no claim; one it
    // raises claims later than any stamp taken before.
    let (world, system, missions, _) = busy();
    let fog = unseen();
    let mut state = MissionsWindowState::default();
    assert_eq!(state.focus_stamp(), None);
    let mut earlier = crate::window_focus::FocusStamp::default();
    earlier.take();
    assert!(state.open(
        &world,
        &fog,
        &missions,
        system,
        (20, 30),
        CockpitFaction::Alliance,
        scaled()
    ));
    let stamp = state.focus_stamp().expect("a window is open");
    assert!(stamp > earlier.stamp(), "{stamp} after {}", earlier.stamp());
    assert!(state.focus.holds());
    state.set_holds_focus(false);
    assert!(!state.focus.holds());
}

#[test]
fn a_minimized_window_comes_back_with_its_selected_mission_and_tab() {
    // FUN_00428b40 keeps the minimized window's object in its rail slot
    // for every window kind (A0 personnel-states/39, the Fleet window
    // back with its selection), so reopening shows it as it was.
    let (world, system, missions, [_, _, _, spy]) = busy();
    let fog = unseen();
    let mut state = MissionsWindowState::default();
    let open = |state: &mut MissionsWindowState, at| {
        state.open(&world, &fog, &missions, system, at, CockpitFaction::Alliance, scaled())
    };
    assert!(open(&mut state, (20, 30)));
    assert!(state.show_member(&missions, system, MissionMember::Character(spy)));
    state.windows[0].tab = MissionsTab::Decoys;
    let at = state.windows[0].logical_position;
    assert_eq!(state.hide(system), Some(at));
    assert_eq!(state.window_count(), 0);
    assert!(state.take_created());

    assert!(open(&mut state, (0, 0)));
    let window = &state.windows[0];
    assert_eq!(
        (window.selected, window.tab, window.logical_position),
        (Some(1), MissionsTab::Decoys, at)
    );
    assert!(state.hidden.is_empty());
}

#[test]
fn the_details_name_the_own_targets_and_only_the_other_sides_system_missions() {
    // FUN_004a10a0: the player's own mission names its order's target
    // (+0x70); another side's mission names the system for families
    // 0x50..0x5f (Espionage 0x52), else nothing: TEXTSTRA 34087
    // "Target Unknown" (Assassination 0x63).
    let (mut world, system, mut missions, [first, ..]) = busy();
    let other = character(&mut world, system, "Killer", false);
    send(
        &mut missions,
        MissionKind::Assassination,
        false,
        &[other],
        &[],
        system,
        Some(first),
    );
    let fog = unseen();
    let rows = mission_rows(&world, &fog, &missions, Faction::Alliance, system);
    let name = |row: &MissionRow| {
        target_name(&world, target(&missions, Faction::Alliance, system, row))
    };
    assert_eq!(
        rows.iter().map(name).collect::<Vec<_>>(),
        ["Sluis Van", "Sluis Van", "Target Unknown"]
    );

    // The player's own Assassination names its target character.
    let own = character(&mut world, system, "Agent", true);
    let enemy = character(&mut world, system, "Vader", false);
    let mut missions = MissionState::new();
    send(
        &mut missions,
        MissionKind::Assassination,
        true,
        &[own],
        &[],
        system,
        Some(enemy),
    );
    let rows = mission_rows(&world, &fog, &missions, Faction::Alliance, system);
    assert_eq!(
        target_name(
            &world,
            target(&missions, Faction::Alliance, system, &rows[0])
        ),
        "Vader"
    );
    assert_eq!(
        target_view(&world, Target::Character(enemy)).map(|(source, id, _)| (source, id)),
        Some((DllSource::Gokres, Some(18_176 + 832)))
    );
}

#[test]
fn an_objects_target_picture_is_its_portrait_a_mini_less_0x4000() {
    // FUN_0042c3b0(.., 1, 1): class +0x30 & 0xfff, against the mini's
    // + 0x4000; GOKRES 1600 is the KDY-150's 122 by 50 portrait.
    let (mut world, _) = world(ControlKind::Uncontrolled);
    let battery =
        world
            .defense_facilities
            .insert(rebellion_core::world::DefenseFacilityInstance {
                class_dat_id: DatId::new(0x2200_0001),
                side: rebellion_core::dat::Faction::Empire,
            });
    assert_eq!(
        target_view(
            &world,
            Target::Object(MissionTarget::DefenseFacility(battery))
        ),
        Some((DllSource::Gokres, Some(16_896 - 0x4000), "KDY-150".into()))
    );
}

#[test]
fn a_death_star_target_shows_the_living_hulls_portrait() {
    // FUN_0042c3b0(.., 1, 1) on the hull MissionTarget::DeathStar
    // names: GOKRES 18312 (its mini) less 0x4000; a destroyed hull is
    // no target.
    let (mut world, _) = world(ControlKind::Uncontrolled);
    let mut class = |dat_id: u32, name: &str| {
        world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass {
                dat_id: DatId::new(dat_id),
                name: name.into(),
                ..Default::default()
            })
    };
    let destroyer = class(133, "Imperial I Star Destroyer");
    let death_star = class(136, "Death Star");
    let ship = rebellion_core::world::ShipInstance::new;
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        capital_ships: vec![ship(destroyer, 10, false), ship(death_star, 10, false)],
        has_death_star: true,
        ..Default::default()
    });
    assert_eq!(
        target_view(&world, Target::Object(MissionTarget::DeathStar(fleet))),
        Some((
            DllSource::Gokres,
            Some(18_312 - 0x4000),
            "Death Star".into()
        ))
    );
    world.fleets[fleet].capital_ships[1].alive = false;
    assert_eq!(
        target_view(&world, Target::Object(MissionTarget::DeathStar(fleet))),
        None
    );
}

#[test]
fn a_member_without_a_mapped_mini_still_lists_its_name() {
    // FUN_004a0e10 adds a row for every member; port: a character the
    // port maps no mini for keeps its name.
    let (mut world, system, missions, [first, ..]) = busy();
    world.characters[first].dat_id = DatId::new(1);
    let rows = member_rows(
        &world,
        &unseen(),
        &missions,
        Faction::Alliance,
        system,
        missions.missions()[0].id,
        MissionsTab::Agents,
    );
    assert_eq!(
        rows.iter()
            .map(|row| (row.mini, row.label.as_str()))
            .collect::<Vec<_>>(),
        [(None, "Leia"), (Some(18_176 + 832), "Mon")]
    );
    assert!(
        rows.iter()
            .all(|row| !row.marks.contains(&(DllSource::Strategy, 11_501))),
        "a member at the system is not en route"
    );
    // A character's mini stands on STRATEGY 11500 (FUN_0042c3b0).
    assert_eq!(rows[1].marks, [(DllSource::Strategy, 11_500)]);
    assert!(rows[0].marks.is_empty(), "no mini, no backdrop");
    // FUN_004f7800: an injured character is damaged, so FUN_0042c3b0
    // draws STRATEGY 11502 under its mini.
    let mon = match rows[1].member {
        MissionMember::Character(key) => key,
        MissionMember::SpecialForce(_) => unreachable!(),
    };
    world.characters[mon].injury = 5;
    let rows = member_rows(
        &world,
        &unseen(),
        &missions,
        Faction::Alliance,
        system,
        missions.missions()[0].id,
        MissionsTab::Agents,
    );
    assert_eq!(
        rows[1].marks,
        [(DllSource::Strategy, 11_500), (DllSource::Strategy, 11_502)]
    );
    assert!(rows[0].marks.is_empty());
}

#[test]
fn a_bitmap_centres_with_integer_offsets_and_a_members_name_sits_under_its_mini() {
    // FUN_0049fef0 and FUN_004a0e10: (box - size) / 2 in integers; the
    // name's top is the mini's height plus its offset (item +0x34).
    assert_eq!(centered((122.0, 50.0), [61, 25]), (30.0, 12.0));
    assert_eq!(centered((122.0, 43.0), [66, 25]), (28.0, 9.0));
    assert_eq!(centered((122.0, 50.0), [122, 50]), (0.0, 0.0));
    assert_eq!(picture_at(Some([61, 25])), (138.0, 49.0));
    assert_eq!(picture_at(None), (108.0, 37.0));
    assert_eq!(member_text_top(25), 34.0);
    assert_eq!(member_text_top(0), 21.0);
}

/// The canvas 10 by 20 pixels in and twice the original size.
fn scaled() -> CockpitLayout {
    let viewport = CockpitViewport {
        x: 10.0,
        y: 20.0,
        width: 1280.0,
        height: 960.0,
    };
    CockpitLayout {
        canvas: viewport,
        galaxy: viewport,
        scale: 2.0,
    }
}

const ORIGIN: (i16, i16) = (20, 30);

/// A logical point in the window opened at `ORIGIN` on [`scaled`].
fn at(x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(
        10.0 + (f32::from(ORIGIN.0) + x) * 2.0,
        20.0 + (f32::from(ORIGIN.1) + y) * 2.0,
    )
}

fn opened(
    world: &GameWorld,
    missions: &MissionState,
    system: SystemKey,
) -> MissionsWindowState {
    let mut state = MissionsWindowState::default();
    assert!(state.open(
        world,
        &unseen(),
        missions,
        system,
        ORIGIN,
        CockpitFaction::Alliance,
        scaled()
    ));
    state
}

#[test]
fn the_windows_rects_and_release_target_follow_the_canvas_offset_and_scale() {
    // The mission rows at (5, 24 + 50 n), the tabs at (105, 127) and
    // (166, 127), the member rows at (107, 145 + 43 n) (FUN_0049f540,
    // FUN_00609ae0); +0x70 gives the subject (FUN_004aa470).
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    // FUN_0045aac0 tells the agent of a created window only.
    assert!(state.take_created());
    assert!(state.open(
        &world,
        &unseen(),
        &missions,
        system,
        (300, 100),
        CockpitFaction::Alliance,
        scaled()
    ));
    assert!(!state.take_created());
    assert_eq!(state.window_count(), 1);
    assert!(state.is_open(system));
    assert_eq!(
        state.mission_row_screen_rect(scaled(), system, 1),
        Some(egui::Rect::from_min_size(
            at(5.0, 74.0),
            egui::vec2(180.0, 100.0)
        ))
    );
    assert_eq!(
        state.tab_screen_rect(scaled(), system, MissionsTab::Decoys),
        Some(egui::Rect::from_min_size(
            at(166.0, 127.0),
            egui::vec2(122.0, 32.0)
        ))
    );
    assert_eq!(
        state.member_row_screen_rect(scaled(), system, 2),
        Some(egui::Rect::from_min_size(
            at(107.0, 231.0),
            egui::vec2(230.0, 86.0)
        ))
    );
    let point = |p: egui::Pos2| (p.x, p.y);
    assert!(state.contains_screen_point(scaled(), point(at(234.0, 303.0))));
    assert!(!state.contains_screen_point(scaled(), point(at(235.5, 150.0))));

    let layer = egui::LayerId::new(egui::Order::Foreground, area_id(system));
    assert_eq!(state.release_target(layer), Some(system));
    let elsewhere = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("elsewhere"));
    assert_eq!(state.release_target(elsewhere), None);

    assert!(state.open(
        &world,
        &unseen(),
        &missions,
        system,
        ORIGIN,
        CockpitFaction::Empire,
        scaled()
    ));
    assert_eq!(state.faction, CockpitFaction::Empire);
    state.clear();
    assert_eq!(state.window_count(), 0);
    assert!(!state.is_open(system));
}

#[derive(Debug, Clone, PartialEq)]
/// A painted line ([`crate::theme::TestLine`]): `pos` is its pen's left
/// and its baseline.
struct Text {
    text: String,
    pos: egui::Pos2,
    font: f32,
}

#[derive(Default)]
struct Run {
    actions: Vec<MissionsWindowAction>,
    texts: Vec<Text>,
    painted: Vec<(u32, egui::Pos2)>,
}

/// Draw the windows once per frame of events, on [`scaled`] for the
/// Alliance; the last frame's text and bitmaps are kept.
fn run(
    world: &GameWorld,
    missions: &MissionState,
    state: &mut MissionsWindowState,
    frames: Vec<Vec<egui::Event>>,
) -> Run {
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let fog = unseen();
    let mut result = Run::default();
    for (index, events) in frames.into_iter().enumerate() {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            time: Some(index as f64 * 0.05),
            events,
            ..Default::default()
        };
        PAINTED.with(|painted| painted.borrow_mut().clear());
        let output = ctx.run(input, |ctx| {
            result.actions.extend(draw_missions_windows(
                ctx,
                world,
                &fog,
                missions,
                state,
                CockpitFaction::Alliance,
                scaled(),
                &mut cache,
            ));
        });
        result.painted = PAINTED.with(|painted| painted.take());
        result.texts = crate::theme::test_lines(&output.shapes)
            .into_iter()
            .map(|line| Text {
                text: line.text,
                pos: line.pen,
                font: line.size,
            })
            .collect();
    }
    result
}

fn hover(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    vec![
        vec![egui::Event::PointerMoved(point)],
        vec![egui::Event::PointerMoved(point)],
    ]
}

fn press(point: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

fn click(point: egui::Pos2) -> Vec<Vec<egui::Event>> {
    let mut frames = hover(point);
    frames.extend([vec![press(point, true)], vec![press(point, false)], vec![]]);
    frames
}

fn text<'a>(run: &'a Run, text: &str) -> &'a Text {
    run.texts
        .iter()
        .find(|value| value.text == text)
        .unwrap_or_else(|| panic!("no {text:?} in {:?}", run.texts))
}

#[test]
fn the_window_paints_its_chrome_rows_frame_tabs_members_and_target() {
    // FUN_0049fef0: 11165, the focused strip of FUN_004a1f60's side (2:
    // 10201) at (2, 2), the title at (21, 2) 16 high; the buttons;
    // each mission's mini at its row's corner with the side 1 frame
    // 11127 over the selected one, its name from (1, 0) wrapped at 87;
    // the selected mission's tabs (Agents pressed); its agents' minis
    // and centred names under them; "Target:" and the target's name
    // centred over the 113-wide rects at (109, 25) and (109, 91).
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    let run = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));

    for (id, point) in [
        (BACKGROUND, at(0.0, 0.0)),
        (10201, at(2.0, 2.0)),
        (SECTOR_NORMAL, at(3.0, 3.0)),
        (MINIMIZE_NORMAL, at(204.0, 3.0)),
        (CLOSE_NORMAL, at(218.0, 3.0)),
        (0x4c10, at(5.0, 24.0)),
        (MISSION_FRAME[0], at(5.0, 24.0)),
        (0x5c13, at(5.0, 74.0)),
        (11561, at(105.0, 127.0)),
        (11562, at(166.0, 127.0)),
        (18_176 + 832, at(107.0, 145.0)),
        (18_176 + 832, at(107.0, 188.0)),
    ] {
        assert!(
            run.painted.contains(&(id, point)),
            "{id} at {point:?} in {:?}",
            run.painted
        );
    }
    assert_eq!(
        run.painted
            .iter()
            .filter(|(id, _)| MISSION_FRAME.contains(id))
            .count(),
        1
    );
    // The target system's planet (FUN_0045c970), whose size the test
    // cache lacks, so it lands at the box's corner.
    assert!(run
        .painted
        .contains(&(planet_resource_id(DatId::new(100)), at(108.0, 37.0))));

    // DrawTextA (theme::paint_gdi_text): each line's pen and baseline,
    // tmAscent 13 below a font-5 box's top and 11 below a font-10 one;
    // DT_CENTER halves the spare width in whole pixels.
    let centred = |words: &str, font: u16, x: f32, w: f32| {
        let (_, width) = crate::theme::gdi_pen_positions(font, words).unwrap();
        at(x + ((w - width) / 2.0).floor(), 0.0).x
    };
    let title = text(&run, "Mission at Sluis Van");
    assert_eq!(
        (title.pos, title.font),
        (at(21.0, 2.0 + 13.0), crate::theme::gdi_font(5, 2.0).size)
    );
    // Font 10 (FUN_0049f540: the list's slot +0x18 with 10).
    let row = text(&run, "Diplomacy");
    assert_eq!(
        (row.pos, row.font),
        (at(6.0, 24.0 + 11.0), crate::theme::gdi_font(10, 2.0).size)
    );
    let member = text(&run, "Leia");
    assert_eq!(member.pos.x, centred("Leia", 10, 108.0, 115.0));
    assert_eq!(member.pos.y, at(0.0, 166.0 + 11.0).y);
    let label = text(&run, "Target:");
    assert_eq!(label.pos.x, centred("Target:", 5, 109.0, 113.0));
    assert_eq!(label.pos.y, at(0.0, 25.0 + 13.0).y);
    let name = text(&run, "Sluis Van");
    assert_eq!(name.pos.x, centred("Sluis Van", 5, 109.0, 113.0));
    assert_eq!(name.pos.y, at(0.0, 91.0 + 13.0).y);
}

#[test]
fn a_right_click_on_a_mission_selects_it_and_opens_its_menu() {
    // The window's slot 22 (FUN_004a0400) gives the selected mission to
    // FUN_004ac5c0; A0 missionstatus/02 shows its Encyclopedia, Status,
    // Abort.
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    let point = at(40.0, 90.0);
    let mut frames = hover(point);
    let right = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    frames.extend([vec![right(true)], vec![right(false)], vec![]]);
    let run = run(&world, &missions, &mut state, frames);
    let espionage = missions.missions()[1].id;
    assert_eq!(state.windows[0].selected, Some(espionage));
    assert!(
        run.actions.iter().any(|action| matches!(
            action,
            MissionsWindowAction::OpenObjectMenu { system: s, mission, .. }
                if *s == system && *mission == espionage
        )),
        "{:?}",
        run.actions
    );
}

#[test]
fn a_click_on_another_mission_selects_it_and_a_tab_click_switches_the_members() {
    // FUN_004a0c60 refreshes for a different mission and FUN_004a0ca0
    // reselects the Agents tab; id 0x16 refills for the clicked tab.
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);

    let decoys = run(&world, &missions, &mut state, click(at(190.0, 135.0)));
    assert_eq!(state.windows[0].tab, MissionsTab::Decoys);
    assert!(decoys.painted.contains(&(11563, at(166.0, 127.0))));
    assert_eq!(decoys.actions, [MissionsWindowAction::SelectSystem(system)]);

    let other = run(&world, &missions, &mut state, click(at(40.0, 90.0)));
    let espionage = missions.missions()[1].id;
    assert_eq!(state.windows[0].selected, Some(espionage));
    assert_eq!(state.windows[0].tab, MissionsTab::Agents);
    assert!(other.painted.contains(&(MISSION_FRAME[0], at(5.0, 74.0))));
    assert!(other.painted.contains(&(11566, at(105.0, 127.0))));
    assert!(other.texts.iter().any(|value| value.text == "Spy"));

    // The same mission keeps the current tab.
    state.windows[0].tab = MissionsTab::Decoys;
    let _ = run(&world, &missions, &mut state, click(at(40.0, 90.0)));
    assert_eq!(state.windows[0].tab, MissionsTab::Decoys);
}

#[test]
fn a_selected_mission_that_ends_clears_the_selection_and_hides_the_details() {
    // FUN_004a0c20 clears +0x1c4 when its row goes; FUN_004a0ca0 then
    // hides the strip and the member list, FUN_004a10a0 the texts.
    let (world, system, mut missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    let first = missions.missions()[0].id;
    assert!(missions.cancel(first).is_some());

    let run = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    assert_eq!(state.windows[0].selected, None);
    assert!(!run
        .painted
        .iter()
        .any(|(id, _)| (11560..=11569).contains(id) || MISSION_FRAME.contains(id)));
    assert!(!run.texts.iter().any(|value| value.text == "Target:"));
    assert!(run.painted.contains(&(0x5c13, at(5.0, 24.0))));
}

/// Press at screen point `from`, move to `to` and hold; then release.
/// Returns the window's position while held and after the release.
fn move_window(
    world: &GameWorld,
    missions: &MissionState,
    state: &mut MissionsWindowState,
    from: egui::Pos2,
    to: egui::Pos2,
) -> ((i16, i16), (i16, i16)) {
    let mut frames = hover(from);
    frames.extend([
        vec![press(from, true)],
        vec![egui::Event::PointerMoved(to)],
        vec![egui::Event::PointerMoved(to)],
    ]);
    run(world, missions, state, frames);
    let held = state.windows[0].logical_position;
    run(world, missions, state, vec![vec![press(to, false)], vec![]]);
    (held, state.windows[0].logical_position)
}

// The window's constructor sets +0xb8 bit 4, so a left press on its own
// surface holds it (FUN_00606ee0); it follows the pointer unbounded
// (FUN_006071e0) and the drop puts it back inside the galaxy view
// (FUN_00607010). A press on the list goes to the list.
#[test]
fn the_window_moves_by_its_surface_and_drops_inside_the_galaxy_view() {
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    let (held, dropped) = move_window(
        &world,
        &missions,
        &mut state,
        at(100.0, 10.0),
        at(130.0, 30.0),
    );
    assert_eq!((held, dropped), ((50, 50), (50, 50)));
    let from = at(130.0, 30.0);
    let to = from - egui::vec2(200.0, 0.0);
    let (held, dropped) = move_window(&world, &missions, &mut state, from, to);
    assert_eq!((held, dropped), ((-50, 50), (0, 50)));
    let list = egui::pos2(10.0 + 40.0 * 2.0, 20.0 + (50.0 + 200.0) * 2.0);
    let (held, dropped) = move_window(
        &world,
        &missions,
        &mut state,
        list,
        list + egui::vec2(60.0, 60.0),
    );
    assert_eq!((held, dropped), ((0, 50), (0, 50)));
}

#[test]
fn the_title_buttons_open_the_sector_minimize_and_close() {
    // 0xca opens the sector window (FUN_00429ce0); 0x15 posts 0x466;
    // 0x14 closes.
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    let sector = run(&world, &missions, &mut state, click(at(9.0, 9.0)));
    assert_eq!(
        sector.actions,
        [
            MissionsWindowAction::OpenSector(system),
            MissionsWindowAction::SelectSystem(system)
        ]
    );

    let minimized = run(&world, &missions, &mut state, click(at(210.0, 9.0)));
    assert_eq!(
        minimized.actions,
        [MissionsWindowAction::Minimize {
            system,
            logical_position: ORIGIN
        }]
    );
    assert_eq!(state.window_count(), 0);

    let mut state = opened(&world, &missions, system);
    let closed = run(&world, &missions, &mut state, click(at(224.0, 9.0)));
    assert!(closed.actions.is_empty());
    assert_eq!(state.window_count(), 0);
}

#[test]
fn a_title_button_shows_its_pressed_art_only_while_the_button_is_held_over_it() {
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    let hovered = run(&world, &missions, &mut state, hover(at(210.0, 9.0)));
    assert!(hovered.painted.contains(&(MINIMIZE_NORMAL, at(204.0, 3.0))));

    let mut frames = hover(at(224.0, 9.0));
    frames.push(vec![press(at(224.0, 9.0), true)]);
    let held = run(&world, &missions, &mut state, frames);
    assert!(held.painted.contains(&(CLOSE_PRESSED, at(218.0, 3.0))));
    assert!(held.painted.contains(&(MINIMIZE_NORMAL, at(204.0, 3.0))));
    assert_eq!(state.window_count(), 1);
}

// A0 mission-reports/missions-window-enroute-day276: each title
// button's key pixel shows the window's background 11165 (grey (90, 88,
// 94) at (3, 16), (204, 16), (218, 16)), not the key's green or the
// strip: the buttons are children blitted keyed (FUN_00602d30 ->
// FUN_005fd0f0).
#[test]
fn the_title_buttons_key_pixels_show_the_windows_background() {
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    crate::fleet_window::tests::KEY_PIXELS.with(|pixels| pixels.borrow_mut().clear());
    let _ = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    let pixels = crate::fleet_window::tests::KEY_PIXELS.with(|pixels| pixels.take());
    for x in [3.0, 204.0, 218.0] {
        assert!(
            pixels.contains(&(KeyPixel::Background(BACKGROUND), at(x, 16.0))),
            "{x} in {pixels:?}"
        );
    }
}

// A0 mission-reports/missions-window-enroute-day276: under the Agents
// and Decoys tabs' key pixels (their art's bottom-left corners, window
// (105, 142) and (166, 142)) A0 shows (166, 202, 240) and (90, 88, 94):
// 11165 at (0, 15) and (61, 15), where 11165 is black at (105, 142) and
// (166, 142). FUN_0049f540 gives each tab (FUN_00602150) the window's
// surface +0xa4, and FUN_00602d30 copies its backdrop from it at the
// tab's own (x, y) in the strip (+0x10c, +0x110), not in the window.
#[test]
fn the_tabs_key_pixels_show_the_background_at_their_strip_position() {
    let (world, system, missions, _) = busy();
    let mut state = opened(&world, &missions, system);
    crate::fleet_window::tests::KEY_PIXELS.with(|pixels| pixels.borrow_mut().clear());
    let output = run(&world, &missions, &mut state, hover(at(150.0, 10.0)));
    let pixels = crate::fleet_window::tests::KEY_PIXELS.with(|pixels| pixels.take());
    // Each repaint places 11165 so its (tab x, 15) lands on the key
    // pixel: both at the strip's corner (105, 127).
    assert_eq!(
        output
            .painted
            .iter()
            .filter(|&&entry| entry == (BACKGROUND, at(105.0, 127.0)))
            .count(),
        2,
        "{:?}",
        output.painted
    );
    for (x, from_x) in [(105.0, 0), (166.0, 61)] {
        assert!(
            pixels.contains(&(KeyPixel::BackgroundAt(BACKGROUND, from_x, 15), at(x, 142.0))),
            "{x} in {pixels:?}"
        );
    }
}

#[test]
fn a_frozen_copy_lists_the_other_sides_missions_as_seen_with_their_members() {
    // FUN_004a1590 and FUN_004a1ba0 over the player's copy
    // (FUN_004f2d10, FUN_004f9a60): the Empire's Diplomacy aimed at the
    // system stays listed with its agent, as seen, after it has ended
    // (GameObj_SetReadyForDelete FUN_004f82d0 keeps the copy until the
    // side sees the system again).
    let (mut world, system) = world(ControlKind::Controlled(Faction::Empire));
    let agent = character(&mut world, system, "Agent", false);
    world.characters[agent].dat_id = DatId::new(833);
    let mut missions = MissionState::new();
    let id = send(&mut missions, MissionKind::Diplomacy, false, &[agent], &[], system, None);
    let movement = rebellion_core::movement::MovementState::default();
    let transport = rebellion_core::troop_transport::TroopTransportState::default();
    let copy = rebellion_core::object_views::SystemView::of(&world, &world.systems[system], false);
    let extras =
        rebellion_core::object_views::SystemExtras::of(&world, &movement, &missions, system, false);
    let details = rebellion_core::object_views::SystemDetails::of(
        &world, &movement, &missions, &transport, system,
    );
    let dat_id = world.systems[system].dat_id;
    world.object_views.alliance_systems.insert(dat_id, copy);
    world.object_views.alliance_extras.insert(dat_id, extras);
    world.object_views.alliance_details.insert(dat_id, details);
    world.characters[agent].current_system = None;
    let unseen = FogState::new(Faction::Alliance);
    let ended = MissionState::new();
    let rows = mission_rows(&world, &unseen, &ended, Faction::Alliance, system);
    assert_eq!(rows.iter().map(|row| (row.mission, row.side)).collect::<Vec<_>>(), [(id, 2)]);
    assert_eq!(rows[0].label, kind_name(MissionKind::Diplomacy));
    let members = member_rows(&world, &unseen, &ended, Faction::Alliance, system, id, MissionsTab::Agents);
    assert_eq!(members.iter().map(|row| row.label.as_str()).collect::<Vec<_>>(), ["Agent"]);
    assert!(member_rows(&world, &unseen, &ended, Faction::Alliance, system, id, MissionsTab::Decoys)
        .is_empty());
    // The Empire's own view holds no copy of its own mission.
    assert!(mission_rows(&world, &FogState::new(Faction::Empire), &missions, Faction::Empire, system)
        .is_empty());
    world.object_views.alliance_details.clear();
    assert!(mission_rows(&world, &unseen, &ended, Faction::Alliance, system).is_empty());
}
