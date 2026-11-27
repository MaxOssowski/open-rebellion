use super::*;

#[test]
fn an_injury_posts_injured_and_full_healing_posts_healed() {
    // FUN_0048fdf0 (notification 0x106): TEXTSTRA 0x7160/0x7161 with a
    // non-zero +0x94, 0x7162/0x7163 at 0; pictures 0x41a and 0x41b.
    let mut world = GameWorld::default();
    let character = world.characters.insert(rebellion_core::world::Character {
        name: "Han Solo".into(),
        is_alliance: true,
        injury: 7,
        ..rebellion_core::world::Character::default()
    });
    let mut log = MessageLog::new();
    push_character_health(&mut log, &world, character, 3, &Default::default());
    world.characters[character].injury = 0;
    push_character_health(&mut log, &world, character, 9, &Default::default());
    let messages = log.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages[0].text,
        "Han Solo Injured: Han Solo has been injured."
    );
    assert_eq!(messages[0].picture.map(|p| p.background), Some(0x41a));
    assert_eq!(
        messages[1].text,
        "Han Solo Healed: Han Solo has been healed of all injuries."
    );
    assert_eq!(messages[1].picture.map(|p| p.background), Some(0x41b));
    // The Healed picture carries the character's portrait
    // (FUN_004c5000), found by its record and major flag: Vader is an
    // Empire major character (MJCHARSD 641).
    let vader = world.characters.insert(rebellion_core::world::Character {
        dat_id: rebellion_core::ids::DatId::new(0x281),
        name: "Darth Vader".into(),
        is_empire: true,
        is_major: true,
        ..rebellion_core::world::Character::default()
    });
    push_character_health(&mut log, &world, vader, 10, &Default::default());
    let portrait = log.messages()[2].picture.and_then(|p| p.foreground);
    assert!(portrait.is_some());
    assert_eq!(
        portrait,
        rebellion_render::story_messages::character_portrait(
            rebellion_core::ids::DatId::new(0x281),
            true
        )
    );
}

#[test]
fn character_health_reaches_the_other_side_when_it_holds_a_copy() {
    // FUN_0048fdf0 builds the message for the reader's side copy of
    // the character (FUN_004f2d10 on +0x20) and drops it without one.
    // A0 2026-11-18-edited-saves-9 c-01 (75fa03d7fe70), c-04
    // (38812467bf31): the Empire, whose Finder lists Han at Umgul,
    // reads "Han Solo Injured".
    let mut world = GameWorld::default();
    let han = world.characters.insert(rebellion_core::world::Character {
        dat_id: rebellion_core::ids::DatId::new(0x243),
        name: "Han Solo".into(),
        is_alliance: true,
        injury: 7,
        ..rebellion_core::world::Character::default()
    });
    let mut log = MessageLog::new();
    push_character_health(&mut log, &world, han, 3, &Default::default());
    world.object_views.empire_characters.insert(
        rebellion_core::ids::DatId::new(0x243),
        rebellion_core::object_views::CharacterCopy::default(),
    );
    push_character_health(&mut log, &world, han, 4, &Default::default());
    let audiences: Vec<_> = log
        .messages()
        .iter()
        .map(|message| message.audience)
        .collect();
    assert_eq!(audiences, [RailAudience::Alliance, RailAudience::Both]);
}

#[test]
fn character_health_reads_its_textstra_title_and_body_and_a_death() {
    // FUN_0048fdf0: 0x7160 / 0x7161 as title and body; a destroyed
    // character (+0x50 bit 3) reads 0x7164 / 0x7365 over 0x42e (side 1)
    // or 0x433.
    let mut world = GameWorld::default();
    let han = world.characters.insert(rebellion_core::world::Character {
        name: "Han Solo".into(),
        is_alliance: true,
        injury: 7,
        ..rebellion_core::world::Character::default()
    });
    let texts = std::collections::HashMap::from([
        (0x7160, "{1:1} Injured".to_owned()),
        (0x7161, "{1:1} has been injured.".to_owned()),
        (0x7164, "{1:1} Killed".to_owned()),
        (0x7365, "{1:1} has been killed.".to_owned()),
    ]);
    let mut log = MessageLog::new();
    push_character_health(&mut log, &world, han, 3, &texts);
    world.characters[han].mark_killed();
    push_character_health(&mut log, &world, han, 4, &texts);
    let messages = log.messages();
    assert_eq!(messages[0].title.as_deref(), Some("Han Solo Injured"));
    assert_eq!(messages[0].text, "Han Solo has been injured.");
    assert_eq!(messages[1].title.as_deref(), Some("Han Solo Killed"));
    assert_eq!(messages[1].text, "Han Solo has been killed.");
    assert_eq!(messages[1].picture.map(|p| p.background), Some(0x42e));
}

