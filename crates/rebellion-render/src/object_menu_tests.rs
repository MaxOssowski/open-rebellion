use super::*;
use rebellion_core::ids::SystemKey;

const MISSION_GATE: OrderGates = OrderGates {
    mission: true,
    fleet_move: false,
    character_move: false,
    troop_move: false,
    special_force_move: false,
    squadron_move: false,
    headquarters_move: false,
    defenses_move: false,
    ship_move: false,
    rename: false,
    destination: false,
    reserved: false,
    reserved_checked: false,
    stop: false,
    build: false,
    bombard: [false; 4],
    assault: false,
    command: [false; 4],
    retire: false,
    scrap: false,
    orders_stopped: false,
    abort: false,
};

fn labels(rows: &[ObjectMenuRow]) -> Vec<&'static str> {
    rows.iter().map(|row| row.item.label).collect()
}

fn enabled(rows: &[ObjectMenuRow]) -> Vec<&'static str> {
    rows.iter()
        .filter(|row| row.enabled)
        .map(|row| row.item.label)
        .collect()
}

#[test]
fn a_missions_window_mission_offers_encyclopedia_status_and_abort() {
    // A0 missionstatus/02: Encyclopedia and Status white, Abort grey
    // until the mission may be aborted; Status opens Mission Status.
    let mission = Some(MenuObject::Mission(7));
    let rows = object_menu_rows(mission, OrderGates::default());
    assert_eq!(labels(&rows), ["Encyclopedia", "Status", "Abort"]);
    assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
    let abortable = OrderGates {
        abort: true,
        ..OrderGates::default()
    };
    assert!(enabled(&object_menu_rows(mission, abortable)).contains(&"Abort"));
    assert_eq!(
        MenuObject::Mission(7).status_object(),
        Some(StatusObject::Mission(7))
    );
}

#[test]
fn the_menu_reports_each_rows_label_and_state_in_order() {
    let selection = Some(MenuObject::Character(CharacterKey::default()));
    let rows = object_menu_rows(selection, MISSION_GATE);
    let menu = ObjectMenuState::new(selection, MISSION_GATE, (0, 0));
    assert_eq!(menu.selection(), selection);
    assert!(!menu.row_checked(ObjectMenuCommand::Move));
    let states: Vec<_> = menu.row_states().collect();
    assert_eq!(
        states,
        rows.iter()
            .map(|row| (row.item.label, row.enabled))
            .collect::<Vec<_>>()
    );
    assert!(states.iter().any(|&(_, on)| on) && states.iter().any(|&(_, on)| !on));
}

#[test]
fn a_characters_menu_matches_the_manual_and_the_strategy_records() {
    // Manual p. 99 and Fig. 3.45; STRATEGY RT_RCDATA words 2 and 5;
    // FUN_004f2400 over FUN_00536bc0.
    let rows = object_menu_rows(
        Some(MenuObject::Character(CharacterKey::default())),
        MISSION_GATE,
    );
    assert_eq!(
        labels(&rows),
        [
            "Move",
            "Confirmed Move",
            "Mission",
            "Command",
            "Encyclopedia",
            "Status",
            "Retire"
        ]
    );
    assert_eq!(
        rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
        [0x201, 0x202, 0x240, 0x160, 0x100, 0x103, 0x242]
    );
    assert_eq!(
        rows.iter()
            .map(|row| row.item.label_string_id)
            .collect::<Vec<_>>(),
        [12312, 12311, 12320, 12352, 12292, 12293, 12336]
    );
    assert_eq!(
        rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
        [10, 12, 300, 400, 1000, 1001, 2002]
    );
    // Record 0x160's word 4 marks the Command submenu parent.
    assert_eq!(
        rows.iter()
            .filter(|row| row.item.submenu)
            .map(|row| row.item.label)
            .collect::<Vec<_>>(),
        ["Command"]
    );
}

#[test]
fn a_characters_command_submenu_lists_none_admiral_general_commander() {
    // STRATEGY RT_RCDATA 0x260..0x263: parent 0x160, sort 402, 409, 406,
    // 408, TEXTSTRA 12354, 12356, 12358, 12360, bitmap 11902; each row
    // enabled by FUN_004ed750, none checked (FUN_00580d80 ->
    // FUN_0051fd30 over the fresh order's empty +0x40 list).
    let gates = OrderGates {
        command: [false, true, true, false],
        ..OrderGates::default()
    };
    let rows = object_submenu_rows(Some(MenuObject::Character(CharacterKey::default())), gates);
    assert_eq!(labels(&rows), ["None", "Admiral", "General", "Commander"]);
    assert_eq!(
        rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
        [0x260, 0x262, 0x263, 0x261]
    );
    assert_eq!(
        rows.iter()
            .map(|row| row.item.label_string_id)
            .collect::<Vec<_>>(),
        [12354, 12358, 12360, 12356]
    );
    assert_eq!(
        rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
        [402, 406, 408, 409]
    );
    assert_eq!(enabled(&rows), ["Admiral", "Commander"]);
    assert_eq!(
        rows.iter().map(|row| row.entry().icon).collect::<Vec<_>>(),
        [None; 4]
    );
    assert_eq!(
        rows.iter().map(|row| row.item.command).collect::<Vec<_>>(),
        [0x260, 0x262, 0x263, 0x261].map(ObjectMenuCommand::GiveCommand)
    );
    // A special force or a fleet has no Command rows.
    assert!(object_submenu_rows(
        Some(MenuObject::SpecialForce(SpecialForceKey::default())),
        gates
    )
    .is_empty());
    assert!(
        object_submenu_rows(Some(MenuObject::Fleet(FleetKey::default())), gates)
            .iter()
            .all(|row| !matches!(row.item.command, ObjectMenuCommand::GiveCommand(_)))
    );
}

#[test]
fn a_special_forces_menu_has_no_command_submenu() {
    // FUN_00503fa0 copies only the unit list of FUN_00536bc0.
    let rows = object_menu_rows(
        Some(MenuObject::SpecialForce(SpecialForceKey::default())),
        MISSION_GATE,
    );
    assert_eq!(
        labels(&rows),
        [
            "Move",
            "Confirmed Move",
            "Mission",
            "Encyclopedia",
            "Status",
            "Retire"
        ]
    );
}

#[test]
fn a_fleets_menu_matches_the_manual_and_the_strategy_records() {
    // Manual Fig. 3.64; FUN_004ff8e0's list; STRATEGY RT_RCDATA words 2,
    // 4 and 5 (ghidra/notes/move-order.md, "The Fleet menu").
    let rows = object_menu_rows(Some(MenuObject::Fleet(FleetKey::default())), MISSION_GATE);
    assert_eq!(
        labels(&rows),
        [
            "Move",
            "Confirmed Move",
            "Planetary Bombardment",
            "Planetary Assault",
            "Rename",
            "Encyclopedia",
            "Status",
            "Scrap"
        ]
    );
    assert_eq!(
        rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
        [0x201, 0x202, 0x120, 0x234, 0x203, 0x100, 0x103, 0x200]
    );
    assert_eq!(
        rows.iter()
            .map(|row| row.item.label_string_id)
            .collect::<Vec<_>>(),
        [12312, 12311, 12313, 12318, 12291, 12292, 12293, 12295]
    );
    assert_eq!(
        rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
        [10, 12, 200, 220, 500, 1000, 1001, 2000]
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.item.submenu)
            .map(|row| row.item.label)
            .collect::<Vec<_>>(),
        ["Planetary Bombardment"]
    );
    // A fleet is no mission member, so Mission's gate never reaches it.
    // No bombardment target is enabled, so neither is their parent
    // (A0 status/05-fleet-menu.png).
    assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
}

