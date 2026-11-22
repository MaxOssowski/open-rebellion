use super::*;
use rebellion_core::dat::{ExplorationStatus, SectorGroup};
use rebellion_core::ids::{CapitalShipKey, DatId, FighterKey};
use rebellion_core::world::{ControlKind, FighterEntry, Fleet, Sector, ShipInstance, System};

#[test]
fn trench_run_outcomes_route_to_their_source_movies() {
    assert_eq!(
        trench_run_cutscene_path(TacticalTrenchRunOutcome::Success),
        "assets/references/ref-videos/201.webm"
    );
    assert_eq!(
        trench_run_cutscene_path(TacticalTrenchRunOutcome::Failure),
        "assets/references/ref-videos/202.webm"
    );
}

#[test]
fn each_victory_outcome_names_its_winning_faction() {
    let system = rebellion_core::ids::SystemKey::default();
    assert!(victory_winner_is_alliance(
        &rebellion_core::victory::VictoryOutcome::HqCaptured {
            winner: Faction::Alliance,
            loser: Faction::Empire,
            hq_system: system,
        }
    ));
    assert!(!victory_winner_is_alliance(
        &rebellion_core::victory::VictoryOutcome::HqCaptured {
            winner: Faction::Empire,
            loser: Faction::Alliance,
            hq_system: system,
        }
    ));
    assert!(!victory_winner_is_alliance(
        &rebellion_core::victory::VictoryOutcome::HqDestroyed {
            winner: Faction::Empire,
            loser: Faction::Alliance,
            hq_system: system,
        }
    ));
    assert!(!victory_winner_is_alliance(
        &rebellion_core::victory::VictoryOutcome::DeathStarVictory {
            target_system: system,
        }
    ));
}

#[test]
fn the_battle_summary_opens_over_the_command_center_not_the_tactical_view() {
    // FUN_00488b90 -> FUN_0041d730 -> FUN_0042a640: once the tactical
    // module has returned and the Command Center (DAT_006b14cc) shows
    // again (FUN_0042d490), the summary is created as its child window.
    assert_eq!(
        battle_summary_mode(&GameMode::TacticalCombat, true),
        GameMode::Galaxy
    );
    // The battle itself, and the Battle Alert, keep the tactical mode.
    assert_eq!(
        battle_summary_mode(&GameMode::TacticalCombat, false),
        GameMode::TacticalCombat
    );
    // A summary open while Game Options shows stays under it.
    let options = GameMode::GameOptions {
        origin: GameOptionsOrigin::CommandCenter,
    };
    assert_eq!(battle_summary_mode(&options, true), options);
    assert_eq!(
        battle_summary_mode(&GameMode::Galaxy, true),
        GameMode::Galaxy
    );
}

#[test]
fn the_galaxy_clock_holds_while_the_battle_summary_is_open() {
    // FUN_0040a050 state 3 sets the game's +0x14, so FUN_0041e390 skips
    // the day step (FUN_0051dea0), until state 0xc clears it
    // (FUN_0041e320); state 0xb waits for the summary's Close
    // (FUN_00470ea0 -> FUN_00487890 -> FUN_00436400).
    assert!(galaxy_clock_runs(&GameMode::Galaxy, false, 0));
    assert!(!galaxy_clock_runs(&GameMode::Galaxy, true, 0));
    assert!(!galaxy_clock_runs(&GameMode::TacticalCombat, false, 0));
}

#[test]
fn the_galaxy_clock_stops_once_the_victory_word_says_the_game_is_over() {
    // FUN_0041e390 runs the sub-tick only while FUN_0051de80 is 0, and
    // FUN_0051dea0 checks it again: the scheduler's galaxy word +0x74
    // with 0x10000000 (FUN_005180e0's game over) holds the sub-ticks and
    // the day while the advisor's victory speech plays (END-01-C003).
    use rebellion_core::victory::{WORD_EMPIRE_WINS, WORD_GAME_OVER, WORD_HQ_LOST};
    assert!(galaxy_clock_runs(&GameMode::Galaxy, false, WORD_HQ_LOST));
    assert!(!galaxy_clock_runs(
        &GameMode::Galaxy,
        false,
        WORD_HQ_LOST | WORD_EMPIRE_WINS | WORD_GAME_OVER
    ));
}

