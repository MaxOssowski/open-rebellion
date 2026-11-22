use super::*;

#[test]
fn a_natural_disaster_is_filed_on_the_resource_rail_and_an_uprising_on_popular_support() {
    // FUN_0048e940's class (0x65b490) files Natural Disaster as it
    // files New Resources and Resources Depleted: A0
    // 2026-11-15-partial-cells/r2-message-index-natural-disaster-row-
    // d824 (sha256 b4dea7dc97f2) shows the row's Resource icon (0x2ad6)
    // and r2-msg-natural-disaster-on-yavin-d824 (bc967b60eb32) the same
    // title image. Uprising Begins / Ends (FUN_00499460) stay on
    // Popular Support.
    use rebellion_core::uprising::UprisingEvent;
    let system = rebellion_core::ids::SystemKey::default();
    assert_eq!(
        uprising_message_rail(&UprisingEvent::Disaster {
            system,
            tick: 0,
            total_energy: 0,
            raw_materials: 0,
            destroyed: Vec::new(),
            lost_products: Vec::new(),
        }),
        MessageRail::Resource
    );
    assert_eq!(
        uprising_message_rail(&UprisingEvent::UprisingBegan { system, tick: 0 }),
        MessageRail::PopularSupport
    );
    assert_eq!(
        uprising_message_rail(&UprisingEvent::UprisingEnded { system, tick: 0 }),
        MessageRail::PopularSupport
    );
}

/// The shipped TEXTSTRA message texts; the test needs the original
/// data in data/base.
fn shipped_texts() -> std::collections::HashMap<u16, String> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
    let texts = rebellion_data::message_texts(&base);
    assert!(!texts.is_empty(), "no TEXTSTRA.DLL in {}", base.display());
    texts
}

// A0 2026-11-16-partial-cells/r-9 (sha256 f519dea42d7a, an Empire
// game, OBJ-11-C011): the Imperial Probe Droid's Recon Mission Report,
// FUN_004927c0 case 0x54 (0x70e8 over 0x70e9) then 0x70e6, word for
// word as A0 shows it.
#[test]
#[ignore = "needs the original data in data/base"]
fn the_shipped_texts_build_the_a0_probe_droid_recon_report() {
    use rebellion_core::missions::{MissionOutcome, MissionResult};
    use rebellion_render::story_messages as story;
    let texts = shipped_texts();
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
        });
    let result = MissionResult {
        mission_id: 1,
        tick: 252,
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
    // The system names are a_return_place_is_named_by_its_system's.
    let (leader, target, _, killed) = report_names(&world, &result);
    let facts = story::ReportFacts {
        success: true,
        leader: leader.as_ref().map(|(name, major)| (name.as_str(), *major)),
        system: "Chandrila",
        target: target.as_deref(),
        target_killed: killed,
        returning: Some("Coruscant"),
        ..story::ReportFacts::default()
    };
    assert_eq!(
        story::mission_report_texts(MissionKind::Reconnaissance, &facts, &texts),
        Some((
            "Recon Mission Report".to_owned(),
            "The reconnaissance mission to Chandrila was successful.\n\nPersonnel are returning to Coruscant."
                .to_owned()
        ))
    );
}

// A0 2026-11-15-partial-cells (an Alliance game): each message's title
// and text as the original's own texts build them.
#[test]
#[ignore = "needs the original data in data/base"]
fn the_shipped_texts_build_the_a0_disaster_rescue_capture_and_bounty_messages() {
    use rebellion_render::story_messages as story;
    let texts = shipped_texts();
    let pair = |title: &str, body: &str| Some((title.to_owned(), body.to_owned()));
    // r2-msg-natural-disaster-on-yavin-d824 (bc967b60eb32), EVT-01-C002.
    assert_eq!(
        story::disaster_texts("Yavin", true, &[], &texts),
        pair(
            "Natural Disaster on Yavin",
            "There has been a natural disaster on Yavin which has destroyed resources there."
        )
    );
    // msg-han-mission-report-rescue-success-d818view (b114a1a92986):
    // the report, a blank line, then where the personnel return.
    let facts = story::ReportFacts {
        success: true,
        leader: Some(("Han Solo", true)),
        system: "Corellia",
        target: Some("Orrimaarko"),
        returning: Some("Yavin"),
        ..story::ReportFacts::default()
    };
    assert_eq!(
        story::mission_report_texts(MissionKind::Rescue, &facts, &texts),
        pair(
            "Han Solo Mission Report",
            "My mission to rescue Orrimaarko from Corellia was a success.\n\nPersonnel are returning to Yavin."
        )
    );
    // r2-msg-rescue-mission-foiled-pzob-d1188 (7a9bc30025d8): Wedge's
    // picture, the kind's title (Wedge is not a major character).
    assert_eq!(
        story::mission_foiled_texts(
            ("Wedge Antilles", false),
            rebellion_render::mission_dialog::kind_name(MissionKind::Rescue),
            "Pzob",
            Some("Yavin"),
            &texts
        ),
        pair(
            "Rescue Mission Foiled",
            "The Rescue mission to Pzob has been foiled by opposing forces.  \nPersonnel are returning to Yavin."
        )
    );
    let character = |name: &str| rebellion_core::world::Character {
        name: name.into(),
        is_alliance: true,
        ..rebellion_core::world::Character::default()
    };
    // r2-msg-han-solo-captured-yavin-d616 (885de2a03b34) and
    // msg-orrimaarko-captured-corellia-d825view (704310248349).
    for (name, system) in [("Han Solo", "Yavin"), ("Orrimaarko", "Corellia")] {
        let message = story::captured_message(
            None,
            &character(name),
            false,
            system,
            CockpitFaction::Alliance,
            0,
            &texts,
        )
        .unwrap();
        assert_eq!(
            message.title.as_deref(),
            Some(format!("{name} Captured").as_str())
        );
        assert_eq!(
            message.text,
            format!("{name} was captured by the Empire at {system}.")
        );
    }
    // msg-orrimaarko-escaped-d821view (9c2e291d9b9e).
    let escaped = story::escaped_message(
        &character("Orrimaarko"),
        false,
        "Corellia",
        CockpitFaction::Alliance,
        0,
        &texts,
    )
    .unwrap();
    assert_eq!(escaped.title.as_deref(), Some("Orrimaarko Escaped"));
    assert_eq!(
        escaped.text,
        "Orrimaarko escaped from the Empire at Corellia."
    );
    // msg-han-attacked-by-bounty-hunters-failed-d946 (40e35b8b1dcf) and
    // r2-...-d1525 (25b06c1cfee3), EVT-01-C018.
    let mut world = GameWorld::default();
    let han = world.characters.insert(character("Han Solo"));
    let messages = story::bounty_messages(&world, han, 0, &texts);
    let attacked = messages
        .iter()
        .find(|message| message.title.as_deref() == Some("Han Solo Attacked by Bounty Hunters"))
        .expect("the Alliance's message");
    assert_eq!(
        attacked.text,
        "Han Solo was attacked by bounty hunters, who failed to capture him."
    );
}
