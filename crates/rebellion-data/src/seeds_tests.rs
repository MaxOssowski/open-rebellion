use super::*;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::path::PathBuf;

#[test]
fn special_forces_roll_each_skill_from_their_class_and_skip_unknown_classes() {
    // FUN_00535e40: skill = class base + rand(0..=variance). A unit whose
    // class is not in SPECFCSD keeps the zeroed skills of FUN_005336b0.
    let mut world = GameWorld::default();
    let class = DatId::new(0x3c00_0001);
    let mut skills = [SkillPair {
        base: 0,
        variance: 0,
    }; 8];
    skills[1] = SkillPair {
        base: 55,
        variance: 0,
    };
    skills[5] = SkillPair {
        base: 20,
        variance: 10,
    };
    world.special_force_classes.insert(
        class,
        rebellion_core::world::SpecialForceClassDef {
            skills,
            mission_mask: 1,
        },
    );
    let unit = |class_dat_id| SpecialForceUnit {
        class_dat_id,
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
    };
    let known = world.special_forces.insert(unit(class));
    let unknown = world.special_forces.insert(unit(DatId::new(0)));

    roll_special_force_skills(&mut world, &mut Xoshiro256PlusPlus::seed_from_u64(42));

    let rolled = world.special_forces[known].skills;
    assert_eq!(rolled[1], 55);
    // The one draw is the combat variance; seed 42 gives a nonzero bonus,
    // so a roll that ignored the variance would fail here.
    let bonus: u32 = Xoshiro256PlusPlus::seed_from_u64(42).gen_range(0..=10);
    assert_ne!(bonus, 0);
    assert_eq!(rolled[5], 20 + bonus);
    assert_eq!(rolled[0], 0);
    assert_eq!(world.special_forces[unknown].skills, [0; 8]);
}

#[test]
fn special_forces_with_no_variance_draw_nothing_from_the_rng() {
    // The shipped SPECFCSD variances are all 0 (ghidra/notes/
    // mission-lifecycle.md, "Member skills"), so the seed roll must leave
    // the stream where it was.
    let mut world = GameWorld::default();
    let class = DatId::new(0x3c00_0001);
    world.special_force_classes.insert(
        class,
        rebellion_core::world::SpecialForceClassDef {
            skills: [SkillPair {
                base: 30,
                variance: 0,
            }; 8],
            mission_mask: 1,
        },
    );
    world.special_forces.insert(SpecialForceUnit {
        class_dat_id: class,
        is_alliance: true,
        skills: [0; 8],
        on_mission: false,
    });
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(7);
    let untouched = rng.clone();

    roll_special_force_skills(&mut world, &mut rng);

    assert_eq!(rng, untouched);
    assert!(world.special_forces.values().all(|u| u.skills == [30; 8]));
}

#[test]
fn game_start_rolls_force_potential_and_a_force_base_only_for_the_aware() {
    // FUN_004f11f0: ForceAware is the class is_known_jedi (set when the
    // character is converted), ForcePotential a FUN_0053e2f0 roll on
    // jedi_probability; FUN_004eee60 gives a Force base only to a
    // character with both, base + rand(0..=variance) (FUN_0053e290).
    // Potential alone leaves the character unaware, at Force 0.
    let mut world = GameWorld::default();
    let mut add = |jedi_probability, force_aware, base, variance| {
        world.characters.insert(rebellion_core::world::Character {
            jedi_probability,
            force_aware,
            jedi_level: rebellion_core::world::SkillPair { base, variance },
            ..Default::default()
        })
    };
    let latent = add(100, false, 10, 0);
    let never = add(0, false, 10, 40);
    let known = add(100, true, 50, 0);
    let rolled = add(100, true, 10, 40);
    let known_without_potential = add(0, true, 120, 0);

    roll_character_stats(&mut world, &mut Xoshiro256PlusPlus::seed_from_u64(7));

    let c = &world.characters;
    assert!(c[latent].force_potential && !c[latent].force_aware);
    assert_eq!(c[latent].force_base, 0);
    assert!(!c[never].force_potential);
    assert_eq!(c[never].force_base, 0);
    assert_eq!(c[known].force_base, 50);
    assert!((10..=50).contains(&c[rolled].force_base));
    assert!(!c[known_without_potential].force_potential);
    assert_eq!(c[known_without_potential].force_base, 0);
    // The class template stays for later rolls.
    assert_eq!(c[rolled].jedi_level.base, 10);
    assert_eq!(c[rolled].jedi_level.variance, 40);
}

#[test]
fn a_zero_jedi_probability_never_gives_force_potential() {
    // FUN_0053e2f0: rand(0..=99) < p, so p = 0 never passes and p = 1
    // passes on a draw of 0.
    let mut potential = [0_u32; 2];
    for seed in 0..1000 {
        let mut world = GameWorld::default();
        let keys = [0, 1].map(|jedi_probability| {
            world.characters.insert(rebellion_core::world::Character {
                jedi_probability,
                ..Default::default()
            })
        });
        roll_character_stats(&mut world, &mut Xoshiro256PlusPlus::seed_from_u64(seed));
        for (count, key) in potential.iter_mut().zip(keys) {
            *count += u32::from(world.characters[key].force_potential);
        }
    }
    assert_eq!(potential[0], 0);
    assert!((1..=40).contains(&potential[1]), "{potential:?}");
}