#[test]
fn the_galaxy_mood_counts_the_systems_each_side_holds_from_the_viewer() {
    // FUN_004ebe40: bit 4 the side holds it, 8 the opponent, 0x10
    // neither; FUN_004c6830 and FUN_0049db10 sum bits 4 and 8.
    let (mut world, tokmia, _) = victory_word_world();
    let mut third = world.systems[tokmia].clone();
    third.control = ControlKind::Uprising(Faction::Alliance);
    world.systems.insert(third.clone());
    third.control = ControlKind::Uncontrolled;
    world.systems.insert(third.clone());
    third.control = ControlKind::Contested;
    world.systems.insert(third.clone());
    third.control = ControlKind::Controlled(Faction::Neutral);
    world.systems.insert(third);
    assert_eq!(systems_held_by_viewer(&world, true), (2, 1));
    assert_eq!(systems_held_by_viewer(&world, false), (1, 2));
}

/// The Alliance's headquarters, Tokmia, and Coruscant, the Empire's,
/// with the four characters of the victory conditions word.
fn victory_word_world() -> (GameWorld, SystemKey, SystemKey) {
    let mut world = GameWorld::default();
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(0x9200_0000),
        name: "Churba".into(),
        group: SectorGroup::Core,
        x: 0,
        y: 0,
        systems: vec![],
    });
    let system = |dat_id: u32, faction: Faction| System {
        last_arrival: None,
        dat_id: DatId::new(dat_id),
        name: String::new(),
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
        is_headquarters: true,
        is_destroyed: false,
        control: ControlKind::Controlled(faction),
    };
    let tokmia = world.systems.insert(system(0x9000_0001, Faction::Alliance));
    let coruscant = world.systems.insert(system(0x9000_0109, Faction::Empire));
    for index in [0x242, 0x240, 0x281, 0x280] {
        world.characters.insert(rebellion_core::world::Character {
            dat_id: DatId::new(index),
            ..Default::default()
        });
    }
    (world, tokmia, coruscant)
}

/// A Fast clock half way through day 45.
fn clock_inside_day_45() -> (GameClock, f32) {
    let mut clock = GameClock::new();
    clock.set_speed(GameSpeed::Fast);
    let day = GameSpeed::Fast.seconds_per_day().unwrap();
    clock.queue_days(45);
    for _ in 0..45 {
        clock.advance(0.0);
    }
    assert_eq!(clock.advance(day * 0.5), []);
    (clock, day)
}

#[test]
fn an_ending_left_waiting_does_not_reach_the_next_game() {
    // FUN_00414a80 (state 3) destroys the galaxy window that holds the
    // ending's state (+0xa8, FUN_00425c90), so a new or loaded game
    // starts with none.
    let mut pending_ending = Some(ending_cutscene(false, false));
    drop_left_ending(&mut pending_ending);
    assert_eq!(pending_ending, None);
}

#[test]
fn a_win_ordered_between_day_steps_holds_the_clock_on_the_order_s_day() {
    // FUN_005180e0 stores the word when the event that ends the game is
    // handled (Event300_GameObjExistingChanged_Handle -> FUN_00517cd0,
    // in FUN_0051df30's pass), and FUN_0041e390 reads its 0x10000000
    // (FUN_0051de80) before every sub-tick; FUN_0051dea0, the day's only
    // writer (one call, 0x41e3e9), never runs after it. A0 2026-11-17
    // edited-saves-5 run 2: the player's assault took Coruscant on day
    // 94 and the clock read 94 up to the victory film.
    use rebellion_core::victory::WORD_GAME_OVER;
    let (mut world, tokmia, coruscant) = victory_word_world();
    let conditions = VictoryConditions::HeadquartersOnly;
    let mut victory_state = VictoryState::new(tokmia, coruscant);
    let (mut clock, day) = clock_inside_day_45();
    // The player's Destroy System runs at once, half way through day 45.
    world.systems[tokmia].is_destroyed = true;
    let mut msg_log = MessageLog::default();
    let mut pending_ending = None;
    // The next frame: the word before the clock.
    settle_victory_word_between_days(
        &mut victory_state,
        &world,
        conditions,
        &std::collections::HashMap::new(),
        &mut msg_log,
        None,
        advisor_now(&clock),
        &mut pending_ending,
        false,
        clock.tick,
    );
    if galaxy_clock_runs(&GameMode::Galaxy, false, victory_state.conditions_word) {
        clock.advance(day);
    }
    assert_eq!(clock.tick, 45, "the game ends on the order's own day");
    assert_ne!(victory_state.conditions_word & WORD_GAME_OVER, 0);
    assert!(victory_state.resolved);
    // FUN_00425c90 state 0x12: the Empire player's win, MDATA.108.
    assert_eq!(pending_ending, Some(108));
}