#[test]
fn a_fleets_move_rows_follow_its_move_rule() {
    // FUN_0053c100 -> FUN_004fdc70: each move order's +0x18 enables its
    // row; the two rows share one rule.
    let fleet = Some(MenuObject::Fleet(FleetKey::default()));
    let gates = OrderGates {
        mission: false,
        fleet_move: true,
        character_move: false,
        troop_move: true,
        special_force_move: false,
        squadron_move: true,
        headquarters_move: true,
        defenses_move: false,
        ship_move: true,
        rename: false,
        destination: false,
        reserved: false,
        reserved_checked: false,
        stop: false,
        build: false,
        bombard: [false; 4],
        assault: false,
        command: [false; 4],
        retire: false,
        scrap: false,
        orders_stopped: false,
        abort: false,
    };
    assert_eq!(
        enabled(&object_menu_rows(fleet, gates)),
        ["Move", "Confirmed Move", "Encyclopedia", "Status"]
    );
    assert_eq!(
        enabled(&object_menu_rows(fleet, OrderGates::default())),
        ["Encyclopedia", "Status"]
    );
    // A character's Move follows its own gate; Command, a submenu
    // parent, follows its rows (none enabled here).
    let character = Some(MenuObject::Character(CharacterKey::default()));
    assert_eq!(
        enabled(&object_menu_rows(character, gates)),
        ["Encyclopedia", "Status"]
    );
}

#[test]
fn a_characters_move_row_follows_its_move_rule() {
    // 0x201 and 0x202 -> 0x241 (FUN_0053c810): the rows are enabled
    // when the character resolves (FUN_004f4a00 only builds the
    // command).
    let gates = OrderGates {
        character_move: true,
        ..OrderGates::default()
    };
    let character = Some(MenuObject::Character(CharacterKey::default()));
    assert_eq!(
        enabled(&object_menu_rows(character, gates)),
        ["Move", "Confirmed Move", "Encyclopedia", "Status"]
    );
    let unit = Some(MenuObject::SpecialForce(SpecialForceKey::default()));
    assert!(!enabled(&object_menu_rows(unit, gates)).contains(&"Move"));
}

#[test]
fn a_special_forces_move_rows_follow_their_own_rule() {
    // FUN_00536bc0: a unit lists Move and Confirmed Move as a character
    // does; both expand to the Move mission 0x241.
    let gates = OrderGates {
        special_force_move: true,
        ..OrderGates::default()
    };
    let unit = Some(MenuObject::SpecialForce(SpecialForceKey::default()));
    assert_eq!(
        enabled(&object_menu_rows(unit, gates)),
        ["Move", "Confirmed Move", "Encyclopedia", "Status"]
    );
    // The unit's gate opens no character's rows.
    let character = Some(MenuObject::Character(CharacterKey::default()));
    let rows = enabled(&object_menu_rows(character, gates));
    assert!(!rows.contains(&"Move") && !rows.contains(&"Confirmed Move"));
}

#[test]
fn a_characters_retire_row_follows_its_rule() {
    // 0x242 (TEXTSTRA 12336, sort 2002): enabled through event 0x230,
    // for a character (FUN_004ed460) or a special force (FUN_00533ba0;
    // both classes list the unit orders of FUN_00536bc0); not for a
    // regiment, whose list has no 0x242.
    let gates = OrderGates {
        retire: true,
        abort: false,
        ..OrderGates::default()
    };
    let character = Some(MenuObject::Character(CharacterKey::default()));
    assert_eq!(
        enabled(&object_menu_rows(character, gates)),
        ["Encyclopedia", "Status", "Retire"]
    );
    let unit = Some(MenuObject::SpecialForce(SpecialForceKey::default()));
    assert!(enabled(&object_menu_rows(unit, gates)).contains(&"Retire"));
    let troop = Some(MenuObject::Troop(TroopKey::default()));
    // A frozen copy's unit offers only Encyclopedia and Status
    // (FUN_0051d990).
    let viewed = object_menu_rows(
        Some(MenuObject::ViewedUnit {
            system: SystemKey::default(),
            unit: crate::viewed_contents::ViewedUnit::Facility {
                class: rebellion_core::ids::DatId::new(0x2200_0001),
                owner: 2,
            },
        }),
        MISSION_GATE,
    );
    assert_eq!(
        viewed.iter().map(|row| row.item).collect::<Vec<_>>(),
        [ENCYCLOPEDIA, STATUS]
    );
    assert!(!object_menu_rows(troop, gates)
        .iter()
        .any(|row| row.item.label == "Retire"));
}

#[test]
fn rename_is_enabled_for_a_fleet_or_ship_its_rule_allows() {
    // 0x203 (TEXTSTRA 12291) on a fleet's and a ship's menu
    // (FUN_00502bd0); a character's menu does not offer it.
    let gates = OrderGates {
        rename: true,
        ..OrderGates::default()
    };
    let fleet = Some(MenuObject::Fleet(FleetKey::default()));
    let ship = Some(MenuObject::Ship {
        fleet: FleetKey::default(),
        index: 0,
        roster: 0,
    });
    for selection in [fleet, ship] {
        assert!(enabled(&object_menu_rows(selection, gates)).contains(&"Rename"));
        assert!(
            !enabled(&object_menu_rows(selection, OrderGates::default())).contains(&"Rename")
        );
    }
    let character = Some(MenuObject::Character(CharacterKey::default()));
    assert!(!enabled(&object_menu_rows(character, gates)).contains(&"Rename"));
}

#[test]
fn a_facility_icons_destination_follows_its_rule() {
    // sector-icon-menus.md: Destination (0x214, TEXTSTRA 12290) on the
    // facility icon acts on the system's production areas.
    let gates = OrderGates {
        destination: true,
        ..OrderGates::default()
    };
    assert!(enabled(&object_menu_rows(icon(Quadrant::System), gates)).contains(&"Destination"));
    assert!(!enabled(&object_menu_rows(
        icon(Quadrant::System),
        OrderGates::default()
    ))
    .contains(&"Destination"));
}

#[test]
fn the_end_of_the_game_greys_every_order() {
    // FUN_0051d990 asks FUN_0051de80 for each order: the victory
    // conditions word's bit 0x10000000. A0 pending-facility/33 (day
    // 633, the game over) greys the ships band's Build, Stop,
    // Destination, Rename and Reserved and keeps Encyclopedia and Status.
    let band = Some(MenuObject::Producer {
        system: SystemKey::default(),
        area: ProductionArea::Shipyard,
    });
    let open = OrderGates {
        destination: true,
        stop: true,
        build: true,
        reserved: true,
        ..OrderGates::default()
    };
    assert_eq!(
        enabled(&object_menu_rows(band, open)),
        ["Build", "Stop", "Destination", "Encyclopedia", "Status", "Reserved"]
    );
    let stopped = OrderGates {
        orders_stopped: true,
        ..open
    };
    assert_eq!(
        enabled(&object_menu_rows(band, stopped)),
        ["Encyclopedia", "Status"]
    );
    // The submenus' rows too.
    let fleet = Some(MenuObject::Fleet(FleetKey::default()));
    let bombard = OrderGates {
        bombard: [true; 4],
        ..OrderGates::default()
    };
    assert!(object_submenu_rows(fleet, bombard).iter().all(|row| row.enabled));
    assert!(
        object_submenu_rows(
            fleet,
            OrderGates {
                orders_stopped: true,
                ..bombard
            }
        )
        .iter()
        .all(|row| !row.enabled)
    );
    let character = Some(MenuObject::Character(CharacterKey::default()));
    let command = OrderGates {
        command: [true; 4],
        orders_stopped: true,
        ..OrderGates::default()
    };
    assert!(object_submenu_rows(character, command).iter().all(|row| !row.enabled));
}