#[test]
fn the_force_base_roll_draws_across_the_class_variance() {
    // FUN_0053e290(variance): 0..=variance, so base + variance appears.
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..400 {
        let mut world = GameWorld::default();
        let key = world.characters.insert(rebellion_core::world::Character {
            jedi_probability: 100,
            force_aware: true,
            jedi_level: rebellion_core::world::SkillPair {
                base: 10,
                variance: 2,
            },
            ..Default::default()
        });
        roll_character_stats(&mut world, &mut Xoshiro256PlusPlus::seed_from_u64(seed));
        seen.insert(world.characters[key].force_base);
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![10, 11, 12]);
}

fn bare_system(world: &mut GameWorld, control: ControlKind) -> SystemKey {
    world.systems.insert(rebellion_core::world::System {
        dat_id: DatId::new(0x9000_0001),
        name: "Here".into(),
        exploration_status: ExplorationStatus::Explored,
        popularity_alliance: 0.5,
        popularity_empire: 0.5,
        is_populated: true,
        total_energy: 4,
        raw_materials: 2,
        control,
        ..Default::default()
    })
}

#[test]
fn seeded_facilities_take_their_systems_side_and_neutral_systems_serve_nobody() {
    // The setup seeders create each facility with its system's side bits
    // (ghidra/notes/facility-ownership.md); seeding marks them all one
    // side first, so a skipped pass would leave Empire everywhere.
    let mut world = GameWorld::default();
    let alliance = bare_system(&mut world, ControlKind::Controlled(Faction::Alliance));
    let empire = bare_system(&mut world, ControlKind::Controlled(Faction::Empire));
    let neutral = bare_system(&mut world, ControlKind::Uncontrolled);
    for system in [alliance, empire, neutral] {
        dispatch_facility_item(0x2900_0001, system, false, &mut world);
        dispatch_facility_item(0x2200_0001, system, false, &mut world);
        dispatch_facility_item(0x2C00_0001, system, false, &mut world);
    }

    assign_facility_sides(&mut world);

    for (system, side) in [
        (alliance, Faction::Alliance),
        (empire, Faction::Empire),
        (neutral, Faction::Neutral),
    ] {
        let sides: Vec<Faction> = world
            .facilities_at(system)
            .into_iter()
            .map(|f| world.facility(f).unwrap().1)
            .collect();
        assert_eq!(sides, vec![side; 3], "{side:?} system");
    }
}

#[test]
fn facility_family_0x2c_seeds_a_mine_and_0x2d_a_refinery() {
    // FUN_0056a6f0 places 0x2c000001 as its mine; the System Status
    // counters and Build Selection command 0x6c read 0x2c as mine
    // (ghidra/notes/facility-ownership.md).
    let mut world = GameWorld::default();
    let system = bare_system(&mut world, ControlKind::Uncontrolled);
    dispatch_facility_item(0x2C00_0001, system, false, &mut world);
    dispatch_facility_item(0x2D00_0001, system, false, &mut world);

    let kinds: Vec<(u32, bool)> = world.systems[system]
        .production_facilities
        .iter()
        .map(|&k| {
            let f = &world.production_facilities[k];
            (f.class_dat_id.raw(), f.is_mine)
        })
        .collect();
    assert_eq!(kinds, vec![(0x2C00_0001, true), (0x2D00_0001, false)]);
}

#[test]
fn facility_items_land_in_their_familys_list() {
    // SYFC family ranges (module header): HQ 0x20 and yards 0x28..0x2a
    // manufacture, 0x22..0x25 defend, an unknown family is skipped.
    let mut world = GameWorld::default();
    let system = bare_system(&mut world, ControlKind::Uncontrolled);
    for item in [0x2000_0001, 0x2800_0001, 0x2900_0001, 0x2200_0001, 0x3000_0001] {
        dispatch_facility_item(item, system, false, &mut world);
    }

    let here = &world.systems[system];
    let yards: Vec<(u32, bool)> = here
        .manufacturing_facilities
        .iter()
        .map(|&k| {
            let f = &world.manufacturing_facilities[k];
            (f.class_dat_id.raw(), f.is_shipyard)
        })
        .collect();
    assert_eq!(
        yards,
        vec![(0x2000_0001, false), (0x2800_0001, true), (0x2900_0001, false)]
    );
    assert_eq!(here.defense_facilities.len(), 1);
    assert!(here.production_facilities.is_empty());
}

fn gdata_path() -> PathBuf {
    // Relative to workspace root when running `cargo test`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .join("data/base")
}

// ── M3: Special system tests ────────────────────────────────────────────