#[test]
fn a_detection_injury_posts_character_injured() {
    // FUN_0048fdf0: an injury the detection deals (advance effects)
    // posts 0x7160/0x7161; a zero roll posts nothing.
    let mut world = GameWorld::default();
    let han = world.characters.insert(rebellion_core::world::Character {
        name: "Han Solo".into(),
        is_alliance: true,
        ..rebellion_core::world::Character::default()
    });
    let wedge = world.characters.insert(rebellion_core::world::Character {
        name: "Wedge Antilles".into(),
        is_alliance: true,
        ..rebellion_core::world::Character::default()
    });
    let effects = [
        MissionEffect::CharacterInjured {
            character: wedge,
            injury: 0,
        },
        MissionEffect::CharacterInjured {
            character: han,
            injury: 4,
        },
    ];
    rebellion_core::missions::apply_advance_effects(&mut world, &effects);
    let mut log = MessageLog::new();
    push_advance_messages(&mut log, &world, &effects, &[], 5, &Default::default());
    assert_eq!(world.characters[han].injury, 4);
    assert_eq!(log.messages().len(), 1);
    assert_eq!(
        log.messages()[0].text,
        "Han Solo Injured: Han Solo has been injured."
    );
}

/// The live loop's order: read the Force changes, store the effects,
/// post the messages.
fn advance_messages(world: &mut GameWorld, effects: &[MissionEffect]) -> MessageLog {
    let texts = std::collections::HashMap::from([
        (0x7160, "{1:1} Injured".to_owned()),
        (0x7161, "{1:1} has been injured.".to_owned()),
        (0x7180, "{1:1} Force Growth".to_owned()),
        (0x7181, "New insights.".to_owned()),
    ]);
    let changes = advance_force_changes(world, effects);
    rebellion_core::missions::apply_advance_effects(world, effects);
    let mut log = MessageLog::new();
    push_advance_messages(&mut log, world, effects, &changes, 5, &texts);
    log
}

#[test]
fn each_force_gain_of_the_detection_posts_its_own_force_growth_in_effect_order() {
    // FUN_0056f8b0 / FUN_0056ffa0 (the evade) and FUN_00560200 (the
    // encounter's lessons) write ForceExperience through FUN_004ee150,
    // whose hook (slot +0x320, FUN_004f1700) runs FUN_004eee40 ->
    // FUN_004ee0d0 -> FUN_004f0220 -> slot +0x394 FUN_004f1e00: 0x1e1
    // with that write's old and new Force. FUN_0048ed80 builds nothing
    // from Force 0.
    let mut world = GameWorld::default();
    let luke = world.characters.insert(rebellion_core::world::Character {
        name: "Luke Skywalker".into(),
        is_alliance: true,
        is_major: true,
        force_base: 50,
        ..rebellion_core::world::Character::default()
    });
    let han = world.characters.insert(rebellion_core::world::Character {
        name: "Han Solo".into(),
        is_alliance: true,
        ..rebellion_core::world::Character::default()
    });
    let leia = world.characters.insert(rebellion_core::world::Character {
        name: "Leia Organa".into(),
        is_alliance: true,
        is_major: true,
        ..rebellion_core::world::Character::default()
    });
    let effects = [
        MissionEffect::ForceExperienceGained {
            character: luke,
            amount: 1,
        },
        MissionEffect::CharacterInjured {
            character: han,
            injury: 4,
        },
        MissionEffect::ForceExperienceGained {
            character: luke,
            amount: 1,
        },
        MissionEffect::ForceExperienceGained {
            character: leia,
            amount: 1,
        },
    ];
    assert_eq!(
        advance_force_changes(&world, &effects),
        [(0, luke, 50, 51), (2, luke, 51, 52), (3, leia, 0, 1)]
    );
    let log = advance_messages(&mut world, &effects);
    let titles: Vec<_> = log
        .messages()
        .iter()
        .map(|m| m.title.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(
        titles,
        [
            "Luke Skywalker Force Growth",
            "Han Solo Injured",
            "Luke Skywalker Force Growth"
        ]
    );
    assert_eq!(world.characters[luke].force_level(), 52);
}

#[test]
fn dagobahs_end_in_the_advance_posts_lukes_force_growth() {
    // FUN_005751d0 (step 0xb): Luke's ForceExperience rises through
    // FUN_004ee150, which posts 0x1e1 (A0 alliance-game2/
    // luke-force-growth, day 643), before Luke_SetDagobahCompleted.
    let mut world = GameWorld::default();
    let luke = world.characters.insert(rebellion_core::world::Character {
        name: "Luke Skywalker".into(),
        is_alliance: true,
        is_major: true,
        force_base: 50,
        ..rebellion_core::world::Character::default()
    });
    let log = advance_messages(
        &mut world,
        &[MissionEffect::DagobahCompleted {
            character: luke,
            force_gain: 30,
        }],
    );
    assert_eq!(log.messages().len(), 1);
    assert_eq!(
        log.messages()[0].title.as_deref(),
        Some("Luke Skywalker Force Growth")
    );
    assert!(world.characters[luke].dagobah_completed);
}

#[test]
fn a_force_discovery_posts_future_jedi_discovered_to_the_trainers_side() {
    // FUN_0048f1e0 (notification 0x362): 0x7178 with 0x717a below Force
    // 100 (GNPRTB 2581, absent here, so 0), on the Empire's rail only.
    let mut world = GameWorld::default();
    let vader = world.characters.insert(rebellion_core::world::Character {
        dat_id: rebellion_core::ids::DatId::new(0x281),
        name: "Darth Vader".into(),
        is_empire: true,
        is_major: true,
        is_jedi_trainer: true,
        force_base: 120,
        ..rebellion_core::world::Character::default()
    });
    let klev = world.characters.insert(rebellion_core::world::Character {
        name: "Klev".into(),
        is_empire: true,
        ..rebellion_core::world::Character::default()
    });
    let texts = std::collections::HashMap::from([
        (0x7178, "Future Jedi Discovered".to_owned()),
        (0x7179, "I have detected that {1:1} has the ability to use the Force.  We should consider allowing me to conduct training in the way of the Force.".to_owned()),
    ]);
    let mut log = MessageLog::new();
    push_force_discoveries(
        &mut log,
        &world,
        &[rebellion_core::force_discovery::ForceDiscovered {
            character: klev,
            discoverer: vader,
            force_base: None,
            learns_heritage: false,
            tick: 4,
        }],
        &texts,
    );
    let messages = log.messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].title.as_deref(), Some("Future Jedi Discovered"));
    assert!(messages[0].text.starts_with("I have detected that Klev"));
    assert_eq!(messages[0].audience, RailAudience::Empire);
}

