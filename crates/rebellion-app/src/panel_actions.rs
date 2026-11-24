//! The War Room's panel actions (`PanelAction`) applied to the game, and
//! a band's Destination drop, moved out of `main.rs` (roadmap M2). The
//! code reads `main.rs`'s items through `super`.

use super::*;

/// A band's Destination (`0x214`) dropped on `target` (`FUN_00422ce0`,
/// window type 9): a system sets the band's own area; a fleet, or a ship
/// entry of it, is taken by a shipyard or training band of the fleet's side
/// and refused for a construction band (`FUN_00537180`, `1`/`0x25`). The
/// refusal and an empty drop log only.
#[expect(
    clippy::too_many_arguments,
    reason = "The drop reads the windows under the point and the player's side."
)]
pub(super) fn band_destination(
    ctx: &egui_macroquad::egui::Context,
    world: &GameWorld,
    movement: &rebellion_core::movement::MovementState,
    fog: &FogState,
    layout: rebellion_render::cockpit::CockpitLayout,
    windows: ReleaseWindows<'_>,
    pointer: egui_macroquad::egui::Pos2,
    (system, area): (SystemKey, rebellion_core::manufacturing::ProductionArea),
    target: Option<ReleaseTarget>,
    is_alliance: bool,
) -> Option<PanelAction> {
    match target {
        Some(ReleaseTarget::System(destination)) => Some(PanelAction::SetDestination {
            system,
            area: Some(area),
            destination,
        }),
        Some(ReleaseTarget::Fleet { fleet, system: at }) => {
            match rebellion_core::manufacturing::fleet_destination_area(
                world,
                Some(area),
                fleet,
                is_alliance,
            ) {
                Ok(area) => Some(PanelAction::SetFleetDestination {
                    system,
                    area,
                    fleet,
                    at,
                    ship: rebellion_render::targeting::release_ship(
                        ctx, world, movement, fog, layout, windows, pointer,
                    )
                    .filter(|&(of, _)| of == fleet)
                    .map(|(_, ship)| ship),
                }),
                Err(refusal) => {
                    macroquad::logging::info!(
                        "[interface] command=0x214 destination=production_destination status=refused drop=1 reason={}",
                        refusal.status().map_or_else(|| "other_side".to_owned(), |status| format!("{status:#x}"))
                    );
                    None
                }
            }
        }
        None => {
            macroquad::logging::info!(
                "[interface] command=0x214 destination=production_destination status=refused drop=1"
            );
            None
        }
    }
}

