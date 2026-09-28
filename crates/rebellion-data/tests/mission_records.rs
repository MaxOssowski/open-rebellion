use std::path::PathBuf;

use rebellion_core::ids::DatId;
use rebellion_core::world::Skill;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory")
        .parent()
        .expect("repository root")
        .join("data/base")
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn missnsd_records_carry_the_mission_timer_and_flags() {
    // FUN_005236e0 reads the timer from record +0x50/+0x54 and the flags
    // from +0x58..+0x64, the file fields after `target_flags`
    // (ghidra/notes/mission-lifecycle.md, "The mission record").
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");

    assert_eq!(world.mission_records.len(), 25);
    let diplomacy = world.mission_record(0x51).expect("Diplomacy record");
    assert_eq!(diplomacy.dat_id, DatId::new(0x5100_0010));
    assert_eq!(
        (diplomacy.timer_min_days, diplomacy.timer_spread_days),
        (5, 10)
    );
    assert!(diplomacy.repeats && diplomacy.detection_phases && diplomacy.can_resign);
    assert!(!diplomacy.hidden);
    let sabotage = world.mission_record(0x69).expect("Sabotage record");
    assert_eq!(
        (sabotage.timer_min_days, sabotage.timer_spread_days),
        (1, 2)
    );
    assert!(!sabotage.repeats);
}

#[test]
#[ignore = "requires original data/base DAT files"]
fn seeded_special_forces_take_their_skills_from_specfcsd() {
    // FUN_00535e40 sets each skill to the class base + rand(0..=variance);
    // the shipped SPECFCSD variances are 0.
    let world = rebellion_data::load_game_data(&data_dir()).expect("load original game data");

    assert_eq!(world.special_force_classes.len(), 9);
    let class = &world.special_force_classes[&DatId::new(0x3c00_0001)];
    assert_eq!(class.skills[Skill::Espionage as usize].base, 55);
    assert_eq!(class.mission_mask, 0x1);
    assert!(!world.special_forces.is_empty());
    for unit in world.special_forces.values() {
        let class = &world.special_force_classes[&unit.class_dat_id];
        for (value, template) in unit.skills.iter().zip(class.skills) {
            assert_eq!(*value, template.base);
        }
    }
}
