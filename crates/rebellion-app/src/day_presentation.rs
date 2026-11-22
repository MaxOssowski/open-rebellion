//! The app's day presentation (`AppDayPresentation`): the shared day
//! loop's hooks that post messages, play sounds and queue movies and
//! battles, moved out of `main.rs`. The code reads `main.rs`'s items
//! through `super`.

use super::*;

/// The interactive game's side of the shared day functions
/// (`rebellion_data::simulation::DayPresentation`): the messages, movies
/// and the player's agent.
pub(super) struct AppDayPresentation<'a> {
    pub(super) log: &'a mut MessageLog,
    pub(super) battled: &'a mut std::collections::HashSet<rebellion_core::ids::SystemKey>,
    pub(super) message_texts: &'a std::collections::HashMap<u16, String>,
    pub(super) cockpit_faction: CockpitFaction,
    pub(super) advisor_schedule: Option<&'a mut rebellion_render::advisor_schedule::AdvisorSchedule>,
    pub(super) advisor_now: u32,
    pub(super) pending_ending: &'a mut Option<u32>,
    pub(super) player_faction: MissionFaction,
    pub(super) player_agent: &'a mut PlayerAgent,
    pub(super) advisor: &'a mut AdvisorState,
    pub(super) stash: DayStash,
    pub(super) game_mode: &'a mut GameMode,
    pub(super) tactical_state: &'a mut TacticalState,
    /// The planet squadrons up for the player's battle.
    pub(super) battle_sorties: &'a mut Vec<rebellion_core::squadrons::Sortie>,
    pub(super) tactical_flags: [bool; 5],
    /// The Empire's fog; the shared day advances the Alliance's.
    pub(super) other_fog: &'a mut FogState,
    pub(super) assault_summary: &'a mut rebellion_render::assault_summary::AssaultSummaryState,
    /// The sounds; none in a headless test.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) audio: Option<(&'a mut audio::AudioEngine, &'a AudioVolumeState)>,
}

/// What a step's messages need from before it was applied.
#[derive(Default)]
pub(super) struct DayStash {
    /// Each repaired ship's damage flag before the repair.
    pub(super) ships_damaged: Vec<Option<bool>>,
    /// Each side's and tree's Done flag before the tally (FUN_0052f440
    /// posts 0x129..0x12b when it turns on).
    pub(super) research_done: Vec<(bool, rebellion_core::research::TechType, bool)>,
    /// The Force each member had before the missions' day
    /// (`advance_force_changes`).
    pub(super) force_changes: Vec<AdvanceForceChange>,
    /// Each system's holder before the economy's control changes.
    pub(super) previous_control: Vec<(SystemKey, Option<Faction>)>,
    /// Each planet a pass's Death Star destroys: its name, its holder
    /// before the blast (FUN_004950d0 +0x64) and the characters it kills.
    pub(super) destroyed: Vec<(SystemKey, String, Option<Faction>, Vec<CharacterKey>)>,
}

impl AppDayPresentation<'_> {
    /// Play a sound effect (none headless).
    #[cfg(not(target_arch = "wasm32"))]
    fn sfx(&mut self, kind: SfxKind) {
        if let Some((engine, vol)) = self.audio.as_mut() {
            engine.play_sfx(kind, vol);
        }
    }

    /// An uprising event's message, from the world before it is applied:
    /// Uprising Begins / Ends (0x14b, FUN_00499460), Natural Disaster
    /// (0x154, FUN_0048e940), New Resources or Resources Depleted (0x155,
    /// FUN_0048e940) and the incident's line.
    #[expect(
        clippy::too_many_lines,
        reason = "each uprising event has its own text, picture, sound and rail"
    )]
    fn uprising_message(&mut self, world: &GameWorld, evt: &rebellion_core::uprising::UprisingEvent) {
        use rebellion_core::uprising::UprisingEvent;
        let message_texts = self.message_texts;
        // Timer 0x384 only sets the support (FUN_0050c9f0); it posts no
        // message of its own.
        if let UprisingEvent::SupportEroded { .. } = evt {
            return;
        }
        // Notification 0x155, New Resources or Resources Depleted
        // (FUN_0048e940, TEXTSTRA 0x7194-0x719a).
        if let UprisingEvent::ResourceChanged {
            system, tick, code, ..
        } = evt
        {
            let name = world
                .systems
                .get(*system)
                .map_or_else(|| "unknown".into(), |s| s.name.clone());
            if let Some((title, text)) =
                rebellion_render::story_messages::resource_texts(&name, *code, message_texts)
            {
                self.log.push(filed(
                    GameMessage::at_system(*tick, text, MessageCategory::Event, *system)
                        .with_title(title)
                        .with_picture_if(Some(pictures::system_incident(false)))
                        .with_sounds(MessageSounds::resources())
                        // FUN_0048e940 writes 0x1c.
                        .with_advisor_event(0x1c),
                    MessageRail::Resource,
                    system_audience(world, *system),
                ));
            }
            return;
        }
        let (system, tick) = match evt {
            UprisingEvent::ResourceChanged { system, tick, .. }
            | UprisingEvent::SupportEroded { system, tick, .. }
            | UprisingEvent::UprisingBegan { system, tick }
            | UprisingEvent::UprisingEnded { system, tick }
            | UprisingEvent::UprisingIncident { system, tick, .. }
            | UprisingEvent::Disaster { system, tick, .. } => (*system, *tick),
        };
        let name = world
            .systems
            .get(system)
            .map_or_else(|| "unknown".into(), |s| s.name.clone());
        let holder = world.systems.get(system).and_then(|s| s.control.faction());
        let (mut text, category) = match evt {
            UprisingEvent::UprisingBegan { .. } => {
                // A revolt helps the player when the enemy holds the system.
                let player_gains = holder.is_some_and(|holder| {
                    (holder == Faction::Alliance)
                        != (self.player_faction == MissionFaction::Alliance)
                });
                advisor_uprising(self.advisor, &name, player_gains);
                (format!("Uprising at {name}!"), MessageCategory::Diplomacy)
            }
            UprisingEvent::UprisingEnded { .. } => (
                format!("The uprising at {name} has ended"),
                MessageCategory::Diplomacy,
            ),
            UprisingEvent::UprisingIncident { .. } => (
                format!("Uprising incident at {name}"),
                MessageCategory::Diplomacy,
            ),
            UprisingEvent::Disaster { .. } => (
                format!("Natural disaster strikes {name}"),
                MessageCategory::Event,
            ),
            UprisingEvent::SupportEroded { .. } | UprisingEvent::ResourceChanged { .. } => return,
        };
        // FUN_00499460 and FUN_0048e940; an incident has no class.
        let picture = match evt {
            UprisingEvent::UprisingBegan { .. } => Some(pictures::uprising_begins()),
            UprisingEvent::UprisingEnded { .. } => {
                pictures::uprising_ends(pictures::system_side(world, system))
            }
            UprisingEvent::UprisingIncident { .. }
            | UprisingEvent::SupportEroded { .. }
            | UprisingEvent::ResourceChanged { .. } => None,
            UprisingEvent::Disaster { .. } => Some(pictures::system_incident(true)),
        };
        // The original texts: Uprising Begins / Ends (0x14b, FUN_00499460)
        // for both sides, Natural Disaster (0x154, FUN_0048e940) for the
        // holder. port: a disaster on a neutral system keeps the port's
        // line, off the Message Index.
        let mut title = None;
        let mut audience = system_audience(world, system);
        let original = match evt {
            UprisingEvent::UprisingBegan { .. } | UprisingEvent::UprisingEnded { .. } => {
                audience = holder
                    .filter(|side| *side != Faction::Neutral)
                    .map(|_| RailAudience::Both);
                rebellion_render::story_messages::uprising_texts(
                    &name,
                    holder,
                    matches!(evt, UprisingEvent::UprisingBegan { .. }),
                    message_texts,
                )
            }
            UprisingEvent::Disaster {
                total_energy,
                raw_materials,
                destroyed,
                ..
            } if audience.is_some() => {
                let units: Vec<String> = destroyed
                    .iter()
                    .filter_map(|&facility| world.facility(facility))
                    .filter_map(|(class, _)| {
                        world.buildable_classes.get(&class).map(|c| c.name.clone())
                    })
                    .collect();
                let resources = world.systems.get(system).is_some_and(|s| {
                    *total_energy < s.total_energy || *raw_materials < s.raw_materials
                });
                let texts = rebellion_render::story_messages::disaster_texts(
                    &name,
                    resources,
                    &units,
                    message_texts,
                );
                if texts.is_none() && !message_texts.is_empty() {
                    // Nothing lost: the class stays silent.
                    return;
                }
                texts
            }
            // Notification 0x152 (SystemUprisingIncidentNotif, FUN_00512580)
            // maps to no message type (FUN_00488de0): port: the incident
            // keeps its line in the log, off the Message Index.
            UprisingEvent::UprisingIncident { .. } => {
                audience = None;
                None
            }
            _ => None,
        };
        if let Some((original_title, body)) = original {
            title = Some(original_title);
            text = body;
        }
        // FUN_00499460's code by reader; FUN_0048e940 writes 0x1c.
        let events = match evt {
            UprisingEvent::UprisingBegan { .. } | UprisingEvent::UprisingEnded { .. } => {
                let begins = matches!(evt, UprisingEvent::UprisingBegan { .. });
                [
                    uprising_event(holder, Faction::Alliance, begins),
                    uprising_event(holder, Faction::Empire, begins),
                ]
            }
            UprisingEvent::Disaster { .. } => [0x1c; 2],
            _ => [0; 2],
        };
        // Notification 3, Uprising Message. FUN_00499460: 0x454 / 0x455;
        // FUN_0048e940: 0x44e.
        let sounds = match evt {
            UprisingEvent::UprisingBegan { .. } => MessageSounds::uprising(true),
            UprisingEvent::UprisingEnded { .. } => MessageSounds::uprising(false),
            UprisingEvent::Disaster { .. } => MessageSounds::disaster(),
            _ => MessageSounds::NONE,
        };
        let mut message = GameMessage::at_system(tick, text, category, system)
            .with_picture_if(picture)
            .with_sounds(sounds)
            .with_advisor_events(events[0], events[1]);
        if let Some(title) = title {
            message = message.with_title(title);
        }
        self.log.push(filed(message, uprising_message_rail(evt), audience));
    }
}