pub(super) fn apply_panel_action(
    action: PanelAction,
    world: &mut GameWorld,
    map_state: &mut GalaxyMapState,
    mfg_state: &mut ManufacturingState,
    mission_state: &mut MissionState,
    movement_state: &mut MovementState,
    troop_transport_state: &mut TroopTransportState,
    fog_state: &mut FogState,
    ai_state: &mut AIState,
    msg_log: &mut MessageLog,
    player_faction: &mut MissionFaction,
    clock: &mut GameClock,
    dual_ai_mode: &mut bool,
    secondary_ai_state: &mut Option<AIState>,
    victory_state: &mut VictoryState,
    campaign_config: CampaignConfig,
    blockade_state: &mut BlockadeState,
    event_state: &EventState,
    mod_runtime: &mut rebellion_data::mods::ModRuntime,
    message_texts: &std::collections::HashMap<u16, String>,
    assault_summaries: &mut rebellion_render::assault_summary::AssaultSummaryState,
    refusal: &mut Option<RefusedOrder>,
    #[cfg(not(target_arch = "wasm32"))] audio_engine: &mut audio::AudioEngine,
    #[cfg(not(target_arch = "wasm32"))] audio_vol: &AudioVolumeState,
    #[cfg(not(target_arch = "wasm32"))] _sounds_dir: &Path,
) {
    match action {
        // FUN_00578f30 → FUN_00556390 (ghidra/notes/regiment-unload.md).
        PanelAction::MoveRegiment { troop, target } => {
            // The shared order (rebellion_data::player_command).
            let result = rebellion_data::player_command::move_regiment(
                world,
                troop_transport_state,
                movement_state,
                blockade_state.blockaded_systems(),
                troop,
                target,
                clock.tick,
                *player_faction == MissionFaction::Alliance,
            );
            // port: FUN_00487c90's advisor reaction; this line stands in.
            if let Err(error) = result {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        format!("Regiment move rejected: {error}"),
                        MessageCategory::Event,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        // FUN_00578f30 → FUN_00556390 for the headquarters
        // (rebellion_core::headquarters_move).
        PanelAction::MoveHeadquarters { destination } => {
            let result = rebellion_core::headquarters_move::move_headquarters(
                world,
                troop_transport_state,
                victory_state,
                *player_faction == MissionFaction::Alliance,
                destination,
                clock.tick,
            );
            macroquad::logging::info!(
                "[interface] command=0x201 headquarters destination={:?} result={:?}",
                world.systems.get(destination).map(|value| value.name.as_str()),
                result
            );
            // port: FUN_00487c90's advisor reaction; this line stands in.
            if let Err(error) = result {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        format!("Headquarters move rejected: {error}"),
                        MessageCategory::Event,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        // FUN_00557ce0's 0x201 for a squadron (rebellion_core::squadrons).
        PanelAction::MoveSquadron { class, from, to } => {
            let alliance = *player_faction == MissionFaction::Alliance;
            // port: FUN_00487c90's advisor reaction; this line stands in.
            if let Err(error) = rebellion_data::player_command::move_squadron(
                world,
                troop_transport_state,
                alliance,
                class,
                from,
                to,
                clock.tick,
            ) {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        format!("Squadron move rejected: {error}"),
                        MessageCategory::Event,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        PanelAction::FocusFleetSystem(sys_key) => {
            if world.systems.contains_key(sys_key) {
                map_state.selected_system = Some(sys_key);
            }
        }
        // ghidra/notes/fleet-join-split.md.
        PanelAction::JoinFleet { mover, target } => {
            let result = rebellion_core::fleet_join::join_fleet(
                world,
                movement_state,
                troop_transport_state,
                &mover,
                target,
                *player_faction == MissionFaction::Alliance,
            );
            if let Err(error) = result {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        format!("Fleet move rejected: {error}"),
                        MessageCategory::Event,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        PanelAction::SetDestination {
            system,
            area,
            destination,
        } => {
            let areas = area.map_or(
                rebellion_core::manufacturing::ProductionArea::ALL.to_vec(),
                |area| vec![area],
            );
            for area in &areas {
                mfg_state.set_destination(system, *area, destination);
            }
            macroquad::logging::info!(
                "[interface] command=0x214 destination=production_destination status=set areas={} to={}",
                areas.len(),
                world
                    .systems
                    .get(destination)
                    .map_or("", |value| value.name.as_str())
            );
        }
        PanelAction::SetFleetDestination {
            system,
            area,
            fleet,
            at,
            ship,
        } => {
            let set = match ship {
                Some(ship) => mfg_state.set_ship_destination(system, area, fleet, ship, at),
                None => mfg_state.set_fleet_destination(system, area, fleet, at),
            };
            let named =
                ship.and_then(|ship| rebellion_core::products::ship_name(world, fleet, ship));
            macroquad::logging::info!(
                "[interface] command=0x214 destination=production_destination status={} areas=1 {}={}",
                if set { "set" } else { "refused" },
                if named.is_some() { "to_ship" } else { "to_fleet" },
                named.or_else(|| world.fleet_name(fleet)).unwrap_or("")
            );
        }
        PanelAction::RenameProduct { product, name } => {
            let renamed = rebellion_core::products::rename(world, mfg_state, product, name);
            macroquad::logging::info!(
                "[interface] command=0x203 product={product} renamed={renamed}"
            );
        }
        PanelAction::Rename { fleet, ship, name } => {
            let renamed = match ship {
                Some(index) => world.rename_ship(fleet, index, &name),
                None => world.rename_fleet(fleet, &name),
            };
            macroquad::logging::info!(
                "[interface] command=0x203 destination=rename status={} object={} name={name}",
                if renamed { "applied" } else { "refused" },
                if ship.is_some() { "ship" } else { "fleet" }
            );
        }
        PanelAction::CreateFleet {
            fleet,
            ships,
            roster,
        } => {
            let result = rebellion_core::fleet_join::create_fleet(
                world,
                movement_state,
                troop_transport_state,
                fleet,
                &ships,
                roster,
                *player_faction == MissionFaction::Alliance,
            );
            if let Err(error) = result {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        format!("Create Fleet rejected: {error}"),
                        MessageCategory::Event,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        PanelAction::MoveShips {
            fleet,
            ships,
            roster,
            system,
        } => {
            // The ships' own Move: into the side's bit-1 spare at the
            // system (FUN_00552300 -> FUN_005097d0), on arrival elsewhere.
            let result = rebellion_core::fleet_join::move_ships_into_spare(
                world,
                movement_state,
                troop_transport_state,
                fleet,
                &ships,
                roster,
                system,
                *player_faction == MissionFaction::Alliance,
            );
            if let Err(error) = result {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        format!("Fleet move rejected: {error}"),
                        MessageCategory::Event,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        PanelAction::DispatchFleet {
            fleet,
            destination,
            troops,
        } => {
            let expected_is_alliance = *player_faction == MissionFaction::Alliance;
            // The shared order (rebellion_data::player_command), as a
            // replay applies it.
            match rebellion_data::player_command::dispatch_fleet(
                world,
                movement_state,
                troop_transport_state,
                fleet,
                destination,
                &troops,
                expected_is_alliance,
            ) {
                Ok(departure) => {
                    let origin_name = world
                        .systems
                        .get(departure.origin)
                        .map_or("Unknown", |system| system.name.as_str());
                    let destination_name = world
                        .systems
                        .get(departure.destination)
                        .map_or("Unknown", |system| system.name.as_str());
                    msg_log.push(GameMessage::at_system(
                        clock.tick,
                        format!(
                            "Fleet departed {} for {} ({} days)",
                            origin_name, destination_name, departure.transit_ticks,
                        ),
                        MessageCategory::Event,
                        departure.destination,
                    ));
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        audio_engine.play_sfx(SfxKind::FleetDeparture, audio_vol);
                    }
                }
                Err(refusal) => {
                    let text = match refusal {
                        rebellion_data::player_command::DispatchRefusal::Move(error) => {
                            format!("Fleet move rejected: {error}")
                        }
                        rebellion_data::player_command::DispatchRefusal::Embark(error) => {
                            format!("Troop embarkation rejected: {error}")
                        }
                    };
                    msg_log.refuse(
                        GameMessage::new(clock.tick, text, MessageCategory::Event),
                        UNTRACED_REFUSAL,
                    );
                }
            }
        }
        PanelAction::ToggleReserved { system, area } => {
            let reserved = match area {
                Some(area) => {
                    let reserved = !mfg_state.is_reserved(system, area);
                    mfg_state.set_reserved(system, area, reserved);
                    reserved
                }
                None => mfg_state.toggle_system_reserved(system),
            };
            macroquad::logging::info!(
                "[interface] command=0x216 destination=production_reserved status=set reserved={reserved} area={area:?} system={}",
                world.systems.get(system).map_or("", |value| value.name.as_str())
            );
        }
        PanelAction::StopProduction { system, area } => {
            mfg_state.stop(system, area);
            macroquad::logging::info!(
                "[interface] command=0x213 destination=production_stop status=stopped area={area:?} system={}",
                world.systems.get(system).map_or("", |value| value.name.as_str())
            );
        }
        // FUN_00438980 -> FUN_0041ce20: the units replace what the band
        // was building, each at its class cost in build points and named
        // at the order (build_selection::confirm_order).
        PanelAction::BuildProduction {
            system,
            area,
            kind,
            count,
        } => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            if rebellion_core::build_selection::confirm_order(
                world,
                mfg_state,
                system,
                area,
                kind,
                count,
                is_alliance,
            ) {
                macroquad::logging::info!(
                    "[interface] command=0x21x destination=production_build status=queued area={area:?} count={count} system={}",
                    world.systems.get(system).map_or("", |value| value.name.as_str())
                );
            }
        }
        PanelAction::DispatchMission {
            kind,
            faction,
            team,
            decoys,
            target,
            target_character,
            target_object,
            tick,
        } => {
            // The first agent names the order in its messages.
            let char_name = team
                .iter()
                .find_map(|member| member.character())
                .and_then(|key| world.characters.get(key))
                .map_or_else(|| "Your team".into(), |c| c.name.clone());
            let request = rebellion_core::missions::MissionRequest {
                kind,
                faction,
                team,
                decoys,
                target_system: target,
                target_character,
                target_object,
                tick,
            };
            let has_target = request.target_character.is_some() || request.target_object.is_some();
            let has_decoys = !request.decoys.is_empty();
            if let Err(refused) = mission_state.dispatch_guarded(request, world) {
                // FUN_00487c90 shows no text; the side's advisor schedules a
                // reaction (`mission-dialog.md`, "Refusal"). The line is the
                // port's, for a game without the advisor's messages.
                let (status, decoys_without_target) =
                    refusal_status(refused, has_target, has_decoys);
                let text = match refused {
                    rebellion_core::missions::MissionRefusal::EmptyTeam => {
                        format!("{char_name} is a prisoner and cannot lead a mission")
                    }
                    rebellion_core::missions::MissionRefusal::MemberUnavailable(_) => {
                        format!("{char_name} cannot join this mission")
                    }
                    rebellion_core::missions::MissionRefusal::TargetUnavailable => {
                        "This target cannot be sabotaged".to_string()
                    }
                    rebellion_core::missions::MissionRefusal::MembersNotAllowed => {
                        format!("{char_name} cannot undertake this mission")
                    }
                };
                *refusal = Some(RefusedOrder {
                    status,
                    decoys_without_target,
                    text,
                });
                return;
            }
            let sys_name = world
                .systems
                .get(target)
                .map_or_else(|| "unknown".into(), |s| s.name.clone());
            let kind_name = match kind {
                MissionKind::Diplomacy => "Diplomacy",
                MissionKind::Recruitment => "Recruitment",
                MissionKind::Sabotage => "Sabotage",
                MissionKind::Assassination => "Assassination",
                MissionKind::Espionage => "Espionage",
                MissionKind::Rescue => "Rescue",
                MissionKind::Abduction => "Abduction",
                MissionKind::InciteUprising => "Incite Uprising",
                MissionKind::SubdueUprising => "Subdue Uprising",
                MissionKind::DeathStarSabotage => "Death Star Sabotage",
                MissionKind::Reconnaissance => "Reconnaissance",
                MissionKind::ShipDesignResearch => "Ship Design Research",
                MissionKind::FacilityDesignResearch => "Facility Design Research",
                MissionKind::TroopTrainingResearch => "Troop Training Research",
                MissionKind::Autoscrap => "Autoscrap",
                MissionKind::Move => "Move",
                MissionKind::Return => "Return",
                MissionKind::Adrift => "Adrift",
                MissionKind::JediTraining => "Jedi Training",
                MissionKind::Dagobah => "Dagobah",
                MissionKind::Bounty => "Bounty",
                MissionKind::Palace => "Palace",
                MissionKind::Pickup => "Pickup",
            };
            msg_log.push(GameMessage::at_system(
                clock.tick,
                format!("{char_name} dispatched on {kind_name} mission to {sys_name}"),
                MessageCategory::Mission,
                target,
            ));
        }
        // Save/load actions are handled by the caller before dispatching here;
        // they require access to the full save state and are not routed through
        // this helper.
        PanelAction::SelectFaction(_)
        | PanelAction::SaveGame { .. }
        | PanelAction::LoadGame { .. }
        | PanelAction::OpenModManager => {
            // Handled by UI state toggle (not a world mutation)
        }
        PanelAction::ToggleMod { ref name } => {
            mod_runtime.toggle_mod(name);
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Toggled mod: {name}"),
                MessageCategory::Event,
            ));
        }
        PanelAction::ReloadMods => {
            mod_runtime.refresh();
            let mod_errors = mod_runtime.apply_enabled(world);
            for err in &mod_errors {
                eprintln!("Mod reload error: {err:?}");
            }
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Reloaded {} mods", mod_runtime.discovered.len()),
                MessageCategory::Event,
            ));
        }
        // FUN_00486fb0 -> FUN_00487740: the order validates (+0x1c), then
        // executes, or FUN_00487c90 reports the refusal. port: the draws
        // come from the day.
        PanelAction::OrderBombardment {
            fleets,
            system,
            kind,
        } => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            if rebellion_core::bombardment::order_enabled(
                world,
                movement_state,
                is_alliance,
                &fleets,
                system,
                kind,
            ) {
                let mut bombard = BombardEnv::new(clock.tick, Some(victory_state.alliance_hq));
                let outcome = bombard
                    .bombard_reported(
                        world,
                        troop_transport_state,
                        Faction::of_alliance(is_alliance),
                        &fleets,
                        system,
                        kind,
                    )
                    .clone();
                for (system, losses) in &bombard.losses {
                    blockade_state.record_losses(*system, *losses);
                }
                log_bombardment(world, system, &outcome, clock.tick, msg_log);
                present_assault_reports(
                    std::mem::take(&mut bombard.reports),
                    world,
                    *player_faction,
                    message_texts,
                    assault_summaries,
                    msg_log,
                    clock.tick,
                );
                if outcome.system_destroyed {
                    destroy_planet(
                        world,
                        system,
                        &outcome.killed,
                        mfg_state,
                        blockade_state,
                        msg_log,
                        clock.tick,
                        message_texts,
                    );
                }
            } else {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        "Planetary bombardment refused".to_string(),
                        MessageCategory::Combat,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        // FUN_00486fb0 -> FUN_00487740: the Move order's 0x241 validates
        // (FUN_004f4b60: a target of another side is 1/0x28), then its
        // command 0x250 creates the Move mission (FUN_0054d360), or
        // FUN_00487c90 reports the refusal.
        PanelAction::MoveCharacter {
            character,
            system,
            fleet,
        } => {
            let result = if rebellion_core::missions::move_place_allowed(
                world,
                *player_faction,
                system,
                fleet,
            ) {
                mission_state
                    .dispatch_guarded(
                        rebellion_core::missions::MissionRequest::single(
                            MissionKind::Move,
                            *player_faction,
                            character,
                            system,
                            None,
                            clock.tick,
                        ),
                        world,
                    )
                    .inspect(|&id| {
                        if let Some(fleet) = fleet {
                            mission_state.set_target_fleet(id, fleet);
                        }
                    })
                    .map_err(|refusal| format!("{refusal:?}"))
            } else {
                Err("1/0x28".to_string())
            };
            macroquad::logging::info!(
                "[interface] character_move character={:?} to={:?} fleet={:?} result={:?}",
                world.characters.get(character).map(|c| c.name.as_str()),
                world.systems.get(system).map(|s| s.name.as_str()),
                fleet.and_then(|key| fleet_label(world, key)),
                result,
            );
            if result.is_err() {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        "Move refused".to_string(),
                        MessageCategory::Event,
                    ),
                    if result.as_ref().err().map(String::as_str) == Some("1/0x28") {
                        OTHER_SIDE_REFUSAL
                    } else {
                        UNTRACED_REFUSAL
                    },
                );
            }
        }
        // A special force's Move: the Move mission 0x41000001 with the unit
        // as its team, as a character's.
        PanelAction::MoveProducts {
            products,
            system,
            fleet,
        } => {
            for product in products {
                let moved =
                    rebellion_core::products::move_to_target(
                        world,
                        product,
                        system,
                        fleet,
                        fleet.map_or(0, |fleet| troop_transport_state.carried_count(fleet)),
                    );
                macroquad::logging::info!(
                    "[interface] command=0x201 product={product} moved={moved}"
                );
            }
        }
        PanelAction::MoveSpecialForce { unit, system, fleet } => {
            let result = if rebellion_core::missions::move_place_allowed(
                world,
                *player_faction,
                system,
                fleet,
            ) {
                mission_state
                    .dispatch_guarded(
                        rebellion_core::missions::MissionRequest {
                            kind: MissionKind::Move,
                            faction: *player_faction,
                            team: vec![MissionMember::SpecialForce(unit)],
                            decoys: Vec::new(),
                            target_system: system,
                            target_character: None,
                            target_object: None,
                            tick: clock.tick,
                        },
                        world,
                    )
                    .inspect(|&id| {
                        if let Some(fleet) = fleet {
                            mission_state.set_target_fleet(id, fleet);
                        }
                    })
                    .map_err(|refusal| format!("{refusal:?}"))
            } else {
                Err("1/0x28".to_string())
            };
            macroquad::logging::info!(
                "[interface] special_force_move unit={unit:?} to={:?} fleet={:?} result={result:?}",
                world.systems.get(system).map(|s| s.name.as_str()),
                fleet.and_then(|key| fleet_label(world, key)),
            );
            if result.is_err() {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        "Move refused".to_string(),
                        MessageCategory::Event,
                    ),
                    if result.as_ref().err().map(String::as_str) == Some("1/0x28") {
                        OTHER_SIDE_REFUSAL
                    } else {
                        UNTRACED_REFUSAL
                    },
                );
            }
        }
        // A Move's one 0x241 for several members (FUN_0053c810): the Move
        // mission 0x41000001 with them all as its team.
        PanelAction::MoveMembers {
            members,
            system,
            fleet,
        } => {
            let result = if rebellion_core::missions::move_place_allowed(
                world,
                *player_faction,
                system,
                fleet,
            ) {
                mission_state
                    .dispatch_guarded(
                        rebellion_core::missions::MissionRequest {
                            kind: MissionKind::Move,
                            faction: *player_faction,
                            team: members.clone(),
                            decoys: Vec::new(),
                            target_system: system,
                            target_character: None,
                            target_object: None,
                            tick: clock.tick,
                        },
                        world,
                    )
                    .inspect(|&id| {
                        if let Some(fleet) = fleet {
                            mission_state.set_target_fleet(id, fleet);
                        }
                    })
                    .map_err(|refusal| format!("{refusal:?}"))
            } else {
                Err("1/0x28".to_string())
            };
            macroquad::logging::info!(
                "[interface] members_move members={} to={:?} fleet={:?} result={result:?}",
                members.len(),
                world.systems.get(system).map(|s| s.name.as_str()),
                fleet.and_then(|key| fleet_label(world, key)),
            );
            if result.is_err() {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        "Move refused".to_string(),
                        MessageCategory::Event,
                    ),
                    if result.as_ref().err().map(String::as_str) == Some("1/0x28") {
                        OTHER_SIDE_REFUSAL
                    } else {
                        UNTRACED_REFUSAL
                    },
                );
            }
        }
        // FUN_00486fb0 -> FUN_00487740: the Command order validates
        // (FUN_004ed750), then takes effect (FUN_00561bb0), or FUN_00487c90
        // reports the refusal.
        PanelAction::OrderCommand { character, order } => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            let Some(kind) = rebellion_core::character_command::CommandKind::of_order(order)
            else {
                return;
            };
            let en_route = mission_state
                .is_en_route(rebellion_core::missions::MissionMember::Character(character));
            let result = rebellion_core::character_command::give(
                world,
                character,
                kind,
                is_alliance,
                en_route,
            );
            macroquad::logging::info!(
                "[interface] command={order:#x} character={:?} result={:?} now={:?}",
                world.characters.get(character).map(|c| c.name.as_str()),
                result.as_ref().map(Vec::len),
                world.characters.get(character).map(|c| c.command),
            );
            if result.is_err() {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        "Command refused".to_string(),
                        MessageCategory::Event,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        // FUN_0041ce20(order, 1): the confirmed Retire validates again
        // (FUN_004ed460) and fires event 0x230 (FUN_00540f40 ->
        // FUN_00534c00), or FUN_00487c90 reports the refusal.
        PanelAction::Retire(member) => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            let en_route = mission_state.is_en_route(member);
            let result = match member {
                MissionMember::Character(key) => {
                    rebellion_core::character_retire::retire(world, key, is_alliance, en_route)
                }
                MissionMember::SpecialForce(key) => {
                    rebellion_core::character_retire::retire_special_force(
                        world,
                        key,
                        is_alliance,
                        en_route,
                    )
                }
            };
            macroquad::logging::info!(
                "[interface] command=0x242 destination=retire member={:?} result={:?}",
                member,
                result
            );
            if result.is_err() {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        "Retire refused".to_string(),
                        MessageCategory::Event,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        // FUN_00486fb0 -> FUN_00487740 -> FUN_00563390: the order
        // validates (+0x1c), then executes (FUN_0058c120), or FUN_00487c90
        // reports the refusal. port: the draws come from the day.
        PanelAction::OrderAssault { fleets, system } => {
            let is_alliance = *player_faction == MissionFaction::Alliance;
            if rebellion_core::assault::order_enabled(
                world,
                movement_state,
                troop_transport_state,
                is_alliance,
                &fleets,
                system,
            ) {
                let mut env = BombardEnv::new(clock.tick, Some(victory_state.alliance_hq));
                let outcome = env
                    .assault(
                        world,
                        troop_transport_state,
                        Faction::of_alliance(is_alliance),
                        &fleets,
                        system,
                    )
                    .clone();
                for (system, losses) in &env.losses {
                    blockade_state.record_losses(*system, *losses);
                }
                log_assault(world, system, &outcome, clock.tick, msg_log);
                // The landed regiments take the system in the assault's own
                // pass (FUN_00508660 -> FUN_0050b610 -> FUN_00510d70, event
                // 0x31b -> FUN_00517aa0 -> FUN_005180e0 for Coruscant), so
                // the victory word settles on the assault's day (A0
                // 2026-11-17-edited-saves-5 run 2: day 94 throughout).
                tactical_flow::occupy_system(world, system, clock.tick, msg_log);
                macroquad::logging::info!(
                    "[assault] system={} holder={:?} day={}",
                    world.systems[system].name,
                    world.systems[system].control,
                    clock.tick
                );
                present_assault_reports(
                    std::mem::take(&mut env.reports),
                    world,
                    *player_faction,
                    message_texts,
                    assault_summaries,
                    msg_log,
                    clock.tick,
                );
            } else {
                msg_log.refuse(
                    GameMessage::new(
                        clock.tick,
                        "Planetary assault refused".to_string(),
                        MessageCategory::Combat,
                    ),
                    UNTRACED_REFUSAL,
                );
            }
        }
        PanelAction::AdvanceTicks(n) => {
            // The clock owes N days; the frame loop runs each through the
            // same simulation step as a day of normal play.
            clock.queue_days(n);
        }
        // The caller routes speed through the clock's pause stop.
        PanelAction::SetGameSpeed(_) => {}
        PanelAction::ToggleDualAI => {
            *dual_ai_mode = !*dual_ai_mode;
            if *dual_ai_mode {
                // Create persistent second AI for the opposite faction
                let second_faction = match ai_state.faction {
                    Some(AiFaction::Empire) => AiFaction::Alliance,
                    _ => AiFaction::Empire,
                };
                *secondary_ai_state = Some(AIState::new(second_faction));
            } else {
                *secondary_ai_state = None;
            }
            let state_str = if *dual_ai_mode { "ENABLED" } else { "DISABLED" };
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Dual AI mode {state_str}"),
                MessageCategory::Event,
            ));
        }
        PanelAction::ForceVictoryCheck => {
            let tick_ev = rebellion_core::tick::TickEvent { tick: clock.tick };
            let result = rebellion_core::victory::VictorySystem::check(
                victory_state,
                world,
                &[tick_ev],
                campaign_config.victory_conditions,
            );
            if let Some(outcome) = result {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("Victory check: {outcome:?}"),
                    MessageCategory::Event,
                ));
            } else {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "Victory check: no winner yet".to_string(),
                    MessageCategory::Event,
                ));
            }
        }
        PanelAction::RevealAllFog => {
            // Reveal all systems in fog state
            for (sys_key, _) in &world.systems {
                fog_state.reveal(sys_key);
            }
        }
        PanelAction::ToggleSeeEverything => {
            let views = &mut world.object_views;
            views.see_everything = !views.see_everything;
            msg_log.push(GameMessage::new(
                clock.tick,
                format!(
                    "See everything (debug): {}",
                    if views.see_everything { "on" } else { "off" }
                ),
                MessageCategory::Event,
            ));
        }
        PanelAction::ExportGameLog => {
            // Resolve system keys to names before export
            msg_log.resolve_system_names(|key| world.systems.get(key).map(|s| s.name.clone()));
            let path = std::path::PathBuf::from("game_log.jsonl");
            match msg_log.export_jsonl(&path) {
                Ok(()) => {
                    msg_log.push(GameMessage::new(
                        clock.tick,
                        format!("Exported game log to {}", path.display()),
                        MessageCategory::Event,
                    ));
                }
                Err(e) => {
                    eprintln!("Failed to export game log: {e}");
                }
            }
        }
        PanelAction::ShowGameStats => {
            let alliance_systems = world
                .systems
                .values()
                .filter(|s| s.control.is_controlled_by(Faction::Alliance))
                .count();
            let empire_systems = world
                .systems
                .values()
                .filter(|s| s.control.is_controlled_by(Faction::Empire))
                .count();
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Stats: tick {}, Alliance {} systems, Empire {} systems, {} fleets, {} characters",
                    clock.tick, alliance_systems, empire_systems, world.fleets.len(), world.characters.len()),
                MessageCategory::Event,
            ));
        }
        PanelAction::ListActiveMissions => {
            let missions = mission_state.missions();
            if missions.is_empty() {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    "No active missions".to_string(),
                    MessageCategory::Mission,
                ));
            } else {
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("{} active missions:", missions.len()),
                    MessageCategory::Mission,
                ));
                for m in missions {
                    let char_name = m
                        .lead_character()
                        .and_then(|key| world.characters.get(key))
                        .map_or_else(|| "Unknown".into(), |c| c.name.clone());
                    let sys_name = world
                        .systems
                        .get(m.target_system)
                        .map_or_else(|| "unknown".into(), |s| s.name.clone());
                    msg_log.push(GameMessage::new(
                        clock.tick,
                        format!("  {:?} — {} at {}", m.kind, char_name, sys_name),
                        MessageCategory::Mission,
                    ));
                }
            }
        }
        PanelAction::ListActiveFleets => {
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("{} fleets:", world.fleets.len()),
                MessageCategory::Event,
            ));
            for (_, fleet) in &world.fleets {
                let sys_name = world
                    .systems
                    .get(fleet.location)
                    .map_or_else(|| "unknown".into(), |s| s.name.clone());
                let faction = if fleet.is_alliance {
                    "Alliance"
                } else {
                    "Empire"
                };
                let ship_count = fleet.ship_count() as usize
                    + fleet
                        .fighters
                        .iter()
                        .map(|e| e.count as usize)
                        .sum::<usize>();
                msg_log.push(GameMessage::new(
                    clock.tick,
                    format!("  {faction} fleet at {sys_name} — {ship_count} ships"),
                    MessageCategory::Event,
                ));
            }
        }
        PanelAction::ShowEventCount => {
            let total = event_state.events().len();
            let fired = event_state
                .events()
                .iter()
                .filter(|e| event_state.has_fired(e.id))
                .count();
            msg_log.push(GameMessage::new(
                clock.tick,
                format!("Events: {total} defined, {fired} fired"),
                MessageCategory::Event,
            ));
        }

    }
}