#[test]
fn a_move_posts_personnel_arrive_to_its_side() {
    // FUN_004955b0 (notification 0x200): 0x70c1 / 0x70c4 for a major
    // leader with company, on the side's Mission rail, background 0x412.
    use rebellion_core::missions::{MissionMember, PersonnelArrival};
    let mut world = GameWorld::default();
    let klatooine = world.systems.insert(rebellion_core::world::System {
        last_arrival: None,
        dat_id: rebellion_core::ids::DatId::new(2),
        name: "Klatooine".into(),
        sector: rebellion_core::ids::SectorKey::default(),
        x: 0,
        y: 0,
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
        control: rebellion_core::world::ControlKind::Uncontrolled,
    });
    let mut add = |name: &str, is_major, dat_id| {
        world.characters.insert(rebellion_core::world::Character {
            dat_id: rebellion_core::ids::DatId::new(dat_id),
            name: name.into(),
            is_alliance: true,
            is_major,
            ..rebellion_core::world::Character::default()
        })
    };
    // MJCHARSD 0x240 is Mon Mothma, the speaker map's 0x17.
    let mothma = add("Mon Mothma", true, 0x240);
    let afyon = add("Afyon", false, 0x3000_0001);
    let texts = std::collections::HashMap::from([
        (0x70c0, "Personnel Arrive at {2:1}".to_owned()),
        (0x70c1, "{1:1} Arrives at {2:1}".to_owned()),
        (0x70c2, "\n{1:1}".to_owned()),
        (
            0x70c3,
            "The following personnel have arrived at {2:1}.\n".to_owned(),
        ),
        (
            0x70c4,
            "I have arrived at {2:1}.  The following personnel have arrived with me:".to_owned(),
        ),
    ]);
    let arrival = |members: Vec<MissionMember>| PersonnelArrival {
        mission_id: 1,
        faction: MissionFaction::Alliance,
        system: klatooine,
        members,
        tick: 7,
    };
    let mut log = MessageLog::new();
    push_personnel_arrivals(
        &mut log,
        &world,
        &[
            arrival(vec![
                MissionMember::Character(mothma),
                MissionMember::Character(afyon),
            ]),
            arrival(vec![MissionMember::Character(afyon)]),
        ],
        &texts,
    );
    let messages = log.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages[0].title.as_deref(),
        Some("Mon Mothma Arrives at Klatooine")
    );
    assert_eq!(
        messages[0].text,
        "I have arrived at Klatooine.  The following personnel have arrived with me:\nAfyon"
    );
    assert_eq!(messages[0].audience, RailAudience::Alliance);
    assert_eq!(messages[0].rail, Some(MessageRail::Mission));
    assert_eq!(messages[0].picture.map(|p| p.background), Some(0x412));
    assert_eq!(messages[0].advisor_event, [0x17, 0x17]);
    assert!(messages[0].picture.and_then(|p| p.foreground).is_some());
    assert_eq!(
        messages[1].title.as_deref(),
        Some("Personnel Arrive at Klatooine")
    );
    assert_eq!(
        messages[1].text,
        "The following personnel have arrived at Klatooine.\n\nAfyon"
    );
    assert_eq!(messages[1].advisor_event, [0x15, 0x15]);
}

#[test]
fn informants_tell_their_side_of_the_system_on_the_popular_support_rail() {
    // FUN_00491060 (notification 0x153): 0x7150 / 0x7151, picture
    // 0x3e8, event 0x1c, Display lit (A0 informants-clean/
    // test-day1457-selonia, an unedited Alliance game).
    let world = GameWorld::default();
    let texts = std::collections::HashMap::from([
        (0x7150, "Informants Provide Information".to_owned()),
        (
            0x7151,
            "Informants have provided us with information about {1:1}.".to_owned(),
        ),
    ]);
    let at = rebellion_core::ids::SystemKey::default();
    let message = informants_message(&world, at, true, 3, &texts).unwrap();
    assert_eq!(
        message.title.as_deref(),
        Some("Informants Provide Information")
    );
    assert_eq!(
        message.text,
        "Informants have provided us with information about unknown."
    );
    assert_eq!(message.rail, Some(MessageRail::PopularSupport));
    assert_eq!(message.audience, RailAudience::Alliance);
    assert_eq!(message.picture.map(|p| p.background), Some(0x3e8));
    assert_eq!(message.advisor_event, [0x1c, 0x1c]);
    assert_eq!(message.system, Some(at));
    assert_eq!(
        informants_message(&world, at, false, 3, &texts).map(|m| m.audience),
        Some(RailAudience::Empire)
    );
    assert!(informants_message(&world, at, true, 3, &Default::default()).is_none());
}