#[test]
#[ignore = "requires original data/base DAT files"]
fn a_new_game_lands_the_garrison_tables_fighter_squadrons_on_their_planets() {
    // FUN_0051ae20 / FUN_0051acb0 / FUN_0051ad90 -> FUN_0051be90 ->
    // FUN_0051bf30: a garrison group's items after its null first item
    // are created in the system, so its squadrons stand on the planet.
    // CMUNCRTB holds 10 TIE Fighter squadrons (0x1c000005), CMUNYVTB 2
    // and CMUNHQTB 6 Alliance squadrons (0x1c000003, 0x1c000004).
    // A0 TAC-05-C009 (manifest): the Empire's day-2 save has 11 TIE
    // squadrons on Coruscant's planet.
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");
    let planet = |seq: u32| {
        let system = world
            .systems
            .values()
            .find(|system| system.dat_id.raw() == seq)
            .expect("special system");
        let mut names: Vec<(&str, u32)> = system
            .fighters
            .iter()
            .map(|entry| {
                assert_eq!(entry.carrier, None);
                (
                    world.fighter_classes[entry.class].name.as_str(),
                    entry.count,
                )
            })
            .collect();
        names.sort_unstable();
        names
    };
    assert_eq!(planet(CORUSCANT_SEQ_ID), [("TIE Fighter", 10)]);
    assert_eq!(planet(YAVIN_SEQ_ID), [("X-wing", 1), ("Y-wing", 1)]);
    let hq = world
        .systems
        .values()
        .find(|system| system.is_headquarters && system.dat_id.raw() != CORUSCANT_SEQ_ID)
        .expect("Alliance HQ")
        .dat_id
        .raw();
    assert_eq!(planet(hq), [("X-wing", 4), ("Y-wing", 2)]);
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn special_systems_match_original_roles() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // Coruscant: Empire-controlled, populated, charted, has fleet + facilities
    let coruscant = world
        .systems
        .iter()
        .find(|(_, s)| s.dat_id.raw() == CORUSCANT_SEQ_ID);
    assert!(coruscant.is_some(), "Coruscant must exist");
    let (_ck, cs) = coruscant.unwrap();
    assert!(cs.is_populated, "Coruscant must be populated");
    assert_eq!(
        cs.exploration_status,
        ExplorationStatus::Explored,
        "Coruscant must be charted"
    );
    assert!(
        cs.control.is_controlled_by(Faction::Empire),
        "Coruscant must be Empire-controlled"
    );
    assert!(
        !cs.fleets.is_empty(),
        "Coruscant must have fleets (CMUNEFTB)"
    );
    assert!(
        !cs.manufacturing_facilities.is_empty(),
        "Coruscant must have facilities (FACLCRTB)"
    );

    // Yavin: Alliance-controlled, populated, charted, has ground units
    let yavin = world
        .systems
        .iter()
        .find(|(_, s)| s.dat_id.raw() == YAVIN_SEQ_ID);
    assert!(yavin.is_some(), "Yavin must exist");
    let (yk, ys) = yavin.unwrap();
    assert!(ys.is_populated, "Yavin must be populated");
    assert_eq!(
        ys.exploration_status,
        ExplorationStatus::Explored,
        "Yavin must be charted"
    );
    assert!(
        ys.control.is_controlled_by(Faction::Alliance),
        "Yavin must be Alliance-controlled"
    );
    assert!(
        !ys.is_headquarters,
        "Yavin is the Alliance base, not the randomized Rebel HQ"
    );
    assert!(
        !ys.ground_units.is_empty(),
        "Yavin must have ground units (CMUNYVTB)"
    );

    // Rebel HQ: must exist, be Alliance-controlled, populated, charted,
    // and be a DIFFERENT system from Yavin.
    // Find it: it's the Alliance HQ system that is NOT Yavin.
    let rebel_hq = world.systems.iter().find(|(k, s)| {
        s.is_headquarters
            && s.control.is_controlled_by(Faction::Alliance)
            && s.dat_id.raw() != YAVIN_SEQ_ID
            && *k != yk
    });
    assert!(
        rebel_hq.is_some(),
        "Rebel HQ must be a separate system from Yavin"
    );
    let (_rhk, rhs) = rebel_hq.unwrap();
    assert!(rhs.is_populated, "Rebel HQ must be populated");
    assert_eq!(
        rhs.exploration_status,
        ExplorationStatus::Explored,
        "Rebel HQ must be charted"
    );

    // Rebel HQ must be a rim system (not core)
    let hq_sector = world.sectors.get(rhs.sector);
    assert!(hq_sector.is_some(), "Rebel HQ sector must exist");
    let group = hq_sector.unwrap().group;
    assert!(
        group == SectorGroup::RimInner || group == SectorGroup::RimOuter,
        "Rebel HQ must be a rim system, got {group:?}"
    );

    // FACLHQTB should be at Rebel HQ, NOT at Yavin
    assert!(
        !rhs.manufacturing_facilities.is_empty()
            || !rhs.defense_facilities.is_empty()
            || !rhs.production_facilities.is_empty(),
        "Rebel HQ should have facilities from FACLHQTB"
    );
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn fixed_fleet_tables_only_target_special_systems() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // Collect all systems that have fleets
    let systems_with_fleets: Vec<_> = world
        .systems
        .iter()
        .filter(|(_, s)| !s.fleets.is_empty())
        .map(|(_, s)| s.dat_id.raw())
        .collect();

    // With fixed seed tables only, fleets should appear at special systems only
    // (Coruscant, Yavin, or Rebel HQ). The Rebel HQ seq_id varies by RNG seed,
    // but Coruscant and Yavin should always be present.
    assert!(
        systems_with_fleets.contains(&CORUSCANT_SEQ_ID),
        "Coruscant should have fleet(s)"
    );

    // Empire fleets should NOT be at more than ~3 systems (was 10 in old proximity model)
    let empire_fleet_system_count = world
        .systems
        .iter()
        .filter(|(_, s)| {
            s.fleets
                .iter()
                .any(|&fk| world.fleets.get(fk).is_some_and(|f| !f.is_alliance))
        })
        .count();
    assert!(
        empire_fleet_system_count <= 3,
        "Empire fleets should be at <= 3 systems (3-system model), found {empire_fleet_system_count}"
    );
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn deterministic_rebel_hq_with_same_seed() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(12345),
        ..SeedOptions::default()
    };

    let world1 =
        crate::load_game_data_with_options(&path, &seed_options).expect("first load failed");
    let world2 =
        crate::load_game_data_with_options(&path, &seed_options).expect("second load failed");

    // Find Rebel HQ in both worlds
    let find_rebel_hq = |world: &GameWorld| -> Option<u32> {
        world
            .systems
            .iter()
            .find(|(_, s)| {
                s.is_headquarters
                    && s.control.is_controlled_by(Faction::Alliance)
                    && s.dat_id.raw() != YAVIN_SEQ_ID
            })
            .map(|(_, s)| s.dat_id.raw())
    };

    let hq1 = find_rebel_hq(&world1);
    let hq2 = find_rebel_hq(&world2);
    assert!(hq1.is_some(), "Rebel HQ should exist in world 1");
    assert_eq!(hq1, hq2, "Same RNG seed must produce same Rebel HQ system");
}

