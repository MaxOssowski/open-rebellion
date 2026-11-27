use super::*;

const LIST_BACKGROUND: u32 = crate::selection_list::BACKGROUND;
const LIST_STEP: f32 = crate::selection_list::STEP;

#[test]
fn a_list_items_name_breaks_into_lines_within_its_text_width() {
    // FUN_00601b80 draws an item's name with DrawTextA, DT_WORDBREAK:
    // A0 2026-11-16-partial-cells/r-3 (sha256 15244144bb9c) shows the
    // Imperial Probe Droid on the agents page as "Imperial Probe" over
    // "Droid", each centred under the mini.
    // Both lines in the regular 16-pixel cell (13 ppem), one cell
    // apart: r-3's lines start 16 rows apart, as tall and thin as
    // rescue-agents-tab-han-d763's "Han Solo" (b667575fbb6b).
    assert_eq!(crate::theme::original_font_cell(ITEM_FONT), (16.0, false));
    for (name, lines) in [
        ("Imperial Probe Droid", vec!["Imperial Probe", "Droid"]),
        ("Han Solo", vec!["Han Solo"]),
    ] {
        assert_eq!(item_name_lines(name), lines, "{name}");
    }
}
use rebellion_core::ids::SystemKey;
use rebellion_core::world::Character;

fn members(n: usize) -> Vec<MissionMember> {
    let mut world = GameWorld::default();
    (0..n)
        .map(|_| MissionMember::Character(world.characters.insert(Character::default())))
        .collect()
}

fn open_with(team: Vec<MissionMember>) -> MissionDialogState {
    let mut state = MissionDialogState::default();
    assert!(state.open(
        MissionFaction::Alliance,
        SystemKey::default(),
        team,
        vec![MissionKind::Diplomacy, MissionKind::Espionage],
    ));
    state
}

#[test]
fn the_box_lists_the_kinds_by_their_key_and_shows_the_first() {
    // A0 kinds2/50..55: Veers on Leia lists Assassination (key 11)
    // above Abduction (12) and shows Assassination.
    let mut state = MissionDialogState::default();
    assert!(state.open(
        MissionFaction::Empire,
        SystemKey::default(),
        members(1),
        box_order(vec![MissionKind::Abduction, MissionKind::Assassination]),
    ));
    let dialog = state.dialog().unwrap();
    assert_eq!(
        dialog.kinds(),
        [MissionKind::Assassination, MissionKind::Abduction]
    );
    assert_eq!(dialog.kind(), MissionKind::Assassination);
}

#[test]
fn the_box_lists_diplomacy_above_espionage_and_research_above_both() {
    // A0 2026-11-13-mission-dialogs/30..32: Dodonna on Drall lists
    // Diplomacy (FUN_0046cab0 key 7) above Espionage (9) and opens on
    // Diplomacy; 40..43: Wedge there lists Ship Design Research (4),
    // Diplomacy, Espionage. MISSNSD order is Diplomacy 0x10, Espionage
    // 0x13, research 0x20.
    assert_eq!(
        box_order(vec![MissionKind::Diplomacy, MissionKind::Espionage]),
        [MissionKind::Diplomacy, MissionKind::Espionage]
    );
    let mut state = MissionDialogState::default();
    assert!(state.open(
        MissionFaction::Alliance,
        SystemKey::default(),
        members(1),
        box_order(vec![
            MissionKind::Diplomacy,
            MissionKind::Espionage,
            MissionKind::ShipDesignResearch,
        ]),
    ));
    let dialog = state.dialog().unwrap();
    assert_eq!(
        dialog.kinds(),
        [
            MissionKind::ShipDesignResearch,
            MissionKind::Diplomacy,
            MissionKind::Espionage
        ]
    );
    assert_eq!(dialog.kind(), MissionKind::ShipDesignResearch);
}

#[test]
fn the_box_orders_every_keyed_kind_by_fun_0046cab0_and_unkeyed_after() {
    // FUN_0046cab0's keys 1..15; Palace (0x64) falls to the counter.
    let keyed = [
        MissionKind::Recruitment,
        MissionKind::JediTraining,
        MissionKind::SubdueUprising,
        MissionKind::ShipDesignResearch,
        MissionKind::FacilityDesignResearch,
        MissionKind::TroopTrainingResearch,
        MissionKind::Diplomacy,
        MissionKind::InciteUprising,
        MissionKind::Espionage,
        MissionKind::Reconnaissance,
        MissionKind::Assassination,
        MissionKind::Abduction,
        MissionKind::Rescue,
        MissionKind::DeathStarSabotage,
        MissionKind::Sabotage,
    ];
    let mut given = vec![MissionKind::Palace];
    given.extend(keyed.iter().rev());
    let mut expected = keyed.to_vec();
    expected.push(MissionKind::Palace);
    assert_eq!(box_order(given), expected);
}

#[test]
fn the_box_puts_unkeyed_families_last_in_their_given_order() {
    // FUN_0046cab0: families outside its switch take DAT_006a8794 + 1,
    // which starts at 100, so they follow the keyed kinds.
    assert_eq!(
        box_order(vec![
            MissionKind::Palace,
            MissionKind::Sabotage,
            MissionKind::Bounty,
            MissionKind::Recruitment,
        ]),
        [
            MissionKind::Recruitment,
            MissionKind::Sabotage,
            MissionKind::Palace,
            MissionKind::Bounty
        ]
    );
}

#[test]
fn a_dialog_opened_on_a_prisoner_submits_it_as_the_target_and_names_it() {
    // The order keeps the target its release set (vtable +0x2c), and
    // the first page names it under the art (37, 294).
    let mut world = GameWorld::default();
    let prisoner = world.characters.insert(Character {
        name: "Han Solo".into(),
        ..Default::default()
    });
    let target = MissionTargetPick {
        system: SystemKey::default(),
        character: Some(prisoner),
        object: None,
    };
    let team = members(1);
    let mut state = MissionDialogState::default();
    assert!(state.open(
        MissionFaction::Alliance,
        target,
        team.clone(),
        vec![MissionKind::Rescue],
    ));

    assert_eq!(
        object_target_name(&world, target).as_deref(),
        Some("Han Solo")
    );
    assert_eq!(
        object_target_name(&world, SystemKey::default().into()),
        None
    );
    assert_eq!(
        state.begin(),
        Some(MissionDialogAction::Begin {
            kind: MissionKind::Rescue,
            faction: MissionFaction::Alliance,
            team,
            decoys: Vec::new(),
            target,
        })
    );
}