#[test]
fn a_maintenance_shortfall_in_transit_names_no_system_but_keeps_its_target() {
    // A0 maintenance-shortfall/test-day1224-corvette-no-system (an
    // unedited Alliance game): "The following units have been destroyed
    // at  due to a maintenance shortfall:" over "Corellian Corvette 2",
    // picture 1013, Display lit; FUN_00498970 finds no system up a
    // fleet in hyperspace.
    let world = GameWorld::default();
    let texts = std::collections::HashMap::from([
        (0x7080, "Maintenance Shortfall".to_owned()),
        (
            0x7081,
            "The following units have been destroyed at {3:1} due to a maintenance shortfall:\n"
                .to_owned(),
        ),
        (0x7082, "{1:1}\n".to_owned()),
    ]);
    let lost = maintenance_message(&world, None, "Corellian Corvette 2", true, 9, &texts).unwrap();
    assert_eq!(
        lost.text,
        "The following units have been destroyed at  due to a maintenance shortfall:\nCorellian Corvette 2\n"
    );
    assert_eq!(lost.system, None);
    assert!(lost.unplaced_target);
    assert_eq!(lost.rail, Some(MessageRail::Manufacturing));
    assert_eq!(lost.audience, RailAudience::Alliance);
    assert_eq!(lost.picture.map(|p| p.background), Some(0x3f5));
    assert_eq!(lost.advisor_event, [0xc, 0xc]);
    let placed = maintenance_message(
        &world,
        Some(rebellion_core::ids::SystemKey::default()),
        "TIE Fighter",
        false,
        9,
        &texts,
    )
    .unwrap();
    assert_eq!(
        placed.system,
        Some(rebellion_core::ids::SystemKey::default())
    );
    assert!(!placed.unplaced_target);
    assert_eq!(placed.picture.map(|p| p.background), Some(0x3f6));
}