#[test]
fn a_bands_build_stop_and_destination_follow_their_rules() {
    // manufacturing-build-selection.md, "Selection and menu": a band's
    // manager offers Build (0x210..0x212), Stop (0x213), Destination
    // (0x214) and Reserved (0x216), and for ships Rename (0x215).
    let band = |area| {
        Some(MenuObject::Producer {
            system: SystemKey::default(),
            area,
        })
    };
    let gates = OrderGates {
        destination: true,
        stop: true,
        build: true,
        ..OrderGates::default()
    };
    for area in ProductionArea::ALL {
        assert_eq!(
            enabled(&object_menu_rows(band(area), gates)),
            ["Build", "Stop", "Destination", "Encyclopedia", "Status"]
        );
        assert_eq!(
            enabled(&object_menu_rows(band(area), OrderGates::default())),
            ["Encyclopedia", "Status"]
        );
    }
    // A0 menus3/30-33: Reserved is enabled on the player's bands and
    // carries the check mark (STRATEGY 11902) while the manager is
    // Reserved; a checked row draws the mark in the icon column.
    let reserve = OrderGates {
        reserved: true,
        ..OrderGates::default()
    };
    let row = |gates: OrderGates| {
        object_menu_rows(band(ProductionArea::ConstructionYard), gates)
            .into_iter()
            .find(|row| row.item.command == ObjectMenuCommand::Reserved)
            .unwrap()
    };
    assert!(row(reserve).enabled && !row(reserve).checked);
    assert_eq!(row(reserve).entry().icon, None);
    let checked = row(OrderGates {
        reserved_checked: true,
        ..reserve
    });
    assert!(checked.checked);
    assert_eq!(checked.entry().icon, Some(11_902));
    let menu = ObjectMenuState::new(
        band(ProductionArea::ConstructionYard),
        OrderGates {
            reserved_checked: true,
            ..reserve
        },
        (0, 0),
    );
    assert!(menu.row_checked(ObjectMenuCommand::Reserved));
    assert!(!menu.row_checked(ObjectMenuCommand::Destination));
    assert!(!row(OrderGates::default()).enabled);
    // The facility icon's Reserved follows the same gates for every
    // manager of the system (FUN_00512700 kind 4 -> FUN_0052c170;
    // Event216_vt14 -> ManuMgr_vt1d4 marks it).
    let icon_reserved = |gates: OrderGates| {
        object_menu_rows(icon(Quadrant::System), gates)
            .into_iter()
            .find(|row| row.item.command == ObjectMenuCommand::Reserved)
            .unwrap()
    };
    let marked = icon_reserved(OrderGates {
        reserved_checked: true,
        ..reserve
    });
    assert!(marked.enabled && marked.checked);
    assert_eq!(marked.entry().icon, Some(11_902));
    assert!(icon_reserved(reserve).enabled && !icon_reserved(reserve).checked);
    assert!(!icon_reserved(OrderGates::default()).enabled);
    // No other icon offers it.
    assert!(object_menu_rows(icon(Quadrant::Defenses), reserve)
        .iter()
        .all(|row| row.item.command != ObjectMenuCommand::Reserved));
    // Build and Stop belong to the bands alone.
    let icon_rows = enabled(&object_menu_rows(icon(Quadrant::System), gates));
    assert!(!icon_rows.contains(&"Stop") && !icon_rows.contains(&"Build"));
}

fn icon(quadrant: Quadrant) -> Option<MenuObject> {
    Some(MenuObject::SystemIcon {
        system: SystemKey::default(),
        quadrant,
    })
}

#[test]
fn each_sector_icons_menu_lists_its_kinds_orders() {
    // FUN_0051d990 asks the system (vtable 0x0065e640, +0x3c
    // FUN_00507290) for the orders of the selection's kind;
    // FUN_0050f2e0 gives kind 4 0x200, 0x214, 0x216; kind 8 0x201, 0x202,
    // 0x200; kind 0x10 0x201, 0x202, 0x200, 0x220..0x223, 0x234; kind
    // 0x40 0x250. STRATEGY RT_RCDATA words 2 and 5 sort and name them;
    // manual Fig. 3.50 shows the mission icon's Encyclopedia, Status,
    // Abort.
    let kinds = |quadrant| -> Vec<u16> {
        object_menu_rows(icon(quadrant), OrderGates::default())
            .iter()
            .map(|row| row.item.kind)
            .collect()
    };
    assert_eq!(
        kinds(Quadrant::Fleets),
        [0x201, 0x202, 0x120, 0x234, 0x100, 0x103, 0x200]
    );
    assert_eq!(
        kinds(Quadrant::Defenses),
        [0x201, 0x202, 0x100, 0x103, 0x200]
    );
    assert_eq!(kinds(Quadrant::System), [0x214, 0x100, 0x103, 0x216, 0x200]);
    assert_eq!(kinds(Quadrant::Missions), [0x100, 0x103, 0x250]);

    let rows = object_menu_rows(icon(Quadrant::System), OrderGates::default());
    assert_eq!(
        labels(&rows),
        ["Destination", "Encyclopedia", "Status", "Reserved", "Scrap"]
    );
    assert_eq!(
        rows.iter()
            .map(|row| (row.item.sort_key, row.item.label_string_id))
            .collect::<Vec<_>>(),
        [
            (120, 12290),
            (1000, 12292),
            (1001, 12293),
            (1002, 12294),
            (2000, 12295)
        ]
    );
    let abort = object_menu_rows(icon(Quadrant::Missions), OrderGates::default())[2].item;
    assert_eq!(
        (abort.label, abort.sort_key, abort.label_string_id),
        ("Abort", 2003, 12362)
    );
    // Abort follows its rule (FUN_00521600 through event 0x261).
    let gates = OrderGates {
        abort: true,
        ..OrderGates::default()
    };
    assert_eq!(
        enabled(&object_menu_rows(icon(Quadrant::Missions), gates)),
        ["Encyclopedia", "Abort"]
    );
}