#[test]
fn a_character_target_shows_its_gokres_portrait() {
    // FUN_0042c3b0(.., 1, 1) draws the portrait, GOKRES (class +0x30 &
    // 0xfff), the mini's id less 0x4000 (fleet-window.md). A0
    // 2026-11-15-partial-cells/rescue-dialog-han-on-leia-fleet90-d1428
    // (sha256 44c8c052b951) shows captive Leia Organa's in the target
    // box; Luke after Dagobah takes the Jedi Knight's 0x850.
    let mut world = GameWorld::default();
    let leia = world.characters.insert(Character {
        name: "Leia Organa".into(),
        dat_id: rebellion_core::ids::DatId::new(577),
        is_major: true,
        is_captive: true,
        ..Character::default()
    });
    let luke = world.characters.insert(Character {
        name: "Luke Skywalker".into(),
        dat_id: rebellion_core::ids::DatId::new(0x242),
        is_major: true,
        dagobah_completed: true,
        ..Character::default()
    });
    let pick = |character| MissionTargetPick {
        system: SystemKey::default(),
        character: Some(character),
        object: None,
    };
    assert_eq!(target_portrait(&world, pick(leia)), Some(2_113));
    assert_eq!(target_portrait(&world, pick(luke)), Some(0x850));
    assert_eq!(target_portrait(&world, SystemKey::default().into()), None);
}

#[test]
fn a_death_star_or_capital_ship_target_shows_its_mini_less_0x4000() {
    // FUN_0042c3b0(.., 1, 1)'s last branch: GOKRES (+0x30 & 0xfff), the
    // mini less 0x4000. A0 2026-11-16-edited-saves/x-46 (eb45cb97): the
    // Death Star target over GOKRES 1928 (mini 18312).
    let mut world = GameWorld::default();
    let death_star =
        world
            .capital_ship_classes
            .insert(rebellion_core::world::CapitalShipClass {
                dat_id: rebellion_core::ids::DatId::new(
                    rebellion_core::world::DEATH_STAR_CLASS_ID,
                ),
                ..Default::default()
            });
    let corvette = world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            dat_id: rebellion_core::ids::DatId::new(69),
            ..Default::default()
        });
    assert!(world.capital_ship_classes[death_star].is_death_star());
    let fleet = world.insert_fleet(rebellion_core::world::Fleet {
        capital_ships: vec![
            rebellion_core::world::ShipInstance::new(corvette, 100, true),
            rebellion_core::world::ShipInstance::new(death_star, 100, true),
        ],
        has_death_star: true,
        ..Default::default()
    });
    let object = |object| MissionTargetPick {
        system: SystemKey::default(),
        character: None,
        object: Some(object),
    };
    assert_eq!(
        target_portrait(&world, object(MissionTarget::DeathStar(fleet))),
        Some(1928)
    );
    assert_eq!(
        target_portrait(
            &world,
            object(MissionTarget::CapitalShip {
                fleet,
                class: corvette
            })
        ),
        Some(crate::bmp_cache::resources::gokres::MINI_SHIP_CORELLIAN_CORVETTE - 0x4000)
    );
    assert_eq!(
        target_portrait(
            &world,
            object(MissionTarget::Fighter {
                fleet,
                class: Default::default()
            })
        ),
        None
    );
}

#[test]
fn a_facility_or_death_star_target_is_named_by_its_class() {
    // FUN_00437880 names a product by its class; the Death Star hull by
    // its capital ship class.
    use rebellion_core::ids::DatId;
    let mut world = GameWorld::default();
    let yard = world.manufacturing_facilities.insert(
        rebellion_core::world::ManufacturingFacilityInstance {
            class_dat_id: DatId::new(0x2800_0001),
            side: rebellion_core::dat::Faction::Empire,
            is_shipyard: true,
        },
    );
    let mine =
        world
            .production_facilities
            .insert(rebellion_core::world::ProductionFacilityInstance {
                class_dat_id: DatId::new(0x2c00_0001),
                side: rebellion_core::dat::Faction::Empire,
                is_mine: true,
            });
    world.buildable_classes.insert(
        DatId::new(0x2800_0001),
        rebellion_core::world::BuildableClass::default(),
    );
    world.buildable_classes.insert(
        DatId::new(0x2c00_0001),
        rebellion_core::world::BuildableClass {
            name: "Mine".into(),
            ..Default::default()
        },
    );
    let class = world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            dat_id: DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
            name: "Death Star".into(),
            ..Default::default()
        });
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        capital_ships: vec![rebellion_core::world::ShipInstance::new(class, 100, true)],
        has_death_star: true,
        ..Default::default()
    });
    let name = |object| {
        object_target_name(
            &world,
            MissionTargetPick {
                system: SystemKey::default(),
                character: None,
                object: Some(object),
            },
        )
    };

    assert_eq!(
        name(MissionTarget::ProductionFacility(mine)).as_deref(),
        Some("Mine")
    );
    // A class with an empty name falls back to its mini's label.
    assert_eq!(
        name(MissionTarget::ManufacturingFacility(yard)).as_deref(),
        crate::manufacturing_window::class_mini(DatId::new(0x2800_0001))
            .map(|(_, label)| label)
    );
    assert_eq!(
        name(MissionTarget::DeathStar(fleet)).as_deref(),
        Some("Death Star")
    );
}

#[test]
fn a_team_with_no_legal_mission_opens_no_dialog() {
    // FUN_0042a320: an empty preflight list drops the order instead.
    let mut state = MissionDialogState::default();

    let opened = state.open(
        MissionFaction::Alliance,
        SystemKey::default(),
        members(1),
        vec![],
    );

    assert!(!opened);
    assert!(!state.is_open());
}

#[test]
fn the_dialog_opens_on_the_mission_page_with_the_team_as_agents_and_no_decoys() {
    // FUN_00402720 builds order 0x240 with +0x58 empty; tab 0x97 starts
    // selected (FUN_0060d700(.., 1, 0)).
    let team = members(2);
    let state = open_with(team.clone());
    let dialog = state.dialog().unwrap();

    assert_eq!(dialog.agents(), team.as_slice());
    assert!(dialog.decoys().is_empty());
    assert_eq!(dialog.page(), MissionDialogPage::Mission);
    assert_eq!(dialog.kind(), MissionKind::Diplomacy);
}