#[test]
fn a_headquarters_lost_between_day_steps_sets_its_bit_and_film_before_the_clock_moves() {
    // The whole word settles at the event, not only its game-over bit:
    // Event300_GameObjExistingChanged_Handle (FUN_005140c0) ->
    // FUN_00517cd0 -> FUN_005180e0 stores it in FUN_0051df30's pass, and
    // FUN_0051e050 -> FUN_00518460 posts 0x128 (the caption film 102,
    // TEXTSTRA 0x7189, FUN_0048e5a0) in the same pass, before the next
    // sub-tick. In a Standard game the HQ alone does not end the game.
    use rebellion_core::victory::{WORD_GAME_OVER, WORD_HQ_LOST};
    let (mut world, tokmia, coruscant) = victory_word_world();
    let conditions = VictoryConditions::Standard;
    let mut victory_state = VictoryState::new(tokmia, coruscant);
    let (mut clock, day) = clock_inside_day_45();
    // The player's Destroy System runs at once, half way through day 45.
    world.systems[tokmia].is_destroyed = true;
    let texts = std::collections::HashMap::from([(0x7189, "HQ destroyed".to_string())]);
    let mut msg_log = MessageLog::default();
    let mut pending_ending = None;
    // The next frame: the word before the clock.
    settle_victory_word_between_days(
        &mut victory_state,
        &world,
        conditions,
        &texts,
        &mut msg_log,
        None,
        advisor_now(&clock),
        &mut pending_ending,
        false,
        clock.tick,
    );
    assert_eq!(clock.tick, 45);
    assert_eq!(victory_state.conditions_word, WORD_HQ_LOST);
    let films: Vec<u32> = msg_log.queued_story_movies().map(|m| m.movie).collect();
    assert_eq!(films, [102], "the 0x128 film is queued on the order's day");
    // The game goes on: the clock runs.
    assert!(galaxy_clock_runs(
        &GameMode::Galaxy,
        false,
        victory_state.conditions_word
    ));
    assert_eq!(clock.advance(day).len(), 1);
    assert_eq!(victory_state.conditions_word & WORD_GAME_OVER, 0);
    assert!(!victory_state.resolved);
    assert_eq!(pending_ending, None);
}

#[test]
fn the_advisor_keeps_speaking_while_a_story_movie_plays() {
    // FUN_0041cea0 gates the advisor on its object DAT_006b14e4
    // (FUN_00435a50), not on the galaxy view; the story movie runs on
    // Smacker's thread (FUN_005f7020) while FUN_00413560 keeps ticking.
    // A0 2026-11-17 edited-saves-5 h-07..h-09: MDATA.105 follows
    // MDATA.103's last frame after one strategy frame, C-3PO silent.
    assert!(advisor_runs(&GameMode::Galaxy));
    assert!(advisor_runs(&GameMode::Cutscene {
        kind: CutsceneKind::Story(103)
    }));
    // The end's movie tears the game down (FUN_00414830). port: the
    // tactical view and the menus keep the advisor still, as before.
    assert!(!advisor_runs(&GameMode::Cutscene {
        kind: CutsceneKind::Ending(105)
    }));
    assert!(!advisor_runs(&GameMode::TacticalCombat));
    assert!(!advisor_runs(&GameMode::MainMenu));
}