#[test]
fn a_right_release_on_a_finder_opens_encyclopedia_and_status_disabled() {
    // A0 personnel-states/03, 04: the Personnel Finder's row menu,
    // Encyclopedia and Status grey; slot +0x58 is FUN_00461740.
    let layout =
        crate::cockpit::CockpitState::new(CockpitFaction::Alliance).layout_for(640.0, 480.0);
    let release = |at: egui::Pos2, button| {
        let ctx = egui::Context::default();
        let mut opened = None;
        for events in [
            vec![egui::Event::PointerMoved(at)],
            vec![egui::Event::PointerButton {
                pos: at,
                button,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![egui::Event::PointerButton {
                pos: at,
                button,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                opened = opened.take().or_else(|| {
                    ObjectMenuState::for_finder(ctx, layout, |(x, _)| x < 300.0)
                });
            });
        }
        opened
    };
    let menu = release(egui::pos2(200.0, 343.0), egui::PointerButton::Secondary)
        .expect("the release on the Finder opens the menu");
    assert_eq!(menu.selection(), None);
    assert_eq!(labels(&menu.rows), ["Encyclopedia", "Status"]);
    assert!(enabled(&menu.rows).is_empty());
    assert_eq!(menu.point, (200.0, 343.0));
    assert_eq!(
        release(egui::pos2(400.0, 343.0), egui::PointerButton::Secondary),
        None,
        "off the Finder"
    );
    assert_eq!(
        release(egui::pos2(200.0, 343.0), egui::PointerButton::Primary),
        None,
        "a left release"
    );
}

#[test]
fn a_sector_icons_status_stays_disabled_because_its_object_is_a_system() {
    // FUN_0051d990: Encyclopedia is enabled for a single selection;
    // Status only when that object is not a system (0x90..0x97). An
    // icon's selection is its system (item +0x68, FUN_004f5b10).
    for quadrant in Quadrant::ALL {
        // With no target enabled the fleet icon's bombardment parent is
        // disabled too (A0 menus/08-fleet-icon-menu.png).
        assert_eq!(
            enabled(&object_menu_rows(icon(quadrant), OrderGates::default())),
            ["Encyclopedia"],
            "{quadrant:?}"
        );
    }
}

#[test]
fn a_fleet_icons_move_rows_follow_its_fleets_move_rule() {
    // FUN_0053c100 -> FUN_0053c4b0: the team is the system's fleets of
    // the order's side (FUN_00512700, kind 0x10); the other icons' Move
    // rows follow their own teams' rules.
    let every_gate = OrderGates {
        mission: true,
        fleet_move: true,
        character_move: true,
        troop_move: true,
        special_force_move: false,
        squadron_move: true,
        headquarters_move: true,
        defenses_move: false,
        ship_move: true,
        rename: false,
        destination: false,
        reserved: false,
        reserved_checked: false,
        stop: true,
        build: true,
        bombard: [false; 4],
        assault: false,
        command: [false; 4],
        retire: false,
        scrap: false,
        orders_stopped: false,
        abort: false,
    };
    assert_eq!(
        enabled(&object_menu_rows(icon(Quadrant::Fleets), every_gate)),
        ["Move", "Confirmed Move", "Encyclopedia"]
    );
    for quadrant in [Quadrant::System, Quadrant::Defenses, Quadrant::Missions] {
        assert_eq!(
            enabled(&object_menu_rows(icon(quadrant), every_gate)),
            ["Encyclopedia"],
            "{quadrant:?}"
        );
    }
}

#[test]
fn a_defenses_icons_move_rows_follow_its_mixed_teams_move_rule() {
    // FUN_0050f2e0 lists 0x201 and 0x202 for kind 8; FUN_0053c100 asks
    // FUN_0053c4b0 and each sub-order's +0x18 for the team FUN_00512700
    // kind 8 gives (gates.defenses_move). The other icons ignore it.
    let gates = OrderGates {
        defenses_move: true,
        ..OrderGates::default()
    };
    assert_eq!(
        enabled(&object_menu_rows(icon(Quadrant::Defenses), gates)),
        ["Move", "Confirmed Move", "Encyclopedia"]
    );
    for quadrant in [Quadrant::System, Quadrant::Fleets, Quadrant::Missions] {
        assert_eq!(
            enabled(&object_menu_rows(icon(quadrant), gates)),
            ["Encyclopedia"],
            "{quadrant:?}"
        );
    }
    // The members' own Move gates do not open the icon's rows.
    let members = OrderGates {
        troop_move: true,
        character_move: true,
        squadron_move: true,
        special_force_move: true,
        ..OrderGates::default()
    };
    assert_eq!(
        enabled(&object_menu_rows(icon(Quadrant::Defenses), members)),
        ["Encyclopedia"]
    );
}

#[test]
fn a_capital_ships_menu_offers_create_fleet_after_the_moves() {
    // FUN_00502bd0 (0x203, 0x270 after FUN_00557ce0's 0x201, 0x202,
    // 0x204, 0x200); sorted by STRATEGY record word 2 (record 624 is 50).
    let ship = Some(MenuObject::Ship {
        fleet: FleetKey::default(),
        index: 0,
        roster: 0,
    });
    let rows = object_menu_rows(ship, MISSION_GATE);
    assert_eq!(
        labels(&rows),
        [
            "Move",
            "Confirmed Move",
            "Create Fleet",
            "Rename",
            "Encyclopedia",
            "Status",
            "Scrap"
        ]
    );
    let create = rows[2].item;
    assert_eq!(
        (create.kind, create.sort_key, create.label_string_id),
        (0x270, 50, 12319)
    );
}

#[test]
fn a_menu_for_several_ships_keeps_the_team_orders_and_greys_encyclopedia_and_status() {
    // FUN_0051d990: Encyclopedia and Status only for a single
    // selection; the orders take the whole team (manual p. 120: Ctrl
    // selects ships, then Create Fleet).
    let ship = |index| MenuObject::Ship {
        fleet: FleetKey::default(),
        index,
        roster: 0,
    };
    let gates = OrderGates {
        ship_move: true,
        ..OrderGates::default()
    };
    let menu = ObjectMenuState::new(Some(ship(0)), gates, (0, 0))
        .with_team(vec![ship(0), ship(2)]);
    assert_eq!(enabled(&menu.rows), ["Move", "Confirmed Move", "Create Fleet"]);
    assert_eq!(menu.team(), [ship(0), ship(2)]);
    // Characters together keep Mission (one team, FUN_00487c50).
    let mut world = rebellion_core::world::GameWorld::default();
    let (a, b) = (
        world.characters.insert(Default::default()),
        world.characters.insert(Default::default()),
    );
    let both = ObjectMenuState::new(Some(MenuObject::Character(a)), MISSION_GATE, (0, 0))
        .with_team(vec![MenuObject::Character(a), MenuObject::Character(b)]);
    assert!(enabled(&both.rows).contains(&"Mission"), "{:?}", enabled(&both.rows));
    // One object keeps its single menu.
    let one = ObjectMenuState::new(Some(ship(0)), gates, (0, 0)).with_team(vec![ship(0)]);
    assert_eq!(
        enabled(&one.rows),
        ["Move", "Confirmed Move", "Create Fleet", "Encyclopedia", "Status"]
    );
}

#[test]
fn a_team_menu_enables_scrap_and_a_member_teams_retire_by_the_teams_gates() {
    // FUN_0051d990: each order's +0x18 runs with the whole selection as
    // its team; Scrap's (FUN_0051fe20 over FUN_005201c0's command per
    // member) and Retire's pass when every member's does.
    let ship = |index| MenuObject::Ship {
        fleet: FleetKey::default(),
        index,
        roster: 0,
    };
    let gates = OrderGates {
        ship_move: true,
        scrap: true,
        ..OrderGates::default()
    };
    let ships = ObjectMenuState::new(Some(ship(0)), gates, (0, 0))
        .with_team(vec![ship(0), ship(2)]);
    assert!(enabled(&ships.rows).contains(&"Scrap"), "{:?}", enabled(&ships.rows));
    let refused = ObjectMenuState::new(
        Some(ship(0)),
        OrderGates {
            scrap: false,
            ..gates
        },
        (0, 0),
    )
    .with_team(vec![ship(0), ship(2)]);
    assert!(!enabled(&refused.rows).contains(&"Scrap"));
    let mut world = rebellion_core::world::GameWorld::default();
    let (a, b) = (
        world.characters.insert(Default::default()),
        world.characters.insert(Default::default()),
    );
    let retire = OrderGates {
        retire: true,
        ..MISSION_GATE
    };
    let both = ObjectMenuState::new(Some(MenuObject::Character(a)), retire, (0, 0))
        .with_team(vec![MenuObject::Character(a), MenuObject::Character(b)]);
    assert!(enabled(&both.rows).contains(&"Retire"), "{:?}", enabled(&both.rows));
    // The Command orders take one object (FUN_00580db0).
    assert!(both.submenu.is_empty());
}

#[test]
fn a_capital_ships_move_and_create_fleet_follow_its_fleets_move_rule() {
    // port: its fleet's side and orbit (FUN_004f9860). A0 fleet3/02:
    // Medium Transport 1's Confirmed Move is white with its Move (it was
    // grey here before that capture).
    let ship = Some(MenuObject::Ship {
        fleet: FleetKey::default(),
        index: 1,
        roster: 0,
    });
    let gates = OrderGates {
        ship_move: true,
        ..OrderGates::default()
    };
    assert_eq!(
        enabled(&object_menu_rows(ship, gates)),
        ["Move", "Confirmed Move", "Create Fleet", "Encyclopedia", "Status"]
    );
    assert_eq!(
        enabled(&object_menu_rows(ship, OrderGates::default())),
        ["Encyclopedia", "Status"]
    );
    let fleet = Some(MenuObject::Fleet(FleetKey::default()));
    assert_eq!(
        enabled(&object_menu_rows(fleet, gates)),
        ["Encyclopedia", "Status"]
    );
}

#[test]
fn a_regiments_menu_offers_move_confirmed_move_and_scrap() {
    // FUN_00504b30: 0x201, 0x202, 0x204 (no STRATEGY record) and 0x200.
    let rows = object_menu_rows(Some(MenuObject::Troop(TroopKey::default())), MISSION_GATE);
    assert_eq!(
        labels(&rows),
        ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"]
    );
    assert_eq!(
        rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
        [0x201, 0x202, 0x100, 0x103, 0x200]
    );
}

#[test]
fn a_regiments_move_and_confirmed_move_follow_its_gate() {
    // A0 .artifacts/a0-vm/status/11-troop-menu.png: the player's
    // regiment shows Move and Confirmed Move enabled (FUN_00504b30).
    let troop = Some(MenuObject::Troop(TroopKey::default()));
    let gates = OrderGates {
        troop_move: true,
        special_force_move: false,
        squadron_move: true,
        ..OrderGates::default()
    };
    assert_eq!(
        enabled(&object_menu_rows(troop, gates)),
        ["Move", "Confirmed Move", "Encyclopedia", "Status"]
    );
    assert_eq!(
        enabled(&object_menu_rows(troop, OrderGates::default())),
        ["Encyclopedia", "Status"]
    );
    // The fleet's gate does not reach a regiment, nor the regiment's a
    // fleet.
    let fleet_gate = OrderGates {
        fleet_move: true,
        ..OrderGates::default()
    };
    assert_eq!(
        enabled(&object_menu_rows(troop, fleet_gate)),
        ["Encyclopedia", "Status"]
    );
    assert_eq!(
        enabled(&object_menu_rows(
            Some(MenuObject::Fleet(FleetKey::default())),
            gates
        )),
        ["Encyclopedia", "Status"]
    );
}

#[test]
fn only_characters_and_special_forces_are_mission_members() {
    let character = CharacterKey::default();
    let unit = SpecialForceKey::default();
    assert_eq!(
        MenuObject::Character(character).mission_member(),
        Some(MissionMember::Character(character))
    );
    assert_eq!(
        MenuObject::SpecialForce(unit).mission_member(),
        Some(MissionMember::SpecialForce(unit))
    );
    assert_eq!(
        MenuObject::Fleet(FleetKey::default()).mission_member(),
        None
    );
    assert_eq!(
        MenuObject::Troop(TroopKey::default()).mission_member(),
        None
    );
}

#[test]
fn mission_follows_the_order_rule_and_encyclopedia_needs_one_selection() {
    // FUN_0051fe20 enables Mission; FUN_0051d990 enables Encyclopedia
    // for a single selection.
    let character = Some(MenuObject::Character(CharacterKey::default()));
    assert_eq!(
        enabled(&object_menu_rows(character, MISSION_GATE)),
        ["Mission", "Encyclopedia", "Status"]
    );
    assert_eq!(
        enabled(&object_menu_rows(character, OrderGates::default())),
        ["Encyclopedia", "Status"]
    );
}

#[test]
fn status_is_enabled_for_any_single_object_that_is_not_a_system() {
    // FUN_0051d990 enables 0x103 for a single selection that is not a
    // system (ghidra/notes/status-window.md); a sector window's icon
    // stands for its system.
    let status = |selection| {
        enabled(&object_menu_rows(Some(selection), OrderGates::default())).contains(&"Status")
    };
    for selection in [
        MenuObject::Character(CharacterKey::default()),
        MenuObject::SpecialForce(SpecialForceKey::default()),
        MenuObject::Fleet(FleetKey::default()),
        MenuObject::Troop(TroopKey::default()),
        MenuObject::Ship {
            fleet: FleetKey::default(),
            index: 0,
            roster: 0,
        },
        MenuObject::Producer {
            system: SystemKey::default(),
            area: ProductionArea::Shipyard,
        },
        MenuObject::Fighter {
            fleet: FleetKey::default(),
            index: 0,
        },
        MenuObject::DefenseFacility(DefenseFacilityKey::default()),
    ] {
        assert!(status(selection), "{selection:?}");
    }
    assert!(!status(MenuObject::SystemIcon {
        system: SystemKey::default(),
        quadrant: Quadrant::Fleets,
    }));
}

#[test]
fn a_squadron_lists_move_confirmed_move_and_scrap() {
    // FUN_00502fe0 -> FUN_00557ce0: a squadron's class lists 0x201,
    // 0x202, 0x204 (no STRATEGY record) and 0x200; FUN_0051d990 adds
    // 0x100 and 0x103.
    let gates = OrderGates {
        squadron_move: true,
        ..MISSION_GATE
    };
    for selection in [
        MenuObject::Fighter {
            fleet: FleetKey::default(),
            index: 1,
        },
        MenuObject::PlanetFighter {
            system: SystemKey::default(),
            class: FighterKey::default(),
        },
    ] {
        let rows = object_menu_rows(Some(selection), gates);
        assert_eq!(
            labels(&rows),
            ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"],
            "{selection:?}"
        );
        // A0 .artifacts/a0-vm/status/20-fighter-menu.png: the player's
        // planet squadron shows Move and Confirmed Move enabled.
        assert_eq!(
            enabled(&rows),
            ["Move", "Confirmed Move", "Encyclopedia", "Status"],
            "{selection:?}"
        );
        assert_eq!(
            enabled(&object_menu_rows(Some(selection), MISSION_GATE)),
            ["Encyclopedia", "Status"],
            "{selection:?}"
        );
        assert_eq!(selection.mission_member(), None);
    }
    assert_eq!(
        MenuObject::Fighter {
            fleet: FleetKey::default(),
            index: 1,
        }
        .status_object(),
        Some(StatusObject::Fighter {
            fleet: FleetKey::default(),
            index: 1,
        })
    );
    assert_eq!(
        MenuObject::PlanetFighter {
            system: SystemKey::default(),
            class: FighterKey::default(),
        }
        .status_object(),
        Some(StatusObject::PlanetFighter {
            system: SystemKey::default(),
            class: FighterKey::default(),
        })
    );
}

#[test]
fn the_headquarters_lists_move_confirmed_move_encyclopedia_and_status_without_scrap() {
    // The HQ class's slot +0x3c (vtable 0x0065f280, FUN_00526860 ->
    // FUN_00526c00): 0x201, 0x202 and 0x204, which has no STRATEGY
    // record. FUN_0051d990 adds 0x100 and 0x103, Status enabled since
    // the object is no system. The manual (p. 134, Fig. 3.82): "drag
    // the HQ icon onto a new system or right-click, select Move".
    let hq = MenuObject::Headquarters(ManufacturingFacilityKey::default());
    let rows = object_menu_rows(Some(hq), MISSION_GATE);

    assert_eq!(
        labels(&rows),
        ["Move", "Confirmed Move", "Encyclopedia", "Status"]
    );
    assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
    // A0 headquarters/01: Move and Confirmed Move white while the HQ
    // may move (FUN_00578c00).
    let movable = OrderGates {
        headquarters_move: true,
        defenses_move: false,
        ..OrderGates::default()
    };
    assert_eq!(
        enabled(&object_menu_rows(Some(hq), movable)),
        ["Move", "Confirmed Move", "Encyclopedia", "Status"]
    );
    assert_eq!(
        hq.status_object(),
        Some(StatusObject::ManufacturingFacility(
            ManufacturingFacilityKey::default()
        ))
    );
    assert_eq!(hq.scrap_target(), None);
}

#[test]
fn an_unfinished_facility_lists_encyclopedia_status_and_a_grey_scrap() {
    // A0 pending-facility/12: a refinery under construction at Commenor
    // opens Encyclopedia, Status and a grey Scrap (FUN_005399e0's list;
    // its Scrap names no target, so the main.rs gate refuses it).
    let pending = MenuObject::PendingFacility {
        system: SystemKey::default(),
        class: rebellion_core::ids::DatId::new(0x2d00_0002),
        owner: 1,
        arrival: None,
    };
    let rows = object_menu_rows(Some(pending), MISSION_GATE);
    assert_eq!(labels(&rows), ["Encyclopedia", "Status", "Scrap"]);
    assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
    assert_eq!(pending.scrap_target(), None);
    assert_eq!(
        pending.status_object(),
        Some(StatusObject::PendingFacility {
            system: SystemKey::default(),
            class: rebellion_core::ids::DatId::new(0x2d00_0002),
            owner: 1,
            arrival: None,
        })
    );
}

#[test]
fn an_unfinished_squadron_keeps_its_move_rows_with_a_grey_scrap() {
    // A0 pending-facility/24: Pzob's A-wing under construction opens
    // Move, Confirmed Move, Encyclopedia, Status and a grey Scrap
    // (FUN_00502fe0's list); A0 2026-11-15 "u-3": the unfinished TIE's
    // Move and Confirmed Move are white. Its Move follows its gate
    // (the squadron's vt74 asks nothing of Completed).
    let pending = MenuObject::PendingFighter {
        system: SystemKey::default(),
        class: FighterKey::default(),
        product: Some(7),
    };
    let movable = OrderGates {
        squadron_move: true,
        ..MISSION_GATE
    };
    let rows = object_menu_rows(Some(pending), movable);
    assert_eq!(
        labels(&rows),
        ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"]
    );
    assert_eq!(
        enabled(&rows),
        ["Move", "Confirmed Move", "Encyclopedia", "Status"]
    );
    let shut = object_menu_rows(Some(pending), MISSION_GATE);
    assert_eq!(enabled(&shut), ["Encyclopedia", "Status"]);
    assert_eq!(pending.scrap_target(), None);
    assert_eq!(pending.mission_member(), None);
    assert_eq!(
        pending.status_object(),
        Some(StatusObject::PendingFighter {
            system: SystemKey::default(),
            class: FighterKey::default(),
        })
    );
}

#[test]
fn an_unfinished_ship_keeps_move_and_rename_and_greys_create_fleet_and_scrap() {
    // A0 2026-11-15 "u-2-carrack-unfinished-menu": Move, Confirmed
    // Move and Rename white, Create Fleet grey, Scrap grey.
    let pending = MenuObject::PendingShip { product: 4 };
    // Its Scrap names no target, so the scrap gate is shut.
    assert_eq!(pending.scrap_target(), None);
    let gates = OrderGates {
        ship_move: true,
        rename: true,
        ..MISSION_GATE
    };
    let rows = object_menu_rows(Some(pending), gates);
    assert_eq!(
        labels(&rows),
        [
            "Move",
            "Confirmed Move",
            "Create Fleet",
            "Rename",
            "Encyclopedia",
            "Status",
            "Scrap"
        ]
    );
    assert_eq!(
        enabled(&rows),
        ["Move", "Confirmed Move", "Rename", "Encyclopedia", "Status"]
    );
    assert_eq!(pending.mission_member(), None);
    assert_eq!(
        pending.status_object(),
        Some(StatusObject::PendingShip { product: 4 })
    );
}

#[test]
fn a_squadron_in_hyperspace_greys_its_move_rows_and_scrap() {
    // FUN_00502fe0's list, as for a squadron on the planet; an object in
    // hyperspace takes no order (FUN_004f9860), so Move, Confirmed Move
    // and Scrap are grey, as A0 2026-11-11-regiment-en-route/03 shows
    // them on an en route regiment.
    let travelling = MenuObject::TravellingFighter {
        to: rebellion_core::squadrons::SquadronPlace::Planet(SystemKey::default()),
        class: FighterKey::default(),
        arrival: 9,
    };
    let movable = OrderGates {
        squadron_move: true,
        ..MISSION_GATE
    };
    let rows = object_menu_rows(Some(travelling), movable);
    assert_eq!(
        labels(&rows),
        ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"]
    );
    assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
    assert_eq!(travelling.scrap_target(), None);
    assert_eq!(travelling.mission_member(), None);
    assert_eq!(
        travelling.status_object(),
        Some(StatusObject::TravellingFighter {
            to: rebellion_core::squadrons::SquadronPlace::Planet(SystemKey::default()),
            class: FighterKey::default(),
            arrival: 9,
        })
    );
}

#[test]
fn a_regiment_in_training_keeps_its_move_rows_with_a_grey_scrap() {
    // FUN_00504b30's list: Move, Confirmed Move and Scrap. Its Scrap
    // names no target, so it is grey. Its Move follows its gate, as an
    // unfinished squadron's (A0 2026-11-15 "u-3"; the regiment's vt74
    // asks nothing of Completed).
    let pending = MenuObject::PendingTroop {
        system: SystemKey::default(),
        class: rebellion_core::ids::DatId::new(0x1000_0002),
        product: Some(3),
    };
    let movable = OrderGates {
        troop_move: true,
        ..MISSION_GATE
    };
    let rows = object_menu_rows(Some(pending), movable);
    assert_eq!(
        labels(&rows),
        ["Move", "Confirmed Move", "Encyclopedia", "Status", "Scrap"]
    );
    assert_eq!(
        enabled(&rows),
        ["Move", "Confirmed Move", "Encyclopedia", "Status"]
    );
    assert_eq!(pending.scrap_target(), None);
    assert_eq!(
        pending.status_object(),
        Some(StatusObject::PendingTroop {
            system: SystemKey::default(),
            class: rebellion_core::ids::DatId::new(0x1000_0002),
        })
    );
}

#[test]
fn a_facility_lists_scrap_encyclopedia_and_status() {
    // Every facility class keeps the base order list (slot +0x3c,
    // FUN_005399e0): order 0x200, Scrap (FUN_00539c20). FUN_0051d990
    // adds 0x100 and 0x103. Scrap follows its gate
    // (`a_scrap_row_follows_its_gate_for_the_units_it_names`).
    for (selection, status) in [
        (
            MenuObject::DefenseFacility(DefenseFacilityKey::default()),
            StatusObject::DefenseFacility(DefenseFacilityKey::default()),
        ),
        (
            MenuObject::ManufacturingFacility(ManufacturingFacilityKey::default()),
            StatusObject::ManufacturingFacility(ManufacturingFacilityKey::default()),
        ),
        (
            MenuObject::ProductionFacility(ProductionFacilityKey::default()),
            StatusObject::ProductionFacility(ProductionFacilityKey::default()),
        ),
    ] {
        let rows = object_menu_rows(Some(selection), MISSION_GATE);
        assert_eq!(labels(&rows), ["Encyclopedia", "Status", "Scrap"]);
        assert_eq!(enabled(&rows), ["Encyclopedia", "Status"]);
        assert_eq!(selection.mission_member(), None);
        assert_eq!(selection.status_object(), Some(status));
    }
}

#[test]
fn a_scrap_row_follows_its_gate_for_the_units_it_names() {
    // 0x200 (TEXTSTRA 12295) is enabled by its +0x18 (the object's slot
    // +0x5c, scrap::check). A0 status/11, 20, 22 and 28 show it enabled
    // on the player's regiment, planet squadron, battery and training
    // facility; status/02 shows it disabled on an enemy ship. A0
    // menus/24-fleet1-menu.png shows it enabled on the player's fleet.
    let gates = OrderGates {
        scrap: true,
        ..OrderGates::default()
    };
    for selection in [
        MenuObject::Troop(TroopKey::default()),
        MenuObject::Ship {
            fleet: FleetKey::default(),
            index: 0,
            roster: 0,
        },
        MenuObject::PlanetFighter {
            system: SystemKey::default(),
            class: FighterKey::default(),
        },
        MenuObject::DefenseFacility(DefenseFacilityKey::default()),
        MenuObject::ManufacturingFacility(ManufacturingFacilityKey::default()),
        MenuObject::ProductionFacility(ProductionFacilityKey::default()),
        MenuObject::Fleet(FleetKey::default()),
    ] {
        assert!(selection.scrap_target().is_some(), "{selection:?}");
        assert!(
            enabled(&object_menu_rows(Some(selection), gates)).contains(&"Scrap"),
            "{selection:?}"
        );
        assert!(
            !enabled(&object_menu_rows(Some(selection), OrderGates::default()))
                .contains(&"Scrap"),
            "{selection:?}"
        );
    }
    assert_eq!(
        MenuObject::Fleet(FleetKey::default()).scrap_target(),
        Some(ScrapTarget::Fleet(FleetKey::default()))
    );
    // A0 menus3/07, 10 and 11: an icon's Scrap is enabled too; its gate
    // checks the icon's kind's objects (MenuObject::scrap_team).
    for quadrant in [Quadrant::System, Quadrant::Defenses, Quadrant::Fleets] {
        let icon = MenuObject::SystemIcon {
            system: SystemKey::default(),
            quadrant,
        };
        assert!(enabled(&object_menu_rows(Some(icon), gates)).contains(&"Scrap"));
        assert!(!enabled(&object_menu_rows(Some(icon), OrderGates::default()))
            .contains(&"Scrap"));
    }
}

#[test]
fn an_icons_scrap_team_is_its_kinds_objects_and_another_selection_its_own() {
    // FUN_00512700 by the item's kind (4, 8, 0x10, 0x40); A0 menus3/08,
    // 12 and 13.
    let mut world = rebellion_core::world::GameWorld::default();
    let system = world.systems.insert(rebellion_core::world::System {
        dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
        name: "Coruscant".into(),
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        ..Default::default()
    });
    let troop = world.troops.insert(rebellion_core::world::TroopUnit {
        class_dat_id: rebellion_core::ids::DatId::new(0x1000_0001),
        is_alliance: true,
        regiment_strength: 100,
    });
    world.systems[system].ground_units.push(troop);
    let yard = world.manufacturing_facilities.insert(
        rebellion_core::world::ManufacturingFacilityInstance {
            class_dat_id: rebellion_core::ids::DatId::new(0x2800_0001),
            side: rebellion_core::dat::Faction::Alliance,
            is_shipyard: false,
        },
    );
    world.systems[system].manufacturing_facilities.push(yard);
    let team = |quadrant| {
        MenuObject::SystemIcon { system, quadrant }
            .scrap_team(
                &rebellion_core::movement::MovementState::new(),
                &world,
                rebellion_core::dat::Faction::Alliance,
            )
    };
    assert_eq!(team(Quadrant::Defenses), [ScrapTarget::Troop(troop)]);
    assert_eq!(
        team(Quadrant::System),
        [ScrapTarget::Facility(
            rebellion_core::world::FacilityRef::Manufacturing(yard)
        )]
    );
    assert!(team(Quadrant::Fleets).is_empty());
    assert!(team(Quadrant::Missions).is_empty());
    assert_eq!(
        MenuObject::Troop(troop).scrap_team(
                &rebellion_core::movement::MovementState::new(),
                &world,
                rebellion_core::dat::Faction::Alliance,
            ),
        [ScrapTarget::Troop(troop)]
    );
    assert_eq!(
        Quadrant::ALL.map(Quadrant::kind),
        [4, 8, 0x10, 0x40]
    );
}

#[test]
fn a_spare_holding_only_an_unfinished_ship_takes_no_scrap() {
    // A0 2026-11-15 "u-2-fleet4-only-unfinished-menu": Scrap grey.
    use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, QueueItem};
    let mut world = rebellion_core::world::GameWorld::default();
    let system = world.systems.insert(rebellion_core::world::System {
        dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
        name: "Corellia".into(),
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control: rebellion_core::world::ControlKind::Controlled(
            rebellion_core::dat::Faction::Empire,
        ),
        ..Default::default()
    });
    let carrack = world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass::default());
    let mut manufacturing = ManufacturingState::new();
    manufacturing.enqueue(system, QueueItem::new(BuildableKind::CapitalShip(carrack), 5));
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    let spare = world.fleets.keys().next().unwrap();
    let movement = rebellion_core::movement::MovementState::new();
    let empire = rebellion_core::dat::Faction::Empire;
    assert!(MenuObject::Fleet(spare)
        .scrap_team(&movement, &world, empire)
        .is_empty());
    world.systems[system].fleets.push(spare);
    assert_eq!(
        MenuObject::Fleet(spare).scrap_team(&movement, &world, empire),
        [ScrapTarget::Fleet(spare)]
    );
}