#[test]
fn selected_agents_move_to_the_end_of_the_decoys_in_order() {
    // FUN_0046c3c0 case 0xca.
    let team = members(4);
    let mut state = open_with(team.clone());
    let dialog = state.dialog_mut().unwrap();
    dialog.toggle(team[1]);
    dialog.move_selected(true);
    dialog.toggle(team[3]);
    dialog.toggle(team[0]);

    dialog.move_selected(true);

    assert_eq!(dialog.agents(), &[team[2]]);
    assert_eq!(dialog.decoys(), &[team[1], team[0], team[3]]);
}

#[test]
fn selected_decoys_move_back_to_the_agents_and_the_selection_clears() {
    // FUN_0046c3c0 case 0xcb clears each moved item's +0x3c bit 0.
    let team = members(2);
    let mut state = open_with(team.clone());
    let dialog = state.dialog_mut().unwrap();
    dialog.toggle(team[0]);
    dialog.move_selected(true);
    dialog.toggle(team[0]);

    dialog.move_selected(false);
    dialog.move_selected(true);

    assert_eq!(dialog.agents(), &[team[1], team[0]]);
    assert!(dialog.decoys().is_empty());
}

#[test]
fn a_move_takes_only_selected_members_of_its_source_list() {
    let team = members(2);
    let mut state = open_with(team.clone());
    let dialog = state.dialog_mut().unwrap();
    dialog.toggle(team[0]);

    dialog.move_selected(false);

    assert_eq!(dialog.agents(), team.as_slice());
    assert!(dialog.decoys().is_empty());
}

#[test]
fn begin_mission_submits_the_chosen_kind_with_both_lists_and_closes() {
    // Case 0x66 sets the order's +0x4c to the selected kind, submits it
    // (FUN_0041ce20), and falls through to the close.
    let team = members(2);
    let mut state = open_with(team.clone());
    let target = state.dialog().unwrap().target;
    let dialog = state.dialog_mut().unwrap();
    dialog.select_kind(1);
    dialog.toggle(team[1]);
    dialog.move_selected(true);

    let action = state.begin();

    assert_eq!(
        action,
        Some(MissionDialogAction::Begin {
            kind: MissionKind::Espionage,
            faction: MissionFaction::Alliance,
            team: vec![team[0]],
            decoys: vec![team[1]],
            target,
        })
    );
    assert!(!state.is_open());
}

/// The recovered Alliance cockpit at 640 by 480, scaled and offset.
#[test]
fn a_bitmap_wider_than_its_button_is_cropped_not_stretched() {
    // FUN_0046a9c0 makes "Cancel" 0x40 by 0x21 (64 by 33); STRATEGY 10596
    // is 66 by 33, and FUN_005fc140 blits it at its own size.
    let control = egui::Rect::from_min_size(egui::pos2(170.0, 320.0), egui::vec2(64.0, 33.0));

    let drawn = native(control, egui::vec2(66.0, 33.0), 1.0);

    assert_eq!(
        drawn,
        egui::Rect::from_min_size(control.min, egui::vec2(66.0, 33.0))
    );
    let doubled = native(control, egui::vec2(66.0, 33.0), 2.0);
    assert_eq!(doubled.size(), egui::vec2(132.0, 66.0));
}

#[test]
fn the_dialog_sits_on_whole_pixels_in_both_cockpits() {
    // port: the original blits at integer coordinates; a half-pixel
    // origin would resample every bitmap. Both galaxy views center the
    // 259 by 355 window on a half pixel at 640 by 480.
    for faction in [
        crate::cockpit::CockpitFaction::Alliance,
        crate::cockpit::CockpitFaction::Empire,
    ] {
        let layout = crate::cockpit::CockpitState::new(faction).layout_for(640.0, 480.0);

        let rect = dialog_rect(layout);

        assert_eq!(rect.min, rect.min.round(), "{faction:?}");
        assert_eq!(
            rect.size(),
            egui::vec2(MISSION_DIALOG_WIDTH, MISSION_DIALOG_HEIGHT)
        );
    }
}

fn layout(scale: f32) -> CockpitLayout {
    use crate::cockpit::CockpitViewport;
    CockpitLayout {
        canvas: CockpitViewport {
            x: 10.0,
            y: 20.0,
            width: 640.0 * scale,
            height: 480.0 * scale,
        },
        galaxy: CockpitViewport {
            x: 10.0 + 55.0 * scale,
            y: 20.0 + 40.0 * scale,
            width: 485.0 * scale,
            height: 350.0 * scale,
        },
        scale,
    }
}

#[test]
fn a_galaxy_window_is_placed_with_truncated_halves() {
    // FUN_00606980: left + (right - left - width) / 2, truncating. In a
    // 485 by 350 galaxy view at (55, 40), a 379 by 272 window sits at
    // 55 + 53 and 40 + 39; a 380 by 271 one at 55 + 52 and 40 + 39,
    // where rounding the centre gives 108 and 80.
    let rect = galaxy_centered_rect(layout(2.0), 379.0, 272.0);
    assert_eq!(rect.min, egui::pos2(10.0 + 108.0 * 2.0, 20.0 + 79.0 * 2.0));
    assert_eq!(rect.size(), egui::vec2(758.0, 544.0));
    let rect = galaxy_centered_rect(layout(1.0), 380.0, 271.0);
    assert_eq!(rect.min, egui::pos2(10.0 + 107.0, 20.0 + 79.0));
}

fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

/// Run one frame per entry of `frames` with the pointer at dialog pixel
/// `(x, y)`, after two idle frames that let the new area lay out.
fn drive(
    world: &GameWorld,
    state: &mut MissionDialogState,
    point: (f32, f32),
    frames: Vec<Vec<egui::Event>>,
) -> Option<MissionDialogAction> {
    drive_at(world, state, point, frames, 1.0)
}

/// [`drive`] at a layout `scale`.
fn drive_at(
    world: &GameWorld,
    state: &mut MissionDialogState,
    (x, y): (f32, f32),
    frames: Vec<Vec<egui::Event>>,
    scale: f32,
) -> Option<MissionDialogAction> {
    let layout = layout(scale);
    let pos = dialog_rect(layout).min + egui::vec2(x, y) * scale;
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut emitted = None;
    for extra in [vec![], vec![]].into_iter().chain(frames) {
        let mut events = vec![egui::Event::PointerMoved(pos)];
        events.extend(extra.into_iter().map(|event| match event {
            egui::Event::PointerButton { pressed, .. } => press(pos, pressed),
            other => other,
        }));
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0) * scale,
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            if let Some(action) = draw_mission_dialog(ctx, world, state, layout, &mut cache) {
                emitted = Some(action);
            }
        });
    }
    emitted
}