impl AppDayPresentation<'_> {
    /// A mission's report and the messages of its effects, from the world
    /// before the result is applied: the app's own report code runs on a
    /// copy of the world and research, so it reads each effect's state as
    /// it posts (`apply_mission_result`), while the shared integrator
    /// applies the result to the campaign.
    fn mission_result_messages(
        &mut self,
        world: &GameWorld,
        states: &rebellion_data::simulation::SimulationStates,
        result: &rebellion_core::missions::MissionResult,
        continuing: bool,
    ) {
        let message_texts = self.message_texts;
        let current_tick = states.clock.tick;
        let mut world = world.clone();
        let mut research = states.research.clone();
        apply_mission_result(
            result,
            continuing,
            states.uprising.is_uprising(result.target_system),
            &mut world,
            &mut research,
            self.log,
            message_texts,
            #[cfg(not(target_arch = "wasm32"))]
            self.audio.as_mut().map(|(engine, vol)| (&mut **engine, *vol)),
        );
        let world = &world;
        // Notification 0x212 (FUN_0048fa00), after Han's capture;
        // notification 0x1e0 case 5 (FUN_00490340) for a rescuer the Palace
        // mission's failure leaves with Jabba.
        for effect in &result.effects {
            if let MissionEffect::BountyAttack { character, .. } = effect {
                for message in rebellion_render::story_messages::bounty_messages(
                    world,
                    *character,
                    current_tick,
                    message_texts,
                ) {
                    self.log.push(message);
                }
            }
            // FUN_004fc080: an object destroyed by an assassination (reason
            // 7) posts 0x306, Killed by Imperial Assassins (FUN_0048b050);
            // one destroyed by sabotage (reason 6) posts 0x305, Death Star
            // Sabotaged for the Death Star (FUN_0048d470).
            let system_name = world
                .systems
                .get(result.target_system)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            if let MissionEffect::CharacterKilled { character, .. } = effect {
                if result.kind == MissionKind::Assassination {
                    if let Some(message) = world.characters.get(*character).and_then(|c| {
                        rebellion_render::story_messages::assassins_message(
                            c,
                            &system_name,
                            current_tick,
                            message_texts,
                        )
                    }) {
                        self.log.push(message);
                    }
                }
            }
            if let MissionEffect::TargetSabotaged {
                target: rebellion_core::missions::MissionTarget::DeathStar(_),
                system,
            } = effect
            {
                let name = world
                    .systems
                    .get(*system)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                // FUN_0048d470: a story movie, MDATA.104.
                if let Some(movie) =
                    rebellion_render::story_messages::death_star_sabotaged_movie(&name, message_texts)
                {
                    self.log.queue_story_movie(movie);
                }
            }
            // Notification 0x231 (FUN_0048b6e0) as the Pickup sets
            // PickupInProgress.
            if matches!(effect, MissionEffect::PickupInProgress) {
                if let Some(message) = jabba_delivers_message(world, current_tick, message_texts) {
                    self.log.push(message);
                }
            }
            if let MissionEffect::CharacterCaptured {
                character,
                captured_by,
                at_system,
            } = effect
            {
                if rebellion_core::jabba::held_at_palace(world, *character) {
                    if let Some(message) = rebellion_render::story_messages::jabba_captures_message(
                        &world.characters[*character],
                        current_tick,
                        message_texts,
                    ) {
                        self.log.push(message);
                    }
                } else if let Some(captive) = world.characters.get(*character) {
                    // Notification 0x1e0 (FUN_00490340), each side's message.
                    let place = world
                        .systems
                        .get(*at_system)
                        .map_or("", |system| system.name.as_str());
                    for reader in [CockpitFaction::Alliance, CockpitFaction::Empire] {
                        if let Some(message) = rebellion_render::story_messages::captured_message(
                            Some(*at_system),
                            captive,
                            *captured_by == MissionFaction::Alliance,
                            place,
                            reader,
                            current_tick,
                            message_texts,
                        ) {
                            self.log.push(message);
                        }
                    }
                }
            }
        }
        // Advisor trigger for player faction missions.
        if result.faction == self.player_faction {
            let kind_name = format!("{:?}", result.kind);
            let success = result.outcome == rebellion_core::missions::MissionOutcome::Success;
            advisor_mission_result(self.advisor, &kind_name, success);
        }
    }

    /// Notification 0x14b, Uprising Ends (FUN_00499460), for both sides of
    /// the held system, when a Subdue Uprising's success ends the revolt.
    fn uprising_subdued_message(
        &mut self,
        world: &GameWorld,
        ended: &rebellion_core::uprising::UprisingEvent,
    ) {
        let rebellion_core::uprising::UprisingEvent::UprisingEnded { system, tick } = *ended else {
            return;
        };
        let name = world
            .systems
            .get(system)
            .map_or_else(|| "unknown".into(), |s| s.name.clone());
        macroquad::logging::info!(
            "[interface] uprising_subdued system={} tick={}",
            name,
            tick
        );
        let holder = world.systems.get(system).and_then(|s| s.control.faction());
        let (title, text, audience) =
            match rebellion_render::story_messages::uprising_texts(
                &name,
                holder,
                false,
                self.message_texts,
            ) {
                Some((title, body)) => (Some(title), body, Some(RailAudience::Both)),
                None => (
                    None,
                    format!("Uprising subdued at {name}"),
                    system_audience(world, system),
                ),
            };
        let mut message = GameMessage::at_system(tick, text, MessageCategory::Diplomacy, system)
            .with_picture_if(pictures::uprising_ends(pictures::system_side(world, system)))
            .with_sounds(MessageSounds::uprising(false))
            .with_advisor_events(
                uprising_event(holder, Faction::Alliance, false),
                uprising_event(holder, Faction::Empire, false),
            );
        if let Some(title) = title {
            message = message.with_title(title);
        }
        self.log.push(filed(message, MessageRail::PopularSupport, audience));
    }
}