// Manual p. 45: the Alliance HQ "begins the game at a random system on
// the Galactic Rim"; p. 49: "the game begins randomly every time you
// play". New games started a few seconds apart must not share one HQ.
#[test]
#[ignore = "requires original data/base DAT files"]
fn new_games_started_at_different_times_place_the_rebel_hq_at_different_rim_systems() {
    let path = gdata_path();
    let hq_for = |seconds: f64| -> u32 {
        let options = SeedOptions {
            rng_seed: Some(campaign_seed_from_clock(seconds)),
            ..SeedOptions::default()
        };
        let world = crate::load_game_data_with_options(&path, &options).expect("load");
        world
            .systems
            .values()
            .find(|s| s.is_headquarters && s.control.is_controlled_by(Faction::Alliance))
            .map(|s| s.dat_id.raw())
            .expect("rebel HQ")
    };
    let start = 1_795_000_000.0;
    let places: std::collections::BTreeSet<u32> = (0..8)
        .map(|game| hq_for(start + f64::from(game) * 7.0))
        .collect();
    assert!(
        places.len() >= 4,
        "eight games put the HQ at only {places:?}"
    );
}

#[test]
fn a_campaign_seed_follows_the_clock_reading() {
    assert_eq!(campaign_seed_from_clock(1.5), 1_500_000);
    assert_ne!(
        campaign_seed_from_clock(1_795_000_000.0),
        campaign_seed_from_clock(1_795_000_001.0)
    );
    assert_eq!(campaign_seed_from_clock(-3.0), 0);
}

// ── M4: Character stat rolling + placement tests ────────────────────────

#[test]
#[ignore = "requires original data/base DAT files"]
fn fixed_named_characters_spawn_at_expected_systems() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // Find system keys
    let coruscant_key = world
        .systems
        .iter()
        .find(|(_, s)| s.dat_id.raw() == CORUSCANT_SEQ_ID)
        .map(|(k, _)| k);
    let yavin_key = world
        .systems
        .iter()
        .find(|(_, s)| s.dat_id.raw() == YAVIN_SEQ_ID)
        .map(|(k, _)| k);
    let rebel_hq_key = world
        .systems
        .iter()
        .find(|(_, s)| {
            s.is_headquarters
                && s.control.is_controlled_by(Faction::Alliance)
                && s.dat_id.raw() != YAVIN_SEQ_ID
        })
        .map(|(k, _)| k);

    assert!(coruscant_key.is_some(), "Coruscant must exist");
    assert!(yavin_key.is_some(), "Yavin must exist");
    assert!(rebel_hq_key.is_some(), "Rebel HQ must exist");

    // Check Alliance characters at Yavin
    for name in &[
        "Luke Skywalker",
        "Leia Organa",
        "Han Solo",
        "Wedge Antilles",
        "Chewbacca",
        "Jan Dodonna",
    ] {
        let (_, ch) = world
            .characters
            .iter()
            .find(|(_, c)| c.name == *name)
            .unwrap_or_else(|| panic!("{name} missing from TEXTSTRA.DLL names"));
        assert_eq!(
            ch.current_system, yavin_key,
            "{} should be at Yavin, but is at {:?}",
            name, ch.current_system
        );
    }

    // Check Mon Mothma at Rebel HQ
    let (_, mm) = world
        .characters
        .iter()
        .find(|(_, c)| c.name == "Mon Mothma")
        .expect("Mon Mothma missing from TEXTSTRA.DLL names");
    assert_eq!(
        mm.current_system, rebel_hq_key,
        "Mon Mothma should be at Rebel HQ"
    );

    // Check Empire characters at Coruscant
    for name in &["Emperor Palpatine", "Darth Vader"] {
        let (_, ch) = world
            .characters
            .iter()
            .find(|(_, c)| c.name == *name)
            .unwrap_or_else(|| panic!("{name} missing from TEXTSTRA.DLL names"));
        assert_eq!(
            ch.current_system, coruscant_key,
            "{} should be at Coruscant, but is at {:?}",
            name, ch.current_system
        );
    }
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn character_stats_are_rolled_not_raw() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // After rolling, all character skill variances should be 0
    for (_, character) in &world.characters {
        assert_eq!(
            character.diplomacy.variance, 0,
            "Character {} diplomacy variance should be 0 after rolling",
            character.name
        );
        assert_eq!(
            character.espionage.variance, 0,
            "Character {} espionage variance should be 0 after rolling",
            character.name
        );
        assert_eq!(
            character.combat.variance, 0,
            "Character {} combat variance should be 0 after rolling",
            character.name
        );
        assert_eq!(
            character.leadership.variance, 0,
            "Character {} leadership variance should be 0 after rolling",
            character.name
        );
        assert_eq!(
            character.loyalty.variance, 0,
            "Character {} loyalty variance should be 0 after rolling",
            character.name
        );
    }
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn deterministic_stat_rolls_with_same_seed() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(99999),
        ..SeedOptions::default()
    };

    let world1 =
        crate::load_game_data_with_options(&path, &seed_options).expect("first load failed");
    let world2 =
        crate::load_game_data_with_options(&path, &seed_options).expect("second load failed");

    // Same seed must produce identical character stats
    for ((_, c1), (_, c2)) in world1.characters.iter().zip(world2.characters.iter()) {
        assert_eq!(c1.name, c2.name, "Character order must be deterministic");
        assert_eq!(
            c1.diplomacy.base, c2.diplomacy.base,
            "Character {} diplomacy should be deterministic",
            c1.name
        );
        assert_eq!(
            c1.combat.base, c2.combat.base,
            "Character {} combat should be deterministic",
            c1.name
        );
        assert_eq!(
            c1.leadership.base, c2.leadership.base,
            "Character {} leadership should be deterministic",
            c1.name
        );
    }
}