/// Press and release the primary button at dialog pixel `(x, y)`.
fn click(
    world: &GameWorld,
    state: &mut MissionDialogState,
    point: (f32, f32),
) -> Option<MissionDialogAction> {
    let at = egui::Pos2::ZERO;
    drive(
        world,
        state,
        point,
        vec![vec![press(at, true)], vec![press(at, false)]],
    )
}

/// Two clicks in a row at `(x, y)`, a double click.
fn double_click(world: &GameWorld, state: &mut MissionDialogState, point: (f32, f32)) {
    let at = egui::Pos2::ZERO;
    let click = [vec![press(at, true)], vec![press(at, false)]];
    drive(
        world,
        state,
        point,
        click.iter().chain(&click).cloned().collect(),
    );
}

/// One wheel notch down over `(x, y)`.
fn wheel_down(world: &GameWorld, state: &mut MissionDialogState, point: (f32, f32)) {
    let notch = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        modifiers: egui::Modifiers::default(),
    };
    drive(world, state, point, vec![vec![notch], vec![]]);
}

/// A world holding `n` characters, and a dialog open with them as agents.
fn dialog_in_world(n: usize) -> (GameWorld, Vec<MissionMember>, MissionDialogState) {
    let mut world = GameWorld::default();
    let team: Vec<MissionMember> = (0..n)
        .map(|index| {
            MissionMember::Character(world.characters.insert(Character {
                name: format!("Agent {index}"),
                ..Character::default()
            }))
        })
        .collect();
    let state = open_with(team.clone());
    (world, team, state)
}

#[test]
fn the_middle_bottom_button_begins_the_mission() {
    // FUN_0046a9c0: control 0x66 at (102, 320), 64 by 33, "Begin Mission".
    let (world, team, mut state) = dialog_in_world(1);

    let action = click(&world, &mut state, (134.0, 336.0));

    assert!(matches!(
        action,
        Some(MissionDialogAction::Begin { kind: MissionKind::Diplomacy, team: ref agents, .. })
            if *agents == team
    ));
    assert!(!state.is_open());
}

#[test]
fn the_right_bottom_button_and_the_close_box_discard_the_order() {
    // Controls 0x65 "Cancel" at (170, 320) and 0x64 at (242, 3) destroy the
    // order and close.
    for point in [(202.0, 336.0), (249.0, 10.0)] {
        let (world, _, mut state) = dialog_in_world(1);

        let action = click(&world, &mut state, point);

        assert_eq!(action, None);
        assert!(!state.is_open());
    }
}

#[test]
fn the_left_bottom_button_opens_the_encyclopedia_and_keeps_the_dialog() {
    // Control 0x67 at (33, 320) -> FUN_00429f30.
    let (world, _, mut state) = dialog_in_world(1);

    let action = click(&world, &mut state, (65.0, 336.0));

    assert_eq!(action, Some(MissionDialogAction::Encyclopedia));
    assert!(state.is_open());
}

#[test]
fn the_second_tab_shows_the_agents_page_where_a_selected_agent_becomes_a_decoy() {
    // Tab 0x98 at (7 + 130, 20); the agents list at (8, 93) with 59-pixel
    // items; arrow 0xca at (120, 136).
    let (world, team, mut state) = dialog_in_world(2);

    click(&world, &mut state, (195.0, 36.0));
    assert_eq!(state.dialog().unwrap().page(), MissionDialogPage::Agents);
    click(&world, &mut state, (61.0, 93.0 + 59.0 + 29.0));
    click(&world, &mut state, (128.0, 144.0));

    let dialog = state.dialog().unwrap();
    assert_eq!(dialog.agents(), &[team[0]]);
    assert_eq!(dialog.decoys(), &[team[1]]);
}

#[test]
fn the_dialog_stays_above_a_system_window_that_raises_itself_every_frame() {
    // FUN_0042a320 opens the dialog over the galaxy view's windows; a
    // focused system window raises itself each frame (system_window.rs).
    let (world, _, mut state) = dialog_in_world(1);
    let layout = layout(1.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let window = egui::Id::new("raising_window");
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 520.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, window));
            egui::Area::new(window)
                .order(egui::Order::Foreground)
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| {
                    ui.allocate_response(egui::vec2(700.0, 520.0), egui::Sense::click());
                });
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        });
    }

    assert_eq!(
        ctx.layer_id_at(dialog_rect(layout).center())
            .map(|layer| layer.id),
        Some(egui::Id::new("original-mission-dialog"))
    );
}

#[test]
fn the_dialog_takes_the_pointer_only_over_itself_centered_in_the_galaxy_view() {
    // hyp: FUN_00606980 places the window in the galaxy view's rectangle.
    let layout = layout(2.0);
    let (_, _, mut state) = dialog_in_world(1);
    let center = (
        10.0 + (55.0 + 485.0 / 2.0) * 2.0,
        20.0 + (40.0 + 350.0 / 2.0) * 2.0,
    );
    let (half_w, half_h) = (MISSION_DIALOG_WIDTH, MISSION_DIALOG_HEIGHT);

    assert!(state.contains_screen_point(layout, center));
    assert!(
        state.contains_screen_point(layout, (center.0 - half_w + 1.0, center.1 - half_h + 1.0))
    );
    assert!(!state.contains_screen_point(layout, (center.0 + half_w + 1.0, center.1)));
    assert!(!state.contains_screen_point(layout, (center.0, center.1 - half_h - 1.0)));
    state.close();
    assert!(!state.is_open());
    assert!(!state.contains_screen_point(layout, center));
}

#[test]
fn the_missions_button_opens_the_kind_list_and_a_click_on_it_closes_it() {
    // Control 0x68 at (101, 174) posts command 1000 to the box, which
    // opens its drop-down under the box (FUN_0060c210, FUN_0060cac0).
    let (world, _, mut state) = dialog_in_world(1);

    click(&world, &mut state, (133.0, 183.0));
    assert!(state.dialog().unwrap().list_open());
    // The drop-down is 117 pixels tall (113 * 1 + 4) from y = 175.
    click(&world, &mut state, (135.0, 290.0));

    assert!(!state.dialog().unwrap().list_open());
}