#[test]
fn a_squadron_back_at_full_strength_reports_to_its_side_by_its_container() {
    // FUN_00499de0, a squadron (families 0x1c..0x1f): TEXTSTRA 0x7040 /
    // 0x7042 name it and its container, +0x30 0x457 + (side != 1),
    // +0x28 9; aboard a ship it names the ship's fleet on the Fleet rail
    // (+0x34 0x80), on a planet the system on the Defense rail (0x40).
    use rebellion_core::squadrons::SquadronPlace;
    use rebellion_core::world::{FighterClass, FighterEntry, Fleet};
    let texts: std::collections::HashMap<u16, String> = [
        (0x7040, "Squadron at Full Strength"),
        (
            0x7042,
            "The {1:1} Squadron attached to {2:1} has replaced all fighters lost in combat.",
        ),
    ]
    .into_iter()
    .map(|(id, text)| (id, text.to_owned()))
    .collect();
    let mut world = GameWorld::default();
    let system = world.systems.insert(rebellion_core::world::System {
        last_arrival: None,
        dat_id: rebellion_core::ids::DatId::new(0x9000_0000),
        name: "Kuat".into(),
        sector: rebellion_core::ids::SectorKey::default(),
        x: 0,
        y: 0,
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
        control: rebellion_core::world::ControlKind::Uncontrolled,
    });
    let tie = world.fighter_classes.insert(FighterClass {
        name: "TIE Fighter".into(),
        is_empire: true,
        squadron_size: 12,
        ..FighterClass::default()
    });
    let entry = FighterEntry {
        class: tie,
        count: 1,
        carrier: None,
        damage: None,
    };
    world.systems[system].fighters.push(entry.clone());
    let fleet = world.fleets.insert(Fleet {
        location: system,
        capital_ships: vec![],
        fighters: vec![entry],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.systems[system].fleets.push(fleet);

    let planet =
        squadron_full_strength_message(&world, SquadronPlace::Planet(system), 0, 5, &texts)
            .unwrap();
    assert_eq!(planet.title.as_deref(), Some("Squadron at Full Strength"));
    assert_eq!(
        planet.text,
        "The TIE Fighter Squadron attached to Kuat has replaced all fighters lost in combat."
    );
    assert_eq!(planet.rail, Some(MessageRail::Defense));
    assert_eq!(planet.audience, RailAudience::Empire);
    assert_eq!(planet.sounds.strategy, 0x458);
    assert_eq!(planet.system, Some(system));

    let aboard =
        squadron_full_strength_message(&world, SquadronPlace::Fleet(fleet), 0, 5, &texts).unwrap();
    assert_eq!(aboard.rail, Some(MessageRail::Fleet));
    let label = rebellion_render::system_window::fleet_label(&world, fleet).unwrap_or_default();
    assert!(
        aboard.text.contains(&format!("attached to {label} has")),
        "{}",
        aboard.text
    );
    assert!(
        squadron_full_strength_message(&world, SquadronPlace::Fleet(fleet), 1, 5, &texts).is_none()
    );
}

#[test]
fn a_research_complete_opens_its_class_in_the_encyclopedia_wherever_it_comes_from() {
    // FUN_00499aa0's class 0x0065bb40 is kind 4 (vtable +0x10 0x5a6700):
    // Display opens the class's Encyclopedia entry (FUN_00467f10 ->
    // FUN_0041d6b0), as A0 research-complete/02 does for the Nebulon-B
    // Frigate. Research missions and the economy share this builder.
    use rebellion_core::ids::DatId;
    use rebellion_core::research::TechType;
    use rebellion_core::world::CapitalShipClass;
    let texts: std::collections::HashMap<u16, String> = [
        (0x7048, "{1:1} Research Complete"),
        (
            0x7049,
            "R&D Reports that the {1:1} is now available for manufacture.",
        ),
    ]
    .into_iter()
    .map(|(id, text)| (id, text.to_owned()))
    .collect();
    let mut world = GameWorld::default();
    world.capital_ship_classes.insert(CapitalShipClass {
        dat_id: DatId::new(0x1400_0002),
        name: "Nebulon-B Frigate".into(),
        is_alliance: true,
        research_order: 1,
        refined_material_cost: 10,
        ..CapitalShipClass::default()
    });
    let message = research_complete_message(&world, true, TechType::Ship, 1, 7, &texts).unwrap();
    assert_eq!(
        message.title.as_deref(),
        Some("Nebulon-B Frigate Research Complete")
    );
    assert_eq!(
        message.text,
        "R&D Reports that the Nebulon-B Frigate is now available for manufacture."
    );
    assert_eq!(message.encyclopedia, Some(0x1400_0002));
    assert_eq!(message.rail, Some(MessageRail::Manufacturing));
    assert_eq!(message.audience, RailAudience::Alliance);
    assert!(research_complete_message(&world, true, TechType::Ship, 2, 7, &texts).is_none());
}

#[test]
fn a_blockade_a_battle_leaves_posts_no_blockade_message() {
    // FUN_004960f0 is silent while +0x88 bit 0x4000 is set, which
    // FUN_0050a9e0 sets as the blockade begins in the pass the battle
    // bit changes (A0 battle-clean/, an unedited Alliance game: no
    // Blockade listed after the battles at Yavin and Umgul). Without a
    // battle the holder reads 0x70a2 / 0x70a3 on the Conflict rail.
    let mut world = GameWorld::default();
    let umgul = world.systems.insert(rebellion_core::world::System {
        last_arrival: None,
        dat_id: rebellion_core::ids::DatId::new(2),
        name: "Umgul".into(),
        sector: rebellion_core::ids::SectorKey::default(),
        x: 0,
        y: 0,
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
        control: rebellion_core::world::ControlKind::Controlled(Faction::Alliance),
    });
    let fleet = world.fleets.insert(rebellion_core::world::Fleet {
        location: umgul,
        capital_ships: vec![],
        fighters: vec![],
        characters: vec![],
        is_alliance: false,
        has_death_star: false,
        special_forces: Vec::new(),
    });
    world.systems[umgul].fleets.push(fleet);
    let texts = std::collections::HashMap::from([
        (0x70a2, "{1:1} Under Blockade".to_owned()),
        (
            0x70a3,
            "{1:4} ships have been detected at {2:1}.  The world is under enemy blockade."
                .to_owned(),
        ),
        (0x4d70, "Alliance".to_owned()),
        (0x4d71, "Imperial".to_owned()),
    ]);
    let message = blockade_started_message(&world, umgul, false, true, 5, &texts).unwrap();
    assert_eq!(message.title.as_deref(), Some("Umgul Under Blockade"));
    assert_eq!(message.rail, Some(MessageRail::Conflict));
    assert_eq!(message.audience, RailAudience::Alliance);
    assert_eq!(message.system, Some(umgul));
    let quiet = blockade_started_message(&world, umgul, true, true, 5, &texts).unwrap();
    assert_eq!(quiet.rail, None);
    assert_eq!(quiet.title, None);
    assert_eq!(quiet.text, "Blockade established at Umgul");
    // Without the texts, the port's line on both sides' Conflict rail;
    // with them, a neutral holder's class is silent.
    let bare =
        blockade_started_message(&world, umgul, false, true, 5, &Default::default()).unwrap();
    assert_eq!(bare.title, None);
    assert_eq!(bare.audience, RailAudience::Both);
    assert_eq!(bare.rail, Some(MessageRail::Conflict));
    world.systems[umgul].control = rebellion_core::world::ControlKind::Uncontrolled;
    assert!(blockade_started_message(&world, umgul, false, true, 5, &texts).is_none());
}

#[test]
fn saboteurs_strike_goes_to_the_victims_side_on_the_conflict_rail() {
    // FUN_0048b9a0: 0x7083..0x7085, category 0x100, event 0xc, the
    // picture by the reader (FUN_0048bbf0). A0 saboteurs-clean/
    // test-day651-saboteurs-yavin (an unedited Alliance game): the
    // Alliance's regiment at Yavin, "Saboteurs Strike at Yavin" on the
    // Conflict list, picture STRATEGY 1013 (0x3f5) to the pixel.
    let mut world = GameWorld::default();
    let yavin = world.systems.insert(rebellion_core::world::System {
        last_arrival: None,
        dat_id: rebellion_core::ids::DatId::new(2),
        name: "Yavin".into(),
        sector: rebellion_core::ids::SectorKey::default(),
        x: 0,
        y: 0,
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
        control: rebellion_core::world::ControlKind::Controlled(Faction::Alliance),
    });
    let texts = std::collections::HashMap::from([
        (0x7083, "Saboteurs Strike at {3:1}".to_owned()),
        (
            0x7084,
            "The following units were destroyed by saboteurs at {3:1}:\n".to_owned(),
        ),
        (0x7085, "{1:1}\n".to_owned()),
    ]);
    let victim = Some((Faction::Alliance, "Alliance Army Regiment".to_owned()));
    let message = saboteurs_strike_message(&world, victim, yavin, 651, &texts).unwrap();
    assert_eq!(message.title.as_deref(), Some("Saboteurs Strike at Yavin"));
    assert_eq!(
        message.text,
        "The following units were destroyed by saboteurs at Yavin:\nAlliance Army Regiment\n"
    );
    assert_eq!(message.audience, RailAudience::Alliance);
    assert_eq!(message.rail, Some(MessageRail::Conflict));
    assert_eq!(message.picture.map(|p| p.background), Some(0x3f5));
    assert_eq!(message.advisor_event, [0xc, 0xc]);
    assert_eq!(message.system, Some(yavin));
    // The Empire's unit goes to the Empire with 0x3f6; a neutral one
    // posts nothing, nor does a gone one.
    let imperial = Some((Faction::Empire, "Stormtrooper Regiment".to_owned()));
    let message = saboteurs_strike_message(&world, imperial, yavin, 651, &texts).unwrap();
    assert_eq!(message.audience, RailAudience::Empire);
    assert_eq!(message.picture.map(|p| p.background), Some(0x3f6));
    let neutral = Some((Faction::Neutral, "Mine".to_owned()));
    assert!(saboteurs_strike_message(&world, neutral, yavin, 651, &texts).is_none());
    assert!(saboteurs_strike_message(&world, None, yavin, 651, &texts).is_none());
}

#[test]
fn the_headquarters_arrival_goes_to_the_alliances_defense_rail() {
    // FUN_004981c0 case 4: 0x707c / 0x707d, category 0x40, 0x3fe,
    // event 6, the Alliance only.
    let mut world = GameWorld::default();
    let dantooine = world.systems.insert(rebellion_core::world::System {
        last_arrival: None,
        dat_id: rebellion_core::ids::DatId::new(2),
        name: "Dantooine".into(),
        sector: rebellion_core::ids::SectorKey::default(),
        x: 0,
        y: 0,
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
        control: rebellion_core::world::ControlKind::Uncontrolled,
    });
    let texts = std::collections::HashMap::from([
        (0x707c, "Headquarters Arrives".to_owned()),
        (
            0x707d,
            "The Alliance Headquarters has arrived at {2:1}".to_owned(),
        ),
    ]);
    let transit = rebellion_core::headquarters_move::HeadquartersTransit {
        origin: dantooine,
        destination: dantooine,
        arrival_tick: 30,
    };
    let message = headquarters_arrival_message(&world, &transit, &texts).unwrap();
    assert_eq!(message.title.as_deref(), Some("Headquarters Arrives"));
    assert_eq!(
        message.text,
        "The Alliance Headquarters has arrived at Dantooine"
    );
    assert_eq!(message.audience, RailAudience::Alliance);
    assert_eq!(message.rail, Some(MessageRail::Defense));
    assert_eq!(message.picture.map(|p| p.background), Some(0x3fe));
    assert_eq!(message.advisor_event, [6, 6]);
    assert_eq!(message.system, Some(dantooine));
    assert!(headquarters_arrival_message(&world, &transit, &Default::default()).is_none());
}

#[test]
fn an_escape_frees_the_captive_and_tells_both_sides() {
    // FUN_00490340 cases 1 and 2: the captor reads 0x715c / 0x715d,
    // the escapee's side 0x715e / 0x715f.
    let mut world = GameWorld::default();
    let leia = world.characters.insert(rebellion_core::world::Character {
        name: "Leia Organa".into(),
        is_alliance: false,
        is_empire: true,
        is_captive: true,
        captured_by: Some(Faction::Empire),
        current_system: Some(rebellion_core::ids::SystemKey::default()),
        ..rebellion_core::world::Character::default()
    });
    let texts = std::collections::HashMap::from([
        (0x715c, "{1:1} Escaped".to_owned()),
        (0x715d, "{1:1} escaped from the {3:1} at {2:1}.".to_owned()),
        (0x715e, "{1:1} Escaped".to_owned()),
        (
            0x715f,
            "{1:1} has escaped from imprisonment at {2:1}.".to_owned(),
        ),
    ]);
    let mut log = MessageLog::new();
    escape_character(&mut world, leia, true, 4, &mut log, &texts);
    let c = &world.characters[leia];
    assert!(c.is_alliance && !c.is_empire && !c.is_captive && c.captured_by.is_none());
    let messages = log.messages();
    assert_eq!(messages.len(), 2);
    // The escapee's side reads case 1, the captor's case 2 (A0
    // empire-clean/31-needa-captured settles the side test).
    assert_eq!(messages[0].audience, RailAudience::Alliance);
    assert!(
        messages[0].text.contains("escaped from the"),
        "{}",
        messages[0].text
    );
    assert_eq!(messages[1].audience, RailAudience::Empire);
    assert!(
        messages[1].text.contains("has escaped from imprisonment"),
        "{}",
        messages[1].text
    );
    assert_eq!(messages[1].picture.map(|p| p.background), Some(0x405));
    // Each names the place it was held at, for Display.
    assert!(messages
        .iter()
        .all(|m| m.system == Some(rebellion_core::ids::SystemKey::default())));
    // With no captor recorded, the captor is the side it escaped from.
    let wedge = world.characters.insert(rebellion_core::world::Character {
        name: "Wedge".into(),
        is_captive: true,
        ..rebellion_core::world::Character::default()
    });
    let mut log = MessageLog::new();
    escape_character(&mut world, wedge, true, 5, &mut log, &texts);
    assert_eq!(log.messages()[1].audience, RailAudience::Empire);
    assert!(
        log.messages()[1]
            .text
            .contains("has escaped from imprisonment"),
        "{}",
        log.messages()[1].text
    );
}

#[test]
fn a_return_place_is_named_by_its_system() {
    // FUN_004927c0's 0x70e6 names the container's name (+0x30).
    use rebellion_core::missions::ReturnPlace;
    let mut world = GameWorld::default();
    let coruscant = world.systems.insert(rebellion_core::world::System {
        last_arrival: None,
        dat_id: rebellion_core::ids::DatId::new(2),
        name: "Coruscant".into(),
        sector: rebellion_core::ids::SectorKey::default(),
        x: 0,
        y: 0,
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
        control: rebellion_core::world::ControlKind::Uncontrolled,
    });
    assert_eq!(
        return_place_name(&world, ReturnPlace::System(coruscant)),
        Some("Coruscant".into())
    );
    assert_eq!(
        return_place_name(&world, ReturnPlace::System(Default::default())),
        None
    );
}

#[test]
fn a_report_names_its_leader_target_and_recruit_and_sees_the_target_killed() {
    // FUN_004927c0 reads +0x74, +0x70 and +0xa8; +0x50 bit 3 is a
    // destroyed target.
    use rebellion_core::missions::{MissionOutcome, MissionResult};
    let mut world = GameWorld::default();
    let mut add = |name: &str, is_major| {
        world.characters.insert(rebellion_core::world::Character {
            name: name.into(),
            is_major,
            ..rebellion_core::world::Character::default()
        })
    };
    let leia = add("Leia Organa", true);
    let dodonna = add("Jan Dodonna", false);
    let wedge = add("Wedge Antilles", false);
    let other = add("Other", false);
    let mut result = MissionResult {
        mission_id: 1,
        tick: 3,
        kind: MissionKind::Assassination,
        faction: MissionFaction::Alliance,
        character: Some(leia),
        lead_force: None,
        target_system: Default::default(),
        target_character: Some(dodonna),
        target_object: None,
        returning_to: None,
        outcome: MissionOutcome::Success,
        effects: vec![
            MissionEffect::CharacterKilled {
                character: other,
                faction: MissionFaction::Empire,
            },
            MissionEffect::CharacterRecruited {
                character: wedge,
                system: Default::default(),
                faction: MissionFaction::Alliance,
                pool_emptied: false,
            },
        ],
    };
    assert_eq!(
        report_names(&world, &result),
        (
            Some(("Leia Organa".into(), true)),
            Some("Jan Dodonna".into()),
            Some("Wedge Antilles".into()),
            false
        )
    );
    result.effects.push(MissionEffect::CharacterKilled {
        character: dodonna,
        faction: MissionFaction::Empire,
    });
    assert!(report_names(&world, &result).3);
    result.effects.clear();
    result.character = None;
    assert_eq!(
        report_names(&world, &result),
        (None, Some("Jan Dodonna".into()), None, false)
    );
}

#[test]
fn a_recon_led_by_a_special_force_names_the_force_as_its_leader() {
    // FUN_004927c0 reads the leader +0x74 through FUN_004f2d10, which
    // resolves a special force as well as a character; case 0x54
    // reports only when it resolves. A0 2026-11-16-partial-cells/r-9
    // (sha256 f519dea4) is the Imperial Probe Droid's report.
    use rebellion_core::missions::{MissionOutcome, MissionResult};
    let mut world = GameWorld::default();
    let class = rebellion_core::ids::DatId::new(0x3c00_0005);
    world.buildable_classes.insert(
        class,
        rebellion_core::world::BuildableClass {
            name: "Imperial Probe Droid".into(),
            ..Default::default()
        },
    );
    let droid = world
        .special_forces
        .insert(rebellion_core::world::SpecialForceUnit {
            class_dat_id: class,
            is_alliance: false,
            skills: [0; 8],
            on_mission: true,
            on_hidden_mission: false,
        });
    let result = MissionResult {
        mission_id: 1,
        tick: 3,
        kind: MissionKind::Reconnaissance,
        faction: MissionFaction::Empire,
        character: None,
        lead_force: Some(droid),
        target_system: Default::default(),
        target_character: None,
        target_object: None,
        returning_to: None,
        outcome: MissionOutcome::Success,
        effects: Vec::new(),
    };
    assert_eq!(
        report_names(&world, &result).0,
        Some(("Imperial Probe Droid".into(), false))
    );
}

#[test]
fn a_shipped_report_runs_into_its_question_and_the_port_line_takes_a_space() {
    // FUN_004927c0 appends 0x70db to the body it has built.
    let question = Some("Do you wish the mission to continue?");
    assert_eq!(
        report_title_and_text(
            Some((
                "Diplomacy Mission Report".into(),
                "The population supports the Alliance.\n".into()
            )),
            "port".into(),
            question
        ),
        (
            Some("Diplomacy Mission Report".into()),
            "The population supports the Alliance.\nDo you wish the mission to continue?".into()
        )
    );
    assert_eq!(
        report_title_and_text(
            None,
            "Alliance Sabotage mission at Averam failed".into(),
            question
        ),
        (
            None,
            "Alliance Sabotage mission at Averam failed Do you wish the mission to continue?"
                .into()
        )
    );
    assert_eq!(
        report_title_and_text(None, "line".into(), None),
        (None, "line".into())
    );
}

#[test]
fn a_foiled_mission_posts_enemy_mission_foiled_to_its_opponent() {
    // FUN_00491500 (notification 0x1f1): 0x713e / 0x713f to the reader
    // that is not the mission's side, picture 0x431 / 0x432 by reader.
    use rebellion_core::missions::MissionEnded;
    let mut world = GameWorld::default();
    let averam = world.systems.insert(rebellion_core::world::System {
        last_arrival: None,
        dat_id: rebellion_core::ids::DatId::new(2),
        name: "Averam".into(),
        sector: rebellion_core::ids::SectorKey::default(),
        x: 0,
        y: 0,
        exploration_status: rebellion_core::dat::ExplorationStatus::Explored,
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
        control: rebellion_core::world::ControlKind::Uncontrolled,
    });
    let texts = std::collections::HashMap::from([
        (0x713e, "Enemy Mission Foiled".to_owned()),
        (
            0x713f,
            "An enemy {1:1} mission to {2:1} has been foiled by our forces.".to_owned(),
        ),
    ]);
    let ended = |faction, end_code| MissionEnded {
        mission_id: 1,
        kind: MissionKind::Espionage,
        faction,
        character: None,
        target_system: averam,
        end_code,
        tick: 43,
        returning_to: None,
    };
    let mut log = MessageLog::new();
    push_foiled_missions(
        &mut log,
        &world,
        &[
            ended(MissionFaction::Alliance, 3),
            ended(MissionFaction::Empire, 1),
            ended(MissionFaction::Empire, 4),
        ],
        &texts,
    );
    let messages = log.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].title.as_deref(), Some("Enemy Mission Foiled"));
    assert_eq!(
        messages[0].text,
        "An enemy Espionage mission to Averam has been foiled by our forces."
    );
    assert_eq!(messages[0].audience, RailAudience::Empire);
    assert_eq!(messages[0].picture.map(|p| p.background), Some(0x432));
    assert_eq!(messages[0].rail, Some(MessageRail::Mission));
    assert_eq!(messages[1].audience, RailAudience::Alliance);
    assert_eq!(messages[1].picture.map(|p| p.background), Some(0x431));

    // With a leader, codes 3..5 also report to the mission's own side
    // (0x714a / 0x714b for a minor character, background 0x416 / 0x417);
    // code 7 does not.
    let texts = {
        let mut texts = texts;
        texts.insert(0x714a, "{2:1} Mission Foiled".to_owned());
        texts.insert(
            0x714b,
            "The {2:1} mission to {3:1} has been foiled by opposing forces.  ".to_owned(),
        );
        texts
    };
    let agent = world.characters.insert(rebellion_core::world::Character {
        name: "Agent".into(),
        is_alliance: true,
        ..rebellion_core::world::Character::default()
    });
    let led = |end_code| MissionEnded {
        character: Some(agent),
        ..ended(MissionFaction::Alliance, end_code)
    };
    let mut log = MessageLog::new();
    push_foiled_missions(&mut log, &world, &[led(4), led(7)], &texts);
    let messages = log.messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages[0].title.as_deref(),
        Some("Espionage Mission Foiled")
    );
    assert_eq!(
        messages[0].text,
        "The Espionage mission to Averam has been foiled by opposing forces.  "
    );
    assert_eq!(messages[0].audience, RailAudience::Alliance);
    assert_eq!(messages[0].picture.map(|p| p.background), Some(0x416));
    assert_eq!(messages[1].title.as_deref(), Some("Enemy Mission Foiled"));
    assert_eq!(messages[1].audience, RailAudience::Empire);
    assert_eq!(messages[2].title.as_deref(), Some("Enemy Mission Foiled"));
    // FUN_00491500 ends every report on where the personnel return
    // (0x70e6 with the members' place): A0 2026-11-15-partial-cells/
    // r2-msg-rescue-mission-foiled-pzob-d1188 (sha256 7a9bc30025d8)
    // reads "The Rescue mission to Pzob has been foiled by opposing
    // forces." over "Personnel are returning to Yavin.".
    let texts = {
        let mut texts = texts;
        texts.insert(0x70e6, "\nPersonnel are returning to {1:1}.".to_owned());
        texts
    };
    let home = |end_code| MissionEnded {
        returning_to: Some(rebellion_core::missions::ReturnPlace::System(averam)),
        ..led(end_code)
    };
    let mut log = MessageLog::new();
    push_foiled_missions(&mut log, &world, &[home(4)], &texts);
    assert_eq!(
        log.messages()[0].text,
        format!(
            "The Espionage mission to Averam has been foiled by opposing forces.  \nPersonnel are returning to {}.",
            world.systems[averam].name
        )
    );
    world.characters[agent].mark_killed();
    let mut log = MessageLog::new();
    push_foiled_missions(&mut log, &world, &[led(4)], &texts);
    assert_eq!(log.messages().len(), 1);
    assert_eq!(log.messages()[0].audience, RailAudience::Empire);
}
