use super::*;

#[test]
fn an_own_assault_opens_at_once_and_is_filed_in_the_conflict_messages() {
    // FUN_0048a060 kind 7 (the attacker is the viewer, FUN_00497530):
    // FUN_005f5440 files the report, then FUN_0041d7d0 opens it; the
    // rail's mask, its light and the advisor are kind 3's only.
    // A0 2026-11-14-bombard-assault/41..43: the Alliance's own
    // Assault on Commenor and two Orbital bombardments of Commenor
    // stand in All and Conflict Messages; Display reopens the summary.
    let mut world = GameWorld::default();
    let sector = world.sectors.insert(rebellion_core::world::Sector {
        dat_id: rebellion_core::ids::DatId::new(0x8000_0001),
        name: "Corellian".into(),
        group: rebellion_core::dat::SectorGroup::Core,
        x: 0,
        y: 0,
        systems: vec![],
    });
    let system = world.systems.insert(rebellion_core::world::System {
        last_arrival: None,
        dat_id: rebellion_core::ids::DatId::new(0x9000_0001),
        name: "Commenor".into(),
        sector,
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
        control: rebellion_core::world::ControlKind::Controlled(Faction::Empire),
    });
    let report = rebellion_core::assault_report::AssaultReport::start(
        &world,
        &TroopTransportState::default(),
        Faction::Alliance,
        &[],
        system,
    )
    .expect("a report");
    let texts: std::collections::HashMap<u16, String> = [(0x7090, "Assault on {1:1}".to_string())]
        .into_iter()
        .collect();
    let mut summaries = rebellion_render::assault_summary::AssaultSummaryState::default();
    let mut log = MessageLog::new();
    present_assault_reports(
        vec![report],
        &world,
        MissionFaction::Alliance,
        &texts,
        &mut summaries,
        &mut log,
        5,
    );
    assert_eq!(summaries.window_count(), 1);
    let [message] = log.messages() else {
        panic!("one message: {:?}", log.messages().len());
    };
    assert_eq!(message.title.as_deref(), Some("Assault on Commenor"));
    assert_eq!(message.rail, Some(MessageRail::Conflict));
    assert!(message.audience.includes(true));
    assert!(message.assault_report.is_some());
    // Kind 7 lights no rail.
    assert_eq!(log.unread_mask(true), 0);
    // FUN_004697b0: Display lights for kind 3 (or 5 with a target), so
    // kind 7's stays grey (A0 2026-11-17-edited-saves-5/i-05).
    assert!(message.plain);
    // The defender's kind 3 report (FUN_00497530) keeps it lit.
    let defended = rebellion_core::assault_report::AssaultReport::start(
        &world,
        &TroopTransportState::default(),
        Faction::Alliance,
        &[],
        system,
    )
    .expect("a report");
    let mut imperial = MessageLog::new();
    present_assault_reports(
        vec![defended],
        &world,
        MissionFaction::Empire,
        &texts,
        &mut summaries,
        &mut imperial,
        5,
    );
    let [filed] = imperial.messages() else {
        panic!("one message: {:?}", imperial.messages().len());
    };
    assert!(!filed.plain);
    assert!(filed.unread);
}