impl AppDayPresentation<'_> {
    /// The yards' completions and departures: Units Deployed (0x107,
    /// FUN_00497940), with the advisor's line and the build sound.
    fn manufactured_messages(
        &mut self,
        world: &GameWorld,
        transport: &rebellion_core::troop_transport::TroopTransportState,
        advance: &rebellion_core::manufacturing::ManufacturingAdvance,
    ) {
        let message_texts = self.message_texts;
        for completion in &advance.completions {
            let sys_name = world
                .systems
                .get(completion.system)
                .map_or_else(|| "unknown".into(), |s| s.name.clone());
            let aboard = rebellion_core::products::boarded_ship(world, transport, completion);
            let (title, text) = rebellion_render::story_messages::deployed_texts(
                world,
                completion.kind,
                completion.name.as_deref(),
                &sys_name,
                aboard,
                message_texts,
            )
            .map_or((None, format!("Construction complete at {sys_name}")), |(t, b)| {
                (Some(t), b)
            });
            let mut message = GameMessage::at_system(
                completion.tick,
                text,
                MessageCategory::Manufacturing,
                completion.system,
            )
            .with_picture_if(pictures::deployed(world, completion.kind))
            .with_sounds(MessageSounds::deployed(
                matches!(
                    completion.kind,
                    rebellion_core::manufacturing::BuildableKind::Fighter(_)
                ),
                system_audience(world, completion.system) != Some(RailAudience::Empire),
            ))
            // FUN_00497940 writes code 3 (advisor-briefing.md, "The events").
            .with_advisor_event(3);
            if let Some(title) = title {
                message = message.with_title(title);
            }
            self.log.push(filed(
                message,
                rebellion_render::story_messages::unit_message_rail(completion.kind, false),
                system_audience(world, completion.system),
            ));
            advisor_manufacturing_complete(self.advisor, &sys_name);
            #[cfg(not(target_arch = "wasm32"))]
            self.sfx(SfxKind::BuildComplete);
        }
        let system_name = |key| {
            world
                .systems
                .get(key)
                .map_or_else(|| "unknown".to_string(), |s| s.name.clone())
        };
        // A product travelling in its container leaves as a delivery does
        // (rebellion_core::products::settle).
        for departure in advance.departures.iter().chain(&advance.trips) {
            let origin = system_name(departure.origin);
            let destination = system_name(departure.destination);
            // Notification 0x107, Units Deployed (FUN_00497940): the product
            // is Deployed on completion (FUN_0052bee0), already bound to its
            // destination, which the message names.
            let aboard = rebellion_core::products::travelling_ship(world, departure);
            let (title, text) = rebellion_render::story_messages::deployed_texts(
                world,
                departure.kind,
                departure.name.as_deref(),
                &destination,
                aboard,
                message_texts,
            )
            .map_or(
                (
                    None,
                    format!("Construction complete at {origin}, en route to {destination}"),
                ),
                |(title, body)| (Some(title), body),
            );
            let mut message = GameMessage::at_system(
                departure.tick,
                text,
                MessageCategory::Manufacturing,
                departure.destination,
            )
            .with_picture_if(pictures::deployed(world, departure.kind))
            .with_sounds(MessageSounds::deployed(
                matches!(
                    departure.kind,
                    rebellion_core::manufacturing::BuildableKind::Fighter(_)
                ),
                system_audience(world, departure.origin) != Some(RailAudience::Empire),
            ))
            .with_advisor_event(3);
            if let Some(title) = title {
                message = message.with_title(title);
            }
            self.log.push(filed(
                message,
                rebellion_render::story_messages::unit_message_rail(departure.kind, false),
                system_audience(world, departure.origin),
            ));
            advisor_manufacturing_complete(self.advisor, &origin);
            #[cfg(not(target_arch = "wasm32"))]
            self.sfx(SfxKind::BuildComplete);
        }
    }

    /// Notification 0x105 clearing, Unit Arrival (FUN_004981c0): a product
    /// or delivery completed at its destination.
    fn unit_arrived_message(
        &mut self,
        world: &GameWorld,
        completion: &rebellion_core::manufacturing::CompletionEvent,
    ) {
        let arrived_at = world
            .systems
            .get(completion.system)
            .map_or_else(|| "unknown".to_string(), |s| s.name.clone());
        let (title, text) = rebellion_render::story_messages::arrival_texts(
            world,
            completion.kind,
            &arrived_at,
            self.message_texts,
        )
        .map_or((None, format!("Unit arrived at {arrived_at}")), |(t, b)| {
            (Some(t), b)
        });
        let mut message = GameMessage::at_system(
            completion.tick,
            text,
            MessageCategory::Manufacturing,
            completion.system,
        )
        .with_picture_if(pictures::deployed(world, completion.kind))
        // FUN_004981c0: ships are case 2, other units case 3.
        .with_sounds(MessageSounds::arrival(
            if matches!(
                completion.kind,
                rebellion_core::manufacturing::BuildableKind::CapitalShip(_)
            ) {
                pictures::Arrival::Fleet
            } else {
                pictures::Arrival::Units
            },
            system_audience(world, completion.system) != Some(RailAudience::Empire),
        ))
        .with_advisor_event(rebellion_render::story_messages::arrival_event(completion.kind));
        if let Some(title) = title {
            message = message.with_title(title);
        }
        self.log.push(filed(
            message,
            rebellion_render::story_messages::unit_message_rail(completion.kind, true),
            system_audience(world, completion.system),
        ));
    }

    /// Notification 0xd, Unit Arrival (FUN_004981c0 case 3), for regiments
    /// travelling on their own, and the line for those lost on arrival.
    fn regiment_arrival_messages(
        &mut self,
        world: &GameWorld,
        regiments: &rebellion_core::troop_transport::RegimentArrivals,
    ) {
        let message_texts = self.message_texts;
        for transit in &regiments.arrived {
            let system = match transit.leg {
                RegimentLeg::Surface(system) => Some(system),
                RegimentLeg::Fleet(fleet) => world.fleets.get(fleet).map(|f| f.location),
            };
            let Some(system) = system else { continue };
            let name = world
                .systems
                .get(system)
                .map_or_else(|| "unknown".into(), |s| s.name.clone());
            let audience = world
                .troops
                .get(transit.troop)
                .map(|troop| RailAudience::side(troop.is_alliance));
            // Notification 0xd, Unit Arrival (FUN_004981c0 case 3: 0x707a /
            // 0x707b and the unit's 0x707e line).
            let shipped = world.troops.get(transit.troop).and_then(|troop| {
                rebellion_render::story_messages::arrival_texts(
                    world,
                    rebellion_core::manufacturing::BuildableKind::Troop(troop.class_dat_id),
                    &name,
                    message_texts,
                )
            });
            let (title, text) = match shipped {
                Some((title, body)) => (Some(title), body),
                None => (None, format!("Regiment arrived at {name}")),
            };
            let mut message =
                GameMessage::at_system(transit.arrival_tick, text, MessageCategory::Mission, system);
            if let Some(title) = title {
                message = message.with_title(title);
            }
            self.log.push(filed(
                message
                    .with_picture_if(world.troops.get(transit.troop).and_then(|troop| {
                        pictures::arrival(
                            pictures::Arrival::Units,
                            pictures::reader(troop.is_alliance),
                        )
                    }))
                    .with_sounds(MessageSounds::arrival(
                        pictures::Arrival::Units,
                        audience != Some(RailAudience::Empire),
                    ))
                    // FUN_004981c0's units case (3), code 6.
                    .with_advisor_event(6),
                MessageRail::Fleet,
                audience,
            ));
        }
        for transit in &regiments.lost {
            // Event 0x303, GameObjDestroyedOnArrivalNotif.
            self.log.push(GameMessage::at_system(
                transit.arrival_tick,
                "Regiment lost on arrival: its destination no longer exists".to_string(),
                MessageCategory::Mission,
                transit.origin,
            ));
        }
    }

    /// Unit Arrival (FUN_004981c0 case 3) for squadrons travelling on their
    /// own, and the line for those lost on arrival.
    fn squadron_arrival_messages(
        &mut self,
        world: &GameWorld,
        arrived: &[rebellion_core::squadrons::SquadronTransit],
        lost: &[rebellion_core::squadrons::SquadronTransit],
    ) {
        let message_texts = self.message_texts;
        for transit in arrived {
            let system = match transit.to {
                SquadronPlace::Planet(system) => Some(system),
                SquadronPlace::Fleet(fleet) => world.fleets.get(fleet).map(|f| f.location),
            };
            let Some(system) = system else { continue };
            let name = world
                .systems
                .get(system)
                .map_or_else(|| "unknown".into(), |s| s.name.clone());
            let alliance = rebellion_core::squadrons::is_alliance(world, transit.class);
            let shipped = rebellion_render::story_messages::arrival_texts(
                world,
                rebellion_core::manufacturing::BuildableKind::Fighter(transit.class),
                &name,
                message_texts,
            );
            let (title, text) = match shipped {
                Some((title, body)) => (Some(title), body),
                None => (None, format!("Squadron arrived at {name}")),
            };
            let mut message =
                GameMessage::at_system(transit.arrival_tick, text, MessageCategory::Mission, system);
            if let Some(title) = title {
                message = message.with_title(title);
            }
            self.log.push(filed(
                message
                    .with_picture_if(alliance.and_then(|alliance| {
                        pictures::arrival(pictures::Arrival::Units, pictures::reader(alliance))
                    }))
                    .with_sounds(MessageSounds::arrival(
                        pictures::Arrival::Units,
                        alliance != Some(false),
                    ))
                    .with_advisor_event(6),
                MessageRail::Fleet,
                alliance.map(RailAudience::side),
            ));
        }
        for transit in lost {
            // Event 0x303, GameObjDestroyedOnArrivalNotif.
            self.log.push(GameMessage::at_system(
                transit.arrival_tick,
                "Squadron lost on arrival: its destination no longer exists".to_string(),
                MessageCategory::Mission,
                transit.origin,
            ));
        }
    }
}