#[test]
fn the_kind_list_scrolls_by_pixels_on_its_bar_and_a_click_chooses_an_item() {
    // FUN_0060cac0: the list at (37, 177), 196 by 113, of 113-pixel
    // items, its RCDATA 0x299a bar at the right (221..233). A0 kinds/23:
    // the down arrow scrolls it by pixels, the wheel not at all.
    let (world, _, mut state) = dialog_in_world(1);
    click(&world, &mut state, (133.0, 183.0));
    assert!(state.dialog().unwrap().list_open());

    let wheel = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        modifiers: egui::Modifiers::default(),
    };
    drive(&world, &mut state, (100.0, 230.0), vec![vec![wheel]]);
    let dialog = state.dialog().unwrap();
    assert_eq!((dialog.kind(), dialog.list_scroll), (MissionKind::Diplomacy, 0.0));

    // The down arrow's last 9 rows: one step of 10, the list stays.
    click(&world, &mut state, (227.0, 286.0));
    let dialog = state.dialog().unwrap();
    assert!(dialog.list_open());
    assert_eq!(dialog.list_scroll, LIST_STEP);

    // A click 108 pixels down the list falls on the second item.
    click(&world, &mut state, (100.0, 177.0 + 108.0));
    let dialog = state.dialog().unwrap();
    assert!(!dialog.list_open());
    assert_eq!(dialog.kind(), MissionKind::Espionage);
}

#[test]
fn the_kind_list_opens_on_the_shown_kind_and_lays_its_items_113_apart() {
    // FUN_0060cac0's list at (37, 177): item i's name along its top,
    // centred in its 200-pixel surface at the list's left (A0 kinds/23
    // shows the icon at the list's x + 2 + 35); items past the list's
    // 113 rows are not drawn.
    let world = GameWorld::default();
    let mut state = MissionDialogState::default();
    let team = vec![MissionMember::Character(Default::default())];
    assert!(state.open(
        MissionFaction::Empire,
        SystemKey::default(),
        team,
        vec![MissionKind::Diplomacy, MissionKind::Espionage, MissionKind::Recruitment],
    ));
    state.dialog.as_mut().unwrap().select_kind(1);
    click(&world, &mut state, (133.0, 183.0));
    assert_eq!(state.dialog().unwrap().list_scroll, 113.0);
    state.dialog.as_mut().unwrap().list_scroll = 60.0;

    let layout = layout(1.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut output = None;
    for _ in 0..2 {
        output = Some(ctx.run(egui::RawInput::default(), |ctx| {
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        }));
    }
    let origin = dialog_rect(layout).min;
    let lines: Vec<(f32, f32, f32, String)> = {
        let mut rows: Vec<(f32, f32, f32, String)> = Vec::new();
        let mut glyphs: Vec<(egui::Pos2, String)> = output
            .unwrap()
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) if text.pos.y > origin.y + 170.0 => {
                    Some((text.pos, text.galley.text().to_owned()))
                }
                _ => None,
            })
            .collect();
        glyphs.sort_by(|a, b| (a.0.y, a.0.x).partial_cmp(&(b.0.y, b.0.x)).unwrap());
        for (pos, ch) in glyphs {
            match rows.last_mut() {
                Some(row) if (row.0 - pos.y).abs() < 1.0 => {
                    row.2 = pos.x;
                    row.3.push_str(&ch);
                }
                _ => rows.push((pos.y, pos.x, pos.x, ch)),
            }
        }
        rows
    };
    // Item 1, Espionage: its top at 177 + 113 - 60 = 230; item 2's at
    // 343 is past the list and not drawn.
    let espionage = lines
        .iter()
        .find(|row| row.3 == "Espionage")
        .unwrap_or_else(|| panic!("{lines:?}"));
    assert!(espionage.0 > origin.y + 230.0 && espionage.0 < origin.y + 230.0 + 16.0, "{espionage:?}");
    let centre = (espionage.1 + espionage.2) / 2.0;
    assert!((centre - (origin.x + 37.0 + 100.0)).abs() < 8.0, "{espionage:?}");
    assert!(!lines.iter().any(|row| row.3.contains("Recruitment")), "{lines:?}");
}

#[test]
fn the_agents_list_breaks_a_long_name_into_centred_lines_a_cell_apart() {
    // A0 2026-11-16-partial-cells/r-3 (sha256 15244144bb9c): "Imperial
    // Probe" over "Droid", the second line 16 rows (font 4's cell)
    // below the first, both centred on the 107-wide item at (8, 95);
    // drawn at scale 2.
    let mut world = GameWorld::default();
    let droid = world
        .special_forces
        .insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0005),
            is_alliance: false,
            skills: [0; 8],
            on_mission: false,
            on_hidden_mission: false,
        });
    let mut state = open_with(vec![MissionMember::SpecialForce(droid)]);
    click(&world, &mut state, (195.0, 36.0));
    let layout = layout(2.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut output = None;
    for _ in 0..2 {
        output = Some(ctx.run(egui::RawInput::default(), |ctx| {
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        }));
    }
    let lines = crate::theme::test_lines(&output.unwrap().shapes);
    let line = |text: &str| {
        lines
            .iter()
            .find(|line| line.text == text)
            .unwrap_or_else(|| panic!("{text} in {:?}", lines.iter().map(|l| &l.text).collect::<Vec<_>>()))
            .clone()
    };
    let (first, second) = (line("Imperial Probe"), line("Droid"));
    assert_eq!(second.pen.y - first.pen.y, 16.0 * 2.0);
    let centre = |line: &crate::theme::TestLine| line.pen.x + line.width / 2.0;
    assert!((centre(&first) - centre(&second)).abs() <= 3.0, "{first:?} {second:?}");
    let item = dialog_rect(layout).min.x + (8.0 + ITEM_SIZE.0 / 2.0) * 2.0;
    assert!((centre(&first) - item).abs() <= 3.0, "{first:?} {item}");
}