#[test]
fn an_empty_selection_lists_encyclopedia_and_status_disabled() {
    // FUN_0051d990 always adds 0x100 and 0x103; neither is enabled
    // without exactly one selected object.
    let rows = object_menu_rows(None, MISSION_GATE);
    assert_eq!(labels(&rows), ["Encyclopedia", "Status"]);
    assert!(enabled(&rows).is_empty());
}

#[test]
fn a_submenu_parent_is_enabled_only_while_one_of_its_rows_is() {
    // hyp from A0: Planetary Bombardment is grey with every target
    // disabled (status/05-fleet-menu.png, menus/08-fleet-icon-menu.png)
    // and white beside an enabled one; Command likewise.
    let fleet = Some(MenuObject::Fleet(FleetKey::default()));
    let bombard = |index: usize| OrderGates {
        bombard: std::array::from_fn(|at| at == index),
        ..OrderGates::default()
    };
    for index in 0..4 {
        assert!(
            enabled(&object_menu_rows(fleet, bombard(index)))
                .contains(&"Planetary Bombardment"),
            "{index}"
        );
    }
    assert!(!enabled(&object_menu_rows(fleet, OrderGates::default()))
        .contains(&"Planetary Bombardment"));
    let character = Some(MenuObject::Character(CharacterKey::default()));
    let command = |index: usize| OrderGates {
        command: std::array::from_fn(|at| at == index),
        ..OrderGates::default()
    };
    for index in 0..4 {
        assert!(
            enabled(&object_menu_rows(character, command(index))).contains(&"Command"),
            "{index}"
        );
    }
    assert!(!enabled(&object_menu_rows(character, OrderGates::default())).contains(&"Command"));
}