#[test]
fn roll_skill_pair_unit() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
    let mut pair = SkillPair {
        base: 50,
        variance: 20,
    };
    roll_skill_pair(&mut pair, &mut rng);
    assert!(
        pair.base >= 50 && pair.base <= 70,
        "rolled base should be in [50, 70], got {}",
        pair.base
    );
    assert_eq!(pair.variance, 0, "variance should be zeroed after rolling");
}

#[test]
fn roll_skill_pair_zero_variance() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
    let mut pair = SkillPair {
        base: 100,
        variance: 0,
    };
    roll_skill_pair(&mut pair, &mut rng);
    assert_eq!(
        pair.base, 100,
        "zero-variance pair should keep original base"
    );
    assert_eq!(pair.variance, 0);
}

// ── M5: Support/popularity initialization tests ─────────────────────────

#[test]
#[ignore = "requires original data/base DAT files"]
fn support_ranges_match_original_rules() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // No system should have both popularity values at 0.0 (the old default).
    // Special systems have 1.0/0.0 or 0.0/1.0; others should have non-zero values.
    let mut zero_pop_count = 0;
    let mut total_count = 0;
    for (_, sys) in &world.systems {
        total_count += 1;
        if sys.popularity_alliance == 0.0 && sys.popularity_empire == 0.0 {
            zero_pop_count += 1;
        }
    }
    assert!(
        zero_pop_count == 0,
        "No system should have both popularity values at 0.0, found {zero_pop_count} of {total_count} systems"
    );

    // Core controlled systems should have support in [0.2, 1.0] range
    // (weak base = 20, strong base + extra = 90, plus special at 100)
    for (_, sys) in &world.systems {
        let sector = world.sectors.get(sys.sector);
        if sector.is_some_and(|s| s.group == SectorGroup::Core) {
            match sys.control {
                ControlKind::Controlled(Faction::Alliance) => {
                    assert!(
                        sys.popularity_alliance >= 0.2 && sys.popularity_alliance <= 1.0,
                        "Alliance-controlled core system '{}' popularity_alliance={} should be in [0.2, 1.0]",
                        sys.name, sys.popularity_alliance
                    );
                }
                ControlKind::Controlled(Faction::Empire) => {
                    assert!(
                        sys.popularity_empire >= 0.2 && sys.popularity_empire <= 1.0,
                        "Empire-controlled core system '{}' popularity_empire={} should be in [0.2, 1.0]",
                        sys.name, sys.popularity_empire
                    );
                }
                _ => {
                    // Neutral core: both in [0.41, 0.59]
                    assert!(
                        sys.popularity_alliance >= 0.40 && sys.popularity_alliance <= 0.60,
                        "Neutral core system '{}' popularity_alliance={} should be in [0.40, 0.60]",
                        sys.name, sys.popularity_alliance
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn alliance_core_control_follows_sdprtb_buckets() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // Count core systems by faction control (excluding special systems).
    let coruscant = world
        .systems
        .iter()
        .find(|(_, s)| s.dat_id.raw() == CORUSCANT_SEQ_ID)
        .map(|(k, _)| k);
    let yavin = world
        .systems
        .iter()
        .find(|(_, s)| s.dat_id.raw() == YAVIN_SEQ_ID)
        .map(|(k, _)| k);
    let rebel_hq = world
        .systems
        .iter()
        .find(|(_, s)| {
            s.is_headquarters
                && s.control.is_controlled_by(Faction::Alliance)
                && s.dat_id.raw() != YAVIN_SEQ_ID
        })
        .map(|(k, _)| k);

    let special_keys: Vec<SystemKey> = [coruscant, yavin, rebel_hq]
        .iter()
        .filter_map(|k| *k)
        .collect();

    let mut alliance_controlled = 0;
    let mut empire_controlled = 0;
    let mut total_core = 0;

    for (key, sys) in &world.systems {
        if special_keys.contains(&key) {
            continue;
        }
        let sector = world.sectors.get(sys.sector);
        if !sector.is_some_and(|s| s.group == SectorGroup::Core) {
            continue;
        }
        total_core += 1;
        match sys.control {
            ControlKind::Controlled(Faction::Alliance) => alliance_controlled += 1,
            ControlKind::Controlled(Faction::Empire) => empire_controlled += 1,
            _ => {}
        }
    }

    // Medium difficulty reads SDPRTB 7680 (strong: Alliance 20%, Empire 25%)
    // and 7681 (weak: Alliance 0%, Empire 10%); each bucket is
    // floor(core * pct / 100). The Alliance keeps exactly its buckets. The
    // Empire's count is not pinned: the post-load pass in lib.rs also gives
    // it every neutral system that holds only Empire assets.
    assert!(total_core > 0, "Should have core systems");
    assert_eq!(
        alliance_controlled,
        total_core * 20 / 100,
        "Alliance core control (got {alliance_controlled}/{total_core})"
    );
    assert!(
        empire_controlled > 0,
        "Empire should control some core systems (got {empire_controlled}/{total_core})"
    );
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn populated_systems_follow_gnprtb_rules() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // All core systems should be populated (GNPRTB 7730 = 100).
    let mut core_count = 0;
    let mut core_populated = 0;
    let mut rim_count = 0;
    let mut rim_populated = 0;
    for (_, sys) in &world.systems {
        let sector = world.sectors.get(sys.sector);
        let group = sector.map_or(SectorGroup::RimOuter, |s| s.group);
        match group {
            SectorGroup::Core => {
                core_count += 1;
                if sys.is_populated {
                    core_populated += 1;
                }
            }
            SectorGroup::RimInner | SectorGroup::RimOuter => {
                rim_count += 1;
                if sys.is_populated {
                    rim_populated += 1;
                }
            }
        }
    }

    assert_eq!(
        core_count, core_populated,
        "All {core_count} core systems should be populated, but only {core_populated} are"
    );

    // Rim: ~31% should be populated (GNPRTB 7731 = 31).
    // With enough systems, expect 15-50% range (statistical variability).
    if rim_count > 5 {
        let pct = f64::from(rim_populated) / f64::from(rim_count) * 100.0;
        assert!(
            pct > 10.0 && pct < 60.0,
            "Rim populated percentage should be roughly ~31%, got {pct:.1}% ({rim_populated}/{rim_count})"
        );
    }
}

// ── M6: Energy/raw materials and procedural facility tests ──────────

// RimSystem_vtbc FUN_0056a550 and CoreSystem_vtbc FUN_00566d40 roll
// energy and raw materials for every system, populated or not (manual
// p. 49: "Endor Has Resources, but Is Unpopulated"). The earlier
// "unpopulated systems have 0 energy" assertion cited no source and is
// contradicted by both functions.
#[test]
#[ignore = "requires original data/base DAT files"]
fn every_system_rolls_energy_and_raw_materials_whether_or_not_it_is_populated() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    let mut unpopulated = 0;
    let mut unpopulated_with_raw = 0;
    for (_, sys) in &world.systems {
        // Rim energy is at least GNPRTB 7725 = 1, Core at least 7721 = 10;
        // both are clamped to 7714 = 15.
        assert!(
            (1..=15).contains(&sys.total_energy),
            "system '{}' energy={} should be in [1, 15]",
            sys.name,
            sys.total_energy
        );
        // FUN_005599c0 / FUN_005596a0 clamp raw materials to the energy.
        assert!(
            sys.raw_materials <= sys.total_energy,
            "system '{}' raw_materials={} should be <= total_energy={}",
            sys.name,
            sys.raw_materials,
            sys.total_energy
        );
        if !sys.is_populated {
            unpopulated += 1;
            if sys.raw_materials > 0 {
                unpopulated_with_raw += 1;
            }
        }
    }
    assert!(unpopulated > 10, "the Rim keeps unpopulated systems");
    assert!(
        unpopulated_with_raw * 2 > unpopulated,
        "{unpopulated_with_raw} of {unpopulated} unpopulated systems have raw materials"
    );
}

/// A draw sequence for [`ResourceRoll::roll`]: each `next_u32` returns
/// the next queued value.
struct Queued(std::collections::VecDeque<u32>, usize);

impl rand::RngCore for Queued {
    fn next_u32(&mut self) -> u32 {
        self.1 += 1;
        self.0.pop_front().unwrap_or(0)
    }
    fn next_u64(&mut self) -> u64 {
        u64::from(self.next_u32())
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand::RngCore::try_fill_bytes(self, dest).unwrap();
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        for byte in dest {
            *byte = 0;
        }
        Ok(())
    }
}

// FUN_0053e290(n) draws 0..=n: the highest draws reach base + n.
#[test]
fn a_rim_roll_reaches_the_top_of_each_draw() {
    let rim = ResourceRoll {
        energy_base: 1,
        energy_draws: [4, 9],
        raw_base: 1,
        raw_draw: 14,
    };
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(11);
    let rolls: Vec<(i32, i32)> = (0..2000)
        .map(|_| rim.roll((0, 15), (0, 15), &mut rng))
        .collect();
    assert_eq!(rolls.iter().map(|r| r.0).max(), Some(14));
    assert_eq!(rolls.iter().map(|r| r.0).min(), Some(1));
    assert_eq!(rolls.iter().map(|r| r.1).max(), Some(14));
}

// FUN_005599c0: raw materials never exceed the system's energy, and
// FUN_00559970 clamps energy to GNPRTB 7713..7714.
#[test]
fn rim_raw_materials_never_exceed_the_systems_energy() {
    let rim = ResourceRoll {
        energy_base: 1,
        energy_draws: [4, 9],
        raw_base: 1,
        raw_draw: 14,
    };
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(7);
    for _ in 0..500 {
        let (energy, raw) = rim.roll((0, 15), (0, 12), &mut rng);
        assert!((1..=12).contains(&energy), "energy {energy}");
        assert!(
            (1..=energy).contains(&raw),
            "raw {raw} over energy {energy}"
        );
    }
}

// FUN_0053e290(0) asks FUN_005f5700 for one value, which returns 0
// without a draw; a Core roll (one energy draw) takes two draws.
#[test]
fn a_zero_draw_takes_no_random_number() {
    let core = ResourceRoll {
        energy_base: 10,
        energy_draws: [4, 0],
        raw_base: 5,
        raw_draw: 9,
    };
    let mut rng = Queued(std::collections::VecDeque::new(), 0);
    let _ = core.roll((0, 15), (0, 15), &mut rng);
    let core_draws = rng.1;
    let mut rng = Queued(std::collections::VecDeque::new(), 0);
    let _ = ResourceRoll {
        energy_draws: [4, 9],
        ..core
    }
    .roll((0, 15), (0, 15), &mut rng);
    assert!(rng.1 > core_draws, "a second energy draw takes numbers");
    assert_eq!(original_draw(0, &mut rng), 0);
    let before = rng.1;
    let _ = original_draw(0, &mut rng);
    assert_eq!(rng.1, before);
}

// FUN_0053e290 with a negative n draws n..=0.
#[test]
fn a_negative_draw_stays_between_n_and_zero() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(3);
    let draws: Vec<i32> = (0..200).map(|_| original_draw(-3, &mut rng)).collect();
    assert!(draws.iter().all(|d| (-3..=0).contains(d)));
    assert!(draws.contains(&-3) && draws.contains(&0));
}

// FUN_0053e0f0: the high bound wins when the bounds cross.
#[test]
fn the_original_clamp_lets_the_high_bound_win() {
    assert_eq!(clamp_original(7, 0, 15), 7);
    assert_eq!(clamp_original(-2, 0, 15), 0);
    assert_eq!(clamp_original(20, 0, 15), 15);
    assert_eq!(clamp_original(5, 9, 3), 3);
}

/// The eight SYFCCRTB.DAT rows and seven SYFCRMTB.DAT rows as
/// (threshold, class), in file order.
const SYFCCRTB_ROWS: [(u32, u32); 8] = [
    (0, 0),
    (36, 0x2d00_0002),
    (79, 0x2800_0001),
    (82, 0x2900_0002),
    (85, 0x2a00_0003),
    (88, 0x2400_0003),
    (96, 0x2300_0002),
    (99, 0x2200_0001),
];
const SYFCRMTB_ROWS: [(u32, u32); 7] = [
    (0, 0),
    (91, 0x2d00_0002),
    (96, 0x2800_0001),
    (97, 0x2900_0002),
    (98, 0x2a00_0003),
    (99, 0x2400_0003),
    (100, 0x2300_0002),
];

fn syfc_table(rows: &[(u32, u32)]) -> SyfcTableFile {
    SyfcTableFile {
        field1: 1,
        info: "SeedTableEntry".to_string(),
        entries: (1u32..)
            .zip(rows)
            .map(
                |(id, &(threshold, facility))| dat_dumper::types::syfc_table::SyfcEntry {
                    id,
                    field2: 1,
                    system_id: threshold,
                    facility,
                },
            )
            .collect(),
    }
}

#[test]
fn a_core_seed_roll_selects_the_last_row_at_or_below_it() {
    // FUN_00595090: walk the rows to the first whose threshold exceeds the
    // roll and return its predecessor (FUN_005f5c60), so each threshold is
    // a lower bound. The class-0 first row means "no facility".
    let table = syfc_table(&SYFCCRTB_ROWS);
    let weights = parse_facility_weights(Some(&table));
    let expected = [
        (0, 0),
        (35, 0),
        (36, 0x2d00_0002),
        (78, 0x2d00_0002),
        (79, 0x2800_0001),
        (81, 0x2800_0001),
        (82, 0x2900_0002),
        (85, 0x2a00_0003),
        (88, 0x2400_0003),
        (95, 0x2400_0003),
        (96, 0x2300_0002),
        (98, 0x2300_0002),
        (99, 0x2200_0001),
        (100, 0x2200_0001),
    ];
    for (roll, class) in expected {
        assert_eq!(
            facility_for_roll(&weights, roll),
            Some(class),
            "roll {roll}"
        );
    }
}

#[test]
fn a_rim_seed_roll_leaves_the_slot_empty_below_91() {
    // FUN_00595090 over SYFCRMTB.DAT, as above.
    let table = syfc_table(&SYFCRMTB_ROWS);
    let weights = parse_facility_weights(Some(&table));
    let expected = [
        (0, 0),
        (90, 0),
        (91, 0x2d00_0002),
        (95, 0x2d00_0002),
        (96, 0x2800_0001),
        (97, 0x2900_0002),
        (98, 0x2a00_0003),
        (99, 0x2400_0003),
        (100, 0x2300_0002),
    ];
    for (roll, class) in expected {
        assert_eq!(
            facility_for_roll(&weights, roll),
            Some(class),
            "roll {roll}"
        );
    }
}

#[test]
fn a_class_0_seed_roll_spends_the_energy_slot_without_a_facility() {
    // FUN_00566de0 / FUN_0056a6f0: one FUN_00559850 roll per energy slot;
    // the class-0 row creates nothing and the loop goes on. With no raw
    // materials no slot becomes a mine, and a system outside any sector
    // reads the rim table.
    let core = syfc_table(&SYFCCRTB_ROWS);
    let rim = syfc_table(&SYFCRMTB_ROWS);
    let mut world = GameWorld::default();
    let system = bare_system(&mut world, ControlKind::Uncontrolled);
    world.systems[system].total_energy = 200;
    world.systems[system].raw_materials = 0;

    let mut rng = Xoshiro256PlusPlus::seed_from_u64(11);
    generate_procedural_facilities(
        &mut world,
        &SeedOptions::default(),
        Some(&core),
        Some(&rim),
        &mut rng,
    );

    let weights = parse_facility_weights(Some(&rim));
    let mut rolls = Xoshiro256PlusPlus::seed_from_u64(11);
    let expected = (0..200)
        .filter(|_| pick_weighted_facility(&weights, &mut rolls) != Some(0))
        .count();
    let here = &world.systems[system];
    let seeded = here.defense_facilities.len()
        + here.manufacturing_facilities.len()
        + here.production_facilities.len();
    assert_eq!(seeded, expected);
    assert!((1..100).contains(&seeded), "seeded {seeded}");
}

#[test]
fn a_seed_table_without_rows_selects_nothing() {
    assert_eq!(facility_for_roll(&parse_facility_weights(None), 50), None);
}

#[test]
fn seed_rolls_cover_0_through_100() {
    // FUN_00559850 rolls FUN_0053e290(100), which is FUN_005f5700(101):
    // 0..=100. Roll 100 is the only one that reaches the last rim row.
    let table = syfc_table(&SYFCRMTB_ROWS);
    let weights = parse_facility_weights(Some(&table));
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(7);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..20_000 {
        seen.insert(pick_weighted_facility(&weights, &mut rng));
    }
    let expected: std::collections::BTreeSet<_> = SYFCRMTB_ROWS
        .iter()
        .map(|&(_, class)| Some(class))
        .collect();
    assert_eq!(seen, expected);
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn procedural_facilities_placed_at_populated_systems() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // Count total facilities across all systems (excluding special HQ facilities).
    let mut total_facilities = 0;
    let mut systems_with_facilities = 0;
    for (_, sys) in &world.systems {
        let fac_count = sys.defense_facilities.len()
            + sys.manufacturing_facilities.len()
            + sys.production_facilities.len();
        total_facilities += fac_count;
        if fac_count > 0 {
            systems_with_facilities += 1;
        }
    }

    // With procedural generation, we should have significantly more facilities
    // than just the HQ ones (which would be ~10-20 from FACLCRTB + FACLHQTB).
    assert!(
        total_facilities > 30,
        "Procedural facility generation should produce many facilities, got {total_facilities}"
    );
    assert!(
        systems_with_facilities > 10,
        "Multiple systems should have facilities, got {systems_with_facilities} systems"
    );
}

// ── M7: Maintenance-budget unit seeding tests ───────────────────────

#[test]
#[ignore = "requires original data/base DAT files"]
fn maintenance_budget_seeding_spends_without_overshoot() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // After budget seeding, there should be more fleets than just the 3-system
    // fixed tables (~3 fleets from CMUNEFTB + CMUNAFTB groups). Budget seeding
    // adds additional fleets from CMUNEMTB/CMUNALTB bundles.
    let total_fleets = world.fleets.len();
    assert!(
        total_fleets >= 4,
        "Budget seeding should produce at least 4 fleets (3 fixed + budget), got {total_fleets}"
    );

    // Both factions should have at least 1 fleet each (from fixed tables at minimum).
    let alliance_fleets = world.fleets.values().filter(|f| f.is_alliance).count();
    let empire_fleets = world.fleets.values().filter(|f| !f.is_alliance).count();
    assert!(
        alliance_fleets >= 1,
        "Alliance should have fleets, got {alliance_fleets}"
    );
    assert!(
        empire_fleets >= 1,
        "Empire should have fleets, got {empire_fleets}"
    );

    // Total ground troops should exist (from fixed garrison + bundles + garrison pass).
    let total_troops = world.troops.len();
    assert!(
        total_troops > 5,
        "Should have ground troops from fixed garrisons + budget + garrison pass, got {total_troops}"
    );
}

#[test]
#[ignore = "requires original data/base DAT files"]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
fn low_support_systems_get_garrison_troops() {
    let path = gdata_path();
    let seed_options = SeedOptions {
        rng_seed: Some(42),
        ..SeedOptions::default()
    };
    let world = crate::load_game_data_with_options(&path, &seed_options)
        .expect("load_game_data_with_options failed");

    // Check that controlled systems with low support have garrison troops.
    // The threshold is GNPRTB 7761 = 60, so systems with support < 60% should
    // have troops from the garrison pass.
    let mut low_support_with_troops = 0;
    let mut low_support_total = 0;

    for (_, sys) in &world.systems {
        let (support, is_controlled) = match sys.control {
            ControlKind::Controlled(Faction::Alliance) => (sys.popularity_alliance, true),
            ControlKind::Controlled(Faction::Empire) => (sys.popularity_empire, true),
            _ => (0.0, false),
        };
        if !is_controlled {
            continue;
        }
        let support_100 = (support * 100.0) as i32;
        if support_100 < 60 {
            low_support_total += 1;
            if !sys.ground_units.is_empty() {
                low_support_with_troops += 1;
            }
        }
    }

    if low_support_total > 0 {
        assert!(
            low_support_with_troops > 0,
            "Low-support controlled systems should have garrison troops ({low_support_with_troops}/{low_support_total})"
        );
    }
}

#[test]
fn each_galaxy_size_reads_its_own_sdprtb_budget_row() {
    use rebellion_core::dat::GalaxySize;
    // SDPRTB parameters 5168-5170 are the Standard, Large, and Huge rows.
    assert_eq!(maintenance_budget_param(GalaxySize::Standard), 5168);
    assert_eq!(maintenance_budget_param(GalaxySize::Large), 5169);
    assert_eq!(maintenance_budget_param(GalaxySize::Huge), 5170);
}