#[test]
fn the_kind_list_paints_black_only_under_its_tile_column() {
    // A0 kinds/31-needa-00 and 2026-11-15-partial-cells/rescue-kinds-
    // list-one-item-d763 (sha256 df49db0d184e): the white frame's right
    // column is x 233 and its bottom row y 290; right of the 195-wide
    // tile (x 230..234 outside the list and bar) the dialog's ground
    // shows, and the popup's last row y 291 is the tile's row 55
    // (black but for x 38, 39, 58, 88, 89). (Build Selection's popup is
    // black to its edges, A0 build-selection4/08.)
    let (world, _, mut state) = dialog_in_world(1);
    click(&world, &mut state, (133.0, 183.0));
    let layout = layout(1.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut output = None;
    for _ in 0..2 {
        output = Some(ctx.run(egui::RawInput::default(), |ctx| {
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        }));
    }
    let origin = dialog_rect(layout).min;
    let fills: Vec<egui::Rect> = output
        .unwrap()
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            egui::Shape::Rect(rect)
                if rect.fill.a() > 0
                    && [rect.fill.r(), rect.fill.g(), rect.fill.b()] == [0, 0, 0]
                    && rect.rect.min.y >= origin.y + 170.0 =>
            {
                Some(rect.rect.translate(-origin.to_vec2()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        [egui::Rect::from_min_size(
            egui::pos2(35.0, 175.0),
            egui::vec2(195.0, 117.0)
        )]
    );
}

#[test]
fn the_one_kind_lists_thumb_ends_a_row_above_the_track_end() {
    // FUN_0060d020 gives the bar the popup's 117-pixel page over a range
    // of 118 (FUN_00609ae0): a 94-high thumb from y 186, its bottom cap
    // at 274 (A0 2026-11-15-partial-cells/rescue-kinds-list-one-item-
    // d763, sha256 df49db0d184e, and kinds/31-needa-00: the cap's last
    // row at 279).
    let (world, team, _) = dialog_in_world(1);
    let mut state = MissionDialogState::default();
    assert!(state.open(
        MissionFaction::Alliance,
        SystemKey::default(),
        team,
        vec![MissionKind::Rescue],
    ));
    click(&world, &mut state, (133.0, 183.0));
    let layout = layout(1.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    for _ in 0..2 {
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        });
    }
    let origin = dialog_rect(layout).min;
    let art = crate::status_window::BAR_ART;
    let at = |id| {
        crate::fleet_window::tests::PAINTED.with(|painted| {
            painted
                .borrow()
                .iter()
                .filter(|(painted, _)| *painted == id)
                .map(|(_, pos)| pos.y - origin.y)
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(at(art.top), [186.0]);
    assert_eq!(at(art.bottom), [274.0]);
}

#[test]
fn the_kind_list_tiles_its_background_inside_a_white_frame() {
    // A0 kinds/31: STRATEGY 10598 (195 by 61) tiled from the window's
    // top-left (35, 175), inside a white one-pixel frame whose outer
    // edge spans (35, 175)..(234, 291); the window's own 200 by 117
    // rect is black.
    let (world, _, mut state) = dialog_in_world(1);
    click(&world, &mut state, (133.0, 183.0));
    let layout = layout(1.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut output = None;
    for _ in 0..2 {
        crate::fleet_window::tests::PAINTED.with(|painted| painted.borrow_mut().clear());
        output = Some(ctx.run(egui::RawInput::default(), |ctx| {
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        }));
    }
    let origin = dialog_rect(layout).min;
    let tiles: Vec<egui::Pos2> = crate::fleet_window::tests::PAINTED.with(|painted| {
        painted
            .borrow()
            .iter()
            .filter(|(id, _)| *id == LIST_BACKGROUND)
            .map(|(_, pos)| *pos - origin.to_vec2())
            .collect()
    });
    assert_eq!(
        tiles,
        // One column: A0 kinds/31-needa-00 and rescue-kinds-list-one-
        // item-d763 show the dialog's ground at (230..232, 176), where a
        // second column's row 1 would be black.
        [egui::pos2(35.0, 175.0), egui::pos2(35.0, 236.0)]
    );
    let frames: Vec<egui::Rect> = output
        .unwrap()
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            // White, faded in with its area (premultiplied).
            egui::Shape::Rect(rect)
                if rect.stroke.width == 1.0
                    && rect.stroke.color.a() > 0
                    && [rect.stroke.color.r(), rect.stroke.color.g(), rect.stroke.color.b()]
                        == [rect.stroke.color.a(); 3] =>
            {
                Some((rect.rect, clipped.clip_rect))
            }
            _ => None,
        })
        .map(|(rect, clip)| {
            assert_eq!(
                clip.translate(-origin.to_vec2()),
                egui::Rect::from_min_size(egui::pos2(35.0, 175.0), egui::vec2(199.0, 116.0))
            );
            rect.translate(-origin.to_vec2())
        })
        .collect();
    assert_eq!(
        frames,
        [egui::Rect::from_min_size(egui::pos2(35.5, 175.5), egui::vec2(198.0, 115.0))]
    );
}

#[test]
fn a_click_chooses_the_item_under_it_at_any_scale() {
    // The row under the pointer, in dialog pixels at twice the size.
    let (world, _, mut state) = dialog_in_world(1);
    drive_at(&world, &mut state, (133.0, 183.0), vec![vec![press(egui::Pos2::ZERO, true)], vec![press(egui::Pos2::ZERO, false)]], 2.0);
    assert!(state.dialog().unwrap().list_open());
    state.dialog.as_mut().unwrap().list_scroll = 10.0;
    drive_at(&world, &mut state, (100.0, 177.0 + 108.0), vec![vec![press(egui::Pos2::ZERO, true)], vec![press(egui::Pos2::ZERO, false)]], 2.0);
    assert_eq!(state.dialog().unwrap().kind(), MissionKind::Espionage);
}

#[test]
fn the_kind_list_names_the_shown_kind_white_and_the_others_gray() {
    // A0 kinds/31: the shown "Diplomacy" white, "Espionage" gray
    // (0x808080, the item's +0x60) under it.
    let (world, _, mut state) = dialog_in_world(1);
    click(&world, &mut state, (133.0, 183.0));
    state.dialog.as_mut().unwrap().list_scroll = 60.0;
    let layout = layout(1.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut output = None;
    for _ in 0..2 {
        output = Some(ctx.run(egui::RawInput::default(), |ctx| {
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        }));
    }
    let list = dialog_rect(layout).min + egui::vec2(37.0, 177.0);
    let colours = |word: &str| -> Vec<egui::Color32> {
        let mut glyphs: Vec<(egui::Pos2, String, egui::Color32)> = output
            .as_ref()
            .unwrap()
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.pos.y >= list.y => Some((
                    text.pos,
                    text.galley.text().to_owned(),
                    text.galley.job.sections[0].format.color,
                )),
                _ => None,
            })
            .collect();
        glyphs.sort_by(|a, b| (a.0.y, a.0.x).partial_cmp(&(b.0.y, b.0.x)).unwrap());
        let joined: String = glyphs.iter().map(|g| g.1.as_str()).collect();
        let at = joined.find(word).unwrap_or_else(|| panic!("{word} in {joined:?}"));
        glyphs[at..at + word.len()].iter().map(|g| g.2).collect()
    };
    assert!(colours("Espionage").iter().all(|c| *c == LIST_KIND_TEXT));
    assert_eq!(LIST_KIND_TEXT, egui::Color32::from_rgb(0x80, 0x80, 0x80));
}

#[test]
fn the_open_kind_list_leaves_the_bottom_buttons_free() {
    let (world, _, mut state) = dialog_in_world(1);
    click(&world, &mut state, (133.0, 183.0));

    let action = click(&world, &mut state, (134.0, 336.0));

    assert!(matches!(action, Some(MissionDialogAction::Begin { .. })));
}

#[test]
fn a_wheel_notch_scrolls_the_agents_list_by_one_item() {
    // port: the lists' scroll bar (base 0x29fc) is not drawn.
    let (world, team, mut state) = dialog_in_world(3);
    click(&world, &mut state, (195.0, 36.0));
    wheel_down(&world, &mut state, (61.0, 150.0));

    click(&world, &mut state, (61.0, 93.0 + 29.0));
    click(&world, &mut state, (128.0, 144.0));

    assert_eq!(state.dialog().unwrap().decoys(), &[team[1]]);
}

#[test]
fn a_double_click_on_an_agent_makes_it_a_decoy() {
    // FUN_0046c3c0: notification 0x309 on list 0xc8 becomes case 0xca.
    let (world, team, mut state) = dialog_in_world(2);
    click(&world, &mut state, (195.0, 36.0));

    double_click(&world, &mut state, (61.0, 93.0 + 29.0));

    let dialog = state.dialog().unwrap();
    assert_eq!(dialog.agents(), &[team[1]]);
    assert_eq!(dialog.decoys(), &[team[0]]);
}

#[test]
fn a_wheel_step_moves_one_item_and_stays_in_the_list() {
    assert_eq!(scrolled(0, 3, -1.0), 1);
    assert_eq!(scrolled(2, 3, -1.0), 2);
    assert_eq!(scrolled(2, 3, 1.0), 1);
    assert_eq!(scrolled(0, 3, 1.0), 0);
    assert_eq!(scrolled(1, 3, 0.0), 1);
}

#[test]
fn a_bitmap_is_centered_by_the_original_s_half_differences() {
    // FUN_0046a9c0: (0x6b - w) / 2, (0x3b - h) / 2 for a 107 by 59 item.
    let bounds = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(214.0, 118.0));

    let rect = centered(bounds, egui::vec2(61.0, 25.0), 2.0);

    assert_eq!(rect.min, egui::pos2(56.0, 54.0));
    assert_eq!(rect.size(), egui::vec2(122.0, 50.0));
    // C's division truncates toward zero: an 80-high portrait in the
    // 79-high target box sits at its top, (79 - 80) / 2 = 0 (A0
    // 2026-11-15-partial-cells/rescue-dialog-han-on-leia-fleet90-d1428,
    // sha256 44c8c052b951: Leia's portrait from the box's y 211).
    let target = egui::Rect::from_min_size(egui::pos2(51.0, 211.0), egui::vec2(165.0, 79.0));
    assert_eq!(
        centered(target, egui::vec2(80.0, 80.0), 1.0).min,
        egui::pos2(93.0, 211.0)
    );
    let doubled = egui::Rect::from_min_size(egui::pos2(102.0, 422.0), egui::vec2(330.0, 158.0));
    assert_eq!(
        centered(doubled, egui::vec2(80.0, 80.0), 2.0).min,
        egui::pos2(186.0, 422.0)
    );
}

#[test]
fn a_member_shows_its_system_window_mini_and_name() {
    // FUN_0042c3b0(.., 0, 1) draws GOKRES (class +0x30 & 0xfff) + 0x4000,
    // the same mini the system window lists.
    let mut world = GameWorld::default();
    let luke = world.characters.insert(Character {
        name: "Luke Skywalker".into(),
        dat_id: rebellion_core::ids::DatId::new(576),
        is_major: true,
        ..Character::default()
    });
    let spies = world
        .special_forces
        .insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: rebellion_core::ids::DatId::new(0x3c00_0004),
            is_alliance: true,
            skills: [0; 8],
            on_mission: false,
            on_hidden_mission: false,
        });

    assert_eq!(
        member_item(&world, MissionMember::Character(luke)),
        Some((Some(18_496), "Luke Skywalker".to_string()))
    );
    assert_eq!(
        member_item(&world, MissionMember::SpecialForce(spies)),
        Some((Some(17_731), "Bothan Spies".to_string()))
    );
    // FUN_0042c3b0 lays the personnel backdrop 11500 under each mini,
    // and 11502 under an injured character's (FUN_004f7800).
    let backdrop = (DllSource::Strategy, 11_500);
    assert_eq!(
        member_layers(&world, MissionMember::Character(luke), 18_496),
        [backdrop]
    );
    assert_eq!(
        member_layers(&world, MissionMember::SpecialForce(spies), 17_731),
        [backdrop]
    );
    world.characters[luke].injury = 2;
    assert_eq!(
        member_layers(&world, MissionMember::Character(luke), 18_496),
        [backdrop, (DllSource::Strategy, 11_502)]
    );
}