#[test]
fn a_row_becomes_a_menu_entry_without_an_icon() {
    // The character records carry no icon (words 7 and 8 are zero).
    let rows = object_menu_rows(
        Some(MenuObject::Character(CharacterKey::default())),
        OrderGates {
            command: [true, false, false, false],
            ..MISSION_GATE
        },
    );
    let command = rows.iter().find(|row| row.item.label == "Command").unwrap();
    assert_eq!(
        command.entry(),
        GameMenuEntry {
            label: "Command",
            icon: None,
            enabled: true,
            submenu: true,
            check: false,
        }
    );
    let mission = rows.iter().find(|row| row.item.label == "Mission").unwrap();
    assert!(mission.entry().enabled && !mission.entry().submenu);
}

fn galaxy_layout() -> CockpitLayout {
    crate::cockpit::CockpitState::new(CockpitFaction::Alliance).layout_for(1280.0, 960.0)
}

/// Draw `menu` for two layout frames, then click the centre of row
/// `row`. Returns the choice and the menu's rectangle.
fn click_row(
    menu: &mut Option<ObjectMenuState>,
    row: usize,
) -> (Option<(ObjectMenuCommand, Option<MenuObject>)>, egui::Rect) {
    let layout = galaxy_layout();
    let ctx = egui::Context::default();
    let mut cache = BmpCache::new();
    let count = menu.as_ref().map_or(0, |open| open.rows.len());
    let mut chosen = None;
    let mut rect = egui::Rect::NOTHING;
    for frame in 0..4 {
        let mut events = Vec::new();
        if frame >= 2 {
            rect = object_menu_rect(&ctx).expect("the menu is laid out");
            let height = (rect.height() - 2.0 * layout.scale) / count as f32;
            let pos = egui::pos2(
                rect.center().x,
                rect.min.y + 2.0 * layout.scale + height * (row as f32 + 0.5),
            );
            events.push(egui::Event::PointerMoved(pos));
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: frame == 2,
                modifiers: egui::Modifiers::default(),
            });
        }
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 960.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            if let Some(choice) = draw_object_menu(
                ctx,
                menu,
                &mut cache,
                layout,
                CockpitFaction::Alliance,
                true,
            ) {
                chosen = Some(choice);
            }
        });
    }
    (chosen, rect)
}

