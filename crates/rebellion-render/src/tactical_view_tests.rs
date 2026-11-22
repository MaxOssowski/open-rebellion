use super::*;

#[test]
fn hud_tooltips_follow_the_controls_state() {
    // FUN_0059b550 / FUN_0059b450: "Task Force N" once assigned, else
    // 0xad92; FUN_0059b690 disables an empty fighter group; FUN_0059b130
    // swaps Pause / Resume and Highlight / Dim (A0 TAC-02-C007, C009).
    let mut forces = [false; 8];
    forces[0] = true;
    forces[3] = true;
    let mut fighters = [false; 4];
    fighters[2] = true;
    let tip = |control, paused, lit| hud_tooltip(control, forces, fighters, paused, lit, lit);
    assert_eq!(
        tip(TacticalHudControl::TaskForce(0), false, false),
        (0xad93, true)
    );
    assert_eq!(
        tip(TacticalHudControl::TaskForce(3), false, false),
        (0xad96, true)
    );
    assert_eq!(
        tip(TacticalHudControl::TaskForce(7), false, false),
        (0xad92, true)
    );
    assert_eq!(
        tip(TacticalHudControl::FighterGroup(2), false, false),
        (0xad9f, true)
    );
    assert_eq!(
        tip(TacticalHudControl::FighterGroup(0), false, false),
        (0xad9d, false)
    );
    assert_eq!(
        tip(TacticalHudControl::NavigationSet(3), false, false),
        (0xada5, true)
    );
    assert_eq!(tip(TacticalHudControl::Pause, false, false), (0xadb7, true));
    assert_eq!(tip(TacticalHudControl::Pause, true, false), (0xadb8, true));
    assert_eq!(
        tip(TacticalHudControl::HighlightAlliance, false, false),
        (0xada6, true)
    );
    assert_eq!(
        tip(TacticalHudControl::HighlightAlliance, false, true),
        (0xada7, true)
    );
    assert_eq!(
        tip(TacticalHudControl::HighlightEmpire, false, false),
        (0xada8, true)
    );
    assert_eq!(
        tip(TacticalHudControl::HighlightEmpire, false, true),
        (0xada9, true)
    );
    // FUN_0059c650 and FUN_0059bd30.
    let camera = [
        TacticalHudControl::ZoomIn,
        TacticalHudControl::ZoomOut,
        TacticalHudControl::CameraLeft,
        TacticalHudControl::CameraRight,
        TacticalHudControl::CameraUp,
        TacticalHudControl::CameraDown,
        TacticalHudControl::CameraTarget,
        TacticalHudControl::CameraRecall,
        TacticalHudControl::CameraMemorize,
        TacticalHudControl::BattleOptions,
    ]
    .map(|control| tip(control, false, false).0);
    assert_eq!(
        camera,
        [0xadb6, 0xadb5, 0xadb4, 0xadb3, 0xadb2, 0xadb1, 0xadb0, 0xadad, 0xadab, 0xadaa]
    );
}

#[test]
fn a_battle_tip_drops_the_ampersand_of_maneuvers_and_tactics() {
    // TEXTTACT 0xddcd "Maneuvers & Tactics" on the capital-ship and
    // fighter panels' left button shows "Maneuvers  Tactics" in the
    // original's tip (A0 a0-vm/2026-11-13-tactical-panel-tips/15 and
    // 48, sha256 in that folder's SHA256SUMS): the prefix-style tooltip
    // control drops '&' (win_tooltip::shown_text).
    assert_eq!(
        tactical_tip_shown("Maneuvers & Tactics"),
        "Maneuvers  Tactics"
    );
    assert_eq!(tactical_tip_shown("Missions"), "Missions");
}

#[test]
fn panel_tooltips_follow_the_panel_and_state() {
    // FUN_005dcc70, FUN_005df110, FUN_005e4110 / FUN_005e7030.
    use TacticalCommandControl as C;
    assert_eq!(command_tooltip(C::Confirm, true, false, false), 0xddb9);
    assert_eq!(command_tooltip(C::Confirm, false, false, false), 0xdda0);
    assert_eq!(command_tooltip(C::ToggleTactic, true, false, false), 0xddcf);
    assert_eq!(command_tooltip(C::ToggleTactic, true, true, false), 0xddce);
    assert_eq!(
        command_tooltip(C::PreviousCapital, false, false, true),
        0xdd81
    );
    assert_eq!(
        command_tooltip(C::PreviousCapital, false, false, false),
        0xdd84
    );
    assert_eq!(command_tooltip(C::NextCapital, false, false, true), 0xdd82);
    assert_eq!(command_tooltip(C::NextCapital, false, false, false), 0xdd85);
    let fixed = [
        C::OpenManeuvers,
        C::OpenMissions,
        C::LeftHook,
        C::RightHook,
        C::Hammer,
        C::Anvil,
        C::HoldPosition,
        C::AttackCapitalShips,
        C::AttackFighters,
        C::Recover,
        C::AttackDeathStar,
        C::Cancel,
    ]
    .map(|control| command_tooltip(control, true, false, false));
    assert_eq!(
        fixed,
        [
            0xddcd, 0xdd8c, 0xddd0, 0xddd1, 0xddd2, 0xddd3, 0xddd4, 0xdde5, 0xddeb, 0xdde2, 0xdde6,
            0xdda1
        ]
    );
}
use rebellion_core::ids::{CapitalShipKey, DatId, FighterKey};
use rebellion_core::tactical_resources::{
    capital_ship_tactical_resource, death_star_tactical_resource, fighter_tactical_resource,
    DEATH_STAR_TACTICAL_RESOURCE,
};

fn test_ship(dat_id: u32, roster: usize, is_alliance: bool, alive: bool) -> TacticalShip {
    TacticalShip {
        class_key: CapitalShipKey::default(),
        identity: TacticalObjectIdentity {
            class_dat_id: DatId::new(dat_id),
            fleet_roster_index: roster,
            is_alliance,
        },
        tactical_resource: capital_ship_tactical_resource(DatId::new(dat_id)),
        death_star_resource: death_star_tactical_resource(DatId::new(dat_id)),
        source_position: TacticalWorldPosition::ORIGIN,
        source_forward: if is_alliance {
            TacticalWorldVector::SOURCE_FORWARD
        } else {
            TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            }
        },
        source_desired_forward: if is_alliance {
            TacticalWorldVector::SOURCE_FORWARD
        } else {
            TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            }
        },
        source_velocity: TacticalWorldVector::ZERO,
        source_waypoint: None,
        navigation_route: Vec::new(),
        source_collision_envelope: None,
        name: format!("ship-{dat_id}"),
        x: 0.0,
        y: 0.0,
        hull_current: 1,
        hull_max: 1,
        shield: 0,
        shield_max: 0,
        subsystem_condition: TacticalSubsystemCondition {
            shields: 0,
            weapons: 0,
            tractor: 0,
            engines: 0,
            hyperdrive: 0,
        },
        subsystem_capacity: TacticalSubsystemCapacity {
            shields: false,
            weapons: false,
            tractor: false,
            engines: false,
            hyperdrive: 0,
        },
        subsystem_damage: TacticalSubsystemDamage::default(),
        sublight_engine_power: 100.0,
        maneuverability: 0,
        tractor_beam_power: 25.0,
        tractor_beam_range: 0.0,
        tractor_request_pending: false,
        ion_engine_stun: 0.0,
        hangar: Vec::new(),
        hangar_launch_stamp: 0.0,
        fighter_capacity: 1,
        maneuver_state_value: None,
        damage_control: 100,
        is_attacker: is_alliance,
        alive,
        selected: false,
        order: TacticalOrder::None,
        tactic: TacticalTactic::StandOff,
        task_force: u8::try_from(roster.min(7)).unwrap_or(7),
        fleet: FleetKey::default(),
        fleet_ship_index: roster,
        sprite_id: None,
        weapon_arcs: [TacticalWeaponArc::default(); 4],
        weapon_ranges: TacticalWeaponRanges::default(),
        weapon_recharge_rate: 0.0,
        shield_recharge_rate: 0.0,
        shield_recharge_carry: 0.0,
        weapon_recharge_queue: Vec::new(),
        attack_target: None,
        manual_targets: Vec::new(),
        escort_target: None,
        contents: Vec::new(),
        retreating: false,
        retreat_progress: 0.0,
        retreated: false,
    }
}

fn test_fighter(dat_id: u32, is_alliance: bool) -> TacticalFighter {
    TacticalFighter {
        class_key: FighterKey::default(),
        identity: TacticalObjectIdentity {
            class_dat_id: DatId::new(dat_id),
            fleet_roster_index: 0,
            is_alliance,
        },
        tactical_resource: fighter_tactical_resource(DatId::new(dat_id)),
        source_position: TacticalWorldPosition::ORIGIN,
        sublight_engine_power: 10.0,
        fleet: FleetKey::default(),
        fleet_fighter_index: 0,
        fleet_squadron_index: 0,
        name: format!("fighter-{dat_id}"),
        x: 0.0,
        y: 0.0,
        squad_count: 12,
        hull_current: 96.0,
        hull_max: 96.0,
        shield: 5.0,
        shield_max: 5.0,
        weapon_arc: TacticalWeaponArc::new(5, 0, 0),
        weapon_ranges: TacticalWeaponRanges {
            laser_cannon: 12.0,
            ion_cannon: 0.0,
            turbolaser: 0.0,
        },
        torpedo_strength: 0,
        torpedo_range: 0.0,
        maneuverability: 5,
        has_hyperdrive: false,
        is_attacker: is_alliance,
        alive: true,
        fighter_group: 0,
        selected: false,
        order: TacticalOrder::None,
        tactic: TacticalTactic::StandOff,
        attack_target: None,
        escort_target: None,
        recovery_state: TacticalFighterRecoveryState::AwaitingCarrier,
        recovery_target: None,
        navigation_route: Vec::new(),
        home_carrier: None,
        in_hangar: false,
        retreating: false,
        retreat_progress: 0.0,
    }
}

fn test_session(
    ships: Vec<TacticalShip>,
    fighters: Vec<TacticalFighter>,
    player_is_attacker: bool,
) -> BattleSession {
    BattleSession {
        system: SystemKey::default(),
        system_name: "Test System".to_string(),
        system_picture_id: 1,
        attacker_fleet: FleetKey::default(),
        defender_fleet: FleetKey::default(),
        attacker_is_alliance: true,
        player_is_attacker,
        phase: BattlePhase::Combat,
        ships,
        fighters,
        death_star: None,
        death_star_beam: None,
        turn_events: Vec::new(),
        withdrawal_executors: Vec::new(),
        task_force_selection: None,
        task_force_formations: [[TacticalTactic::StandOff; 8]; 2],
        selection_state_open: false,
        engagement_bands: Vec::new(),
        force_movers: Vec::new(),
        member_states: Vec::new(),
        force_detours: Vec::new(),
        player_observing: false,
        observer_review_countdown: 0,
        commanders_regrouped: [false; 2],
        recovery_executors: Vec::new(),
        alliance_raid_sent: false,
        fighter_orbit_angles: Vec::new(),
        fighter_movers: Vec::new(),
        fighter_frames: Vec::new(),
        trench_run_outcome: None,
        trench_run_remaining_ms: None,
        trench_run_group: None,
        trench_run_success_rating: 1.0,
        alliance_commander_term: 1.0,
        trench_run_pending_outcome: None,
        trench_run_participants: Vec::new(),
        trench_run_saved: Vec::new(),
        trench_run_chatter: [ORIGINAL_TRENCH_RUN_SILENT_EVENT; 9],
        trench_run_chatter_cursor: 0,
        trench_run_cinematic_pending: false,
        navigation_sets: original_navigation_point_sets(OriginalTacticalLayout::default()),
        source_layout: OriginalTacticalLayout::default(),
        selected_ship: None,
        selected_fighter_group: None,
        placement_confirmed: true,
        start_tick: 0,
        combat_tick: 0,
        tactical_rng: OriginalTacticalRng::new(0x5eed_ba77),
        weapon_draws: 0,
        weapon_effects: Vec::new(),
        impact_effects: Vec::new(),
        pending_audio_cues: Vec::new(),
        pending_voice_cues: Vec::new(),
        voice_announcer: TacticalVoiceAnnouncer::default(),
        field_effects: Vec::new(),
        subsystem_repairs: Vec::new(),
        paused: true,
        combat_speed: 1,
        step_accumulator: 0.0,
        paused_turn_seconds: 0.0,
        end_wait_ms: None,
        winner: None,
        retreat_allowed: [false; 2],
        retreat_done: [false; 2],
        withdrawal_ms: None,
        ai_withdrawal_ms: None,
        diagnostic_log: Vec::new(),
    }
}

#[test]
fn the_battle_summary_columns_hold_operational_and_destroyed_units() {
    // FUN_004714e0: a damaged ship stays operational; only the lost
    // go to Destroyed. Personnel take three columns.
    let mut damaged = test_ship(64, 1, true, true);
    damaged.hull_current = 40;
    damaged.hull_max = 100;
    let mut lost = test_ship(64, 2, true, true);
    lost.alive = false;
    let session = test_session(vec![damaged, lost], Vec::new(), true);
    let world = GameWorld::default();
    let transport = TroopTransportState::default();
    let entries = battle_result_entries(
        &session,
        &world,
        &transport,
        true,
        BattleResultCategory::CapitalShips,
    );
    let columns = battle_result_columns(&entries, false);
    assert_eq!((columns[0].len(), columns[1].len()), (1, 1));
    assert_eq!(battle_result_columns(&entries, true).len(), 3);
    assert_eq!(battle_result_columns(&entries, true)[1].len(), 0);
    // Empty columns: the first "No Survivors" unless all are empty,
    // the middle "None", the last "No Casualties".
    assert_eq!(battle_result_empty_labels(&columns), [None, None]);
    let only_lost = vec![Vec::new(), columns[1].clone()];
    assert_eq!(
        battle_result_empty_labels(&only_lost),
        [Some("No Survivors"), None]
    );
    let none: Vec<Vec<&BattleResultEntry>> = vec![Vec::new(), Vec::new(), Vec::new()];
    assert_eq!(
        battle_result_empty_labels(&none),
        [Some("None"), Some("None"), Some("No Casualties")]
    );
}

#[test]
fn a_unit_is_destroyed_only_when_lost() {
    // FUN_004714e0: Destroyed holds the units the battle ended; every
    // other stays Operational, damaged or not.
    let mut whole = test_ship(64, 1, true, true);
    whole.hull_current = 100;
    let mut sunk = test_ship(64, 2, true, true);
    sunk.hull_current = 0;
    let lost = test_ship(64, 3, true, false);
    let enemy = test_ship(64, 4, false, true);
    let mut fighters = vec![
        test_fighter(1, true),
        test_fighter(1, true),
        test_fighter(1, true),
    ];
    fighters[1].squad_count = 0;
    fighters[2].alive = false;
    let session = test_session(vec![whole, sunk, lost, enemy], fighters, true);
    let world = GameWorld::default();
    let transport = TroopTransportState::default();
    let conditions = |category| {
        battle_result_entries(&session, &world, &transport, true, category)
            .into_iter()
            .map(|entry| entry.condition)
            .collect::<Vec<_>>()
    };
    use BattleResultCondition::{Destroyed, Operational};
    assert_eq!(
        conditions(BattleResultCategory::CapitalShips),
        [Operational, Destroyed, Destroyed]
    );
    assert_eq!(
        conditions(BattleResultCategory::Fighters),
        [Operational, Destroyed, Destroyed]
    );
    // A Death Star is lost when destroyed or out of hull.
    let mut session = session;
    let mut death_star = |destroyed, hull| {
        session.death_star = Some(rebellion_core::tactical::TacticalDeathStar {
            resource: rebellion_core::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: true,
            is_alliance: true,
            source_position: TacticalWorldPosition::ORIGIN,
            hull,
            laser_charge: 0.0,
            shielded: false,
            destroyed,
            action_committed: false,
        });
        battle_result_entries(
            &session,
            &world,
            &transport,
            true,
            BattleResultCategory::CapitalShips,
        )
        .last()
        .map(|entry| (entry.name.clone(), entry.condition))
        .unwrap()
    };
    assert_eq!(death_star(false, 5.0), ("Death Star".into(), Operational));
    assert_eq!(death_star(true, 5.0).1, Destroyed);
    assert_eq!(death_star(false, 0.0).1, Destroyed);
}

#[test]
fn a_destroyed_death_star_wears_its_damage_mark() {
    // A0 2026-11-18-edited-saves-7 d-15 (3c02b66f3450): the destroyed
    // Death Star's portrait carries the damage mark (portrait + 0x2000,
    // FUN_0042c3b0) after the trench run.
    let mut session = test_session(Vec::new(), Vec::new(), true);
    let world = GameWorld::default();
    let transport = TroopTransportState::default();
    let mut entry = |destroyed: bool| {
        session.death_star = Some(rebellion_core::tactical::TacticalDeathStar {
            resource: rebellion_core::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed,
            action_committed: false,
        });
        battle_result_entries(
            &session,
            &world,
            &transport,
            false,
            BattleResultCategory::CapitalShips,
        )
        .pop()
        .unwrap()
    };
    let lost = entry(true);
    let (source, picture) = lost.picture.expect("the Death Star has a portrait");
    assert_eq!(lost.mark, Some((source, picture + 0x2000)));
    assert_eq!(entry(false).mark, None);
}

#[test]
fn an_escaped_unit_is_not_destroyed_and_a_damaged_or_lost_one_wears_its_damage_mark() {
    // FUN_005a90c0: a unit whose jump ended has left (state 9), not been
    // destroyed (state 0xb). FUN_004724a0 -> FUN_0042c3b0(.., 1, 1)
    // puts a damaged object's (+0x50 bit 0x200) GOKRES portrait + 0x2000
    // under it: a ship's hull damage or system hits (FUN_00501c00), a
    // squadron's lost craft (FUN_00503340, slot +0x254).
    let mut world = GameWorld::default();
    let ship_class = world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            dat_id: DatId::new(0x1400_0040),
            ..Default::default()
        });
    let fighter_class = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            dat_id: DatId::new(0x1c00_0001),
            ..Default::default()
        });
    let ship = |hull: i32, alive: bool, retreated: bool, hits: u8| {
        let mut ship = test_ship(64, 1, true, alive);
        ship.class_key = ship_class;
        ship.hull_max = 10;
        ship.hull_current = hull;
        ship.retreated = retreated;
        ship.subsystem_damage.engines = hits;
        ship
    };
    let ships = vec![
        ship(10, true, false, 0),
        ship(6, true, false, 0),
        ship(10, true, false, 1),
        ship(10, false, true, 0),
        ship(4, false, true, 0),
        ship(0, false, false, 0),
    ];
    let fighter = |craft: u32, alive: bool, jumped: bool| {
        let mut fighter = test_fighter(1, true);
        fighter.class_key = fighter_class;
        fighter.hull_current = craft as f32 * 8.0;
        fighter.squad_count = craft;
        fighter.alive = alive;
        fighter.retreat_progress = if jumped { 4_000.0 } else { 0.0 };
        fighter
    };
    let fighters = vec![
        fighter(12, true, false),
        fighter(7, true, false),
        fighter(12, false, true),
        fighter(5, false, true),
        fighter(0, false, false),
        fighter(12, false, false),
    ];
    let session = test_session(ships, fighters, true);
    let transport = TroopTransportState::default();
    let of = |category| {
        battle_result_entries(&session, &world, &transport, true, category)
            .into_iter()
            .map(|entry| (entry.condition, entry.mark))
            .collect::<Vec<_>>()
    };
    use BattleResultCondition::{Destroyed, Operational};
    let portrait = crate::minis::capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap() - 0x4000;
    let ship_mark = Some((DllSource::Gokres, portrait + 0x2000));
    assert_eq!(
        of(BattleResultCategory::CapitalShips),
        [
            (Operational, None),
            (Operational, ship_mark),
            (Operational, ship_mark),
            (Operational, None),
            (Operational, ship_mark),
            (Destroyed, ship_mark),
        ]
    );
    let portrait = crate::minis::fighter_mini_id(DatId::new(0x1c00_0001)).unwrap() - 0x4000;
    let fighter_mark = Some((DllSource::Gokres, portrait + 0x2000));
    assert_eq!(
        of(BattleResultCategory::Fighters),
        [
            (Operational, None),
            (Operational, fighter_mark),
            (Operational, None),
            (Operational, fighter_mark),
            (Destroyed, fighter_mark),
            (Destroyed, fighter_mark),
        ]
    );
}

#[test]
fn a_squadron_still_in_or_back_in_its_carriers_hangar_is_not_destroyed() {
    // State 1 (in its hangar, FUN_005b69d0) and 5 (docked, FUN_005b8630)
    // are out of the fight, not destroyed (state 0xb); the commit keeps
    // them aboard. A lost carrier takes its hangar with it.
    let carrier = |alive: bool, hull: i32, retreated: bool| {
        let mut ship = test_ship(64, 1, true, alive);
        ship.hull_current = hull;
        ship.retreated = retreated;
        ship
    };
    let ships = vec![
        carrier(true, 1, false),
        carrier(false, 0, false),
        carrier(false, 1, true),
        // Lost without its hull run out (not escaped).
        carrier(false, 1, false),
    ];
    let squadron = |home: usize, hangar: bool, docked: bool| {
        let mut fighter = test_fighter(1, true);
        fighter.alive = false;
        fighter.home_carrier = Some(home);
        fighter.in_hangar = hangar;
        if docked {
            fighter.recovery_state =
                rebellion_core::tactical::TacticalFighterRecoveryState::Recovered;
        }
        fighter
    };
    let fighters = vec![
        squadron(0, true, false),
        squadron(0, false, true),
        squadron(1, true, false),
        squadron(1, false, true),
        squadron(2, true, false),
        squadron(2, false, true),
        squadron(3, true, false),
    ];
    let session = test_session(ships, fighters, true);
    let world = GameWorld::default();
    let transport = TroopTransportState::default();
    use BattleResultCondition::{Destroyed, Operational};
    assert_eq!(
        battle_result_entries(
            &session,
            &world,
            &transport,
            true,
            BattleResultCategory::Fighters
        )
        .into_iter()
        .map(|entry| entry.condition)
        .collect::<Vec<_>>(),
        [
            Operational,
            Operational,
            Destroyed,
            Destroyed,
            Operational,
            Operational,
            Destroyed
        ]
    );
}

#[test]
fn a_ships_picture_is_its_classs_portrait_and_two_rows_and_a_part_fit() {
    // FUN_0042c3b0(.., 1, 1): the mini less 0x4000; the list's 166
    // (FUN_00607ea0 at 0x47041f: 347 by 166) holds two 70-high rows
    // (FUN_004714e0's 165 by 70 cells) and the top 26 pixels of a
    // third, which the list window clips.
    let mut world = GameWorld::default();
    let class = world
        .capital_ship_classes
        .insert(rebellion_core::world::CapitalShipClass {
            dat_id: DatId::new(0x1400_0040),
            ..Default::default()
        });
    let ships: Vec<_> = (1..=3)
        .map(|roster| {
            let mut ship = test_ship(64, roster, true, true);
            ship.class_key = class;
            ship
        })
        .collect();
    let session = test_session(ships, Vec::new(), true);
    let transport = TroopTransportState::default();
    let entries = battle_result_entries(
        &session,
        &world,
        &transport,
        true,
        BattleResultCategory::CapitalShips,
    );
    let mini = crate::minis::capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap();
    assert_eq!(entries[0].picture, Some((DllSource::Gokres, mini - 0x4000)));
    assert_eq!(entries[0].backdrop, None);
    let ops = battle_force_page(
        "Hoth",
        &entries,
        true,
        BattleResultCategory::CapitalShips,
        0,
        true,
        WHITE,
        |_, _| Some([10, 10]),
    );
    let pictures = ops
        .iter()
        .filter(|op| matches!(op, PageOp::Bitmap { id, .. } if *id == mini - 0x4000))
        .count();
    assert_eq!(pictures, 3);
}

// A row without a picture keeps its name at the cell's foot strip
// (port: FUN_004724a0's text-only branch is not traced).
#[test]
fn a_row_without_a_picture_keeps_its_name_at_the_cells_foot() {
    let entries = [BattleResultEntry {
        name: "Bare".into(),
        condition: BattleResultCondition::Operational,
        picture: None,
        backdrop: None,
        mark: None,
    }];
    let ops = battle_force_page(
        "Hoth",
        &entries,
        false,
        BattleResultCategory::Troops,
        0,
        true,
        WHITE,
        |_, _| Some([10, 10]),
    );
    let baseline = ops.iter().find_map(|op| match op {
        PageOp::Text { text, baseline, .. } if text == "Bare" => Some(*baseline),
        _ => None,
    });
    assert_eq!(baseline, Some(136.0 + 70.0 - 14.0 + 7.0 + 8.0));
}

#[test]
fn the_forces_page_draws_the_originals_layout() {
    // FUN_0046f360 / FUN_00471200 / FUN_004714e0 / FUN_004724a0.
    let entry = |name: &str, condition| BattleResultEntry {
        name: name.into(),
        condition,
        picture: Some((DllSource::Gokres, 0x640)),
        backdrop: Some((DllSource::Strategy, 11_514)),
        mark: None,
    };
    let entries = [
        entry("Kept", BattleResultCondition::Operational),
        entry("Gone", BattleResultCondition::Destroyed),
        entry("Gone too", BattleResultCondition::Destroyed),
    ];
    let size = |_, id| Some(if id == 11_514 { [122, 50] } else { [100, 40] });
    let ops = battle_force_page(
        "Hoth",
        &entries,
        false,
        BattleResultCategory::Troops,
        0,
        true,
        WHITE,
        size,
    );
    let texts: Vec<(String, f32, f32, f32, bool)> = ops
        .iter()
        .filter_map(|op| match op {
            PageOp::Text {
                text,
                x,
                baseline,
                size,
                centered,
                ..
            } => Some((text.clone(), *x, *baseline, *size, *centered)),
            PageOp::Bitmap { .. } | PageOp::Wrapped { .. } | PageOp::Clip(_) => None,
        })
        .collect();
    let find = |name: &str| texts.iter().find(|entry| entry.0 == name).cloned().unwrap();
    // A0 TAC-07-C005 capital rows (window coordinates): title 22, label
    // 45, category 102, heads 121, a name 7 rows into its strip; each
    // baseline is a cap height (0.7 em) lower.
    assert_eq!(
        find("Battle at Hoth"),
        ("Battle at Hoth".into(), 212.0, 37.0, 22.0, true)
    );
    assert_eq!(
        find("Imperial Forces"),
        ("Imperial Forces".into(), 212.0, 57.0, 16.5, true)
    );
    assert_eq!(
        find("Filters"),
        ("Filters".into(), 86.0, 83.0, 13.75, false)
    );
    assert_eq!(
        find("Trooper Regiments"),
        ("Trooper Regiments".into(), 37.0, 112.0, 13.75, false)
    );
    assert_eq!(find("Operational").1, 37.0 + 82.5);
    assert_eq!(find("Destroyed").1, 204.0 + 82.5);
    assert_eq!(find("Destroyed").2, 131.0);
    // The name hangs from the picture's foot (FUN_004724a0 +0x34):
    // the 50-high backdrop from row 10 of the 70-high cell ends at 60
    // (A0 TAC-07-C007), the name's text 3 rows lower.
    assert_eq!(
        find("Kept"),
        (
            "Kept".into(),
            37.0 + 82.5,
            136.0 + 10.0 + 50.0 + 3.0 + 8.0,
            11.0,
            true
        )
    );
    assert_eq!(find("Gone too").1, 202.0 + 82.5);
    assert_eq!(find("Gone too").2, 206.0 + 10.0 + 50.0 + 3.0 + 8.0);
    let bitmaps: Vec<(u32, f32, f32)> = ops
        .iter()
        .filter_map(|op| match op {
            PageOp::Bitmap { id, x, y, .. } => Some((*id, *x, *y)),
            PageOp::Text { .. } | PageOp::Wrapped { .. } | PageOp::Clip(_) => None,
        })
        .collect();
    // The two-column art, the pressed Trooper Regiments button.
    assert_eq!(bitmaps[0], (10_761, 12.0, 13.0));
    assert!(bitmaps.contains(&(0x286f, 228.0, 59.0)));
    assert!(bitmaps.contains(&(0x286c, 130.0, 59.0)));
    // The backdrop and the picture centred in the cell (FUN_004724a0:
    // `(cell - height) / 2`; A0 TAC-07-C007 puts the 50-high picture
    // at row 10): (165 - 122) / 2 = 21, (70 - 50) / 2 = 10; (165 -
    // 100) / 2 = 32, (70 - 40) / 2 = 15.
    assert!(bitmaps.contains(&(11_514, 37.0 + 21.0, 136.0 + 10.0)));
    assert!(bitmaps.contains(&(0x640, 37.0 + 32.0, 136.0 + 15.0)));
    assert!(bitmaps.contains(&(0x640, 202.0 + 32.0, 206.0 + 15.0)));
    // Personnel: three columns and the three-column art.
    let ops = battle_force_page(
        "Hoth",
        &[],
        true,
        BattleResultCategory::Personnel,
        0,
        false,
        WHITE,
        size,
    );
    assert!(ops.contains(&PageOp::Bitmap {
        source: DllSource::Strategy,
        id: 10_762,
        x: 12.0,
        y: 13.0
    }));
    let heads: Vec<(String, f32)> = ops
        .iter()
        .filter_map(|op| match op {
            PageOp::Text { text, x, .. }
                if ["Survivors", "Captured", "Killed"].contains(&text.as_str()) =>
            {
                Some((text.clone(), *x))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        heads,
        [
            ("Survivors".into(), 92.0),
            ("Captured".into(), 204.0),
            ("Killed".into(), 315.0)
        ]
    );
    let labels: Vec<(String, f32, f32)> = ops
        .iter()
        .filter_map(|op| match op {
            PageOp::Text {
                text, x, baseline, ..
            } if ["None", "No Casualties"].contains(&text.as_str()) => {
                Some((text.clone(), *x, *baseline))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        labels,
        // The list's 110-wide cells from x 37 (FUN_004714e0), not the
        // heads' x (FUN_0046f360 places those at 37, 149, 260); font 6
        // capitals from row 149 (A0 TAC-07-C005), baseline 12 lower.
        [
            ("None".into(), 92.0, 161.0),
            ("None".into(), 202.0, 161.0),
            ("No Casualties".into(), 312.0, 161.0)
        ]
    );
    assert!(ops.iter().any(|op| matches!(op, PageOp::Text { text, color, .. } if text == "Alliance Forces" && *color == Color::from_rgba(255, 0, 0, 255))));
}

/// TEXTSTRA RCDATA 0x7002..0x700f as shipped, and the sides'
/// adjectives (0x4d70 / 0x4d71).
fn battle_texts() -> HashMap<u16, String> {
    [
        (0x7002, "The {1:4} fleet is victorious. "),
        (0x7003, "The {1:4} fleet is defeated. "),
        (0x7004, "The battle at {1:1} is indecisive. "),
        (
            0x7005,
            "{1:1} has been successfully defended from {2:4} forces. ",
        ),
        (0x7006, "{1:1} is now under blockade by {2:4} forces. "),
        (0x7007, "{1:1} has been cleared of {2:4} forces. "),
        (0x7008, "{1:1} is now under blockade by {2:4} forces. "),
        (0x7009, "The {2:4} attack on {1:1} has failed. "),
        (
            0x700a,
            "The {2:4} fleet was victorious at the neutral world {1:1}. ",
        ),
        (0x700b, "The {1:4} fleet has been completely destroyed. "),
        (0x700c, "The {1:4} fleet has withdrawn. "),
        (0x700d, "The {1:4} fleet has withdrawn to {2:1}. "),
        (0x700e, "There has been no victor.  "),
        (0x700f, "All {1:4} and {2:4} ships have been destroyed. "),
        (
            0x7015,
            "{2:4} forces have maintained the blockade of {1:1}. ",
        ),
        (0x7016, "{2:4} forces have broken the blockade of {1:1}. "),
        (0x4d70, "Alliance"),
        (0x4d71, "Imperial"),
    ]
    .into_iter()
    .map(|(id, text)| (id, text.to_owned()))
    .collect()
}

#[test]
fn the_session_gives_the_report_its_withdrawn_side_and_system() {
    // FUN_0049b060: +0x54 is the side whose fleet withdrew; the system's
    // side bits pick the sentence (port: from the world's control).
    let mut fled = test_ship(64, 1, true, true);
    fled.retreated = true;
    let stayed = test_ship(64, 2, false, true);
    let mut session = test_session(vec![fled, stayed], Vec::new(), true);
    session.winner = Some(CombatWinner::Defender);
    let world = GameWorld::default();
    let lines = battle_result_summary_lines(&session, &world);
    // The texts are not loaded in tests: the port's English names the
    // withdrawal.
    assert!(
        lines.iter().any(|line| line.contains("withdrawn")),
        "{lines:?}"
    );
    session.ships[0].retreated = false;
    session.ships[1].retreated = true;
    let lines = battle_result_summary_lines(&session, &world);
    assert!(
        lines.iter().any(|line| line.contains("withdrawn")),
        "{lines:?}"
    );
    session.ships[1].retreated = false;
    let lines = battle_result_summary_lines(&session, &world);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("completely destroyed")),
        "{lines:?}"
    );
}

#[test]
fn the_app_hands_the_report_its_texts() {
    // set_battle_texts: the report then reads TEXTSTRA. With no winner
    // the withdrawn side matters: here the Alliance's (the viewer's,
    // as the attacker) withdrew (FUN_0049b150).
    set_battle_texts(battle_texts());
    let mut fled = test_ship(64, 1, true, true);
    fled.retreated = true;
    let session = test_session(vec![fled, test_ship(64, 2, false, true)], Vec::new(), true);
    let lines = battle_result_summary_lines(&session, &GameWorld::default());
    assert_eq!(lines[0], "The battle at Test System is indecisive. ");
    assert_eq!(
        lines[1],
        "There has been no victor.  The Alliance fleet has withdrawn. The Imperial fleet has been completely destroyed."
    );
}

#[test]
fn the_battle_report_speaks_for_the_viewer() {
    // FUN_0049b0f0: the viewer's win (FUN_0049b330), loss
    // (FUN_0049b4d0) or no winner (FUN_0049b150); guide-059 reads "The
    // Imperial fleet is victorious." then "Deyer is now under blockade
    // by Imperial forces. The Alliance fleet has been completely
    // destroyed."
    let texts = battle_texts();
    let report = |viewer, winner, withdrawn, system_side| {
        battle_report_lines(
            &texts,
            &BattleReport {
                system_name: "Deyer",
                viewer_alliance: viewer,
                winner_alliance: winner,
                withdrawn,
                destination: None,
                last_arrival: None,
                system_side,
            },
        )
    };
    assert_eq!(
        report(false, Some(false), None, Some(true)),
        [
            "The Imperial fleet is victorious. ",
            "Deyer is now under blockade by Imperial forces. The Alliance fleet has been completely destroyed."
        ]
    );
    // The same battle for the Alliance viewer.
    assert_eq!(
        report(true, Some(false), None, Some(true)),
        [
            "The Alliance fleet is defeated. ",
            "Deyer is now under blockade by Imperial forces. The Alliance fleet has been completely destroyed."
        ]
    );
    // A win at home: defended; at a neutral world: cleared; the loser
    // withdrew.
    assert_eq!(
        report(true, Some(true), Some(false), Some(true))[1],
        "Deyer has been successfully defended from Imperial forces. The Imperial fleet has withdrawn."
    );
    assert_eq!(
        report(true, Some(true), None, None)[1],
        "Deyer has been cleared of Imperial forces. The Imperial fleet has been completely destroyed."
    );
    // A loss: a failed attack on the enemy's system, at a neutral world.
    assert_eq!(
        report(true, Some(false), Some(true), Some(false))[1],
        "The Alliance attack on Deyer has failed. The Alliance fleet has withdrawn."
    );
    assert_eq!(
        report(true, Some(false), None, None)[1],
        "The Imperial fleet was victorious at the neutral world Deyer. The Alliance fleet has been completely destroyed."
    );
    // No winner.
    assert_eq!(
        report(true, None, None, Some(true)),
        [
            "The battle at Deyer is indecisive. ",
            "There has been no victor.  All Alliance and Imperial ships have been destroyed."
        ]
    );
    assert_eq!(
        report(true, None, Some(true), Some(true))[1],
        "There has been no victor.  The Alliance fleet has withdrawn. The Imperial fleet has been completely destroyed."
    );
    assert_eq!(
        report(true, None, Some(false), Some(true))[1],
        "There has been no victor.  The Alliance fleet has been completely destroyed. The Imperial fleet has withdrawn."
    );
    // Without the texts, the port's English.
    let empty = HashMap::new();
    let fallback = battle_report_lines(
        &empty,
        &BattleReport {
            system_name: "Deyer",
            viewer_alliance: true,
            winner_alliance: Some(false),
            withdrawn: Some(true),
            destination: None,
            last_arrival: None,
            system_side: None,
        },
    );
    assert_eq!(fallback[0], "The Imperial fleet is victorious.");
    assert_eq!(
        fallback[2],
        "The Alliance fleet has withdrawn from the system."
    );
    let none = battle_report_lines(
        &empty,
        &BattleReport {
            system_name: "Deyer",
            viewer_alliance: true,
            winner_alliance: None,
            withdrawn: None,
            destination: None,
            last_arrival: None,
            system_side: None,
        },
    );
    assert_eq!(none[0], "Neither fleet is victorious.");
}

#[test]
fn a_blockade_is_maintained_or_broken_when_the_systems_side_arrived_last() {
    // FUN_0049b330 / FUN_0049b4d0: with +0x5c (the last arrived side,
    // FUN_0049c0d0) equal to the system's side, the winner maintained
    // (0x7015) or broke (0x7016) the blockade. A0 VM run
    // (results-tactical-withdrawn.png): the Imperial viewer won at
    // Alliance Yavin after the Alliance fleet arrived and read
    // "Imperial forces have maintained the blockade of Yavin."
    let texts = battle_texts();
    let report = |viewer, winner, system_side, last_arrival| {
        battle_report_lines(
            &texts,
            &BattleReport {
                system_name: "Yavin",
                viewer_alliance: viewer,
                winner_alliance: winner,
                withdrawn: Some(!winner.unwrap()),
                destination: None,
                system_side,
                last_arrival,
            },
        )[1]
        .clone()
    };
    assert_eq!(
        report(false, Some(false), Some(true), Some(true)),
        "Imperial forces have maintained the blockade of Yavin. The Alliance fleet has withdrawn."
    );
    assert_eq!(
        report(true, Some(true), Some(true), Some(true)),
        "Alliance forces have broken the blockade of Yavin. The Imperial fleet has withdrawn."
    );
    // The viewer lost: the viewer arrived last at its own system and
    // the enemy maintained its blockade; the enemy arrived last at its
    // own and broke the viewer's.
    assert_eq!(
        report(true, Some(false), Some(true), Some(true)),
        "Imperial forces have maintained the blockade of Yavin. The Alliance fleet has withdrawn."
    );
    assert_eq!(
        report(false, Some(true), Some(true), Some(true)),
        "Alliance forces have broken the blockade of Yavin. The Imperial fleet has withdrawn."
    );
    // The other side arrived last: an attack, not a blockade.
    assert_eq!(
        report(false, Some(false), Some(true), Some(false)),
        "Yavin is now under blockade by Imperial forces. The Alliance fleet has withdrawn."
    );
}

#[test]
fn the_viewers_withdrawn_fleet_is_reported_at_its_destination() {
    // FUN_0049b4d0 / FUN_0049b150: the viewer's withdrawal reads 0x700d
    // "The {1:4} fleet has withdrawn to {2:1}. " with +0x48 (TEXTSTRA
    // as shipped); the enemy's reads 0x700c.
    let texts = battle_texts();
    let report = |winner, withdrawn| {
        battle_report_lines(
            &texts,
            &BattleReport {
                system_name: "Deyer",
                viewer_alliance: true,
                winner_alliance: winner,
                withdrawn,
                destination: Some("Yavin"),
                last_arrival: None,
                system_side: Some(false),
            },
        )
    };
    assert_eq!(
        report(Some(false), Some(true))[1],
        "The Alliance attack on Deyer has failed. The Alliance fleet has withdrawn to Yavin."
    );
    assert_eq!(
        report(None, Some(true))[1],
        "There has been no victor.  The Alliance fleet has withdrawn to Yavin. The Imperial fleet has been completely destroyed."
    );
    assert_eq!(
        report(None, Some(false))[1],
        "There has been no victor.  The Alliance fleet has been completely destroyed. The Imperial fleet has withdrawn."
    );
}

#[test]
fn a_failed_alliance_attack_reads_as_a0() {
    // A0 TAC-01-C006 and TAC-06-C004 (Wine, the Alliance's simulated
    // Battle at Coruscant): "The Alliance fleet is defeated." over "The
    // Alliance attack on Coruscant has failed. The Alliance fleet has
    // withdrawn to Averam." (FUN_0049b4d0: 0x7003, 0x7009, 0x700d).
    let lines = battle_report_lines(
        &battle_texts(),
        &BattleReport {
            system_name: "Coruscant",
            viewer_alliance: true,
            winner_alliance: Some(false),
            withdrawn: Some(true),
            destination: Some("Averam"),
            last_arrival: None,
            system_side: Some(false),
        },
    );
    assert_eq!(
        lines,
        [
            "The Alliance fleet is defeated. ",
            "The Alliance attack on Coruscant has failed. The Alliance fleet has withdrawn to Averam."
        ]
    );
}

#[test]
fn an_alliance_fleet_destroyed_where_the_empire_broke_its_blockade_reads_as_a0() {
    // A0 TAC-01-C013 (the VM's new small Alliance game, day 49, the
    // Battle at Mon Calamari): the last Alliance ship died, the battle
    // ended by itself, and the strategy screen showed "The Alliance
    // fleet is defeated." over "Imperial forces have broken the
    // blockade of Mon Calamari. The Alliance fleet has been completely
    // destroyed." (FUN_0049b4d0: 0x7003, 0x7016 with the enemy, as the
    // side that arrived last held the system, then 0x700b).
    let lines = battle_report_lines(
        &battle_texts(),
        &BattleReport {
            system_name: "Mon Calamari",
            viewer_alliance: true,
            winner_alliance: Some(false),
            withdrawn: None,
            destination: None,
            last_arrival: Some(false),
            system_side: Some(false),
        },
    );
    assert_eq!(
        lines,
        [
            "The Alliance fleet is defeated. ",
            "Imperial forces have broken the blockade of Mon Calamari. The Alliance fleet has been completely destroyed."
        ]
    );
}

#[test]
fn the_alliances_withdrawal_from_its_own_system_reads_as_a0() {
    // A0 battle-clean/01-summary-withdrawn-umgul and 02-display (an
    // unedited Alliance game, day 650, the Empire's Battle at Yavin,
    // Simulate Results): "The Alliance fleet is defeated." over "Yavin
    // is now under blockade by Imperial forces. The Alliance fleet has
    // withdrawn to Umgul." (FUN_0049b4d0: 0x7003, 0x7006, 0x700d); the
    // Display page reads "The Alliance fleet was defeated at Yavin.
    // Your fleet has withdrawn to Umgul." (0x7011 with +0x48) over
    // 0x7013, with both buttons.
    let mut texts = battle_texts();
    for (id, text) in [
        (
            0x7011,
            "The {1:4} fleet was defeated at {2:1}. Your fleet has withdrawn to {3:1}. ",
        ),
        (
            0x7013,
            "Select one of the following buttons to close this display and go directly to... ",
        ),
    ] {
        texts.insert(id, text.to_owned());
    }
    let report = BattleReport {
        system_name: "Yavin",
        viewer_alliance: true,
        winner_alliance: Some(false),
        withdrawn: Some(true),
        destination: Some("Umgul"),
        last_arrival: Some(false),
        system_side: Some(true),
    };
    assert_eq!(
        battle_report_lines(&texts, &report),
        [
            "The Alliance fleet is defeated. ",
            "Yavin is now under blockade by Imperial forces. The Alliance fleet has withdrawn to Umgul."
        ]
    );
    assert_eq!(
        battle_display_lines(&texts, &report),
        (
            [
                "The Alliance fleet was defeated at Yavin. Your fleet has withdrawn to Umgul."
                    .to_owned(),
                "Select one of the following buttons to close this display and go directly to..."
                    .to_owned()
            ],
            true
        )
    );
    // The Empire's side (A0 empire-clean/41-summary-withdrawn-chandrila
    // and 42-display, a new Easy Empire game, day 689): "Chandrila is
    // now under blockade by Alliance forces. The Imperial fleet has
    // withdrawn to Coruscant." and "The Imperial fleet was defeated at
    // Chandrila. Your fleet has withdrawn to Coruscant."; the Empire's
    // Fleet button 10766 keyed on its blue (0 of 16055 pixels off).
    let imperial = BattleReport {
        system_name: "Chandrila",
        viewer_alliance: false,
        winner_alliance: Some(true),
        withdrawn: Some(false),
        destination: Some("Coruscant"),
        last_arrival: Some(true),
        system_side: Some(false),
    };
    assert_eq!(
        battle_report_lines(&texts, &imperial)[1],
        "Chandrila is now under blockade by Alliance forces. The Imperial fleet has withdrawn to Coruscant."
    );
    assert_eq!(
        battle_display_lines(&texts, &imperial).0[0],
        "The Imperial fleet was defeated at Chandrila. Your fleet has withdrawn to Coruscant."
    );
}

#[test]
fn the_alliances_defence_of_its_own_system_reads_as_a0() {
    // A0 battle-clean/11-summary-defended-averam and 12-display (an
    // unedited Alliance game, day 672, the Empire's Battle at Averam,
    // Simulate Results): "The Alliance fleet is victorious." over
    // "Averam has been successfully defended from Imperial forces. The
    // Imperial fleet has withdrawn." (FUN_0049b4d0: 0x7002, 0x7005,
    // 0x700c); the Display page reads 0x7012, "The Alliance fleet is
    // victorious. The world Averam is now under Alliance blockade.",
    // its own system as any won one, over 0x7013 with both buttons.
    let mut texts = battle_texts();
    for (id, text) in [
        (
            0x7012,
            "The {1:4} fleet is victorious. The world {2:1} is now under {1:4} blockade. ",
        ),
        (
            0x7013,
            "Select one of the following buttons to close this display and go directly to... ",
        ),
    ] {
        texts.insert(id, text.to_owned());
    }
    let report = BattleReport {
        system_name: "Averam",
        viewer_alliance: true,
        winner_alliance: Some(true),
        withdrawn: Some(false),
        destination: None,
        last_arrival: Some(false),
        system_side: Some(true),
    };
    assert_eq!(
        battle_report_lines(&texts, &report),
        [
            "The Alliance fleet is victorious. ",
            "Averam has been successfully defended from Imperial forces. The Imperial fleet has withdrawn."
        ]
    );
    assert_eq!(
        battle_display_lines(&texts, &report),
        (
            [
                "The Alliance fleet is victorious. The world Averam is now under Alliance blockade."
                    .to_owned(),
                "Select one of the following buttons to close this display and go directly to..."
                    .to_owned()
            ],
            true
        )
    );
    // The Empire's side (A0 empire-clean/51-summary-defended-coruscant
    // and 52-display, a new Easy Empire game, day 728): "The Imperial
    // fleet is victorious." over "Coruscant has been successfully
    // defended from Alliance forces. The Alliance fleet has withdrawn.",
    // and 0x7012 "... The world Coruscant is now under Imperial
    // blockade." with both buttons.
    let imperial = BattleReport {
        system_name: "Coruscant",
        viewer_alliance: false,
        winner_alliance: Some(false),
        withdrawn: Some(true),
        destination: None,
        last_arrival: Some(true),
        system_side: Some(false),
    };
    assert_eq!(
        battle_report_lines(&texts, &imperial),
        [
            "The Imperial fleet is victorious. ",
            "Coruscant has been successfully defended from Alliance forces. The Alliance fleet has withdrawn."
        ]
    );
    assert_eq!(
        battle_display_lines(&texts, &imperial),
        (
            [
                "The Imperial fleet is victorious. The world Coruscant is now under Imperial blockade."
                    .to_owned(),
                "Select one of the following buttons to close this display and go directly to..."
                    .to_owned()
            ],
            true
        )
    );
}

#[test]
fn the_alliances_destroyed_fleet_reads_as_a0() {
    // A0 battle-clean/23-summary-destroyed-umgul and 24-display (an
    // unedited Alliance game, day 718, the Empire's Battle at Umgul,
    // Simulate Results): "The Alliance fleet is defeated." over "Umgul
    // is now under blockade by Imperial forces. The Alliance fleet has
    // been completely destroyed." (FUN_0049b4d0: 0x7003, 0x7006,
    // 0x700b); the Display page reads 0x7010, "The Alliance fleet was
    // defeated at Umgul. Your fleet has been destroyed.", over 0x7014
    // with the system's button alone.
    let mut texts = battle_texts();
    for (id, text) in [
        (
            0x7010,
            "The {1:4} fleet was defeated at {2:1}. Your fleet has been destroyed. ",
        ),
        (
            0x7014,
            "Select the following button to close this display and go directly to... ",
        ),
    ] {
        texts.insert(id, text.to_owned());
    }
    let report = BattleReport {
        system_name: "Umgul",
        viewer_alliance: true,
        winner_alliance: Some(false),
        withdrawn: None,
        destination: None,
        last_arrival: Some(false),
        system_side: Some(true),
    };
    assert_eq!(
        battle_report_lines(&texts, &report),
        [
            "The Alliance fleet is defeated. ",
            "Umgul is now under blockade by Imperial forces. The Alliance fleet has been completely destroyed."
        ]
    );
    assert_eq!(
        battle_display_lines(&texts, &report),
        (
            [
                "The Alliance fleet was defeated at Umgul. Your fleet has been destroyed."
                    .to_owned(),
                "Select the following button to close this display and go directly to..."
                    .to_owned()
            ],
            false
        )
    );
    // The Empire's side (A0 empire-clean/11-summary-destroyed-umgul and
    // 12-display, a new Easy Empire game, day 253): "The Imperial fleet
    // is defeated." over "Umgul is now under blockade by Alliance
    // forces. The Imperial fleet has been completely destroyed.", and
    // "The Imperial fleet was defeated at Umgul. Your fleet has been
    // destroyed." with the system's button alone.
    let imperial = BattleReport {
        viewer_alliance: false,
        winner_alliance: Some(true),
        last_arrival: Some(true),
        system_side: Some(false),
        ..report
    };
    assert_eq!(
        battle_report_lines(&texts, &imperial),
        [
            "The Imperial fleet is defeated. ",
            "Umgul is now under blockade by Alliance forces. The Imperial fleet has been completely destroyed."
        ]
    );
    assert_eq!(
        battle_display_lines(&texts, &imperial).0[0],
        "The Imperial fleet was defeated at Umgul. Your fleet has been destroyed."
    );
}

#[test]
fn the_empires_withdrawal_from_a_blockade_reads_as_a0() {
    // A0 battle-alert-empire/04-result (the Empire game's Battle at
    // Tralus, Simulate Results): "The Imperial fleet is defeated." over
    // "Tralus is now under blockade by Alliance forces. The Imperial
    // fleet has withdrawn to Coruscant." (FUN_0049b4d0: 0x7003, 0x7006,
    // 0x700d).
    let texts = battle_texts();
    let lines = battle_report_lines(
        &texts,
        &BattleReport {
            system_name: "Tralus",
            viewer_alliance: false,
            winner_alliance: Some(true),
            withdrawn: Some(false),
            destination: Some("Coruscant"),
            last_arrival: Some(true),
            system_side: Some(false),
        },
    );
    assert_eq!(
        lines,
        [
            "The Imperial fleet is defeated. ",
            "Tralus is now under blockade by Alliance forces. The Imperial fleet has withdrawn to Coruscant."
        ]
    );
}

#[test]
fn the_report_finds_the_viewers_withdrawal_destination_in_the_world() {
    // FUN_0049b720 sets +0x48 from the viewer's withdrawn fleet; the
    // port asks withdraw_loser's search (FUN_005545e0): the nearest
    // system its side holds. An enemy withdrawal names none.
    use rebellion_core::dat::Faction;
    use rebellion_core::world::ControlKind;
    let mut world = GameWorld::default();
    let battle = test_system(&mut world, ControlKind::Controlled(Faction::Empire), None);
    let far = test_system(&mut world, ControlKind::Controlled(Faction::Alliance), None);
    let near = test_system(&mut world, ControlKind::Controlled(Faction::Alliance), None);
    world.systems[far].name = "Hoth".into();
    world.systems[far].x = 90;
    world.systems[near].name = "Yavin".into();
    world.systems[near].x = 40;
    let mut fled = test_ship(64, 1, true, true);
    fled.retreated = true;
    let mut session = test_session(vec![fled, test_ship(64, 2, false, true)], Vec::new(), true);
    session.system = battle;
    assert_eq!(
        battle_report(&session, &world, None).destination,
        Some("Yavin")
    );
    session.ships[0].retreated = false;
    session.ships[1].retreated = true;
    assert_eq!(battle_report(&session, &world, None).destination, None);
}

#[test]
fn the_battle_alert_tells_the_viewer_who_is_moving_on_whom() {
    // FUN_0049c0d0 with TEXTSTRA RCDATA 0x7020..0x7025 as shipped.
    let mut texts = battle_texts();
    for (id, text) in [
        (0x7020, "Battle at {1:1}"),
        (0x7021, "The {1:4} fleet is threatening {2:1}. {3:4} forces are moving to intercept. "),
        (0x7022, "The {1:4} fleet is attempting to break the {3:4} blockade of {2:4}. "),
        (0x7023, "The {1:4} fleet has entered the {2:1} system. {3:4} forces have been detected on an intercept course. "),
        (0x7024, "The {1:4} fleet is attempting to break the {2:4} blockade at {3:1}"),
        (0x7025, "{1:4} and {2:4} forces are about to engage in battle near the {3:1} system. "),
    ] {
        texts.insert(id, text.to_owned());
    }
    // guide-055: the Empire's viewer at the Alliance's Deyer, where the
    // Empire arrived last.
    assert_eq!(
        battle_alert_lines(&texts, false, "Deyer", Some(true), Some(false)),
        [
            "Battle at Deyer",
            "The Imperial fleet has entered the Deyer system. Alliance forces have been detected on an intercept course."
        ]
    );
    // The Alliance's viewer of the same battle: its system is threatened.
    assert_eq!(
        battle_alert_lines(&texts, true, "Deyer", Some(true), Some(false))[1],
        "The Imperial fleet is threatening Deyer. Alliance forces are moving to intercept."
    );
    // The viewer arrived last at its own system: it breaks a blockade.
    assert_eq!(
        battle_alert_lines(&texts, true, "Deyer", Some(true), Some(true))[1],
        "The Alliance fleet is attempting to break the Imperial blockade of Deyer."
    );
    // The enemy arrived last at its own system.
    assert_eq!(
        battle_alert_lines(&texts, true, "Deyer", Some(false), Some(false))[1],
        "The Imperial fleet is attempting to break the Alliance blockade at Deyer"
    );
    assert_eq!(
        battle_alert_lines(&texts, false, "Deyer", None, Some(true))[1],
        "Imperial and Alliance forces are about to engage in battle near the Deyer system."
    );
    // Without the texts, the port's English says the same.
    let empty = HashMap::new();
    for (viewer, side, last) in [
        (false, Some(true), Some(false)),
        (true, Some(true), Some(false)),
        (true, Some(true), Some(true)),
        (true, Some(false), Some(false)),
        (false, None, Some(true)),
    ] {
        assert_eq!(
            battle_alert_lines(&empty, viewer, "Deyer", side, last),
            battle_alert_lines(&texts, viewer, "Deyer", side, last)
        );
    }
}

fn test_system(
    world: &mut GameWorld,
    control: rebellion_core::world::ControlKind,
    last_arrival: Option<rebellion_core::dat::Faction>,
) -> SystemKey {
    world.systems.insert(rebellion_core::world::System {
        last_arrival,
        dat_id: DatId::new(0x9000_0001),
        name: "Deyer".into(),
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control,
        ..Default::default()
    })
}

#[test]
fn the_battle_alert_lists_a_side_s_fleets_then_its_planet_squadrons() {
    // FUN_00450770: the heading (0x1923), each fleet's name, its ships
    // (FUN_00450e10 font 5 with the mini) and the units aboard (font 4,
    // indented); then the system's name and the side's squadrons on the
    // planet (FUN_00503a50). guide-057 shows "Alliance Forces",
    // "Deyer", "A-wing".
    use rebellion_core::world::{
        CapitalShipClass, Character, ControlKind, FighterClass, FighterEntry, Fleet, ShipInstance,
    };
    let mut world = GameWorld::default();
    let system = test_system(&mut world, ControlKind::Uncontrolled, None);
    let corvette = world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(0x1400_0040),
        name: "Corellian Corvette".into(),
        is_alliance: true,
        ..CapitalShipClass::default()
    });
    let awing = world.fighter_classes.insert(FighterClass {
        dat_id: DatId::new(0x1c00_0001),
        name: "A-wing".into(),
        is_alliance: true,
        ..FighterClass::default()
    });
    let tie = world.fighter_classes.insert(FighterClass {
        dat_id: DatId::new(0x1c00_0005),
        name: "TIE Fighter".into(),
        is_alliance: false,
        ..FighterClass::default()
    });
    let leia = world.characters.insert(Character {
        name: "Leia".into(),
        ..Default::default()
    });
    let fleet = |is_alliance, capital_ships, fighters, characters| Fleet {
        location: system,
        capital_ships,
        fighters,
        characters,
        is_alliance,
        ..Default::default()
    };
    let rebels = world.fleets.insert(fleet(
        true,
        vec![ShipInstance::new(corvette, 0, false)],
        vec![FighterEntry {
            class: awing,
            count: 1,
            carrier: None,
            damage: None,
        }],
        vec![leia],
    ));
    let empire = world.fleets.insert(fleet(false, vec![], vec![], vec![]));
    world.systems[system].fleets = vec![empire, rebels];
    world.systems[system].fighters = vec![
        FighterEntry {
            class: awing,
            count: 2,
            carrier: None,
            damage: None,
        },
        FighterEntry {
            class: tie,
            count: 1,
            carrier: None,
            damage: None,
        },
    ];
    let rows = battle_alert_rows(
        &world,
        &TroopTransportState::default(),
        system,
        BattleAlertList::Forces { alliance: true },
    );
    let summary: Vec<(&str, AlertFont, bool, bool)> = rows
        .iter()
        .map(|row| {
            (
                row.text.as_str(),
                row.font,
                !row.picture.is_empty(),
                row.indent,
            )
        })
        .collect();
    let name = world.fleet_name(rebels).unwrap_or_default().to_owned();
    assert_eq!(
        summary,
        [
            ("Alliance Forces", AlertFont::Heading, false, false),
            (name.as_str(), AlertFont::Bold, false, false),
            ("Corellian Corvette", AlertFont::Bold, true, false),
            ("A-wing", AlertFont::Plain, true, true),
            ("Leia", AlertFont::Plain, false, true),
            ("Deyer", AlertFont::Bold, false, false),
            ("A-wing", AlertFont::Plain, true, false),
            ("A-wing", AlertFont::Plain, true, false),
        ]
    );
    // The ship's mini is keyed over nothing; the squadron's too.
    assert_eq!(
        rows[2].picture,
        [(
            DllSource::Gokres,
            crate::minis::capital_ship_mini_id(DatId::new(0x1400_0040)).unwrap()
        )]
    );
    // The Empire's tab: its empty fleet's name, its TIE on the planet.
    let rows = battle_alert_rows(
        &world,
        &TroopTransportState::default(),
        system,
        BattleAlertList::Forces { alliance: false },
    );
    let texts: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
    let name = world.fleet_name(empire).unwrap_or_default().to_owned();
    assert_eq!(
        texts,
        ["Imperial Forces", name.as_str(), "Deyer", "TIE Fighter"]
    );
    // The unfinished Death Star the system's yard builds stands in the
    // battle (FUN_0040b0e0 0x40b4d2 -> FUN_0040bcd0): its row follows
    // the Empire's fleets, its mini over the unfinished mark 0x2d34.
    use rebellion_core::manufacturing::{BuildableKind, ManufacturingState, QueueItem};
    let death_star = world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(rebellion_core::world::DEATH_STAR_CLASS_ID),
        is_empire: true,
        ..Default::default()
    });
    let mut manufacturing = ManufacturingState::new();
    manufacturing.enqueue(
        system,
        QueueItem::new(BuildableKind::CapitalShip(death_star), 10),
    );
    rebellion_core::products::reconcile(&mut world, &manufacturing);
    let rows = battle_alert_rows(
        &world,
        &TroopTransportState::default(),
        system,
        BattleAlertList::Forces { alliance: false },
    );
    let texts: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Imperial Forces",
            name.as_str(),
            "Death Star",
            "Deyer",
            "TIE Fighter"
        ]
    );
    assert!(rows[2].picture.contains(&(DllSource::Strategy, 11_572)));
    let rows = battle_alert_rows(
        &world,
        &TroopTransportState::default(),
        system,
        BattleAlertList::Forces { alliance: true },
    );
    assert!(rows.iter().all(|row| row.text != "Death Star"));
}

// FUN_00450770 lists each ship's own squadrons (FUN_005039d0) right
// under it, indented; a squadron on no ship follows the ships (port).
#[test]
fn the_battle_alert_lists_each_ships_squadrons_under_it() {
    use rebellion_core::world::{
        CapitalShipClass, ControlKind, FighterClass, FighterEntry, Fleet, ShipInstance,
    };
    let mut world = GameWorld::default();
    let system = test_system(&mut world, ControlKind::Uncontrolled, None);
    let cruiser = world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(0x1400_0040),
        name: "Cruiser".into(),
        is_alliance: true,
        ..CapitalShipClass::default()
    });
    let fighter = |world: &mut GameWorld, name: &str| {
        world.fighter_classes.insert(FighterClass {
            dat_id: DatId::new(0x1c00_0001),
            name: name.into(),
            is_alliance: true,
            ..FighterClass::default()
        })
    };
    let (xwing, ywing, awing) = (
        fighter(&mut world, "X-wing"),
        fighter(&mut world, "Y-wing"),
        fighter(&mut world, "A-wing"),
    );
    let mut first = ShipInstance::new(cruiser, 0, false);
    first.id = 11;
    let mut second = ShipInstance::new(cruiser, 0, false);
    second.id = 12;
    let entry = |class, count, carrier| FighterEntry {
        class,
        count,
        carrier,
        damage: None,
    };
    let rebels = world.fleets.insert(Fleet {
        location: system,
        capital_ships: vec![first, second],
        // The last one's ship is gone from the fleet.
        fighters: vec![
            entry(ywing, 1, Some(12)),
            entry(xwing, 2, Some(11)),
            entry(awing, 1, None),
            entry(awing, 1, Some(99)),
        ],
        is_alliance: true,
        ..Default::default()
    });
    world.systems[system].fleets = vec![rebels];
    let rows = battle_alert_rows(
        &world,
        &TroopTransportState::default(),
        system,
        BattleAlertList::Forces { alliance: true },
    );
    let summary: Vec<(&str, bool)> = rows
        .iter()
        .skip(2)
        .map(|row| (row.text.as_str(), row.indent))
        .collect();
    assert_eq!(
        summary,
        [
            ("Cruiser", false),
            ("X-wing", true),
            ("X-wing", true),
            ("Cruiser", false),
            ("Y-wing", true),
            ("A-wing", true),
            ("A-wing", true),
        ]
    );
}

#[test]
fn the_battle_alert_lists_a_systems_facilities_by_kind_as_a0() {
    // A0 battle-alert-empire/01-system-assets (the Battle at Tralus, day
    // 300): GenCore Level I, Orbital Shipyard, Construction Yard, Mine,
    // Mine (FUN_00539d70: defenses, then yards, then mines).
    use rebellion_core::dat::Faction;
    use rebellion_core::world::{
        ControlKind, DefenseFacilityInstance, ManufacturingFacilityInstance,
        ProductionFacilityInstance,
    };
    let mut world = GameWorld::default();
    let system = test_system(&mut world, ControlKind::Controlled(Faction::Empire), None);
    let yard = |world: &mut GameWorld, id, is_shipyard| {
        world
            .manufacturing_facilities
            .insert(ManufacturingFacilityInstance {
                class_dat_id: DatId::new(id),
                side: Faction::Empire,
                is_shipyard,
            })
    };
    let construction = yard(&mut world, 0x2a00_0003, false);
    let shipyard = yard(&mut world, 0x2800_0001, true);
    let mine = |world: &mut GameWorld| {
        world
            .production_facilities
            .insert(ProductionFacilityInstance {
                class_dat_id: DatId::new(0x2c00_0001),
                side: Faction::Empire,
                is_mine: true,
            })
    };
    let mines = [mine(&mut world), mine(&mut world)];
    let gencore = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(0x2300_0003),
        side: Faction::Empire,
    });
    world.systems[system].manufacturing_facilities = vec![shipyard, construction];
    world.systems[system].production_facilities = mines.to_vec();
    world.systems[system].defense_facilities = vec![gencore];
    let rows = battle_alert_rows(
        &world,
        &TroopTransportState::default(),
        system,
        BattleAlertList::SystemAssets,
    );
    let texts: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "System Assets",
            "GenCore Level I",
            "Orbital Shipyard",
            "Construction Yard",
            "Mine",
            "Mine"
        ]
    );
    assert!(rows[1..].iter().all(|row| !row.picture.is_empty()));
}

#[test]
fn the_battle_alert_lists_the_system_assets() {
    // FUN_00450770(0): the heading 0x1925, the facilities
    // (FUN_00539d70), the regiments (FUN_00504c40) and the personnel
    // (FUN_00536da0) on the planet, font 4, not indented. guide-056
    // lists "LNR Series II" and "GenCore Level II".
    use rebellion_core::dat::Faction;
    use rebellion_core::world::{Character, ControlKind, DefenseFacilityInstance, TroopUnit};
    let mut world = GameWorld::default();
    let system = test_system(&mut world, ControlKind::Controlled(Faction::Alliance), None);
    let lnr = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(0x2200_0005),
        side: Faction::Alliance,
    });
    let army = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0002),
        is_alliance: true,
        regiment_strength: 1,
    });
    world.systems[system].defense_facilities = vec![lnr];
    world.systems[system].ground_units = vec![army];
    world.characters.insert(Character {
        name: "Mon Mothma".into(),
        current_system: Some(system),
        ..Default::default()
    });
    world.characters.insert(Character {
        name: "Elsewhere".into(),
        ..Default::default()
    });
    let rows = battle_alert_rows(
        &world,
        &TroopTransportState::default(),
        system,
        BattleAlertList::SystemAssets,
    );
    let texts: Vec<(&str, AlertFont, bool)> = rows
        .iter()
        .map(|row| (row.text.as_str(), row.font, row.indent))
        .collect();
    assert_eq!(
        texts,
        [
            ("System Assets", AlertFont::Heading, false),
            ("LNR Series II", AlertFont::Plain, false),
            ("Alliance Army Regiment", AlertFont::Plain, false),
            ("Mon Mothma", AlertFont::Plain, false),
        ]
    );
    // A regiment's mini sits on its backdrop, STRATEGY 11513.
    assert_eq!(
        rows[2].picture,
        [(DllSource::Strategy, 11_513), (DllSource::Gokres, 17_473)]
    );
}

#[test]
fn the_battle_alert_scene_text_sits_where_the_original_paints_it() {
    // FUN_0044f860: the title at (0, 16), 400 wide, centred, font 0xb;
    // the summary at (36, 216), 328 wide, font 6, over a black shadow at
    // (37, 217), all in window coordinates: A0 TAC-01-C010 shows the
    // title centred on window x 200 with capitals from row 20, and the
    // summary's capitals from row 219 at x 36. (The port had read
    // guide-055 as about 48 from the window, a scene-relative reading.)
    let green = Color::from_rgba(0, 255, 0, 255);
    let ops = battle_alert_scene_text("Battle at Deyer", Some("The Imperial fleet"), green);
    assert_eq!(
        ops,
        [
            PageOp::Text {
                text: "Battle at Deyer".into(),
                x: 200.0,
                baseline: 35.0,
                size: BATTLE_WINDOW_FONT_B,
                color: green,
                centered: true,
                bold: false,
            },
            PageOp::Wrapped {
                lines: vec!["The Imperial fleet".into()],
                x: 37.0,
                baseline: 232.0,
                width: 328.0,
                size: BATTLE_WINDOW_FONT_6,
                color: BLACK,
                bold: false,
            },
            PageOp::Wrapped {
                lines: vec!["The Imperial fleet".into()],
                x: 36.0,
                baseline: 231.0,
                width: 328.0,
                size: BATTLE_WINDOW_FONT_6,
                color: green,
                bold: false,
            },
        ]
    );
    // The list tabs keep the title only.
    assert_eq!(
        battle_alert_scene_text("Battle at Deyer", None, green).len(),
        1
    );
}

#[test]
fn each_battle_alert_tab_fills_its_own_list() {
    // FUN_00450660: tab 1 hides the list; 2, 3, 4 fill 1, 2, 0.
    assert_eq!(BattleAlertList::of(BattleAlertTab::Summary), None);
    assert_eq!(
        BattleAlertList::of(BattleAlertTab::AllianceForces),
        Some(BattleAlertList::Forces { alliance: true })
    );
    assert_eq!(
        BattleAlertList::of(BattleAlertTab::ImperialForces),
        Some(BattleAlertList::Forces { alliance: false })
    );
    assert_eq!(
        BattleAlertList::of(BattleAlertTab::System),
        Some(BattleAlertList::SystemAssets)
    );
}

#[test]
fn an_injured_character_in_the_results_wears_the_damage_mark() {
    // FUN_004724a0 passes the personnel page's minis (local_b0 1):
    // FUN_0042c3b0 puts STRATEGY 11502 over the backdrop 11500 for a
    // damaged character (FUN_004f7800: injury != 0). The regiments get
    // no mark: GameObj_SetDamaged has no regiment caller.
    use rebellion_core::world::Character;
    let mut world = GameWorld::default();
    let mut person = |injury| {
        world.characters.insert(Character {
            name: "Luke".into(),
            dat_id: DatId::new(0x3000_0240),
            is_major: true,
            injury,
            ..Default::default()
        })
    };
    let (well, hurt) = (person(0), person(5));
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        characters: vec![well, hurt],
        is_alliance: true,
        ..Default::default()
    });
    let mut session = test_session(Vec::new(), Vec::new(), true);
    session.attacker_fleet = fleet;
    session.defender_fleet = fleet;
    let entries = battle_result_entries(
        &session,
        &world,
        &TroopTransportState::default(),
        true,
        BattleResultCategory::Personnel,
    );
    assert_eq!(
        entries.iter().map(|entry| entry.mark).collect::<Vec<_>>(),
        [None, Some((DllSource::Strategy, 11_502))]
    );
}

#[test]
fn an_injured_character_in_the_battle_alert_wears_the_damage_mark() {
    // FUN_004f7800: injury != 0 is damaged; FUN_0042c3b0 puts STRATEGY
    // 11502 over the backdrop 11500 for personnel.
    use rebellion_core::world::{Character, ControlKind};
    let mut world = GameWorld::default();
    let system = test_system(&mut world, ControlKind::Uncontrolled, None);
    let mut person = |injury| {
        world.characters.insert(Character {
            name: "Luke".into(),
            dat_id: DatId::new(0x3000_0240),
            is_major: true,
            current_system: Some(system),
            injury,
            ..Default::default()
        })
    };
    person(0);
    person(5);
    let rows = battle_alert_rows(
        &world,
        &TroopTransportState::default(),
        system,
        BattleAlertList::SystemAssets,
    );
    let mini = rebellion_core::pictures::character_mini_resource_id(DatId::new(0x3000_0240), true)
        .unwrap();
    assert_eq!(
        rows[1].picture,
        [(DllSource::Strategy, 11_500), (DllSource::Gokres, mini)]
    );
    assert_eq!(
        rows[2].picture,
        [
            (DllSource::Strategy, 11_500),
            (DllSource::Strategy, 11_502),
            (DllSource::Gokres, mini)
        ]
    );
}

#[test]
fn the_forces_list_scrolls_by_pixels_under_its_bar() {
    // FUN_0046f360: the list (37, 136), 347 by 166, of 70-high rows
    // (50 for Personnel, FUN_004714e0), and its bar (FUN_0060a490)
    // whose range is in pixels (FUN_00609ae0: content + 5 - 166).
    assert_eq!(
        battle_result_row_height(BattleResultCategory::CapitalShips),
        70.0
    );
    assert_eq!(
        battle_result_row_height(BattleResultCategory::Personnel),
        50.0
    );
    let bar = battle_result_bar(210.0, 0);
    assert_eq!(
        (bar.x, bar.y, bar.view, bar.max),
        (372.0, 136.0, 166.0, 49.0)
    );
    // The bar's art from STRATEGY.DLL, clipped to its 12 by 166.
    let ops = battle_result_bar_ops(&bar, None);
    assert_eq!(
        ops.first(),
        Some(&PageOp::Clip(Some((372.0, 136.0, 12.0, 166.0))))
    );
    assert_eq!(ops.last(), Some(&PageOp::Clip(None)));
    assert!(ops.contains(&PageOp::Bitmap {
        source: DllSource::Strategy,
        id: crate::list_scroll_bar::RESULTS_ART.up[0],
        x: 372.0,
        y: 136.0,
    }));
    // Two rows fit: no bar (FUN_0060b5d0).
    assert!(battle_result_bar_ops(&battle_result_bar(140.0, 0), None).is_empty());
    // The Alliance window at (62, 49) (A0 TAC-07-C001). port: a wheel
    // notch over the list moves a row, within the bar's range.
    let at = |x: f32, y: f32| (62.0 + x, 49.0 + y);
    let scroll = |from, (x, y), wheel, content, category| {
        result_list_scroll(from, at(x, y), wheel, content, category)
    };
    let fighters = BattleResultCategory::Fighters;
    assert_eq!(scroll(0, (37.0, 136.0), -1.0, 280.0, fighters), 70);
    assert_eq!(scroll(70, (37.0, 136.0), -1.0, 280.0, fighters), 119);
    assert_eq!(scroll(0, (37.0, 136.0), -1.0, 140.0, fighters), 0);
    assert_eq!(scroll(30, (383.5, 301.5), 1.0, 280.0, fighters), 0);
    assert_eq!(scroll(119, (100.0, 200.0), 1.0, 280.0, fighters), 49);
    assert_eq!(scroll(70, (100.0, 200.0), 0.0, 280.0, fighters), 70);
    for point in [
        (36.5, 200.0),
        (384.0, 200.0),
        (100.0, 135.5),
        (100.0, 302.0),
    ] {
        assert_eq!(
            scroll(0, point, -1.0, 630.0, BattleResultCategory::Troops),
            0
        );
    }
    // The rows are the longest column's: three operational, one
    // destroyed.
    let entry = |condition| BattleResultEntry {
        name: String::new(),
        condition,
        picture: None,
        backdrop: None,
        mark: None,
    };
    let mixed = [
        entry(BattleResultCondition::Operational),
        entry(BattleResultCondition::Destroyed),
        entry(BattleResultCondition::Operational),
        entry(BattleResultCondition::Operational),
    ];
    assert_eq!(
        battle_result_list_length(&mixed, BattleResultCategory::CapitalShips),
        3
    );
    assert_eq!(
        battle_result_content(&mixed, BattleResultCategory::CapitalShips),
        210.0
    );
    assert_eq!(
        battle_result_list_length(&[], BattleResultCategory::CapitalShips),
        0
    );
    // A new tab or category starts at the top; the same keeps it.
    let (mut shown, mut at_pixel) = (BattleResultCategory::Fighters, 20);
    select_resetting_scroll(&mut shown, &mut at_pixel, BattleResultCategory::Fighters);
    assert_eq!(at_pixel, 20);
    select_resetting_scroll(&mut shown, &mut at_pixel, BattleResultCategory::Troops);
    assert_eq!((shown, at_pixel), (BattleResultCategory::Troops, 0));
    // The list clips its cells to the 166-high window: at 0 the third
    // row's top 26 pixels show; scrolled 70, the rows move up 70 and the
    // fourth comes in.
    let entries: Vec<BattleResultEntry> = (0..4)
        .map(|index| BattleResultEntry {
            name: format!("Ship {index}"),
            condition: BattleResultCondition::Operational,
            picture: None,
            backdrop: None,
            mark: None,
        })
        .collect();
    let page = |scroll| {
        battle_force_page(
            "Hoth",
            &entries,
            true,
            BattleResultCategory::CapitalShips,
            scroll,
            true,
            WHITE,
            |_, _| None,
        )
    };
    let names = |scroll| {
        page(scroll)
            .into_iter()
            .filter_map(|op| match op {
                PageOp::Text { text, baseline, .. } if text.starts_with("Ship") => {
                    Some((text, baseline))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let first = names(0);
    assert_eq!(
        first
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>(),
        ["Ship 0", "Ship 1", "Ship 2"]
    );
    let scrolled = names(70);
    assert_eq!(scrolled[0], ("Ship 1".to_owned(), first[0].1));
    assert_eq!(scrolled[2].0, "Ship 3");
    let ops = page(0);
    assert!(ops.contains(&PageOp::Clip(Some((37.0, 136.0, 347.0, 166.0)))));
    assert_eq!(ops.last(), Some(&PageOp::Clip(None)));
}

#[test]
fn the_forces_lists_bar_takes_the_pointer_in_window_coordinates() {
    // FUN_0046f360: the bar at window (372, 136); a click on its down
    // arrow (window y 293..302) steps the list's 10 pixels
    // (FUN_00607ea0's step), drawn pressed while held; a wheel notch
    // over the list moves a row (port).
    let origin = battle_window_origin();
    let window = |x: f32, y: f32| (origin.0 + x, origin.1 + y);
    let mut state = TacticalState::default();
    let content = 280.0;
    let arrow = window(378.0, 297.0);
    battle_result_list_step(&mut state, content, arrow, (true, true, false), 0.0, 0.0);
    assert_eq!(
        battle_result_pressed_arrow(&state, content, arrow, true),
        Some(crate::list_scroll_bar::Part::Down)
    );
    assert_eq!(
        battle_result_pressed_arrow(&state, content, arrow, false),
        None
    );
    assert_eq!(
        battle_result_pressed_arrow(&state, content, window(378.0, 200.0), true),
        None
    );
    battle_result_list_step(&mut state, content, arrow, (false, false, true), 0.0, 0.1);
    assert_eq!(state.battle_result_scroll, 10);
    // The wheel over the list.
    battle_result_list_step(
        &mut state,
        content,
        window(100.0, 200.0),
        (false, false, false),
        -1.0,
        0.2,
    );
    assert_eq!(state.battle_result_scroll, 80);
    // A held thumb is not an arrow.
    battle_result_list_step(
        &mut state,
        content,
        window(378.0, 200.0),
        (true, true, false),
        0.0,
        0.3,
    );
    assert_eq!(
        battle_result_pressed_arrow(&state, content, window(378.0, 200.0), true),
        None
    );
    // The clip maps window coordinates through the letterboxed canvas.
    let canvas = TacticalCanvas::new(1280.0, 800.0);
    let clip = page_clip_on_screen(canvas, (62.0, 49.0), (372.0, 136.0, 12.0, 166.0));
    let (left, top) = canvas.point(434.0, 185.0);
    assert_eq!(
        (clip.x, clip.y, clip.width, clip.height),
        (left, top, 12.0 * canvas.scale, 166.0 * canvas.scale)
    );
    assert!(canvas.scale > 1.0);
}

#[test]
fn the_battle_alert_list_scrolls_by_pixels_under_its_bar() {
    // FUN_0044f860: the list (83, 40), 256 by 234, of 40-high rows
    // (0x44fcde), clips its rows; its bar (0x29fc, RCDATA 10748: 10
    // wide, art 10749..10756) sits at the list's right edge (329). Nine
    // rows (360) scroll 360 + 5 - 234 = 131 (FUN_00609ae0).
    let rows: Vec<AlertRow> = (0..9)
        .map(|index| AlertRow {
            text: format!("row {index}"),
            font: AlertFont::Bold,
            picture: Vec::new(),
            indent: false,
        })
        .collect();
    let texts = |scroll| {
        battle_alert_list(&rows, scroll)
            .into_iter()
            .filter_map(|op| match op {
                PageOp::Text { text, baseline, .. } => Some((text, baseline)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    // 234 high: five rows and 34 pixels of a sixth.
    assert_eq!(texts(0).len(), 6);
    let scrolled = texts(120);
    // Drawn from the cell's top: 13 down for a 16 px cell.
    assert_eq!(scrolled[0], ("row 3".to_owned(), 53.0));
    // Scrolled 39, the first row's last pixel still shows.
    assert_eq!(texts(39)[0].0, "row 0");
    assert_eq!(texts(40)[0].0, "row 1");
    assert_eq!(scrolled.last().unwrap().0, "row 8");
    let ops = battle_alert_list(&rows, 0);
    assert_eq!(
        ops.first(),
        Some(&PageOp::Clip(Some((83.0, 40.0, 256.0, 234.0))))
    );
    assert_eq!(ops.last(), Some(&PageOp::Clip(None)));
    let bar = alert_list_bar(alert_list_content(9), 0);
    assert_eq!((bar.x, bar.max), (329.0, 131.0));
    // The 13-wide art sits at 10 / 2 - 13 / 2 = -1 from the bar.
    let bar_ops = alert_bar_ops(&bar, None);
    assert_eq!(
        bar_ops.first(),
        Some(&PageOp::Clip(Some((329.0, 40.0, 10.0, 234.0))))
    );
    assert!(bar_ops.contains(&PageOp::Bitmap {
        source: DllSource::Strategy,
        id: 10_750,
        x: 328.0,
        y: 40.0,
    }));
    // Five rows fit: no bar.
    assert!(alert_bar_ops(&alert_list_bar(alert_list_content(5), 0), None).is_empty());
}

#[test]
fn the_wheel_scrolls_the_alert_list_only_over_it_and_a_new_tab_resets_it() {
    // The list at window (83, 40), 256 by 234; the Alliance window at (62, 49)
    // (A0 TAC-01-C010). port: a wheel notch moves a row (40).
    let content = alert_list_content(9);
    let inside = (62.0 + 83.0, 49.0 + 40.0);
    assert_eq!(alert_list_scroll(0, inside, -1.0, content), 40);
    assert_eq!(alert_list_scroll(40, inside, 1.0, content), 0);
    assert_eq!(alert_list_scroll(120, inside, -1.0, content), 131);
    assert_eq!(alert_list_scroll(40, inside, 0.0, content), 40);
    let last = (62.0 + 83.0 + 255.5, 49.0 + 40.0 + 233.5);
    assert_eq!(alert_list_scroll(0, last, -1.0, content), 40);
    for outside in [
        (62.0 + 82.5, 100.0),
        (62.0 + 83.0 + 256.0, 100.0),
        (200.0, 49.0 + 39.5),
        (200.0, 49.0 + 40.0 + 234.0),
    ] {
        assert_eq!(alert_list_scroll(0, outside, -1.0, content), 0);
    }
    // Nothing to scroll: five rows fit.
    assert_eq!(alert_list_scroll(0, inside, -1.0, alert_list_content(5)), 0);
    let (mut tab, mut scroll) = (BattleAlertTab::AllianceForces, 2);
    select_resetting_scroll(&mut tab, &mut scroll, BattleAlertTab::AllianceForces);
    assert_eq!(scroll, 2);
    select_resetting_scroll(&mut tab, &mut scroll, BattleAlertTab::System);
    assert_eq!((tab, scroll), (BattleAlertTab::System, 0));
    // A click on the bar's down arrow steps the list's 10, drawn pressed
    // while held.
    let mut state = TacticalState::default();
    let origin = battle_window_origin();
    let arrow = (origin.0 + 333.0, origin.1 + 40.0 + 230.0);
    alert_list_step(
        (
            &mut state.battle_alert_scroll,
            &mut state.battle_alert_scroll_hold,
        ),
        content,
        arrow,
        (true, true, false),
        0.0,
        0.0,
    );
    assert_eq!(
        alert_pressed_arrow(&state, content, arrow, true),
        Some(crate::list_scroll_bar::Part::Down)
    );
    assert_eq!(alert_pressed_arrow(&state, content, arrow, false), None);
    assert_eq!(
        alert_pressed_arrow(&state, content, (origin.0 + 333.0, origin.1 + 150.0), true),
        None
    );
    alert_list_step(
        (
            &mut state.battle_alert_scroll,
            &mut state.battle_alert_scroll_hold,
        ),
        content,
        arrow,
        (false, false, true),
        0.0,
        0.1,
    );
    assert_eq!(state.battle_alert_scroll, 10);
}

#[test]
fn the_battle_alert_list_rows_are_forty_high_from_83_40() {
    // FUN_0044f860: FUN_00607ea0(.., 0x53, 0x28, 0x100, 0xea) with
    // rows 0x28 high; FUN_00450e10: text at 0x46 past a picture, 0x5a
    // when indented by 0x14.
    let text = |text: &str, font| AlertRow {
        text: text.into(),
        font,
        picture: Vec::new(),
        indent: false,
    };
    let picture = |indent| AlertRow {
        text: "A-wing".into(),
        font: AlertFont::Plain,
        picture: vec![(DllSource::Gokres, 7)],
        indent,
    };
    let mut rows = vec![
        text("Alliance Forces", AlertFont::Heading),
        picture(false),
        picture(true),
    ];
    rows.extend((0..5).map(|_| text("more", AlertFont::Bold)));
    // The list's clip steps aside, the rows' steps in order.
    let ops: Vec<PageOp> = battle_alert_list(&rows, 0)
        .into_iter()
        .filter(|op| !matches!(op, PageOp::Clip(_)))
        .collect();
    // Font 0xb from the cell's top (A0 alert-imperial-forces).
    assert_eq!(
        ops[0],
        PageOp::Text {
            text: "Alliance Forces".into(),
            x: 211.0,
            baseline: 59.0,
            size: BATTLE_WINDOW_FONT_B,
            color: WHITE,
            centered: true,
            bold: false,
        }
    );
    assert_eq!(
        ops[1],
        PageOp::Bitmap {
            source: DllSource::Gokres,
            id: 7,
            x: 83.0,
            y: 80.0
        }
    );
    assert!(
        matches!(&ops[2], PageOp::Text { x, baseline, centered: false, bold: false, .. } if *x == 153.0 && *baseline == 93.0)
    );
    assert_eq!(
        ops[3],
        PageOp::Bitmap {
            source: DllSource::Gokres,
            id: 7,
            x: 103.0,
            y: 120.0
        }
    );
    assert!(matches!(&ops[4], PageOp::Text { x, .. } if *x == 173.0));
    // Font 5 rows are bold.
    assert!(
        matches!(&ops[5], PageOp::Text { bold: true, size, .. } if *size == BATTLE_WINDOW_FONT_4)
    );
    // 234 high: the sixth row (top 240, middle 260) is the last whose
    // middle fits; the seventh is cut.
    let texts = ops
        .iter()
        .filter(|op| matches!(op, PageOp::Text { .. }))
        .count();
    assert_eq!(texts, 6);
}

#[test]
fn the_battle_alert_reads_the_system_side_and_last_arrival() {
    // FUN_0049c0d0: +0x24 bits 6..7 and +0x78 bits 6..7.
    use rebellion_core::dat::Faction;
    use rebellion_core::world::ControlKind;
    let mut world = GameWorld::default();
    let mut system = |control, last_arrival| test_system(&mut world, control, last_arrival);
    let rebel = system(
        ControlKind::Controlled(Faction::Alliance),
        Some(Faction::Empire),
    );
    let imperial = system(
        ControlKind::Controlled(Faction::Empire),
        Some(Faction::Alliance),
    );
    let neutral = system(ControlKind::Uncontrolled, None);
    assert_eq!(battle_alert_sides(&world, rebel), (Some(true), Some(false)));
    assert_eq!(
        battle_alert_sides(&world, imperial),
        (Some(false), Some(true))
    );
    assert_eq!(battle_alert_sides(&world, neutral), (None, None));
    assert_eq!(
        battle_alert_sides(&GameWorld::default(), rebel),
        (None, None)
    );
}

#[test]
fn each_battle_window_names_its_original_score() {
    // FUN_0044f860 (the alert) and FUN_0046f360 (the summary).
    let mut state = TacticalState::default();
    assert_eq!(state.screen_music(), None);
    let mut session = test_session(
        vec![test_ship(64, 1, true, true), test_ship(64, 2, false, true)],
        Vec::new(),
        true,
    );
    session.phase = BattlePhase::Combat;
    state.session = Some(session);
    assert_eq!(state.screen_music(), None);
    state.battle_alert_open = true;
    let viewer = player_is_alliance(state.session.as_ref().unwrap());
    assert_eq!(
        state.screen_music(),
        Some(crate::audio::battle_alert_music(viewer))
    );
    state.battle_alert_open = false;
    let session = state.session.as_mut().unwrap();
    session.phase = BattlePhase::Results;
    session.winner = Some(if viewer == session.attacker_is_alliance {
        CombatWinner::Attacker
    } else {
        CombatWinner::Defender
    });
    assert_eq!(
        state.screen_music(),
        Some(crate::audio::battle_summary_music(viewer, true))
    );
    let session = state.session.as_mut().unwrap();
    session.winner = Some(CombatWinner::Draw);
    assert_eq!(
        state.screen_music(),
        Some(crate::audio::battle_summary_music(viewer, false))
    );
}

#[test]
fn the_display_page_tells_the_viewer_where_its_fleet_is() {
    // FUN_0049b330 / FUN_0049b4d0 / FUN_0049b150: +0xac reads 0x7012
    // on the viewer's win, 0x7011 on its withdrawal (port: without
    // a destination found, no " to {3:1}"), else 0x7010; +0xb8 reads 0x7013 for two
    // buttons, 0x7014 for one. TEXTSTRA as shipped.
    let mut texts = battle_texts();
    for (id, text) in [
        (
            0x7010,
            "The {1:4} fleet was defeated at {2:1}. Your fleet has been destroyed. ",
        ),
        (
            0x7011,
            "The {1:4} fleet was defeated at {2:1}. Your fleet has withdrawn to {3:1}. ",
        ),
        (
            0x7012,
            "The {1:4} fleet is victorious. The world {2:1} is now under {1:4} blockade. ",
        ),
        (
            0x7013,
            "Select one of the following buttons to close this display and go directly to... ",
        ),
        (
            0x7014,
            "Select the following button to close this display and go directly to... ",
        ),
    ] {
        texts.insert(id, text.to_owned());
    }
    let display = |texts: &HashMap<u16, String>, winner, withdrawn| {
        battle_display_lines(
            texts,
            &BattleReport {
                system_name: "Deyer",
                viewer_alliance: false,
                winner_alliance: winner,
                withdrawn,
                destination: None,
                last_arrival: None,
                system_side: None,
            },
        )
    };
    let two = "Select one of the following buttons to close this display and go directly to...";
    let one = "Select the following button to close this display and go directly to...";
    assert_eq!(
        display(&texts, Some(false), Some(true)),
        (
            [
                "The Imperial fleet is victorious. The world Deyer is now under Imperial blockade."
                    .to_owned(),
                two.to_owned()
            ],
            true
        )
    );
    assert_eq!(
        display(&texts, Some(true), Some(false)),
        (
            [
                "The Imperial fleet was defeated at Deyer. Your fleet has withdrawn.".to_owned(),
                two.to_owned()
            ],
            true
        )
    );
    // No winner, the viewer withdrew: its fleet still has a button.
    assert!(display(&texts, None, Some(false)).1);
    let lost = (
        [
            "The Imperial fleet was defeated at Deyer. Your fleet has been destroyed.".to_owned(),
            one.to_owned(),
        ],
        false,
    );
    assert_eq!(display(&texts, Some(true), None), lost);
    // The enemy's withdrawal leaves the viewer's fleet destroyed.
    assert_eq!(display(&texts, None, Some(true)), lost);
    // Without the texts, the port's English says the same.
    assert_eq!(display(&HashMap::new(), Some(true), None), lost);
    assert_eq!(display(&HashMap::new(), Some(false), None).0[1], two);
    assert_eq!(
        display(&HashMap::new(), None, Some(false)).0[0],
        "The Imperial fleet was defeated at Deyer. Your fleet has withdrawn."
    );
    // With +0x48 the message names the destination (0x7011's {3:1}).
    let (lines, fleet) = battle_display_lines(
        &texts,
        &BattleReport {
            system_name: "Deyer",
            viewer_alliance: false,
            winner_alliance: Some(true),
            withdrawn: Some(false),
            destination: Some("Coruscant"),
            last_arrival: None,
            system_side: None,
        },
    );
    assert_eq!(
        lines[0],
        "The Imperial fleet was defeated at Deyer. Your fleet has withdrawn to Coruscant."
    );
    assert!(fleet);
}

#[test]
fn the_display_page_lays_out_its_buttons_like_the_original() {
    // FUN_0046f360: the lines at (24, 40) and (24, 130), 328 wide; the
    // system button 0xcb at (29, 182) with 0x2a0c, the fleet button
    // 0xca at (229, 182) with 0x2a0d / 0x2a0e by the viewer, pressed
    // 0x2a0f / 0x2a10; the labels at y 278, the fleet's 0x1942.
    let lines = ["message".to_owned(), "instruction".to_owned()];
    let red = Color::from_rgba(255, 0, 0, 255);
    let ops = battle_display_page("Deyer", &lines, true, true, Some(DisplayButton::Fleet), red);
    let bitmaps: Vec<(u32, f32, f32)> = ops
        .iter()
        .filter_map(|op| match op {
            PageOp::Bitmap { id, x, y, .. } => Some((*id, *x, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(
        bitmaps,
        [
            (10763, 12.0, 13.0),
            (10764, 29.0, 182.0),
            (10767, 229.0, 182.0)
        ]
    );
    let wrapped: Vec<(f32, f32, f32)> = ops
        .iter()
        .filter_map(|op| match op {
            PageOp::Wrapped {
                x,
                baseline,
                width,
                color,
                ..
            } => {
                assert_eq!(*color, red);
                Some((*x, *baseline, *width))
            }
            _ => None,
        })
        .collect();
    // A0 TAC-07-C013: font 6 capitals from window rows 43 and 133.
    assert_eq!(wrapped, [(24.0, 55.0, 328.0), (24.0, 145.0, 328.0)]);
    let labels: Vec<(&str, f32, f32)> = ops
        .iter()
        .filter_map(|op| match op {
            PageOp::Text {
                text,
                x,
                baseline,
                centered: true,
                color,
                ..
            } if !text.starts_with("Battle at") => {
                assert_eq!(*color, WHITE);
                Some((text.as_str(), *x, *baseline))
            }
            _ => None,
        })
        .collect();
    assert_eq!(labels, [("Deyer", 113.5, 293.0), ("Fleet", 313.5, 293.0)]);
    // FUN_0046f360 (0x46fa33): the second line in font 7, bold.
    let bolds: Vec<bool> = ops
        .iter()
        .filter_map(|op| match op {
            PageOp::Wrapped { bold, .. } => Some(*bold),
            _ => None,
        })
        .collect();
    assert_eq!(bolds, [false, true]);
    // A0 TAC-07-C013: the page keeps its title, font 0xb from row 22.
    assert!(ops.iter().any(|op| matches!(op,
        PageOp::Text { text, baseline, color, .. }
            if text == "Battle at Deyer" && *baseline == 37.0 && *color == red)));
    // The Empire's viewer: its fleet art and pressed system art.
    let ops = battle_display_page(
        "Deyer",
        &lines,
        false,
        true,
        Some(DisplayButton::System),
        red,
    );
    assert!(ops.contains(&PageOp::Bitmap {
        source: DllSource::Strategy,
        id: 10768,
        x: 29.0,
        y: 182.0
    }));
    assert!(ops.contains(&PageOp::Bitmap {
        source: DllSource::Strategy,
        id: 10766,
        x: 229.0,
        y: 182.0
    }));
    // No fleet: one button.
    let ops = battle_display_page("Deyer", &lines, false, false, None, red);
    assert!(!ops
        .iter()
        .any(|op| matches!(op, PageOp::Bitmap { x, .. } if *x == 229.0)));
    assert!(!ops
        .iter()
        .any(|op| matches!(op, PageOp::Text { text, .. } if text == "Fleet")));
}

#[test]
fn the_display_buttons_answer_inside_their_rectangles() {
    assert_eq!(
        display_button_at(29.0, 182.0, false),
        Some(DisplayButton::System)
    );
    assert_eq!(
        display_button_at(197.9, 277.9, true),
        Some(DisplayButton::System)
    );
    assert_eq!(display_button_at(198.0, 200.0, true), None);
    assert_eq!(display_button_at(28.9, 200.0, true), None);
    assert_eq!(display_button_at(100.0, 181.9, true), None);
    assert_eq!(display_button_at(100.0, 278.0, true), None);
    assert_eq!(
        display_button_at(229.0, 182.0, true),
        Some(DisplayButton::Fleet)
    );
    assert_eq!(
        display_button_at(397.9, 277.9, true),
        Some(DisplayButton::Fleet)
    );
    assert_eq!(display_button_at(300.0, 200.0, false), None);
}

#[test]
fn the_summary_windows_last_row_is_painted_black_across_its_width() {
    // A0 TAC-07-C001..C009 and 2026-11-18-edited-saves-7 d-14: row 330
    // of the 470 x 331 window is black under the frame's blue row.
    for alliance in [true, false] {
        let window = battle_summary_window_rect(alliance);
        let row = BATTLE_WINDOW_UNPAINTED_ROW;
        assert_eq!((row.x, row.width), (0.0, window.w));
        assert_eq!((row.y + row.height, row.height), (window.h, 1.0));
    }
}

#[test]
fn the_summary_frame_lies_over_the_scene_and_under_the_texts() {
    // A0 TAC-01-C006 / TAC-01-C013 (Alliance) and empire-game3/
    // battle-result-coruscant (Empire) show the frame's corner arcs
    // (10335 / 10336, keyed on blue) over the scene at (12, 13).
    let lines = vec!["The Imperial fleet is victorious. ".to_owned()];
    let white = Color::from_rgba(255, 255, 255, 255);
    for (alliance, frame) in [(true, 10_335), (false, 10_336)] {
        assert_eq!(battle_result_skin(alliance).frame, frame);
        let scene = resources::strategy::EVENT_BATTLE_EMPIRE_VICTORY;
        let ops = frame_over_page(battle_summary_page("Hoth", scene, &lines, white), frame);
        let bitmaps: Vec<u32> = ops
            .iter()
            .filter_map(|op| match op {
                PageOp::Bitmap { id, .. } => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(bitmaps, [scene, frame]);
        assert_eq!(
            ops[1],
            PageOp::Bitmap {
                source: DllSource::Strategy,
                id: frame,
                x: 0.0,
                y: 0.0
            }
        );
        assert!(matches!(ops[2], PageOp::Text { .. }), "{ops:?}");
    }
}

#[test]
fn the_summary_page_heads_the_outcome_over_its_shadowed_lines() {
    // FUN_0046f360 (guide-059): the title at (12, 18) and the first
    // line at (12, 225), 400 wide, centred; the rest at (24, 255), 352
    // wide, over a black shadow at (25, 256).
    let lines = vec![
        "The Imperial fleet is victorious.".to_owned(),
        "Hoth is now under blockade by Imperial forces.".to_owned(),
        "The Alliance fleet has been completely destroyed.".to_owned(),
    ];
    let green = Color::from_rgba(0, 255, 0, 255);
    let ops = battle_summary_page(
        "Hoth",
        resources::strategy::EVENT_BATTLE_EMPIRE_VICTORY,
        &lines,
        green,
    );
    assert_eq!(
        ops[0],
        PageOp::Bitmap {
            source: DllSource::Strategy,
            id: resources::strategy::EVENT_BATTLE_EMPIRE_VICTORY,
            x: 12.0,
            y: 13.0
        }
    );
    assert_eq!(
        ops[2],
        // A0 TAC-07-C001: font 0xb capitals from window row 229.
        PageOp::Text {
            text: lines[0].clone(),
            x: 212.0,
            baseline: 244.0,
            size: BATTLE_WINDOW_FONT_B,
            color: green,
            centered: true,
            bold: false,
        }
    );
    assert_eq!(
        ops[3],
        PageOp::Wrapped {
            lines: lines[1..].to_vec(),
            x: 25.0,
            baseline: 271.0,
            width: 352.0,
            size: BATTLE_WINDOW_FONT_6,
            color: BLACK,
            bold: false,
        }
    );
    assert_eq!(
        ops[4],
        PageOp::Wrapped {
            lines: lines[1..].to_vec(),
            x: 24.0,
            baseline: 270.0,
            width: 352.0,
            size: BATTLE_WINDOW_FONT_6,
            color: green,
            bold: false,
        }
    );
    // A0 TAC-07-C001: font 0xb capitals from window row 22.
    assert!(
        matches!(&ops[1], PageOp::Text { text, baseline, .. } if text == "Battle at Hoth" && *baseline == 37.0)
    );
    // No outcome, no lines: the scene and the title only.
    let ops = battle_summary_page(
        "Hoth",
        resources::strategy::EVENT_BATTLE_MUTUAL_DESTRUCTION,
        &[],
        green,
    );
    assert_eq!(ops.len(), 2);
    assert!(
        matches!(ops[0], PageOp::Bitmap { id, .. } if id == resources::strategy::EVENT_BATTLE_MUTUAL_DESTRUCTION)
    );
}

#[test]
fn the_summary_scene_follows_the_viewer_and_the_withdrawal() {
    // FUN_0046f360: 0x2d63 with no winner; 0x2a05..0x2a08 by the
    // viewer's win, withdrawal or loss. guide-059 is the Empire's win
    // (0x2a08, 10760).
    let scene = |viewer, winner, withdrawn| {
        battle_summary_scene(&BattleReport {
            system_name: "Deyer",
            viewer_alliance: viewer,
            winner_alliance: winner,
            withdrawn,
            destination: None,
            last_arrival: None,
            system_side: None,
        })
    };
    assert_eq!(scene(false, Some(false), None), 10_760);
    assert_eq!(scene(true, Some(true), None), 10_759);
    assert_eq!(scene(true, Some(true), Some(false)), 10_758);
    assert_eq!(scene(false, Some(false), Some(true)), 10_757);
    assert_eq!(scene(true, Some(false), Some(true)), 10_757);
    assert_eq!(scene(false, Some(true), Some(false)), 10_758);
    assert_eq!(scene(true, Some(false), None), 10_760);
    assert_eq!(scene(false, Some(true), None), 10_759);
    assert_eq!(scene(true, None, Some(true)), 11_619);
    assert_eq!(scene(false, None, None), 11_619);
}

#[test]
fn the_battle_summary_category_bar_follows_the_original() {
    // FUN_0046f360: the bar at (130, 59), 49 by 41 buttons, the
    // viewer's art; the labels TEXTSTRA 0x1928..0x1931.
    assert_eq!(
        battle_category_rect(0),
        NativeRect::new(130.0, 59.0, 49.0, 41.0)
    );
    assert_eq!(
        battle_category_rect(3),
        NativeRect::new(277.0, 59.0, 49.0, 41.0)
    );
    assert_eq!(
        battle_category_art(BattleResultCategory::CapitalShips, true),
        (0x286c, 0x286b)
    );
    assert_eq!(
        battle_category_art(BattleResultCategory::Fighters, false),
        (0x287c, 0x287b)
    );
    assert_eq!(
        battle_category_art(BattleResultCategory::Troops, true),
        (0x2870, 0x286f)
    );
    assert_eq!(
        battle_category_art(BattleResultCategory::Personnel, false),
        (0x2876, 0x2875)
    );
    assert_eq!(BattleResultCategory::Troops.title(), "Trooper Regiments");
    assert_eq!(BattleResultCategory::Fighters.title(), "Fighter Squadrons");
}

#[test]
fn auto_resolve_enters_results_and_projects_losses_once() {
    let mut first_ship = test_ship(64, 1, true, true);
    first_ship.hull_current = 100;
    first_ship.hull_max = 100;
    let mut second_ship = test_ship(64, 2, true, true);
    second_ship.hull_current = 100;
    second_ship.hull_max = 100;
    let mut fighters = vec![test_fighter(1, true), test_fighter(1, true)];
    fighters[1].fleet_squadron_index = 1;
    let session = test_session(vec![first_ship, second_ship], fighters, true);
    let attacker_fleet = session.attacker_fleet;
    let defender_fleet = session.defender_fleet;
    let system = session.system;
    let mut state = TacticalState {
        session: Some(session),
        ..TacticalState::default()
    };
    let result = SpaceCombatResult {
        attacker_fleet,
        defender_fleet,
        system,
        winner: AutoResolveSide::Attacker,
        ship_damage: vec![rebellion_core::combat::ShipDamageEvent {
            fleet: attacker_fleet,
            ship_index: 0,
            hull_before: 100,
            hull_after: 0,
            hits_after: 0,
        }],
        fighter_losses: vec![rebellion_core::combat::FighterLossEvent {
            fleet: attacker_fleet,
            fighter_index: 0,
            squads_before: 2,
            squads_after: 1,
            craft_after: Vec::new(),
        }],
        tick: 0,
    };

    assert!(state.present_auto_resolve_result(&result));
    assert!(state.strategic_results_applied());
    let result_session = state.session.as_mut().unwrap();
    assert_eq!(
        result_session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::battle_won(
            TacticalVoiceFaction::Alliance,
            false
        )]
    );
    assert_eq!(result_session.phase, BattlePhase::Results);
    assert_eq!(result_session.winner, Some(CombatWinner::Attacker));
    assert!(!result_session.ships[0].alive);
    assert!(result_session.ships[1].alive);
    assert!(result_session.fighters[0].alive);
    assert!(!result_session.fighters[1].alive);
}

#[test]
fn the_step_that_ends_the_battle_draws_the_battle_not_the_summary() {
    // FUN_0040a050 state 0x16 closes the tactical module (FUN_005975a0)
    // before FUN_00488b90 -> FUN_0042a640 creates the summary on the
    // Command Center: the original never shows it in the tactical view.
    assert!(!battle_frame_draws_summary(false));
    // A battle already in its results when the frame began (a caller
    // that keeps it in the tactical view) still shows its window.
    assert!(battle_frame_draws_summary(true));
}

#[test]
fn escape_closes_the_battle_summary_and_the_arrows_step_its_category_bar() {
    // FUN_004710f0 (vtable 0x65a300 +0x4c): 0x1b closes the window as
    // the Close button 0xc9 does (slot +0x30, FUN_0041ce80); 0x25 / 0x27
    // press the bar +0x160's previous / next visible button, or its
    // first when the selected one has no neighbour that way.
    assert_eq!(
        battle_summary_key(true, true, false),
        Some(BattleSummaryKey::Close)
    );
    assert_eq!(
        battle_summary_key(false, true, false),
        Some(BattleSummaryKey::Step { right: false })
    );
    assert_eq!(
        battle_summary_key(false, false, true),
        Some(BattleSummaryKey::Step { right: true })
    );
    assert_eq!(battle_summary_key(false, false, false), None);

    use BattleResultCategory::{CapitalShips, Fighters, Personnel, Troops};
    assert_eq!(
        battle_category_after_arrow(Fighters, true, true),
        Some(Troops)
    );
    assert_eq!(
        battle_category_after_arrow(Fighters, false, true),
        Some(CapitalShips)
    );
    // No neighbour that way: the first button.
    assert_eq!(
        battle_category_after_arrow(Personnel, true, true),
        Some(CapitalShips)
    );
    assert_eq!(
        battle_category_after_arrow(CapitalShips, false, true),
        Some(CapitalShips)
    );
    // The bar hidden (the summary and Display pages): a neighbour is
    // skipped as invisible, so nothing changes, but an end still
    // selects the first.
    assert_eq!(battle_category_after_arrow(Fighters, true, false), None);
    assert_eq!(
        battle_category_after_arrow(Personnel, true, false),
        Some(CapitalShips)
    );
}

#[test]
fn the_battle_summary_becomes_a_470_by_331_strategy_window_when_the_battle_ends() {
    // FUN_00488b90 -> FUN_0041d730 -> FUN_0042a640 creates the summary
    // once the battle's result is in, as a child window of the Command
    // Center (DAT_006b14cc); FUN_0046f140 gives it 0x1d6 by 0x14b. A0
    // TAC-01-C013 shows it at (62, 49) over the Alliance's Command
    // Center, A0 TAC-07-C002 the Empire's battle windows at (125, 52).
    assert_eq!(
        battle_summary_window_rect(true),
        Rect::new(62.0, 49.0, 470.0, 331.0)
    );
    assert_eq!(
        battle_summary_window_rect(false),
        macroquad::prelude::Rect::new(125.0, 52.0, 470.0, 331.0)
    );

    let mut state = TacticalState {
        session: Some(test_session(Vec::new(), Vec::new(), true)),
        ..TacticalState::default()
    };
    let native = (640.0, 480.0);
    // In the battle there is no summary.
    assert!(!state.battle_summary_open());
    assert!(!state.battle_summary_contains_screen_point(native, (100.0, 100.0)));

    state.session.as_mut().unwrap().phase = BattlePhase::Results;
    assert!(state.battle_summary_open());
    // The Alliance's window, edges included and excluded.
    for (point, inside) in [
        ((62.0, 49.0), true),
        ((61.0, 49.0), false),
        ((62.0, 48.0), false),
        ((531.0, 379.0), true),
        ((532.0, 379.0), false),
        ((531.0, 380.0), false),
    ] {
        assert_eq!(
            state.battle_summary_contains_screen_point(native, point),
            inside,
            "{point:?}"
        );
    }
    // Twice the size: the strategy screen's fit doubles it.
    assert_eq!(
        state.battle_summary_screen_rect((1280.0, 960.0)),
        Some(Rect::new(124.0, 98.0, 940.0, 662.0))
    );
    // A wider screen centres the 640 x 480 fit: 80 px each side.
    assert_eq!(
        state.battle_summary_screen_rect((800.0, 480.0)),
        Some(Rect::new(142.0, 49.0, 470.0, 331.0))
    );
    assert!(state.battle_summary_contains_screen_point((1280.0, 960.0), (124.0, 98.0)));
    assert!(!state.battle_summary_contains_screen_point((1280.0, 960.0), (122.0, 98.0)));

    // The Empire's viewer: its window opens at (125, 52).
    state.session.as_mut().unwrap().player_is_attacker = false;
    assert!(!state.battle_summary_contains_screen_point(native, (100.0, 100.0)));
    assert!(state.battle_summary_contains_screen_point(native, (125.0, 52.0)));

    // The Battle Alert is the battle's other window, not the summary.
    state.battle_alert_open = true;
    assert!(!state.battle_summary_open());
    assert_eq!(state.battle_summary_screen_rect(native), None);
    assert!(!state.battle_summary_contains_screen_point(native, (125.0, 52.0)));
}

#[test]
fn a_simulated_retreat_whose_fleet_escaped_reports_its_withdrawal() {
    // A0 message-rail/retreat-result-commenor (Retreat on the Battle
    // Alert): "The Alliance fleet is defeated." over "Commenor is now
    // under blockade by Imperial forces. The Alliance fleet has
    // withdrawn to Bortras." The abstract battle's escaped hull stays
    // whole (FUN_005af910); the report's +0x54 is the loser that left
    // (FUN_0049b060, FUN_0050c6c0), so 0x700d follows, not 0x700b.
    let mut escaped = test_ship(64, 1, true, true);
    escaped.hull_current = 100;
    escaped.hull_max = 100;
    escaped.shield = 7;
    // A squadron that left with the fleet keeps its craft.
    let session = test_session(
        vec![escaped, test_ship(64, 2, false, true)],
        vec![test_fighter(1, true)],
        true,
    );
    let attacker_fleet = session.attacker_fleet;
    let defender_fleet = session.defender_fleet;
    let system = session.system;
    let mut state = TacticalState {
        session: Some(session),
        ..TacticalState::default()
    };
    // No report before the battle has its result.
    assert_eq!(state.battle_summary_lines(&GameWorld::default()), None);
    assert!(state.present_auto_resolve_result(&SpaceCombatResult {
        attacker_fleet,
        defender_fleet,
        system,
        winner: AutoResolveSide::Defender,
        // Hit on its way out, yet whole.
        ship_damage: vec![rebellion_core::combat::ShipDamageEvent {
            fleet: attacker_fleet,
            ship_index: 0,
            hull_before: 100,
            hull_after: 40,
            hits_after: 0,
        }],
        fighter_losses: vec![rebellion_core::combat::FighterLossEvent {
            fleet: attacker_fleet,
            fighter_index: 0,
            squads_before: 1,
            squads_after: 1,
            craft_after: Vec::new(),
        }],
        tick: 0,
    }));
    // The app logs the same lines the summary page draws, once.
    let logged = state.take_summary_log(&GameWorld::default()).unwrap();
    assert_eq!(state.take_summary_log(&GameWorld::default()), None);
    assert_eq!(
        state.battle_summary_lines(&GameWorld::default()).as_ref(),
        Some(&logged)
    );
    assert!(
        logged.iter().any(|line| line.contains("withdrawn")),
        "{logged:?}"
    );
    let session = state.session.as_ref().unwrap();
    assert!(session.ships[0].alive && session.ships[0].retreated);
    assert_eq!(
        (session.ships[0].hull_current, session.ships[0].shield),
        (40, 7)
    );
    assert!(!session.ships[1].retreated);
    assert!(session.fighters[0].alive && session.fighters[0].squad_count == 12);
    let world = GameWorld::default();
    let report = battle_report(session, &world, Some(true));
    assert_eq!(report.withdrawn, Some(true));
    assert_eq!(
        battle_summary_scene(&report),
        resources::strategy::EVENT_BATTLE_ALLIANCE_DEFEATED
    );
    let lines = battle_report_lines(
        &battle_texts(),
        &BattleReport {
            destination: Some("Bortras"),
            system_name: "Commenor",
            last_arrival: Some(false),
            ..report
        },
    );
    assert_eq!(
        lines,
        [
            "The Alliance fleet is defeated. ",
            "Commenor is now under blockade by Imperial forces. The Alliance fleet has withdrawn to Bortras."
        ]
    );
}

#[test]
fn a_simulated_battle_whose_loser_died_reports_no_withdrawal() {
    // A0 TAC-01-C013: the last Alliance hull died, so the report reads
    // 0x700b "has been completely destroyed" (FUN_0049b4d0).
    let mut lost = test_ship(64, 1, true, true);
    lost.hull_current = 100;
    lost.hull_max = 100;
    let session = test_session(vec![lost, test_ship(64, 2, false, true)], Vec::new(), true);
    let attacker_fleet = session.attacker_fleet;
    let mut state = TacticalState {
        session: Some(session.clone()),
        ..TacticalState::default()
    };
    assert!(state.present_auto_resolve_result(&SpaceCombatResult {
        attacker_fleet,
        defender_fleet: session.defender_fleet,
        system: session.system,
        winner: AutoResolveSide::Defender,
        ship_damage: vec![rebellion_core::combat::ShipDamageEvent {
            fleet: attacker_fleet,
            ship_index: 0,
            hull_before: 100,
            hull_after: 0,
            hits_after: 0,
        }],
        fighter_losses: Vec::new(),
        tick: 0,
    }));
    let session = state.session.as_ref().unwrap();
    assert!(!session.ships[0].retreated && !session.ships[1].retreated);
    assert_eq!(
        battle_report(session, &GameWorld::default(), None).withdrawn,
        None
    );
}

// Source: FUN_005c1360 calls the scene object's vtable 0x24. The ship
// class 0x66d198 has FUN_005d3ac0 there (0x66d1bc); the fighter group
// class 0x66d1f8 (FUN_005d43c0) has 0x403ef0, a bare `ret 8`.
#[test]
fn a_fighter_group_held_by_a_field_shows_no_cloud() {
    assert!(shows_field_cloud(TacticalUnitRef::Ship(0)));
    assert!(!shows_field_cloud(TacticalUnitRef::Fighter(0)));
}

// Source: FUN_005d3e90 effect dispatcher; FUN_005a7500 hit ordering;
// FUN_005d39a0 decal origins and sizes (0x5d39a0..0x5d3aab: 5.0, x 0.5
// for the hits, x 1.5 for 3360; 3300 takes (64, 64) at 0x5d3a9a).
#[test]
fn original_effect_dispatch_preserves_resources_sizes_and_priority_groups() {
    let cases = [
        (
            WeaponKind::LaserCannon,
            OriginalTacticalImpactStage::Hit,
            OriginalTacticalEffectSequence::StandardHit,
            3060,
            6,
            ((32, 32), 2.5),
        ),
        (
            WeaponKind::IonCannon,
            OriginalTacticalImpactStage::Hit,
            OriginalTacticalEffectSequence::IonHit,
            3180,
            6,
            ((32, 32), 2.5),
        ),
        (
            WeaponKind::LaserCannon,
            OriginalTacticalImpactStage::Damage,
            OriginalTacticalEffectSequence::StandardDamage,
            3120,
            7,
            ((32, 32), 5.0),
        ),
        (
            WeaponKind::IonCannon,
            OriginalTacticalImpactStage::Damage,
            OriginalTacticalEffectSequence::IonDamage,
            3240,
            16,
            ((64, 32), 5.0),
        ),
        (
            WeaponKind::Turbolaser,
            OriginalTacticalImpactStage::Damage,
            OriginalTacticalEffectSequence::TurbolaserDamage,
            3300,
            7,
            ((64, 64), 5.0),
        ),
        (
            WeaponKind::LaserCannon,
            OriginalTacticalImpactStage::Destroyed,
            OriginalTacticalEffectSequence::Destroyed,
            3360,
            16,
            ((64, 64), 7.5),
        ),
    ];
    for (weapon, stage, expected, base, frames, (origin, size)) in cases {
        let actual = original_tactical_effect_sequence(weapon, stage);
        assert_eq!(actual, expected);
        assert_eq!(actual.resource_base(), base);
        assert_eq!(actual.frame_count(), frames);
        assert_eq!(actual.decal_origin(), origin);
        assert_eq!(actual.decal_size(), size);
    }

    let mut effects = Vec::new();
    assert!(queue_original_tactical_impact(
        &mut effects,
        4,
        WeaponKind::IonCannon,
        OriginalTacticalImpactStage::Hit,
    ));
    assert!(!queue_original_tactical_impact(
        &mut effects,
        4,
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Hit,
    ));
    assert!(queue_original_tactical_impact(
        &mut effects,
        4,
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Damage,
    ));
    assert!(!queue_original_tactical_impact(
        &mut effects,
        4,
        WeaponKind::Turbolaser,
        OriginalTacticalImpactStage::Damage,
    ));
    assert!(queue_original_tactical_impact(
        &mut effects,
        4,
        WeaponKind::IonCannon,
        OriginalTacticalImpactStage::Destroyed,
    ));
    assert_eq!(
        effects[0].sequence,
        OriginalTacticalEffectSequence::Destroyed
    );
}

/// Step one original render call at a time and record what each call
/// leaves on screen: the type-303 resource, or `None` once removed.
fn impact_frames_per_render_call(
    weapon: WeaponKind,
    stage: OriginalTacticalImpactStage,
    calls: usize,
) -> Vec<Option<(u32, bool)>> {
    let mut session = test_session(Vec::new(), Vec::new(), true);
    queue_original_tactical_impact(&mut session.impact_effects, 0, weapon, stage);
    (0..calls)
        .map(|_| {
            session.advance_presentational_effects(ORIGINAL_TACTICAL_RENDER_SECONDS);
            session
                .impact_effects
                .first()
                .map(|effect| (effect.resource_id(), effect.hull_shown()))
        })
        .collect()
}

// Source: FUN_005d41a0 adds one to the +0x140 counter on every call
// (0x5d437c) but swaps the image to base + counter only once the +0xf0
// timer passes _DAT_0066d154 (0.1 s, 0x5d422d); FUN_005c25b0 calls it
// through FUN_005c2c80 once per scene render, and FUN_005c1cf0 holds
// each render until _DAT_0066c954 (0.02 s) has passed.
#[test]
fn a_destroyed_hull_shows_every_fifth_fireball_frame_at_the_render_cap() {
    let frames = impact_frames_per_render_call(
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Destroyed,
        18,
    );
    let resources = frames
        .iter()
        .map(|frame| frame.map(|(resource, _)| resource))
        .collect::<Vec<_>>();
    let mut expected = vec![Some(3360); 5];
    expected.extend([Some(3365); 5]);
    expected.extend([Some(3370); 5]);
    expected.push(Some(3375));
    expected.extend([None, None]);
    assert_eq!(resources, expected);
}

// Source: FUN_005d41a0 0x5d434f..0x5d4379: with the destroyed bit (0x80)
// set and the counter at 11 it calls vtable 0x24 (1, 0) and (2, 0) and
// vtable 4, FUN_005d3180, whose vtable 0x30 (FUN_005d3420) deletes the
// hull's mesh visual. The fireball runs on to the count of 16.
#[test]
fn a_destroyed_hull_stays_under_its_fireball_until_the_eleventh_render_call() {
    let frames = impact_frames_per_render_call(
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Destroyed,
        16,
    );
    let hulls = frames
        .iter()
        .map(|frame| frame.map(|(_, hull)| hull))
        .collect::<Vec<_>>();
    let mut expected = vec![Some(true); 11];
    expected.extend([Some(false); 5]);
    assert_eq!(hulls, expected);
}

/// Run the earlier `stages` on ship 0 for `renders_between` renders each,
/// then kill it and record what each later render leaves on screen.
fn death_frames_after_running_sequences(
    stages: &[(WeaponKind, OriginalTacticalImpactStage)],
    renders_between: usize,
    renders: usize,
) -> Vec<Option<(u32, bool)>> {
    let mut session = test_session(Vec::new(), Vec::new(), true);
    for &(weapon, stage) in stages {
        assert!(queue_original_tactical_impact(
            &mut session.impact_effects,
            0,
            weapon,
            stage,
        ));
        for _ in 0..renders_between {
            session.advance_presentational_effects(ORIGINAL_TACTICAL_RENDER_SECONDS);
        }
    }
    assert!(queue_original_tactical_impact(
        &mut session.impact_effects,
        0,
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Destroyed,
    ));
    (0..renders)
        .map(|_| {
            session.advance_presentational_effects(ORIGINAL_TACTICAL_RENDER_SECONDS);
            session
                .impact_effects
                .first()
                .map(|effect| (effect.resource_id(), effect.hull_shown()))
        })
        .collect()
}

// Source: FUN_005d3e90 adds a 0x14-byte node to the scene list +0x5e4 on
// every accepted state change (0x5d4013) and never removes the earlier
// one; FUN_005c2c80 calls FUN_005d41a0 once per node (0x5c2cd6) in each
// render. With a hit still running the dying ship has two nodes, so the
// +0x140 counter steps two per render; only a render's first call can
// pass the 0.1 s timer (0x5d422d), and the call at 11 deletes the hull
// (0x5d4358). ghidra/notes/tactical-effect-call-rate.md; footage
// RdDI7YURucY 29:23.2 shows 3360 then 3370 for about 0.2 s.
#[test]
fn a_ship_killed_during_a_hit_runs_its_fireball_twice_as_fast() {
    let frames = death_frames_after_running_sequences(
        &[(WeaponKind::LaserCannon, OriginalTacticalImpactStage::Hit)],
        2,
        9,
    );
    let mut expected = vec![Some((3360, true)); 5];
    expected.extend([Some((3370, false)); 3]);
    expected.push(None);
    assert_eq!(frames, expected);
}

// Source: as above, with a hit and then a damage sequence running there
// are three nodes (FUN_005d3e90 0x5d4013, FUN_005c2c80 0x5c2cd6). The
// fifth render's first call swaps to 3375, and its second finds the
// count of 16 and deletes the visual (FUN_005d41a0 vtable 0xa4).
#[test]
fn a_ship_killed_during_a_hit_and_a_damage_sequence_shows_only_the_first_fireball_frame() {
    let frames = death_frames_after_running_sequences(
        &[
            (WeaponKind::LaserCannon, OriginalTacticalImpactStage::Hit),
            (WeaponKind::LaserCannon, OriginalTacticalImpactStage::Damage),
        ],
        1,
        6,
    );
    let mut expected = vec![Some((3360, true)); 3];
    expected.extend([Some((3360, false)); 2]);
    expected.push(None);
    assert_eq!(frames, expected);
}

// Source: FUN_005d3e90 0x5d4013 adds one node per accepted state change;
// a rejected change (same exclusive group) adds none.
#[test]
fn each_accepted_replacement_adds_one_render_call() {
    let mut session = test_session(Vec::new(), Vec::new(), true);
    let mut queue = |stage| {
        queue_original_tactical_impact(
            &mut session.impact_effects,
            0,
            WeaponKind::LaserCannon,
            stage,
        )
    };
    assert!(queue(OriginalTacticalImpactStage::Hit));
    assert!(!queue(OriginalTacticalImpactStage::Hit));
    assert!(queue(OriginalTacticalImpactStage::Damage));
    assert!(queue(OriginalTacticalImpactStage::Destroyed));
    assert_eq!(session.impact_effects.len(), 1);
    assert_eq!(session.impact_effects[0].nodes, 3);
    session.advance_presentational_effects(ORIGINAL_TACTICAL_RENDER_SECONDS);
    assert_eq!(session.impact_effects[0].frame, 3);
}

// Source: FUN_005d3e90 sets the hit count to 6; FUN_005d41a0 timing as above.
#[test]
fn a_laser_hit_holds_its_first_frame_then_shows_its_last_for_one_render() {
    let frames =
        impact_frames_per_render_call(WeaponKind::LaserCannon, OriginalTacticalImpactStage::Hit, 7);
    let mut expected = vec![Some((3060, true)); 5];
    expected.push(Some((3065, true)));
    expected.push(None);
    assert_eq!(frames, expected);
}

// Source: FUN_005d41a0's call at 11 deletes a destroyed hull's mesh.
#[test]
fn the_scene_keeps_a_dead_hull_only_while_its_fireball_has_it() {
    let ships = vec![
        test_ship(1, 0, true, true),
        test_ship(2, 1, false, false),
        test_ship(3, 2, false, false),
    ];
    let mut session = test_session(ships, Vec::new(), true);
    queue_original_tactical_impact(
        &mut session.impact_effects,
        1,
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Destroyed,
    );
    queue_original_tactical_impact(
        &mut session.impact_effects,
        2,
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Damage,
    );
    assert_eq!(
        (0..4)
            .map(|index| hull_in_scene(&session, index))
            .collect::<Vec<_>>(),
        [true, true, false, false]
    );
    for _ in 0..12 {
        session.advance_presentational_effects(ORIGINAL_TACTICAL_RENDER_SECONDS);
    }
    assert!(hull_in_scene(&session, 0));
    assert!(!hull_in_scene(&session, 1));
}

/// A battle whose defender has just lost its last ship: the killing
/// hit's Destroyed fire is queued and the hull is out of the fight.
fn battle_with_the_last_defender_just_destroyed() -> BattleSession {
    let ships = vec![
        test_ship(64, 0, true, true),
        test_ship(128, 0, false, false),
    ];
    let mut session = test_session(ships, Vec::new(), true);
    session.paused = false;
    queue_original_tactical_impact(
        &mut session.impact_effects,
        1,
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Destroyed,
    );
    session
}

// Source: FUN_005a9ec0 0x005aa49e/0x005aa4b4 (state 5, +0xe70 =
// DAT_0066c244 4000.0); FUN_005bea30 0x005bebb6-0x005bebcf takes the
// turn length (1000) at each turn's end and ends at 0.0 (_DAT_0066c770).
// A0 TAC-03-C004 seq-t1: the result screen 4.8 s after the last fire.
#[test]
fn a_battle_ends_four_turn_ends_after_a_side_is_out_of_the_fight() {
    let mut session = battle_with_the_last_defender_just_destroyed();
    assert!(!session.step());
    assert_eq!(
        session.end_wait_ms,
        Some(rebellion_core::tactical::ORIGINAL_TACTICAL_END_WAIT_MILLISECONDS)
    );
    assert_eq!(session.phase, BattlePhase::Combat);
    assert_eq!(session.winner, None);
    // The outcome voice (+0xe98) is asked when the wait starts, once.
    let voices = session.take_pending_voice_cues();
    assert_eq!(
        voices,
        vec![TacticalVoiceCue::battle_won(
            session.player_voice_faction(),
            false
        )]
    );
    // Ticks 2..15 cross three turn ends (4, 8, 12); the fourth (16)
    // ends the battle.
    for _ in 2..16 {
        assert!(!session.step());
    }
    assert_eq!(session.end_wait_ms, Some(1000.0));
    assert!(session.take_pending_voice_cues().is_empty());
    assert!(session.step());
    assert_eq!(session.combat_tick, 16);
    assert_eq!(session.phase, BattlePhase::Results);
    assert_eq!(session.winner, Some(CombatWinner::Attacker));
}

// Source: the manager update that counts the end wait is skipped while
// paused (FUN_005aacb0 0x005aad2e), but the renders still run the fire
// (FUN_005c25b0 -> FUN_005c2c80 -> FUN_005d41a0). A0 TAC-03-C004
// (2026-11-19 run r6, t-03..t-11): the last Imperial ship's fire runs
// to its end under "Battle Paused" and the result comes after Resume.
#[test]
fn the_last_ships_fire_runs_out_while_paused_and_the_battle_waits() {
    let mut session = battle_with_the_last_defender_just_destroyed();
    session.step();
    session.paused = true;
    let mut previous_poses = None;
    assert!(hull_in_scene(&session, 1));
    for _ in 0..30 * 60 {
        advance_tactical_frame(&mut session, &mut previous_poses, 1.0 / 60.0, false);
    }
    assert!(session.impact_effects.is_empty());
    assert!(!hull_in_scene(&session, 1));
    assert_eq!(session.phase, BattlePhase::Combat);
    assert_eq!(
        session.end_wait_ms,
        Some(rebellion_core::tactical::ORIGINAL_TACTICAL_END_WAIT_MILLISECONDS)
    );
    // Resumed, the overdue turn begins at once and the wait runs on.
    session.paused = false;
    let mut frames = 0;
    while session.phase == BattlePhase::Combat {
        advance_tactical_frame(&mut session, &mut previous_poses, 1.0 / 60.0, false);
        frames += 1;
        assert!(frames < 5 * 60, "the battle ends within the wait");
    }
    assert_eq!(session.winner, Some(CombatWinner::Attacker));
}

// Source: FUN_005c1cf0 renders only after _DAT_0066c954 (0.02 s).
#[test]
fn impact_frames_wait_for_a_whole_render_period() {
    let mut session = test_session(Vec::new(), Vec::new(), true);
    queue_original_tactical_impact(
        &mut session.impact_effects,
        0,
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Hit,
    );
    session.advance_presentational_effects(0.019);
    assert_eq!(session.impact_effects[0].frame, 0);
    session.advance_presentational_effects(0.001_5);
    assert_eq!(session.impact_effects[0].frame, 1);
    session.advance_presentational_effects(0.5);
    assert!(session.impact_effects.is_empty());
}

// Source: FUN_005d3de0 / FUN_005b1ea0 / FUN_005b1f60 projectile profiles.
#[test]
fn original_projectile_profiles_preserve_thresholds_shapes_scales_colors_and_durations() {
    let cases = [
        (
            WeaponKind::LaserCannon,
            28.7,
            Some(0),
            true,
            (0.5, 1, 0, 1.0),
        ),
        (
            WeaponKind::LaserCannon,
            ORIGINAL_LASER_PROJECTILE_THRESHOLD,
            Some(0),
            true,
            (0.5, 2, 0, 1.0),
        ),
        (
            WeaponKind::Turbolaser,
            30.0,
            Some(15),
            false,
            (0.65, 1, 1, 1.0),
        ),
        (
            WeaponKind::Turbolaser,
            40.0,
            Some(14),
            false,
            (0.75, 2, 1, 1.0),
        ),
        (
            WeaponKind::Turbolaser,
            40.0,
            Some(15),
            false,
            (1.0, 3, 1, 2.0),
        ),
        (
            WeaponKind::IonCannon,
            ORIGINAL_ION_PROJECTILE_THRESHOLD,
            Some(0),
            true,
            (1.0, 3, 2, 2.0),
        ),
        (
            WeaponKind::FighterAttack,
            10.0,
            None,
            true,
            (0.2, 1, 0, 1.0),
        ),
    ];
    for (weapon, strength, ordinal, alliance, expected) in cases {
        let profile = original_projectile_profile(weapon, strength, ordinal, alliance);
        assert_eq!(
            (
                profile.longitudinal_scale,
                profile.shape_variant,
                profile.color_selector,
                profile.duration,
            ),
            expected
        );
    }
}

// Our own presentation between 250 ms steps; no source claim beyond the
// original's per-frame movement (0x5a9ec0).
#[test]
fn units_are_drawn_between_their_last_two_steps() {
    let from = TacticalWorldPosition {
        x: 0.0,
        y: 2.0,
        z: -10.0,
    };
    let to = TacticalWorldPosition {
        x: 4.0,
        y: 2.0,
        z: -6.0,
    };
    let east = TacticalWorldVector {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    let north = TacticalWorldVector::SOURCE_FORWARD;
    let (position, forward) = presented_pose(Some((from, east)), (to, north), 0.25);
    assert_eq!(
        position,
        TacticalWorldPosition {
            x: 1.0,
            y: 2.0,
            z: -9.0
        }
    );
    let expected = TacticalWorldVector {
        x: 0.75,
        y: 0.0,
        z: 0.25,
    }
    .normalized();
    assert!((forward.x - expected.x).abs() < 1e-6 && (forward.z - expected.z).abs() < 1e-6);
    assert_eq!(
        presented_pose(Some((from, east)), (to, north), 1.0),
        (to, north)
    );
    assert_eq!(presented_pose(Some((from, east)), (to, north), 0.0).0, from);
    assert_eq!(presented_pose(None, (to, north), 0.5), (to, north));
    // Poses are kept for the step after the one they were taken at only.
    let mut session = test_session(
        vec![test_ship(64, 0, true, true)],
        vec![test_fighter(1, true)],
        true,
    );
    session.ships[0].source_position = from;
    session.fighters[0].source_position = to;
    session.combat_tick = 7;
    let poses = TacticalPoses::of(&session);
    assert_eq!(poses.tick, 7);
    assert_eq!(poses.ships, vec![(from, session.ships[0].source_forward)]);
    assert_eq!(poses.fighters, vec![to]);
    assert!(poses.before_latest_step(&session).is_none());
    session.combat_tick = 8;
    assert!(poses.before_latest_step(&session).is_some());
    session.combat_tick = 9;
    assert!(poses.before_latest_step(&session).is_none());
    // A turn through the opposite direction falls back to the current.
    let south = TacticalWorldVector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    assert_eq!(presented_pose(Some((to, south)), (to, north), 0.5).1, north);
}

// Source: FUN_005ee590 / LAB_005eeb90 projectile interpolation.
#[test]
fn retained_projectile_interpolates_and_expires_by_source_duration() {
    let ships = vec![test_ship(64, 0, true, true), test_ship(128, 0, false, true)];
    let mut session = test_session(ships, Vec::new(), true);
    assert!(queue_original_tactical_projectile(
        &mut session.weapon_effects,
        &session.ships,
        0,
        1,
        WeaponKind::LaserCannon,
        ORIGINAL_LASER_PROJECTILE_THRESHOLD,
    ));
    assert_eq!(session.weapon_effects[0].shape_variant, 2);
    session.advance_presentational_effects(0.5);
    assert_eq!(session.weapon_effects[0].progress(), 0.5);
    session.advance_presentational_effects(0.499);
    assert_eq!(session.weapon_effects.len(), 1);
    session.advance_presentational_effects(0.002);
    assert!(session.weapon_effects.is_empty());
}

/// A paused battle with one live effect of each kind: a hit sequence,
/// a projectile, a tractor field and a fired superlaser beam.
fn paused_session_with_every_effect() -> BattleSession {
    let ships = vec![test_ship(64, 0, true, true), test_ship(128, 0, false, true)];
    let mut session = test_session(ships, Vec::new(), true);
    queue_original_tactical_impact(
        &mut session.impact_effects,
        1,
        WeaponKind::LaserCannon,
        OriginalTacticalImpactStage::Hit,
    );
    assert!(queue_original_tactical_projectile(
        &mut session.weapon_effects,
        &session.ships,
        0,
        1,
        WeaponKind::LaserCannon,
        ORIGINAL_LASER_PROJECTILE_THRESHOLD,
    ));
    assert!(set_original_tactical_field(
        &mut session.field_effects,
        0,
        TacticalUnitRef::Ship(1),
        OriginalTacticalFieldKind::Tractor,
        true,
    ));
    session.death_star_beam = Some(rebellion_core::tactical::TacticalDeathStarBeam {
        target: 1,
        source_position: TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        target_position: TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        fired: true,
        elapsed: 0.0,
    });
    session.paused = true;
    session
}

// Source: the Pause Battle command 0xad22 (FUN_0059b130) only counts
// +0x8c8 (FUN_005aac30/FUN_005aac60). The render FUN_005c25b0 still runs
// FUN_005c2c80 -> FUN_005d3cc0, FUN_005d41a0 and 0x005da830, none of
// which tests +0x8c8, and moves the scene, whose callback LAB_005eeb90
// carries each projectile on its own timer.
#[test]
fn a_paused_battle_keeps_its_effects_running_but_not_its_combat() {
    let mut session = paused_session_with_every_effect();
    session.voice_announcer.state = rebellion_core::tactical::TacticalVoiceAnnouncerState::Waiting;
    session.voice_announcer.countdown_ms = 1000.0;
    let mut previous_poses = None;
    advance_tactical_frame(&mut session, &mut previous_poses, 0.11, false);
    // FUN_005ada10 runs on every manager update, paused or not.
    assert_eq!(session.voice_announcer.countdown_ms, 890.0);
    assert!(session.impact_effects[0].frame > 0);
    assert_eq!(session.weapon_effects[0].progress(), 0.11);
    assert_eq!(session.field_effects[0].frame, 1);
    assert_eq!(session.death_star_beam.unwrap().elapsed, 0.11);
    assert_eq!(session.combat_tick, 0);
    assert_eq!(session.step_accumulator, 0.0);
    assert!(previous_poses.is_none());
}

#[test]
fn the_still_fixture_switch_holds_a_paused_battles_effects() {
    let mut session = paused_session_with_every_effect();
    let before = (
        session.impact_effects[0].frame,
        session.weapon_effects[0].elapsed,
        session.field_effects[0].frame,
    );
    advance_tactical_frame(&mut session, &mut None, 0.11, true);
    assert_eq!(
        (
            session.impact_effects[0].frame,
            session.weapon_effects[0].elapsed,
            session.field_effects[0].frame,
        ),
        before
    );
    assert_eq!(session.death_star_beam.unwrap().elapsed, 0.0);
    // Unpaused, the switch holds nothing.
    session.paused = false;
    advance_tactical_frame(&mut session, &mut None, 0.1, true);
    assert_eq!(session.weapon_effects[0].progress(), 0.1);
}

#[test]
fn a_player_battle_never_holds_its_effects_while_paused() {
    assert!(!TacticalState::default().freezes_paused_effects());
}

#[cfg(feature = "interface-test-fixtures")]
#[test]
fn the_fixture_switch_holds_effects_once_set() {
    let mut state = TacticalState::default();
    state.freeze_paused_effects_for_fixture();
    assert!(state.freezes_paused_effects());
}

#[test]
fn an_unpaused_frame_steps_combat_four_times_a_second_per_speed_unit() {
    let mut session = paused_session_with_every_effect();
    session.paused = false;
    session.combat_speed = 2;
    let mut previous_poses = None;
    advance_tactical_frame(&mut session, &mut previous_poses, 0.3, true);
    assert_eq!(session.combat_tick, 2);
    assert!((session.step_accumulator - 0.4).abs() < 1e-5);
    assert!(previous_poses.is_some());
    assert_eq!(session.death_star_beam.map(|beam| beam.elapsed), Some(0.3));
}

/// A battle whose Death Star beam has just started (turn 4's first
/// step, `0x005ba520`), with its target one turn from the kill.
fn session_with_a_beam_just_fired() -> BattleSession {
    let mut session = test_session(vec![test_ship(64, 0, true, true)], Vec::new(), false);
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 100.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });
    assert!(session.commit_death_star_shot(0));
    session.paused = false;
    for _ in 0..16 {
        session.step();
    }
    assert!(session.death_star_beam.unwrap().fired);
    session
}

fn advance_for(session: &mut BattleSession, poses: &mut Option<TacticalPoses>, seconds: f32) {
    // Renders of 0.02 s (FUN_005c1cf0).
    for _ in 0..(seconds / 0.02).round() as u32 {
        advance_tactical_frame(session, poses, 0.02, false);
    }
}

// Source: A0 TAC-03-C004, .artifacts/a0-vm/2026-11-18-edited-saves-8
// seq-p2-paused-beam and seq-p3-resume-kill (run 4): the beam started
// 0.20 s before Pause Battle; paused, it grew to full length (p-05, dt
// 0.810 s) and vanished with the target intact (p-06, dt 0.841 s), and
// nothing changed for 6 s (p-07). After Resume the target's fire showed
// within 0.13 s of the resume (k0048 button, k0076 first fire pixel).
// Trace: 0x005da830 grows the beam on its own timer; FUN_005aacb0
// skips the manager (0x005aad2e) while +0x8c8 is set; FUN_005bea30
// cuts an overdue turn to elapsed % 1000 + 1000 (0x005beb48).
#[test]
fn a_beam_paused_mid_flight_ends_while_paused_and_kills_on_resume() {
    let mut session = session_with_a_beam_just_fired();
    let mut poses = None;
    advance_for(&mut session, &mut poses, 0.2);
    let tick = session.combat_tick;
    let phase = session.step_accumulator;
    session.paused = true;
    advance_for(&mut session, &mut poses, 0.6);
    let beam = session.death_star_beam.unwrap();
    assert!(death_star_beam_progress(&beam).is_some_and(|share| share > 0.75));
    advance_for(&mut session, &mut poses, 0.3);
    assert_eq!(
        death_star_beam_progress(&session.death_star_beam.unwrap()),
        None
    );
    advance_for(&mut session, &mut poses, 6.0);
    // No combat step while paused: the target lives and nothing moves
    // (the drawn fraction stays where the pause left it).
    assert!(session.ships[0].alive);
    assert_eq!(session.combat_tick, tick);
    assert_eq!(session.step_accumulator, phase);
    session.paused = false;
    advance_tactical_frame(&mut session, &mut poses, 0.02, false);
    assert!(!session.ships[0].alive);
    assert_eq!(session.impact_effects.len(), 1);
}

// Source: A0 TAC-03-C004 seq-p1-paused-projectiles (run 3, p-01): Pause
// Battle 0.12 s after a laser shot; the projectiles moved on and were
// gone 0.89 s after the press, and no ship moved. Trace: LAB_005eeb90
// (FUN_005ee590) on its own timer +0x40, cleared once t passes +0x58.
#[test]
fn projectiles_in_flight_finish_while_paused_and_ships_hold_still() {
    let mut session = paused_session_with_every_effect();
    session.death_star_beam = None;
    let positions: Vec<_> = session
        .ships
        .iter()
        .map(|ship| ship.source_position)
        .collect();
    let mut poses = None;
    advance_for(&mut session, &mut poses, 0.12);
    assert_eq!(session.weapon_effects.len(), 1);
    advance_for(&mut session, &mut poses, 0.9);
    assert!(session.weapon_effects.is_empty());
    advance_for(&mut session, &mut poses, 4.0);
    let after: Vec<_> = session
        .ships
        .iter()
        .map(|ship| ship.source_position)
        .collect();
    assert_eq!(after, positions);
    assert_eq!(session.combat_tick, 0);
    assert!(poses.is_none());
}

// Source: FUN_005bea30 cuts an elapsed time of a turn or more to
// elapsed % turn + turn (0x005beb48-0x005beb52), then starts the next
// turn at now - remainder.
#[test]
fn a_long_pause_owes_one_turn_and_keeps_the_remainder_as_its_phase() {
    let mut session = test_session(vec![test_ship(64, 0, true, true)], Vec::new(), false);
    session.combat_tick = 5;
    session.step_accumulator = 0.5;
    // 1.5 steps of the turn done; 10.25 s paused is 41 steps.
    session.paused_turn_seconds = 10.25;
    session.resume_paused_turn_clock(4.0);
    // Phase 42.5 steps -> 42.5 % 4 + 4 = 6.5; one step was already run.
    assert_eq!(session.step_accumulator, 5.5);
    assert_eq!(session.paused_turn_seconds, 0.0);
    // A short pause is plain elapsed time.
    session.step_accumulator = 0.5;
    session.paused_turn_seconds = 0.25;
    session.resume_paused_turn_clock(4.0);
    assert_eq!(session.step_accumulator, 1.5);
    // Exactly a turn is overdue: the turn begins now, phase 0.
    session.step_accumulator = 0.0;
    session.paused_turn_seconds = 0.75;
    session.resume_paused_turn_clock(4.0);
    assert_eq!(session.step_accumulator, 3.0);
    // No pause leaves the clock alone.
    session.resume_paused_turn_clock(4.0);
    assert_eq!(session.step_accumulator, 3.0);
}

// Source: FUN_005bea30's first update (+0x8c0 set) stores the turn
// start +0x89c (0x005bea8a); before it there is no clock to run on.
#[test]
fn a_pause_before_the_first_combat_frame_owes_no_turn() {
    let mut session = test_session(vec![test_ship(64, 0, true, true)], Vec::new(), true);
    assert!(!session.turn_clock_started());
    let mut poses = None;
    advance_for(&mut session, &mut poses, 3.0);
    assert_eq!(session.paused_turn_seconds, 0.0);
    session.paused = false;
    advance_tactical_frame(&mut session, &mut poses, 0.02, false);
    assert!(session.turn_clock_started());
    assert_eq!(session.combat_tick, 0);
    session.combat_tick = 1;
    session.step_accumulator = 0.0;
    assert!(session.turn_clock_started());
    // The fixture switch holds the clock with the effects.
    session.paused = true;
    advance_tactical_frame(&mut session, &mut poses, 0.5, true);
    assert_eq!(session.paused_turn_seconds, 0.0);
    advance_tactical_frame(&mut session, &mut poses, 0.5, false);
    assert_eq!(session.paused_turn_seconds, 0.5);
}

// Source: FUN_005b24d0 through FUN_005b2550 field effects.
#[test]
fn gravity_field_preempts_tractor_then_restores_it_at_ten_hertz() {
    let mut session = test_session(Vec::new(), Vec::new(), true);
    assert!(set_original_tactical_field(
        &mut session.field_effects,
        1,
        TacticalUnitRef::Ship(4),
        OriginalTacticalFieldKind::Tractor,
        true,
    ));
    assert_eq!(session.field_effects[0].resource_id(), Some(3620));
    session.advance_presentational_effects(0.1);
    assert_eq!(session.field_effects[0].resource_id(), Some(3621));
    assert!(set_original_tactical_field(
        &mut session.field_effects,
        2,
        TacticalUnitRef::Ship(4),
        OriginalTacticalFieldKind::Gravity,
        true,
    ));
    assert_eq!(session.field_effects[0].resource_id(), Some(3520));
    assert!(set_original_tactical_field(
        &mut session.field_effects,
        3,
        TacticalUnitRef::Ship(4),
        OriginalTacticalFieldKind::Tractor,
        true,
    ));
    assert_eq!(session.field_effects[0].source_counts(), (2, 1));
    assert!(set_original_tactical_field(
        &mut session.field_effects,
        2,
        TacticalUnitRef::Ship(4),
        OriginalTacticalFieldKind::Gravity,
        false,
    ));
    assert_eq!(session.field_effects[0].resource_id(), Some(3620));
    assert!(set_original_tactical_field(
        &mut session.field_effects,
        1,
        TacticalUnitRef::Ship(4),
        OriginalTacticalFieldKind::Tractor,
        false,
    ));
    assert!(set_original_tactical_field(
        &mut session.field_effects,
        3,
        TacticalUnitRef::Ship(4),
        OriginalTacticalFieldKind::Tractor,
        false,
    ));
    assert!(session.field_effects.is_empty());
}

// Source: FUN_005b24d0 through FUN_005b2550 field source limits.
#[test]
fn exact_field_sources_enforce_one_tractor_and_four_gravity_targets() {
    let ships = (0..6)
        .map(|index| test_ship(64, index, true, true))
        .collect();
    let mut session = test_session(ships, Vec::new(), true);

    assert!(session.set_tractor_field(0, 1, true));
    assert!(!session.set_tractor_field(0, 1, true));
    assert!(!session.set_tractor_field(0, 2, true));
    assert!(!session.set_tractor_field(2, 1, false));
    assert!(session.set_tractor_field(0, 1, false));
    assert!(session.set_tractor_field(0, 2, true));

    for target in 1..=4 {
        assert!(session.set_gravity_field(0, target, true));
    }
    assert!(!session.set_gravity_field(0, 1, true));
    assert!(!session.set_gravity_field(0, 5, true));
    assert!(!session.set_gravity_field(1, 4, false));
    assert!(session.set_gravity_field(0, 4, false));
    assert!(session.set_gravity_field(0, 5, true));

    let target_two = session
        .field_effects
        .iter()
        .find(|effect| effect.target == TacticalUnitRef::Ship(2))
        .expect("target two should retain both field kinds");
    assert_eq!(target_two.source_ids(), (&[0][..], &[0][..]));
    assert_eq!(target_two.resource_id(), Some(3520));
}

// Source: FUN_005e45f0 / FUN_005e7540 subsystem condition indicators.
#[test]
fn subsystem_condition_uses_exact_five_source_bands() {
    let condition = TacticalSubsystemCondition {
        shields: 0,
        weapons: 24,
        tractor: 25,
        engines: 50,
        hyperdrive: 75,
    };
    assert_eq!(condition.resource_ids(), [1201, 1207, 1213, 1219, 1225]);
    assert_eq!(original_tactical_condition_band(1), 1);
    assert_eq!(original_tactical_condition_band(49), 2);
    assert_eq!(original_tactical_condition_band(74), 3);
    assert_eq!(original_tactical_condition_band(100), 4);
}

// Source: FUN_005b0e50 / FUN_005b3f10 subsystem condition from damage.
#[test]
fn subsystem_condition_follows_hull_hits_and_hyperdrive_capacity() {
    let condition = TacticalSubsystemCondition::from_source_state(
        980,
        1000,
        TacticalSubsystemCapacity {
            shields: true,
            weapons: true,
            tractor: true,
            engines: true,
            hyperdrive: 2,
        },
        TacticalSubsystemDamage {
            shields: 1,
            weapons: 2,
            tractor: 3,
            engines: 4,
            hyperdrive: 1,
        },
    );
    assert_eq!(condition.percentages(), [73, 48, 23, 0, 50]);
    assert_eq!(condition.resource_ids(), [1204, 1208, 1212, 1216, 1224]);

    let unavailable = TacticalSubsystemCondition::from_source_state(
        100,
        100,
        TacticalSubsystemCapacity {
            shields: false,
            weapons: false,
            tractor: false,
            engines: false,
            hyperdrive: 0,
        },
        TacticalSubsystemDamage::default(),
    );
    assert_eq!(unavailable.percentages(), [0; 5]);
}

// Source: FUN_005b1490 repair kind selection in hit order.
#[test]
fn subsystem_repair_selection_preserves_source_hit_order() {
    let damage = TacticalSubsystemDamage {
        shields: 1,
        weapons: 1,
        tractor: 2,
        engines: 2,
        hyperdrive: 1,
    };
    assert_eq!(damage.total_hits(), 7);
    let expected = [
        TacticalSubsystemKind::Engines,
        TacticalSubsystemKind::Engines,
        TacticalSubsystemKind::Shields,
        TacticalSubsystemKind::Hyperdrive,
        TacticalSubsystemKind::Tractor,
        TacticalSubsystemKind::Tractor,
        TacticalSubsystemKind::Weapons,
    ];
    for (index, kind) in expected.into_iter().enumerate() {
        assert_eq!(
            original_subsystem_repair_kind(damage, index as u16 + 1),
            Some(kind)
        );
    }
    assert_eq!(original_subsystem_repair_kind(damage, 0), None);
    assert_eq!(original_subsystem_repair_kind(damage, 8), None);
}

// Source: FUN_005b1490 repair chance; FUN_005b0330 repair interval.
#[test]
fn subsystem_repair_uses_inclusive_damage_control_chance() {
    let mut ship = test_ship(64, 0, true, true);
    ship.subsystem_capacity.engines = true;
    ship.subsystem_damage.engines = 2;
    ship.damage_control = 40;
    ship.refresh_subsystem_condition();
    assert_eq!(attempt_original_subsystem_repair(&mut ship, 0, 0, 1), None);
    assert_eq!(attempt_original_subsystem_repair(&mut ship, 0, 41, 1), None);
    let repair = attempt_original_subsystem_repair(&mut ship, 0, 40, 1)
        .expect("inclusive chance boundary should repair one engine hit");
    assert_eq!(repair.kind, TacticalSubsystemKind::Engines);
    assert_eq!((repair.hits_before, repair.hits_after), (2, 1));
    assert_eq!(ship.subsystem_damage.engines, 1);
    assert_eq!(ship.subsystem_condition.engines, 75);
    assert_eq!(ORIGINAL_SUBSYSTEM_REPAIR_INTERVAL_TURNS, 50);
}

// Source: FUN_005b0330 runs once a turn (0x005a9de0, slot +0x9c) and
// tries FUN_005b1490 every 50 000 ms, so at turn 50, step 200, for a
// ship with hits only.
#[test]
fn a_damaged_ship_tries_its_repair_at_the_fiftieth_turn() {
    let mut damaged = test_ship(64, 0, true, true);
    damaged.subsystem_capacity.engines = true;
    damaged.subsystem_damage.engines = 1;
    damaged.damage_control = 100;
    damaged.refresh_subsystem_condition();
    let mut enemy = test_ship(128, 0, false, true);
    for ship in [&mut damaged, &mut enemy] {
        ship.hull_current = 1_000_000;
        ship.hull_max = 1_000_000;
        ship.weapon_arcs = [TacticalWeaponArc::default(); 4];
    }
    let mut session = test_session(vec![damaged, enemy], Vec::new(), true);
    session.paused = false;
    for _ in 0..199 {
        session.step();
    }
    assert_eq!(session.ships[0].subsystem_damage.engines, 1);
    session.step();
    assert_eq!(session.combat_tick, 200);
    assert_eq!(session.subsystem_repairs.len(), 1);
    assert_eq!(session.subsystem_repairs[0].ship, 0);
    assert_eq!(session.ships[0].subsystem_damage.engines, 0);
}

// Source: FUN_005b05c0 sublight conversion; FUN_005ad750 engine mode.
#[test]
fn engine_condition_uses_damage_mode_and_active_tractor_power_not_hull() {
    let mut target = test_ship(64, 0, true, true);
    target.hull_current = 10;
    target.hull_max = 100;
    target.subsystem_capacity.engines = true;
    target.sublight_engine_power = 100.0;
    target.maneuver_state_value = Some(4.0);
    target.subsystem_damage.engines = 1;

    let mut source = test_ship(128, 0, false, true);
    source.hull_current = 50;
    source.hull_max = 100;
    source.subsystem_capacity.tractor = true;
    source.tractor_beam_power = 40.0;
    source.subsystem_damage.tractor = 1;

    let mut session = test_session(vec![target, source], Vec::new(), true);
    assert!(session.set_tractor_field(1, 0, true));
    let held = session.subsystem_mobility(0).unwrap();
    assert_eq!(held.base_engine_power, 100.0);
    assert_eq!(held.engine_mode_bonus, 5.0);
    assert_eq!(held.active_tractor_power, 10.0);
    assert_eq!(held.effective_engine_power, 68.75);
    assert_eq!(held.engine_percent, 68);
    assert_eq!(session.ships[0].subsystem_condition.engines, 68);

    assert!(session.set_tractor_field(1, 0, false));
    let released = session.subsystem_mobility(0).unwrap();
    assert_eq!(released.active_tractor_power, 0.0);
    assert_eq!(released.effective_engine_power, 78.75);
    assert_eq!(released.engine_percent, 78);
    assert_eq!(session.ships[0].subsystem_condition.engines, 78);
}

// Source: FUN_005ad750 / FUN_005afb70 engine mode bonus clamp.
#[test]
fn maneuver_state_produces_the_source_clamped_one_through_nine_bonus() {
    assert_eq!(original_engine_mode_bonus(None), 1.0);
    assert_eq!(original_engine_mode_bonus(Some(0.0)), 9.0);
    assert_eq!(original_engine_mode_bonus(Some(4.0)), 5.0);
    assert_eq!(original_engine_mode_bonus(Some(8.0)), 1.0);
    assert_eq!(original_engine_mode_bonus(Some(9.0)), 1.0);
    assert_eq!(original_engine_mode_bonus(Some(-1.0)), 9.0);
}

// Source: FUN_005cd460 turn; FUN_005b2f30 movement and collision.
#[test]
fn movement_uses_effective_power_alignment_and_millisecond_integration() {
    let mut alliance = test_ship(64, 0, true, true);
    alliance.subsystem_capacity.engines = true;
    alliance.source_position.z = -56.0;
    alliance.sublight_engine_power = 100.0;
    alliance.maneuver_state_value = Some(4.0);
    alliance.subsystem_damage.engines = 1;

    let mut empire = test_ship(128, 0, false, true);
    empire.subsystem_capacity.engines = true;
    empire.source_position.z = 56.0;
    empire.sublight_engine_power = 100.0;
    empire.maneuver_state_value = Some(4.0);
    empire.subsystem_damage.engines = 1;

    alliance.source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
    alliance.source_waypoint = Some(TacticalWorldPosition {
        x: 0.0,
        y: 0.0,
        z: 56.0,
    });
    empire.source_forward = TacticalWorldVector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    empire.source_desired_forward = empire.source_forward;
    empire.source_waypoint = Some(TacticalWorldPosition {
        x: 0.0,
        y: 0.0,
        z: -56.0,
    });
    let close = |left: f32, right: f32| assert!((left - right).abs() < 1e-3, "{left} {right}");

    // The anchor moves 0.95 x 78.75 x 0.25 (0x005eb1a0, DAT_0066d84c);
    // within one frame's travel and facing its heading, the member
    // snaps onto it (FUN_005b2f30, DAT_0066c508).
    let mut session = test_session(vec![alliance, empire], Vec::new(), true);
    session.advance_original_tactical_movement(250.0);
    close(session.ships[0].source_position.z, -56.0 + 18.703_125);
    close(session.ships[1].source_position.z, 56.0 - 18.703_125);
    assert_eq!(session.ships[0].source_velocity, TacticalWorldVector::ZERO);

    // Farther behind, it moves along its level forward at the force's
    // speed times its alignment.
    session.ships[0].source_position.z = -80.0;
    session.advance_original_tactical_movement(250.0);
    assert_eq!(session.ships[0].source_velocity.z, 78.75);
    assert_eq!(session.ships[0].source_position.z, -60.3125);

    // Facing away from its last heading it does not move on the level,
    // but climbs toward its place at 0.4 of the speed.
    session.ships[0].source_desired_forward = TacticalWorldVector {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };
    session.ships[0].source_position.y = -20.0;
    session.advance_original_tactical_movement(250.0);
    assert_eq!(session.ships[0].source_velocity, TacticalWorldVector::ZERO);
    assert_eq!(session.ships[0].source_position.z, -60.3125);
    close(session.ships[0].source_position.y, -20.0 + 7.875);
}

// Source: FUN_005b2e60 collision volume overlap test.
#[test]
fn collision_predicate_preserves_strict_source_boundaries() {
    let envelope = OriginalTacticalCollisionEnvelope {
        vertical_radius: 1.0,
        planar_diameter: 2.0,
    };
    let mut first = test_ship(64, 0, true, true);
    first.source_collision_envelope = Some(envelope);
    let mut second = test_ship(128, 0, false, true);
    second.source_collision_envelope = Some(envelope);

    second.source_position.z = 1.999;
    assert!(original_tactical_objects_overlap(&first, &second));
    second.source_position.z = 2.0;
    assert!(!original_tactical_objects_overlap(&first, &second));
    second.source_position.z = 0.0;
    second.source_position.y = 1.999;
    assert!(original_tactical_objects_overlap(&first, &second));
    second.source_position.y = 2.0;
    assert!(!original_tactical_objects_overlap(&first, &second));
}

// Source: FUN_005b2f30 / FUN_005b2e60 collision position restore.
#[test]
fn movement_restores_previous_position_when_mesh_envelopes_overlap() {
    let envelope = OriginalTacticalCollisionEnvelope {
        vertical_radius: 1.0,
        planar_diameter: 2.0,
    };
    let mut moving = test_ship(64, 0, true, true);
    moving.subsystem_capacity.engines = true;
    moving.sublight_engine_power = 4.0;
    moving.source_collision_envelope = Some(envelope);
    moving.source_position.z = 0.0;

    let mut fixed = test_ship(128, 0, false, true);
    fixed.source_collision_envelope = Some(envelope);
    fixed.source_position.z = 2.0;

    moving.source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
    moving.source_waypoint = Some(TacticalWorldPosition {
        x: 0.0,
        y: 0.0,
        z: 10.0,
    });

    // The member's snap to its place at z 1.1875 (0.95 x 5 x 0.25)
    // would overlap the fixed ship, so it stays.
    let mut session = test_session(vec![moving.clone(), fixed], Vec::new(), true);
    session.advance_original_tactical_movement(250.0);
    assert_eq!(session.ships[0].source_position.z, 0.0);
    assert!((session.force_movers[0].anchor.z - 1.1875).abs() < 1e-4);

    let mut open = test_session(vec![moving], Vec::new(), true);
    open.advance_original_tactical_movement(250.0);
    assert!((open.ships[0].source_position.z - 1.1875).abs() < 1e-4);
}

// Source: FUN_005ba270 maneuverability; FUN_005cd460 turn rate.
#[test]
fn current_to_desired_turn_uses_source_rates_and_normalization() {
    let close = |left: f32, right: f32| assert!((left - right).abs() < 0.000_01);
    close(original_capital_turn_rate(0), std::f32::consts::PI / 12.0);
    close(
        original_capital_turn_rate(4),
        5.0 * std::f32::consts::PI / 18.0,
    );
    close(original_capital_turn_rate(20), std::f32::consts::PI / 2.0);

    let mut ship = test_ship(64, 0, true, true);
    ship.maneuverability = 4;
    ship.source_desired_forward = TacticalWorldVector {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    turn_original_tactical_ship(&mut ship, 250.0);
    let turn = 5.0 * std::f32::consts::PI / 72.0;
    close(ship.source_forward.x, turn.sin());
    close(ship.source_forward.y, 0.0);
    close(ship.source_forward.z, turn.cos());
    close(
        ship.source_forward
            .x
            .hypot(ship.source_forward.y)
            .hypot(ship.source_forward.z),
        1.0,
    );

    turn_original_tactical_ship(&mut ship, 10_000.0);
    assert_eq!(ship.source_forward, ship.source_desired_forward);
}

// Source: FUN_005a8f70 waypoint arrival; FUN_005cf980 recovery states.
#[test]
fn waypoint_arrival_and_fighter_docking_complete_source_executors() {
    let mut carrier = test_ship(64, 0, true, true);
    carrier.subsystem_capacity.engines = true;
    carrier.source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
    carrier.source_waypoint = Some(TacticalWorldPosition {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    carrier.order = TacticalOrder::Anvil;
    // A maneuver is run against a target, which the arrival's line asks
    // for (FUN_005cc300: +0x98 not empty).
    carrier.attack_target = Some(TacticalAttackTarget::CapitalShip(0));

    let mut fighter = test_fighter(1, true);
    // 2.5 behind the carrier, which arrives where it stands, facing it
    // (+z).
    fighter.source_position.z = -2.5;
    fighter.sublight_engine_power = 4.0;
    fighter.order = TacticalOrder::Recover;
    fighter.recovery_state = TacticalFighterRecoveryState::Returning;
    fighter.recovery_target = Some(0);

    let mut session = test_session(vec![carrier], vec![fighter], true);
    // The recovery executor's install (FUN_005ca6d0 case 2), then the
    // squadron on its way (state 2).
    session.begin_original_fighter_recovery(&[0]);
    session.fighters[0].recovery_state = TacticalFighterRecoveryState::Returning;
    session.fighters[0].recovery_target = Some(0);
    session.advance_original_tactical_movement(250.0);
    // The anchor is already within the route's 1.0 (0x005eb1a0), so it
    // arrives where it stands.
    assert_eq!(
        session.ships[0].source_position,
        TacticalWorldPosition::ORIGIN
    );
    assert_eq!(session.ships[0].source_waypoint, None);
    assert_eq!(session.ships[0].source_velocity, TacticalWorldVector::ZERO);
    assert_eq!(session.ships[0].order, TacticalOrder::None);

    // 0x5b8f70: it flies its forward at its speed (commander term 1 plus
    // 4) for 250 ms, then FUN_005cf980's turn docks it within 2.0
    // (state 3) and posts event 0x1a.
    session.advance_recovering_fighters(250.0);
    session.advance_recovery_executors();
    assert_eq!(session.fighters[0].source_position.z, -1.25);
    assert_eq!(
        session.fighters[0].recovery_state,
        TacticalFighterRecoveryState::Docking
    );
    assert!(session.fighters[0].alive);

    // Its handler (FUN_005b8bd0, FUN_005b8630) takes it in.
    let docked = session
        .turn_events
        .iter()
        .find(|queued| matches!(queued.event, TacticalTurnEvent::FighterDocked { .. }))
        .expect("the docking event")
        .event
        .clone();
    session.run_turn_event(docked);
    assert_eq!(
        session.fighters[0].recovery_state,
        TacticalFighterRecoveryState::Recovered
    );
    assert!(!session.fighters[0].alive);
    assert_eq!(session.fighters[0].squad_count, 12);
    assert_eq!(session.fighters[0].order, TacticalOrder::None);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![
            TacticalVoiceCue::capital_maneuver_complete(TacticalVoiceFaction::Alliance, 0,),
            TacticalVoiceCue::fighter_recovery_complete(TacticalVoiceFaction::Alliance, 0,),
        ]
    );
}

// Source: FUN_005cf980 / FUN_005cf940 fighter recovery state: the
// executor's turn finds no room and plays the group's voice once
// (+0x10); the enemy group's plays nothing for the player.
#[test]
fn fighter_recovery_warns_each_player_group_once_when_carriers_are_full() {
    let mut carrier = test_ship(64, 0, true, true);
    carrier.fighter_capacity = 1;
    let mut reserved = test_fighter(1, true);
    reserved.recovery_state = TacticalFighterRecoveryState::Returning;
    reserved.recovery_target = Some(0);
    let mut first = test_fighter(1, true);
    first.fleet_squadron_index = 1;
    first.selected = true;
    let mut second = test_fighter(1, true);
    second.fleet_squadron_index = 2;
    second.selected = true;
    let mut enemy = test_fighter(5, false);
    enemy.selected = true;
    let mut session = test_session(vec![carrier], vec![reserved, first, second, enemy], true);
    for index in 1..4 {
        session.fighters[index].order = TacticalOrder::Recover;
    }

    session.begin_original_fighter_recovery(&[1, 2, 3]);
    assert!(session.take_pending_voice_cues().is_empty());
    session.advance_recovery_executors();

    assert!(session.fighters[1..].iter().all(|fighter| {
        fighter.recovery_state == TacticalFighterRecoveryState::AwaitingCarrier
    }));
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::fighter_recovery_capacity(
            TacticalVoiceFaction::Alliance,
            0,
        )]
    );
}

// FUN_005ad530: the player's fighter groups without a hyperdrive drop
// their attack and take order 2, the recovery to a carrier
// (FUN_005ca6d0, FUN_005cf980). When the side's time ends, event 0x2f
// (FUN_005b2720) destroys every group of the side still in the fight
// without a hyperdrive, one still on its way to an escaped carrier
// included; a group that docked first keeps its craft for the commit
// (FUN_0040c780, ghidra/notes/planet-squadrons.md).
#[test]
fn a_withdrawal_sends_the_fighters_to_their_carriers_and_away() {
    let mut carrier = test_ship(64, 0, true, true);
    carrier.fighter_capacity = 1;
    carrier.subsystem_capacity.hyperdrive = 1;
    carrier.subsystem_condition.hyperdrive = 100;
    // FUN_005b1b70 wants engine power to jump.
    carrier.subsystem_capacity.engines = true;
    carrier.subsystem_condition.engines = 100;
    let mut wing = test_fighter(1, true);
    wing.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
    // The enemy outlasts the jump, so the battle goes on.
    let mut enemy_ship = test_ship(64, 0, false, true);
    enemy_ship.hull_current = 1_000_000;
    enemy_ship.hull_max = 1_000_000;
    let mut enemy_wing = test_fighter(5, false);
    enemy_wing.hull_current = 1_000_000.0;
    enemy_wing.hull_max = 1_000_000.0;
    enemy_wing.weapon_arc = TacticalWeaponArc::default();
    enemy_wing.torpedo_strength = 0;
    // A carrier with no hyperdrive stays.
    let mut grounded = test_ship(64, 1, true, true);
    grounded.fighter_capacity = 1;
    let mut empty = test_fighter(1, true);
    empty.squad_count = 0;
    let mut session = test_session(
        vec![carrier, enemy_ship, grounded],
        vec![wing, enemy_wing, empty],
        true,
    );

    session.begin_player_withdrawal();
    withdraw_player_fighters(&mut session);

    let wing = &session.fighters[0];
    assert_eq!(wing.order, TacticalOrder::Recover);
    assert_eq!(wing.attack_target, None);
    // SetOrder(2) (0x005ad604) posts the recovery's executor (event 0xa)
    // with the command stamp 3; FUN_005cf980 reserves the carrier at
    // turn 4, step 16.
    assert_eq!(wing.recovery_target, None);
    assert_eq!(session.fighters[1].order, TacticalOrder::None);
    // An empty group takes no order.
    assert_eq!(session.fighters[2].order, TacticalOrder::None);
    // A group bound for the carrier that stays, and one only awaiting
    // the escaping carrier, do not leave.
    // Both are members of the group's recovery (order 2).
    let mut bound = test_fighter(1, true);
    bound.order = TacticalOrder::Recover;
    // Docking on it already (state 3), its event 0x1a queued
    // (FUN_005cf980, FUN_005b8bd0).
    bound.recovery_state = TacticalFighterRecoveryState::Docking;
    bound.recovery_target = Some(2);
    let mut waiting = test_fighter(1, true);
    waiting.order = TacticalOrder::Recover;
    waiting.recovery_state = TacticalFighterRecoveryState::AwaitingCarrier;
    waiting.recovery_target = Some(0);
    session.fighters.push(bound);
    session.fighters.push(waiting);
    session.post_turn_event(
        3,
        TacticalTurnEvent::FighterDocked {
            fighter: 3,
            carrier: 2,
        },
    );

    // Without engine power the first group, away from the carrier, is
    // still on its way to it when it leaves. (The carrier turns at
    // pi / 12 a second before it moves away: FUN_005b2f30.)
    session.paused = false;
    session.fighters[0].sublight_engine_power = 0.0;
    session.fighters[0].source_position.z = 30.0;
    for _ in 0..16 {
        session.step();
    }
    // Reserved on turn 4's update (state 1); its 0x2c lands at turn 7.
    let wing = &session.fighters[0];
    assert_eq!(wing.recovery_target, Some(0));
    assert_eq!(wing.recovery_state, TacticalFighterRecoveryState::Reserved);

    // The side's time ends on the next turn; the carrier, at its exit
    // line, then jumps for 4000 ms (FUN_005cc300, FUN_005a6b40).
    session.ships[0].source_position.z = session.withdrawal_exit_z(true);
    let key = session.force_key(0);
    let at_exit = session.ships[0].source_position;
    for mover in &mut session.force_movers {
        if mover.force == key {
            mover.anchor = at_exit;
        }
    }
    session.withdrawal_ms = Some(ORIGINAL_TACTICAL_STEP_MILLISECONDS);
    // The time ends at turn 5, step 20, where the frame's order step
    // decides the jump, stamped 7 (0x005ad4a0); its event 0x2d runs at
    // turn 8 (step 32), and its 0x2e, stamped 14 on step 47, at turn
    // 15, step 60 (FUN_005a96b0).
    for _ in 16..59 {
        session.step();
    }
    assert!(!session.ships[0].retreated);
    // In its jump the carrier faces away from the enemy, along its
    // side's vector reversed (FUN_005a6b40 writes +0x10 each step).
    assert_eq!(session.ships[0].source_forward.z, -1.0);
    session.step();
    assert_eq!(session.combat_tick, 60);
    assert!(session.ships[0].retreated);
    // At the side's end the groups left in the fight without a
    // hyperdrive are destroyed (event 0x2f, FUN_005b2720): the one still
    // on its way to the escaped carrier and the one only awaiting it.
    assert_eq!(session.fighters[0].squad_count, 0);
    assert_eq!(session.fighters[4].squad_count, 0);
    // The group bound for the carrier that stays docked on it first
    // (FUN_005cf980) and keeps its craft; that carrier, without a
    // hyperdrive, is destroyed.
    let bound = &session.fighters[3];
    assert!(bound.recovery_state == TacticalFighterRecoveryState::Recovered);
    assert!(bound.squad_count > 0);
    assert!(!session.ships[2].alive && !session.ships[2].retreated);
    assert!(session.fighters[1].alive);
}

// FUN_005ad530: a fighter group with a hyperdrive (FUN_005b1bc0 above 0;
// every Alliance FIGHTSD class, of the Empire's only record 8) takes
// order 3, heads for its edge and jumps there when the side's time ends
// (FUN_005c6610, FUN_005b1b70) with its craft.
#[test]
fn a_withdrawal_holds_fighters_with_a_hyperdrive_then_they_jump() {
    let mut carrier = test_ship(64, 0, true, true);
    carrier.fighter_capacity = 2;
    let mut wing = test_fighter(1, true);
    wing.has_hyperdrive = true;
    wing.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
    let mut stalled = test_fighter(1, true);
    stalled.has_hyperdrive = true;
    stalled.sublight_engine_power = 0.0;
    let mut enemy_wing = test_fighter(8, false);
    enemy_wing.has_hyperdrive = true;
    enemy_wing.hull_current = 1_000_000.0;
    enemy_wing.hull_max = 1_000_000.0;
    enemy_wing.weapon_arc = TacticalWeaponArc::default();
    enemy_wing.torpedo_strength = 0;
    let mut enemy_ship = test_ship(64, 0, false, true);
    enemy_ship.hull_current = 1_000_000;
    enemy_ship.hull_max = 1_000_000;
    let mut session = test_session(
        vec![carrier, enemy_ship],
        vec![wing, stalled, enemy_wing],
        true,
    );

    session.begin_player_withdrawal();
    withdraw_player_fighters(&mut session);

    let wing = &session.fighters[0];
    assert_eq!(wing.order, TacticalOrder::None);
    assert_eq!(wing.attack_target, None);
    assert_eq!(wing.recovery_target, None);
    assert_eq!(
        wing.recovery_state,
        TacticalFighterRecoveryState::AwaitingCarrier
    );

    assert!(wing.retreating);

    // At its edge when the time ends, it jumps for 4000 ms. Its
    // executor (event 0xa, stamped 3) comes at turn 4, step 16
    // (FUN_005ca6d0); until then it keeps its course.
    session.paused = false;
    for _ in 0..16 {
        session.step();
    }
    session.fighters[0].source_position.z = session.withdrawal_exit_z(true);
    // Its group's anchor goes with it (FUN_005c72b0's +0x3c).
    let exit_z = session.withdrawal_exit_z(true);
    for mover in &mut session.fighter_movers {
        if mover.slots.iter().any(|(member, _)| *member == 0) {
            mover.anchor.z = exit_z;
        }
    }
    session.withdrawal_ms = Some(ORIGINAL_TACTICAL_STEP_MILLISECONDS);
    // The time ends at turn 5, step 20; the jump, stamped 7, runs at
    // turn 8 and it leaves at turn 15, step 60 (events 0x2d and 0x2e,
    // FUN_005a96b0).
    for _ in 16..59 {
        session.step();
    }
    assert!(session.fighters[0].alive);
    session.step();
    assert_eq!(session.combat_tick, 60);
    let wing = &session.fighters[0];
    assert!(!wing.alive && wing.squad_count > 0);
    // No engine power, no jump: destroyed at the side's end
    // (FUN_005b2720); the other side's group stays.
    assert!(!session.fighters[1].alive && session.fighters[1].squad_count == 0);
    assert!(session.fighters[2].alive);
}

// FUN_005a8a90 -> FUN_005c7e80 asks FUN_005b1b70: Withdraw needs a ship
// with a hyperdrive and engine power.
#[test]
fn a_withdrawal_takes_get_immediate_result_and_observe_away() {
    // FUN_005dab90 hides Get Immediate Result and Observe / Take Command
    // once a side is withdrawing (FUN_005adde0); A0 TAC-06-C009 shows
    // them blank after a confirmed withdrawal, Game Options and Close
    // still there.
    let mut ship = test_ship(64, 0, true, true);
    ship.subsystem_capacity.hyperdrive = 1;
    ship.subsystem_condition.hyperdrive = 100;
    ship.subsystem_capacity.engines = true;
    ship.subsystem_condition.engines = 100;
    let mut session = test_session(vec![ship], Vec::new(), true);
    let gates = BattleOptionsGates::of(Some(&session));
    assert!(gates.withdraw && gates.simulate && gates.observe);
    // FUN_005ad530: the side's time starts and its ships head out.
    session.withdrawal_ms = Some(30_000.0);
    session.ships[0].retreating = true;
    let gates = BattleOptionsGates::of(Some(&session));
    assert!(!gates.simulate && !gates.observe);
    assert!(!gates.enabled(TacticalBattleOptionsControl::SimulateRemainder));
    assert!(!gates.enabled(TacticalBattleOptionsControl::ObserveToggle));
    assert!(!gates.enabled(TacticalBattleOptionsControl::Withdraw));
    assert!(gates.enabled(TacticalBattleOptionsControl::GameOptions));
    assert!(gates.enabled(TacticalBattleOptionsControl::Close));
    session.withdrawal_ms = None;
    session.ai_withdrawal_ms = Some(30_000.0);
    assert!(!BattleOptionsGates::of(Some(&session)).observe);
    let none = BattleOptionsGates::of(None);
    assert!(!none.withdraw && none.simulate && none.observe);
}

#[test]
fn withdraw_needs_a_ship_that_can_jump() {
    let mut ship = test_ship(64, 0, true, true);
    ship.subsystem_capacity.hyperdrive = 1;
    ship.subsystem_condition.hyperdrive = 100;
    let mut session = test_session(vec![ship], Vec::new(), true);
    assert!(!tactical_withdraw_enabled(&session));
    session.ships[0].subsystem_capacity.engines = true;
    session.ships[0].subsystem_condition.engines = 100;
    assert!(tactical_withdraw_enabled(&session));
    session.ships[0].subsystem_condition.hyperdrive = 0;
    assert!(!tactical_withdraw_enabled(&session));
}

// FUN_005ad530 takes the withdrawing player's task forces off the
// command panel (FUN_005a0320, FUN_005a0410; FUN_0059f5f0 gates the
// group commands): no fighter group or capital task force can be
// selected while the withdrawal runs.
#[test]
fn a_withdrawing_side_cannot_select_its_groups() {
    let carrier = test_ship(64, 0, true, true);
    let mut wing = test_fighter(1, true);
    wing.fighter_group = 0;
    let mut session = test_session(vec![carrier], vec![wing], true);
    assert!(select_fighter_group(&mut session, 0));
    session.begin_player_withdrawal();
    withdraw_player_fighters(&mut session);
    assert!(!select_fighter_group(&mut session, 0));
    assert_eq!(select_task_force(&mut session, 0), 0);
}

// Source: FUN_005cf980 fighter recovery completion.
#[test]
fn fighter_recovery_completion_waits_for_the_last_live_group_member() {
    let carrier = test_ship(64, 0, true, true);
    let mut docking = test_fighter(1, true);
    docking.recovery_state = TacticalFighterRecoveryState::Docking;
    docking.recovery_target = Some(0);
    let mut still_returning = test_fighter(1, true);
    still_returning.fleet_squadron_index = 1;
    still_returning.recovery_state = TacticalFighterRecoveryState::Returning;
    still_returning.recovery_target = Some(0);
    still_returning.source_position.z = 8.0;
    let mut session = test_session(vec![carrier], vec![docking, still_returning], true);

    // Event 0x1a for the docking one (FUN_005b8bd0).
    session.run_turn_event(TacticalTurnEvent::FighterDocked {
        fighter: 0,
        carrier: 0,
    });

    assert!(!session.fighters[0].alive);
    assert!(session.fighters[1].alive);
    assert!(session.take_pending_voice_cues().is_empty());

    session.fighters[1].recovery_state = TacticalFighterRecoveryState::Docking;
    session.run_turn_event(TacticalTurnEvent::FighterDocked {
        fighter: 1,
        carrier: 0,
    });

    assert!(!session.fighters[1].alive);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::fighter_recovery_complete(
            TacticalVoiceFaction::Alliance,
            0,
        )]
    );
}

// Source: _DAT_0066d07c / _DAT_0066d094 / _DAT_0066d098 maneuver params.
#[test]
fn maneuver_waypoints_preserve_source_rotations_and_scale() {
    let current = TacticalWorldPosition {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    let target = TacticalWorldPosition {
        x: 9.0,
        y: 6.0,
        z: 15.0,
    };
    let (sin, cos) = ORIGINAL_MANEUVER_ANGLE.sin_cos();
    let close = |left: f32, right: f32| assert!((left - right).abs() < 0.000_01);

    let left = original_maneuver_waypoint(TacticalOrder::LeftHook, current, target).unwrap();
    close(left.x, 1.0 + (cos * 8.0 - sin * 12.0) * 0.75);
    close(left.y, 5.0);
    close(left.z, 3.0 + (cos * 12.0 + sin * 8.0) * 0.75);

    let right = original_maneuver_waypoint(TacticalOrder::RightHook, current, target).unwrap();
    close(right.x, 1.0 + (cos * 8.0 + sin * 12.0) * 0.75);
    close(right.y, 5.0);
    close(right.z, 3.0 + (cos * 12.0 - sin * 8.0) * 0.75);

    let hammer = original_maneuver_waypoint(TacticalOrder::Hammer, current, target).unwrap();
    close(hammer.x, 7.0);
    close(hammer.y, 2.0 + (cos * 4.0 - sin * 12.0) * 0.75);
    close(hammer.z, 3.0 + (cos * 12.0 + sin * 4.0) * 0.75);

    let anvil = original_maneuver_waypoint(TacticalOrder::Anvil, current, target).unwrap();
    close(anvil.x, 7.0);
    close(anvil.y, 2.0 + (cos * 4.0 + sin * 12.0) * 0.75);
    close(anvil.z, 3.0 + (cos * 12.0 - sin * 4.0) * 0.75);
}

// Source: FUN_005a8f70 waypoint; FUN_005cf980 recovery executor.
#[test]
fn committed_maneuver_hold_and_recover_enter_original_executors() {
    let mut player = test_ship(64, 0, true, true);
    player.selected = true;
    player.source_position.z = -50.0;
    player.fighter_capacity = 2;
    let mut enemy = test_ship(128, 0, false, true);
    enemy.source_position = TacticalWorldPosition {
        x: 10.0,
        y: 4.0,
        z: 50.0,
    };
    let mut fighter = test_fighter(1, true);
    fighter.selected = true;
    let mut session = test_session(vec![player, enemy], vec![fighter], true);
    session.selected_ship = Some(0);

    assert_eq!(
        assign_selected_command(
            &mut session,
            TacticalOrder::LeftHook,
            Some(TacticalTactic::Surround),
        ),
        (1, 0)
    );
    assert_eq!(session.ships[0].order, TacticalOrder::LeftHook);
    assert_eq!(session.ships[0].tactic, TacticalTactic::Surround);
    assert_eq!(session.ships[0].source_waypoint, None);
    run_posted_orders(&mut session);
    assert!(session.ships[0].source_waypoint.is_some());
    assert_ne!(
        session.ships[0].source_desired_forward,
        TacticalWorldVector::ZERO
    );

    assert_eq!(
        assign_selected_command(&mut session, TacticalOrder::HoldPosition, None),
        (1, 0)
    );
    run_posted_orders(&mut session);
    assert_eq!(session.ships[0].source_waypoint, None);
    assert_eq!(
        session.ships[0].source_desired_forward,
        TacticalWorldVector::ZERO
    );
    assert_eq!(session.ships[0].source_velocity, TacticalWorldVector::ZERO);

    session.ships[0].selected = false;
    session.selected_ship = None;
    session.selected_fighter_group = Some(0);
    assert_eq!(
        assign_selected_command(&mut session, TacticalOrder::Recover, None),
        (0, 1)
    );
    assert_eq!(session.fighters[0].order, TacticalOrder::Recover);
    run_posted_orders(&mut session);
    // The executor's next turn reserves the carrier (state 1); its event
    // 0x2c gives state 2 three turns later.
    session.advance_recovery_executors();
    assert_eq!(
        session.fighters[0].recovery_state,
        TacticalFighterRecoveryState::Reserved
    );
    assert_eq!(session.fighters[0].recovery_target, Some(0));
}

// Source: FUN_005d0bb0 / FUN_005a8fc0 attack target acquisition.
#[test]
fn attack_orders_acquire_first_eligible_typed_target_for_both_unit_classes() {
    let mut player_ship = test_ship(64, 0, true, true);
    player_ship.selected = true;
    let dead_enemy_ship = test_ship(128, 0, false, false);
    let live_enemy_ship = test_ship(129, 1, false, true);

    let mut player_fighter = test_fighter(1, true);
    player_fighter.fighter_group = 0;
    let mut recovered_enemy_fighter = test_fighter(5, false);
    recovered_enemy_fighter.recovery_state = TacticalFighterRecoveryState::Recovered;
    let live_enemy_fighter = test_fighter(6, false);
    let mut session = test_session(
        vec![player_ship, dead_enemy_ship, live_enemy_ship],
        vec![player_fighter, recovered_enemy_fighter, live_enemy_fighter],
        true,
    );
    session.selected_ship = Some(0);

    assert_eq!(
        assign_selected_command(&mut session, TacticalOrder::AttackCapitalShips, None,),
        (1, 0)
    );
    run_posted_orders(&mut session);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(2))
    );

    assert_eq!(
        assign_selected_command(&mut session, TacticalOrder::AttackFighters, None),
        (1, 0)
    );
    run_posted_orders(&mut session);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::FighterGroup(2))
    );

    session.ships[0].selected = false;
    session.selected_ship = None;
    session.fighters[0].selected = true;
    session.selected_fighter_group = Some(0);
    assert_eq!(
        assign_selected_command(&mut session, TacticalOrder::AttackFighters, None),
        (0, 1)
    );
    run_posted_orders(&mut session);
    assert_eq!(
        session.fighters[0].attack_target,
        Some(TacticalAttackTarget::FighterGroup(2))
    );
}

// Source: FUN_005d0bb0 attack target replacement.
#[test]
fn attack_executor_preserves_live_engagement_and_replaces_invalid_target() {
    let mut player = test_ship(64, 0, true, true);
    player.selected = true;
    player.attack_target = Some(TacticalAttackTarget::CapitalShip(2));
    let first_enemy = test_ship(128, 0, false, true);
    let second_enemy = test_ship(129, 1, false, true);
    let mut session = test_session(vec![player, first_enemy, second_enemy], Vec::new(), true);
    session.selected_ship = Some(0);

    assign_selected_command(&mut session, TacticalOrder::AttackCapitalShips, None);
    run_posted_orders(&mut session);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(2))
    );

    session.ships[2].alive = false;
    assign_selected_command(&mut session, TacticalOrder::AttackCapitalShips, None);
    run_posted_orders(&mut session);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(1))
    );
}

// Source: FUN_005d0bb0 / FUN_005a8fc0 target lifecycle.
#[test]
fn attack_target_lifecycle_reacquires_same_class_and_clears_exhausted_lists() {
    let mut player_ship = test_ship(64, 0, true, true);
    player_ship.order = TacticalOrder::AttackCapitalShips;
    player_ship.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
    // FUN_005cdf50 skips a ship whose jump has started (state 6); a
    // withdrawing ship before its jump stays a target.
    let mut jumping_enemy = test_ship(128, 0, false, true);
    jumping_enemy.retreating = true;
    jumping_enemy.retreat_progress = ORIGINAL_TACTICAL_STEP_MILLISECONDS;
    let live_enemy = test_ship(129, 1, false, true);

    let mut player_fighter = test_fighter(1, true);
    player_fighter.order = TacticalOrder::AttackFighters;
    player_fighter.attack_target = Some(TacticalAttackTarget::FighterGroup(1));
    let mut recovered_enemy_fighter = test_fighter(5, false);
    recovered_enemy_fighter.recovery_state = TacticalFighterRecoveryState::Recovered;
    let live_enemy_fighter = test_fighter(6, false);
    let mut session = test_session(
        vec![player_ship, jumping_enemy, live_enemy],
        vec![player_fighter, recovered_enemy_fighter, live_enemy_fighter],
        true,
    );

    refresh_original_attack_targets(&mut session);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(2))
    );
    assert_eq!(
        session.fighters[0].attack_target,
        Some(TacticalAttackTarget::FighterGroup(2))
    );

    session.ships[2].alive = false;
    session.fighters[2].squad_count = 0;
    refresh_original_attack_targets(&mut session);
    assert_eq!(session.ships[0].attack_target, None);
    assert_eq!(session.fighters[0].attack_target, None);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![
            TacticalVoiceCue::capital_orders_complete(TacticalVoiceFaction::Alliance, 0,),
            TacticalVoiceCue::fighter_orders_complete(TacticalVoiceFaction::Alliance, 0,),
        ]
    );

    session.ships[0].order = TacticalOrder::AttackFighters;
    refresh_original_attack_targets(&mut session);
    assert_eq!(
        session.ships[0].attack_target, None,
        "an exhausted fighter list must not fall through to a capital target"
    );
    assert!(session.take_pending_voice_cues().is_empty());
}

// Source: FUN_005cf980 recovery state reset.
#[test]
fn recovered_fighter_clears_attack_fighters_without_capital_fire_or_audio() {
    fn session_with_source_order(order: TacticalOrder) -> BattleSession {
        let mut source = test_ship(64, 0, true, true);
        source.order = order;
        source.attack_target = match order {
            TacticalOrder::AttackFighters => Some(TacticalAttackTarget::FighterGroup(0)),
            TacticalOrder::AttackCapitalShips => Some(TacticalAttackTarget::CapitalShip(1)),
            _ => None,
        };
        source.weapon_arcs[0] = TacticalWeaponArc::new(300, 0, 0);
        source.weapon_ranges.laser_cannon = 100.0;

        let mut capital = test_ship(128, 0, false, true);
        capital.hull_current = 1_000;
        capital.hull_max = 1_000;
        capital.shield = 1_000;
        capital.shield_max = 1_000;

        // FUN_005cf980 advances Docking (state 3) to Recovered (state 4)
        // without removing the strategic squadron count. FUN_005d0bb0
        // and FUN_005a8fc0 keep Attack Fighters bound to class 1, so the
        // resulting empty typed list must not fall through to a capital.
        let mut exhausted_fighter = test_fighter(5, false);
        exhausted_fighter.recovery_state = TacticalFighterRecoveryState::Docking;
        exhausted_fighter.recovery_target = Some(1);
        exhausted_fighter.squad_count = 12;

        let mut session = test_session(vec![source, capital], vec![exhausted_fighter], true);
        session.paused = false;
        // Its docking event (0x1a, posted with state 3 by FUN_005cf980)
        // runs at turn 4.
        session.post_turn_event(
            3,
            TacticalTurnEvent::FighterDocked {
                fighter: 0,
                carrier: 1,
            },
        );
        // The source's draw opens its turn 1 (FUN_005b3a40's 0 to 2).
        session.tactical_rng = (0..)
            .map(OriginalTacticalRng::new)
            .find(|rng| {
                let mut rng = *rng;
                rng.inclusive(0, rebellion_core::tactical::ORIGINAL_WEAPON_TURN_DRAW_MAX) == 0
            })
            .expect("a stream opens the turn");
        session
    }

    let mut ordered = session_with_source_order(TacticalOrder::AttackFighters);
    let ordered_capital_before = (ordered.ships[1].hull_current, ordered.ships[1].shield);

    // Through the turn that would fire at step 4 (0x005a9de0,
    // FUN_005b3a40) and its hit at step 24.
    for _ in 0..24 {
        ordered.step();
    }

    assert_eq!(
        ordered.fighters[0].recovery_state,
        TacticalFighterRecoveryState::Recovered
    );
    assert!(!ordered.fighters[0].alive);
    assert_eq!(ordered.fighters[0].squad_count, 12);
    assert_eq!(ordered.ships[0].attack_target, None);
    assert_eq!(
        (ordered.ships[1].hull_current, ordered.ships[1].shield),
        ordered_capital_before
    );
    assert!(ordered.weapon_effects.is_empty());
    assert!(ordered.impact_effects.is_empty());
    assert!(ordered.pending_audio_cues.is_empty());

    // With no order, a laser-armed ship's force takes the nearest enemy
    // fighter group as its target (FUN_005cb6e0, FUN_005aa970), and its
    // own search finds only fighter groups (FUN_005b3a40 on the fighter
    // tree +0x900, FUN_005cdf50): it never fires at the capital ship.
    let mut idle = session_with_source_order(TacticalOrder::None);
    let idle_capital_before = (idle.ships[1].hull_current, idle.ships[1].shield);
    for _ in 0..24 {
        idle.step();
    }
    assert_eq!(
        (idle.ships[1].hull_current, idle.ships[1].shield),
        idle_capital_before
    );
    assert!(idle.weapon_effects.is_empty());

    // The capital ship as the force's target (FUN_005b6530) is hit.
    let mut autonomous = session_with_source_order(TacticalOrder::AttackCapitalShips);
    let autonomous_capital_before = (autonomous.ships[1].hull_current, autonomous.ships[1].shield);

    // Decided at turn 1 (step 4), the fire event runs at turn 5 (step
    // 20) and the hit at turn 6 (step 24) (FUN_005b3f10: +0x87c + 1.0
    // and + 2.0).
    for _ in 0..19 {
        autonomous.step();
    }
    assert!(autonomous.weapon_effects.is_empty());
    autonomous.step();
    assert_eq!(autonomous.weapon_effects.len(), 1);
    assert!(autonomous.impact_effects.is_empty());
    assert_eq!(
        (autonomous.ships[1].hull_current, autonomous.ships[1].shield),
        autonomous_capital_before
    );
    for _ in 0..4 {
        autonomous.step();
    }

    assert_ne!(
        (autonomous.ships[1].hull_current, autonomous.ships[1].shield),
        autonomous_capital_before
    );
    assert_eq!(autonomous.weapon_effects.len(), 1);
    assert_eq!(autonomous.impact_effects.len(), 1);
    assert!(!autonomous.pending_audio_cues.is_empty());
}

/// Runs the shots' fire events, then their hit events, as the queue does
/// one turn apart (`FUN_005b3f10`: `+0x87c + 1.0` and `+ 2.0`), then the
/// subsystem hits the hits posted (`FUN_005b5f50`, events `0x1e`..`0x22`).
fn resolve_test_shots(
    ships: Vec<TacticalShip>,
    shots: &[rebellion_core::tactical::TacticalShot],
) -> BattleSession {
    let mut session = test_session(ships, Vec::new(), true);
    session.tactical_rng = OriginalTacticalRng::new(1);
    for &shot in shots {
        session.fire_weapon_shot(shot);
    }
    for &shot in shots {
        session.hit_weapon_shot(shot);
    }
    for queued in std::mem::take(&mut session.turn_events) {
        if let rebellion_core::tactical::TacticalTurnEvent::SubsystemHit { ship, kind } =
            queued.event
        {
            session.hit_subsystem(ship, kind);
        }
    }
    session
}

/// The same for shots a session's own fire step decided.
fn run_posted_orders(session: &mut BattleSession) {
    session.run_posted_order_executors();
}

fn resolve_session_shots(
    session: &mut BattleSession,
    shots: &[rebellion_core::tactical::TacticalShot],
) {
    for &shot in shots {
        session.fire_weapon_shot(shot);
    }
    for &shot in shots {
        session.hit_weapon_shot(shot);
    }
}

// Source: FUN_005d0bb0 / FUN_005a8fc0 explicit attack targeting.
#[test]
fn explicit_attack_order_never_redirects_an_invalid_target() {
    let mut source = test_ship(64, 0, true, true);
    source.order = TacticalOrder::AttackCapitalShips;
    source.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
    source.weapon_arcs[0] = TacticalWeaponArc::new(300, 0, 0);
    source.weapon_ranges.laser_cannon = 100.0;
    let dead_target = test_ship(128, 0, false, false);
    let live_target = test_ship(129, 1, false, true);
    let ships = vec![source, dead_target, live_target];
    let original_hull = ships[2].hull_current;

    let mut shots = Vec::new();
    let mut firing = test_session(ships, Vec::new(), true);
    firing.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Ship(0),
        &mut shots,
    );
    let resolved = resolve_test_shots(firing.ships, &shots);
    let ships = &resolved.ships;
    let projectiles = &resolved.weapon_effects;
    let impacts = &resolved.impact_effects;
    let audio_cues = &resolved.pending_audio_cues;

    assert_eq!(ships[2].hull_current, original_hull);
    assert!(projectiles.is_empty());
    assert!(impacts.is_empty());
    assert!(audio_cues.is_empty());
}

// Source: FUN_005b5100 / FUN_005b5b60 / FUN_005b5f50 hull damage selection.
#[test]
fn hull_subsystem_selection_preserves_every_source_boundary() {
    assert_eq!(original_hull_subsystem_kind(50.0), None);
    assert_eq!(
        original_hull_subsystem_kind(50.001),
        Some(TacticalSubsystemKind::Shields)
    );
    assert_eq!(
        original_hull_subsystem_kind(65.0),
        Some(TacticalSubsystemKind::Shields)
    );
    assert_eq!(
        original_hull_subsystem_kind(65.001),
        Some(TacticalSubsystemKind::Weapons)
    );
    assert_eq!(
        original_hull_subsystem_kind(80.001),
        Some(TacticalSubsystemKind::Tractor)
    );
    assert_eq!(
        original_hull_subsystem_kind(90.001),
        Some(TacticalSubsystemKind::Engines)
    );
    assert_eq!(
        original_hull_subsystem_kind(95.001),
        Some(TacticalSubsystemKind::Hyperdrive)
    );
    assert_eq!(original_hull_subsystem_kind(100.001), None);
}

// Source: FUN_005b54d0 shield damage probability.
#[test]
fn shield_damage_uses_inclusive_source_probability() {
    let mut ship = test_ship(64, 0, true, true);
    ship.hull_current = 1000;
    ship.hull_max = 1000;
    ship.shield = 100;
    ship.shield_max = 100;
    ship.subsystem_capacity.shields = true;
    ship.refresh_subsystem_condition();
    let mut ships = vec![ship.clone()];
    let mut fields = Vec::new();
    apply_original_capital_damage(
        &mut ships,
        &mut fields,
        0,
        20,
        OriginalTacticalDamageRolls {
            shield: 20,
            hull: 0,
        },
    );
    assert_eq!(ships[0].subsystem_damage.shields, 1);
    assert_eq!(ships[0].subsystem_condition.shields, 75);

    let mut ships = vec![ship];
    apply_original_capital_damage(
        &mut ships,
        &mut fields,
        0,
        20,
        OriginalTacticalDamageRolls {
            shield: 21,
            hull: 0,
        },
    );
    assert_eq!(ships[0].subsystem_damage.shields, 0);
    assert_eq!(ships[0].subsystem_condition.shields, 100);
}

// Source: FUN_005b5100 / FUN_005b5b60 / FUN_005b5f50 hull damage subsystems.
#[test]
fn hull_damage_mutates_each_source_subsystem_category() {
    let cases = [
        (
            51,
            TacticalSubsystemDamage {
                shields: 1,
                ..TacticalSubsystemDamage::default()
            },
        ),
        (
            66,
            TacticalSubsystemDamage {
                weapons: 1,
                ..TacticalSubsystemDamage::default()
            },
        ),
        (
            81,
            TacticalSubsystemDamage {
                tractor: 1,
                ..TacticalSubsystemDamage::default()
            },
        ),
        (
            91,
            TacticalSubsystemDamage {
                engines: 1,
                ..TacticalSubsystemDamage::default()
            },
        ),
        (
            96,
            TacticalSubsystemDamage {
                hyperdrive: 1,
                ..TacticalSubsystemDamage::default()
            },
        ),
    ];
    for (roll, expected) in cases {
        let mut ship = test_ship(64, 0, true, true);
        ship.hull_current = 1000;
        ship.hull_max = 1000;
        ship.subsystem_capacity = TacticalSubsystemCapacity {
            shields: true,
            weapons: true,
            tractor: true,
            engines: true,
            hyperdrive: 2,
        };
        ship.refresh_subsystem_condition();
        let mut ships = vec![ship];
        let stage = apply_original_capital_damage(
            &mut ships,
            &mut Vec::new(),
            0,
            1,
            OriginalTacticalDamageRolls {
                shield: 100,
                hull: roll,
            },
        );
        assert_eq!(stage, OriginalTacticalImpactStage::Damage);
        assert_eq!(ships[0].hull_current, 999);
        assert_eq!(ships[0].subsystem_damage, expected);
    }
}

// Source: FUN_005b5100 hull damage subsystem destruction path.
#[test]
fn destroyed_tractor_subsystem_clears_its_live_field_source() {
    let mut source = test_ship(64, 0, true, true);
    source.hull_current = 1000;
    source.hull_max = 1000;
    source.subsystem_capacity.tractor = true;
    source.subsystem_damage.tractor = 3;
    source.refresh_subsystem_condition();
    let target = test_ship(128, 0, false, true);
    let mut ships = vec![source, target];
    let mut fields = Vec::new();
    assert!(set_original_tactical_field(
        &mut fields,
        0,
        TacticalUnitRef::Ship(1),
        OriginalTacticalFieldKind::Tractor,
        true,
    ));

    apply_original_capital_damage(
        &mut ships,
        &mut fields,
        0,
        1,
        OriginalTacticalDamageRolls {
            shield: 100,
            hull: 81,
        },
    );
    assert_eq!(ships[0].subsystem_damage.tractor, 4);
    assert_eq!(ships[0].subsystem_condition.tractor, 0);
    assert!(fields.is_empty());
}

// Source: FUN_005b6530 weapon arc classifier.
#[test]
fn weapon_arc_classifier_preserves_source_order_and_boundaries() {
    let source = test_ship(64, 0, true, true);
    let target_at = |x: f32, z: f32| {
        let mut target = test_ship(128, 0, false, true);
        target.source_position = TacticalWorldPosition { x, y: 0.0, z };
        target
    };

    assert_eq!(
        original_tactical_weapon_arc(&source, &target_at(0.0, 10.0)),
        OriginalTacticalWeaponArc::Fore
    );
    assert_eq!(
        original_tactical_weapon_arc(&source, &target_at(10.0, 0.0)),
        OriginalTacticalWeaponArc::Starboard
    );
    assert_eq!(
        original_tactical_weapon_arc(&source, &target_at(0.0, -10.0)),
        OriginalTacticalWeaponArc::Aft
    );
    assert_eq!(
        original_tactical_weapon_arc(&source, &target_at(-10.0, 0.0)),
        OriginalTacticalWeaponArc::Port
    );
    assert_eq!(
        original_tactical_weapon_arc(&source, &target_at(10.0, 10.0)),
        OriginalTacticalWeaponArc::Fore,
        "the +45 degree boundary belongs to the fore arc"
    );
    assert_eq!(
        original_tactical_weapon_arc(&source, &target_at(10.0, -10.0)),
        OriginalTacticalWeaponArc::Starboard,
        "the +135 degree boundary remains lateral"
    );
    assert_eq!(
        original_tactical_weapon_arc(&source, &target_at(-10.0, -10.0)),
        OriginalTacticalWeaponArc::Port,
        "the -135 degree boundary remains lateral"
    );
}

// Source: FUN_005b3a40 / FUN_005b3f10 / FUN_005b6530 weapon fire.
#[test]
fn direct_weapon_callback_fires_every_family_and_queues_the_arc_once() {
    let mut source = test_ship(64, 0, true, true);
    source.hull_current = 1000;
    source.hull_max = 1000;
    source.weapon_arcs[0] = TacticalWeaponArc::new(60, 40, 80);
    source.weapon_ranges = TacticalWeaponRanges {
        laser_cannon: 100.0,
        ion_cannon: 100.0,
        turbolaser: 100.0,
    };
    source.order = TacticalOrder::AttackCapitalShips;
    source.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
    let mut target = test_ship(128, 0, false, true);
    target.source_position.z = 10.0;
    target.hull_current = 1000;
    target.hull_max = 1000;
    target.shield = 1000;
    target.shield_max = 1000;
    let ships = vec![source, target];

    let mut shots = Vec::new();
    let mut firing = test_session(ships, Vec::new(), true);
    firing.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Ship(0),
        &mut shots,
    );
    let resolved = resolve_test_shots(firing.ships, &shots);
    let ships = &resolved.ships;
    let projectiles = &resolved.weapon_effects;
    let audio_cues = &resolved.pending_audio_cues;

    assert_eq!(
        projectiles
            .iter()
            .map(|effect| effect.kind)
            .collect::<Vec<_>>(),
        vec![
            WeaponKind::IonCannon,
            WeaponKind::LaserCannon,
            WeaponKind::Turbolaser
        ]
    );
    assert_eq!(ships[0].weapon_recharge_queue, vec![0]);
    assert!(!ships[0].weapon_arcs[0].ready);
    assert_eq!(ships[0].weapon_arcs[0].energy, 0.0);
    assert_eq!(ships[1].shield, 965);
    assert_eq!(
        audio_cues.iter().map(|cue| cue.event).collect::<Vec<_>>(),
        // The fire events run one turn before the hit events
        // (FUN_005b3f10: +0x87c + 1.0 and + 2.0).
        // Fired at a capital ship (vtable 0x0066c140: events 2, 0, 1),
        // and held by its shield (0x0e, 0x0d, 0x0c).
        vec![
            TacticalAudioEvent::IonAtCapital,
            TacticalAudioEvent::LaserAtCapital,
            TacticalAudioEvent::Turbolaser,
            TacticalAudioEvent::IonShieldHit,
            TacticalAudioEvent::LaserShieldHit,
            TacticalAudioEvent::ShieldHit,
        ]
    );
    assert!(audio_cues.iter().all(|cue| {
        let base = cue.event.resource_base();
        cue.resource_id >= base && cue.resource_id < base + u32::from(cue.event.variant_count())
    }));
}

// Source: FUN_005b3a40 / FUN_005b3f10 automatic weapon fire. The search
// takes enemy fighter groups only (FUN_005cdf50 on the fighter tree
// +0x900), so the two targets are groups.
#[test]
fn automatic_weapon_callback_keeps_first_arc_on_ties_and_fires_each_ready_arc() {
    let mut source = test_ship(64, 0, true, true);
    source.hull_current = 100;
    source.hull_max = 100;
    source.weapon_arcs[0] = TacticalWeaponArc::new(0, 0, 30);
    source.weapon_arcs[1] = TacticalWeaponArc::new(0, 0, 30);
    source.weapon_ranges.turbolaser = 100.0;
    // Attack Fighters with no target searches (order 4, 0x5b3b83).
    source.order = TacticalOrder::AttackFighters;
    let mut fore = test_fighter(5, false);
    fore.source_position.z = 10.0;
    let mut starboard = test_fighter(5, false);
    starboard.source_position = TacticalWorldPosition {
        x: 10.0,
        y: 0.0,
        z: 0.0,
    };
    let mut session = test_session(vec![source], vec![fore, starboard], true);

    let mut shots = Vec::new();
    session.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Ship(0),
        &mut shots,
    );
    resolve_session_shots(&mut session, &shots);

    assert_eq!(
        session
            .weapon_effects
            .iter()
            .map(|effect| effect.target_position)
            .collect::<Vec<_>>(),
        vec![
            session.fighters[0].source_position,
            session.fighters[1].source_position
        ]
    );
    assert_eq!(session.ships[0].weapon_recharge_queue, vec![0, 1]);
    assert_eq!(session.pending_audio_cues.len(), 4);
}

// Source: FUN_005d2360 weapon strength coefficient; FUN_005b6530 gives a
// direct target its full strength, the search of FUN_005b3a40 scales
// its fighter targets by the hull (FUN_005b1770).
#[test]
fn weapon_candidates_enforce_range_and_apply_hull_scaling_only_to_auto_fire() {
    let strengths = |z: f32| {
        let mut source = test_ship(64, 0, true, true);
        source.hull_current = 50;
        source.hull_max = 100;
        source.weapon_arcs[0] = TacticalWeaponArc::new(0, 0, 100);
        source.weapon_ranges.turbolaser = 10.0;
        source.order = TacticalOrder::AttackCapitalShips;
        source.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        let mut target = test_ship(128, 0, false, true);
        target.source_position.z = z;
        let direct = test_session(vec![source.clone(), target], Vec::new(), true);
        source.order = TacticalOrder::AttackFighters;
        source.attack_target = None;
        let mut wing = test_fighter(5, false);
        wing.source_position.z = z;
        let automatic = test_session(vec![source], vec![wing], true);
        [direct, automatic].map(|mut session| {
            let mut shots = Vec::new();
            session.fire_unit_weapons(
                rebellion_core::tactical::TacticalUnitRef::Ship(0),
                &mut shots,
            );
            shots.iter().map(|shot| shot.strength).collect::<Vec<_>>()
        })
    };
    let [direct, automatic] = strengths(10.0);
    assert_eq!(direct.len(), 1);
    assert!((direct[0] - 26.666_668).abs() < 0.000_01);
    // A turbolaser's half against fighters (FUN_005d2490), then the hull.
    assert_eq!(automatic.len(), 1);
    assert!((automatic[0] - 6.666_667).abs() < 0.000_01);

    let [direct, automatic] = strengths(10.001);
    assert!(direct.is_empty());
    assert!(automatic.is_empty());
}

// Source: FUN_005b64e0 / FUN_005b6320 weapon recharge FIFO.
#[test]
fn weapon_recharge_is_fifo_and_preserves_leftover_energy() {
    let mut ship = test_ship(64, 0, true, true);
    ship.hull_current = 100;
    ship.hull_max = 100;
    ship.weapon_recharge_rate = 8.0;
    ship.weapon_arcs[0] = TacticalWeaponArc::new(0, 0, 10);
    ship.weapon_arcs[1] = TacticalWeaponArc::new(0, 0, 10);
    for arc in &mut ship.weapon_arcs[..2] {
        arc.ready = false;
        arc.energy = 0.0;
    }
    ship.weapon_recharge_queue = vec![0, 1];

    ship.advance_original_recharge(1.0);
    assert_eq!(ship.weapon_arcs[0].energy, 8.0);
    assert_eq!(ship.weapon_arcs[1].energy, 0.0);
    assert_eq!(ship.weapon_recharge_queue, vec![0, 1]);

    ship.advance_original_recharge(0.5);
    assert_eq!(ship.weapon_arcs[0].energy, 10.0);
    assert!(ship.weapon_arcs[0].ready);
    assert_eq!(ship.weapon_arcs[1].energy, 2.0);
    assert_eq!(ship.weapon_recharge_queue, vec![1]);
}

// Source: FUN_005b05c0 weapon recharge scale 3.75.
#[test]
fn shield_recharge_retains_fractional_source_energy_and_damage_penalties() {
    let mut ship = test_ship(64, 0, true, true);
    ship.hull_current = 100;
    ship.hull_max = 100;
    ship.shield = 0;
    ship.shield_max = 10;
    ship.shield_recharge_rate = 3.0;

    for _ in 0..4 {
        ship.advance_original_recharge(0.25);
    }
    assert_eq!(ship.shield, 3);
    assert_eq!(ship.shield_recharge_carry, 0.0);

    ship.subsystem_damage.shields = 1;
    ship.advance_original_recharge(1.0);
    assert_eq!(ship.shield, 5);
    assert!((ship.shield_recharge_carry - 0.25).abs() < f32::EPSILON);
}

// Source: FUN_005b3f10 subsystem condition affecting weapon arcs.
#[test]
fn destroyed_weapon_subsystem_disables_and_repair_recharges_live_arcs() {
    let mut ship = test_ship(64, 0, true, true);
    ship.hull_current = 100;
    ship.hull_max = 100;
    ship.subsystem_capacity.weapons = true;
    ship.weapon_recharge_rate = 10.0;
    ship.weapon_arcs[0] = TacticalWeaponArc::new(0, 0, 10);

    for _ in 0..4 {
        ship.add_subsystem_hit(TacticalSubsystemKind::Weapons);
    }
    assert!(!ship.weapon_arcs[0].ready);
    assert_eq!(ship.weapon_arcs[0].energy, 0.0);
    assert_eq!(ship.weapon_recharge_queue, vec![0]);
    ship.remove_subsystem_hit(TacticalSubsystemKind::Weapons);
    ship.advance_original_recharge(4.0);
    assert!(ship.weapon_arcs[0].ready);
    assert_eq!(ship.weapon_arcs[0].energy, 10.0);
    assert!(ship.weapon_recharge_queue.is_empty());
}

// Source: FUN_005b3a40 / FUN_005b3f10 fire and FUN_005b5100's damage:
// a shield that takes the whole shot loses 100%, so its draw of 0 to
// 100 is a shield hit, and the hull is untouched.
#[test]
fn live_ship_fire_uses_the_shared_subsystem_damage_path() {
    let mut source = test_ship(64, 0, true, true);
    source.hull_current = 1000;
    source.hull_max = 1000;
    source.weapon_arcs[0] = TacticalWeaponArc::new(300, 0, 0);
    source.weapon_ranges.laser_cannon = 100.0;
    // A capital ship fires at one only as its force's target
    // (FUN_005b3a40, FUN_005b6530).
    source.order = TacticalOrder::AttackCapitalShips;
    source.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
    let mut target = test_ship(128, 0, false, true);
    target.hull_current = 1000;
    target.hull_max = 1000;
    target.shield = 10;
    target.shield_max = 10;
    target.subsystem_capacity.shields = true;
    target.refresh_subsystem_condition();
    let ships = vec![source, target];

    let mut shots = Vec::new();
    let mut firing = test_session(ships, Vec::new(), true);
    firing.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Ship(0),
        &mut shots,
    );
    assert_eq!(shots.len(), 1);
    let damage = rebellion_core::tactical::original_tactical_integer_damage(shots[0].strength);
    firing.ships[1].shield = damage;
    firing.ships[1].shield_max = damage;
    let resolved = resolve_test_shots(firing.ships, &shots);
    let ships = &resolved.ships;
    let projectiles = &resolved.weapon_effects;
    let impacts = &resolved.impact_effects;
    let audio_cues = &resolved.pending_audio_cues;

    assert_eq!((ships[1].shield, ships[1].hull_current), (0, 1000));
    assert_eq!(ships[1].subsystem_damage.shields, 1);
    assert!(ships[1].subsystem_condition.shields < 100);
    assert_eq!(projectiles.len(), 1);
    assert_eq!(impacts.len(), 1);
    assert_eq!(audio_cues.len(), 2);
}

// Source: FUN_005bae60's event table (+0x50 counts, +0x51c words) and
// the sounds the weapon slots ask for (0x005b00a0..0x005b00e0,
// FUN_005ba0f0, FUN_005ba0b0, FUN_005b5100..FUN_005b5f50).
#[test]
fn tactical_audio_event_bank_covers_every_recovered_wave_once() {
    let events = [
        TacticalAudioEvent::LaserAtCapital,
        TacticalAudioEvent::Turbolaser,
        TacticalAudioEvent::IonAtCapital,
        TacticalAudioEvent::LaserAtAllianceFighter,
        TacticalAudioEvent::LaserAtImperialFighter,
        TacticalAudioEvent::IonAtFighter,
        TacticalAudioEvent::ShieldHit,
        TacticalAudioEvent::LaserShieldHit,
        TacticalAudioEvent::IonShieldHit,
        TacticalAudioEvent::HullHit,
        TacticalAudioEvent::LaserHullHit,
        TacticalAudioEvent::IonHullHit,
    ];
    let cues: Vec<_> = events
        .into_iter()
        .flat_map(|event| {
            (0..event.variant_count())
                .map(move |variant| TacticalAudioCue::from_variant(event, variant))
        })
        .collect();

    assert_eq!(
        cues.iter()
            .map(|cue| cue.event.event_id())
            .collect::<Vec<_>>(),
        vec![
            0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 4, 4, 5, 5, 0x0c, 0x0c, 0x0c, 0x0d, 0x0d,
            0x0d, 0x0e, 0x0e, 0x0e, 0x11, 0x11, 0x11, 0x12, 0x12, 0x12, 0x13, 0x13, 0x13,
        ]
    );
    assert_eq!(
        cues.iter().map(|cue| cue.resource_id).collect::<Vec<_>>(),
        (13_000..=13_017)
            .chain(13_030..=13_038)
            .chain(13_045..=13_053)
            .collect::<Vec<_>>()
    );
    // The jumps keep 13063 and 13065.
    assert_eq!(TacticalAudioEvent::FighterJump.resource_base(), 13_063);
    assert_eq!(TacticalAudioEvent::CapitalJump.resource_base(), 13_065);
}

// Source: FUN_005bad50 draws a variant only for 2 to 4 variants.
#[test]
fn a_sound_draws_its_variant_only_with_two_to_four_variants() {
    let mut rng = OriginalTacticalRng::new(7);
    let untouched = rng;
    assert_eq!(TacticalAudioEvent::CapitalJump.draw_variant(&mut rng), 0);
    assert_eq!(rng, untouched);
    let mut expected = rng;
    let variant = TacticalAudioEvent::LaserAtCapital.draw_variant(&mut rng);
    assert_eq!(u32::from(variant), expected.inclusive(0, 3));
    assert_eq!(rng, expected);
    let variant = TacticalAudioEvent::LaserAtAllianceFighter.draw_variant(&mut rng);
    assert_eq!(u32::from(variant), expected.inclusive(0, 1));
    let variant = TacticalAudioEvent::HullHit.draw_variant(&mut rng);
    assert_eq!(u32::from(variant), expected.inclusive(0, 2));
    assert_eq!(rng, expected);
}

// Source: FUN_0061a310 / FUN_005a8a70 MSVC-compatible global RNG.
#[test]
fn tactical_rng_preserves_the_recovered_recurrence_and_inclusive_range() {
    let mut rng = OriginalTacticalRng::new(1);
    assert_eq!(
        (0..6).map(|_| rng.next_u15()).collect::<Vec<_>>(),
        vec![41, 18_467, 6_334, 26_500, 19_169, 15_724]
    );

    let mut rng = OriginalTacticalRng::new(1);
    assert_eq!(rng.inclusive(1, 10), 2);
    assert_eq!(rng.inclusive(0, 100), 85);
    assert_ne!(rng.state(), 1);
}

#[test]
fn tactical_voice_cues_preserve_source_event_and_resource_families() {
    // FUN_005bae60 supplies the event/resource arithmetic; the named
    // command callers and audited voice map supply these family offsets.
    assert_eq!(
        TacticalVoiceCue::battle_ready(TacticalVoiceFaction::Alliance),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::BattleReady,
            faction: TacticalVoiceFaction::Alliance,
            source_event: 0x20,
            resource_id: 14_001,
        }
    );
    assert_eq!(
        TacticalVoiceCue::fighter_attack(TacticalVoiceFaction::Alliance, 3).resource_id,
        14_040
    );
    assert_eq!(
        TacticalVoiceCue::capital_maneuver_complete(TacticalVoiceFaction::Alliance, 7).source_event,
        0x36
    );
    assert_eq!(
        TacticalVoiceCue::fighter_maneuver_complete(TacticalVoiceFaction::Empire, 3).source_event,
        0xb4
    );
    assert_eq!(
        TacticalVoiceCue::capital_orders_complete(TacticalVoiceFaction::Alliance, 7).source_event,
        0x50
    );
    assert_eq!(
        TacticalVoiceCue::fighter_orders_complete(TacticalVoiceFaction::Empire, 3).source_event,
        0xce
    );
    assert_eq!(
        TacticalVoiceCue::target_destroyed_by_capital(TacticalVoiceFaction::Alliance, 7)
            .source_event,
        0x5d
    );
    assert_eq!(
        TacticalVoiceCue::target_destroyed_by_fighter(TacticalVoiceFaction::Empire, 3).source_event,
        0xdb
    );
    assert_eq!(
        TacticalVoiceCue::capital_destroyed(TacticalVoiceFaction::Alliance, 7).source_event,
        0x6a
    );
    assert_eq!(
        TacticalVoiceCue::fighter_losses(TacticalVoiceFaction::Empire, 3).source_event,
        0xe8
    );
    assert_eq!(
        TacticalVoiceCue::capital_formation(TacticalVoiceFaction::Empire, 7),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::CapitalFormation,
            faction: TacticalVoiceFaction::Empire,
            source_event: 0xf4,
            resource_id: 15_091,
        }
    );
    assert_eq!(
        TacticalVoiceCue::fighter_mission(TacticalVoiceFaction::Empire, 3).resource_id,
        15_104
    );
    assert_eq!(
        TacticalVoiceCue::withdrawal_started(TacticalVoiceFaction::Alliance),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::WithdrawalStarted,
            faction: TacticalVoiceFaction::Alliance,
            source_event: 0x8f,
            resource_id: 14_112,
        }
    );
    assert_eq!(
        TacticalVoiceCue::fighter_recovery_complete(TacticalVoiceFaction::Alliance, 0),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::FighterRecoveryComplete,
            faction: TacticalVoiceFaction::Alliance,
            source_event: 0x87,
            resource_id: 14_104,
        }
    );
    assert_eq!(
        TacticalVoiceCue::fighter_recovery_complete(TacticalVoiceFaction::Empire, 3),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::FighterRecoveryComplete,
            faction: TacticalVoiceFaction::Empire,
            source_event: 0x108,
            resource_id: 15_111,
        }
    );
    assert_eq!(
        TacticalVoiceCue::fighter_recovery_capacity(TacticalVoiceFaction::Alliance, 3),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::FighterRecoveryCapacity,
            faction: TacticalVoiceFaction::Alliance,
            source_event: 0x8e,
            resource_id: 14_111,
        }
    );
    assert_eq!(
        TacticalVoiceCue::fighter_recovery_capacity(TacticalVoiceFaction::Empire, 0),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::FighterRecoveryCapacity,
            faction: TacticalVoiceFaction::Empire,
            source_event: 0x109,
            resource_id: 15_112,
        }
    );
    assert_eq!(
        TacticalVoiceCue::withdrawal_hyperdrive_warning(TacticalVoiceFaction::Alliance),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::WithdrawalHyperdriveWarning,
            faction: TacticalVoiceFaction::Alliance,
            source_event: 0x95,
            resource_id: 14_118,
        }
    );
    assert_eq!(
        TacticalVoiceCue::withdrawal_hyperdrive_warning(TacticalVoiceFaction::Empire),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::WithdrawalHyperdriveWarning,
            faction: TacticalVoiceFaction::Empire,
            source_event: 0x10e,
            resource_id: 15_117,
        }
    );
    assert_eq!(
        TacticalVoiceCue::battle_won(TacticalVoiceFaction::Empire, false).resource_id,
        15_129
    );
    assert_eq!(TacticalVoiceCue::death_star_firing().resource_id, 15_120);
    assert_eq!(TacticalVoiceCue::trench_run_started(0).source_event, 0x139);
    assert_eq!(
        TacticalVoiceCue::trench_run_committed(0).source_event,
        0x13a
    );
    assert_eq!(
        TacticalVoiceCue::trench_run_outcome(3, TacticalTrenchRunOutcome::Success),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::TrenchRunSucceeded,
            faction: TacticalVoiceFaction::Alliance,
            source_event: 0x138,
            resource_id: 15_159,
        }
    );
    assert_eq!(
        TacticalVoiceCue::target_task_force_rejected(TacticalVoiceFaction::Alliance),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::TargetTaskForceRejected,
            faction: TacticalVoiceFaction::Alliance,
            source_event: 0x84,
            resource_id: 14_101,
        }
    );
    assert_eq!(
        TacticalVoiceCue::target_task_force_rejected(TacticalVoiceFaction::Empire),
        TacticalVoiceCue {
            event: TacticalVoiceEvent::TargetTaskForceRejected,
            faction: TacticalVoiceFaction::Empire,
            source_event: 0x102,
            resource_id: 15_105,
        }
    );

    let mut empire = test_session(Vec::new(), Vec::new(), false);
    empire.queue_battle_ready_voice();
    assert_eq!(
        empire.take_pending_voice_cues(),
        vec![TacticalVoiceCue::battle_ready(TacticalVoiceFaction::Empire)]
    );
    empire.queue_withdrawal_started_voice();
    assert_eq!(
        empire.take_pending_voice_cues(),
        vec![TacticalVoiceCue::withdrawal_started(
            TacticalVoiceFaction::Empire
        )]
    );
}

// Source: FUN_005a5cf0 chooses the local firing group for a destroyed
// target and the local casualty group when the opponent destroys it.
#[test]
fn destruction_voice_callback_uses_source_and_target_group_perspectives() {
    let mut cues = Vec::new();
    queue_original_destruction_voice(
        &mut cues,
        TacticalVoiceFaction::Alliance,
        true,
        true,
        TacticalVoiceGroup::Fighter(2),
        false,
        TacticalVoiceGroup::Capital(4),
        OriginalTacticalImpactStage::Destroyed,
    );
    queue_original_destruction_voice(
        &mut cues,
        TacticalVoiceFaction::Alliance,
        true,
        false,
        TacticalVoiceGroup::Capital(3),
        true,
        TacticalVoiceGroup::Capital(4),
        OriginalTacticalImpactStage::Destroyed,
    );
    queue_original_destruction_voice(
        &mut cues,
        TacticalVoiceFaction::Alliance,
        true,
        false,
        TacticalVoiceGroup::Capital(3),
        true,
        TacticalVoiceGroup::Fighter(1),
        OriginalTacticalImpactStage::Destroyed,
    );
    queue_original_destruction_voice(
        &mut cues,
        TacticalVoiceFaction::Alliance,
        true,
        true,
        TacticalVoiceGroup::Capital(0),
        false,
        TacticalVoiceGroup::Capital(0),
        OriginalTacticalImpactStage::Damage,
    );

    assert_eq!(
        cues,
        vec![
            TacticalVoiceCue::target_destroyed_by_fighter(TacticalVoiceFaction::Alliance, 2,),
            TacticalVoiceCue::capital_destroyed(TacticalVoiceFaction::Alliance, 4),
            TacticalVoiceCue::fighter_losses(TacticalVoiceFaction::Alliance, 1),
        ]
    );
}

#[test]
fn trench_run_groups_route_to_their_exact_source_start_and_outcome_events() {
    // FUN_005bae60 maps these four Alliance group triplets into the
    // recovered 15133..15163 trench-run voice bank.
    let expected = [
        (0, 0x139, 0x13b, 0x13c),
        (1, 0x12d, 0x12f, 0x130),
        (2, 0x131, 0x133, 0x134),
        (3, 0x135, 0x137, 0x138),
    ];

    for (group, start, failure, success) in expected {
        assert_eq!(
            TacticalVoiceCue::trench_run_started(group).source_event,
            start
        );
        assert_eq!(
            TacticalVoiceCue::trench_run_outcome(group, TacticalTrenchRunOutcome::Failure)
                .source_event,
            failure
        );
        assert_eq!(
            TacticalVoiceCue::trench_run_outcome(group, TacticalTrenchRunOutcome::Success)
                .source_event,
            success
        );
    }
}

// FUN_005d0ae0: a run under way, its approach included, takes no
// second Attack Death Star order.
#[test]
fn a_second_attack_death_star_order_waits_for_the_run_in_its_approach() {
    let mut fighter = test_fighter(3, true);
    fighter.order = TacticalOrder::AttackDeathStar;
    let mut session = test_session(Vec::new(), vec![fighter], true);
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });
    begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);
    assert_eq!(session.take_pending_voice_cues().len(), 1);
    begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);
    assert!(session.take_pending_voice_cues().is_empty());
    assert_eq!(session.turn_events.len(), 1);
}

#[test]
fn death_star_attack_entry_routes_each_local_perspective_once() {
    // FUN_005d0340 selects the active Alliance RGBG call for the attacking
    // player and source event 0x115 for the Imperial station owner at the
    // order; FUN_005cfec0 state 5 then plays the follow-up, or 0x11d.
    let mut attacking_fighter = test_fighter(3, true);
    attacking_fighter.fighter_group = 2;
    attacking_fighter.order = TacticalOrder::AttackDeathStar;
    let mut alliance = test_session(Vec::new(), vec![attacking_fighter], true);
    alliance.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });
    begin_original_attack_order(&mut alliance, &[], &[0], TacticalOrder::AttackDeathStar);
    assert_eq!(
        alliance.take_pending_voice_cues(),
        vec![TacticalVoiceCue::trench_run_started(2)]
    );
    assert_eq!(alliance.trench_run_remaining_ms, None);
    assert_eq!(alliance.trench_run_group, Some(2));
    // The result object posts its state 2 (0x005d0884) at +0x87c.
    assert_eq!(
        alliance.turn_events,
        vec![rebellion_core::tactical::TacticalQueuedEvent {
            stamp: 3,
            event: rebellion_core::tactical::TacticalTurnEvent::TrenchRunState(2),
        }]
    );
    alliance.commit_original_trench_run();
    assert_eq!(
        alliance.take_pending_voice_cues(),
        vec![
            TacticalVoiceCue::trench_run_committed(2),
            TacticalVoiceCue::trench_run_chatter(0x11e),
        ]
    );
    assert_eq!(
        alliance.trench_run_remaining_ms,
        Some(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS)
    );

    let mut attacking_fighter = test_fighter(3, true);
    attacking_fighter.fighter_group = 2;
    attacking_fighter.order = TacticalOrder::AttackDeathStar;
    let mut empire = test_session(Vec::new(), vec![attacking_fighter], false);
    empire.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });
    begin_original_attack_order(&mut empire, &[], &[0], TacticalOrder::AttackDeathStar);
    assert_eq!(
        empire.take_pending_voice_cues(),
        vec![TacticalVoiceCue::death_star_under_attack()]
    );
    empire.commit_original_trench_run();
    assert_eq!(
        empire.take_pending_voice_cues(),
        vec![
            TacticalVoiceCue::death_star_attack_continuing(),
            TacticalVoiceCue::trench_run_chatter(0x11e),
        ]
    );
    assert_eq!(
        empire.trench_run_remaining_ms,
        Some(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS)
    );
}

#[test]
fn tactical_voice_bank_maps_all_source_events_to_exact_resources() {
    // FUN_005bae60 registers every event from 0x20 through 0x13c.
    let cues = (0x20..=0x13c)
        .map(|source_event| {
            TacticalVoiceCue::from_source_event(TacticalVoiceEvent::SourceMapped, source_event)
        })
        .collect::<Vec<_>>();
    assert_eq!(cues.len(), 285);
    assert_eq!(
        cues.iter()
            .map(|cue| (cue.faction, cue.resource_id))
            .collect::<HashSet<_>>()
            .len(),
        285
    );
    assert_eq!(cues.first().unwrap().resource_id, 14_001);
    assert_eq!(cues[121].resource_id, 14_122);
    assert_eq!(cues[122].resource_id, 15_001);
    assert_eq!(cues[253].resource_id, 15_132);
    assert_eq!(cues[254].resource_id, 15_133);
    assert_eq!(cues.last().unwrap().resource_id, 15_163);
}

#[cfg(feature = "interface-test-fixtures")]
#[test]
fn tactical_audio_fixture_queues_every_weapon_and_voice_resource_once() {
    // The fixture mirrors FUN_005bae60's complete source-event mapping,
    // grouped by the two owned resource DLLs for deterministic loading.
    // The unit sounds follow the weapons: 0x14..0x19 (13054-13059,
    // FUN_005ba520, FUN_005b8df0, FUN_005b0100), 0x1b (13061,
    // FUN_005a9170), 0x1c and 0x1e (13062, 13064, FUN_005a6d60).
    let mut state = TacticalState {
        session: Some(test_session(Vec::new(), Vec::new(), true)),
        ..TacticalState::default()
    };

    state.configure_tactical_audio_fixture();

    let session = state.session.as_mut().unwrap();
    assert_eq!(
        session
            .take_pending_audio_cues()
            .into_iter()
            .map(|cue| cue.resource_id)
            .collect::<Vec<_>>(),
        (13_000..=13_017)
            .chain(13_030..=13_038)
            .chain(13_045..=13_059)
            .chain([13_061, 13_062, 13_064])
            .collect::<Vec<_>>()
    );
    let voice_resources = session
        .take_pending_voice_cues()
        .into_iter()
        .map(|cue| cue.resource_id)
        .collect::<Vec<_>>();
    let expected = (14_001..=14_122)
        .chain(15_133..=15_163)
        .chain(15_001..=15_132)
        .collect::<Vec<_>>();
    assert_eq!(voice_resources, expected);
    assert_eq!(voice_resources.len(), 285);
}

// Source: FUN_005d3ac0 field slot sequence.
#[test]
fn original_slot_sequence_matches_executable_branch_order() {
    let actual: Vec<_> = OriginalTacticalSlots::default().take(9).collect();
    assert_eq!(actual[0].to_bits(), (-0.0f32).to_bits());
    assert_eq!(
        actual,
        vec![0.0, 5.0, -5.0, -10.0, 10.0, 15.0, -15.0, -20.0, 20.0]
    );
}

// Source: FUN_005ab650 / FUN_005a9030 world positions from layout.
#[test]
fn production_participants_retain_dat_identity_and_source_world_positions() {
    let mut ships = vec![
        test_ship(64, 0, true, true),
        test_ship(65, 2, true, true),
        test_ship(66, 7, true, false),
        test_ship(133, 0, false, true),
        test_ship(134, 1, false, true),
        test_ship(135, 2, false, true),
    ];
    let mut fighters = vec![test_fighter(3, true), test_fighter(5, false)];

    let layout = BattleSession::assign_original_world_positions(&mut ships, &mut fighters);
    assert_eq!(layout.first_active_objects, 3);
    assert_eq!(layout.second_active_objects, 4);
    assert_eq!(layout.battle_extent, 112.0);
    assert_eq!(
        ships[0].source_position,
        TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: -56.0
        }
    );
    assert_eq!(
        ships[1].source_position,
        TacticalWorldPosition {
            x: 5.0,
            y: 0.0,
            z: -56.0
        }
    );
    assert_eq!(ships[2].source_position, TacticalWorldPosition::ORIGIN);
    assert_eq!(
        ships[3].source_position,
        TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 56.0
        }
    );
    assert_eq!(
        ships[4].source_position,
        TacticalWorldPosition {
            x: 5.0,
            y: 0.0,
            z: 56.0
        }
    );
    assert_eq!(
        ships[5].source_position,
        TacticalWorldPosition {
            x: -5.0,
            y: 0.0,
            z: 56.0
        }
    );
    assert_eq!(
        fighters[0].source_position,
        TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: -36.0
        }
    );
    assert_eq!(
        fighters[1].source_position,
        TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 36.0
        }
    );
    assert_eq!(ships[1].identity.class_dat_id, DatId::new(65));
    assert_eq!(ships[1].identity.fleet_roster_index, 2);
}

// Source: FUN_005ae460 fighter group assignment.
#[test]
fn fighter_groups_follow_source_ordinals_for_both_factions() {
    let mut fighters = [
        test_fighter(3, true),
        test_fighter(4, true),
        test_fighter(1, true),
        test_fighter(2, true),
        test_fighter(5, false),
        test_fighter(6, false),
        test_fighter(7, false),
        test_fighter(8, false),
    ];
    for fighter in &mut fighters {
        fighter.fighter_group = 4;
    }

    BattleSession::assign_original_fighter_groups(&mut fighters);
    assert_eq!(
        fighters.map(|fighter| fighter.fighter_group),
        [0, 1, 2, 3, 0, 1, 2, 3]
    );
}

// Source: FUN_005c81d0 task force leader selection and slots.
#[test]
fn task_force_leaders_follow_weapon_categories_square_root_counts_and_source_slots() {
    let mut ships = (0..9)
        .map(|index| test_ship(64, index, true, true))
        .collect::<Vec<_>>();
    for ship in &mut ships[..4] {
        ship.weapon_arcs[0] = TacticalWeaponArc::new(0, 1, 0);
    }
    for ship in &mut ships[4..8] {
        ship.weapon_arcs[0] = TacticalWeaponArc::new(1, 0, 0);
    }

    BattleSession::assign_original_task_forces(&mut ships);

    assert_eq!(
        ships.iter().map(|ship| ship.task_force).collect::<Vec<_>>(),
        vec![0, 0, 1, 1, 2, 2, 3, 3, 4]
    );
    assert_eq!(ships[0].source_position.y, 0.0);
    assert_eq!(ships[2].source_position.y, 8.0);
    assert_eq!(ships[4].source_position.y, -8.0);
    assert_eq!(ships[6].source_position.y, 16.0);
    assert_eq!(ships[8].source_position.y, -16.0);
}

// Source: FUN_005c81d0 sorts by -(sublight + 10000 * range).
#[test]
fn task_force_capability_order_prioritizes_range_then_available_sublight() {
    let mut ships = (0..4)
        .map(|index| test_ship(64, index, true, true))
        .collect::<Vec<_>>();
    for ship in &mut ships {
        ship.weapon_arcs[0] = TacticalWeaponArc::new(1, 0, 0);
    }
    ships[0].weapon_ranges.laser_cannon = 10.0;
    ships[0].sublight_engine_power = 50.0;
    ships[1].weapon_ranges.laser_cannon = 10.0;
    ships[1].sublight_engine_power = 200.0;
    ships[2].weapon_ranges.laser_cannon = 20.0;
    ships[2].sublight_engine_power = 1.0;
    ships[3].weapon_ranges.laser_cannon = 5.0;
    ships[3].sublight_engine_power = 500.0;

    BattleSession::assign_original_task_forces(&mut ships);

    assert_eq!(
        ships.iter().map(|ship| ship.task_force).collect::<Vec<_>>(),
        vec![1, 0, 0, 1]
    );
    assert_eq!(ships[2].source_position.x, 0.0);
    assert_eq!(ships[0].source_position.x, 0.0);
}

// Source: FUN_005ea250 / FUN_005ec400 formation footprint and slots;
// the first follower stands on the slot axis's negative side
// (0x005ec448..0x005ec452, the anchor less the slot at 0x005eb040).
#[test]
fn task_force_followers_use_largest_footprint_and_alternating_slots() {
    let mut ships = (0..3)
        .map(|index| test_ship(64, index, true, true))
        .collect::<Vec<_>>();
    for ship in &mut ships {
        ship.weapon_arcs[0] = TacticalWeaponArc::new(1, 0, 0);
        ship.source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 4.0,
        });
    }
    ships[1].source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
        vertical_radius: 2.0,
        planar_diameter: 10.0,
    });
    ships[0].source_position.x = -0.0;

    BattleSession::assign_original_task_forces(&mut ships);

    assert_eq!(ships[0].task_force, 0);
    assert_eq!(ships[1].task_force, 0);
    assert_eq!(ships[2].task_force, 0);
    assert_eq!(ships[0].source_position.x.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(ships[1].source_position.x, -12.0);
    assert_eq!(ships[2].source_position.x, 12.0);
    assert!(ships.iter().all(|ship| ship.source_position.y == 0.0));
}

// Source: FUN_005ea030 / FUN_005ec400 slots; 0x005eb1a0 anchor step and
// turn; FUN_005b2f30 members.
#[test]
fn task_force_followers_rotate_and_move_with_the_retained_anchor() {
    let mut ships = (0..2)
        .map(|index| test_ship(64, index, true, true))
        .collect::<Vec<_>>();
    for ship in &mut ships {
        ship.weapon_arcs[0] = TacticalWeaponArc::new(1, 0, 0);
        ship.subsystem_capacity.engines = true;
        ship.sublight_engine_power = 4.0;
        ship.source_forward = TacticalWorldVector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        ship.source_desired_forward = ship.source_forward;
        ship.source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 10.0,
        });
    }
    BattleSession::assign_original_task_forces(&mut ships);
    assert_eq!(ships[1].source_position.z, 12.0);
    // The follower faces -x, toward the mover's place for it, which
    // stands on the side axis's negative side.
    ships[1].source_forward.x = -1.0;
    ships[1].source_desired_forward.x = -1.0;
    let goal = TacticalWorldPosition {
        x: 50.0,
        y: 0.0,
        z: 0.0,
    };
    ships[0].source_waypoint = Some(goal);
    let mut session = test_session(ships, Vec::new(), true);

    session.advance_original_tactical_movement(250.0);

    // The mover starts on the lead unit, facing the Alliance's +z with
    // its slot axis +x (FUN_005ea030), and turns toward its goal at the
    // shortest member rate, its slots with it (0x005eae40, FUN_005ead90).
    let mover = session.force_movers[0];
    let applied = original_capital_turn_rate(session.ships[0].maneuverability) * 0.25;
    assert!((mover.forward.x - applied.sin()).abs() < 1e-5, "{mover:?}");
    assert!((mover.lateral.x - applied.cos()).abs() < 1e-5, "{mover:?}");
    assert!((mover.lateral.z + applied.sin()).abs() < 1e-5, "{mover:?}");
    assert!((mover.anchor.x - 0.95 * 5.0 * 0.25).abs() < 1e-4);
    // The lead unit snaps onto the anchor; the follower steers toward
    // its slot, 12 along the turned axis's negative side, at its own
    // pace.
    assert!((session.ships[0].source_position.x - mover.anchor.x).abs() < 1e-4);
    let place = TacticalWorldPosition {
        x: mover.anchor.x - mover.lateral.x * 12.0,
        y: 0.0,
        z: mover.anchor.z - mover.lateral.z * 12.0,
    };
    let before = (place.x - 0.0).hypot(place.z - 12.0);
    let follower = session.ships[1].source_position;
    let after = (place.x - follower.x).hypot(place.z - follower.z);
    assert!(after < before, "{follower:?} {place:?}");
}

// Source: FUN_005ae460 fighter group split threshold.
#[test]
fn empty_fighter_group_splits_first_half_only_above_seven_members() {
    let mut ten = (0..10)
        .map(|index| {
            let mut fighter = test_fighter(3, true);
            fighter.fleet_squadron_index = index;
            fighter.fighter_group = 4;
            fighter
        })
        .collect::<Vec<_>>();
    BattleSession::assign_original_fighter_groups(&mut ten);
    assert!(ten[..5].iter().all(|fighter| fighter.fighter_group == 1));
    assert!(ten[5..].iter().all(|fighter| fighter.fighter_group == 0));

    let mut seven = (0..7)
        .map(|_| {
            let mut fighter = test_fighter(3, true);
            fighter.fighter_group = 4;
            fighter
        })
        .collect::<Vec<_>>();
    BattleSession::assign_original_fighter_groups(&mut seven);
    assert!(seven.iter().all(|fighter| fighter.fighter_group == 0));
}

// Source: FUN_005b05c0 / FUN_005b9c60 sublight conversion for fighters.
#[test]
fn fighter_roster_expands_each_strategic_squadron_with_exact_dat_stats() {
    let mut world = GameWorld::default();
    let class = world
        .fighter_classes
        .insert(rebellion_core::world::FighterClass {
            dat_id: DatId::new(1),
            name: "X-wing".into(),
            is_alliance: true,
            squadron_size: 12,
            shield_strength: 5,
            sub_light_engine: 11,
            maneuverability: 8,
            laser_cannon_fore: 5,
            laser_cannon_range: 12,
            torpedoes: 4,
            torpedoes_range: 7,
            ..rebellion_core::world::FighterClass::default()
        });
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        fighters: vec![rebellion_core::world::FighterEntry {
            class,
            count: 2,
            carrier: None,
            damage: None,
        }],
        is_alliance: true,
        ..Default::default()
    });
    let mut ships = Vec::new();
    let mut fighters = Vec::new();

    BattleSession::expand_fleet(&world, fleet, true, &mut ships, &mut fighters);

    assert!(ships.is_empty());
    assert_eq!(fighters.len(), 2);
    assert_eq!(fighters[0].fleet_squadron_index, 0);
    assert_eq!(fighters[1].fleet_squadron_index, 1);
    assert_eq!(fighters[0].squad_count, 12);
    assert_eq!(fighters[0].hull_current, 96.0);
    assert_eq!(fighters[0].shield, 5.0);
    assert_eq!(fighters[0].sublight_engine_power, 4.4);
    assert_eq!(fighters[0].weapon_arc.laser_cannons, 5);
    assert_eq!(fighters[0].weapon_ranges.laser_cannon, 12.0);
    assert_eq!(fighters[0].torpedo_strength, 4);
    assert_eq!(fighters[0].torpedo_range, 7.0);
}

// Source: FUN_005b9c00 / FUN_005b9c30 fighter hull scaling.
#[test]
fn fighter_hull_drives_weapon_shield_recharge_and_live_craft_count() {
    let mut fighter = test_fighter(1, true);
    assert!((fighter.weapon_hull_factor() - 45.012).abs() < 0.001);
    assert!((fighter.shield_recharge_rate() - 0.3).abs() < 0.001);

    fighter.weapon_arc.ready = false;
    fighter.weapon_arc.energy = 0.0;
    fighter.shield = 0.0;
    fighter.advance_original_recharge(1.0);
    assert!(fighter.weapon_arc.ready);
    assert_eq!(
        fighter.weapon_arc.energy,
        fighter.weapon_arc.energy_capacity
    );
    assert!((fighter.shield - 0.3).abs() < 0.001);

    fighter.hull_current = 79.9;
    fighter.refresh_craft_count();
    assert_eq!(fighter.squad_count, 10);
    fighter.hull_current = 0.0;
    fighter.refresh_craft_count();
    assert_eq!(fighter.squad_count, 0);
    assert!(!fighter.alive);
}

// Source: FUN_005b6530 (vtable +0xa8) offers a laser candidate of count
// * fighter modifier (+0x6c) * coefficient (+0x6b); FUN_005b3f10 fires
// that strength. FUN_005b9c00's hull * 3.751 / 8 is the weapon recharge
// (vtable +0x60), not a strength factor.
#[test]
fn a_fighter_group_fires_its_laser_count_times_the_coefficient_at_fighters() {
    let mut source = test_fighter(1, true);
    source.source_position.z = 0.0;
    let mut target = test_fighter(5, false);
    target.source_position.z = 6.0;
    let mut session = test_session(Vec::new(), vec![source, target], true);

    let mut shots = Vec::new();
    session.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Fighter(0),
        &mut shots,
    );
    let laser = shots
        .iter()
        .find(|shot| shot.kind == WeaponKind::LaserCannon)
        .expect("the group fires its lasers at the enemy group");
    let expected = 5.0 * f32::from_bits(0x3e88_8889);
    assert!(
        (laser.strength - expected).abs() < 1e-4,
        "{}",
        laser.strength
    );

    // The nearest enemy is a direct target (FUN_005b6530), whose
    // strength takes no hull scale: at half hull the shot is the same.
    session.fighters[0].hull_current = 48.0;
    session.fighters[0].weapon_arc.ready = true;
    session.fighters[0].weapon_arc.energy = session.fighters[0].weapon_arc.energy_capacity;
    let mut shots = Vec::new();
    session.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Fighter(0),
        &mut shots,
    );
    let laser = shots
        .iter()
        .find(|shot| shot.kind == WeaponKind::LaserCannon)
        .expect("the damaged group still fires");
    assert!(
        (laser.strength - expected).abs() < 1e-4,
        "{}",
        laser.strength
    );
}

// Source: FUN_005d2490 / FUN_005d24e0 fighter damage modifiers.
#[test]
fn fighter_damage_uses_float_shields_hull_and_maneuver_ratio() {
    let mut target = test_fighter(5, false);
    target.shield = 5.0;
    target.shield_max = 5.0;
    target.maneuverability = 7;
    let mut fighters = vec![target];

    assert_eq!(
        apply_original_fighter_damage(&mut fighters, 0, 9.0, 4.0),
        OriginalTacticalImpactStage::Hit
    );
    assert!((fighters[0].shield - 0.5).abs() < 0.001);
    assert_eq!(fighters[0].hull_current, 96.0);

    assert_eq!(
        apply_original_fighter_damage(&mut fighters, 0, 9.0, 4.0),
        OriginalTacticalImpactStage::Damage
    );
    assert_eq!(fighters[0].shield, 0.0);
    assert!((fighters[0].hull_current - 92.0).abs() < 0.001);
    assert_eq!(fighters[0].squad_count, 12);
}

// Source: FUN_005b23e0 through FUN_005b25d0 fighter battery sources.
#[test]
fn fighter_fore_battery_fires_source_families_and_torpedoes_at_capitals() {
    let mut fighter = test_fighter(2, true);
    fighter.weapon_arc = TacticalWeaponArc::new(8, 6, 0);
    fighter.weapon_ranges = TacticalWeaponRanges {
        laser_cannon: 20.0,
        ion_cannon: 18.0,
        turbolaser: 0.0,
    };
    fighter.torpedo_strength = 12;
    fighter.torpedo_range = 7.0;
    fighter.order = TacticalOrder::AttackCapitalShips;
    fighter.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
    fighter.source_position.z = 0.0;
    let mut target = test_ship(128, 0, false, true);
    target.source_position.z = 6.0;
    target.hull_current = 1000;
    target.hull_max = 1000;
    target.shield = 0;
    target.shield_max = 0;
    let mut session = test_session(vec![target], vec![fighter], true);

    let mut shots = Vec::new();
    session.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Fighter(0),
        &mut shots,
    );
    resolve_session_shots(&mut session, &shots);

    assert!(session.ships[0].hull_current < 1000);
    assert!(!session.fighters[0].weapon_arc.ready);
    assert_eq!(session.weapon_effects.len(), 3);
    assert_eq!(session.weapon_effects[0].kind, WeaponKind::IonCannon);
    assert_eq!(session.weapon_effects[1].kind, WeaponKind::LaserCannon);
    assert_eq!(session.weapon_effects[2].kind, WeaponKind::FighterAttack);
    assert_eq!(session.weapon_effects[0].source, usize::MAX);
    assert_eq!(
        session.weapon_effects[0].target_position,
        session.ships[0].source_position
    );
    assert_eq!(
        session
            .pending_audio_cues
            .iter()
            .map(|cue| cue.event)
            .collect::<Vec<_>>(),
        // The fire events run one turn before the hit events
        // (FUN_005b3f10: +0x87c + 1.0 and + 2.0).
        // At a capital ship (events 2, 0, and the torpedo's 1 from the
        // firer's +0xc8), through a shield of 0 (0x13, 0x12, 0x11).
        vec![
            TacticalAudioEvent::IonAtCapital,
            TacticalAudioEvent::LaserAtCapital,
            TacticalAudioEvent::Turbolaser,
            TacticalAudioEvent::IonHullHit,
            TacticalAudioEvent::LaserHullHit,
            TacticalAudioEvent::HullHit,
        ]
    );
}

// Source: FUN_005d2490 / FUN_005d24e0 fighter-vs-fighter modifiers.
#[test]
fn dogfight_ignores_ion_and_applies_target_maneuverability() {
    let mut source = test_fighter(2, true);
    source.weapon_arc = TacticalWeaponArc::new(8, 6, 0);
    source.weapon_ranges = TacticalWeaponRanges {
        laser_cannon: 20.0,
        ion_cannon: 18.0,
        turbolaser: 0.0,
    };
    source.maneuverability = 2;
    source.order = TacticalOrder::AttackFighters;
    source.attack_target = Some(TacticalAttackTarget::FighterGroup(1));
    let mut target = test_fighter(6, false);
    target.shield = 0.0;
    target.shield_max = 0.0;
    target.maneuverability = 8;
    target.weapon_arc = TacticalWeaponArc::default();
    target.source_position.z = 10.0;
    let mut session = test_session(Vec::new(), vec![source, target], true);

    let mut shots = Vec::new();
    session.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Fighter(0),
        &mut shots,
    );
    resolve_session_shots(&mut session, &shots);

    assert!(session.fighters[1].hull_current < 96.0);
    assert!(session.fighters[1].hull_current > 60.0);
    assert_eq!(session.weapon_effects.len(), 1);
    assert_eq!(session.weapon_effects[0].kind, WeaponKind::LaserCannon);
    assert_eq!(session.weapon_effects[0].target, usize::MAX);
}

// Source: FUN_005d0bb0 / FUN_005a8fc0 typed attack target.
#[test]
fn capital_attack_fighters_uses_typed_target_without_capital_fallback() {
    let mut source = test_ship(64, 0, true, true);
    source.weapon_arcs[0] = TacticalWeaponArc::new(300, 10, 0);
    source.weapon_ranges.laser_cannon = 100.0;
    source.weapon_ranges.ion_cannon = 100.0;
    source.maneuverability = 8;
    // FUN_005b0be0 gives an engineless ship 1.0; with engines its mode
    // contribution 1 plus its rating 8 makes 9.
    source.subsystem_capacity.engines = true;
    source.subsystem_condition.engines = 100;
    source.order = TacticalOrder::AttackFighters;
    source.attack_target = Some(TacticalAttackTarget::FighterGroup(0));
    let mut target = test_fighter(5, false);
    target.shield = 0.0;
    target.shield_max = 0.0;
    target.source_position.z = 10.0;
    let mut session = test_session(vec![source], vec![target], true);
    let mut shots = Vec::new();

    session.fire_unit_weapons(
        rebellion_core::tactical::TacticalUnitRef::Ship(0),
        &mut shots,
    );
    resolve_session_shots(&mut session, &shots);

    assert!(!session.fighters[0].alive);
    assert_eq!(session.weapon_effects.len(), 1);
    assert_eq!(session.weapon_effects[0].kind, WeaponKind::LaserCannon);
    assert!(!session.ships[0].weapon_arcs[0].ready);
    assert_eq!(
        session
            .pending_audio_cues
            .iter()
            .map(|cue| cue.event)
            .collect::<Vec<_>>(),
        // At an Imperial group (FUN_005ba0f0: side 1, event 4) with no
        // shield (FUN_005b5100: 0x12).
        vec![
            TacticalAudioEvent::LaserAtImperialFighter,
            TacticalAudioEvent::LaserHullHit
        ]
    );
}

#[test]
fn original_tactical_canvas_letterboxes_without_distorting_controls() {
    let native = TacticalCanvas::new(640.0, 480.0);
    assert_eq!((native.x, native.y, native.scale), (0.0, 0.0, 1.0));
    let wide = TacticalCanvas::new(1280.0, 800.0);
    assert_eq!(wide.scale, 5.0 / 3.0);
    assert_eq!(wide.x, (1280.0 - 640.0 * wide.scale) * 0.5);
    let (screen_x, screen_y) = wide.point(574.0, 317.0);
    let (logical_x, logical_y) = wide.logical_pointer(screen_x, screen_y);
    assert!((logical_x - 574.0).abs() < 0.001);
    assert!((logical_y - 317.0).abs() < 0.001);
}

#[test]
fn a_full_x_wing_squadron_reads_one_twelve_of_twelve() {
    // A0 TAC-02-C002 reads "X-wings: 1 12/12"; DAT 3 is the X-wing
    // (tactical ordinal 29, portrait 2030).
    let squadron = test_fighter(3, true);
    let count = tactical_fighter_count(std::iter::once(&squadron)).unwrap();
    assert_eq!(count.line(), "X-wings: 1 12/12");
}

#[test]
fn the_count_line_names_the_first_type_and_sums_every_squadron() {
    // Code at 0x5e1c80 tries types 0x1d..0x24 in order and names the
    // first present; FUN_005c6530/FUN_005c6570 sum all squadrons with
    // trunc(hull / 8), at least 1 (FUN_005ba330, FUN_005ba380).
    let a_wing = test_fighter(1, true);
    let mut x_wing = test_fighter(3, true);
    x_wing.hull_current = 50.0;
    let mut nearly_lost = test_fighter(3, true);
    nearly_lost.hull_current = 3.0;
    let group = [a_wing, x_wing, nearly_lost];
    let count = tactical_fighter_count(group.iter()).unwrap();
    assert_eq!(count.line(), "X-wings: 2 19/36");
    assert_eq!(tactical_fighter_count(std::iter::empty()), None);
}

#[test]
fn a_squadron_outside_the_eight_types_gives_no_count_line() {
    let mut unknown = test_fighter(3, true);
    unknown.tactical_resource = Some(TacticalFighterResource {
        tactical_ordinal: 37,
        first_side_resource_base: 0,
        opposing_side_resource_base: 0,
    });
    assert_eq!(tactical_fighter_count(std::iter::once(&unknown)), None);
}

// 0x005a0d10: a click on a ship of the other side (+0x168 against the
// player's +0x9ec) builds the enemy state 0x66bba8 (FUN_005a1110).
#[test]
fn a_click_on_an_enemy_ship_selects_it_alone() {
    let mut own = test_ship(1, 0, true, true);
    own.selected = true;
    let mut session = test_session(vec![own, test_ship(2, 0, false, true)], Vec::new(), true);
    session.selected_ship = Some(0);
    select_clicked_ship(&mut session, 1, false);
    assert!(!session.ships[0].selected);
    assert!(session.ships[1].selected);
    assert_eq!(session.selected_ship, Some(1));
}

// 0x005a1240 (0x66bba8 slot +0xc): a Ctrl click from the enemy state
// starts a new one-unit selection; it never adds to the enemy's.
#[test]
fn a_ctrl_click_from_an_enemy_selection_starts_a_new_one() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(2, 0, false, true)],
        Vec::new(),
        true,
    );
    select_clicked_ship(&mut session, 1, false);
    select_clicked_ship(&mut session, 0, true);
    assert!(session.ships[0].selected);
    assert!(!session.ships[1].selected);
    select_clicked_ship(&mut session, 1, true);
    assert!(!session.ships[0].selected);
    assert!(session.ships[1].selected);
}

// The player's own ships still gather with Ctrl (0x66bc98).
#[test]
fn a_ctrl_click_on_an_own_ship_adds_it_to_the_selection() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(3, 1, true, true)],
        Vec::new(),
        true,
    );
    select_clicked_ship(&mut session, 0, false);
    select_clicked_ship(&mut session, 1, true);
    assert!(session.ships[0].selected && session.ships[1].selected);
    select_clicked_ship(&mut session, 1, true);
    assert!(session.ships[0].selected && !session.ships[1].selected);
}

// FUN_005a1110 shows its unit like FUN_005a1550 (FUN_005c5f20,
// FUN_0051c9b0), with no group slot of the player's.
#[test]
fn an_enemy_selection_shows_its_ship_in_the_panel() {
    let mut enemy = test_ship(2, 0, false, true);
    enemy.name = "Star Destroyer".into();
    let mut session = test_session(vec![test_ship(1, 0, true, true), enemy], Vec::new(), true);
    select_clicked_ship(&mut session, 1, false);
    let hud = TacticalGroupHud::from_session(&session);
    assert_eq!(hud.selected_ship_name, Some("Star Destroyer"));
    assert_eq!(hud.selected_task_force, None);
    assert_eq!(
        hud.selected_ship_hull,
        Some((session.ships[1].hull_current, session.ships[1].hull_max))
    );
}

// Only a living, selected ship of the other side is an enemy selection.
#[test]
fn an_enemy_selection_is_a_living_selected_enemy_ship() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(2, 0, false, true)],
        Vec::new(),
        true,
    );
    session.ships[0].selected = true;
    session.selected_ship = Some(0);
    assert!(selected_enemy_ship(&session).is_none());
    session.selected_ship = Some(1);
    assert!(selected_enemy_ship(&session).is_none());
    session.ships[1].selected = true;
    assert!(selected_enemy_ship(&session).is_some());
    session.ships[1].alive = false;
    assert!(selected_enemy_ship(&session).is_none());
}

// FUN_005a1550 / FUN_005a1110 show their unit whatever its group: our
// grouped or ungrouped ship and an enemy ship all fill the panel.
#[test]
fn the_panel_shows_any_selected_ship() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(2, 0, false, true)],
        Vec::new(),
        true,
    );
    select_clicked_ship(&mut session, 0, false);
    assert!(TacticalGroupHud::from_session(&session)
        .panel_ship_name()
        .is_some());
    session.ships[0].task_force = TACTICAL_UNGROUPED_TASK_FORCE;
    let hud = TacticalGroupHud::from_session(&session);
    assert!(hud.panel_ship_name().is_some());
    assert_eq!(hud.selected_task_force, None);
    select_clicked_ship(&mut session, 1, false);
    assert!(TacticalGroupHud::from_session(&session)
        .panel_ship_name()
        .is_some());
}

// A0 VM 2026-10-27: the enemy's frame is red, ours yellow.
#[test]
fn an_enemy_selection_frame_is_red() {
    let session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(2, 0, false, true)],
        Vec::new(),
        true,
    );
    assert!(enemy_selection_frame_color(&session, &session.ships[0]).is_none());
    assert_eq!(
        enemy_selection_frame_color(&session, &session.ships[1]),
        Some(Color::new(1.0, 0.0, 0.0, 1.0))
    );
}

// FUN_005e4b40 with A0 VM 2026-10-27: the caption is the unit's group
// or "Task Force Unassigned"; an enemy reads "Orders Unavailable".
#[test]
fn the_panel_caption_and_orders_follow_the_unit_and_its_side() {
    let mut grouped = test_ship(1, 0, true, true);
    grouped.task_force = 2;
    grouped.order = TacticalOrder::HoldPosition;
    let mut alone = test_ship(3, 1, true, true);
    alone.task_force = TACTICAL_UNGROUPED_TASK_FORCE;
    let mut enemy = test_ship(2, 0, false, true);
    enemy.task_force = 0;
    let mut session = test_session(vec![grouped, alone, enemy], Vec::new(), true);
    select_clicked_ship(&mut session, 0, false);
    let hud = TacticalGroupHud::from_session(&session);
    assert_eq!(hud.panel_caption(), "Task Force # 3");
    assert_eq!(hud.panel_orders_line(), TacticalOrder::HoldPosition.label());
    assert!(!hud.selected_enemy);
    select_clicked_ship(&mut session, 1, false);
    assert_eq!(
        TacticalGroupHud::from_session(&session).panel_caption(),
        "Task Force Unassigned"
    );
    select_clicked_ship(&mut session, 2, false);
    let hud = TacticalGroupHud::from_session(&session);
    assert!(hud.selected_enemy);
    assert_eq!(hud.panel_caption(), "Task Force Unassigned");
    assert_eq!(hud.panel_orders_line(), "Orders Unavailable");
}

// FUN_005e4b40: a route reads "Navigating" (0xdd8e), "Disabled" (0xdd91)
// with no speed (FUN_005b17f0); a target "Attacking" (0xdd8d) and its
// name (A0 TAC-04-C002 "Navigating", A0 TAC-02/03 "Attacking Victory
// Destroyer 1", "Attacking Swift").
#[test]
fn the_panel_orders_line_names_the_route_or_the_target() {
    let mut ship = test_ship(1, 0, true, true);
    ship.order = TacticalOrder::AttackCapitalShips;
    ship.subsystem_capacity.engines = true;
    let mut enemy = test_ship(2, 0, false, true);
    enemy.name = "Victory Destroyer 1".to_owned();
    let mut session = test_session(vec![ship, enemy], vec![test_fighter(5, false)], true);
    session.fighters[0].name = "Red Group".to_owned();
    select_clicked_ship(&mut session, 0, false);
    let line =
        |session: &BattleSession| TacticalGroupHud::from_session(session).panel_orders_line();
    assert_eq!(line(&session), "Attack Capital Ships");
    session.ships[0].manual_targets = vec![TacticalAttackTarget::FighterGroup(0)];
    assert_eq!(line(&session), "Attacking Red Group");
    session.ships[0].attack_target = Some(TacticalAttackTarget::CapitalShip(1));
    assert_eq!(line(&session), "Attacking Victory Destroyer 1");
    session.ships[0].navigation_route = vec![at(1.0, 0.0, 0.0)];
    assert_eq!(line(&session), "Navigating");
    session.ships[0].subsystem_capacity.engines = false;
    assert_eq!(line(&session), "Disabled");
}

// FUN_005e4b40 / FUN_005e16a0: order code 1 (the mover's +0x68) keeps
// "Escort" before the escorted ship's name; a squadron's line follows
// its route and target as a ship's does, with no speed test.
#[test]
fn an_escort_names_its_ship_and_a_squadron_its_route_or_target() {
    let mut escort = test_ship(1, 0, true, true);
    escort.order = TacticalOrder::Escort;
    escort.escort_target = Some(1);
    let mut carrier = test_ship(3, 0, true, true);
    carrier.name = "Nebulon-B Frigate 1".to_owned();
    let mut enemy = test_ship(2, 0, false, true);
    enemy.name = "Swift".to_owned();
    let mut session = test_session(
        vec![escort, carrier, enemy],
        vec![test_fighter(5, true)],
        true,
    );
    select_clicked_ship(&mut session, 0, false);
    assert_eq!(
        TacticalGroupHud::from_session(&session).panel_orders_line(),
        "Escort Nebulon-B Frigate 1"
    );
    fn fighter(session: &BattleSession) -> Option<TacticalShipActivity<'_>> {
        TacticalShipActivity::of_fighter(session, 0)
    }
    assert_eq!(fighter(&session), None);
    session.fighters[0].attack_target = Some(TacticalAttackTarget::CapitalShip(2));
    assert_eq!(
        fighter(&session).map(TacticalShipActivity::line).as_deref(),
        Some("Attacking Swift")
    );
    session.fighters[0].order = TacticalOrder::Escort;
    session.fighters[0].escort_target = Some(1);
    assert_eq!(
        fighter(&session).map(TacticalShipActivity::line).as_deref(),
        Some("Escort Nebulon-B Frigate 1")
    );
    session.fighters[0].navigation_route = vec![at(1.0, 0.0, 0.0)];
    session.fighters[0].sublight_engine_power = 0.0;
    assert_eq!(fighter(&session), Some(TacticalShipActivity::Navigating));
}

// FUN_00597610 / FUN_005975d0: the icon of a unit's name id less 0x2000,
// its mini less 0x4000. A0 TAC-03-C012: the Alliance Army Regiment
// (mini 17473) is 2301; A0 VM coruscant-676/05: the Stormtrooper
// Regiment (17536) 2317 and the character with icon 2240 (key 0xa97).
#[test]
fn a_units_contents_icon_follows_its_mini() {
    assert_eq!(tactical_content_icon(17_473), Some(2301));
    assert_eq!(tactical_content_icon(17_536), Some(2317));
    assert_eq!(tactical_content_icon(0x4000 + 0xa97), Some(2240));
    assert_eq!(tactical_content_icon(0x4000 + 0x840), Some(2234));
    assert_eq!(tactical_content_icon(0x4000 + 0x445), None);
    assert_eq!(tactical_content_icon(0x3fff), None);
    assert_eq!(TACTICAL_CONTENT_ICONS.len(), 79);
}

// FUN_0040b0e0: a ship's characters, then its special forces, then its
// regiments (A0 VM coruscant-676/05: the character before the
// Stormtroopers); a killed character is not aboard.
#[test]
fn a_ships_contents_list_its_characters_then_forces_then_regiments() {
    use rebellion_core::world::{Character, SpecialForceUnit, TroopUnit};
    let mut world = GameWorld::default();
    let troop = world.troops.insert(TroopUnit {
        class_dat_id: DatId::new(0x1000_0002),
        is_alliance: true,
        regiment_strength: 1,
    });
    let force = world.special_forces.insert(SpecialForceUnit {
        class_dat_id: DatId::new(0x3c00_0001),
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
    });
    let mothma = world.characters.insert(Character {
        name: "Mon Mothma".into(),
        dat_id: DatId::new(0x240),
        is_major: true,
        ..Default::default()
    });
    let dead = world.characters.insert(Character {
        name: "Gone".into(),
        dat_id: DatId::new(0x241),
        is_major: true,
        is_killed: true,
        ..Default::default()
    });
    let cargo = crate::fleet_window::ShipCargo {
        characters: vec![dead, mothma],
        special_forces: vec![force],
        regiments: vec![troop],
        squadrons: 0,
        incoming_squadrons: Vec::new(),
    };
    let contents = ship_contents_of(&world, &cargo);
    assert_eq!(
        contents
            .iter()
            .map(|content| (content.label.as_str(), content.resource_id))
            .collect::<Vec<_>>(),
        [
            ("Mon Mothma", 0x4840),
            ("Guerrillas", 17_728),
            ("Alliance Army Regiment", 17_473),
        ]
    );
    assert_eq!(
        contents
            .iter()
            .map(|content| tactical_content_icon(content.resource_id))
            .collect::<Vec<_>>(),
        [Some(2234), Some(2308), Some(2301)]
    );
}

// A0 VM coruscant-676/05: three icons fit the 124-wide list, four
// overflow it, and the 84-wide track's thumb is then 81 wide at 515; a
// click moves a cell, clamped to the 4 pixels the list overflows.
#[test]
fn the_contents_bar_shows_once_the_icons_overflow_the_list() {
    assert_eq!(contents_thumb(3, 0.0), None);
    assert_eq!(contents_thumb(4, 0.0), Some((515.0, 81.0)));
    assert_eq!(contents_thumb(4, 4.0), Some((518.0, 81.0)));
    assert_eq!(contents_thumb(8, 0.0), Some((515.0, 40.0)));
    assert_eq!(contents_thumb(8, 132.0), Some((559.0, 40.0)));
    assert_eq!(scrolled_contents(4, 0.0, true), 4.0);
    assert_eq!(scrolled_contents(4, 4.0, false), 0.0);
    assert_eq!(scrolled_contents(8, 32.0, true), 64.0);
    assert_eq!(scrolled_contents(8, 0.0, false), 0.0);
    assert_eq!(contents_scroll_of((Some(1), 32.0), Some(1)), 32.0);
    assert_eq!(contents_scroll_of((Some(1), 32.0), Some(2)), 0.0);
    assert_eq!(contents_scroll_of((None, 32.0), None), 0.0);
}

// FUN_005e16a0: the fighter panel's line names the squadron's target.
#[test]
fn the_fighter_panel_line_names_the_squadrons_target() {
    let mut enemy = test_ship(2, 0, false, true);
    enemy.name = "Swift".to_owned();
    let mut session = test_session(vec![enemy], vec![test_fighter(5, true)], true);
    session.fighters[0].selected = true;
    session.fighters[0].fighter_group = 0;
    session.fighters[0].order = TacticalOrder::AttackCapitalShips;
    session.selected_fighter_group = Some(0);
    let line =
        |session: &BattleSession| TacticalGroupHud::from_session(session).fighter_orders_line();
    assert_eq!(line(&session), "Attack Capital Ships");
    session.fighters[0].attack_target = Some(TacticalAttackTarget::CapitalShip(0));
    assert_eq!(line(&session), "Attacking Swift");
}

// Each battle-window text rasters at its own size on the canvas.
#[test]
fn a_battle_window_text_rasters_at_its_size() {
    let canvas = TacticalCanvas {
        x: 0.0,
        y: 0.0,
        scale: 2.0,
    };
    let params = battle_window_text_params(canvas, BATTLE_WINDOW_FONT_6, RED);
    assert_eq!((params.font_size, params.font_scale), (33, 1.0));
    assert_eq!(params.font_scale_aspect, TACTICAL_STATUS_ASPECT);
    assert_eq!(params.color, RED);
}

// FUN_0051c9b0: the panel shows the clicked unit of a selection across
// two forces.
#[test]
fn the_panel_shows_the_clicked_ship_of_a_mixed_selection() {
    let mut first = test_ship(1, 0, true, true);
    first.name = "First".into();
    first.task_force = 0;
    let mut second = test_ship(3, 1, true, true);
    second.name = "Second".into();
    second.task_force = 1;
    let mut session = test_session(vec![first, second], Vec::new(), true);
    select_clicked_ship(&mut session, 0, false);
    select_clicked_ship(&mut session, 1, true);
    let hud = TacticalGroupHud::from_session(&session);
    assert_eq!(hud.selected_task_force, None);
    assert_eq!(hud.panel_ship_name(), Some("Second"));
    // Unselected again, the first selected ship stands in.
    session.ships[1].selected = false;
    assert_eq!(
        TacticalGroupHud::from_session(&session).panel_ship_name(),
        Some("First")
    );
}

// 0x66bba8's order and tactic slots +0x20, +0x24 and +0x28 are the
// empty 0x0048b440: an enemy selection takes no order.
#[test]
fn an_enemy_selection_takes_no_order() {
    let mut session = test_session(
        vec![
            test_ship(1, 0, true, true),
            test_ship(2, 0, false, true),
            test_ship(4, 1, false, true),
        ],
        Vec::new(),
        true,
    );
    select_clicked_ship(&mut session, 1, false);
    assert_eq!(assign_selected_escort(&mut session, 0), (0, 0));
    assert_eq!(assign_selected_escort(&mut session, 2), (0, 0));
    assert_eq!(session.ships[1].order, TacticalOrder::None);
    assert_eq!(session.ships[1].escort_target, None);
}

fn at(x: f32, y: f32, z: f32) -> TacticalWorldPosition {
    TacticalWorldPosition { x, y, z }
}

// FUN_0059b730 key 0x20 -> 0x66bce8 slot +0x1c (0x005a21e0): the
// nearest enemy capital ship to the selected ship (FUN_005aa970)
// becomes its target, as a right click without Ctrl.
#[test]
fn space_targets_the_enemy_capital_nearest_the_selected_ship() {
    let mut own = test_ship(1, 0, true, true);
    own.selected = true;
    own.manual_targets
        .push(TacticalAttackTarget::CapitalShip(1));
    let mut far = test_ship(2, 0, false, true);
    far.source_position = at(0.0, 0.0, 9.0);
    let mut near = test_ship(3, 1, false, true);
    near.source_position = at(0.0, -4.0, 0.0);
    let mut session = test_session(vec![own, far, near], Vec::new(), true);
    assert!(matches!(
        attack_nearest_enemy(&mut session),
        Some(TacticalFocusAssignment::Assigned { .. })
    ));
    assert_eq!(
        session.ships[0].manual_targets,
        vec![TacticalAttackTarget::CapitalShip(2)]
    );
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(2))
    );
}

// 0x005a3250 / 0x005a4180: several ships search from their centroid
// (the sum times 1 / count, DAT_0066bb54).
#[test]
fn space_searches_from_the_selected_ships_centroid() {
    let mut left = test_ship(1, 0, true, true);
    left.selected = true;
    left.source_position = at(-10.0, 0.0, 0.0);
    let mut right = test_ship(4, 0, true, true);
    right.selected = true;
    right.source_position = at(10.0, 0.0, 0.0);
    // Nearest to the left ship, but the middle one is nearest the centroid.
    let mut by_left = test_ship(2, 0, false, true);
    by_left.source_position = at(-11.0, 0.0, 0.0);
    let mut middle = test_ship(3, 1, false, true);
    middle.source_position = at(0.0, 0.0, 5.0);
    let mut session = test_session(vec![left, right, by_left, middle], Vec::new(), true);
    attack_nearest_enemy(&mut session);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(3))
    );
    assert_eq!(
        session.ships[1].attack_target,
        Some(TacticalAttackTarget::CapitalShip(3))
    );
}

// 0x005a21e0's second search (type 1): with no enemy capital ship in
// the fight, the nearest enemy squadron.
#[test]
fn space_falls_back_to_the_nearest_enemy_squadron() {
    let mut own = test_ship(1, 0, true, true);
    own.selected = true;
    let dead = test_ship(2, 0, false, false);
    let mut far = test_fighter(5, false);
    far.source_position = at(30.0, 0.0, 0.0);
    let mut near = test_fighter(6, false);
    near.source_position = at(3.0, 0.0, 0.0);
    let mut session = test_session(vec![own, dead], vec![far, near], true);
    attack_nearest_enemy(&mut session);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::FighterGroup(1))
    );
}

// An enemy selection's slot +0x1c is the empty 0x006158b0, and so is
// having no selection.
#[test]
fn space_with_an_enemy_or_no_selection_does_nothing() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(2, 0, false, true)],
        Vec::new(),
        true,
    );
    assert!(attack_nearest_enemy(&mut session).is_none());
    select_clicked_ship(&mut session, 1, false);
    assert!(attack_nearest_enemy(&mut session).is_none());
    assert!(session
        .ships
        .iter()
        .all(|ship| ship.attack_target.is_none()));
}

// FUN_005aa970 measures the squared distance over all three axes.
#[test]
fn the_nearest_unit_uses_the_squared_distance_over_three_axes() {
    let mut above = test_ship(2, 0, false, true);
    above.source_position = at(0.0, 3.0, 0.0); // 9
    let mut across = test_ship(3, 1, false, true);
    across.source_position = at(2.0, 0.0, 2.0); // 8
    let mut ahead = test_ship(4, 2, false, true);
    ahead.source_position = at(0.0, 0.0, 2.9); // 8.41
    let session = test_session(vec![above, across, ahead], Vec::new(), true);
    assert_eq!(
        session.nearest_unit_of_side(at(0.0, 0.0, 0.0), false, false),
        Some(TacticalUnitRef::Ship(1))
    );
}

// FUN_005aa970 keeps the first unit on a tie and takes only the given
// side's units in the fight.
#[test]
fn the_nearest_unit_keeps_the_first_of_a_tie_and_skips_other_units() {
    let mut own = test_ship(1, 0, true, true);
    own.source_position = at(0.5, 0.0, 0.0);
    let mut lost = test_ship(2, 0, false, false);
    lost.source_position = at(0.0, 0.0, 0.0);
    let mut first = test_ship(3, 1, false, true);
    first.source_position = at(1.0, 0.0, 0.0);
    let mut second = test_ship(4, 2, false, true);
    second.source_position = at(-1.0, 0.0, 0.0);
    let mut own_fighter = test_fighter(5, true);
    own_fighter.source_position = at(0.1, 0.0, 0.0);
    let mut enemy_fighter = test_fighter(6, false);
    enemy_fighter.source_position = at(5.0, 0.0, 0.0);
    let session = test_session(
        vec![own, lost, first, second],
        vec![own_fighter, enemy_fighter],
        true,
    );
    let origin = at(0.0, 0.0, 0.0);
    assert_eq!(
        session.nearest_unit_of_side(origin, false, false),
        Some(TacticalUnitRef::Ship(2))
    );
    assert_eq!(
        session.nearest_unit_of_side(origin, false, true),
        Some(TacticalUnitRef::Fighter(1))
    );
}

// 0x005a3250: the centroid is the sum times 1 / count on every axis.
#[test]
fn space_aims_from_the_exact_centroid_on_every_axis() {
    let mut first = test_ship(1, 0, true, true);
    first.selected = true;
    first.source_position = at(2.0, 4.0, 6.0);
    let mut second = test_ship(5, 0, true, true);
    second.selected = true;
    second.source_position = at(4.0, 8.0, 10.0);
    let mut ships = vec![first, second];
    // The centroid (3, 6, 8) and decoys where a wrong sum or scale lands.
    for (roster, position) in [
        (0, at(3.0, 6.0, 8.0)),
        (1, at(6.0, 12.0, 16.0)),
        (2, at(-3.0, 6.0, 8.0)),
        (3, at(3.0, -6.0, 8.0)),
        (4, at(3.0, 6.0, -8.0)),
        (5, at(0.0, 6.0, 8.0)),
        (6, at(3.0, 0.0, 8.0)),
        (7, at(3.0, 6.0, 0.0)),
        (8, at(12.0, 24.0, 32.0)),
        // scale + sum on one axis (0.5 + 12, 0.5 + 16)
        (9, at(3.0, 12.5, 8.0)),
        (10, at(3.0, 6.0, 16.5)),
        (11, at(6.5, 6.0, 8.0)),
    ] {
        let mut enemy = test_ship(10 + u32::try_from(roster).unwrap(), roster, false, true);
        enemy.source_position = position;
        ships.push(enemy);
    }
    let mut session = test_session(ships, Vec::new(), true);
    attack_nearest_enemy(&mut session);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(2))
    );
}

// F1 (FUN_0059b730 case 0x70) runs FUN_005c2990 mode 0xb: the chase
// camera (class 0x66d478, 0x005d9b10) behind the task force's lead
// hull, facing its heading. Leaving the follow (a camera control,
// commands 7 and 10) returns to the free camera, which F1 left as it was.
#[test]
fn following_a_task_force_runs_the_chase_camera_behind_its_lead_hull() {
    // Ships F2 must pass over: dead, retreating, the enemy's, and one
    // in task force 1; then the lead and a second member of task force 2.
    let mut ships = Vec::new();
    for (dat_id, is_alliance, alive, retreating, task_force) in [
        (1, true, false, false, 1),
        (2, true, true, true, 1),
        (3, false, true, false, 1),
        (4, true, true, false, 0),
        (5, true, true, false, 1),
        (6, true, true, false, 1),
    ] {
        let mut ship = test_ship(dat_id, 0, is_alliance, alive);
        ship.retreating = retreating;
        ship.task_force = task_force;
        ship.source_position = at(dat_id as f32, 0.0, -53.0);
        ship.x = 100.0 * dat_id as f32;
        ship.y = 10.0 * dat_id as f32;
        ships.push(ship);
    }
    ships[4].source_position = at(3.0, 4.0, -53.0);
    ships[4].source_forward = TacticalWorldVector {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    let mut state = TacticalState::default();
    state.session = Some(test_session(ships, Vec::new(), true));
    let free = crate::tactical_assets::OriginalTacticalCamera::new(false, 100.0);
    state.asset_renderer.restore_camera_state(free);
    state.camera_follow = Some(TacticalCameraFollow::TaskForce(1));
    assert!(update_tactical_camera_follow(&mut state));
    let chase = state
        .asset_renderer
        .chase_view()
        .expect("F2 runs the chase camera");
    // No box is loaded here, so e is 0: the eye is the lead's position.
    assert_eq!(chase.eye, at(3.0, 4.0, -53.0).rendered());
    assert_eq!(chase.direction, macroquad::math::vec3(0.0, 0.0, -1.0));
    assert_eq!(chase.object_id, 10_002);
    assert_eq!(state.asset_renderer.camera_state(), Some(free));
    // port: the 2D fallback view centres the group's members (5 and 6).
    assert_eq!(state.camera_x, 550.0 - ARENA_WIDTH * 0.5);
    assert_eq!(state.camera_y, 55.0 - ARENA_HEIGHT * 0.5);
    // A fighter group (F9-F12) keeps the look-at at its members' centre:
    // no chase. The others are dead, empty, the enemy's or in group 2.
    for (dat_id, is_alliance, alive, squad_count, group, z) in [
        (7, true, false, 12, 1, 30.0),
        (8, true, true, 0, 1, 30.0),
        (9, false, true, 12, 1, 30.0),
        (10, true, true, 12, 2, 30.0),
        (11, true, true, 12, 1, 10.0),
        (12, true, true, 12, 1, 20.0),
    ] {
        let mut squadron = test_fighter(dat_id, is_alliance);
        squadron.alive = alive;
        squadron.squad_count = squad_count;
        squadron.fighter_group = group;
        squadron.source_position = at(z, z, z);
        squadron.x = 10.0 * z;
        squadron.y = z;
        state.session.as_mut().unwrap().fighters.push(squadron);
    }
    state.camera_follow = Some(TacticalCameraFollow::FighterGroup(1));
    assert!(update_tactical_camera_follow(&mut state));
    assert_eq!(state.asset_renderer.chase_view(), None);
    assert_eq!(state.asset_renderer.target_object_id(), Some(10_102));
    assert_eq!(
        state
            .asset_renderer
            .camera_state()
            .map(|camera| camera.target_point()),
        Some(at(15.0, 15.0, 15.0).rendered())
    );
    assert_eq!(state.camera_x, 150.0 - ARENA_WIDTH * 0.5);
    assert_eq!(state.camera_y, 15.0 - ARENA_HEIGHT * 0.5);
    state.camera_follow = Some(TacticalCameraFollow::TaskForce(1));
    assert!(update_tactical_camera_follow(&mut state));
    state.camera_follow = None;
    assert!(!update_tactical_camera_follow(&mut state));
    assert_eq!(state.asset_renderer.chase_view(), None);
}

// F1..F8 follow task forces, F9..F12 fighter groups.
#[test]
fn f_keys_follow_task_forces_then_fighter_groups() {
    let mut ship = test_ship(1, 0, true, true);
    ship.task_force = 2;
    let mut squadron = test_fighter(5, true);
    squadron.fighter_group = 1;
    let session = test_session(vec![ship], vec![squadron], true);
    assert_eq!(
        camera_follow_for_key(&session, 2),
        Some(TacticalCameraFollow::TaskForce(2))
    );
    assert_eq!(camera_follow_for_key(&session, 1), None);
    assert_eq!(
        camera_follow_for_key(&session, 9),
        Some(TacticalCameraFollow::FighterGroup(1))
    );
    assert_eq!(camera_follow_for_key(&session, 10), None);
    // F9 is fighter group 1, not task force 1.
    let mut first = test_ship(1, 0, true, true);
    first.task_force = 0;
    let session = test_session(vec![first], vec![test_fighter(5, true)], true);
    assert_eq!(
        camera_follow_for_key(&session, 8),
        Some(TacticalCameraFollow::FighterGroup(0))
    );
}

// FUN_0059b130: the HUD buttons the keys press.
#[test]
fn the_hud_buttons_pause_zoom_and_toggle() {
    let mut state = TacticalState::default();
    assert_eq!(
        activate_tactical_hud_control(&mut state, TacticalHudControl::Pause),
        Some(TacticalAction::TogglePause)
    );
    state.zoom = 1.0;
    assert_eq!(
        activate_tactical_hud_control(&mut state, TacticalHudControl::ZoomIn),
        None
    );
    assert!((state.zoom - 1.25).abs() < 1e-6);
    state.zoom = 1.0;
    activate_tactical_hud_control(&mut state, TacticalHudControl::ZoomOut);
    assert!((state.zoom - 0.8).abs() < 1e-6);
    let (alliance, empire) = (state.highlight_alliance, state.highlight_empire);
    activate_tactical_hud_control(&mut state, TacticalHudControl::HighlightAlliance);
    activate_tactical_hud_control(&mut state, TacticalHudControl::HighlightEmpire);
    assert_eq!(
        (state.highlight_alliance, state.highlight_empire),
        (!alliance, !empire)
    );
    let visible = state.navigation_sets_visible[0];
    activate_tactical_hud_control(&mut state, TacticalHudControl::NavigationSet(0));
    assert_eq!(state.navigation_sets_visible[0], !visible);
}

// 0x66bbf8 slot +0xc (0x005a4bd0): a right click on an enemy unit gives
// the selected fighter group order 0, that target and its attack voice.
#[test]
fn a_fighter_group_takes_an_enemy_unit_as_its_target() {
    let mut escorting = test_fighter(5, true);
    escorting.order = TacticalOrder::Escort;
    escorting.escort_target = Some(0);
    let mut other_group = test_fighter(6, true);
    other_group.fighter_group = 1;
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(2, 0, false, true)],
        vec![escorting, other_group, test_fighter(7, false)],
        true,
    );
    assert!(select_fighter_group(&mut session, 0));
    let cues = session.pending_voice_cues.len();
    assert_eq!(
        assign_selected_fighter_group_target(
            &mut session,
            TacticalAttackTarget::FighterGroup(2),
            false
        ),
        1
    );
    let fighter = &session.fighters[0];
    assert_eq!(
        fighter.attack_target,
        Some(TacticalAttackTarget::FighterGroup(2))
    );
    assert_eq!(fighter.order, TacticalOrder::None);
    assert_eq!(fighter.escort_target, None);
    assert_eq!(session.fighters[1].attack_target, None);
    assert_eq!(session.pending_voice_cues.len(), cues + 1);
}

// FUN_005ca310 / FUN_005a8e80: with Ctrl, a target the group holds is
// dropped; a new one replaces it (port: one target a squadron). 0x005a4bd0
// voices the new one only while the force's targets (+0x98) and route
// (+0x80) are both empty (each list's slot is its first item,
// FUN_005f5060).
#[test]
fn a_ctrl_right_click_on_a_held_target_drops_it() {
    let mut session = test_session(
        vec![
            test_ship(1, 0, true, true),
            test_ship(2, 0, false, true),
            test_ship(3, 1, false, true),
        ],
        vec![test_fighter(5, true)],
        true,
    );
    assert!(select_fighter_group(&mut session, 0));
    let first = TacticalAttackTarget::CapitalShip(1);
    assign_selected_fighter_group_target(&mut session, first, false);
    let cues = session.pending_voice_cues.len();
    assign_selected_fighter_group_target(&mut session, first, true);
    assert_eq!(session.fighters[0].attack_target, None);
    assert_eq!(session.pending_voice_cues.len(), cues);
    assign_selected_fighter_group_target(&mut session, TacticalAttackTarget::CapitalShip(2), true);
    assert_eq!(
        session.fighters[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(2))
    );
    assert_eq!(session.pending_voice_cues.len(), cues + 1);
    // Holding a target, a Ctrl click on another is silent.
    assign_selected_fighter_group_target(&mut session, TacticalAttackTarget::CapitalShip(1), true);
    assert_eq!(session.pending_voice_cues.len(), cues + 1);
    // With a route, even a plain click is silent.
    session.fighters[0].navigation_route.push(at(1.0, 0.0, 0.0));
    assign_selected_fighter_group_target(&mut session, TacticalAttackTarget::CapitalShip(2), false);
    assert_eq!(session.pending_voice_cues.len(), cues + 1);
}

// A right click takes the lowest-indexed enemy squadron it hits that
// still flies.
#[test]
fn a_right_click_hits_the_first_flying_enemy_squadron() {
    let mut lost = test_fighter(5, false);
    lost.alive = false;
    let mut spent = test_fighter(6, false);
    spent.squad_count = 0;
    let session = test_session(
        Vec::new(),
        vec![
            lost,
            spent,
            test_fighter(7, true),
            test_fighter(8, false),
            test_fighter(9, false),
        ],
        true,
    );
    let hits = [(4, true), (0, true), (1, true), (2, true), (3, true)];
    assert_eq!(enemy_squadron_hit(&session, hits.into_iter()), Some(3));
    assert_eq!(
        enemy_squadron_hit(&session, [(3, false), (4, true)].into_iter()),
        Some(4)
    );
    assert_eq!(enemy_squadron_hit(&session, [(9, true)].into_iter()), None);
}

// Only the group's flying squadrons take the target and give Space its
// centroid.
#[test]
fn a_fighter_group_order_skips_its_lost_squadrons_and_other_groups() {
    let mut member = test_fighter(5, true);
    member.source_position = at(0.0, 0.0, 0.0);
    let mut lost = test_fighter(6, true);
    lost.alive = false;
    lost.source_position = at(100.0, 0.0, 0.0);
    let mut spent = test_fighter(7, true);
    spent.squad_count = 0;
    spent.source_position = at(100.0, 0.0, 0.0);
    let mut other = test_fighter(8, true);
    other.fighter_group = 1;
    other.source_position = at(100.0, 0.0, 0.0);
    let mut enemy_member = test_fighter(9, false);
    enemy_member.source_position = at(100.0, 0.0, 0.0);
    let mut near = test_ship(2, 0, false, true);
    near.source_position = at(1.0, 0.0, 0.0);
    let mut far = test_ship(3, 1, false, true);
    far.source_position = at(60.0, 0.0, 0.0);
    let mut session = test_session(
        vec![near, far],
        vec![member, lost, spent, other, enemy_member],
        true,
    );
    assert!(select_fighter_group(&mut session, 0));
    assert!(attack_nearest_enemy(&mut session).is_some());
    assert_eq!(
        session.fighters[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(0))
    );
    for index in 1..5 {
        assert_eq!(session.fighters[index].attack_target, None, "{index}");
    }
}

// Without a fighter group selected, nothing changes.
#[test]
fn a_fighter_target_needs_a_selected_group() {
    let mut session = test_session(
        vec![test_ship(2, 0, false, true)],
        vec![test_fighter(5, true)],
        true,
    );
    assert_eq!(
        assign_selected_fighter_group_target(
            &mut session,
            TacticalAttackTarget::CapitalShip(0),
            false
        ),
        0
    );
    assert_eq!(session.fighters[0].attack_target, None);
}

// 0x005a50d0: a fighter group's Space searches from its squadrons'
// centroid.
#[test]
fn space_for_a_fighter_group_targets_from_its_squadrons_centroid() {
    let mut left = test_fighter(5, true);
    left.source_position = at(-10.0, 0.0, 0.0);
    let mut right = test_fighter(6, true);
    right.source_position = at(10.0, 0.0, 0.0);
    let mut by_left = test_ship(2, 0, false, true);
    by_left.source_position = at(-11.0, 0.0, 0.0);
    let mut middle = test_ship(3, 1, false, true);
    middle.source_position = at(0.0, 0.0, 5.0);
    let mut session = test_session(vec![by_left, middle], vec![left, right], true);
    assert!(select_fighter_group(&mut session, 0));
    assert!(attack_nearest_enemy(&mut session).is_some());
    assert_eq!(
        session.fighters[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(1))
    );
    assert_eq!(
        session.fighters[1].attack_target,
        Some(TacticalAttackTarget::CapitalShip(1))
    );
}

// 0x66bbf8 slot +0x10 (0x005a4e50): a navigation point becomes the
// group's whole route with order 0 and the group's line (0x2a plus
// the slot); with Ctrl a new point is added and a held one dropped, the
// line playing only while the route (+0x80) was empty.
#[test]
fn a_fighter_group_takes_navigation_points_as_its_route() {
    let mut escorting = test_fighter(5, true);
    escorting.order = TacticalOrder::Escort;
    escorting.escort_target = Some(0);
    let mut session = test_session(Vec::new(), vec![escorting, test_fighter(6, true)], true);
    assert!(select_fighter_group(&mut session, 0));
    let (a, b) = (at(1.0, 0.0, 0.0), at(2.0, 0.0, 0.0));
    let faction = session.player_voice_faction();
    let cues = session.pending_voice_cues.len();
    assert_eq!(
        assign_selected_fighter_group_route(&mut session, a, false),
        2
    );
    assert_eq!(session.fighters[0].navigation_route, vec![a]);
    assert_eq!(session.fighters[1].navigation_route, vec![a]);
    assert_eq!(session.fighters[0].order, TacticalOrder::None);
    assert_eq!(session.fighters[0].escort_target, None);
    assert_eq!(
        session.pending_voice_cues[cues],
        TacticalVoiceCue::fighter_maneuver(faction, 0)
    );
    assign_selected_fighter_group_route(&mut session, b, true);
    assert_eq!(session.fighters[0].navigation_route, vec![a, b]);
    assert_eq!(session.pending_voice_cues.len(), cues + 1);
    assign_selected_fighter_group_route(&mut session, a, true);
    assert_eq!(session.fighters[1].navigation_route, vec![b]);
    assert_eq!(session.pending_voice_cues.len(), cues + 1);
    assign_selected_fighter_group_route(&mut session, a, false);
    assert_eq!(session.fighters[1].navigation_route, vec![a]);
    assert_eq!(session.pending_voice_cues.len(), cues + 2);
    // From an empty route a Ctrl point plays the line.
    for fighter in &mut session.fighters {
        fighter.navigation_route.clear();
    }
    assign_selected_fighter_group_route(&mut session, b, true);
    assert_eq!(session.pending_voice_cues.len(), cues + 3);
}

// FUN_005ca330 / FUN_005a8ed0 / FUN_005a8f20.
#[test]
fn a_ctrl_route_click_drops_a_held_point_and_adds_another() {
    let (a, b) = (at(1.0, 0.0, 0.0), at(2.0, 0.0, 0.0));
    let mut route = vec![a];
    assert!(!toggle_route_point(&mut route, b));
    assert_eq!(route, vec![a, b]);
    assert!(toggle_route_point(&mut route, a));
    assert_eq!(route, vec![b]);
}

// The display marks the routes of our selected ships and squadrons only.
#[test]
fn the_display_marks_only_our_selected_routes() {
    let (a, b, c, d) = (
        at(1.0, 0.0, 0.0),
        at(2.0, 0.0, 0.0),
        at(3.0, 0.0, 0.0),
        at(4.0, 0.0, 0.0),
    );
    let mut ship = test_ship(1, 0, true, true);
    ship.selected = true;
    ship.navigation_route = vec![a];
    let mut idle = test_ship(3, 1, true, true);
    idle.navigation_route = vec![b];
    let mut enemy = test_ship(2, 0, false, true);
    enemy.selected = true;
    enemy.navigation_route = vec![b];
    let mut squadron = test_fighter(5, true);
    squadron.selected = true;
    squadron.navigation_route = vec![c];
    let mut enemy_squadron = test_fighter(6, false);
    enemy_squadron.selected = true;
    enemy_squadron.navigation_route = vec![d];
    let mut resting = test_fighter(7, true);
    resting.navigation_route = vec![d];
    let session = test_session(
        vec![ship, idle, enemy],
        vec![squadron, enemy_squadron, resting],
        true,
    );
    assert_eq!(selected_routes(&session), vec![&[a][..], &[c][..]]);
}

// Only the group's flying squadrons take its route.
// FUN_005c6610's route branch: at the route's end the player's group
// says "maneuver complete" (0x37 plus its slot) only while it holds a
// target (0x5c6a65 / 0x5c6a78 skip the line on an empty +0x98).
#[test]
fn a_fighter_route_ends_with_its_line_only_for_a_group_holding_a_target() {
    let run = |target: bool| {
        let enemy = test_ship(2, 0, false, true);
        let mut wing = test_fighter(5, true);
        wing.fighter_group = 0;
        if target {
            wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        }
        let mut session = test_session(vec![enemy], vec![wing], true);
        let here = session.fighters[0].source_position;
        session.fighters[0].navigation_route = vec![here];
        session.paused = false;
        let faction = session.player_voice_faction();
        let line = TacticalVoiceCue::fighter_maneuver_complete(faction, 0);
        let mut heard = false;
        for _ in 0..40 {
            session.step();
            heard |= session.take_pending_voice_cues().contains(&line);
        }
        (heard, session.fighters[0].navigation_route.is_empty())
    };
    assert_eq!(run(true), (true, true));
    assert_eq!(run(false), (false, true));
}

#[test]
fn a_fighter_group_route_skips_lost_squadrons_and_other_groups() {
    let mut lost = test_fighter(6, true);
    lost.alive = false;
    let mut spent = test_fighter(7, true);
    spent.squad_count = 0;
    let mut other = test_fighter(8, true);
    other.fighter_group = 1;
    let enemy = test_fighter(9, false);
    let mut session = test_session(
        Vec::new(),
        vec![test_fighter(5, true), lost, spent, other, enemy],
        true,
    );
    assert!(select_fighter_group(&mut session, 0));
    let point = at(1.0, 0.0, 0.0);
    assert_eq!(
        assign_selected_fighter_group_route(&mut session, point, false),
        1
    );
    for index in 1..5 {
        assert!(
            session.fighters[index].navigation_route.is_empty(),
            "{index}"
        );
    }
}

// 0x005a3f00: a task force's route line is 0x22 plus its slot; the
// one-unit state's (0x005a1f30) the generic 0x21. A Ctrl click on a
// held point plays nothing, and one on a new point plays only while the
// route (+0x80, slot +0xc -> FUN_005f5060, its first point) is empty.
#[test]
fn a_capital_route_plays_its_task_forces_line() {
    let mut ship = test_ship(1, 0, true, true);
    ship.task_force = 2;
    let mut session = test_session(vec![ship], Vec::new(), true);
    let faction = session.player_voice_faction();
    let point = at(1.0, 0.0, 0.0);
    queue_capital_route_voice(&mut session, point, false);
    assert!(session.pending_voice_cues.is_empty());
    assert!(select_task_force(&mut session, 2) > 0);
    let cues = session.pending_voice_cues.len();
    queue_capital_route_voice(&mut session, point, false);
    assert_eq!(
        session.pending_voice_cues[cues],
        TacticalVoiceCue::capital_maneuver(faction, 2)
    );
    queue_capital_route_voice(&mut session, at(5.0, 0.0, 0.0), true);
    assert_eq!(session.pending_voice_cues.len(), cues + 2);
    session.ships[0].navigation_route.push(point);
    queue_capital_route_voice(&mut session, point, true);
    assert_eq!(session.pending_voice_cues.len(), cues + 2);
    queue_capital_route_voice(&mut session, at(5.0, 0.0, 0.0), true);
    assert_eq!(session.pending_voice_cues.len(), cues + 2);
    session.ships[0].navigation_route.clear();
    select_clicked_ship(&mut session, 0, false);
    queue_capital_route_voice(&mut session, point, false);
    assert_eq!(
        session.pending_voice_cues[cues + 2],
        TacticalVoiceCue::capital_maneuver(faction, TACTICAL_UNGROUPED_TASK_FORCE)
    );
}

// 0x005a3020 (the several-units state's slot +0x10): several selected
// ships given a point form a task force in the first free slot
// (FUN_0059f610) and its line plays (0x22 plus the slot); with no slot
// free, each takes a force of its own and 0x84 plays. The point is the
// new force's whole route, Ctrl or not.
#[test]
fn several_ships_given_a_point_form_a_task_force() {
    let ships = (0..3)
        .map(|roster| test_ship(1, roster, true, true))
        .collect();
    let mut session = test_session(ships, Vec::new(), true);
    for index in 0..2 {
        session.ships[index].selected = true;
        session.ships[index].task_force = TACTICAL_UNGROUPED_TASK_FORCE;
    }
    session.ships[2].task_force = 0;
    session.ships[0].navigation_route.push(at(9.0, 0.0, 0.0));
    let faction = session.player_voice_faction();
    let cues = session.pending_voice_cues.len();
    let navigation = TacticalNavigationProjection {
        set: 0,
        point: 0,
        position: at(1.0, 0.0, 0.0),
        screen: macroquad::math::Vec2::ZERO,
    };
    click_navigation_point(&mut session, &navigation, true);
    assert_eq!(session.task_force_selection, Some(1));
    assert_eq!(session.ships[0].task_force, 1);
    assert_eq!(session.ships[1].task_force, 1);
    assert_eq!(session.ships[0].navigation_route, vec![at(1.0, 0.0, 0.0)]);
    assert_eq!(
        session.pending_voice_cues[cues..],
        [TacticalVoiceCue::capital_maneuver(faction, 1)]
    );
    // No slot free: forces of their own and the 0x84 line.
    let mut session = test_session(
        (0..10)
            .map(|roster| test_ship(1, roster, true, true))
            .collect(),
        Vec::new(),
        true,
    );
    for index in 0..8 {
        session.ships[index].task_force = index as u8;
    }
    for index in 8..10 {
        session.ships[index].selected = true;
        session.ships[index].task_force = TACTICAL_UNGROUPED_TASK_FORCE;
    }
    let cues = session.pending_voice_cues.len();
    click_navigation_point(&mut session, &navigation, false);
    assert_eq!(session.task_force_selection, None);
    assert_eq!(session.ships[8].task_force, TACTICAL_UNGROUPED_TASK_FORCE);
    assert_eq!(
        session.pending_voice_cues[cues..],
        [TacticalVoiceCue::target_task_force_rejected(faction)]
    );
    assert_eq!(session.ships[9].navigation_route, vec![at(1.0, 0.0, 0.0)]);
    // One live ship of ours, with a lost one and an enemy also
    // selected, is a single unit: no new force, its own line.
    let mut lost = test_ship(1, 1, true, true);
    lost.alive = false;
    let mut session = test_session(
        vec![
            test_ship(1, 0, true, true),
            lost,
            test_ship(2, 0, false, true),
        ],
        Vec::new(),
        true,
    );
    for ship in &mut session.ships {
        ship.selected = true;
        ship.task_force = TACTICAL_UNGROUPED_TASK_FORCE;
    }
    assert!(!route_selected_ships_as_a_force(&mut session));
    assert_eq!(session.task_force_selection, None);
    assert!(session.pending_voice_cues.is_empty());
}

// A0 VM 2026-10-27 and 0x005a1af0 / FUN_005aded0: a target given to one
// selected ship takes it out of its numbered group; a task force keeps
// its group.
#[test]
fn a_target_takes_a_single_ship_out_of_its_task_force() {
    let mut first = test_ship(1, 0, true, true);
    first.task_force = 0;
    let mut second = test_ship(3, 1, true, true);
    second.task_force = 0;
    let mut session = test_session(
        vec![first, second, test_ship(2, 0, false, true)],
        Vec::new(),
        true,
    );
    select_clicked_ship(&mut session, 0, false);
    assign_selected_capital_focus_target(&mut session, 2, false);
    assert_eq!(session.ships[0].task_force, TACTICAL_UNGROUPED_TASK_FORCE);
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(2))
    );
    assert_eq!(session.ships[1].task_force, 0);
    assert!(select_task_force(&mut session, 0) > 0);
    assign_selected_capital_focus_target(&mut session, 2, false);
    assert_eq!(session.ships[1].task_force, 0);
}

// FUN_005c2060 posts a Ctrl click with 1 to slot +0xc: with our ships
// selected, an enemy ship is a target added, then dropped (0x005a1af0).
#[test]
fn a_ctrl_click_on_an_enemy_adds_and_drops_a_target_of_our_ships() {
    let mut session = test_session(
        vec![
            test_ship(1, 0, true, true),
            test_ship(2, 0, false, true),
            test_ship(3, 1, false, true),
        ],
        Vec::new(),
        true,
    );
    select_clicked_ship(&mut session, 0, false);
    assert_eq!(ctrl_click_ship(&mut session, 1), "target");
    assert_eq!(ctrl_click_ship(&mut session, 2), "target");
    assert_eq!(
        session.ships[0].manual_targets,
        vec![
            TacticalAttackTarget::CapitalShip(1),
            TacticalAttackTarget::CapitalShip(2)
        ]
    );
    ctrl_click_ship(&mut session, 1);
    assert_eq!(
        session.ships[0].manual_targets,
        vec![TacticalAttackTarget::CapitalShip(2)]
    );
    assert!(session.ships[0].selected && !session.ships[1].selected);
}

// 0x005a4bd0 with 1: a fighter group adds the enemy as its target.
#[test]
fn a_ctrl_click_on_an_enemy_targets_it_for_the_fighter_group() {
    let mut session = test_session(
        vec![test_ship(2, 0, false, true)],
        vec![test_fighter(5, true)],
        true,
    );
    assert!(select_fighter_group(&mut session, 0));
    assert_eq!(ctrl_click_ship(&mut session, 0), "fighter_target");
    assert_eq!(
        session.fighters[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(0))
    );
}

// 0x005a0d10 / 0x005a1240: with none of ours selected, a Ctrl click on
// an enemy selects it.
#[test]
fn a_ctrl_click_on_an_enemy_without_our_selection_selects_it() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(2, 0, false, true)],
        Vec::new(),
        true,
    );
    assert_eq!(ctrl_click_ship(&mut session, 1), "selected");
    assert!(session.ships[1].selected);
    assert_eq!(session.selected_ship, Some(1));
}

// 0x005a3aa0 with 1: a ship joins the selected task force
// (FUN_005ade50) or leaves it (FUN_005a8d30); the last one ends it.
#[test]
fn a_ctrl_click_with_a_task_force_moves_ships_in_and_out_of_it() {
    let mut first = test_ship(1, 0, true, true);
    first.task_force = 0;
    let mut other = test_ship(3, 1, true, true);
    other.task_force = 1;
    let mut session = test_session(vec![first, other], Vec::new(), true);
    assert!(select_task_force(&mut session, 0) > 0);
    assert_eq!(ctrl_click_ship(&mut session, 1), "joined_task_force");
    assert_eq!(session.ships[1].task_force, 0);
    assert!(session.ships[1].selected);
    assert_eq!(ctrl_click_ship(&mut session, 0), "left_task_force");
    assert_eq!(session.ships[0].task_force, TACTICAL_UNGROUPED_TASK_FORCE);
    assert!(!session.ships[0].selected);
    assert_eq!(session.task_force_selection, Some(0));
    assert_eq!(session.selected_ship, Some(1));
    assert_eq!(ctrl_click_ship(&mut session, 1), "left_task_force");
    assert_eq!(session.task_force_selection, None);
    assert_eq!(session.selected_ship, None);
}

// 0x005a3aa0 with 1 and FUN_005ca310: the force's escort of that ship
// is dropped (order 0).
#[test]
fn a_ctrl_click_with_a_task_force_drops_its_escort_of_the_ship() {
    let mut first = test_ship(1, 0, true, true);
    first.task_force = 0;
    first.order = TacticalOrder::Escort;
    first.escort_target = Some(1);
    let mut guarded = test_ship(3, 1, true, true);
    guarded.task_force = 1;
    let mut attacking = test_ship(4, 2, true, true);
    attacking.task_force = 0;
    attacking.order = TacticalOrder::AttackCapitalShips;
    let mut outsider = test_ship(5, 3, true, true);
    outsider.task_force = 2;
    outsider.order = TacticalOrder::Escort;
    outsider.escort_target = Some(1);
    let mut session = test_session(vec![first, guarded, attacking, outsider], Vec::new(), true);
    assert!(select_task_force(&mut session, 0) > 0);
    assert_eq!(ctrl_click_ship(&mut session, 1), "escort_dropped");
    assert_eq!(session.ships[0].escort_target, None);
    assert_eq!(session.ships[0].order, TacticalOrder::None);
    assert_eq!(session.ships[1].task_force, 1);
    // Only the force's escorts of that ship change.
    assert_eq!(session.ships[2].order, TacticalOrder::AttackCapitalShips);
    assert_eq!(session.ships[3].order, TacticalOrder::Escort);
    assert_eq!(session.ships[3].escort_target, Some(1));
}

// 0x005a4bd0 with 1 on our ship: a fighter group escorts it, or drops
// the escort it holds.
#[test]
fn a_ctrl_click_with_a_fighter_group_toggles_its_escort() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true)],
        vec![test_fighter(5, true)],
        true,
    );
    assert!(select_fighter_group(&mut session, 0));
    assert_eq!(ctrl_click_ship(&mut session, 0), "escort");
    assert_eq!(session.fighters[0].escort_target, Some(0));
    assert_eq!(session.fighters[0].order, TacticalOrder::Escort);
    assert_eq!(ctrl_click_ship(&mut session, 0), "escort_dropped");
    assert_eq!(session.fighters[0].escort_target, None);
    assert_eq!(session.fighters[0].order, TacticalOrder::None);
}

// Only our selected living squadrons take an order: Escort, or one from
// the panel. A Recover order keeps the recovery reservation.
#[test]
fn only_our_selected_living_squadrons_take_an_order() {
    let squadron = |selected, alive, squad_count, is_attacker| {
        let mut fighter = test_fighter(5, true);
        fighter.selected = selected;
        fighter.alive = alive;
        fighter.squad_count = squad_count;
        fighter.is_attacker = is_attacker;
        fighter.recovery_state = TacticalFighterRecoveryState::Returning;
        fighter.recovery_target = Some(0);
        fighter
    };
    let fighters = vec![
        squadron(true, true, 12, true),
        squadron(false, true, 12, true),
        squadron(true, false, 12, true),
        squadron(true, true, 0, true),
        squadron(true, true, 12, false),
    ];
    let mut session = test_session(vec![test_ship(1, 0, true, true)], fighters, true);
    assert_eq!(assign_selected_escort(&mut session, 0), (0, 1));
    let escorting = session
        .fighters
        .iter()
        .map(|fighter| fighter.order == TacticalOrder::Escort)
        .collect::<Vec<_>>();
    assert_eq!(escorting, [true, false, false, false, false]);

    session.selected_fighter_group = Some(0);
    session.fighters[0].recovery_target = Some(0);
    assign_selected_command(&mut session, TacticalOrder::Recover, None);
    let recovering = session
        .fighters
        .iter()
        .map(|fighter| fighter.order == TacticalOrder::Recover)
        .collect::<Vec<_>>();
    assert_eq!(recovering, [true, false, false, true, false]);
    assert_eq!(session.fighters[0].recovery_target, Some(0));
    assign_selected_command(&mut session, TacticalOrder::HoldPosition, None);
    assert_eq!(session.fighters[0].recovery_target, None);
}

// SetOrder (0x005ca530) on a force leaving Escort calls SetTactic(1)
// (0x005c9be0). A0 VM 2026-10-28, Battle at Yavin: Red Group on Stand
// Off, Escort Victory Destroyer 2, then a Ctrl click on that ship: "No
// Orders", "Tactics: Surround".
#[test]
fn a_force_leaving_escort_takes_surround() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true)],
        vec![test_fighter(5, true)],
        true,
    );
    session.fighters[0].tactic = TacticalTactic::StandOff;
    assert!(select_fighter_group(&mut session, 0));
    ctrl_click_ship(&mut session, 0);
    assert_eq!(session.fighters[0].tactic, TacticalTactic::StandOff);
    assert_eq!(ctrl_click_ship(&mut session, 0), "escort_dropped");
    assert_eq!(session.fighters[0].tactic, TacticalTactic::Surround);

    // Any other order, here from the panel, leaves Escort the same way;
    // a force that was not escorting keeps its tactic.
    let mut escort = test_ship(3, 1, true, true);
    escort.selected = true;
    escort.task_force = 1;
    escort.order = TacticalOrder::Escort;
    escort.escort_target = Some(0);
    escort.tactic = TacticalTactic::StandOff;
    let mut idle = test_ship(4, 2, true, true);
    idle.task_force = 2;
    idle.tactic = TacticalTactic::StandOff;
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), escort, idle],
        Vec::new(),
        true,
    );
    session.task_force_selection = Some(1);
    assign_selected_command(&mut session, TacticalOrder::HoldPosition, None);
    assert_eq!(session.ships[1].tactic, TacticalTactic::Surround);
    assert_eq!(session.ships[2].tactic, TacticalTactic::StandOff);
    let tactic_of = |task_force| TacticalTurnEvent::TaskForceTactic {
        attacker: true,
        task_force,
        tactic: TacticalTactic::Surround,
    };
    assert!(session
        .turn_events
        .iter()
        .any(|queued| queued.event == tactic_of(1)));
    assert!(!session
        .turn_events
        .iter()
        .any(|queued| queued.event == tactic_of(2)));

    // A task force's Ctrl click drops the escort of each member: one
    // 0x3d for the force. A ship of no force posts none.
    let escort = |id, slot, task_force| {
        let mut ship = test_ship(id, slot, true, true);
        ship.task_force = task_force;
        ship.order = TacticalOrder::Escort;
        ship.escort_target = Some(0);
        ship.tactic = TacticalTactic::StandOff;
        ship
    };
    let mut session = test_session(
        vec![
            test_ship(1, 0, true, true),
            escort(3, 1, 1),
            escort(4, 2, 1),
            escort(5, 3, TACTICAL_UNGROUPED_TASK_FORCE),
        ],
        Vec::new(),
        true,
    );
    session.task_force_selection = Some(1);
    assert_eq!(ctrl_click_ship(&mut session, 0), "escort_dropped");
    assert_eq!(session.ships[1].tactic, TacticalTactic::Surround);
    assert_eq!(session.ships[2].tactic, TacticalTactic::Surround);
    assert_eq!(session.ships[3].order, TacticalOrder::Escort);
    let posted = |session: &BattleSession| {
        session
            .turn_events
            .iter()
            .filter(|queued| matches!(queued.event, TacticalTurnEvent::TaskForceTactic { .. }))
            .count()
    };
    assert_eq!(posted(&session), 1);
    session.leave_escort(&[3], &[]);
    assert_eq!(session.ships[3].tactic, TacticalTactic::Surround);
    assert_eq!(posted(&session), 1);
}

// 0x005a1af0 / 0x005a24d0 with 1: our ship joins or leaves the
// selection.
#[test]
fn a_ctrl_click_on_our_ship_toggles_it_in_the_selection() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), test_ship(3, 1, true, true)],
        Vec::new(),
        true,
    );
    select_clicked_ship(&mut session, 0, false);
    assert_eq!(ctrl_click_ship(&mut session, 1), "toggled");
    assert!(session.ships[0].selected && session.ships[1].selected);
    ctrl_click_ship(&mut session, 1);
    assert!(!session.ships[1].selected);
}

fn navigation_at(position: TacticalWorldPosition) -> TacticalNavigationProjection {
    TacticalNavigationProjection {
        set: 0,
        point: 0,
        position,
        screen: macroquad::math::Vec2::ZERO,
    }
}

// 0x005a1f30 / 0x005a3f00: a point replaces the route of our selected
// living ships with order 0; with Ctrl it toggles (FUN_005ca330).
#[test]
fn a_navigation_click_routes_only_our_selected_ships() {
    let mut first = test_ship(1, 0, true, true);
    first.selected = true;
    first.order = TacticalOrder::Escort;
    first.escort_target = Some(1);
    let mut second = test_ship(3, 1, true, true);
    second.selected = true;
    let mut lost = test_ship(4, 2, true, false);
    lost.selected = true;
    let idle = test_ship(5, 3, true, true);
    let mut enemy = test_ship(2, 0, false, true);
    enemy.selected = true;
    let mut session = test_session(vec![first, second, lost, idle, enemy], Vec::new(), true);
    let (a, b) = (at(1.0, 0.0, 0.0), at(2.0, 0.0, 0.0));
    assert_eq!(
        click_navigation_point(&mut session, &navigation_at(a), false),
        (2, 1, false)
    );
    assert_eq!(session.ships[0].navigation_route, vec![a]);
    assert_eq!(session.ships[0].order, TacticalOrder::None);
    assert_eq!(session.ships[0].escort_target, None);
    assert_eq!(session.ships[0].source_waypoint, Some(a));
    for index in 2..5 {
        assert!(session.ships[index].navigation_route.is_empty(), "{index}");
    }
    assert_eq!(
        click_navigation_point(&mut session, &navigation_at(b), true),
        (2, 2, false)
    );
    assert_eq!(
        click_navigation_point(&mut session, &navigation_at(a), true),
        (2, 1, true)
    );
    assert_eq!(session.ships[1].navigation_route, vec![b]);
    assert_eq!(session.ships[1].source_waypoint, Some(b));
}

// 0x005a4e50: with a fighter group selected, the group takes the point.
#[test]
fn a_navigation_click_with_a_fighter_group_routes_the_group() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true)],
        vec![test_fighter(5, true)],
        true,
    );
    assert!(select_fighter_group(&mut session, 0));
    let point = at(1.0, 0.0, 0.0);
    assert_eq!(
        click_navigation_point(&mut session, &navigation_at(point), false),
        (1, 0, false)
    );
    assert_eq!(session.fighters[0].navigation_route, vec![point]);
    assert!(session.ships[0].navigation_route.is_empty());
}

// 0x005a4bd0's escort toggle counts only the group's flying squadrons;
// 0x005a3aa0's escort test only the force's members.
#[test]
fn a_ctrl_click_escort_counts_only_the_selection() {
    let mut member = test_fighter(5, true);
    member.order = TacticalOrder::Escort;
    member.escort_target = Some(0);
    let escorting = |mut fighter: TacticalFighter| {
        fighter.order = TacticalOrder::Escort;
        fighter.escort_target = Some(0);
        fighter
    };
    let mut lost = escorting(test_fighter(6, true));
    lost.alive = false;
    let mut spent = escorting(test_fighter(7, true));
    spent.squad_count = 0;
    let mut other = escorting(test_fighter(8, true));
    other.fighter_group = 1;
    let enemy = escorting(test_fighter(9, false));
    let mut session = test_session(
        vec![test_ship(1, 0, true, true)],
        vec![member, lost, spent, other, enemy],
        true,
    );
    assert!(select_fighter_group(&mut session, 0));
    assert_eq!(ctrl_click_ship(&mut session, 0), "escort_dropped");
    assert_eq!(session.fighters[0].order, TacticalOrder::None);
    for index in 1..5 {
        assert_eq!(
            session.fighters[index].order,
            TacticalOrder::Escort,
            "{index}"
        );
        assert_eq!(session.fighters[index].escort_target, Some(0), "{index}");
    }
    // A ship of another force escorting it does not count.
    let mut first = test_ship(1, 0, true, true);
    first.task_force = 0;
    let mut outsider = test_ship(3, 1, true, true);
    outsider.task_force = 1;
    outsider.order = TacticalOrder::Escort;
    outsider.escort_target = Some(2);
    let mut lost_escort = test_ship(4, 2, true, false);
    lost_escort.task_force = 0;
    lost_escort.escort_target = Some(2);
    let mut guarded = test_ship(5, 3, true, true);
    guarded.task_force = 2;
    let mut session = test_session(
        vec![first, outsider, guarded, lost_escort],
        Vec::new(),
        true,
    );
    assert!(select_task_force(&mut session, 0) > 0);
    assert_eq!(ctrl_click_ship(&mut session, 2), "joined_task_force");
    assert_eq!(session.ships[1].escort_target, Some(2));
}

// 0x005e45a0: Tab presses the capital panel's next button (0xdd04),
// Shift+Tab its previous one (0xdd03), only while that panel shows.
#[test]
fn tab_steps_through_the_capital_panel_only_while_it_shows() {
    let mut state = TacticalState::default();
    assert_eq!(tab_panel_control(&state, false), None);
    let mut first = test_ship(1, 0, true, true);
    first.task_force = 0;
    let mut session = test_session(vec![first], Vec::new(), true);
    assert!(select_task_force(&mut session, 0) > 0);
    state.session = Some(session);
    assert_eq!(
        tab_panel_control(&state, false),
        Some(TacticalCommandControl::NextCapital)
    );
    assert_eq!(
        tab_panel_control(&state, true),
        Some(TacticalCommandControl::PreviousCapital)
    );
    state.battle_options_open = true;
    assert_eq!(tab_panel_control(&state, false), None);
    state.battle_options_open = false;
    state.withdraw_confirmation_open = true;
    assert_eq!(tab_panel_control(&state, false), None);
    state.withdraw_confirmation_open = false;
    state.command_panel = TacticalCommandPanel::Missions {
        pending_order: TacticalOrder::None,
    };
    assert_eq!(tab_panel_control(&state, false), None);
}

// FUN_005c2990 mode 9 looks at the selection's unit: a ship, or a
// fighter group's first flying squadron (0x005a4710), id 1001 + index.
#[test]
fn the_camera_target_is_the_selected_ship_or_the_groups_squadron() {
    let mut lost = test_fighter(5, true);
    lost.alive = false;
    let mut spent = test_fighter(6, true);
    spent.squad_count = 0;
    let mut other = test_fighter(7, true);
    other.fighter_group = 1;
    let enemy = test_fighter(8, false);
    let mut flying = test_fighter(9, true);
    flying.source_position = at(1.0, 2.0, 3.0);
    let mut ship = test_ship(1, 0, true, true);
    ship.source_position = at(4.0, 5.0, 6.0);
    let mut session = test_session(
        vec![test_ship(2, 0, false, true), ship],
        vec![lost, spent, other, enemy, flying],
        true,
    );
    assert_eq!(camera_target_of_selection(&session), None);
    select_clicked_ship(&mut session, 1, false);
    assert_eq!(
        camera_target_of_selection(&session).map(|(id, _, _, at)| (id, at)),
        Some((2, at(4.0, 5.0, 6.0)))
    );
    assert!(select_fighter_group(&mut session, 0));
    assert_eq!(
        camera_target_of_selection(&session).map(|(id, _, _, at)| (id, at)),
        Some((1005, at(1.0, 2.0, 3.0)))
    );
}

// FUN_006030c0(8) / FUN_00604170 / FUN_005dd3a0: a second click on the
// chosen maneuver releases it, and confirming then gives order 0.
#[test]
fn a_second_click_on_the_chosen_maneuver_cancels_it() {
    assert_eq!(
        toggled_maneuver(TacticalOrder::None, TacticalOrder::Hammer),
        TacticalOrder::Hammer
    );
    assert_eq!(
        toggled_maneuver(TacticalOrder::Anvil, TacticalOrder::Hammer),
        TacticalOrder::Hammer
    );
    assert_eq!(
        toggled_maneuver(TacticalOrder::Hammer, TacticalOrder::Hammer),
        TacticalOrder::None
    );
    let mut state = TacticalState::default();
    let mut ship = test_ship(1, 0, true, true);
    ship.task_force = 0;
    ship.order = TacticalOrder::HoldPosition;
    let mut session = test_session(vec![ship], Vec::new(), true);
    assert!(select_task_force(&mut session, 0) > 0);
    state.session = Some(session);
    activate_tactical_command(&mut state, TacticalCommandControl::OpenManeuvers);
    assert!(matches!(
        state.command_panel,
        TacticalCommandPanel::Maneuvers {
            pending_order: TacticalOrder::HoldPosition,
            ..
        }
    ));
    activate_tactical_command(&mut state, TacticalCommandControl::HoldPosition);
    assert!(matches!(
        state.command_panel,
        TacticalCommandPanel::Maneuvers {
            pending_order: TacticalOrder::None,
            ..
        }
    ));
    activate_tactical_command(&mut state, TacticalCommandControl::Confirm);
    assert_eq!(
        state.session.as_ref().unwrap().ships[0].order,
        TacticalOrder::None
    );
}

// 0x005d9640 looks at the target's drawn frame: the step's pose blended
// the way the scene draws it.
#[test]
fn the_camera_follows_its_targets_drawn_position() {
    let mut ship = test_ship(1, 0, true, true);
    ship.source_position = at(10.0, 0.0, 0.0);
    let mut squadron = test_fighter(5, true);
    squadron.source_position = at(0.0, 0.0, 20.0);
    let mut session = test_session(
        vec![ship],
        vec![test_fighter(6, true), test_fighter(7, true), squadron],
        true,
    );
    session.combat_tick = 5;
    session.step_accumulator = 0.5;
    let mut state = TacticalState::default();
    state.previous_poses = Some(TacticalPoses {
        tick: 4,
        ships: vec![(at(0.0, 0.0, 0.0), TacticalWorldVector::SOURCE_FORWARD)],
        fighters: vec![at(0.0, 0.0, 0.0), at(0.0, 0.0, 0.0), at(0.0, 0.0, 10.0)],
    });
    state.session = Some(session);
    state
        .asset_renderer
        .restore_camera_state(crate::tactical_assets::OriginalTacticalCamera::new(
            false, 100.0,
        ));
    let target = |state: &TacticalState| {
        state
            .asset_renderer
            .camera_state()
            .map(|camera| camera.target_point())
    };
    state
        .asset_renderer
        .focus_target(1, macroquad::math::Vec3::ZERO);
    track_tactical_camera_target(&mut state);
    assert_eq!(target(&state), Some(at(5.0, 0.0, 0.0).rendered()));
    state
        .asset_renderer
        .focus_target(1003, macroquad::math::Vec3::ZERO);
    track_tactical_camera_target(&mut state);
    assert_eq!(target(&state), Some(at(0.0, 0.0, 15.0).rendered()));
}

// A0 VM 2026-10-27: a fighter group still in its hangar gives the
// camera its carrier.
#[test]
fn the_camera_target_of_a_group_in_its_hangar_is_the_carrier() {
    let mut carrier = test_ship(1, 0, true, true);
    carrier.source_position = at(7.0, 8.0, 9.0);
    let mut inside = test_fighter(5, true);
    inside.alive = false;
    inside.in_hangar = true;
    inside.home_carrier = Some(1);
    let mut session = test_session(
        vec![test_ship(2, 0, true, true), carrier],
        vec![inside],
        true,
    );
    session.selected_fighter_group = Some(0);
    assert_eq!(
        camera_target_of_selection(&session).map(|(id, _, _, at)| (id, at)),
        Some((2, at(7.0, 8.0, 9.0)))
    );
    // A lost carrier gives nothing.
    session.ships[1].alive = false;
    assert_eq!(camera_target_of_selection(&session), None);
}

// 0x005d9640 looks at the target object's current frame each frame.
#[test]
fn the_camera_target_tracks_its_object_by_id() {
    let mut ship = test_ship(1, 0, true, true);
    ship.source_position = at(1.0, 2.0, 3.0);
    let mut lost = test_ship(2, 0, false, false);
    lost.source_position = at(9.0, 9.0, 9.0);
    let mut squadron = test_fighter(5, true);
    squadron.source_position = at(4.0, 5.0, 6.0);
    let session = test_session(vec![ship, lost], vec![squadron], true);
    assert_eq!(
        tracked_target_position(&session, 1),
        Some(at(1.0, 2.0, 3.0))
    );
    assert_eq!(tracked_target_position(&session, 2), None);
    assert_eq!(tracked_target_position(&session, 3), None);
    assert_eq!(tracked_target_position(&session, 0), None);
    assert_eq!(
        tracked_target_position(&session, 1001),
        Some(at(4.0, 5.0, 6.0))
    );
    assert_eq!(tracked_target_position(&session, 1002), None);
    // The chase cameras' group ids are not objects.
    assert_eq!(tracked_target_position(&session, 10_001), None);
}

// FUN_0059f800: four 3x3x3 grids spaced k x extent x 0.125, the HUD's
// leftmost button the widest (k = 4), the rightmost the tightest with
// its centre; points run x, then y, then z, from -1 to 1.
#[test]
fn the_navigation_sets_are_the_originals_grids() {
    let layout = OriginalTacticalLayout::from_active_counts(0, 0);
    let sets = original_navigation_point_sets(layout);
    let base = layout.battle_extent * 0.125;
    assert_eq!(
        sets.iter().map(Vec::len).collect::<Vec<_>>(),
        [26, 26, 26, 27]
    );
    for (button, set) in sets.iter().enumerate() {
        let step = (4 - button) as f32 * base;
        assert_eq!(set[0], at(-step, -step, -step), "{button}");
        assert_eq!(set[1], at(-step, -step, 0.0), "{button}");
        assert_eq!(set[3], at(-step, 0.0, -step), "{button}");
        assert_eq!(*set.last().unwrap(), at(step, step, step), "{button}");
    }
    assert!(sets[3].contains(&at(0.0, 0.0, 0.0)));
    assert!(!sets[0].contains(&at(0.0, 0.0, 0.0)));
}

// FUN_005d8c30's layer meshes; A0 VM 2026-10-27: top layer white,
// middle red, bottom blue.
#[test]
fn a_navigation_points_colour_follows_its_layer() {
    assert_eq!(navigation_layer_tint(1.0), vec3(1.0, 1.0, 1.0));
    assert_eq!(navigation_layer_tint(0.0), vec3(1.0, 0.0, 0.0));
    assert_eq!(navigation_layer_tint(-1.0), vec3(0.0, 0.541, 1.0));
}

#[test]
fn a_unit_under_the_pointer_comes_before_a_navigation_point() {
    assert!(no_unit_under_pointer(None, None, None));
    assert!(!no_unit_under_pointer(Some(0), None, None));
    assert!(!no_unit_under_pointer(None, Some(0), None));
    assert!(!no_unit_under_pointer(None, None, Some(0)));
}

// FUN_0059fb30 -> FUN_005c1e80: one marker per shown point, at the point,
// in FUN_0059f560's colour.
#[test]
fn each_shown_point_gets_a_marker_in_its_state_colour() {
    let (first, later, picked, idle) = (
        at(1.0, 0.0, 0.0),
        at(2.0, 1.0, 0.0),
        at(3.0, -1.0, 0.0),
        at(4.0, -1.0, 0.0),
    );
    let mut ship = test_ship(1, 0, true, true);
    ship.selected = true;
    ship.navigation_route = vec![first, later];
    let session = test_session(vec![ship], Vec::new(), true);
    let projections = [first, later, picked, idle].map(|position| TacticalNavigationProjection {
        set: 0,
        point: 0,
        position,
        screen: macroquad::math::Vec2::ZERO,
    });
    let markers = tactical_navigation_markers(&session, &projections, Some(picked));
    assert_eq!(
        markers,
        vec![
            (first.rendered(), vec3(1.0, 1.0, 0.0)),
            (later.rendered(), vec3(1.0, 0.0, 1.0)),
            (picked.rendered(), vec3(1.0, 0.0, 1.0)),
            (idle.rendered(), vec3(0.0, 0.541, 1.0)),
        ]
    );
}

// A0 VM 2026-10-27: the selected navigation point is purple, the
// magenta mesh (FUN_0059f560's +0x2c).
#[test]
fn the_selected_navigation_point_is_purple() {
    let point = at(0.0, 0.0, 0.0);
    assert_eq!(
        navigation_point_tint(point, &[], false),
        vec3(1.0, 0.0, 0.0)
    );
    assert_eq!(navigation_point_tint(point, &[], true), vec3(1.0, 0.0, 1.0));
}

// FUN_005a0840: the selected unit's first route point is yellow (+0x20),
// its later ones magenta (+0x2c), and yellow wins (FUN_0059f560). A0
// TAC-04-C002 shows the first point yellow, TAC-04-C003 a second one
// magenta beside it.
#[test]
fn a_routes_first_point_is_yellow_and_its_later_ones_magenta() {
    let (a, b, c) = (at(1.0, 1.0, 0.0), at(2.0, -1.0, 0.0), at(3.0, 0.0, 0.0));
    let first: &[TacticalWorldPosition] = &[a, b];
    let second: &[TacticalWorldPosition] = &[b];
    assert_eq!(
        navigation_point_tint(a, &[first], false),
        vec3(1.0, 1.0, 0.0)
    );
    assert_eq!(
        navigation_point_tint(b, &[first], false),
        vec3(1.0, 0.0, 1.0)
    );
    assert_eq!(
        navigation_point_tint(b, &[first, second], false),
        vec3(1.0, 1.0, 0.0)
    );
    assert_eq!(
        navigation_point_tint(a, &[first], true),
        vec3(1.0, 1.0, 0.0)
    );
    assert_eq!(
        navigation_point_tint(c, &[first], false),
        vec3(1.0, 0.0, 0.0)
    );
}

// 0x005a0e10 / FUN_005a0e70 and A0 VM 2026-10-27: a left click on a
// navigation point ends the unit selection and selects the point, and
// the camera target button then looks at it (0x005a55f0).
#[test]
fn a_selected_navigation_point_ends_the_unit_selection_and_aims_the_camera() {
    let mut session = test_session(
        vec![test_ship(1, 0, true, true)],
        vec![test_fighter(5, true)],
        true,
    );
    select_clicked_ship(&mut session, 0, false);
    let mut selected = None;
    let point = at(4.0, 5.0, 6.0);
    select_navigation_point(&mut session, &mut selected, point);
    assert_eq!(selected, Some(point));
    assert!(!session.ships[0].selected);
    assert_eq!(session.selected_ship, None);
    let mut state = TacticalState::default();
    state.session = Some(session);
    state.selected_navigation_point = selected;
    state
        .asset_renderer
        .restore_camera_state(crate::tactical_assets::OriginalTacticalCamera::new(
            false, 100.0,
        ));
    let (camera_x, camera_y) = (state.camera_x, state.camera_y);
    activate_tactical_hud_control(&mut state, TacticalHudControl::CameraTarget);
    assert_eq!(
        state
            .asset_renderer
            .camera_state()
            .map(|camera| camera.target_point()),
        Some(point.rendered())
    );
    assert_eq!((state.camera_x, state.camera_y), (camera_x, camera_y));
    // A group button replaces the point selection.
    activate_tactical_hud_control(&mut state, TacticalHudControl::FighterGroup(0));
    assert_eq!(state.selected_navigation_point, None);
}

// FUN_0059b730's virtual keys and the button ids they press.
#[test]
fn each_battle_key_presses_its_original_button() {
    let find = |key: KeyCode| {
        TACTICAL_BUTTON_KEYS
            .iter()
            .filter(|(each, _)| *each == key)
            .map(|(_, control)| *control)
            .collect::<Vec<_>>()
    };
    for (keys, control) in [
        (
            &[KeyCode::Enter, KeyCode::KpDecimal][..],
            TacticalHudControl::CameraTarget,
        ),
        (
            &[KeyCode::PageUp, KeyCode::Kp9, KeyCode::KpAdd],
            TacticalHudControl::ZoomIn,
        ),
        (
            &[KeyCode::PageDown, KeyCode::Kp3, KeyCode::KpSubtract],
            TacticalHudControl::ZoomOut,
        ),
        (
            &[KeyCode::End, KeyCode::Kp1],
            TacticalHudControl::CameraRecall,
        ),
        (
            &[KeyCode::Home, KeyCode::Kp7],
            TacticalHudControl::CameraMemorize,
        ),
        (
            &[KeyCode::Left, KeyCode::Kp4],
            TacticalHudControl::CameraLeft,
        ),
        (&[KeyCode::Up, KeyCode::Kp8], TacticalHudControl::CameraUp),
        (
            &[KeyCode::Right, KeyCode::Kp6],
            TacticalHudControl::CameraRight,
        ),
        (
            &[KeyCode::Down, KeyCode::Kp2],
            TacticalHudControl::CameraDown,
        ),
        (&[KeyCode::Key1], TacticalHudControl::TaskForce(0)),
        (&[KeyCode::Key8], TacticalHudControl::TaskForce(7)),
    ] {
        for key in keys {
            assert_eq!(find(*key), vec![control], "{key:?}");
        }
    }
    // The main row's = and - are not the original's keys (0x6b, 0x6d
    // are the NumPad's).
    assert!(find(KeyCode::Equal).is_empty());
    assert!(find(KeyCode::Minus).is_empty());
}

// FUN_00595ba0 / FUN_00595bc0: only an occupied group gives F1-F12 a
// camera to follow.
#[test]
fn a_camera_follow_key_needs_a_group_with_a_unit_left() {
    let mut lost = test_ship(2, 1, true, false);
    lost.task_force = 1;
    let mut squadron = test_fighter(5, true);
    squadron.fighter_group = 2;
    let mut session = test_session(
        vec![test_ship(1, 0, true, true), lost],
        vec![squadron],
        true,
    );
    session.ships[0].task_force = 0;
    assert!(task_force_exists(&session, 0));
    assert!(!task_force_exists(&session, 1));
    assert!(fighter_group_exists(&session, 2));
    session.fighters[0].squad_count = 0;
    assert!(!fighter_group_exists(&session, 2));
    assert!(!fighter_group_exists(&session, 0));
}

#[test]
fn the_count_line_counts_only_the_players_selected_group() {
    let own = test_fighter(3, true);
    let mut enemy = test_fighter(1, false);
    enemy.fighter_group = 0;
    let mut other_group = test_fighter(4, true);
    other_group.fighter_group = 1;
    let mut session = test_session(Vec::new(), vec![own, enemy, other_group], true);
    session.selected_fighter_group = Some(0);
    let hud = TacticalGroupHud::from_session(&session);
    assert_eq!(
        hud.selected_fighter_count.map(TacticalFighterCount::line),
        Some("X-wings: 1 12/12".to_owned())
    );
}

#[test]
fn a_battle_opens_with_both_side_highlights_off() {
    // A0 TAC-01-C003 (Alliance) and TAC-01-C002 (Empire).
    let state = TacticalState::default();
    assert!(!state.highlight_alliance);
    assert!(!state.highlight_empire);
}

#[test]
fn the_pause_label_takes_the_players_colour() {
    // A0 TAC-01-C003 pure red, TAC-01-C002 pure green.
    assert_eq!(
        tactical_pause_label_color(true),
        Color::new(1.0, 0.0, 0.0, 1.0)
    );
    assert_eq!(
        tactical_pause_label_color(false),
        Color::new(0.0, 1.0, 0.0, 1.0)
    );
}

#[test]
fn the_holocube_spans_half_the_battle_extent_on_every_axis() {
    // FUN_005d4d10 scales a +-0.5 unit cube by the extent +0x8cc.
    let edges = tactical_holocube_edges(112.0);
    assert_eq!(edges.len(), 12);
    for (from, to) in edges {
        assert_eq!((to - from).length(), 112.0);
        for point in [from, to] {
            assert_eq!(point.abs(), vec3(56.0, 56.0, 56.0));
        }
    }
    let corners = edges
        .iter()
        .flat_map(|(from, to)| [*from, *to])
        .map(|point| (point.x as i32, point.y as i32, point.z as i32))
        .collect::<HashSet<_>>();
    assert_eq!(corners.len(), 8);
}

#[test]
fn the_holocube_edges_join_each_corner_to_its_three_neighbours() {
    let edges = tactical_holocube_edges(2.0);
    // Each edge changes exactly one coordinate from -1 to +1.
    for (from, to) in edges {
        let changed = [from.x != to.x, from.y != to.y, from.z != to.z];
        assert_eq!(changed.iter().filter(|c| **c).count(), 1);
        assert!(from.x <= to.x && from.y <= to.y && from.z <= to.z);
    }
    assert_eq!(edges[0], (vec3(-1.0, -1.0, -1.0), vec3(1.0, -1.0, -1.0)));
}

#[test]
fn a_holocube_segment_is_clipped_to_the_aperture() {
    let rect = (0.0, 0.0, 10.0, 10.0);
    assert_eq!(
        clip_segment_to_rect(vec2(-5.0, 5.0), vec2(15.0, 5.0), rect),
        Some((vec2(0.0, 5.0), vec2(10.0, 5.0)))
    );
    assert_eq!(
        clip_segment_to_rect(vec2(-5.0, -1.0), vec2(15.0, -1.0), rect),
        None
    );
    assert_eq!(
        clip_segment_to_rect(vec2(2.0, 2.0), vec2(4.0, 6.0), rect),
        Some((vec2(2.0, 2.0), vec2(4.0, 6.0)))
    );
    assert_eq!(
        clip_segment_to_rect(vec2(5.0, 12.0), vec2(5.0, 20.0), rect),
        None
    );
    // An offset rectangle, and segments lying on its edges.
    let offset = (10.0, 20.0, 30.0, 40.0);
    assert_eq!(
        clip_segment_to_rect(vec2(0.0, 30.0), vec2(40.0, 30.0), offset),
        Some((vec2(10.0, 30.0), vec2(30.0, 30.0)))
    );
    assert_eq!(
        clip_segment_to_rect(vec2(20.0, 0.0), vec2(20.0, 50.0), offset),
        Some((vec2(20.0, 20.0), vec2(20.0, 40.0)))
    );
    assert_eq!(
        clip_segment_to_rect(vec2(10.0, 25.0), vec2(10.0, 35.0), offset),
        Some((vec2(10.0, 25.0), vec2(10.0, 35.0)))
    );
    assert_eq!(
        clip_segment_to_rect(vec2(12.0, 40.0), vec2(18.0, 40.0), offset),
        Some((vec2(12.0, 40.0), vec2(18.0, 40.0)))
    );
    assert_eq!(
        clip_segment_to_rect(vec2(5.0, 25.0), vec2(5.0, 35.0), offset),
        None
    );
}

#[test]
fn the_battle_windows_open_over_each_sides_command_center() {
    // A0 TAC-01-C010/TAC-07-C001 (Alliance), TAC-07-C002 (Empire).
    set_battle_window_origin(false);
    assert_eq!(battle_window_origin(), (125.0, 52.0));
    set_battle_window_origin(true);
    assert_eq!(battle_window_origin(), (62.0, 49.0));
    assert_eq!(
        battle_window_text_color(true),
        Color::from_rgba(255, 0, 0, 255)
    );
    assert_eq!(
        battle_window_text_color(false),
        Color::from_rgba(0, 255, 0, 255)
    );
}

#[test]
fn the_task_force_caption_spaces_its_number() {
    // A0 TAC-03-C016 reads "Task Force # 1".
    assert_eq!(tactical_task_force_caption(0), "Task Force # 1");
    assert_eq!(tactical_task_force_caption(7), "Task Force # 8");
}

#[test]
fn group_bar_slots_are_29_pixels_apart() {
    // A0 TAC-02-C002: task forces from x 60, squadrons from x 347.
    assert_eq!(TASK_FORCE_BUTTONS[1].x - TASK_FORCE_BUTTONS[0].x, 29.0);
    assert_eq!(TASK_FORCE_BUTTONS[7].x, 263.0);
    assert_eq!(FIGHTER_GROUP_BUTTONS[0].x, 347.0);
    assert_eq!(FIGHTER_GROUP_BUTTONS[3].x, 434.0);
}

#[test]
fn a_lit_side_draws_its_ships_boxes_red_or_green() {
    // FUN_0059f380 flags the lit side's units; A0 TAC-04-C006 shows a
    // pure red box around the Alliance ship, TAC-04-C007 and the Corsin
    // battle (a0-vm/research-display/32) pure green ones for the Empire.
    assert_eq!(tactical_highlight_color(true, (false, false)), None);
    assert_eq!(
        tactical_highlight_color(true, (true, false)),
        Some(Color::new(1.0, 0.0, 0.0, 1.0))
    );
    assert_eq!(tactical_highlight_color(false, (true, false)), None);
    assert_eq!(
        tactical_highlight_color(false, (false, true)),
        Some(Color::new(0.0, 1.0, 0.0, 1.0))
    );
    assert_eq!(tactical_highlight_color(true, (false, true)), None);
    let edges = crate::tactical_assets::highlight_box_edges(
        [
            vec3(-1.0, 0.0, 2.0),
            vec3(3.0, -2.0, 4.0),
            vec3(0.0, 1.0, 3.0),
        ]
        .into_iter(),
    )
    .unwrap();
    // Twelve edges of the box (-1..3, -2..1, 2..4), each along one axis.
    for (from, to) in edges {
        let delta = (to - from).abs();
        assert_eq!(
            [delta.x > 0.0, delta.y > 0.0, delta.z > 0.0]
                .iter()
                .filter(|moved| **moved)
                .count(),
            1
        );
        for corner in [from, to] {
            assert!([-1.0, 3.0].contains(&corner.x));
            assert!([-2.0, 1.0].contains(&corner.y));
            assert!([2.0, 4.0].contains(&corner.z));
        }
    }
    assert!(crate::tactical_assets::highlight_box_edges(std::iter::empty()).is_none());
}

#[test]
fn the_alliance_recovers_with_the_x_wing_art() {
    // A0 TAC-05-C021 (Alliance) shows 1170, an X-wing, at (568, 157);
    // TAC-05-C010 the pressed 1171 and TAC-05-C015 the disabled 1172.
    // 1173..1175 are a TIE.
    use resources::tactical as art;
    assert_eq!(
        tactical_command_spec(TacticalCommandControl::Recover, true).1,
        1170
    );
    assert_eq!(
        tactical_command_spec(TacticalCommandControl::Recover, false).1,
        1173
    );
    assert_eq!(
        (
            art::BTN_RECOVER_ALLIANCE_PRESSED,
            art::BTN_RECOVER_ALLIANCE_DISABLED
        ),
        (1171, 1172)
    );
    assert_eq!(
        (
            art::BTN_RECOVER_EMPIRE_PRESSED,
            art::BTN_RECOVER_EMPIRE_DISABLED
        ),
        (1174, 1175)
    );
}

#[test]
fn a_task_force_number_turns_white_while_its_button_is_down() {
    // FUN_0059be80: FUN_00603010(4, 0x2ffffff), white in state 4; A0
    // TAC-03-C001 (Task Force 1 selected) white, TAC-02-C008 black.
    assert_eq!(task_force_number_color(true), WHITE);
    assert_eq!(task_force_number_color(false), BLACK);
    assert_eq!(TASK_FORCE_NUMBER_CAP_TOP + TASK_FORCE_NUMBER.1, 15.0);
}

#[test]
fn squadron_buttons_are_two_rows_shorter_than_their_bitmaps() {
    // FUN_0059c030 creates them 0x19 x 0xf; A0 TAC-02-C012 shows the
    // backdrop 1000 in the 1020 bitmap's last two rows.
    for button in FIGHTER_GROUP_BUTTONS {
        assert_eq!((button.width, button.height), (25.0, 15.0));
    }
    assert!(!FIGHTER_GROUP_BUTTONS[0].contains(350.0, 17.5));
    assert!(FIGHTER_GROUP_BUTTONS[0].contains(350.0, 16.5));
    for button in TASK_FORCE_BUTTONS {
        assert_eq!((button.width, button.height), (25.0, 17.0));
    }
}

#[test]
fn camera_memorize_art_is_on_the_right_and_recall_on_the_left() {
    // A0 TAC-04-C005: bitmap 1040 sits at (594, 444) under the tooltip
    // "Remember Camera Position"; 1042 at (486, 444) recalls the view.
    use resources::tactical as art;
    assert_eq!(art::BTN_CAMERA_MEMORIZE_NORMAL, 1040);
    assert_eq!(art::BTN_CAMERA_RECALL_NORMAL, 1042);
    assert_eq!(
        tactical_hud_rect_control_at(600.0, 450.0),
        Some(TacticalHudControl::CameraMemorize)
    );
    assert_eq!(
        tactical_hud_rect_control_at(492.0, 450.0),
        Some(TacticalHudControl::CameraRecall)
    );
}

// Source: positions of the lower HUD bitmaps in A0 TAC-04-C005.
#[test]
fn original_tactical_control_rects_stop_at_bitmap_edges() {
    assert_eq!(
        tactical_hud_rect_control_at(561.0, 308.0),
        Some(TacticalHudControl::Pause)
    );
    assert_eq!(
        tactical_hud_rect_control_at(588.9, 328.9),
        Some(TacticalHudControl::Pause)
    );
    assert_eq!(tactical_hud_rect_control_at(589.0, 318.0), None);
    assert_eq!(tactical_hud_rect_control_at(560.9, 318.0), None);
    assert_eq!(
        tactical_hud_rect_control_at(491.0, 343.0),
        Some(TacticalHudControl::ZoomIn)
    );
    assert_eq!(tactical_hud_rect_control_at(515.0, 355.0), None);
    assert_eq!(
        tactical_hud_rect_control_at(607.0, 343.0),
        Some(TacticalHudControl::ZoomOut)
    );
    assert_eq!(
        tactical_hud_rect_control_at(515.0, 305.0),
        Some(TacticalHudControl::HighlightAlliance)
    );
    assert_eq!(
        tactical_hud_rect_control_at(483.0, 305.0),
        Some(TacticalHudControl::HighlightEmpire)
    );
    assert_eq!(
        tactical_hud_rect_control_at(485.0, 272.0),
        Some(TacticalHudControl::NavigationSet(0))
    );
    assert_eq!(
        tactical_hud_rect_control_at(628.9, 298.9),
        Some(TacticalHudControl::NavigationSet(3))
    );
    assert_eq!(tactical_hud_rect_control_at(629.0, 285.0), None);
    assert_eq!(
        tactical_hud_rect_control_at(486.0, 444.0),
        Some(TacticalHudControl::CameraRecall)
    );
    assert_eq!(
        tactical_hud_rect_control_at(636.9, 468.9),
        Some(TacticalHudControl::CameraMemorize)
    );
    assert_eq!(tactical_hud_rect_control_at(637.0, 456.0), None);
}

// Source: FUN_005f0f40 / FUN_005f1050 bitmap hit masks, at the bitmap
// positions of A0 TAC-04-C005 and TAC-06-C001.
#[test]
#[ignore = "requires extracted original bitmaps in data/base/ui"]
fn original_tactical_controls_use_source_bitmap_hit_masks() {
    let mut cache = BmpCache::new();
    cache.set_base_path(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base/ui"),
    );

    // Original control hit testing excludes every outer edge, even when
    // the containing rectangle starts there.
    assert_eq!(tactical_hud_control_at(&mut cache, 561.0, 308.0), None);
    assert_eq!(
        tactical_hud_control_at(&mut cache, 563.1, 309.1),
        Some(TacticalHudControl::Pause)
    );

    // Both zoom controls contain transparent matte pixels inside their
    // bounding rectangles. Those pixels must pass through untouched.
    assert_eq!(tactical_hud_control_at(&mut cache, 492.1, 344.1), None);
    assert_eq!(
        tactical_hud_control_at(&mut cache, 496.1, 344.1),
        Some(TacticalHudControl::ZoomIn)
    );
    assert_eq!(tactical_hud_control_at(&mut cache, 608.1, 344.1), None);
    assert_eq!(
        tactical_hud_control_at(&mut cache, 613.1, 344.1),
        Some(TacticalHudControl::ZoomOut)
    );
    assert_eq!(
        tactical_hud_control_at(&mut cache, 506.1, 402.1),
        Some(TacticalHudControl::CameraLeft)
    );
    assert_eq!(
        tactical_hud_control_at(&mut cache, 615.1, 402.1),
        Some(TacticalHudControl::CameraRight)
    );
    assert_eq!(
        tactical_hud_control_at(&mut cache, 560.1, 348.1),
        Some(TacticalHudControl::CameraUp)
    );
    assert_eq!(
        tactical_hud_control_at(&mut cache, 561.1, 456.1),
        Some(TacticalHudControl::CameraDown)
    );

    assert_eq!(
        tactical_hud_control_at(&mut cache, 516.1, 306.1),
        Some(TacticalHudControl::HighlightAlliance)
    );
    assert_eq!(
        tactical_hud_control_at(&mut cache, 484.1, 306.1),
        Some(TacticalHudControl::HighlightEmpire)
    );
    assert_eq!(tactical_hud_control_at(&mut cache, 589.0, 318.0), None);
    assert_eq!(
        tactical_hud_control_at(&mut cache, 618.0, 319.0),
        Some(TacticalHudControl::BattleOptions)
    );

    const OPEN_OPTIONS: BattleOptionsGates = BattleOptionsGates {
        withdraw: true,
        simulate: true,
        observe: true,
    };
    const NO_WITHDRAW: BattleOptionsGates = BattleOptionsGates {
        withdraw: false,
        ..OPEN_OPTIONS
    };
    assert_eq!(
        tactical_battle_options_control_at(&mut cache, OPEN_OPTIONS, 524.0, 64.0),
        Some(TacticalBattleOptionsControl::Withdraw)
    );
    assert_eq!(
        tactical_battle_options_control_at(&mut cache, NO_WITHDRAW, 524.0, 64.0),
        None
    );
    assert_eq!(
        tactical_battle_options_control_at(&mut cache, OPEN_OPTIONS, 518.0, 133.0),
        Some(TacticalBattleOptionsControl::SimulateRemainder)
    );
    assert_eq!(
        tactical_battle_options_control_at(&mut cache, OPEN_OPTIONS, 591.0, 133.0),
        Some(TacticalBattleOptionsControl::ObserveToggle)
    );
    let withdrawing = BattleOptionsGates {
        withdraw: false,
        simulate: false,
        observe: false,
    };
    assert_eq!(
        tactical_battle_options_control_at(&mut cache, withdrawing, 518.0, 133.0),
        None
    );
    assert_eq!(
        tactical_battle_options_control_at(&mut cache, withdrawing, 591.0, 133.0),
        None
    );
    assert_eq!(
        tactical_battle_options_control_at(&mut cache, OPEN_OPTIONS, 531.0, 228.0),
        Some(TacticalBattleOptionsControl::GameOptions)
    );
    assert_eq!(
        tactical_battle_options_control_at(&mut cache, OPEN_OPTIONS, 593.0, 228.0),
        Some(TacticalBattleOptionsControl::Close)
    );
    assert_eq!(
        tactical_withdraw_confirmation_control_at(&mut cache, 561.0, 243.0),
        Some(TacticalWithdrawConfirmationControl::Confirm)
    );
    assert_eq!(
        tactical_withdraw_confirmation_control_at(&mut cache, 601.0, 243.0),
        Some(TacticalWithdrawConfirmationControl::Cancel)
    );
    assert_eq!(
        tactical_withdraw_confirmation_control_at(&mut cache, 548.0, 230.0),
        None
    );
    assert_eq!(
        tactical_withdraw_confirmation_control_at(&mut cache, 615.0, 243.0),
        None
    );

    assert_eq!(
        tactical_hud_control_at(&mut cache, 72.1, 10.1),
        Some(TacticalHudControl::TaskForce(0))
    );
    assert_eq!(
        tactical_hud_control_at(&mut cache, 275.1, 10.1),
        Some(TacticalHudControl::TaskForce(7))
    );
    assert_eq!(tactical_hud_control_at(&mut cache, 60.1, 2.1), None);
    // A0 TAC-02-C002: the old 26 px spacing's slot 2 (86..111) now
    // falls between slots 1 and 2 (60..85, 89..114).
    assert_eq!(tactical_hud_control_at(&mut cache, 87.1, 10.1), None);
    assert_eq!(
        tactical_hud_control_at(&mut cache, 359.1, 10.1),
        Some(TacticalHudControl::FighterGroup(0))
    );
    assert_eq!(
        tactical_hud_control_at(&mut cache, 446.1, 10.1),
        Some(TacticalHudControl::FighterGroup(3))
    );
}

// Source: FUN_005f0c20 builds TACTICAL 1310, FUN_005f0f40 places confirm and
// cancel, and FUN_005f1170 dispatches the withdrawal on confirm (P58-B13).
#[test]
fn withdrawal_requires_native_confirmation_before_dispatch() {
    let mut state = TacticalState {
        session: Some(test_session(Vec::new(), Vec::new(), true)),
        ..TacticalState::default()
    };
    state.battle_options_open = true;

    assert_eq!(
        activate_tactical_battle_options(&mut state, TacticalBattleOptionsControl::Withdraw,),
        TacticalAction::None
    );
    assert!(!state.battle_options_open);
    assert!(state.withdraw_confirmation_open);

    assert_eq!(
        activate_tactical_withdraw_confirmation(
            &mut state,
            TacticalWithdrawConfirmationControl::Cancel,
        ),
        TacticalAction::None
    );
    assert!(!state.withdraw_confirmation_open);
    assert!(state
        .session
        .as_ref()
        .unwrap()
        .pending_voice_cues
        .is_empty());

    state.battle_options_open = true;
    assert_eq!(
        activate_tactical_battle_options(&mut state, TacticalBattleOptionsControl::Withdraw,),
        TacticalAction::None
    );
    assert_eq!(
        activate_tactical_withdraw_confirmation(
            &mut state,
            TacticalWithdrawConfirmationControl::Confirm,
        ),
        TacticalAction::WithdrawFromBattle
    );
    assert!(!state.withdraw_confirmation_open);
    assert_eq!(
        state.session.as_mut().unwrap().take_pending_voice_cues(),
        vec![TacticalVoiceCue::withdrawal_started(
            TacticalVoiceFaction::Alliance
        )]
    );

    let mut operational = test_ship(64, 0, false, true);
    operational.is_attacker = false;
    operational.subsystem_capacity.hyperdrive = 1;
    operational.refresh_subsystem_condition();
    let mut stranded = test_ship(64, 1, false, true);
    stranded.is_attacker = false;
    stranded.subsystem_capacity.hyperdrive = 1;
    stranded.subsystem_damage.hyperdrive = 1;
    stranded.refresh_subsystem_condition();
    let mut empire_state = TacticalState {
        session: Some(test_session(vec![operational, stranded], Vec::new(), false)),
        withdraw_confirmation_open: true,
        ..TacticalState::default()
    };
    assert_eq!(
        activate_tactical_withdraw_confirmation(
            &mut empire_state,
            TacticalWithdrawConfirmationControl::Confirm,
        ),
        TacticalAction::WithdrawFromBattle
    );
    assert_eq!(
        empire_state
            .session
            .as_mut()
            .unwrap()
            .take_pending_voice_cues(),
        vec![
            TacticalVoiceCue::withdrawal_started(TacticalVoiceFaction::Empire),
            TacticalVoiceCue::withdrawal_hyperdrive_warning(TacticalVoiceFaction::Empire),
        ]
    );
}

// Source: REBDLG.DLL resource IDs 10710-10973 battle alert frames.
#[test]
fn battle_alert_skins_match_recovered_faction_resources_and_geometry() {
    let alliance = battle_alert_skin(true);
    assert_eq!(alliance.frame, 10_710);
    assert_eq!(alliance.scene, 10_712);
    assert_eq!(alliance.rail, 10_820);
    assert_eq!(alliance.tabs[0], [10_728, 10_729]);
    assert_eq!(alliance.tabs[3], [10_734, 10_735]);
    // FUN_0044f860: side 1's controls are 0x2ade.., 0x29e2.., 0x29e5..
    // (A0 TAC-01-C010 and battle-clean/10-alert-averam, Alliance:
    // 10974, 10722, 10725; battle-alert-empire/00, Empire: 10971,
    // 10716, 10719).
    assert_eq!(alliance.buttons[0], [10_974, 10_975, 10_976]);
    assert_eq!(alliance.buttons[1], [10_722, 10_723, 10_724]);
    assert_eq!(alliance.buttons[2], [10_725, 10_726, 10_727]);
    assert_eq!(
        alliance.tab_rects,
        [
            NativeRect::new(418.0, 21.0, 41.0, 41.0),
            NativeRect::new(418.0, 81.0, 41.0, 41.0),
            NativeRect::new(418.0, 143.0, 41.0, 41.0),
            NativeRect::new(418.0, 205.0, 41.0, 41.0),
        ]
    );

    let empire = battle_alert_skin(false);
    assert_eq!(empire.frame, 10_711);
    assert_eq!(empire.scene, 10_713);
    // FUN_0044f860 / FUN_00450660: the list tabs' paler scene.
    assert_eq!(empire.list_scene, 10_715);
    assert_eq!(alliance.list_scene, 10_714);
    assert_eq!(
        battle_alert_scene(&alliance, BattleAlertTab::Summary),
        10_712
    );
    for tab in [
        BattleAlertTab::AllianceForces,
        BattleAlertTab::ImperialForces,
        BattleAlertTab::System,
    ] {
        assert_eq!(battle_alert_scene(&empire, tab), 10_715);
    }
    assert_eq!(battle_alert_scene(&empire, BattleAlertTab::Summary), 10_713);
    assert_eq!(empire.rail, 10_821);
    assert_eq!(empire.tabs[0], [10_738, 10_739]);
    assert_eq!(empire.tabs[3], [10_744, 10_745]);
    assert_eq!(empire.buttons[0], [10_971, 10_972, 10_973]);
    assert_eq!(empire.buttons[1], [10_716, 10_717, 10_718]);
    assert_eq!(empire.buttons[2], [10_719, 10_720, 10_721]);
    assert_eq!(
        empire.tab_rects,
        [
            NativeRect::new(426.0, 17.0, 44.0, 41.0),
            NativeRect::new(426.0, 80.0, 44.0, 41.0),
            NativeRect::new(426.0, 143.0, 44.0, 41.0),
            NativeRect::new(426.0, 206.0, 44.0, 41.0),
        ]
    );
}

#[test]
fn the_alert_frame_lies_over_the_scene() {
    // A0 TAC-01-C010 (window (62, 49)): frame 10710's pixels inside the
    // aperture show the frame over list scene 10714 (5931 of them).
    let alliance = battle_alert_skin(true);
    let layers = battle_alert_layers(&alliance, BattleAlertTab::AllianceForces);
    assert_eq!(
        layers.map(|(id, x, y, _)| (id, x, y)),
        [(10_714, 12.0, 13.0), (10_710, 0.0, 0.0)]
    );
    assert_eq!(layers[0].3, Some(Rect::new(0.0, 0.0, 400.0, 310.0)));
    assert_eq!(layers[1].3, None);
    let empire = battle_alert_skin(false);
    assert_eq!(
        battle_alert_layers(&empire, BattleAlertTab::Summary).map(|(id, ..)| id),
        [10_713, 10_711]
    );
}

// Source: manual pages 143 and 148, task-force and fighter-group selection (P58E).
#[test]
fn task_force_and_fighter_group_selection_are_mutually_exclusive() {
    let mut ships = (0..8)
        .map(|index| test_ship(64, index, true, true))
        .collect::<Vec<_>>();
    ships[0].selected = true;
    let mut fighters = (0..4)
        .map(|index| {
            let mut fighter = test_fighter(1, true);
            fighter.fighter_group = index;
            fighter.identity.fleet_roster_index = usize::from(index);
            fighter.fleet_fighter_index = usize::from(index);
            fighter
        })
        .collect::<Vec<_>>();
    fighters.push(test_fighter(5, false));
    let mut session = test_session(ships, fighters, true);
    session.selected_ship = Some(0);

    assert_eq!(select_task_force(&mut session, 7), 1);
    assert_eq!(selected_player_task_force(&session), Some(7));
    assert_eq!(session.selected_ship, Some(7));

    assert!(select_fighter_group(&mut session, 3));
    assert_eq!(session.selected_ship, None);
    assert_eq!(session.selected_fighter_group, Some(3));
    assert!(session.ships.iter().all(|ship| !ship.selected));
    assert_eq!(
        session
            .fighters
            .iter()
            .filter(|fighter| fighter.selected)
            .count(),
        1
    );

    assert_eq!(select_task_force(&mut session, 0), 1);
    assert_eq!(session.selected_fighter_group, None);
    assert!(session.fighters.iter().all(|fighter| !fighter.selected));
}

// Source: manual pages 143 and 148, the Ctrl task-force assignment rule (P58E).
#[test]
fn task_force_assignment_accepts_only_an_existing_or_next_blank_slot() {
    let mut ships = vec![
        test_ship(64, 0, true, true),
        test_ship(65, 1, true, true),
        test_ship(128, 0, false, true),
    ];
    ships[0].selected = true;
    ships[1].selected = true;
    let mut session = test_session(ships, Vec::new(), true);

    assert_eq!(assign_selected_to_task_force(&mut session, 4), 0);
    assert_eq!(session.ships[0].task_force, 0);
    assert_eq!(session.ships[1].task_force, 1);
    assert_eq!(assign_selected_to_task_force(&mut session, 2), 2);
    assert_eq!(session.ships[0].task_force, 2);
    assert_eq!(session.ships[1].task_force, 2);
}

#[test]
fn selected_capital_navigation_wraps_within_the_current_task_force() {
    let mut ships = vec![
        test_ship(64, 0, true, true),
        test_ship(65, 1, true, true),
        test_ship(66, 2, true, true),
        test_ship(128, 0, false, true),
    ];
    for ship in &mut ships[..3] {
        ship.task_force = 2;
        ship.selected = true;
    }
    let mut session = test_session(ships, vec![test_fighter(1, true)], true);
    session.selected_ship = Some(0);

    assert_eq!(cycle_selected_capital(&mut session, -1), Some((0, 2, 2)));
    assert_eq!(session.selected_ship, Some(2));
    assert_eq!(
        session
            .ships
            .iter()
            .enumerate()
            .filter_map(|(index, ship)| ship.selected.then_some(index))
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(cycle_selected_capital(&mut session, 1), Some((2, 0, 2)));
    assert_eq!(cycle_selected_capital(&mut session, 1), Some((0, 1, 2)));
    assert!(session.fighters.iter().all(|fighter| !fighter.selected));
}

// Source: FUN_005cf190 / FUN_005cf8f0 complete a reached waypoint and clear its
// movement intent (P58F11).
#[test]
fn ordered_navigation_route_advances_without_losing_the_next_waypoint() {
    let first = TacticalWorldPosition {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    let second = TacticalWorldPosition {
        x: 0.5,
        y: 0.0,
        z: 0.5,
    };
    let mut ship = test_ship(64, 0, true, true);
    ship.subsystem_capacity.engines = true;
    ship.source_waypoint = Some(first);
    ship.navigation_route = vec![first, second];
    let mut session = test_session(vec![ship], Vec::new(), true);

    // Both points lie within the route's arrival radius 1.0 of the
    // anchor (0x005eb1a0), so each is reached where the force stands.
    session.advance_original_tactical_movement(10.0);
    assert_eq!(
        session.ships[0].source_position,
        TacticalWorldPosition::ORIGIN
    );
    assert_eq!(session.ships[0].source_waypoint, Some(second));
    assert_eq!(session.ships[0].navigation_route, vec![second]);

    session.advance_original_tactical_movement(10.0);
    assert_eq!(
        session.ships[0].source_position,
        TacticalWorldPosition::ORIGIN
    );
    assert_eq!(session.ships[0].source_waypoint, None);
    assert!(session.ships[0].navigation_route.is_empty());
}

// Source: FUN_005d0b10 / FUN_005d0bb0 manual target list.
#[test]
fn ordered_manual_targets_advance_and_clear_as_hostiles_are_destroyed() {
    let mut player = test_ship(64, 0, true, true);
    player.manual_targets = vec![
        TacticalAttackTarget::CapitalShip(1),
        TacticalAttackTarget::CapitalShip(2),
    ];
    player.attack_target = player.manual_targets.first().copied();
    let mut session = test_session(
        vec![
            player,
            test_ship(128, 0, false, true),
            test_ship(129, 1, false, true),
        ],
        Vec::new(),
        true,
    );

    session.ships[1].alive = false;
    refresh_original_attack_targets(&mut session);
    assert_eq!(
        session.ships[0].manual_targets,
        vec![TacticalAttackTarget::CapitalShip(2)]
    );
    assert_eq!(
        session.ships[0].attack_target,
        Some(TacticalAttackTarget::CapitalShip(2))
    );

    session.ships[2].alive = false;
    refresh_original_attack_targets(&mut session);
    assert!(session.ships[0].manual_targets.is_empty());
    assert_eq!(session.ships[0].attack_target, None);
}

#[test]
fn alliance_mixed_task_forces_reject_focus_target_without_mutating_orders() {
    // FUN_005a24d0 queues source event 0x84 when the selected Alliance
    // capitals cannot resolve to one task-force ordinal.
    let mut first = test_ship(64, 0, true, true);
    first.selected = true;
    first.task_force = 1;
    first.manual_targets = vec![TacticalAttackTarget::CapitalShip(3)];
    first.attack_target = first.manual_targets.first().copied();
    first.escort_target = Some(3);
    let mut second = test_ship(65, 1, true, true);
    second.selected = true;
    second.task_force = 2;
    second.manual_targets = vec![TacticalAttackTarget::CapitalShip(4)];
    second.attack_target = second.manual_targets.first().copied();
    second.escort_target = Some(4);
    let hostile = test_ship(128, 2, false, true);
    let mut session = test_session(vec![first, second, hostile], Vec::new(), true);
    let before = session.ships[..2]
        .iter()
        .map(|ship| {
            (
                ship.manual_targets.clone(),
                ship.attack_target,
                ship.escort_target,
            )
        })
        .collect::<Vec<_>>();

    let result = assign_selected_capital_focus_target(&mut session, 2, false);

    assert_eq!(
        result,
        TacticalFocusAssignment::RejectedMixedTaskForces { member_count: 2 }
    );
    assert_eq!(
        session.ships[..2]
            .iter()
            .map(|ship| (
                ship.manual_targets.clone(),
                ship.attack_target,
                ship.escort_target
            ))
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::target_task_force_rejected(
            TacticalVoiceFaction::Alliance
        )]
    );
}

#[test]
fn imperial_mixed_task_forces_reject_focus_target_without_mutating_orders() {
    // FUN_005a24d0 queues source event 0x102 for the same rejected command
    // when the player-side tactical manager is Imperial.
    let hostile = test_ship(64, 0, true, true);
    let mut first = test_ship(128, 1, false, true);
    first.selected = true;
    first.task_force = 3;
    first.manual_targets = vec![TacticalAttackTarget::CapitalShip(0)];
    first.attack_target = first.manual_targets.first().copied();
    let mut second = test_ship(129, 2, false, true);
    second.selected = true;
    second.task_force = 4;
    second.escort_target = Some(0);
    let mut session = test_session(vec![hostile, first, second], Vec::new(), false);
    let before = session.ships[1..]
        .iter()
        .map(|ship| {
            (
                ship.manual_targets.clone(),
                ship.attack_target,
                ship.escort_target,
            )
        })
        .collect::<Vec<_>>();

    let result = assign_selected_capital_focus_target(&mut session, 0, true);

    assert_eq!(
        result,
        TacticalFocusAssignment::RejectedMixedTaskForces { member_count: 2 }
    );
    assert_eq!(
        session.ships[1..]
            .iter()
            .map(|ship| (
                ship.manual_targets.clone(),
                ship.attack_target,
                ship.escort_target
            ))
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::target_task_force_rejected(
            TacticalVoiceFaction::Empire
        )]
    );
}

// Source: FUN_005a24d0 focus target constraint.
#[test]
fn one_task_force_assigns_focus_target_without_rejection_voice() {
    let mut first = test_ship(64, 0, true, true);
    first.selected = true;
    first.task_force = 5;
    let mut second = test_ship(65, 1, true, true);
    second.selected = true;
    second.task_force = 5;
    let hostile = test_ship(128, 2, false, true);
    let mut session = test_session(vec![first, second, hostile], Vec::new(), true);

    let result = assign_selected_capital_focus_target(&mut session, 2, false);

    assert_eq!(
        result,
        TacticalFocusAssignment::Assigned {
            object_ids: vec![1, 2],
            maximum_targets: 1,
        }
    );
    assert!(session.ships[..2].iter().all(|ship| {
        ship.manual_targets == vec![TacticalAttackTarget::CapitalShip(2)]
            && ship.attack_target == Some(TacticalAttackTarget::CapitalShip(2))
    }));
    assert!(session.take_pending_voice_cues().is_empty());
}

// Source: FUN_005a24d0 focus target toggle.
#[test]
fn appending_an_existing_focus_target_toggles_it_off() {
    let mut friendly = test_ship(64, 0, true, true);
    friendly.selected = true;
    friendly.task_force = 5;
    friendly.manual_targets = vec![TacticalAttackTarget::CapitalShip(1)];
    friendly.attack_target = friendly.manual_targets.first().copied();
    let hostile = test_ship(128, 1, false, true);
    let mut session = test_session(vec![friendly, hostile], Vec::new(), true);

    let result = assign_selected_capital_focus_target(&mut session, 1, true);

    assert_eq!(
        result,
        TacticalFocusAssignment::Assigned {
            object_ids: Vec::new(),
            maximum_targets: 0,
        }
    );
    assert!(session.ships[0].manual_targets.is_empty());
    assert_eq!(session.ships[0].attack_target, None);
    assert!(session.take_pending_voice_cues().is_empty());
}

#[test]
fn focus_target_with_no_selected_capitals_is_a_quiet_noop() {
    let friendly = test_ship(64, 0, true, true);
    let hostile = test_ship(128, 1, false, true);
    let mut session = test_session(vec![friendly, hostile], Vec::new(), true);
    let before = session.ships[0].manual_targets.clone();

    let result = assign_selected_capital_focus_target(&mut session, 1, false);

    assert_eq!(
        result,
        TacticalFocusAssignment::Assigned {
            object_ids: Vec::new(),
            maximum_targets: 0,
        }
    );
    assert_eq!(session.ships[0].manual_targets, before);
    assert!(session.take_pending_voice_cues().is_empty());
}

// Source: FUN_005ca6d0 dispatches order codes 0-11 (tactical order
// execution evidence, 2026-09-17); the tactic toggle at +0x38 uses Surround
// 1 and Stand Off 2 (tactical command assignment evidence, 2026-09-17).
#[test]
fn tactical_order_and_tactic_codes_match_the_recovered_source_domain() {
    let orders = [
        (TacticalOrder::None, 0),
        (TacticalOrder::Escort, 1),
        (TacticalOrder::Recover, 2),
        (TacticalOrder::AttackFighters, 4),
        (TacticalOrder::AttackCapitalShips, 5),
        (TacticalOrder::AttackDeathStar, 6),
        (TacticalOrder::LeftHook, 7),
        (TacticalOrder::RightHook, 8),
        (TacticalOrder::Hammer, 9),
        (TacticalOrder::Anvil, 10),
        (TacticalOrder::HoldPosition, 11),
    ];
    for (order, code) in orders {
        assert_eq!(order.source_code(), code);
    }
    assert_eq!(TacticalTactic::Surround.source_code(), 1);
    assert_eq!(TacticalTactic::StandOff.source_code(), 2);
}

// Source: FUN_005ca6d0 dispatches Escort as order code 1; FUN_005d0af0,
// FUN_005cf910 and FUN_005cf920 install and retain its target (P58-B14).
// FUN_005cc300 closes on it like an attack target; when it is destroyed
// the force latches +0xb0 and stands, and order 1 and its target stay
// (only 0x5cb600 / 0x5cb4d0 clear them).
#[test]
fn direct_escort_retains_friendly_target_even_when_it_is_destroyed() {
    let mut escort = test_ship(64, 0, true, true);
    escort.selected = true;
    let target = test_ship(65, 1, true, true);
    let enemy = test_ship(128, 0, false, true);
    let mut session = test_session(vec![escort, target, enemy], Vec::new(), true);
    session.selected_ship = Some(0);

    assert_eq!(assign_selected_escort(&mut session, 1), (1, 0));
    assert_eq!(session.ships[0].order, TacticalOrder::Escort);
    assert_eq!(session.ships[0].escort_target, Some(1));
    // The escorted ship in a force of its own, the escort's in Surround,
    // whose band (0 to 12.5) the port knows.
    session.ships[1].source_position.z = 60.0;
    session.ships[1].task_force = 1;
    session.task_force_formations[1][0] = TacticalTactic::Surround;
    session.update_engagement_bands();
    session.update_force_movers();
    session.advance_engagement_approach();
    assert!(session.ships[0].source_waypoint.is_some());

    session.ships[1].alive = false;
    session.ships[0].source_waypoint = None;
    refresh_original_escort_targets(&mut session);
    session.advance_engagement_approach();
    assert_eq!(session.ships[0].order, TacticalOrder::Escort);
    assert_eq!(session.ships[0].escort_target, Some(1));
    assert_eq!(session.ships[0].source_waypoint, None);
}

// Source: FUN_0040bcd0 counts the system's usable family 0x25 facilities
// (FUN_0051c270) and FUN_005af4e0 stores the answer at +0x70;
// FUN_005ba450 then refuses the fighters' attack (manual p. 124).
#[test]
fn a_death_star_shield_on_the_battle_system_bars_the_fighters_attack() {
    use rebellion_core::world::{DefenseFacilityInstance, Fleet, System};
    let mut world = GameWorld::default();
    let system = world.systems.insert(System {
        dat_id: DatId::new(0x9000_0001),
        name: "Endor".into(),
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        control: rebellion_core::world::ControlKind::Controlled(
            rebellion_core::dat::Faction::Empire,
        ),
        ..Default::default()
    });
    let mut fleet = |is_alliance: bool, has_death_star: bool| {
        world.fleets.insert(Fleet {
            location: system,
            is_alliance,
            has_death_star,
            ..Default::default()
        })
    };
    let rebels = fleet(true, false);
    let empire = fleet(false, true);
    let battery = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(0x2300_0002),
        side: rebellion_core::dat::Faction::Empire,
    });
    world.systems[system].defense_facilities.push(battery);

    let open = BattleSession::new(&world, system, rebels, empire, true, 1, 1);
    let death_star = open.death_star.expect("the Empire's Death Star");
    assert!(!death_star.shielded);
    assert!(death_star.attackable());
    // FUN_005af4e0 stores +0x6c = 100.0; A0 TAC-05-C017 opens with the
    // meter full and the control ready.
    assert_eq!(
        death_star.laser_charge,
        ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
    );

    // DEFFACSD 4, the Death Star shield (TEXTSTRA 8832).
    let shield = world.defense_facilities.insert(DefenseFacilityInstance {
        class_dat_id: DatId::new(0x2500_0004),
        side: rebellion_core::dat::Faction::Empire,
    });
    world.systems[system].defense_facilities.push(shield);

    let mut shielded = BattleSession::new(&world, system, rebels, empire, true, 1, 1);
    let death_star = shielded.death_star.expect("the Empire's Death Star");
    assert!(death_star.shielded);
    assert!(death_star.operational());
    assert!(!death_star.attackable());

    // The mission control (FUN_005ad930) needs a selected fighter group
    // and an enemy Death Star the fighters may attack.
    // One Alliance group on the field (FUN_005ad930 counts them).
    let mut open = open;
    open.fighters.push(test_fighter(1, true));
    shielded.fighters.push(test_fighter(1, true));
    assert!(!attack_death_star_enabled(&open));
    open.selected_fighter_group = Some(0);
    assert!(attack_death_star_enabled(&open));
    shielded.selected_fighter_group = Some(0);
    assert!(!attack_death_star_enabled(&shielded));
    let mut own = BattleSession::new(&world, system, rebels, empire, false, 1, 1);
    own.fighters.push(test_fighter(1, true));
    own.selected_fighter_group = Some(0);
    own.death_star.as_mut().unwrap().shielded = false;
    assert!(!attack_death_star_enabled(&own));
}

fn imperial_death_star_for_test(shielded: bool) -> TacticalDeathStar {
    TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 0.0,
        shielded,
        destroyed: false,
        action_committed: false,
    }
}

fn voice_resources(session: &mut BattleSession) -> Vec<u32> {
    session
        .take_pending_voice_cues()
        .into_iter()
        .map(|cue| cue.resource_id)
        .collect()
}

// Source: FUN_005ada10 (state 0 then 1000, DAT_0066c238 = 6000 ms, then
// state 4: 0x11c for the Empire when FUN_005ad930 opens). A0 TAC-01-C012
// (Empire, Death Star, more Alliance fighter groups): 15001 at 14.08 s,
// 15131 at 20.09 s, the battle still paused.
#[test]
fn the_empire_hears_its_fighter_screen_is_thin_six_seconds_after_battle_ready() {
    let mut session = test_session(
        Vec::new(),
        vec![
            test_fighter(1, true),
            test_fighter(2, true),
            test_fighter(5, false),
        ],
        false,
    );
    session.paused = true;
    session.death_star = Some(imperial_death_star_for_test(false));
    session.queue_battle_ready_voice();
    assert_eq!(voice_resources(&mut session), vec![15_001]);
    session.advance_voice_announcer(5_999.0);
    assert!(voice_resources(&mut session).is_empty());
    session.advance_voice_announcer(1.0);
    assert_eq!(voice_resources(&mut session), vec![15_131]);
    session.advance_voice_announcer(10_000.0);
    assert!(voice_resources(&mut session).is_empty());
}

// Source: FUN_005ada10's Alliance table 0x20, 0x90, 0x93, 0x94 (+0xe74)
// and state 1: shield (+0x70) -> 0x93, else no more Alliance than
// Imperial groups -> 0x94; then state 4: 0x92 when FUN_005ad930 opens,
// 0x94 when it closes.
#[test]
fn the_alliance_hears_the_death_star_warnings_in_order() {
    let mut session = test_session(
        Vec::new(),
        vec![test_fighter(1, true), test_fighter(5, false)],
        true,
    );
    session.death_star = Some(imperial_death_star_for_test(false));
    session.queue_battle_ready_voice();
    assert_eq!(voice_resources(&mut session), vec![14_001]);
    session.advance_voice_announcer(6_000.0);
    assert_eq!(voice_resources(&mut session), vec![14_113]);
    session.advance_voice_announcer(6_000.0);
    assert_eq!(voice_resources(&mut session), vec![14_117]);
    session.advance_voice_announcer(6_000.0);
    assert!(voice_resources(&mut session).is_empty());
    session.fighters.push(test_fighter(2, true));
    session.advance_voice_announcer(16.0);
    assert_eq!(voice_resources(&mut session), vec![14_115]);
    session.fighters[2].alive = false;
    session.advance_voice_announcer(16.0);
    assert_eq!(voice_resources(&mut session), vec![14_117]);

    let mut shielded = test_session(Vec::new(), vec![test_fighter(1, true)], true);
    shielded.death_star = Some(imperial_death_star_for_test(true));
    shielded.queue_battle_ready_voice();
    shielded.advance_voice_announcer(6_000.0);
    shielded.advance_voice_announcer(6_000.0);
    assert_eq!(voice_resources(&mut shielded), vec![14_001, 14_113, 14_116]);

    // Without a Death Star the Alliance goes straight to waiting.
    let mut plain = test_session(Vec::new(), vec![test_fighter(1, true)], true);
    plain.queue_battle_ready_voice();
    plain.advance_voice_announcer(12_000.0);
    assert_eq!(voice_resources(&mut plain), vec![14_001]);
}

// Source: FUN_005ada10's guards: state 0x11c needs the Empire's own
// groups; state 1 gives 0x94 only with no more Alliance than Imperial
// groups, the window shut and no run; closing gives 0x94 only with
// groups on both sides, no run and no player group on order 6.
#[test]
fn the_death_star_announcer_keeps_quiet_when_its_conditions_fail() {
    // The Empire without groups of its own hears nothing at 6 s.
    let mut empire = test_session(
        Vec::new(),
        vec![test_fighter(1, true), test_fighter(2, true)],
        false,
    );
    empire.death_star = Some(imperial_death_star_for_test(false));
    empire.queue_battle_ready_voice();
    empire.advance_voice_announcer(6_000.0);
    assert_eq!(voice_resources(&mut empire), vec![15_001]);

    // The Alliance with more groups: 0x90, then straight to waiting.
    let mut ahead = test_session(
        Vec::new(),
        vec![
            test_fighter(1, true),
            test_fighter(2, true),
            test_fighter(5, false),
        ],
        true,
    );
    ahead.death_star = Some(imperial_death_star_for_test(false));
    ahead.queue_battle_ready_voice();
    ahead.advance_voice_announcer(5_999.0);
    assert_eq!(voice_resources(&mut ahead), vec![14_001]);
    ahead.advance_voice_announcer(1.0);
    assert_eq!(voice_resources(&mut ahead), vec![14_113]);
    ahead.advance_voice_announcer(6_000.0);
    // Watching from the start of the window: it is already open.
    assert_eq!(voice_resources(&mut ahead), vec![14_115]);

    // State 1 with more Alliance groups, the window shut only by a group
    // already on order 6: the counts still forbid 0x94.
    let mut busy = test_session(
        Vec::new(),
        vec![
            test_fighter(1, true),
            test_fighter(2, true),
            test_fighter(5, false),
        ],
        true,
    );
    busy.death_star = Some(imperial_death_star_for_test(false));
    busy.fighters[0].order = TacticalOrder::AttackDeathStar;
    busy.queue_battle_ready_voice();
    busy.advance_voice_announcer(6_000.0);
    busy.advance_voice_announcer(6_000.0);
    assert_eq!(voice_resources(&mut busy), vec![14_001, 14_113]);

    // Two groups a side: no more Alliance than Imperial, so 0x94.
    let mut even = test_session(
        Vec::new(),
        vec![
            test_fighter(1, true),
            test_fighter(2, true),
            test_fighter(5, false),
            test_fighter(6, false),
        ],
        true,
    );
    even.death_star = Some(imperial_death_star_for_test(false));
    even.queue_battle_ready_voice();
    even.advance_voice_announcer(6_000.0);
    even.advance_voice_announcer(6_000.0);
    assert_eq!(voice_resources(&mut even), vec![14_001, 14_113, 14_117]);

    // State 1 with a run under way or no player groups: no 0x94.
    for setup in 0..2 {
        let mut session = test_session(
            Vec::new(),
            vec![test_fighter(1, true), test_fighter(5, false)],
            true,
        );
        session.death_star = Some(imperial_death_star_for_test(false));
        if setup == 0 {
            session.trench_run_remaining_ms = Some(1_000.0);
        } else {
            session.fighters[0].alive = false;
        }
        session.queue_battle_ready_voice();
        session.advance_voice_announcer(6_000.0);
        session.advance_voice_announcer(6_000.0);
        assert_eq!(
            voice_resources(&mut session),
            vec![14_001, 14_113],
            "setup {setup}"
        );
    }

    // Closing without enemy groups, or with a player group on order 6:
    // no 0x94.
    for setup in 0..2 {
        let mut session = test_session(
            Vec::new(),
            vec![
                test_fighter(1, true),
                test_fighter(2, true),
                test_fighter(5, false),
            ],
            true,
        );
        session.death_star = Some(imperial_death_star_for_test(false));
        session.voice_announcer.state = TacticalVoiceAnnouncerState::Watching;
        session.advance_voice_announcer(16.0);
        assert_eq!(voice_resources(&mut session), vec![14_115]);
        if setup == 0 {
            session.fighters[2].alive = false;
            session.fighters[1].alive = false;
            session.fighters[0].alive = false;
        } else {
            session.fighters[0].order = TacticalOrder::AttackDeathStar;
        }
        session.advance_voice_announcer(16.0);
        assert!(voice_resources(&mut session).is_empty(), "setup {setup}");
    }
}

// Source: FUN_005bae60's +0x50 table (4, 4, 4, 2, 2, 2, 1, 1, 1, then 3
// for events 9..0x13 and 1 for 0x14..0x1f). A0 TAC-01-C012's recording
// has event 0x0e (an ion hit the shield holds) in variants 0, 1 and 2.
#[test]
fn every_tactical_sound_event_has_its_original_number_of_variants() {
    assert_eq!(original_audio_variant_count(0x00), 4);
    assert_eq!(original_audio_variant_count(0x02), 4);
    assert_eq!(original_audio_variant_count(0x03), 2);
    assert_eq!(original_audio_variant_count(0x05), 2);
    assert_eq!(original_audio_variant_count(0x06), 1);
    assert_eq!(original_audio_variant_count(0x08), 1);
    assert_eq!(original_audio_variant_count(0x09), 3);
    assert_eq!(original_audio_variant_count(0x13), 3);
    assert_eq!(original_audio_variant_count(0x14), 1);
    assert_eq!(original_audio_variant_count(0x1f), 1);
    assert_eq!(TacticalAudioEvent::Turbolaser.variant_count(), 4);
    assert_eq!(TacticalAudioEvent::IonAtFighter.variant_count(), 2);
    assert_eq!(TacticalAudioEvent::IonShieldHit.variant_count(), 3);
    assert_eq!(TacticalAudioEvent::FighterJump.variant_count(), 1);
    assert_eq!(TacticalAudioEvent::CapitalJump.variant_count(), 1);
}

// Source: FUN_005a6b40: (vtable +0x24 + id +0x3b) & 3 picks
// DAT_006bc47c, DAT_006bc488, DAT_006bc484 or DAT_006bc494.
#[test]
fn a_jump_factor_follows_the_unit_type_and_its_number() {
    assert_eq!(original_jump_factor_index(0, 0), 0);
    assert_eq!(original_jump_factor_index(15, 0), 3);
    assert_eq!(original_jump_factor_index(15, 2), 1);
    assert_eq!(original_jump_factor_index(29, 3), 0);
    assert_eq!(original_jump_factor_index(255, 1), 0);
}

// Source: FUN_005ad930. The manual (p. 151): more Alliance than
// Imperial fighters launched, and one group at a time.
#[test]
fn attack_death_star_needs_more_alliance_fighters_and_no_run_under_way() {
    let mut session = test_session(
        Vec::new(),
        vec![test_fighter(1, true), test_fighter(2, false)],
        true,
    );
    session.selected_fighter_group = Some(0);
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });
    // One each: not more Alliance than Imperial.
    assert!(!attack_death_star_enabled(&session));
    session.fighters.push(test_fighter(3, true));
    assert!(attack_death_star_enabled(&session));
    // A lost Alliance group no longer counts.
    session.fighters[2].alive = false;
    assert!(!attack_death_star_enabled(&session));
    session.fighters[2].alive = true;
    // A run under way.
    session.trench_run_remaining_ms = Some(1_000.0);
    assert!(!attack_death_star_enabled(&session));
    session.trench_run_remaining_ms = None;
    // One of the player's groups already on the order.
    session.fighters[0].order = TacticalOrder::AttackDeathStar;
    assert!(!attack_death_star_enabled(&session));
    // An enemy group's order does not count.
    session.fighters[0].order = TacticalOrder::None;
    session.fighters[1].order = TacticalOrder::AttackDeathStar;
    assert!(attack_death_star_enabled(&session));
}

// Source: FUN_005c4ed0 (control at 0x89,2; meter 0x5f wide, range 100);
// A0 TAC-05-C017/C018/C019.
#[test]
fn death_star_laser_control_sits_at_the_a0_position_and_shows_loading_once_committed() {
    assert_eq!(DEATH_STAR_LASER_BUTTON_RECT.x, 611.0);
    assert_eq!(DEATH_STAR_LASER_BUTTON_RECT.y, 2.0);
    assert_eq!(
        death_star_laser_control_resource(true, false),
        resources::tactical::DEATH_STAR_LASER_READY
    );
    assert_eq!(
        death_star_laser_control_resource(true, true),
        resources::tactical::DEATH_STAR_LASER_FIRED
    );
    // A committed shot has reset the charge: not ready and not armed.
    assert_eq!(
        death_star_laser_control_resource(false, false),
        resources::tactical::DEATH_STAR_LASER_LOADING
    );
}

// Source: FUN_005d4d10 (DAT_0066d244 0.2, DAT_0066d248 -2.0,
// DAT_0066d250 0.09, DAT_0066d258 0.11, DAT_0066d260 0.5). Its
// SetPosition passes -2e as x (0x005d5598), and A0 TAC-03-C020 shows the
// decal where the camera sees -x. Class_0066d278_vt8 (0x005d5970) adds
// the decal to the frame +0x50; FUN_005d59e0 builds the beam on the
// child +0x4c.
#[test]
fn the_death_star_hangs_behind_the_battle_opposite_the_planet() {
    let (center, size) = death_star_backdrop_source(400.0, (0.0, 0.0));
    assert!((center.x + 800.0).abs() < 1e-3);
    assert!(center.y.abs() < 1e-3);
    assert!(center.z.abs() < 1e-3);
    assert_eq!(size, 200.0);
    let (planet, _) = planet_backdrop_source(400.0, (0.0, 0.0));
    assert_eq!(planet.x, -center.x);
    // The frame's random offsets move the decal and its dish child.
    let (moved, _) = death_star_backdrop_source(400.0, (-3.0, 5.0));
    assert!((moved.y + 3.0).abs() < 1e-3);
    assert!((moved.z - 5.0).abs() < 1e-3);
}

// A0 2026-11-18-edited-saves-8 l-07 (377f230464ae): the superlaser's
// star and line leave above the Death Star sphere's centre.
#[test]
fn the_superlaser_leaves_from_the_dish_child_above_the_decal() {
    let dish = death_star_dish_source(400.0, (-3.0, 5.0));
    assert!((dish.x + 800.0).abs() < 1e-3);
    assert!((dish.y - 41.0).abs() < 1e-3);
    assert!((dish.z + 31.0).abs() < 1e-3);
}

#[test]
fn a_selection_frame_line_sits_on_whole_pixels() {
    let one = snap_line_to_pixels(macroquad::math::vec2(60.36, 438.87), 1.0);
    assert_eq!(one, macroquad::math::vec2(60.5, 438.5));
    let two = snap_line_to_pixels(macroquad::math::vec2(60.36, 438.87), 2.0);
    assert_eq!(two, macroquad::math::vec2(60.0, 439.0));
}

// Source: FUN_005d4d10 (DAT_0066d23c 0.1, SetPosition x = 2e at
// 0x005d540d, SetDecalSize(e, e)); A0 TAC-01-C008 shows it at +x.
#[test]
fn the_planet_hangs_two_extents_out_on_x_as_wide_as_the_battle() {
    let (center, size) = planet_backdrop_source(400.0, (0.0, 0.0));
    assert_eq!(center.x, 800.0);
    assert_eq!(center.y, 0.0);
    assert_eq!(center.z, 0.0);
    assert_eq!(size, 400.0);
    let (moved, _) = planet_backdrop_source(400.0, (-4.6, -5.6));
    assert_eq!((moved.y, moved.z), (-4.6, -5.6));
}

// Source: FUN_005c4ed0 meter 0x5f px for charge 0..100; A0 TAC-05-C020
// shows 18 of 95 px about 58 s into the 300 s charge.
#[test]
fn death_star_laser_meter_fills_95_pixels_over_the_full_charge() {
    assert_eq!(death_star_laser_charge_width(0.0), 0.0);
    assert_eq!(death_star_laser_charge_width(58.0 / 3.0), 18.0);
    assert_eq!(death_star_laser_charge_width(50.0), 47.0);
    assert_eq!(death_star_laser_charge_width(100.0), 95.0);
    assert_eq!(death_star_laser_charge_width(250.0), 95.0);
    assert_eq!(death_star_laser_charge_width(-5.0), 0.0);
}

// Source: FUN_005ba7f0 Death Star laser charge rate to 100.
#[test]
fn death_star_operational_predicate_resources_and_laser_charge_match_source_fields() {
    let mut death_star = TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    };
    assert!(death_star.operational());
    assert_eq!(death_star.resource_id(), 5010);
    death_star.advance_laser_charge(300.0);
    assert_eq!(death_star.laser_charge, 100.0);
    death_star.advance_laser_charge(1.0);
    assert_eq!(death_star.laser_charge, 100.0);

    death_star.action_committed = true;
    assert!(!death_star.operational());
    death_star.action_committed = false;
    death_star.destroyed = true;
    assert!(!death_star.operational());
    assert_eq!(death_star.resource_id(), 5020);
}

#[test]
fn death_star_ready_voice_fires_once_only_when_the_players_laser_crosses_full_charge() {
    // FUN_005bae60 maps source event 0x113 to the Imperial ready line;
    // the tactical manager emits it on the player's not-ready/ready edge.
    let attacker = test_ship(64, 0, true, true);
    let defender = test_ship(128, 0, false, true);
    let mut session = test_session(vec![attacker, defender], Vec::new(), true);
    session.paused = false;
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: true,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 99.95,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });

    // The commander charges the laser once a turn (FUN_005ba7f0 from
    // FUN_005c8780, slot +0x80): the edge comes at turn 1, step 4.
    for _ in 0..3 {
        assert!(!session.step());
    }
    assert!(session.take_pending_voice_cues().is_empty());
    assert!(!session.step());
    assert_eq!(session.combat_tick, 4);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::death_star_ready()]
    );
    for _ in 0..4 {
        assert!(!session.step());
    }
    assert_eq!(session.combat_tick, 8);
    assert!(session.take_pending_voice_cues().is_empty());

    let attacker = test_ship(64, 0, true, true);
    let defender = test_ship(128, 0, false, true);
    let mut opponent_owned = test_session(vec![attacker, defender], Vec::new(), true);
    opponent_owned.paused = false;
    opponent_owned.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 99.95,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });

    for _ in 0..4 {
        assert!(!opponent_owned.step());
    }
    assert_eq!(opponent_owned.combat_tick, 4);
    assert!(opponent_owned.take_pending_voice_cues().is_empty());
}

#[test]
fn battle_outcome_voice_distinguishes_draw_destruction_and_each_withdrawal_side() {
    // FUN_005bae60 assigns separate outcome events for withdrawal,
    // destruction victory, and defeat in each faction's recovered bank.
    let alliance = test_ship(64, 0, true, true);
    let empire = test_ship(128, 0, false, true);
    let mut session = test_session(vec![alliance, empire], Vec::new(), true);

    session.queue_battle_outcome_voice(CombatWinner::Draw);
    assert!(session.take_pending_voice_cues().is_empty());

    session.queue_battle_outcome_voice(CombatWinner::Attacker);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::battle_won(
            TacticalVoiceFaction::Alliance,
            false
        )]
    );

    session.ships[1].retreated = true;
    session.queue_battle_outcome_voice(CombatWinner::Attacker);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::battle_won(
            TacticalVoiceFaction::Alliance,
            true
        )]
    );

    session.ships[1].retreated = false;
    session.ships[0].retreated = true;
    session.queue_battle_outcome_voice(CombatWinner::Defender);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::withdrawal_complete(
            TacticalVoiceFaction::Alliance
        )]
    );

    session.ships[0].retreated = false;
    session.queue_battle_outcome_voice(CombatWinner::Defender);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::battle_lost(
            TacticalVoiceFaction::Alliance
        )]
    );

    let alliance = test_ship(64, 0, true, true);
    let empire = test_ship(128, 0, false, true);
    let mut imperial_player = test_session(vec![alliance, empire], Vec::new(), false);
    imperial_player.ships[0].retreated = true;
    imperial_player.queue_battle_outcome_voice(CombatWinner::Defender);
    assert_eq!(
        imperial_player.take_pending_voice_cues(),
        vec![TacticalVoiceCue::battle_won(
            TacticalVoiceFaction::Empire,
            true
        )]
    );
    imperial_player.ships[0].retreated = false;
    imperial_player.queue_battle_outcome_voice(CombatWinner::Attacker);
    assert_eq!(
        imperial_player.take_pending_voice_cues(),
        vec![TacticalVoiceCue::battle_lost(TacticalVoiceFaction::Empire)]
    );
}

// Source: FUN_005ba7f0 Death Star superlaser predicates.
#[test]
fn death_star_superlaser_requires_player_ownership_full_charge_and_enemy_target() {
    let target = test_ship(64, 0, true, true);
    let mut session = test_session(vec![target], Vec::new(), false);
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 99.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });
    assert!(!session.commit_death_star_shot(0));
    session.death_star.as_mut().unwrap().laser_charge = 100.0;
    session.player_is_attacker = true;
    assert!(!session.commit_death_star_shot(0));
    session.player_is_attacker = false;
    session.ships[0].is_attacker = false;
    assert!(!session.commit_death_star_shot(0));
}

// Source: 0x005ba720 posts event 0x44 at +0x87c (3 turns ahead,
// +0x8ac) and the type-3 kill at +0x87c + 1; FUN_005a96b0 runs a stamp
// at the next 1000 ms turn (+0x8a8), and 0x005b2b60 kills the target.
#[test]
fn a_superlaser_shot_fires_after_the_event_lead_and_kills_a_turn_later() {
    let mut target = test_ship(64, 0, true, true);
    target.source_position.z = -24.0;
    let mut session = test_session(vec![target], Vec::new(), false);
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 24.0,
        },
        hull: 100.0,
        laser_charge: 100.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });

    assert!(session.commit_death_star_shot(0));
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::death_star_firing()]
    );
    assert_eq!(session.death_star.unwrap().laser_charge, 0.0);
    assert!(session.death_star.unwrap().action_committed);
    assert!(session.death_star_beam.is_some());
    // Presentation time alone neither fires nor resolves the shot.
    session.advance_presentational_effects(10.0);
    assert!(session.ships[0].alive);
    assert!(!session.death_star_beam.unwrap().fired);

    // Committed in turn 0: event 0x44 is stamped 3 and runs at turn 4.
    session.paused = false;
    for _ in 0..15 {
        session.step();
    }
    assert!(!session.death_star_beam.unwrap().fired);
    session.ships[0].source_position.z = -20.0;
    session.step();
    assert_eq!(session.combat_tick, 16);
    let beam = session.death_star_beam.unwrap();
    assert!(beam.fired);
    assert_eq!(beam.target_position.z, -20.0);
    session.advance_presentational_effects(0.5);
    assert_eq!(session.death_star_beam.unwrap().elapsed, 0.5);

    // The type-3 event is stamped 4 and runs at turn 5.
    for _ in 0..3 {
        session.step();
    }
    assert!(session.ships[0].alive);
    session.step();
    assert_eq!(session.combat_tick, 20);
    assert!(!session.ships[0].alive);
    assert_eq!(session.ships[0].hull_current, 0);
    assert!(session.death_star_beam.is_none());
    assert!(!session.death_star.unwrap().action_committed);
    assert_eq!(session.impact_effects.len(), 1);
}

// Source: 0x005ba520 starts the beam; 0x005da830 grows it for 1 s.
#[test]
fn the_beam_is_drawn_only_from_its_start_through_its_second() {
    let mut beam = rebellion_core::tactical::TacticalDeathStarBeam {
        target: 0,
        source_position: TacticalWorldPosition::ORIGIN,
        target_position: TacticalWorldPosition::ORIGIN,
        fired: false,
        elapsed: 0.5,
    };
    assert_eq!(death_star_beam_progress(&beam), None);
    beam.fired = true;
    assert_eq!(death_star_beam_progress(&beam), Some(0.5));
    beam.elapsed = 1.0;
    assert_eq!(death_star_beam_progress(&beam), Some(1.0));
    beam.elapsed = 1.001;
    assert_eq!(death_star_beam_progress(&beam), None);
}

// Source: 0x005b2b60 kills only a target still in the fight
// (FUN_005ad870) while the Death Star is there (FUN_005a9330).
#[test]
fn a_superlaser_shot_spares_a_target_that_has_jumped_or_a_lost_death_star() {
    for case in 0..2 {
        let mut session = test_session(vec![test_ship(64, 0, true, true)], Vec::new(), false);
        session.death_star = Some(TacticalDeathStar {
            resource: DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 100.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        session.combat_tick = 6;
        assert!(session.commit_death_star_shot(0));
        // Turn 1: event 0x44 stamped 4 (runs at turn 5, step 20), the
        // kill stamped 5 (turn 6, step 24).
        assert_eq!(
            session.turn_events,
            vec![
                rebellion_core::tactical::TacticalQueuedEvent {
                    stamp: 4,
                    event: rebellion_core::tactical::TacticalTurnEvent::DeathStarBeam { target: 0 },
                },
                rebellion_core::tactical::TacticalQueuedEvent {
                    stamp: 5,
                    event: rebellion_core::tactical::TacticalTurnEvent::DeathStarKill { target: 0 },
                },
            ]
        );
        if case == 0 {
            session.ships[0].retreat_progress = 0.5;
        } else {
            session.death_star.as_mut().unwrap().destroyed = true;
        }
        session.combat_tick = 23;
        session.paused = false;
        session.step();
        assert!(session.death_star_beam.is_none());
        assert!(session.ships[0].alive);
        assert!(session.impact_effects.is_empty());
    }
}

// Source: FUN_005cfec0 / FUN_005d04e0 timed trench-run producer.
#[test]
fn attack_death_star_order_uses_the_timed_native_result_producer() {
    let mut fighter = test_fighter(3, true);
    fighter.selected = true;
    fighter.order = TacticalOrder::AttackDeathStar;
    fighter.maneuverability = 9;
    let mut session = test_session(Vec::new(), vec![fighter], true);
    session.trench_run_success_rating = 9.0;
    session.tactical_rng = OriginalTacticalRng::new(15);
    session.selected_fighter_group = Some(0);
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 6.0,
        },
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });

    begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);
    // The order starts the approach (state 2, 0x005d0820); the timer
    // waits for state 5.
    assert_eq!(session.trench_run_remaining_ms, None);
    assert!(session.trench_run_under_way());
    // The approach ends: state 5 (FUN_005cfec0).
    session.commit_original_trench_run();
    assert_eq!(
        session.trench_run_remaining_ms,
        Some(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS)
    );
    session.advance_original_trench_run(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS - 1.0);
    assert_eq!(session.trench_run_outcome, None);
    assert_eq!(session.death_star.unwrap().hull, 100.0);
    session.advance_original_trench_run(1.0);

    let death_star = session.death_star.unwrap();
    assert!(death_star.destroyed);
    assert_eq!(death_star.hull, 0.0);
    assert_eq!(death_star.resource_id(), 5020);
    assert!(session.weapon_effects.is_empty());
    assert_eq!(
        session.trench_run_outcome,
        Some(TacticalTrenchRunOutcome::Success)
    );
    assert_eq!(
        session.take_pending_trench_run_cinematic(),
        Some(TacticalTrenchRunOutcome::Success)
    );
    assert_eq!(session.take_pending_trench_run_cinematic(), None);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![
            TacticalVoiceCue::trench_run_started(0),
            TacticalVoiceCue::trench_run_committed(0),
            TacticalVoiceCue::trench_run_chatter(0x11e),
            TacticalVoiceCue::trench_run_chatter(0x11f),
            TacticalVoiceCue::trench_run_chatter(0x120),
            TacticalVoiceCue::trench_run_chatter(0x128),
            TacticalVoiceCue::trench_run_chatter(0x121),
            TacticalVoiceCue::trench_run_chatter(0x122),
            TacticalVoiceCue::trench_run_chatter(0x129),
            TacticalVoiceCue::trench_run_outcome(0, TacticalTrenchRunOutcome::Success),
        ]
    );
}

// Source: FUN_005cfec0 state 6 removes the first bounded half of the
// retained participant list after a successful trench run.
#[test]
fn successful_trench_run_applies_the_source_half_group_casualty_rule() {
    let mut fighters = (0..4)
        .map(|_| {
            let mut fighter = test_fighter(3, true);
            fighter.order = TacticalOrder::AttackDeathStar;
            fighter.maneuverability = 9;
            fighter
        })
        .collect::<Vec<_>>();
    fighters[0].selected = true;
    let mut session = test_session(Vec::new(), fighters, true);
    session.trench_run_success_rating = 9.0;
    session.tactical_rng = OriginalTacticalRng::new(15);
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });

    begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);
    // The approach ends: state 5 (FUN_005cfec0, FUN_005d04e0).
    session.commit_original_trench_run();
    assert_eq!(session.trench_run_participants, vec![0, 1, 2, 3]);
    assert_eq!(
        session.trench_run_pending_outcome,
        Some(TacticalTrenchRunOutcome::Success)
    );
    session.advance_original_trench_run(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);

    assert!(!session.fighters[0].alive);
    assert!(!session.fighters[1].alive);
    assert!(session.fighters[2].alive);
    assert!(session.fighters[3].alive);
    assert!(session.death_star.unwrap().destroyed);
}

fn trench_run_session(fighter: TacticalFighter, rating: f32, seed: u32) -> BattleSession {
    let mut session = test_session(Vec::new(), vec![fighter], true);
    session.trench_run_success_rating = rating;
    session.tactical_rng = OriginalTacticalRng::new(seed);
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition::ORIGIN,
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });
    begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);
    session
}

// Source: FUN_005d04e0 0x5d06b1..0x5d06e3: the shot is a 1..100 draw
// at or under (FUN_005b23a0 + the commander rating) x 0.5, where
// FUN_005b23a0 is hull / full hull x torpedoes x 0.5333 (FUN_005b1770,
// DAT_0066c4cc). A first visit (no maneuver hit: maneuverability 9
// against a draw under 10) with 400 torpedoes, (213.3 + 1.0) x 0.5,
// cannot miss, though the rating alone (1) almost always would.
#[test]
fn the_trench_run_shot_adds_the_torpedo_term_before_halving() {
    let seed = (1..10_000)
        .find(|&seed| {
            let mut rng = OriginalTacticalRng::new(seed);
            rng.inclusive(1, 10) < 10 && rng.inclusive(1, 100) > 1
        })
        .unwrap();
    let mut fighter = test_fighter(3, true);
    fighter.order = TacticalOrder::AttackDeathStar;
    fighter.maneuverability = 9;
    fighter.torpedo_strength = 400;
    let mut session = trench_run_session(fighter, 1.0, seed);
    session.commit_original_trench_run();
    assert_eq!(
        session.trench_run_pending_outcome,
        Some(TacticalTrenchRunOutcome::Success)
    );
    // Two draws: the maneuver and the shot.
    let mut expected = OriginalTacticalRng::new(seed);
    expected.inclusive(1, 10);
    expected.inclusive(1, 100);
    assert_eq!(session.tactical_rng, expected);
}

// Source: FUN_005d04e0 0x5d05cd..0x5d061e: a maneuver hit draws 1..(hull
// / 8.0) for 8.0 each; on success the loop before LAB_005d076a gives
// every member back the hull and shields FUN_005d08d0 kept at state 5
// (+0x3d0 -> +0x3cc, +0x3c4 -> +0x3c0).
#[test]
fn a_successful_trench_run_leaves_its_maneuver_hit_no_damage() {
    // Maneuverability 0 (effective 1): a maneuver draw over 1 hits; a
    // hull of 96 gives a 1..12 damage draw. At 6 or under, 48 hull or
    // more is left, and the shot, (48 / 96 x 213.3 + 1.0) x 0.5, is 53
    // or more.
    let seed = (1..10_000)
        .find(|&seed| {
            let mut rng = OriginalTacticalRng::new(seed);
            rng.inclusive(1, 10) > 1 && rng.inclusive(1, 12) <= 6 && rng.inclusive(1, 100) <= 53
        })
        .unwrap();
    let mut fighter = test_fighter(3, true);
    fighter.order = TacticalOrder::AttackDeathStar;
    fighter.maneuverability = 0;
    fighter.torpedo_strength = 400;
    fighter.shield = 3.0;
    let mut session = trench_run_session(fighter, 1.0, seed);
    session.commit_original_trench_run();
    assert_eq!(
        session.trench_run_pending_outcome,
        Some(TacticalTrenchRunOutcome::Success)
    );
    let mut expected = OriginalTacticalRng::new(seed);
    expected.inclusive(1, 10);
    expected.inclusive(1, 12);
    expected.inclusive(1, 100);
    assert_eq!(session.tactical_rng, expected);
    let fighter = &session.fighters[0];
    assert!(fighter.alive);
    assert_eq!(fighter.hull_current, 96.0);
    assert_eq!(fighter.shield, 3.0);
}

fn trench_seed(predicate: impl Fn(&mut OriginalTacticalRng) -> bool) -> u32 {
    (1..100_000)
        .find(|&seed| predicate(&mut OriginalTacticalRng::new(seed)))
        .unwrap()
}

fn trench_stream_after(seed: u32, draws: usize) -> OriginalTacticalRng {
    let mut stream = OriginalTacticalRng::new(seed);
    for _ in 0..draws {
        stream.inclusive(1, 10);
    }
    stream
}

fn trench_runner(maneuverability: u32, hull: f32, torpedoes: u32) -> TacticalFighter {
    let mut fighter = test_fighter(3, true);
    fighter.order = TacticalOrder::AttackDeathStar;
    fighter.maneuverability = maneuverability;
    fighter.hull_max = 96.0;
    fighter.hull_current = hull;
    fighter.torpedo_strength = torpedoes;
    fighter.refresh_craft_count();
    fighter
}

// Source: FUN_005d04e0 0x5d06b1..0x5d06e3. At half hull, 30 torpedoes
// give 48 / 96 x 30 x 0.5333 = 8 (FUN_005b23a0); with rating 2 the
// threshold is (8 + 2) x 0.5 = 5: a shot draw of 5 hits at the first
// visit (two draws), a 6 misses and the run goes on.
#[test]
fn the_trench_run_shot_threshold_is_the_halved_torpedo_and_rating_sum() {
    for (shot, hits) in [(5, true), (6, false)] {
        let seed = trench_seed(|rng| rng.inclusive(1, 10) < 10 && rng.inclusive(1, 100) == shot);
        let mut session = trench_run_session(trench_runner(9, 48.0, 30), 2.0, seed);
        session.commit_original_trench_run();
        assert_eq!(
            session.tactical_rng == trench_stream_after(seed, 2),
            hits,
            "shot draw {shot}"
        );
    }
}

// Source: FUN_005d04e0 0x5d05cd..0x5d061e. Maneuverability 0 (effective
// 1) is hit on a draw over 1; a hull of 96 gives a 1..12 damage draw.
// A 6 takes 48 (6 x 8.0), so the shot is (48 / 96 x 16 + 2) x 0.5 = 5:
// a 5 hits after three draws, a 6 does not.
#[test]
fn a_maneuver_hit_takes_eight_hull_per_point_of_its_draw() {
    for (shot, hits) in [(5, true), (6, false)] {
        let seed = trench_seed(|rng| {
            rng.inclusive(1, 10) > 1 && rng.inclusive(1, 12) == 6 && rng.inclusive(1, 100) == shot
        });
        let mut session = trench_run_session(trench_runner(0, 96.0, 30), 2.0, seed);
        session.commit_original_trench_run();
        assert_eq!(
            session.tactical_rng == trench_stream_after(seed, 3),
            hits,
            "shot draw {shot}"
        );
    }
}

// Source: FUN_005d04e0 0x5d0620 and 0x5d0662: a member under two units
// of 8.0 hit takes 100 000.0, has no hull left, takes no shot and is
// not queued again, so the run ends failed after one draw.
#[test]
fn a_member_hit_with_under_two_units_of_hull_takes_no_shot() {
    let seed = trench_seed(|rng| rng.inclusive(1, 10) > 1);
    let mut session = trench_run_session(trench_runner(0, 15.0, 30), 9.0, seed);
    session.commit_original_trench_run();
    assert_eq!(
        session.trench_run_pending_outcome,
        Some(TacticalTrenchRunOutcome::Failure)
    );
    assert_eq!(session.tactical_rng, trench_stream_after(seed, 1));
}

// Source: FUN_005d04e0 0x5d06e9..0x5d06f7: the visit count fails the
// run once it passes 0x3e8. A hull no hit can end, with no torpedoes
// and rating 1 (threshold 0), is visited 1001 times: a maneuver draw,
// a damage draw on a 10, and a shot each.
#[test]
fn the_trench_run_fails_after_its_1001st_visit() {
    let seed = 7;
    let mut runner = trench_runner(9, 96.0, 0);
    runner.hull_max = 1.0e30;
    runner.hull_current = 1.0e30;
    let mut session = trench_run_session(runner, 1.0, seed);
    session.commit_original_trench_run();
    assert_eq!(
        session.trench_run_pending_outcome,
        Some(TacticalTrenchRunOutcome::Failure)
    );
    let mut expected = OriginalTacticalRng::new(seed);
    for _ in 0..1001 {
        if expected.inclusive(1, 10) > 9 {
            expected.inclusive(1, 2);
        }
        expected.inclusive(1, 100);
    }
    assert_eq!(session.tactical_rng, expected);
}

// Source: FUN_005d04e0 takes the group's members (FUN_005c6100 on
// +0x1c), the fighters on the Death Star mission that oppose it; only
// an operational Death Star (hull left, not destroyed) is attacked.
#[test]
fn the_trench_run_takes_only_living_fighters_against_a_living_death_star() {
    let runner = trench_runner(9, 96.0, 0);
    let mut idle = runner.clone();
    idle.order = TacticalOrder::None;
    let mut imperial = runner.clone();
    imperial.is_attacker = false;
    let mut dead = runner.clone();
    dead.alive = false;
    let mut empty = runner.clone();
    empty.squad_count = 0;
    let mut session = trench_run_session(runner.clone(), 1.0, 7);
    session.fighters = vec![runner.clone(), idle, imperial, dead, empty];
    session.commit_original_trench_run();
    assert_eq!(session.trench_run_participants, vec![0]);

    for (hull, destroyed) in [(0.0, false), (100.0, true)] {
        let mut session = trench_run_session(runner.clone(), 1.0, 7);
        if let Some(death_star) = session.death_star.as_mut() {
            death_star.hull = hull;
            death_star.destroyed = destroyed;
        }
        session.commit_original_trench_run();
        assert_eq!(session.trench_run_pending_outcome, None);
        assert_eq!(session.tactical_rng, OriginalTacticalRng::new(7));
    }
}

// Source: FUN_005bae60 Death Star attack failure voice event.
#[test]
fn losing_the_committed_death_star_attackers_routes_failure_once() {
    let mut fighter = test_fighter(3, true);
    fighter.order = TacticalOrder::AttackDeathStar;
    fighter.weapon_arc = TacticalWeaponArc::default();
    fighter.shield = 0.0;
    fighter.shield_max = 0.0;
    fighter.hull_current = 1.0;
    fighter.refresh_craft_count();

    let mut defender = test_ship(128, 0, false, true);
    defender.order = TacticalOrder::AttackFighters;
    defender.attack_target = Some(TacticalAttackTarget::FighterGroup(0));
    defender.weapon_arcs[0] = TacticalWeaponArc::new(300, 0, 0);
    defender.weapon_ranges.laser_cannon = 100.0;

    let mut session = test_session(vec![defender], vec![fighter], true);
    session.paused = false;
    session.death_star = Some(TacticalDeathStar {
        resource: DEATH_STAR_TACTICAL_RESOURCE,
        is_attacker: false,
        is_alliance: false,
        source_position: TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 6.0,
        },
        hull: 100.0,
        laser_charge: 0.0,
        shielded: false,
        destroyed: false,
        action_committed: false,
    });
    begin_original_attack_order(&mut session, &[], &[0], TacticalOrder::AttackDeathStar);
    // The defender's first shot hits with the stream two draws on; the
    // run's own draws (FUN_005d04e0's queue) take a varying count.
    let mut stream = session.tactical_rng;
    stream.inclusive(1, 10);
    stream.inclusive(1, 100);
    // The approach ends: state 5 (FUN_005cfec0), the run committed.
    session.commit_original_trench_run();
    session.tactical_rng = stream;

    // The weapons fire at the turn's updates, step 4 (0x005a9de0); the
    // hit event runs at turn 6, step 24 (FUN_005b3f10: +0x87c + 2.0).
    // The Alliance is then out of the fight: the end wait starts
    // (FUN_005a9ec0 0x005aa49e) and the battle runs on.
    for _ in 0..24 {
        assert!(!session.step());
    }
    assert_eq!(session.combat_tick, 24);
    assert_eq!(
        session.end_wait_ms,
        Some(rebellion_core::tactical::ORIGINAL_TACTICAL_END_WAIT_MILLISECONDS)
    );
    assert!(!session.fighters[0].alive);
    assert!(!session.death_star.unwrap().destroyed);
    assert_eq!(
        session.trench_run_pending_outcome,
        Some(TacticalTrenchRunOutcome::Failure)
    );
    assert_eq!(session.take_pending_trench_run_cinematic(), None);
    session.advance_original_trench_run(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);
    assert_eq!(
        session.take_pending_trench_run_cinematic(),
        Some(TacticalTrenchRunOutcome::Failure)
    );
    assert_eq!(session.take_pending_trench_run_cinematic(), None);
    let cues = session.take_pending_voice_cues();
    assert_eq!(cues.first(), Some(&TacticalVoiceCue::trench_run_started(0)));
    assert!(cues.contains(&TacticalVoiceCue::fighter_losses(
        TacticalVoiceFaction::Alliance,
        0,
    )));
    assert!(cues.contains(&TacticalVoiceCue::battle_lost(
        TacticalVoiceFaction::Alliance,
    )));
    assert_eq!(
        cues.last(),
        Some(&TacticalVoiceCue::trench_run_outcome(
            0,
            TacticalTrenchRunOutcome::Failure,
        ))
    );
}

#[test]
fn a_trench_run_result_leaves_the_battle_running() {
    let mut session = test_session(Vec::new(), Vec::new(), true);
    session.paused = false;
    session.record_trench_run_outcome(TacticalTrenchRunOutcome::Success);

    // FUN_005c0fb0 -> FUN_0059cc60 only stops the music and plays film
    // 201; A0 2026-11-18-edited-saves-7 d-13 (d104e6d5a72e) shows the
    // battle running after it, and its result follows by itself (d-14).
    assert!(!session.paused, "the trench-run result paused the battle");
    assert_eq!(
        session.take_pending_trench_run_cinematic(),
        Some(TacticalTrenchRunOutcome::Success)
    );
}

#[test]
fn trench_run_result_is_recorded_once_and_failure_remains_source_driven() {
    let mut session = test_session(Vec::new(), Vec::new(), true);
    session.record_trench_run_outcome(TacticalTrenchRunOutcome::Failure);
    session.record_trench_run_outcome(TacticalTrenchRunOutcome::Success);

    assert_eq!(
        session.take_pending_trench_run_cinematic(),
        Some(TacticalTrenchRunOutcome::Failure)
    );
    assert_eq!(
        session.trench_run_outcome,
        Some(TacticalTrenchRunOutcome::Failure)
    );
    assert_eq!(session.take_pending_trench_run_cinematic(), None);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::trench_run_outcome(
            0,
            TacticalTrenchRunOutcome::Failure
        )]
    );
}

#[test]
fn alliance_trench_result_selects_only_the_active_alliance_attack_group() {
    // FUN_005bae60's RGBY outcome family is selected from the Alliance
    // group actively executing the Death Star mission.
    let mut imperial_attacker = test_fighter(5, false);
    imperial_attacker.fighter_group = 1;
    imperial_attacker.order = TacticalOrder::AttackDeathStar;
    let mut inactive_alliance = test_fighter(3, true);
    inactive_alliance.fighter_group = 2;
    inactive_alliance.order = TacticalOrder::None;
    let mut active_alliance = test_fighter(4, true);
    active_alliance.fighter_group = 3;
    active_alliance.order = TacticalOrder::AttackDeathStar;
    let mut session = test_session(
        Vec::new(),
        vec![imperial_attacker, inactive_alliance, active_alliance],
        true,
    );

    session.record_trench_run_outcome(TacticalTrenchRunOutcome::Success);

    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::trench_run_outcome(
            3,
            TacticalTrenchRunOutcome::Success
        )]
    );
}

#[test]
fn imperial_trench_run_observer_routes_failure_and_success_to_death_star_events() {
    // FUN_005bae60 supplies distinct Imperial events for a broken-off
    // Alliance attack and destruction of the Death Star.
    let mut failure = test_session(Vec::new(), Vec::new(), false);
    failure.record_trench_run_outcome(TacticalTrenchRunOutcome::Failure);
    assert_eq!(
        failure.take_pending_voice_cues(),
        vec![TacticalVoiceCue::death_star_attack_broken_off()]
    );
    assert_eq!(
        failure.trench_run_outcome,
        Some(TacticalTrenchRunOutcome::Failure)
    );

    let mut success = test_session(Vec::new(), Vec::new(), false);
    success.record_trench_run_outcome(TacticalTrenchRunOutcome::Success);
    assert_eq!(
        success.take_pending_voice_cues(),
        vec![TacticalVoiceCue::death_star_destroyed()]
    );
    assert_eq!(
        success.trench_run_outcome,
        Some(TacticalTrenchRunOutcome::Success)
    );
}

#[test]
fn selected_panel_controls_sit_at_the_a0_panel_origin() {
    // A0 TAC-03-C001: Previous Capital Ship (TACTICAL 1101) is drawn at
    // (489, 38), one pixel right of and below the port's first guess.
    let at = |x, y| {
        tactical_command_rect_control_at(TacticalCommandPanel::Display, true, false, false, x, y)
    };
    assert_eq!(
        at(489.5, 38.5),
        Some(TacticalCommandControl::PreviousCapital)
    );
    assert_eq!(at(488.5, 37.5), None);
    assert_eq!((TACTICAL_PANEL_X, TACTICAL_PANEL_Y), (482.0, 28.0));
    assert_eq!(
        (TACTICAL_SELECTED_SHIP_ART_X, TACTICAL_SELECTED_SHIP_ART_Y),
        (508.0, 45.0)
    );
}

#[test]
fn no_selection_shows_the_players_emblem_panel() {
    // A0 TAC-04-C005 (Alliance) and TAC-01-C002 (Empire).
    assert_eq!(tactical_display_panel_resource(false, false, true), 1303);
    assert_eq!(tactical_display_panel_resource(false, false, false), 1304);
    assert_eq!(tactical_display_panel_resource(false, true, false), 1302);
    assert_eq!(tactical_display_panel_resource(true, false, true), 1307);
    assert_eq!(resources::tactical::BATTLE_OPTIONS_PANEL, 1306);
}

#[test]
fn selected_meters_show_hull_left_and_shield_right() {
    // A0 TAC-03-C006/C005/C003: the right meter empties first and the
    // ship dies when the left one empties; both are 37x10.
    assert_eq!(
        TACTICAL_SELECTED_HULL_METER,
        NativeRect::new(516.0, 109.0, 37.0, 10.0)
    );
    assert_eq!(
        TACTICAL_SELECTED_SHIELD_METER,
        NativeRect::new(585.0, 109.0, 37.0, 10.0)
    );
    assert_eq!(TACTICAL_METER_REMAINING, Color::new(0.0, 0.0, 1.0, 1.0));
    assert_eq!(TACTICAL_METER_LOST, Color::new(1.0, 0.0, 0.0, 1.0));
}

#[test]
fn meter_remaining_part_is_rounded_and_clamped() {
    assert_eq!(tactical_meter_remaining_width(37.0, 1.0), 37.0);
    assert_eq!(tactical_meter_remaining_width(37.0, 0.62), 23.0);
    assert_eq!(tactical_meter_remaining_width(37.0, -0.5), 0.0);
    assert_eq!(tactical_meter_remaining_width(37.0, 1.5), 37.0);
}

#[test]
fn tactical_assignment_hit_regions_follow_the_source_panels() {
    assert_eq!(
        tactical_command_rect_control_at(
            TacticalCommandPanel::Display,
            true,
            false,
            false,
            493.0,
            60.0,
        ),
        Some(TacticalCommandControl::PreviousCapital)
    );
    assert_eq!(
        tactical_command_rect_control_at(
            TacticalCommandPanel::Display,
            true,
            false,
            false,
            618.0,
            60.0,
        ),
        Some(TacticalCommandControl::NextCapital)
    );
    assert_eq!(
        tactical_command_rect_control_at(
            TacticalCommandPanel::Display,
            true,
            true,
            false,
            493.0,
            60.0,
        ),
        None
    );
    assert_eq!(
        tactical_command_rect_control_at(
            TacticalCommandPanel::Display,
            true,
            false,
            false,
            500.0,
            245.0,
        ),
        // A0 TAC-05-C012/C015: the left order button opens Maneuvers
        // Tactics, the right one Missions.
        Some(TacticalCommandControl::OpenManeuvers)
    );
    assert_eq!(
        tactical_command_rect_control_at(
            TacticalCommandPanel::Display,
            true,
            false,
            false,
            570.0,
            245.0,
        ),
        Some(TacticalCommandControl::OpenMissions)
    );
    let maneuvers = TacticalCommandPanel::Maneuvers {
        pending_order: TacticalOrder::None,
        pending_tactic: TacticalTactic::StandOff,
    };
    assert_eq!(
        tactical_command_rect_control_at(maneuvers, true, false, false, 500.0, 160.0),
        Some(TacticalCommandControl::Hammer)
    );
    assert_eq!(
        tactical_command_rect_control_at(maneuvers, true, false, false, 600.0, 170.0),
        Some(TacticalCommandControl::ToggleTactic)
    );
    let missions = TacticalCommandPanel::Missions {
        pending_order: TacticalOrder::None,
    };
    assert_eq!(
        tactical_command_rect_control_at(missions, true, false, false, 575.0, 165.0),
        None
    );
    assert_eq!(
        tactical_command_rect_control_at(missions, true, true, false, 575.0, 165.0),
        Some(TacticalCommandControl::Recover)
    );
    assert_eq!(
        tactical_command_rect_control_at(missions, true, true, false, 575.0, 195.0),
        None
    );
    assert_eq!(
        tactical_command_rect_control_at(missions, true, true, true, 575.0, 195.0),
        Some(TacticalCommandControl::AttackDeathStar)
    );
    assert_eq!(
        tactical_command_rect_control_at(missions, false, true, false, 505.0, 165.0),
        None
    );
}

// Source: FUN_005dcc70's maneuver panel commits order +0x68 and tactic +0x38 on
// check and returns unchanged on cancel (P58F9).
#[test]
fn maneuver_confirm_commits_and_cancel_preserves_selected_capital_orders() {
    let mut ship = test_ship(64, 0, true, true);
    ship.selected = true;
    let mut enemy = test_ship(128, 0, false, true);
    enemy.source_position.z = 50.0;
    let mut session = test_session(vec![ship, enemy], Vec::new(), true);
    session.selected_ship = Some(0);
    let mut state = TacticalState {
        session: Some(session),
        ..TacticalState::default()
    };

    activate_tactical_command(&mut state, TacticalCommandControl::OpenManeuvers);
    activate_tactical_command(&mut state, TacticalCommandControl::Hammer);
    activate_tactical_command(&mut state, TacticalCommandControl::ToggleTactic);
    activate_tactical_command(&mut state, TacticalCommandControl::Cancel);
    let ship = &state.session.as_ref().unwrap().ships[0];
    assert_eq!(ship.order, TacticalOrder::None);
    assert_eq!(ship.tactic, TacticalTactic::StandOff);
    assert!(state
        .session
        .as_ref()
        .unwrap()
        .pending_voice_cues
        .is_empty());

    activate_tactical_command(&mut state, TacticalCommandControl::OpenManeuvers);
    activate_tactical_command(&mut state, TacticalCommandControl::Hammer);
    activate_tactical_command(&mut state, TacticalCommandControl::ToggleTactic);
    activate_tactical_command(&mut state, TacticalCommandControl::Confirm);
    let ship = &state.session.as_ref().unwrap().ships[0];
    assert_eq!(ship.order, TacticalOrder::Hammer);
    assert_eq!(ship.tactic, TacticalTactic::Surround);
    assert_eq!(state.command_panel, TacticalCommandPanel::Display);
    // One selected unit takes a force of its own and the generic
    // assignment line (0x005a1a60: FUN_005aded0, then event 0x77).
    assert_eq!(ship.task_force, TACTICAL_UNGROUPED_TASK_FORCE);
    assert_eq!(
        state.session.as_ref().unwrap().pending_voice_cues,
        vec![TacticalVoiceCue::capital_mission(
            TacticalVoiceFaction::Alliance,
            TACTICAL_UNGROUPED_TASK_FORCE
        )]
    );
    assert_eq!(
        state.session.as_ref().unwrap().pending_voice_cues[0].source_event,
        0x77
    );
}

// The group bar shows the eight slots only (0x5a0300 finds no slot
// for a force of its own), the selection names a slot only when all its
// ships share one, and a force of its own does not use up a blank slot.
#[test]
fn a_force_of_its_own_holds_no_group_slot() {
    let mut grouped = test_ship(64, 0, true, true);
    grouped.task_force = 2;
    let mut own = test_ship(65, 1, true, true);
    own.task_force = TACTICAL_UNGROUPED_TASK_FORCE;
    let mut lost = test_ship(66, 2, true, false);
    lost.task_force = 3;
    let mut enemy = test_ship(128, 0, false, true);
    enemy.task_force = 4;
    let mut session = test_session(vec![grouped, own, lost, enemy], Vec::new(), true);
    let hud = TacticalGroupHud::from_session(&session);
    assert_eq!(
        hud.task_forces,
        [false, false, true, false, false, false, false, false]
    );

    session.ships[1].selected = true;
    assert_eq!(selected_player_task_force(&session), None);
    session.ships[1].selected = false;
    session.ships[0].selected = true;
    assert_eq!(selected_player_task_force(&session), Some(2));
    session.ships[1].selected = true;
    assert_eq!(selected_player_task_force(&session), None);
    session.task_force_selection = Some(2);
    assert_eq!(selected_player_task_force(&session), Some(2));
    session.task_force_selection = None;

    // Slots 0 and 1 are blank; the own force takes neither.
    session.ships[0].selected = false;
    assert_eq!(assign_selected_to_task_force(&mut session, 0), 1);
    assert_eq!(session.ships[1].task_force, 0);
}

// With nothing selected an order forms no force, and a chosen task force
// gives it only to its members still in the fight.
#[test]
fn a_chosen_task_force_orders_only_its_members_in_the_fight() {
    let ships = (0..4)
        .map(|index| {
            let mut ship = test_ship(64, index, true, index != 2);
            ship.task_force = 1;
            ship
        })
        .collect::<Vec<_>>();
    let mut session = test_session(ships, Vec::new(), true);
    session.ships[3].retreating = true;
    assert_eq!(
        assign_selected_command(&mut session, TacticalOrder::HoldPosition, None),
        (0, 0)
    );
    assert_eq!(session.task_force_selection, None);

    session.task_force_selection = Some(1);
    assert_eq!(
        assign_selected_command(&mut session, TacticalOrder::HoldPosition, None),
        (2, 0)
    );
    assert_eq!(session.ships[2].order, TacticalOrder::None);
    assert_eq!(session.ships[3].order, TacticalOrder::None);
    assert!(session.ships.iter().all(|ship| ship.task_force == 1));
}

// The voice follows the selected ships only: one selected unit plays
// the generic line even beside an unselected force of its own.
#[test]
fn a_single_unit_s_order_voice_ignores_unselected_ships() {
    let mut first = test_ship(64, 0, true, true);
    first.task_force = TACTICAL_UNGROUPED_TASK_FORCE;
    first.selected = true;
    let mut other = test_ship(65, 1, true, true);
    other.task_force = TACTICAL_UNGROUPED_TASK_FORCE;
    let mut session = test_session(vec![first, other], Vec::new(), true);
    let panel = TacticalCommandPanel::Maneuvers {
        pending_order: TacticalOrder::HoldPosition,
        pending_tactic: TacticalTactic::StandOff,
    };
    queue_selected_command_voice(&mut session, panel, TacticalOrder::HoldPosition, false);
    assert_eq!(
        session.pending_voice_cues,
        vec![TacticalVoiceCue::capital_mission(
            TacticalVoiceFaction::Alliance,
            TACTICAL_UNGROUPED_TASK_FORCE
        )]
    );
}

// FUN_005cc300 at a route's end: any route of the player's force, not
// only a maneuver's, says "maneuver complete", and only while the force
// holds a target (+0x98).
#[test]
fn a_ships_route_ends_with_its_line_only_while_it_holds_a_target() {
    let run = |target: bool| {
        let mut ship = test_ship(64, 0, true, true);
        ship.subsystem_capacity.engines = true;
        ship.source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
        ship.source_waypoint = Some(TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
        if target {
            ship.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        }
        let mut session = test_session(vec![ship, test_ship(65, 0, false, true)], Vec::new(), true);
        session.ships[1].source_position.x = 300.0;
        session.advance_original_tactical_movement(250.0);
        session.take_pending_voice_cues()
    };
    assert_eq!(
        run(true),
        vec![TacticalVoiceCue::capital_maneuver_complete(
            TacticalVoiceFaction::Alliance,
            0
        )]
    );
    assert!(run(false).is_empty());
}

// FUN_005a8f70's arrival voices a force's completion once no member of
// that force is still on a maneuver; members of other forces and the
// enemy do not hold it (FUN_005a9530), nor do they for exhausted
// attack lists (FUN_005a8fc0).
#[test]
fn a_force_s_completion_waits_only_for_its_own_members() {
    let arriving = || {
        let mut ship = test_ship(64, 0, true, true);
        ship.subsystem_capacity.engines = true;
        ship.source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
        ship.source_waypoint = Some(TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
        ship.order = TacticalOrder::Anvil;
        // FUN_005cc300 voices the arrival only for a force holding a
        // target (+0x98).
        ship.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        ship
    };
    let busy = |alliance: bool, task_force: u8, x: f32| {
        let mut ship = test_ship(65, 1, alliance, true);
        ship.task_force = task_force;
        ship.order = TacticalOrder::LeftHook;
        ship.source_position.x = x;
        ship
    };
    let mut session = test_session(
        vec![arriving(), busy(false, 0, 200.0), busy(true, 1, -200.0)],
        Vec::new(),
        true,
    );
    session.advance_original_tactical_movement(250.0);
    assert_eq!(
        session.take_pending_voice_cues(),
        vec![TacticalVoiceCue::capital_maneuver_complete(
            TacticalVoiceFaction::Alliance,
            0
        )]
    );
    let mut held = test_session(vec![arriving(), busy(true, 0, 200.0)], Vec::new(), true);
    held.advance_original_tactical_movement(250.0);
    assert!(held.take_pending_voice_cues().is_empty());

    // Player ship 0 (force 0) runs out of fighters to attack; the
    // player's force 1 and the enemy's force 0 keep capital targets.
    let mut hunter = test_ship(64, 0, true, true);
    hunter.task_force = 0;
    hunter.order = TacticalOrder::AttackFighters;
    hunter.attack_target = Some(TacticalAttackTarget::FighterGroup(0));
    let mut other_force = test_ship(65, 1, true, true);
    other_force.task_force = 1;
    other_force.order = TacticalOrder::AttackCapitalShips;
    other_force.attack_target = Some(TacticalAttackTarget::CapitalShip(2));
    let mut enemy = test_ship(128, 2, false, true);
    enemy.task_force = 0;
    enemy.order = TacticalOrder::AttackCapitalShips;
    enemy.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
    let mut gone = test_fighter(5, false);
    gone.alive = false;
    gone.squad_count = 0;
    let mut done = test_session(vec![hunter, other_force, enemy], vec![gone], true);
    refresh_original_attack_targets(&mut done);
    assert_eq!(done.ships[0].attack_target, None);
    assert_eq!(
        done.take_pending_voice_cues(),
        vec![TacticalVoiceCue::capital_orders_complete(
            TacticalVoiceFaction::Alliance,
            0
        )]
    );
}

// A task force chosen whole (0x66bc48) keeps its force and slot, and a
// new tactic plays its formation line (0x005a3850: 0x6f + slot).
#[test]
fn a_chosen_task_force_takes_the_maneuver_as_a_force() {
    let mut first = test_ship(64, 0, true, true);
    first.task_force = 2;
    let mut second = test_ship(65, 1, true, true);
    second.task_force = 2;
    let mut enemy = test_ship(128, 0, false, true);
    enemy.source_position.z = 50.0;
    let mut session = test_session(vec![first, second, enemy], Vec::new(), true);
    assert_eq!(select_task_force(&mut session, 2), 2);
    let mut state = TacticalState {
        session: Some(session),
        ..TacticalState::default()
    };
    activate_tactical_command(&mut state, TacticalCommandControl::OpenManeuvers);
    activate_tactical_command(&mut state, TacticalCommandControl::Hammer);
    activate_tactical_command(&mut state, TacticalCommandControl::ToggleTactic);
    activate_tactical_command(&mut state, TacticalCommandControl::Confirm);
    let session = state.session.as_ref().unwrap();
    for ship in &session.ships[..2] {
        assert_eq!(ship.task_force, 2);
        assert_eq!(ship.order, TacticalOrder::Hammer);
        assert_eq!(ship.tactic, TacticalTactic::Surround);
    }
    assert_eq!(
        session.pending_voice_cues,
        vec![TacticalVoiceCue::capital_formation(
            TacticalVoiceFaction::Alliance,
            2
        )]
    );
}

// A ship added to a task force takes its formation place at once
// (FUN_005ec400's first follower, one spacing to the side: at least
// FUN_005ea030's 12).
#[test]
fn a_ship_assigned_to_a_task_force_takes_its_formation_place() {
    let leader = test_ship(64, 0, true, true);
    let mut joining = test_ship(64, 1, true, true);
    joining.task_force = 1;
    joining.source_position.x = 100.0;
    joining.selected = true;
    let mut session = test_session(vec![leader, joining], Vec::new(), true);
    session.ships[0].task_force = 0;
    assert_eq!(assign_selected_to_task_force(&mut session, 0), 1);
    let offset = session.ships[1].source_position.x - session.ships[0].source_position.x;
    assert_eq!(offset.abs(), ORIGINAL_FORMATION_BASE_SPACING);

    // Joining a force keeps its formation; a blank slot is a new force,
    // which starts Stand Off (0x005cbf50).
    session.task_force_formations[1][0] = TacticalTactic::Surround;
    session.task_force_formations[1][1] = TacticalTactic::Surround;
    session.ships[0].selected = false;
    assert_eq!(assign_selected_to_task_force(&mut session, 0), 1);
    assert_eq!(
        session.task_force_formations[1][0],
        TacticalTactic::Surround
    );
    assert_eq!(assign_selected_to_task_force(&mut session, 1), 1);
    assert_eq!(
        session.task_force_formations[1][1],
        TacticalTactic::StandOff
    );
}

// SetTactic (0x005c9be0) posts event 0x3d at the command stamp; only its
// turn rebuilds the force's mover with the tactic's formation
// (0x005c9c50 -> 0x005cb1a0).
#[test]
fn a_force_s_formation_changes_on_its_tactic_turn() {
    let mut first = test_ship(64, 0, true, true);
    first.task_force = 2;
    let mut second = test_ship(65, 1, true, true);
    second.task_force = 2;
    // An enemy that outlasts the steps keeps the battle going.
    let mut enemy = test_ship(128, 0, false, true);
    enemy.hull_current = 1_000_000;
    enemy.hull_max = 1_000_000;
    enemy.source_position.z = 400.0;
    let mut session = test_session(vec![first, second, enemy], Vec::new(), true);
    session.paused = false;
    assert_eq!(select_task_force(&mut session, 2), 2);
    assign_selected_command(
        &mut session,
        TacticalOrder::HoldPosition,
        Some(TacticalTactic::Surround),
    );
    assert_eq!(session.ships[0].tactic, TacticalTactic::Surround);
    assert_eq!(
        session.task_force_formations[1][2],
        TacticalTactic::StandOff
    );
    assert!(session.turn_events.iter().any(|queued| queued.stamp == 3
        && queued.event
            == TacticalTurnEvent::TaskForceTactic {
                attacker: true,
                task_force: 2,
                tactic: TacticalTactic::Surround,
            }));
    for _ in 0..15 {
        session.step();
    }
    assert_eq!(
        session.task_force_formations[1][2],
        TacticalTactic::StandOff
    );
    session.step();
    assert_eq!(
        session.task_force_formations[1][2],
        TacticalTactic::Surround
    );

    // A force formed from a selection starts Stand Off.
    session.task_force_selection = None;
    session.ships[0].selected = true;
    session.ships[1].selected = true;
    session.task_force_formations[1][0] = TacticalTactic::Surround;
    assign_selected_command(&mut session, TacticalOrder::HoldPosition, None);
    assert_eq!(session.ships[0].task_force, 0);
    assert_eq!(
        session.task_force_formations[1][0],
        TacticalTactic::StandOff
    );
}

// 0x005a2c70: several selected units form one new force in the first
// free group slot (0x0059f610), found before they leave their own; with
// none free each takes a force of its own and the 0x84 line plays.
#[test]
fn the_selection_states_that_hum_open_on_a_click_or_key_and_close_on_empty_space() {
    // FUN_005a1110 (enemy unit), FUN_005a1770 (one unit), FUN_005a42e0
    // (task force) and FUN_005a5230 hum their unit through slot +0x3c;
    // the several-unit state 0x66bc98 has no such sound, and the empty
    // state (FUN_00595b70(0, 0)) ends the selection's (FUN_0059e990).
    let mut ships = vec![
        test_ship(64, 0, true, true),
        test_ship(65, 1, true, true),
        test_ship(66, 2, false, true),
    ];
    ships[0].hull_max = 2_400;
    ships[0].task_force = 0;
    ships[1].hull_max = 900;
    ships[1].task_force = 0;
    let mut wing = test_fighter(128, true);
    wing.fighter_group = 0;
    let mut session = test_session(ships, vec![wing], true);
    session.paused = false;
    let hum = |session: &BattleSession| session.selection_hum().map(|hum| hum.cue.resource_id);
    assert_eq!(hum(&session), None);
    assert_eq!(select_task_force(&mut session, 0), 2);
    assert_eq!(hum(&session), Some(13_057));
    select_clicked_ship(&mut session, 1, true);
    assert_eq!(hum(&session), None);
    select_clicked_ship(&mut session, 1, false);
    assert_eq!(hum(&session), Some(13_059));
    select_clicked_ship(&mut session, 2, false);
    assert!(session.selection_state_open);
    assert!(select_fighter_group(&mut session, 0));
    assert_eq!(hum(&session), Some(13_056));
    let mut point = None;
    select_navigation_point(&mut session, &mut point, TacticalWorldPosition::ORIGIN);
    assert_eq!(hum(&session), None);
    session.selected_ship = Some(0);
    assert_eq!(hum(&session), None);
    select_task_force(&mut session, 0);
    assert_eq!(
        cycle_selected_capital(&mut session, 1).map(|(_, next, _)| next),
        Some(1)
    );
    assert_eq!(hum(&session), Some(13_059));
}

#[test]
fn no_unit_hums_behind_the_battle_alert() {
    // The tactical view, and so its selection states, starts only after
    // the Battle Alert's Take Command (FUN_00450540).
    let mut ship = test_ship(64, 0, true, true);
    ship.hull_max = 1_500;
    let mut session = test_session(vec![ship], Vec::new(), true);
    session.paused = false;
    session.selection_state_open = true;
    session.selected_ship = Some(0);
    let mut state = TacticalState {
        session: Some(session),
        ..TacticalState::default()
    };
    state.battle_alert_open = true;
    assert_eq!(state.selection_hum(), None);
    state.battle_alert_open = false;
    assert_eq!(
        state.selection_hum().map(|hum| hum.cue.resource_id),
        Some(13_058)
    );
}

#[test]
fn selected_units_form_a_task_force_in_the_first_free_slot() {
    let ships = (0..4)
        .map(|index| {
            let mut ship = test_ship(64 + index, index as usize, true, true);
            ship.task_force = index as u8;
            ship
        })
        .collect::<Vec<_>>();
    let mut session = test_session(ships, Vec::new(), true);
    session.ships[0].selected = true;
    session.ships[1].selected = true;
    assign_selected_command(&mut session, TacticalOrder::HoldPosition, None);
    assert_eq!(session.ships[0].task_force, 4);
    assert_eq!(session.ships[1].task_force, 4);
    assert_eq!(session.task_force_selection, Some(4));

    let mut full = test_session(
        (0..8)
            .map(|index| {
                let mut ship = test_ship(64, index, true, true);
                ship.task_force = index as u8;
                ship
            })
            .collect(),
        Vec::new(),
        true,
    );
    full.ships[0].selected = true;
    full.ships[1].selected = true;
    assign_selected_command(&mut full, TacticalOrder::HoldPosition, None);
    assert_eq!(full.ships[0].task_force, TACTICAL_UNGROUPED_TASK_FORCE);
    assert_eq!(full.ships[1].task_force, TACTICAL_UNGROUPED_TASK_FORCE);
    assert_eq!(full.task_force_selection, None);
    let panel = TacticalCommandPanel::Maneuvers {
        pending_order: TacticalOrder::HoldPosition,
        pending_tactic: TacticalTactic::StandOff,
    };
    queue_selected_command_voice(&mut full, panel, TacticalOrder::HoldPosition, false);
    assert_eq!(
        full.pending_voice_cues,
        vec![TacticalVoiceCue::target_task_force_rejected(
            TacticalVoiceFaction::Alliance
        )]
    );
}

// Source: FUN_005def00's mission panel enables Recover (order code 2) for a
// selected fighter group and commits it to every member (P58F9).
#[test]
fn mission_confirm_assigns_recover_to_the_whole_selected_fighter_group() {
    let mut fighters = vec![test_fighter(1, true), test_fighter(1, true)];
    for (index, fighter) in fighters.iter_mut().enumerate() {
        fighter.selected = true;
        fighter.identity.fleet_roster_index = index;
        fighter.fleet_fighter_index = index;
    }
    let mut session = test_session(Vec::new(), fighters, true);
    session.selected_fighter_group = Some(0);
    let mut state = TacticalState {
        session: Some(session),
        ..TacticalState::default()
    };

    activate_tactical_command(&mut state, TacticalCommandControl::OpenMissions);
    activate_tactical_command(&mut state, TacticalCommandControl::Recover);
    activate_tactical_command(&mut state, TacticalCommandControl::Confirm);
    assert!(state
        .session
        .as_ref()
        .unwrap()
        .fighters
        .iter()
        .all(|fighter| fighter.order == TacticalOrder::Recover));
    run_posted_orders(state.session.as_mut().unwrap());
    // The mission voice plays on the order; the no-room voice comes from
    // the recovery executor's turn (FUN_005cf980), installed by event 0xa.
    state.session.as_mut().unwrap().advance_recovery_executors();
    assert_eq!(
        state.session.as_ref().unwrap().pending_voice_cues,
        vec![
            TacticalVoiceCue::fighter_mission(TacticalVoiceFaction::Alliance, 0),
            TacticalVoiceCue::fighter_recovery_capacity(TacticalVoiceFaction::Alliance, 0,),
        ]
    );
}

// Source: TACTICAL.DLL resources 1058/1059 paint the 23 by 23 target control
// after the D-pad, at (549,391) in A0 TAC-04-C005. Centering on the 1200
// by 800 arena is the port's 2D fallback.
#[test]
#[ignore = "requires extracted original bitmaps in data/base/ui"]
fn target_control_owns_its_topmost_mask_and_centers_the_selected_position() {
    let mut cache = BmpCache::new();
    cache.set_base_path(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/base/ui"),
    );
    assert_eq!(
        tactical_hud_control_at(&mut cache, 560.1, 402.1),
        Some(TacticalHudControl::CameraTarget)
    );
    assert_eq!(camera_offset_for_target(120.0, 190.0), (-480.0, -210.0));
    assert_eq!(camera_offset_for_target(600.0, 400.0), (0.0, 0.0));
}