impl rebellion_data::simulation::DayPresentation for AppDayPresentation<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "each of the day's timers posts its own messages"
    )]
    fn step(
        &mut self,
        world: &GameWorld,
        states: &rebellion_data::simulation::SimulationStates,
        step: &rebellion_data::simulation::DayStep<'_>,
        phase: rebellion_data::simulation::StepPhase,
    ) {
        use rebellion_core::uprising::UprisingEvent;
        use rebellion_data::simulation::{DayStep, StepPhase};
        let player_is_alliance = self.player_faction == MissionFaction::Alliance;
        let message_texts = self.message_texts;
        match (step, phase) {
            (DayStep::DawnBlockade(events) | DayStep::RunningRegiments(events), StepPhase::Before) => {
                blockade_messages(
                    world,
                    self.log,
                    events,
                    self.battled,
                    player_is_alliance,
                    message_texts,
                    self.cockpit_faction,
                );
            }
            // Bit 0x4000 holds for one pass (FUN_0050a9e0): the battles
            // recorded since the last blockade pass are spent.
            (DayStep::DawnBlockade(_), StepPhase::After) => self.battled.clear(),
            (DayStep::ResearchTally(_), StepPhase::Before) => {
                self.stash.research_done = [true, false]
                    .into_iter()
                    .flat_map(|alliance| {
                        [
                            rebellion_core::research::TechType::Ship,
                            rebellion_core::research::TechType::Troop,
                            rebellion_core::research::TechType::Facility,
                        ]
                        .into_iter()
                        .map(move |tech| (alliance, tech))
                    })
                    .map(|(alliance, tech)| {
                        (alliance, tech, states.research.progress(alliance, tech).done)
                    })
                    .collect();
            }
            (DayStep::ResearchTally(tallies), StepPhase::After) => {
                let tick = states.clock.tick;
                for tally in *tallies {
                    for level in &tally.reached {
                        if let Some(message) = research_complete_message(
                            world,
                            tally.faction_is_alliance,
                            tally.tech,
                            *level,
                            tick,
                            message_texts,
                        ) {
                            self.log.push(message);
                        }
                    }
                }
                // Notifications 0x129..0x12b, Research Exhausted (FUN_0048abc0).
                for (alliance, tech, was_done) in std::mem::take(&mut self.stash.research_done) {
                    if was_done || !states.research.progress(alliance, tech).done {
                        continue;
                    }
                    if let Some((title, text)) =
                        rebellion_render::story_messages::research_exhausted_texts(
                            tech,
                            message_texts,
                        )
                    {
                        self.log.push(filed(
                            GameMessage::new(tick, text, MessageCategory::Event)
                                .with_title(title)
                                .with_picture_if(Some(pictures::research()))
                                .with_sounds(MessageSounds::research())
                                // FUN_0048abc0 (kind 8) writes code 4.
                                .with_advisor_event(4)
                                .plain(),
                            MessageRail::Manufacturing,
                            Some(RailAudience::side(alliance)),
                        ));
                    }
                }
            }
            (DayStep::Autoscraps(scraps), StepPhase::After) => {
                for scrap in *scraps {
                    // A fleet in hyperspace stands at no system its units
                    // can be found at (FUN_00498970 walks up the containers).
                    let in_transit = match scrap.target {
                        rebellion_core::autoscrap::AutoscrapTarget::Ship { fleet, .. }
                        | rebellion_core::autoscrap::AutoscrapTarget::FleetFighter {
                            fleet, ..
                        } => states.movement.is_in_transit(fleet),
                        _ => false,
                    };
                    if let Some(message) = maintenance_message(
                        world,
                        scrap.system.filter(|_| !in_transit),
                        &scrap.unit,
                        scrap.side == Faction::Alliance,
                        scrap.tick,
                        message_texts,
                    ) {
                        self.log.push(message);
                    }
                }
            }
            (DayStep::Repairs(events), StepPhase::Before) => {
                self.stash.ships_damaged = events
                    .iter()
                    .map(|evt| match evt {
                        RepairEvent::ShipRepaired { fleet, ship_index, .. } => {
                            ship_damaged(world, *fleet, *ship_index)
                        }
                        _ => None,
                    })
                    .collect();
            }
            (DayStep::Repairs(events), StepPhase::After) => {
                let repair_tick = states.clock.tick;
                let damaged = std::mem::take(&mut self.stash.ships_damaged);
                for (evt, was_damaged) in events.iter().zip(damaged) {
                    match evt {
                        // The damage flag clearing posts 0x106: Capital
                        // Ship Repaired (FUN_00499de0).
                        RepairEvent::ShipRepaired { fleet, ship_index, .. } => {
                            if was_damaged == Some(true)
                                && ship_damaged(world, *fleet, *ship_index) == Some(false)
                            {
                                if let Some(message) = ship_repaired_message(
                                    world,
                                    *fleet,
                                    *ship_index,
                                    repair_tick,
                                    message_texts,
                                ) {
                                    self.log.push(message);
                                }
                            }
                        }
                        // A squadron's losses reaching 0 clear its damage
                        // flag (FUN_00503340): Squadron at Full Strength
                        // (FUN_00499de0).
                        RepairEvent::SquadronRepaired {
                            place,
                            index,
                            lost_before,
                            damage: None,
                        } if *lost_before > 0 => {
                            if let Some(message) = squadron_full_strength_message(
                                world,
                                *place,
                                *index,
                                repair_tick,
                                message_texts,
                            ) {
                                self.log.push(message);
                            }
                        }
                        _ => {}
                    }
                }
            }
            (DayStep::Healed(healed), StepPhase::After) => {
                for heal in *healed {
                    if heal.injury == 0 {
                        push_character_health(
                            self.log,
                            world,
                            heal.character,
                            heal.tick,
                            message_texts,
                        );
                    }
                }
            }
            (DayStep::ForceDiscovered(discovered), StepPhase::After) => {
                push_force_discoveries(self.log, world, discovered, message_texts);
            }
            (DayStep::Informants(reports), StepPhase::Before) => {
                for report in *reports {
                    // Notification 0x153 (FUN_00491060).
                    if let Some(message) = informants_message(
                        world,
                        report.system,
                        report.recipient == Faction::Alliance,
                        report.tick,
                        message_texts,
                    ) {
                        self.log.push(message);
                    }
                }
            }
            (DayStep::Uprisings(events), StepPhase::Before) => {
                for evt in *events {
                    self.uprising_message(world, evt);
                }
            }
            (DayStep::Uprisings(events), StepPhase::After) => {
                for evt in *events {
                    if let UprisingEvent::UprisingIncident { losses, tick, .. } = evt {
                        for loss in losses {
                            if let rebellion_core::uprising::IncidentLoss::CharacterInjured {
                                character,
                                ..
                            } = loss
                            {
                                push_character_health(
                                    self.log,
                                    world,
                                    *character,
                                    *tick,
                                    message_texts,
                                );
                            }
                        }
                    }
                }
            }
            (DayStep::MissionAdvance(advance), StepPhase::Before) => {
                self.stash.force_changes = advance_force_changes(world, &advance.effects);
            }
            (DayStep::MissionAdvance(advance), StepPhase::After) => {
                let current_tick = states.clock.tick;
                // Member moves and releases come before the results.
                let force_changes = std::mem::take(&mut self.stash.force_changes);
                push_advance_messages(
                    self.log,
                    world,
                    &advance.effects,
                    &force_changes,
                    current_tick,
                    message_texts,
                );
                // Notification 0x361 (FUN_0048c9c0): a member learned the
                // traitor.
                for effect in &advance.effects {
                    if let MissionEffect::TraitorIdentified { character, traitor } = effect {
                        if let (Some(discoverer), Some(traitor)) =
                            (world.characters.get(*character), world.characters.get(*traitor))
                        {
                            if let Some(message) =
                                rebellion_render::story_messages::discovers_traitor_message(
                                    discoverer,
                                    traitor,
                                    current_tick,
                                    message_texts,
                                )
                            {
                                self.log.push(message);
                            }
                        }
                    }
                }
                push_foiled_missions(self.log, world, &advance.ended, message_texts);
                push_personnel_arrivals(self.log, world, &advance.arrivals, message_texts);
                // The encounter class (vtable 0x65b408): Force detection and
                // the confrontation's 0x360 and Luke's 0x211.
                for message in
                    rebellion_render::story_messages::EncounterNotes::gather(world, &advance.effects)
                        .messages(world, &states.missions, current_tick, message_texts)
                {
                    self.log.push(message);
                }
            }
            (DayStep::LukeToDagobah(tick), StepPhase::After) => {
                if let Some(message) =
                    rebellion_render::story_messages::luke_goes_to_dagobah(*tick, message_texts)
                {
                    self.log.push(message);
                }
            }
            // Notification 0x210 when the Dagobah mission sets
            // DagobahCompleted (FUN_005751d0).
            (DayStep::DagobahCompleted(effects), StepPhase::After) => {
                for effect in *effects {
                    if let MissionEffect::DagobahCompleted { character, .. } = effect {
                        if let Some(message) = rebellion_render::story_messages::luke_leaves_dagobah(
                            world,
                            *character,
                            states.clock.tick,
                            message_texts,
                        ) {
                            self.log.push(message);
                        }
                    }
                }
            }
            // FUN_0048e3c0 on 0x220.
            (DayStep::FinalBattle(step), StepPhase::After) => {
                if let rebellion_core::final_battle::FinalBattleStep::Battle(battle) = step {
                    if let Some(message) = rebellion_render::story_messages::final_battle_message(
                        world,
                        battle.luke,
                        battle.tick,
                        message_texts,
                    ) {
                        self.log.push(message);
                    }
                }
            }
            // Notification 0x200 (FUN_0048c4c0), Rescue Attempt. port: posted
            // as the Palace mission is dispatched; the original posts it as
            // the rescuer sets out (EnrouteActive).
            (DayStep::Jabba(step, true), StepPhase::After) => {
                if let rebellion_core::jabba::JabbaStep::Rescue { rescuer, .. } = step {
                    if let Some(message) = world.characters.get(*rescuer).and_then(|c| {
                        rebellion_render::story_messages::rescue_attempt_message(
                            c,
                            states.clock.tick,
                            message_texts,
                        )
                    }) {
                        self.log.push(message);
                    }
                }
            }
            // Notification 0x230, Emperor Arrives at Coruscant (FUN_0048ce80),
            // when SeatOfPower is set.
            (DayStep::SeatOfPower(true), StepPhase::After) => {
                if let Some(message) = rebellion_render::story_messages::emperor_arrives_message(
                    states.clock.tick,
                    message_texts,
                ) {
                    self.log.push(message);
                }
            }
            (
                DayStep::MissionResult {
                    result, continuing, ..
                },
                StepPhase::Before,
            ) => {
                self.mission_result_messages(world, states, result, *continuing);
            }
            (DayStep::MissionResult { ended, .. }, StepPhase::After) => {
                for ended in *ended {
                    self.uprising_subdued_message(world, ended);
                }
            }
            (DayStep::Escapes(effects), StepPhase::Before) => {
                for effect in *effects {
                    if let MissionEffect::CharacterEscaped {
                        character,
                        escaped_to_alliance,
                    } = effect
                    {
                        escape_messages(
                            world,
                            *character,
                            *escaped_to_alliance,
                            states.clock.tick,
                            self.log,
                            message_texts,
                        );
                    }
                }
            }
            (DayStep::EventFired(fired, effects), StepPhase::After) => {
                let current_tick = states.clock.tick;
                for eff in *effects {
                    use rebellion_core::effects::GameEffect;
                    match eff {
                        GameEffect::StoryMessageDisplayed { text, .. } => {
                            // A story event is a message the player reads in
                            // the Message window (DEV-UI-019 retired).
                            self.log.push(rebellion_render::story_messages::story_message(
                                world,
                                fired.event_id,
                                &fired.actions,
                                text.clone(),
                                current_tick,
                                self.cockpit_faction,
                                message_texts,
                                MessageCategory::Event,
                            ));
                        }
                        GameEffect::SpecialForceSpawned {
                            at_system,
                            is_alliance,
                        } => {
                            let name = world
                                .systems
                                .get(*at_system)
                                .map_or_else(|| "unknown".into(), |s| s.name.clone());
                            let side = if *is_alliance { "Alliance" } else { "Imperial" };
                            self.log.push(GameMessage::at_system(
                                current_tick,
                                format!("{side} special force lands at {name}"),
                                MessageCategory::Event,
                                *at_system,
                            ));
                        }
                        _ => {}
                    }
                }
            }
            (DayStep::Economy(events), StepPhase::Before) => {
                self.stash.previous_control = events
                    .iter()
                    .filter_map(|ev| match ev {
                        EconomyEvent::ControlResolved { system, .. } => Some((
                            *system,
                            world.systems.get(*system).and_then(|s| s.control.faction()),
                        )),
                        _ => None,
                    })
                    .collect();
            }
            (DayStep::Economy(events), StepPhase::After) => {
                let tick = states.clock.tick;
                let previous = std::mem::take(&mut self.stash.previous_control);
                for ev in *events {
                    match ev {
                        EconomyEvent::ControlResolved {
                            system,
                            new_control,
                        } => {
                            let before = previous
                                .iter()
                                .find(|(key, _)| key == system)
                                .and_then(|(_, side)| *side);
                            // Notification 0x100, System Joins / Declares
                            // Neutrality (FUN_00499760), while the support
                            // alone moved it. port: no regiment at the
                            // system stands for bit 0x2000 (FUN_00510d70).
                            if world
                                .systems
                                .get(*system)
                                .is_some_and(|sys| sys.ground_units.is_empty())
                            {
                                if let Some(message) = system_joins_message(
                                    world,
                                    *system,
                                    before,
                                    new_control.faction(),
                                    player_is_alliance,
                                    tick,
                                    message_texts,
                                ) {
                                    self.log.push(message);
                                }
                            }
                        }
                        // Notification 0x145, Smuggling (FUN_00499130).
                        EconomyEvent::SmugglingPercentChanged {
                            system,
                            previous,
                            percent,
                        } => {
                            if let Some(sys) = world.systems.get(*system) {
                                if let Some(message) =
                                    rebellion_render::story_messages::smuggling_message(
                                        &sys.name,
                                        sys.control.faction(),
                                        player_is_alliance,
                                        *previous,
                                        *percent,
                                        tick,
                                        message_texts,
                                    )
                                {
                                    self.log.push(message);
                                }
                            }
                        }
                        // Notification 0x148, Near Uprising (FUN_00498cb0).
                        EconomyEvent::TroopSurplusChanged {
                            system,
                            previous,
                            surplus,
                        } => {
                            if let Some(sys) = world.systems.get(*system) {
                                let holder = sys.control.faction();
                                if let Some(message) =
                                    rebellion_render::story_messages::near_uprising_message(
                                        *system,
                                        &sys.name,
                                        holder,
                                        player_is_alliance,
                                        // +0x88 bit 8, Garrisoned (FUN_0050ba90).
                                        !sys.ground_units.is_empty(),
                                        states.uprising.is_uprising(*system),
                                        *previous,
                                        *surplus,
                                        tick,
                                        message_texts,
                                    )
                                {
                                    self.log.push(message.with_picture_if(
                                        holder.and_then(pictures::near_uprising),
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            (DayStep::Manufactured(advance), StepPhase::After) => {
                self.manufactured_messages(world, &states.troop_transport, advance);
            }
            (DayStep::UnitsArrived(completions), StepPhase::After) => {
                for completion in *completions {
                    self.unit_arrived_message(world, completion);
                }
            }
            // Notification 0x160, Facility Idle (FUN_0048be60): a manager's
            // remaining count reached 0. Only the holder's side hears it,
            // and only while the system has a facility of that area.
            (DayStep::Idle(idle), StepPhase::After) => {
                for &(system, area) in *idle {
                    let Some(sys) = world.systems.get(system) else {
                        continue;
                    };
                    let player_holds = sys.control.faction()
                        == Some(if player_is_alliance {
                            Faction::Alliance
                        } else {
                            Faction::Empire
                        });
                    let picture = pictures::idle_yard(world, system, area);
                    if !player_holds || picture.is_none() {
                        continue;
                    }
                    if let Some((title, text)) =
                        rebellion_render::story_messages::facility_idle_texts(
                            &sys.name,
                            area,
                            message_texts,
                        )
                    {
                        self.log.push(filed(
                            GameMessage::at_system(
                                states.clock.tick,
                                text,
                                MessageCategory::Manufacturing,
                                system,
                            )
                            .with_title(title)
                            .with_picture_if(picture)
                            .with_sounds(MessageSounds::facility_idle())
                            // FUN_0048be60 writes code 3.
                            .with_advisor_event(3)
                            // Class_0065b260_vt30: kept while the manager idles.
                            .with_lifetime(
                                rebellion_render::message_log::MessageLifetime::WhileIdle {
                                    system,
                                    area,
                                },
                            ),
                            MessageRail::Manufacturing,
                            Some(RailAudience::side(player_is_alliance)),
                        ));
                    }
                }
            }
            (DayStep::FleetArrivals(arrivals), StepPhase::After) => {
                for arrival in *arrivals {
                    let Some(fleet) = world
                        .fleets
                        .get(arrival.fleet)
                        .filter(|fleet| fleet.location == arrival.system)
                    else {
                        continue;
                    };
                    self.log.push(fleet_arrival_message(
                        world,
                        arrival.fleet,
                        arrival.system,
                        fleet.is_alliance,
                        arrival.tick,
                        message_texts,
                    ));
                    #[cfg(not(target_arch = "wasm32"))]
                    self.sfx(SfxKind::FleetArrival);
                }
            }
            (DayStep::RegimentsArrived(regiments), StepPhase::After) => {
                self.regiment_arrival_messages(world, regiments);
            }
            (DayStep::SquadronsArrived(arrived, lost), StepPhase::After) => {
                self.squadron_arrival_messages(world, arrived, lost);
            }
            // The headquarters' trip ends (rebellion_core::headquarters_move):
            // Unit Arrival case 4 (FUN_004981c0), to the Alliance.
            (DayStep::HeadquartersArrived(transit), StepPhase::After) => {
                macroquad::logging::info!(
                    "[interface] headquarters arrived day={}",
                    transit.arrival_tick
                );
                if let Some(message) = headquarters_arrival_message(world, transit, message_texts) {
                    self.log.push(message);
                }
            }
            (DayStep::BattleFought(result), StepPhase::After) => {
                self.battled.insert(result.system);
                let sys_name = world
                    .systems
                    .get(result.system)
                    .map_or_else(|| "Unknown".into(), |s| s.name.clone());
                let winner_str = match result.winner {
                    CombatSide::Attacker => "Alliance victory",
                    CombatSide::Defender => "Empire victory",
                    CombatSide::Draw => "Draw",
                };
                self.log.push(GameMessage::at_system(
                    states.clock.tick,
                    format!("Space battle at {sys_name} — {winner_str}"),
                    MessageCategory::Combat,
                    result.system,
                ));
                #[cfg(not(target_arch = "wasm32"))]
                self.sfx(SfxKind::CombatStart);
            }
            // port: a log line only. Regiments taking the planet set its
            // control bits 0x30, so FUN_00510d70 leaves bit 0x2000 clear and
            // System Joins (FUN_00499760) stays silent; notification 0x31b
            // has no message (FUN_00488de0).
            (DayStep::Occupied(system, winner), StepPhase::After) => {
                let name = world
                    .systems
                    .get(*system)
                    .map_or("unknown", |value| value.name.as_str());
                self.log.push(GameMessage::at_system(
                    states.clock.tick,
                    format!("{name} occupied by {winner:?}"),
                    MessageCategory::Combat,
                    *system,
                ));
            }
            // port: the fog only records where each side's fleets have
            // stood; the runner's fog is the Alliance's, and the Empire's
            // advances here (ghidra/notes/fog-of-war.md).
            (DayStep::FogAdvanced, StepPhase::After) => {
                FogSystem::advance(self.other_fog, world, &states.movement);
            }
            _ => {}
        }
    }

    fn take_battle(
        &mut self,
        world: &mut GameWorld,
        states: &mut rebellion_data::simulation::SimulationStates,
        system: SystemKey,
        sorties: &[rebellion_core::squadrons::Sortie],
        roll: f64,
    ) -> bool {
        // One player battle at a time, while the galaxy shows.
        if *self.game_mode != GameMode::Galaxy {
            return false;
        }
        let first_fleet = |alliance: bool| {
            world.systems.get(system).and_then(|value| {
                value.fleets.iter().copied().find(|&key| {
                    world
                        .fleets
                        .get(key)
                        .is_some_and(|fleet| fleet.is_alliance == alliance)
                })
            })
        };
        let (Some(attacker), Some(defender)) = (first_fleet(true), first_fleet(false)) else {
            return false;
        };
        let player_is_alliance = self.player_faction == MissionFaction::Alliance;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a roll in 0..1 scaled to the battle's 32-bit seed"
        )]
        let rng_seed = (roll * f64::from(u32::MAX)) as u32;
        if let Err(error) = tactical_flow::begin_player_battle(
            world,
            &states.troop_transport,
            tactical_flow::BattleEntry {
                system,
                attacker,
                defender,
                // The attacker is the Alliance's first fleet.
                player_is_attacker: player_is_alliance,
                tick: states.clock.tick,
                rng_seed,
            },
            self.tactical_state,
            &mut states.combat_cooldowns,
            self.log,
            self.game_mode,
        ) {
            eprintln!("Skipping invalid tactical battle: {error:?}");
            return false;
        }
        *self.battle_sorties = sorties.to_vec();
        // FUN_00485d50: the AI side answers the battle choice with its
        // retreat order, which Take Command's battle keeps (the battle
        // object is shared by both modes).
        let ais: Vec<&AIState> = std::iter::once(&states.ai).chain(&states.ai2).collect();
        let orders = rebellion_core::ai_strategy::retreat_orders(
            &ais,
            world,
            &states.movement,
            &states.troop_transport,
            system,
        );
        if let Some(session) = self.tactical_state.session.as_mut() {
            session.retreat_allowed = orders;
            session.retreat_allowed[usize::from(!player_is_alliance)] = false;
        }
        self.tactical_state.set_display_options(self.tactical_flags);
        #[cfg(not(target_arch = "wasm32"))]
        self.sfx(SfxKind::CombatStart);
        true
    }

    fn ai_actions(&mut self, world: &GameWorld, orders: &rebellion_data::simulation::AiOrders<'_>) {
        let side = match orders.side {
            AiFaction::Alliance => "Alliance",
            AiFaction::Empire => "Empire",
        };
        let tick = orders.day;
        let mut bombardments = orders.bombardments.iter();
        let mut assaults = orders.assaults.iter();
        for (action, &applied) in orders.actions.iter().zip(orders.applied) {
            // Each Bombard or Assault that ran left its outcome, in order.
            match action {
                AIAction::Bombard { system, .. } if applied => {
                    if let Some((_, outcome)) = bombardments.next() {
                        log_bombardment(world, *system, outcome, tick, self.log);
                    }
                    continue;
                }
                AIAction::Assault { system, .. } if applied => {
                    if let Some((_, outcome)) = assaults.next() {
                        log_assault(world, *system, outcome, tick, self.log);
                    }
                    continue;
                }
                _ => {}
            }
            if !applied {
                continue;
            }
            if let AIAction::Scrap { team } = action {
                self.log.push(GameMessage::new(
                    tick,
                    format!("{side} scrapped {} unit(s)", team.len()),
                    MessageCategory::Ai,
                ));
                continue;
            }
            let line = match action {
                AIAction::DispatchMission {
                    kind,
                    target_system,
                    ..
                } => Some((format!("{side} dispatched {kind:?} mission"), *target_system)),
                AIAction::JoinFleet { into, .. } => world
                    .fleets
                    .get(*into)
                    .map(|fleet| (format!("{side} fleet joining a fleet"), fleet.location)),
                AIAction::JoinShips { ships, into, .. } => world.fleets.get(*into).map(|fleet| {
                    (
                        format!("{side} moving {} ship(s) into a fleet", ships.len()),
                        fleet.location,
                    )
                }),
                AIAction::MoveShips { ships, to, .. }
                | AIAction::MoveShipsIntoSpare { ships, to, .. } => Some((
                    format!("{side} moving {} ship(s) to system", ships.len()),
                    *to,
                )),
                AIAction::MoveFleet {
                    to_system,
                    reason,
                    troops,
                    ..
                } => {
                    #[cfg(not(target_arch = "wasm32"))]
                    self.sfx(SfxKind::FleetDeparture);
                    let reason = match reason {
                        FleetMoveReason::Attack => "attack",
                        FleetMoveReason::Reinforce => "reinforce",
                    };
                    let cargo = if troops.is_empty() {
                        String::new()
                    } else {
                        format!(" with {} regiment(s)", troops.len())
                    };
                    Some((format!("{side} fleet moving to system ({reason}){cargo}"), *to_system))
                }
                _ => None,
            };
            if let Some((text, system)) = line {
                self.log
                    .push(GameMessage::at_system(tick, text, MessageCategory::Ai, system));
            }
        }
    }

    fn pass_settling(
        &mut self,
        world: &GameWorld,
        bombard: &rebellion_core::bombardment::BombardEnv,
        day: u64,
    ) {
        present_assault_reports(
            bombard.reports.clone(),
            world,
            self.player_faction,
            self.message_texts,
            self.assault_summary,
            self.log,
            day,
        );
        self.stash.destroyed = bombard
            .destroyed_systems()
            .into_iter()
            .map(|system| {
                let value = world.systems.get(system);
                (
                    system,
                    value.map_or_else(|| "Unknown".to_string(), |s| s.name.clone()),
                    value.and_then(|s| s.control.faction()),
                    bombard.killed_at(system),
                )
            })
            .collect();
    }

    /// A planet destroyed by a Death Star: Character Health for each
    /// character it killed (0x106, FUN_0048fdf0), then System Destroyed by
    /// the Death Star (0x182, FUN_004950d0), a story movie, MDATA.101.
    fn planets_destroyed(
        &mut self,
        world: &GameWorld,
        destroyed: &[rebellion_core::death_star::DeathStarEvent],
    ) {
        for event in destroyed {
            let rebellion_core::death_star::DeathStarEvent::PlanetDestroyed { system, tick } = event;
            let Some((_, name, holder, killed)) =
                self.stash.destroyed.iter().find(|(key, ..)| key == system)
            else {
                continue;
            };
            for &character in killed {
                push_character_health(self.log, world, character, *tick, self.message_texts);
            }
            self.log.queue_story_movie(
                rebellion_render::story_messages::death_star_destroyed_movie(
                    name,
                    *holder,
                    self.message_texts,
                ),
            );
        }
        self.stash.destroyed.clear();
    }

    // Bit 0x4000 holds for one pass (FUN_0050a9e0).
    fn pass_settled(&mut self) {
        self.battled.clear();
    }

    fn ai_day_begun(&mut self, day: u64, speed: GameSpeed, passes: u32) {
        macroquad::logging::info!("[ai] day={} speed={:?} passes={}", day, speed, passes);
    }

    fn player_agent(
        &mut self,
        world: &GameWorld,
        states: &mut rebellion_data::simulation::SimulationStates,
        days: u32,
    ) {
        // Not in dual-AI mode, where an AI plays the player's side.
        if states.ai2.is_some() {
            return;
        }
        for _ in 0..days {
            let orders = self.player_agent.advance(
                world,
                &states.manufacturing,
                &states.economy,
                self.player_faction == MissionFaction::Alliance,
            );
            for order in &orders {
                macroquad::logging::info!("[agent] order={:?}", order);
            }
            rebellion_core::agent_automation::apply_orders(&orders, &mut states.manufacturing);
        }
    }

    fn blockade(&mut self, world: &GameWorld, events: &[rebellion_core::blockade::BlockadeEvent]) {
        blockade_messages(
            world,
            self.log,
            events,
            self.battled,
            self.player_faction == MissionFaction::Alliance,
            self.message_texts,
            self.cockpit_faction,
        );
    }

    fn victory_word(&mut self, _world: &GameWorld, new: u32, day: u64) {
        victory_word_movies(self.log, new, self.message_texts, day);
    }

    fn game_decided(
        &mut self,
        _world: &GameWorld,
        outcome: &rebellion_core::victory::VictoryOutcome,
        victory: &VictoryState,
        day: u64,
    ) {
        game_decided_messages(
            outcome,
            victory,
            self.log,
            self.advisor_schedule.as_deref_mut(),
            self.advisor_now,
            self.pending_ending,
            self.player_faction == MissionFaction::Alliance,
            day,
        );
    }
}