#[test]
fn a_characters_menu_lists_mission_third_of_seven_rows() {
    // STRATEGY RT_RCDATA sort keys: Move 10, Confirmed Move 12, Mission
    // 300, Command, Encyclopedia 1000, Status 1001, Retire 2002.
    let agent = Some(MenuObject::Character(CharacterKey::default()));
    let menu = ObjectMenuState::new(agent, MISSION_GATE, (100, 100));
    assert_eq!(menu.row_count(), 7);
    assert_eq!(menu.row_of(ObjectMenuCommand::Mission), Some(2));
    assert_eq!(menu.row_of(ObjectMenuCommand::Encyclopedia), Some(4));

    // FUN_0051d990 lists only Encyclopedia and Status for no selection.
    let empty = ObjectMenuState::new(None, MISSION_GATE, (100, 100));
    assert_eq!(empty.row_of(ObjectMenuCommand::Mission), None);
}

#[test]
fn choosing_mission_returns_it_with_the_selection_and_closes_the_menu() {
    // FUN_004424c0 reports the kind to the owner's vtable +0x20
    // (FUN_004ac730), which acts on the copied selection +0x11c.
    let agent = Some(MenuObject::Character(CharacterKey::default()));
    let mut menu = Some(ObjectMenuState::new(agent, MISSION_GATE, (100, 100)));
    let (chosen, _) = click_row(&mut menu, 2);
    assert_eq!(chosen, Some((ObjectMenuCommand::Mission, agent)));
    assert_eq!(menu, None);
}