#[test]
fn the_kind_name_is_white_along_the_box_top() {
    // A0 menus2/11-13: the shown kind's name is white (the box's +0x94,
    // FUN_0060bed0 and FUN_0046a9c0), centered along the box's top,
    // with the icon centered below it.
    let (world, _, mut state) = dialog_in_world(1);
    let name = kind_name(state.dialog().unwrap().kind());
    let layout = layout(1.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut output = None;
    for _ in 0..2 {
        output = Some(ctx.run(egui::RawInput::default(), |ctx| {
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        }));
    }
    let origin = dialog_rect(layout).min;
    let (x, y, w, h) = KIND_BOX;
    let bounds = egui::Rect::from_min_size(origin + egui::vec2(x, y), egui::vec2(w, h));
    // paint_gdi_text lays one galley a character.
    let mut glyphs: Vec<(egui::Pos2, String, egui::Color32)> = output
        .unwrap()
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            egui::Shape::Text(text) if bounds.contains(text.pos) => Some((
                text.pos,
                text.galley.text().to_owned(),
                text.galley.job.sections[0].format.color,
            )),
            _ => None,
        })
        .collect();
    glyphs.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
    let text: String = glyphs.iter().map(|(_, ch, _)| ch.as_str()).collect();
    assert_eq!(text, name);
    assert!(glyphs.iter().all(|(_, _, color)| *color == egui::Color32::WHITE));
    // One 16-pixel line (font 4's 13 + 3) from the box's top.
    assert!(
        glyphs
            .iter()
            .all(|(pos, _, _)| pos.y >= bounds.min.y && pos.y < bounds.min.y + 16.0),
        "{glyphs:?}"
    );
    let left = glyphs[0].0.x - bounds.min.x;
    assert!((left - (w - (glyphs.last().unwrap().0.x - glyphs[0].0.x)) / 2.0).abs() < 8.0);
}