#[test]
fn the_battle_summarys_system_button_opens_the_systems_sector_window() {
    // FUN_00470ea0 (Battle Summary, vtable 0x65a300), command 0xcb:
    // FUN_004f3220 then FUN_00429ce0, the sector window; A0 TAC-07-C014
    // shows the Sluis sector window after the Battle at Mon Calamari.
    let mut world = GameWorld::default();
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(1),
        name: "Sluis".into(),
        group: SectorGroup::Core,
        x: 0,
        y: 0,
        systems: vec![],
    });
    let system = world.systems.insert(System {
        last_arrival: None,
        dat_id: DatId::new(2),
        name: "Mon Calamari".into(),
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
        control: ControlKind::Uncontrolled,
    });
    world.sectors[sector].systems.push(system);
    let mut sectors = SectorWindowState::default();
    assert!(open_battle_result_sector(
        &world,
        system,
        &mut sectors,
        CockpitFaction::Alliance,
    ));
    assert_eq!(sectors.window_count(), 1);
    // A second press raises the open window; it opens no other.
    assert!(open_battle_result_sector(
        &world,
        system,
        &mut sectors,
        CockpitFaction::Alliance,
    ));
    assert_eq!(sectors.window_count(), 1);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The tactical result regression keeps its opposing fleets and exact roster assertions together."
)]
fn tactical_space_results_preserve_exact_capital_and_fighter_roster_slots() {
    let mut world = GameWorld::default();
    let sector = world.sectors.insert(Sector {
        dat_id: DatId::new(1),
        name: "Test Sector".into(),
        group: SectorGroup::Core,
        x: 0,
        y: 0,
        systems: vec![],
    });
    let system = world.systems.insert(System {
        last_arrival: None,
        dat_id: DatId::new(2),
        name: "Test System".into(),
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
        control: ControlKind::Uncontrolled,
    });
    let ship_class = CapitalShipKey::default();
    let fighter_class = FighterKey::default();
    let mut preexisting_dead_ship = ShipInstance::new(ship_class, 100, true);
    preexisting_dead_ship.hull_current = 0;
    preexisting_dead_ship.alive = false;
    let attacker = world.fleets.insert(Fleet {
        location: system,
        capital_ships: vec![
            preexisting_dead_ship,
            ShipInstance::new(ship_class, 100, true),
            ShipInstance::new(ship_class, 100, true),
        ],
        fighters: vec![
            FighterEntry {
                class: fighter_class,
                count: 12,
                carrier: None,
                damage: None,
            },
            FighterEntry {
                class: fighter_class,
                count: 12,
                carrier: None,
                damage: None,
            },
        ],
        characters: vec![],
        is_alliance: true,
        has_death_star: true,
        special_forces: Vec::new(),
    });
    let defender = world.fleets.insert(Fleet {
        location: system,
        capital_ships: vec![ShipInstance::new(ship_class, 100, false)],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.systems[system].fleets = vec![attacker, defender];
    let session = rebellion_render::BattleSession {
        system,
        system_name: "Test System".into(),
        system_picture_id: 1,
        attacker_fleet: attacker,
        defender_fleet: defender,
        attacker_is_alliance: true,
        player_is_attacker: true,
        phase: rebellion_render::BattlePhase::Results,
        ships: vec![
            test_tactical_ship(ship_class, attacker, true, 1, 43, true),
            test_tactical_ship(ship_class, attacker, true, 2, 0, false),
            test_tactical_ship(ship_class, defender, false, 0, 91, true),
        ],
        fighters: vec![
            rebellion_render::tactical_view::TacticalFighter {
                class_key: fighter_class,
                identity: rebellion_render::tactical_view::TacticalObjectIdentity {
                    class_dat_id: DatId::new(3),
                    fleet_roster_index: 0,
                    is_alliance: true,
                },
                tactical_resource: rebellion_render::fighter_tactical_resource(DatId::new(3)),
                source_position: rebellion_render::tactical_view::TacticalWorldPosition {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                sublight_engine_power: 0.0,
                fleet: attacker,
                fleet_fighter_index: 0,
                fleet_squadron_index: 0,
                name: "Fighter".into(),
                x: 0.0,
                y: 0.0,
                squad_count: 12,
                hull_current: 96.0,
                hull_max: 96.0,
                shield: 0.0,
                shield_max: 0.0,
                weapon_arc: rebellion_render::tactical_view::TacticalWeaponArc::default(),
                weapon_ranges: rebellion_render::tactical_view::TacticalWeaponRanges::default(),
                torpedo_strength: 0,
                torpedo_range: 0.0,
                maneuverability: 0,
                has_hyperdrive: false,
                is_attacker: true,
                alive: true,
                fighter_group: 0,
                selected: false,
                order: rebellion_render::tactical_view::TacticalOrder::None,
                tactic: rebellion_render::tactical_view::TacticalTactic::StandOff,
                attack_target: None,
                escort_target: None,
                recovery_state:
                    rebellion_render::tactical_view::TacticalFighterRecoveryState::AwaitingCarrier,
                recovery_target: None,
                navigation_route: Vec::new(),
                home_carrier: None,
                in_hangar: false,
                retreating: false,
                retreat_progress: 0.0,
            },
            rebellion_render::tactical_view::TacticalFighter {
                class_key: fighter_class,
                identity: rebellion_render::tactical_view::TacticalObjectIdentity {
                    class_dat_id: DatId::new(3),
                    fleet_roster_index: 1,
                    is_alliance: true,
                },
                tactical_resource: rebellion_render::fighter_tactical_resource(DatId::new(3)),
                source_position: rebellion_render::tactical_view::TacticalWorldPosition {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                sublight_engine_power: 0.0,
                fleet: attacker,
                fleet_fighter_index: 1,
                fleet_squadron_index: 0,
                name: "Fighter".into(),
                x: 0.0,
                y: 0.0,
                squad_count: 0,
                hull_current: 0.0,
                hull_max: 96.0,
                shield: 0.0,
                shield_max: 0.0,
                weapon_arc: rebellion_render::tactical_view::TacticalWeaponArc::default(),
                weapon_ranges: rebellion_render::tactical_view::TacticalWeaponRanges::default(),
                torpedo_strength: 0,
                torpedo_range: 0.0,
                maneuverability: 0,
                has_hyperdrive: false,
                is_attacker: true,
                alive: true,
                fighter_group: 1,
                selected: false,
                order: rebellion_render::tactical_view::TacticalOrder::None,
                tactic: rebellion_render::tactical_view::TacticalTactic::StandOff,
                attack_target: None,
                escort_target: None,
                recovery_state:
                    rebellion_render::tactical_view::TacticalFighterRecoveryState::AwaitingCarrier,
                recovery_target: None,
                navigation_route: Vec::new(),
                home_carrier: None,
                in_hangar: false,
                retreating: false,
                retreat_progress: 0.0,
            },
        ],
        death_star: Some(rebellion_render::tactical_view::TacticalDeathStar {
            resource: rebellion_render::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: true,
            is_alliance: true,
            source_position: rebellion_render::tactical_view::TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            hull: 0.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: true,
            action_committed: false,
        }),
        death_star_beam: None,
        turn_events: Vec::new(),
        withdrawal_executors: Vec::new(),
        task_force_selection: None,
        task_force_formations: Default::default(),
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
        trench_run_participants: vec![],
        trench_run_saved: Vec::new(),
        trench_run_chatter: [0x13d; 9],
        trench_run_chatter_cursor: 0,
        trench_run_cinematic_pending: false,
        navigation_sets: std::array::from_fn(|_| Vec::new()),
        source_layout: rebellion_render::tactical_view::OriginalTacticalLayout::default(),
        selected_ship: None,
        selected_fighter_group: None,
        placement_confirmed: true,
        start_tick: 1,
        combat_tick: 1,
        tactical_rng: rebellion_render::OriginalTacticalRng::new(0x5eed_ba77),
        weapon_draws: 0,
        weapon_effects: vec![],
        impact_effects: vec![],
        pending_audio_cues: vec![],
        pending_voice_cues: vec![],
        voice_announcer: rebellion_render::tactical_view::TacticalVoiceAnnouncer::default(),
        field_effects: vec![],
        subsystem_repairs: vec![],
        paused: false,
        combat_speed: 1,
        step_accumulator: 0.0,
        paused_turn_seconds: 0.0,
        end_wait_ms: None,
        winner: Some(rebellion_render::CombatWinner::Attacker),
        retreat_allowed: [false; 2],
        retreat_done: [false; 2],
        withdrawal_ms: None,
        ai_withdrawal_ms: None,
        diagnostic_log: Vec::new(),
    };

    let battle_return =
        tactical_flow::apply_results(&session, &mut world, &mut TroopTransportState::default());

    assert_eq!(world.fleets[attacker].capital_ships.len(), 1);
    assert_eq!(world.fleets[attacker].capital_ships[0].hull_current, 43);
    assert_eq!(world.fleets[attacker].fighters[0].count, 1);
    assert_eq!(world.fleets[attacker].fighters.len(), 1);
    assert!(!world.fleets[attacker].has_death_star);
    assert_eq!(world.fleets[defender].capital_ships[0].hull_current, 91);
    assert_eq!(
        battle_return.winner,
        Some(rebellion_render::CombatWinner::Attacker)
    );
    assert_eq!(battle_return.winner_fleet, Some(attacker));
    assert!(battle_return.player_won);
}

fn test_tactical_ship(
    class_key: CapitalShipKey,
    fleet: rebellion_core::ids::FleetKey,
    is_attacker: bool,
    fleet_ship_index: usize,
    hull_current: i32,
    alive: bool,
) -> rebellion_render::tactical_view::TacticalShip {
    rebellion_render::tactical_view::TacticalShip {
        class_key,
        identity: rebellion_render::tactical_view::TacticalObjectIdentity {
            class_dat_id: DatId::new(1),
            fleet_roster_index: fleet_ship_index,
            is_alliance: is_attacker,
        },
        tactical_resource: rebellion_render::capital_ship_tactical_resource(DatId::new(1)),
        death_star_resource: rebellion_render::death_star_tactical_resource(DatId::new(1)),
        source_position: rebellion_render::tactical_view::TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        source_forward: rebellion_render::tactical_view::TacticalWorldVector {
            x: 0.0,
            y: 0.0,
            z: if is_attacker { 1.0 } else { -1.0 },
        },
        source_desired_forward: rebellion_render::tactical_view::TacticalWorldVector {
            x: 0.0,
            y: 0.0,
            z: if is_attacker { 1.0 } else { -1.0 },
        },
        source_velocity: rebellion_render::tactical_view::TacticalWorldVector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        source_waypoint: None,
        navigation_route: Vec::new(),
        source_collision_envelope: None,
        name: "Ship".into(),
        x: 0.0,
        y: 0.0,
        hull_current,
        hull_max: 100,
        shield: 0,
        shield_max: 0,
        subsystem_condition: rebellion_render::tactical_view::TacticalSubsystemCondition {
            shields: 0,
            weapons: 0,
            tractor: 0,
            engines: 0,
            hyperdrive: 0,
        },
        subsystem_capacity: rebellion_render::tactical_view::TacticalSubsystemCapacity {
            shields: false,
            weapons: false,
            tractor: false,
            engines: false,
            hyperdrive: 0,
        },
        subsystem_damage: rebellion_render::tactical_view::TacticalSubsystemDamage::default(),
        sublight_engine_power: 0.0,
        maneuverability: 0,
        tractor_beam_power: 0.0,
        tractor_beam_range: 0.0,
        tractor_request_pending: false,
        ion_engine_stun: 0.0,
        hangar: Vec::new(),
        hangar_launch_stamp: 0.0,
        fighter_capacity: 0,
        maneuver_state_value: None,
        damage_control: 0,
        is_attacker,
        alive,
        selected: false,
        order: rebellion_render::tactical_view::TacticalOrder::None,
        tactic: rebellion_render::tactical_view::TacticalTactic::StandOff,
        task_force: u8::try_from(fleet_ship_index.min(7)).unwrap_or(7),
        fleet,
        fleet_ship_index,
        sprite_id: None,
        weapon_arcs: [rebellion_render::tactical_view::TacticalWeaponArc::default(); 4],
        weapon_ranges: rebellion_render::tactical_view::TacticalWeaponRanges::default(),
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