#[test]
fn a_disabled_row_keeps_the_menu_open() {
    let agent = Some(MenuObject::Character(CharacterKey::default()));
    let mut menu = Some(ObjectMenuState::new(agent, MISSION_GATE, (100, 100)));
    let (chosen, _) = click_row(&mut menu, 0);
    assert_eq!(chosen, None);
    assert!(menu.is_some());
}

#[test]
fn the_menu_opens_in_the_galaxy_view_and_flips_at_its_edges() {
    // FUN_00442860 flips at the owner's edges; the owner is the galaxy
    // view (55, 40) to (540, 390) for the Alliance (FUN_00421c70).
    let mut offset = galaxy_layout();
    offset.canvas.x += 10.0;
    offset.canvas.y += 20.0;
    offset.galaxy.x += 10.0;
    offset.galaxy.y += 20.0;
    assert_eq!(
        galaxy_owner(offset),
        CockpitViewport {
            x: 55.0,
            y: 40.0,
            width: 485.0,
            height: 350.0,
        }
    );
    let layout = galaxy_layout();
    assert_eq!(
        galaxy_owner(layout),
        CockpitViewport {
            x: 55.0,
            y: 40.0,
            width: 485.0,
            height: 350.0,
        }
    );
    let agent = Some(MenuObject::Character(CharacterKey::default()));
    let mut inside = Some(ObjectMenuState::new(agent, MISSION_GATE, (100, 100)));
    let (_, rect) = click_row(&mut inside, 0);
    assert_eq!(
        rect.min,
        egui::pos2(
            layout.canvas.x + 100.0 * layout.scale,
            layout.canvas.y + 100.0 * layout.scale
        )
    );
    // Near the galaxy view's bottom right, inside the canvas: it flips.
    let mut corner = Some(ObjectMenuState::new(agent, MISSION_GATE, (530, 380)));
    let (_, rect) = click_row(&mut corner, 0);
    assert_eq!(
        rect.max,
        egui::pos2(
            layout.canvas.x + 530.0 * layout.scale,
            layout.canvas.y + 380.0 * layout.scale
        )
    );
}

#[test]
fn planetary_assault_follows_its_gate_on_a_fleet_and_a_fleet_icon() {
    // FUN_004ff8e0 and FUN_00507290 list 0x234 (STRATEGY sort 220,
    // TEXTSTRA 12318); its +0x18 is FUN_00581ae0
    // (assault::order_enabled).
    let assault = |selection, open: bool| {
        let gates = OrderGates {
            assault: open,
            ..OrderGates::default()
        };
        object_menu_rows(selection, gates)
            .into_iter()
            .find(|row| row.item.command == ObjectMenuCommand::Assault)
            .map(|row| (row.item.kind, row.item.sort_key, row.enabled))
    };
    for selection in [
        Some(MenuObject::Fleet(FleetKey::default())),
        icon(Quadrant::Fleets),
    ] {
        assert_eq!(assault(selection, true), Some((0x234, 220, true)));
        assert_eq!(assault(selection, false), Some((0x234, 220, false)));
    }
    // No other selection offers it.
    assert_eq!(assault(icon(Quadrant::System), true), None);
    assert_eq!(assault(None, true), None);
    // An open menu reports the row's state.
    let fleet = Some(MenuObject::Fleet(FleetKey::default()));
    for open in [true, false] {
        let gates = OrderGates {
            assault: open,
            ..OrderGates::default()
        };
        let menu = ObjectMenuState::new(fleet, gates, (0, 0));
        let row = menu.row_of(ObjectMenuCommand::Assault).unwrap();
        assert_eq!(menu.row_enabled(row), open);
    }
    let menu = ObjectMenuState::new(fleet, OrderGates::default(), (0, 0));
    assert!(!menu.row_enabled(menu.row_count()));
}

#[test]
fn a_fleets_bombardment_submenu_lists_the_four_targets_by_their_gates() {
    // FUN_004ff8e0 lists 0x220..0x223; STRATEGY records 0x220..0x223
    // (parent 0x120, sort 202..208, TEXTSTRA 12314..12317); each is
    // enabled by its +0x18 (FUN_00581e80).
    let gates = OrderGates {
        bombard: [true, false, true, false],
        ..OrderGates::default()
    };
    for selection in [
        MenuObject::Fleet(FleetKey::default()),
        icon(Quadrant::Fleets).unwrap(),
    ] {
        let rows = object_submenu_rows(Some(selection), gates);
        assert_eq!(
            labels(&rows),
            [
                "Target Military Facilities",
                "Target Civilian Facilities",
                "General Bombardment",
                "Destroy System"
            ]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.kind).collect::<Vec<_>>(),
            [0x220, 0x221, 0x222, 0x223]
        );
        assert_eq!(
            rows.iter().map(|row| row.item.sort_key).collect::<Vec<_>>(),
            [202, 204, 206, 208]
        );
        assert_eq!(
            rows.iter()
                .map(|row| row.item.label_string_id)
                .collect::<Vec<_>>(),
            [12314, 12315, 12316, 12317]
        );
        assert_eq!(
            enabled(&rows),
            ["Target Military Facilities", "General Bombardment"]
        );
        assert_eq!(
            rows[3].item.command,
            ObjectMenuCommand::BombardmentTarget(0x223)
        );
    }
    // An open fleet menu finds each target's row; the parent is not in
    // the submenu, and the submenu waits for the highlight.
    let menu =
        ObjectMenuState::new(Some(MenuObject::Fleet(FleetKey::default())), gates, (0, 0));
    assert_eq!(
        menu.submenu_row_of(ObjectMenuCommand::BombardmentTarget(0x222)),
        Some(2)
    );
    assert_eq!(
        menu.submenu_row_of(ObjectMenuCommand::BombardmentTarget(0x220)),
        Some(0)
    );
    assert_eq!(menu.submenu_row_of(ObjectMenuCommand::Bombardment), None);
    assert_eq!(menu.submenu_row_count(), 4);
    // Each row keeps its gate; past the last row nothing is enabled.
    assert_eq!(
        (0..5).map(|row| menu.submenu_row_enabled(row)).collect::<Vec<_>>(),
        [true, false, true, false, false]
    );
    assert!(!menu.submenu_shown());
    // Nothing else has the submenu.
    for selection in [
        Some(MenuObject::Troop(TroopKey::default())),
        icon(Quadrant::System),
        None,
    ] {
        assert_eq!(
            labels(&object_submenu_rows(selection, gates)),
            [] as [&str; 0]
        );
    }
}