#[test]
fn the_title_and_target_label_are_in_font_5() {
    // FUN_0046a9c0: FUN_00601c60(5) before the title at (5, 2) and
    // "Target" at (37, 195); A0 menus2/11 draws the title bold, 91
    // pixels wide.
    let (world, _, mut state) = dialog_in_world(1);
    let layout = layout(1.0);
    let ctx = egui::Context::default();
    crate::theme::load_fonts(&ctx);
    let mut cache = BmpCache::new();
    let mut output = None;
    for _ in 0..2 {
        output = Some(ctx.run(egui::RawInput::default(), |ctx| {
            let _ = draw_mission_dialog(ctx, &world, &mut state, layout, &mut cache);
        }));
    }
    let origin = dialog_rect(layout).min;
    let line = |y: f32| -> (String, Vec<egui::FontFamily>) {
        let mut glyphs: Vec<(f32, String, egui::FontFamily)> = output
            .as_ref()
            .unwrap()
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text)
                    if text.pos.y >= origin.y + y && text.pos.y < origin.y + y + 16.0 =>
                {
                    Some((
                        text.pos.x,
                        text.galley.text().to_owned(),
                        text.galley.job.sections[0].format.font_id.family.clone(),
                    ))
                }
                _ => None,
            })
            .collect();
        glyphs.sort_by(|a, b| a.0.total_cmp(&b.0));
        (
            glyphs.iter().map(|(_, ch, _)| ch.as_str()).collect(),
            glyphs.into_iter().map(|(_, _, family)| family).collect(),
        )
    };
    let bold = egui::FontFamily::Name(crate::theme::ARIAL_BOLD_FAMILY.into());
    for (y, text) in [(2.0, CREATE_MISSION), (195.0, TARGET)] {
        let (drawn, families) = line(y);
        assert!(drawn.contains(text), "{drawn:?}");
        assert!(families.iter().all(|family| *family == bold), "{families:?}");
    }
}

#[test]
fn each_kind_carries_its_textstra_name() {
    // TEXTSTRA 11280..11393, the MISSNSD records' names.
    assert_eq!(kind_name(MissionKind::Diplomacy), "Diplomacy");
    assert_eq!(kind_name(MissionKind::InciteUprising), "Incite Uprising");
    assert_eq!(
        kind_name(MissionKind::DeathStarSabotage),
        "Death Star Sabotage"
    );
    assert_eq!(kind_name(MissionKind::Assassination), "Assassination");
}

#[test]
fn a_kind_past_the_list_is_not_chosen() {
    let mut state = open_with(members(1));
    let dialog = state.dialog_mut().unwrap();

    dialog.select_kind(2);

    assert_eq!(dialog.kind(), MissionKind::Diplomacy);
}

#[test]
fn each_kind_s_icon_is_its_textstra_id_s_low_bits_plus_0x1000_for_the_empire() {
    // FUN_0046a9c0: GOKRES (record +0x30 & 0xfff), + 0x1000 off side 1.
    assert_eq!(
        kind_icon(MissionKind::Diplomacy, MissionFaction::Alliance),
        3088
    );
    assert_eq!(
        kind_icon(MissionKind::Diplomacy, MissionFaction::Empire),
        7184
    );
    assert_eq!(
        kind_icon(MissionKind::SubdueUprising, MissionFaction::Alliance),
        3200
    );
    assert_eq!(
        kind_icon(MissionKind::InciteUprising, MissionFaction::Empire),
        7232
    );
}

#[test]
fn rescue_recon_research_jedi_and_death_star_kinds_show_their_textstra_icons() {
    // FUN_0046a9c0: GOKRES (MISSNSD record +0x30 & 0xfff), + 0x1000 off
    // side 1. TEXTSTRA 11281 Rescue, 11285 Reconnaissance, 11296..11298
    // the research kinds, 11329 Death Star Sabotage, 11330 Jedi Training.
    let cases = [
        (MissionKind::Rescue, 3089),
        (MissionKind::Reconnaissance, 3093),
        (MissionKind::ShipDesignResearch, 3104),
        (MissionKind::FacilityDesignResearch, 3105),
        (MissionKind::TroopTrainingResearch, 3106),
        (MissionKind::DeathStarSabotage, 3137),
        (MissionKind::JediTraining, 3138),
    ];
    for (kind, alliance) in cases {
        assert_eq!(
            kind_icon(kind, MissionFaction::Alliance),
            alliance,
            "{kind:?}"
        );
        assert_eq!(
            kind_icon(kind, MissionFaction::Empire),
            alliance + 0x1000,
            "{kind:?}"
        );
    }
    assert_eq!(kind_name(MissionKind::Reconnaissance), "Reconnaissance");
    assert_eq!(kind_name(MissionKind::Rescue), "Rescue");
    assert_eq!(kind_name(MissionKind::JediTraining), "Jedi Training");
    assert_eq!(
        kind_name(MissionKind::ShipDesignResearch),
        "Ship Design Research"
    );
    assert_eq!(
        kind_name(MissionKind::FacilityDesignResearch),
        "Facility Design Research"
    );
    assert_eq!(
        kind_name(MissionKind::TroopTrainingResearch),
        "Troop Training Research"
    );
}
