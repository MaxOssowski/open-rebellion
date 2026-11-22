//! Move orders (fleets, teams, characters, ships, troops, squadrons,
//! special forces, headquarters), their gates and refusals, moved out of
//! `main.rs`. The code reads `main.rs`'s items through `super`.

use super::*;

/// What a fleet move needs to read when it lands.
pub(super) struct FleetMoveContext<'a> {
    pub(super) world: &'a GameWorld,
    pub(super) movement: &'a MovementState,
    pub(super) blockaded: &'a HashSet<SystemKey>,
    pub(super) faction: MissionFaction,
    pub(super) tick: u64,
    /// The team is a sector window fleet icon's (`FUN_00512700` kind
    /// `0x10`): the order's first team id is the system.
    pub(super) icon: bool,
}

/// What a defenses icon's Move reads.
pub(super) struct DefensesMoveContext<'a> {
    pub(super) world: &'a GameWorld,
    pub(super) movement: &'a MovementState,
    pub(super) missions: &'a MissionState,
    pub(super) transport: &'a TroopTransportState,
    /// Products on their way (`FUN_005150c0`): inside the destination's
    /// container, so in the defenses icon's team.
    pub(super) deliveries: &'a DeliveryState,
    pub(super) blockaded: &'a HashSet<SystemKey>,
    pub(super) faction: MissionFaction,
    pub(super) tick: u64,
}

/// `FUN_0053c100` for a defenses icon's Move and Confirmed Move at
/// `system`: `FUN_0053c4b0` refuses an empty team (`1`/`0x16`), then each
/// sub-order's `+0x18` runs. The `0x241` of the characters and special
/// forces checks each member (`MissionState::move_order_enabled`); the
/// `0x204` group of the regiments and squadrons checks each member's move
/// command (`FUN_00578c00`: `regiment_move_enabled`, `squadrons::move_enabled`).
pub(super) fn defenses_move_enabled(
    world: &GameWorld,
    missions: &MissionState,
    transport: &TroopTransportState,
    deliveries: &DeliveryState,
    faction: MissionFaction,
    system: SystemKey,
) -> bool {
    let is_alliance = faction == MissionFaction::Alliance;
    let team = rebellion_render::quadrant_icons::defenses_move_team(
        world,
        missions,
        Faction::from(faction),
        system,
    );
    !team.is_empty()
        // A regiment or squadron on its way in is in the team, and its own
        // move command refuses while it travels; a product on its way from
        // its facility too (`DeliveryState::surface_arrival_pending`).
        && !transport.surface_arrival_pending(world, system, is_alliance)
        && !deliveries.surface_arrival_pending(system, Faction::from(faction))
        && !rebellion_core::products::surface_arrival_pending(world, system, is_alliance)
        && (team.members.is_empty() || missions.move_order_enabled(world, faction, &team.members))
        && team
            .troops
            .iter()
            .all(|&troop| transport.regiment_move_enabled(world, troop, is_alliance))
        && team.squadrons.iter().all(|&class| {
            rebellion_core::squadrons::move_enabled(
                world,
                is_alliance,
                class,
                SquadronPlace::Planet(system),
            )
        })
}

/// A defenses icon's Move (`0x201`) or Confirmed Move (`0x202`) with its
/// target set, from the menu's targeting or a drop (`FUN_00422ce0`).
///
/// The order resolves its team, the icon's kind 8 objects of the side
/// (`FUN_00512700`), and splits it (`FUN_0053c4b0`): one `0x241` for the
/// characters and special forces (`FUN_0053c810`), one Move mission with
/// them all, then one `0x204` group for the regiments and squadrons
/// (`FUN_0053ca50`). The validator `FUN_0053c1a0` runs every sub-order's, so
/// one refusal refuses the whole order (`FUN_00487740` -> `FUN_00487c90`).
/// The `0x241`'s: its `+0x18` again (`FUN_0051ff30`), the target's side
/// (`FUN_004f4b60`, `1`/`0x28`). The `0x204` group's: `FUN_0053d430`'s
/// `1`/`0x28` rule and each member's command; port: each regiment and
/// squadron class runs its own move check, which holds the group rule for
/// a group of one kind, and a mixed group meets the squadrons' refusal of
/// another side's planet first.
///
/// `FUN_00487cc0` asks for a Confirmed Move. A Move asks only when the
/// first team id's container is a blockaded system of its side; the
/// icon's entry is the system, whose container is a sector, so a Move
/// never asks (read from the image at `0x00487cc0..0x00487e13`). For the
/// same reason `FUN_0049a350` gives no evacuation warning. The window lists
/// a line per member, the `0x241`'s first (`FUN_0053c2e0`).
pub(super) fn issue_defenses_move(
    context: &DefensesMoveContext<'_>,
    system: SystemKey,
    confirmed: bool,
    target: ReleaseTarget,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
    panel_actions: &mut Vec<PanelAction>,
) {
    let team = rebellion_render::quadrant_icons::defenses_move_team(
        context.world,
        context.missions,
        Faction::from(context.faction),
        system,
    );
    issue_team_move(
        context,
        &team,
        (system, SquadronPlace::Planet(system)),
        true,
        confirmed,
        target,
        msg_log,
        move_confirmation_state,
        panel_actions,
    );
}

/// The order a Fleet window drop issues (`FUN_00422ce0`, window type 4):
/// the source window's whole selection (slot `+0x58`) as one `0x201`, or
/// `0x202` with Ctrl down. Fleets move together (`issue_fleet_move`), ships
/// as one ship order, and the right list's regiments, squadrons, characters
/// and special forces as one team move that `FUN_0053c4b0` splits, so one
/// refusal refuses the whole order (`issue_team_move`).
#[derive(Debug, PartialEq)]
pub(super) enum FleetDropOrder {
    Fleets(Vec<FleetKey>),
    Ships(ShipTeam),
    /// Unfinished units (`rebellion_core::products`).
    Products(Vec<u64>),
    Team {
        /// The squadrons' fleet and its system; `None` for a team without
        /// squadrons, whose move reads no place.
        place: Option<(SystemKey, SquadronPlace)>,
        team: rebellion_render::quadrant_icons::DefensesTeam,
    },
}

/// A Fleet window drop's order, its release point and whether it is
/// Confirmed Move; `None` for another action or a squadron of a fleet that
/// is gone.
pub(super) fn fleet_drop_order(
    world: &GameWorld,
    action: FleetWindowAction,
) -> Option<(FleetDropOrder, egui_macroquad::egui::Pos2, bool)> {
    use rebellion_render::quadrant_icons::DefensesTeam;
    match action {
        FleetWindowAction::DragFleet {
            fleets,
            point,
            confirmed,
        } => Some((FleetDropOrder::Fleets(fleets), point, confirmed)),
        FleetWindowAction::DragProducts {
            products,
            point,
            confirmed,
        } => Some((FleetDropOrder::Products(products), point, confirmed)),
        FleetWindowAction::DragShip {
            fleet,
            ships,
            roster,
            point,
            confirmed,
        } => Some((
            FleetDropOrder::Ships(ShipTeam {
                fleet,
                ships,
                roster,
            }),
            point,
            confirmed,
        )),
        FleetWindowAction::DragRegiment {
            troops,
            point,
            confirmed,
        } => Some((
            FleetDropOrder::Team {
                place: None,
                team: DefensesTeam {
                    troops,
                    ..DefensesTeam::default()
                },
            },
            point,
            confirmed,
        )),
        FleetWindowAction::DragMembers {
            members,
            point,
            confirmed,
        } => Some((
            FleetDropOrder::Team {
                place: None,
                team: DefensesTeam {
                    members,
                    ..DefensesTeam::default()
                },
            },
            point,
            confirmed,
        )),
        FleetWindowAction::DragSquadron {
            fleet,
            indices,
            point,
            confirmed,
        } => {
            let value = world.fleets.get(fleet)?;
            let squadrons = indices
                .iter()
                .filter_map(|&index| value.fighters.get(index).map(|entry| entry.class))
                .collect();
            Some((
                FleetDropOrder::Team {
                    place: Some((value.location, SquadronPlace::Fleet(fleet))),
                    team: DefensesTeam {
                        squadrons,
                        ..DefensesTeam::default()
                    },
                },
                point,
                confirmed,
            ))
        }
        _ => None,
    }
}

/// A ships team's Move (`0x201`) with its target: onto a fleet it joins
/// it, onto a system the ships go there (`fleet-join-split.md`).
pub(super) fn ship_move_action(team: ShipTeam, target: ReleaseTarget) -> PanelAction {
    let ShipTeam {
        fleet,
        ships,
        roster,
    } = team;
    match target {
        ReleaseTarget::Fleet { fleet: target, .. } => PanelAction::JoinFleet {
            mover: FleetMover::Ships {
                fleet,
                ships,
                roster,
            },
            target,
        },
        ReleaseTarget::System(system) => PanelAction::MoveShips {
            fleet,
            ships,
            roster,
            system,
        },
    }
}

/// What a menu for several objects issues.
pub(super) enum TeamMenuOrder {
    Target(TargetOrder),
    Panel(PanelAction),
}

/// The order `command` builds for a window selection's `team`
/// (`FUN_00487c50`: the selection is the order's team). Ships move or make
/// a fleet together (manual p. 120: "select multiple ships by holding down
/// the CTRL key while clicking and then right-click on one of the selected
/// ships and select Create Fleet"); characters and special forces go on
/// one mission; anything else moves as one `0x201`, split as a defenses
/// icon's (`FUN_0053c4b0`).
pub(super) fn team_menu_order(
    world: &GameWorld,
    command: ObjectMenuCommand,
    team: &[MenuObject],
    place: Option<(SystemKey, SquadronPlace)>,
) -> Option<TeamMenuOrder> {
    let confirmed = command == ObjectMenuCommand::ConfirmedMove;
    let ships = || {
        let MenuObject::Ship { fleet, roster, .. } = *team.first()? else {
            return None;
        };
        let ships: Option<Vec<usize>> = team
            .iter()
            .map(|object| match *object {
                MenuObject::Ship {
                    fleet: of, index, ..
                } if of == fleet => Some(index),
                _ => None,
            })
            .collect();
        Some((fleet, ships?, roster))
    };
    // The left list's fleets move as one team of fleets (FUN_00422ce0's
    // 0x201 with the selection, FUN_0053c1a0 running each fleet's check).
    let fleets: Option<Vec<FleetKey>> = team
        .iter()
        .map(|object| match *object {
            MenuObject::Fleet(fleet) => Some(fleet),
            _ => None,
        })
        .collect();
    if let (Some(fleets), ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove) =
        (fleets, command)
    {
        return Some(TeamMenuOrder::Target(TargetOrder::FleetMove {
            fleets,
            confirmed,
            icon: false,
        }));
    }
    match command {
        ObjectMenuCommand::Mission => {
            let members: Option<Vec<MissionMember>> =
                team.iter().map(|object| object.mission_member()).collect();
            Some(TeamMenuOrder::Target(TargetOrder::Mission(members?)))
        }
        // FUN_00580b00: Create Fleet runs at once, against the ships' own
        // system.
        ObjectMenuCommand::CreateFleet => {
            let (fleet, ships, roster) = ships()?;
            Some(TeamMenuOrder::Panel(PanelAction::CreateFleet {
                fleet,
                ships,
                roster,
            }))
        }
        ObjectMenuCommand::Move | ObjectMenuCommand::ConfirmedMove => {
            if let Some((fleet, ships, roster)) = ships() {
                return Some(TeamMenuOrder::Target(TargetOrder::ShipMove {
                    fleet,
                    ships,
                    roster,
                    confirmed,
                }));
            }
            let (system, from) = place?;
            let mut moving = rebellion_render::quadrant_icons::DefensesTeam::default();
            for object in team {
                match *object {
                    MenuObject::Character(key) => {
                        moving.members.push(MissionMember::Character(key));
                    }
                    MenuObject::SpecialForce(key) => {
                        moving.members.push(MissionMember::SpecialForce(key));
                    }
                    MenuObject::Troop(key) => moving.troops.push(key),
                    MenuObject::PlanetFighter { class, .. } => moving.squadrons.push(class),
                    MenuObject::Fighter { fleet, index } => moving
                        .squadrons
                        .push(world.fleets.get(fleet)?.fighters.get(index)?.class),
                    _ => return None,
                }
            }
            Some(TeamMenuOrder::Target(TargetOrder::TeamMove {
                system,
                from,
                team: moving,
                confirmed,
            }))
        }
        _ => None,
    }
}

/// The confirmation (`FUN_0049a350`) an unfinished unit's Move opens: one
/// line per unit, its name and its days. hyp: the days are 0, the unit
/// having no trip (`FUN_00556390` moves it at once).
pub(super) fn product_confirmation(
    world: &GameWorld,
    faction: MissionFaction,
    products: Vec<u64>,
    (destination, join): (SystemKey, Option<rebellion_core::ids::FleetKey>),
    blockaded: bool,
) -> MoveConfirmation {
    let lines = products
        .iter()
        .filter_map(|&id| world.products.get(id))
        .map(|product| {
            let name = product.name.clone().or_else(|| match product.kind {
                rebellion_core::manufacturing::BuildableKind::CapitalShip(class) => world
                    .capital_ship_classes
                    .get(class)
                    .map(|value| value.name.clone()),
                rebellion_core::manufacturing::BuildableKind::Fighter(class) => {
                    world.fighter_classes.get(class).map(|value| value.name.clone())
                }
                rebellion_core::manufacturing::BuildableKind::Troop(class)
                | rebellion_core::manufacturing::BuildableKind::SpecialForce(class) => world
                    .buildable_classes
                    .get(&class)
                    .map(|value| value.name.clone()),
                _ => None,
            });
            (name.unwrap_or_default(), 0)
        })
        .collect();
    MoveConfirmation {
        order: ConfirmedOrder::Move,
        faction,
        fleets: Vec::new(),
        characters: Vec::new(),
        special_forces: Vec::new(),
        troops: Vec::new(),
        squadrons: Vec::new(),
        ships: None,
        missions: Vec::new(),
        scrap: Vec::new(),
        blockaded,
        destination,
        join,
        products,
        lines,
    }
}

/// An unfinished regiment's or squadron's Move (`0x201`): the order's
/// `+0x18` asks the product's side (`FUN_004f9860`); its vt74 asks nothing
/// of Completed (A0 2026-11-15 "u-3": the unfinished TIE's Move white).
pub(super) fn product_move_enabled(world: &GameWorld, product: u64, is_alliance: bool) -> bool {
    // FUN_004f9860 refuses an object en route (a finished product
    // travelling to its container).
    world
        .products
        .get(product)
        .is_some_and(|value| value.is_alliance == is_alliance)
        && world.products.arrival(product).is_none()
}

/// Where a window selection's squadrons stand: a Fleet window squadron's
/// fleet, a Defenses window squadron's planet, else `system`'s planet.
pub(super) fn team_squadron_place(team: &[MenuObject], system: SystemKey) -> SquadronPlace {
    team.iter()
        .find_map(|object| match *object {
            MenuObject::Fighter { fleet, .. } => Some(SquadronPlace::Fleet(fleet)),
            MenuObject::PlanetFighter { system, .. } => Some(SquadronPlace::Planet(system)),
            _ => None,
        })
        .unwrap_or(SquadronPlace::Planet(system))
}

/// The gates of a menu for `team` (`FUN_0051d990`): an order is enabled
/// when the order with the whole team as its team is (`+0x18`, which runs
/// each member's own check). port: each member's own Move gate and
/// Mission gate stand in for the team order's.
///
/// `personnel_move`, when given, checks the team's characters and special
/// forces together, as their one `0x241` sub-order does (`FUN_0053cbc0`):
/// `FUN_0054bb90` lets a prisoner go only with a free member, its jailer,
/// so a prisoner's own gate alone would refuse the pair.
pub(super) fn team_gates(
    team: &[MenuObject],
    gates_of: impl Fn(MenuObject) -> OrderGates,
    personnel_move: Option<&dyn Fn(&[MissionMember]) -> bool>,
) -> OrderGates {
    let Some(&first) = team.first() else {
        return OrderGates::default();
    };
    let mut gates = gates_of(first);
    if team.len() > 1 {
        let each: Vec<(MenuObject, OrderGates)> = team
            .iter()
            .map(|&object| (object, gates_of(object)))
            .collect();
        let personnel: Vec<MissionMember> = team
            .iter()
            .filter_map(|object| object.mission_member())
            .collect();
        let together = match personnel_move {
            Some(check) if !personnel.is_empty() => Some(check(&personnel)),
            _ => None,
        };
        let moves = each.iter().all(|(object, gates)| match object {
            MenuObject::Ship { .. } => gates.ship_move,
            MenuObject::Troop(_) => gates.troop_move,
            MenuObject::Fighter { .. } | MenuObject::PlanetFighter { .. } => gates.squadron_move,
            MenuObject::Character(_) => together.unwrap_or(gates.character_move),
            MenuObject::SpecialForce(_) => together.unwrap_or(gates.special_force_move),
            MenuObject::Fleet(_) => gates.fleet_move,
            _ => false,
        });
        gates.ship_move = moves;
        gates.troop_move = moves;
        gates.squadron_move = moves;
        gates.character_move = moves;
        gates.special_force_move = moves;
        gates.fleet_move = moves;
        gates.mission = each.iter().all(|(_, gates)| gates.mission);
        // Scrap and Retire: one command per member, each its own check
        // (FUN_005201c0, FUN_00553770).
        gates.scrap = each.iter().all(|(_, gates)| gates.scrap);
        gates.retire = each.iter().all(|(_, gates)| gates.retire);
    }
    gates
}

/// The confirmation a menu's Scrap (`0x200`) or Retire (`0x242`) opens for
/// several objects, or the refusal: `FUN_00487740` validates the order,
/// whose validator runs each member's command (`FUN_0051ff30`), so one
/// refusal refuses all; then `FUN_00487cc0` always asks, and
/// `FUN_0049a350` lists each member's name under 0x7050 or 0x7051
/// (`FUN_0049a880`, 0x7054). `None` for another order.
pub(super) fn team_confirmation(
    world: &GameWorld,
    movement: &MovementState,
    missions: &MissionState,
    faction: MissionFaction,
    command: ObjectMenuCommand,
    team: &[MenuObject],
) -> Option<Result<MoveConfirmation, &'static str>> {
    let is_alliance = faction == MissionFaction::Alliance;
    let confirmation = |order, characters, special_forces, scrap, lines| MoveConfirmation {
        products: Vec::new(),
        order,
        faction,
        fleets: Vec::new(),
        characters,
        special_forces,
        missions: Vec::new(),
        scrap,
        troops: Vec::new(),
        squadrons: Vec::new(),
        ships: None,
        blockaded: false,
        destination: SystemKey::default(),
        join: None,
        lines,
    };
    match command {
        ObjectMenuCommand::Scrap => {
            let side = Faction::of_alliance(is_alliance);
            let mut targets: Vec<rebellion_core::scrap::ScrapTarget> = team
                .iter()
                .flat_map(|object| object.scrap_team(movement, world, side))
                .collect();
            // A ship leaves its fleet's list when scrapped, so a fleet's
            // ships go from the last (port: the order of the commands).
            targets.sort_by_key(|target| match *target {
                rebellion_core::scrap::ScrapTarget::Ship { index, .. } => {
                    std::cmp::Reverse(index)
                }
                _ => std::cmp::Reverse(0),
            });
            let allowed = !targets.is_empty()
                && targets.iter().all(|&target| {
                    rebellion_core::scrap::check_with_movement(world, movement, side, target)
                        .is_ok()
                });
            if !allowed {
                return Some(Err("Scrap refused"));
            }
            let lines = targets
                .iter()
                .map(|&target| {
                    (
                        rebellion_core::scrap::target_name(world, target).unwrap_or_default(),
                        0,
                    )
                })
                .collect();
            Some(Ok(confirmation(
                ConfirmedOrder::Scrap,
                Vec::new(),
                Vec::new(),
                targets,
                lines,
            )))
        }
        ObjectMenuCommand::Retire => {
            let mut characters = Vec::new();
            let mut special_forces = Vec::new();
            let mut lines = Vec::new();
            for object in team {
                let member = object.mission_member().ok_or("Retire refused");
                let Ok(member) = member else {
                    return Some(Err("Retire refused"));
                };
                let Some(Ok(name)) = retire_check(world, missions, member, is_alliance) else {
                    return Some(Err("Retire refused"));
                };
                match member {
                    MissionMember::Character(key) => characters.push(key),
                    MissionMember::SpecialForce(key) => special_forces.push(key),
                }
                lines.push((name, 0));
            }
            Some(Ok(confirmation(
                ConfirmedOrder::Retire,
                characters,
                special_forces,
                Vec::new(),
                lines,
            )))
        }
        _ => None,
    }
}

/// The team a Defenses window selection moves (`FUN_004a7a20`): its
/// characters and special forces, regiments and squadrons; `None` when it
/// holds a unit not yet completed, one in hyperspace or a defense facility,
/// whose own move refuses.
pub(super) fn defenses_selection_team(
    items: &[rebellion_render::defenses_window::DefensesItem],
) -> Option<rebellion_render::quadrant_icons::DefensesTeam> {
    use rebellion_render::defenses_window::DefensesItem;
    let mut team = rebellion_render::quadrant_icons::DefensesTeam::default();
    for item in items {
        match *item {
            DefensesItem::Character(key) => team.members.push(MissionMember::Character(key)),
            DefensesItem::SpecialForce(key) => {
                team.members.push(MissionMember::SpecialForce(key));
            }
            DefensesItem::Troop(key) => team.troops.push(key),
            DefensesItem::Squadron { class, .. } => team.squadrons.push(class),
            // A squadron in hyperspace takes no order (FUN_004f9860).
            DefensesItem::Defense(_)
            | DefensesItem::ViewedTroop { .. }
            | DefensesItem::Viewed { .. }
            | DefensesItem::Pending { .. }
            | DefensesItem::TravellingSquadron { .. } => return None,
        }
    }
    Some(team)
}

/// A Move (`0x201`) or Confirmed Move (`0x202`) of `team`, the objects of
/// `system`'s surface and personnel, with its target set: the defenses
/// icon's team (`icon`), or a Defenses window list's selection dropped on a
/// window (`FUN_00422ce0`, window type 10). Either splits the same way
/// (`FUN_0053c4b0`): one `0x241` for the characters and special forces, one
/// `0x204` group for the regiments and squadrons, and one refusal refuses
/// the whole order.
///
/// `FUN_00487cc0` asks for a Confirmed Move, and for a Move when the first
/// team member's container is a blockaded system (`+0x88` bit `0x20`) whose
/// side bits are the member's: a window selection on a planet of the
/// player's side under blockade. Its window then carries the evacuation
/// warning (`FUN_0049a350`, 0x7056), which a Confirmed Move from any
/// blockaded system carries too. The icon's first id is the system, whose
/// container is a sector, so neither applies to it; nor to members aboard
/// a fleet, whose container is their ship.
#[expect(
    clippy::too_many_arguments,
    reason = "The order's inputs at the UI boundary, as issue_defenses_move takes them."
)]
pub(super) fn issue_team_move(
    context: &DefensesMoveContext<'_>,
    team: &rebellion_render::quadrant_icons::DefensesTeam,
    (system, from): (SystemKey, SquadronPlace),
    icon: bool,
    confirmed: bool,
    target: ReleaseTarget,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
    panel_actions: &mut Vec<PanelAction>,
) {
    let world = context.world;
    let is_alliance = context.faction == MissionFaction::Alliance;
    let source = if icon { "defenses_icon" } else { "defenses_window" };
    let (destination, join, regiment_target) = match target {
        ReleaseTarget::System(to) => (to, None, RegimentTarget::System(to)),
        ReleaseTarget::Fleet { fleet, system: to } => {
            (to, Some(fleet), RegimentTarget::Fleet(fleet))
        }
    };
    let squadron_to = target.squadron_place();
    let member_days = if team.members.is_empty() {
        Some(Vec::new())
    } else if context
        .missions
        .move_order_enabled(world, context.faction, &team.members)
        && rebellion_core::missions::move_place_allowed(world, context.faction, destination, join)
    {
        rebellion_core::missions::team_move_days(world, &team.members, destination)
    } else {
        None
    };
    let troop_days: Result<Vec<u32>, _> = team
        .troops
        .iter()
        .map(|&troop| {
            context.transport.move_days(
                world,
                context.movement,
                context.blockaded,
                troop,
                regiment_target,
            )
        })
        .collect();
    // A regiment's refusal names its reason, as its own move's does.
    let refusal = troop_days.as_ref().err().map_or_else(
        || "Move refused".to_string(),
        |error| format!("Regiment move rejected: {error}"),
    );
    let troop_days = troop_days.ok();
    // Each class's squadrons together, so the room is checked for them all.
    let mut classes: Vec<(rebellion_core::ids::FighterKey, u32)> = Vec::new();
    for &class in &team.squadrons {
        match classes.iter_mut().find(|(known, _)| *known == class) {
            Some((_, count)) => *count += 1,
            None => classes.push((class, 1)),
        }
    }
    let class_days: Option<Vec<(rebellion_core::ids::FighterKey, u32)>> = classes
        .iter()
        .map(|&(class, count)| {
            context
                .transport
                .squadrons
                .move_days(world, is_alliance, class, count, from, squadron_to)
                .ok()
                .map(|days| (class, days))
        })
        .collect();
    // A regiment or squadron flying in is in the icon's team, and its own
    // move command refuses (`surface_arrival_pending`). A list selects only
    // what it lists.
    let arriving = icon
        && (context
            .transport
            .surface_arrival_pending(world, system, is_alliance)
            || context
                .deliveries
                .surface_arrival_pending(system, Faction::from(context.faction)));
    let (false, false, Some(member_days), Some(troop_days), Some(class_days)) =
        (team.is_empty(), arriving, member_days, troop_days, class_days)
    else {
        macroquad::logging::info!(
            "[interface] command={:#x} {source} status=refused",
            if confirmed { 0x202 } else { 0x201 }
        );
        // port: FUN_00487c90 plays the side's advisor reaction; this line
        // stands in.
        msg_log.refuse(
            GameMessage::new(context.tick, refusal, MessageCategory::Event),
            UNTRACED_REFUSAL,
        );
        return;
    };
    let (characters, special_forces): (Vec<MissionMember>, Vec<MissionMember>) = team
        .members
        .iter()
        .copied()
        .partition(|member| matches!(member, MissionMember::Character(_)));
    let container = (!icon)
        .then(|| team_container_system(world, team, from))
        .flatten();
    let blockaded = container.is_some_and(|system| context.blockaded.contains(&system));
    let own_side = container
        .and_then(|system| world.systems.get(system))
        .is_some_and(|value| value.control.is_controlled_by(Faction::from(context.faction)));
    if !confirmed && !(blockaded && own_side) {
        if !team.members.is_empty() {
            panel_actions.push(PanelAction::MoveMembers {
                members: team.members.clone(),
                system: destination,
                fleet: join,
            });
        }
        panel_actions.extend(team.troops.iter().map(|&troop| PanelAction::MoveRegiment {
            troop,
            target: regiment_target,
        }));
        panel_actions.extend(
            team.squadrons
                .iter()
                .map(|&class| PanelAction::MoveSquadron {
                    class,
                    from,
                    to: squadron_to,
                }),
        );
        return;
    }
    let member_name = |member: MissionMember| match member {
        MissionMember::Character(key) => world.characters.get(key).map(|c| c.name.clone()),
        MissionMember::SpecialForce(key) => world.special_forces.get(key).and_then(|unit| {
            world
                .buildable_classes
                .get(&unit.class_dat_id)
                .map(|class| class.name.clone())
        }),
    };
    let class_name = |class| {
        world
            .fighter_classes
            .get(class)
            .map(|value| value.name.clone())
    };
    let mut lines: Vec<(String, u32)> = team
        .members
        .iter()
        .zip(member_days)
        .filter_map(|(&member, days)| Some((member_name(member)?, days)))
        .collect();
    lines.extend(
        team.troops
            .iter()
            .zip(troop_days)
            .filter_map(|(&troop, days)| {
                Some((troop_name(world, troop)?, days))
            }),
    );
    lines.extend(team.squadrons.iter().filter_map(|&class| {
        let days = class_days.iter().find(|(known, _)| *known == class)?.1;
        Some((class_name(class)?, days))
    }));
    macroquad::logging::info!(
        "[interface] command={:#x} {source} status=asks lines={} blockaded={blockaded}",
        if confirmed { 0x202 } else { 0x201 },
        lines.len()
    );
    move_confirmation_state.open(MoveConfirmation {
        products: Vec::new(),
        order: ConfirmedOrder::Move,
        faction: context.faction,
        fleets: Vec::new(),
        characters: characters
            .into_iter()
            .filter_map(|member| member.character())
            .collect(),
        special_forces: special_forces
            .into_iter()
            .filter_map(|member| match member {
                MissionMember::SpecialForce(key) => Some(key),
                MissionMember::Character(_) => None,
            })
            .collect(),
        missions: Vec::new(),
        scrap: Vec::new(),
        troops: team.troops.clone(),
        squadrons: team.squadrons.iter().map(|&class| (class, from)).collect(),
        ships: None,
        blockaded,
        destination,
        join,
        lines,
    });
}

/// The system a team's first member stands in, when its container is that
/// system (`FUN_00487cc0` reads the first team id's `+0x1c`): a character
/// or special force not aboard a fleet, a regiment on the planet, or a
/// squadron leaving the planet. `None` for one aboard a fleet.
pub(super) fn team_container_system(
    world: &GameWorld,
    team: &rebellion_render::quadrant_icons::DefensesTeam,
    from: SquadronPlace,
) -> Option<SystemKey> {
    let on_planet = |holds: &dyn Fn(&rebellion_core::world::System) -> bool| {
        world
            .systems
            .iter()
            .find(|(_, system)| holds(system))
            .map(|(key, _)| key)
    };
    if let Some(&member) = team.members.first() {
        return match member {
            MissionMember::Character(key) => {
                let character = world.characters.get(key)?;
                character
                    .current_fleet
                    .is_none()
                    .then_some(character.current_system)
                    .flatten()
            }
            MissionMember::SpecialForce(key) => {
                on_planet(&|system| system.special_forces.contains(&key))
            }
        };
    }
    if let Some(&troop) = team.troops.first() {
        return on_planet(&|system| system.ground_units.contains(&troop));
    }
    match (team.squadrons.first(), from) {
        (Some(_), SquadronPlace::Planet(system)) => Some(system),
        _ => None,
    }
}

/// A team of fleets' Move (`0x201`) or Confirmed Move (`0x202`) with its
/// target set: `FUN_00486fb0` validates it (`FUN_00487740`; the move order's
/// validator `FUN_0053c1a0` runs every member's, so one refusal refuses all),
/// then asks first when `FUN_00487cc0` says so, listing one line per member
/// (`FUN_0053c2e0`), or departs. A move onto a fleet joins it
/// (`FUN_004ffc90`, `FUN_004feca0`).
pub(super) fn issue_fleet_move(
    context: &FleetMoveContext<'_>,
    fleets: &[FleetKey],
    confirmed: bool,
    target: ReleaseTarget,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
    panel_actions: &mut Vec<PanelAction>,
) {
    let world = context.world;
    let is_alliance = context.faction == MissionFaction::Alliance;
    let (destination, join) = match target {
        ReleaseTarget::System(system) => (system, None),
        ReleaseTarget::Fleet { fleet, system } => (system, Some(fleet)),
    };
    let checked = match join {
        Some(target) if !fleets.is_empty() => fleets.iter().try_for_each(|fleet| {
            rebellion_core::fleet_join::validate_join(
                world,
                context.movement,
                &FleetMover::Fleet(*fleet),
                target,
                is_alliance,
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
        }),
        _ => validate_fleets_dispatch(context.movement, world, fleets, destination, is_alliance)
            .map_err(|error| error.to_string()),
    };
    if let Err(error) = checked {
        // port: FUN_00487c90 plays the side's advisor reaction; this line
        // stands in.
        msg_log.refuse(
            GameMessage::new(
                context.tick,
                format!("Fleet move rejected: {error}"),
                MessageCategory::Event,
            ),
            UNTRACED_REFUSAL,
        );
        return;
    }
    // FUN_00487cc0 reads the order's first team id. A fleet icon's is the
    // system, whose container is a sector, so only 0x202 asks
    // (0x00487cc0..0x00487e13).
    let asks = if context.icon {
        confirmed
    } else {
        fleets.iter().any(|fleet| match join {
            Some(target) => rebellion_core::fleet_join::join_confirms(
                world,
                context.blockaded,
                *fleet,
                target,
                confirmed,
            ),
            None => fleet_move_confirms(world, context.blockaded, *fleet, confirmed),
        })
    };
    if !asks {
        panel_actions.extend(fleets.iter().map(|fleet| match join {
            Some(target) => PanelAction::JoinFleet {
                mover: FleetMover::Fleet(*fleet),
                target,
            },
            None => PanelAction::DispatchFleet {
                fleet: *fleet,
                destination,
                troops: Vec::new(),
            },
        }));
        return;
    }
    // FUN_0048a340 → FUN_0049a350: a line per member, its name and days.
    let lines = fleets
        .iter()
        .filter_map(|fleet| {
            let value = world.fleets.get(*fleet)?;
            let days = if value.location == destination {
                0
            } else {
                fleet_transit_ticks(value, world, value.location, destination)?
            };
            Some((fleet_label(world, *fleet)?, days))
        })
        .collect();
    move_confirmation_state.open(MoveConfirmation {
        products: Vec::new(),
        order: ConfirmedOrder::Move,
        faction: context.faction,
        fleets: fleets.to_vec(),
        characters: Vec::new(),
        special_forces: Vec::new(),
        missions: Vec::new(),
        scrap: Vec::new(),
        troops: Vec::new(),
        squadrons: Vec::new(),
        ships: None,
        blockaded: false,
        destination,
        join,
        lines,
    });
}

/// A character's Confirmed Move (`0x202` -> `0x241`): the order validates
/// (`FUN_004f4b60`, `1`/`0x28`), then `FUN_00487cc0` opens the confirmation
/// with the trip's days (`FUN_004f4850`). `FUN_0049a350` shows the
/// evacuation warning for a character whose container is a blockaded
/// system.
#[allow(clippy::too_many_arguments)]
pub(super) fn confirm_character_move(
    world: &GameWorld,
    faction: MissionFaction,
    character: rebellion_core::ids::CharacterKey,
    system: rebellion_core::ids::SystemKey,
    fleet: Option<rebellion_core::ids::FleetKey>,
    tick: u64,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
) {
    let days = rebellion_core::missions::move_days(world, character, system);
    let (Some(c), Some(days), true) = (
        world.characters.get(character),
        days,
        rebellion_core::missions::move_place_allowed(world, faction, system, fleet),
    ) else {
        macroquad::logging::info!("[interface] character_move confirm refused");
        msg_log.refuse(
            GameMessage::new(tick, "Move refused".to_string(), MessageCategory::Event),
            UNTRACED_REFUSAL,
        );
        return;
    };
    let blockaded = c.current_fleet.is_none()
        && c.current_system
            .and_then(|key| world.systems.get(key))
            .is_some_and(|sys| {
                rebellion_core::blockade::BlockadeSystem::system_is_blockaded(world, sys)
            });
    macroquad::logging::info!(
        "[interface] character_move confirm days={} blockaded={}",
        days,
        blockaded
    );
    move_confirmation_state.open(MoveConfirmation {
        order: ConfirmedOrder::Move,
        faction,
        fleets: Vec::new(),
        characters: vec![character],
        special_forces: Vec::new(),
        missions: Vec::new(),
        scrap: Vec::new(),
        troops: Vec::new(),
        squadrons: Vec::new(),
        ships: None,
        blockaded,
        destination: system,
        join: fleet,
        products: Vec::new(),
        lines: vec![(c.name.clone(), days)],
    });
}

/// A capital ship's Confirmed Move (`0x202`): the confirmation lists each
/// ship by its name with its days (A0 fleet3/11: "Corellian Corvette 1:  0"
/// onto a fleet at its own system) under the fleet picture; the checkmark
/// moves them as Move does. port: the days are the trip's at the ships'
/// fleet's speed, 0 within the system.
pub(super) fn confirm_ship_move(
    world: &GameWorld,
    faction: MissionFaction,
    team: ShipTeam,
    target: ReleaseTarget,
    move_confirmation_state: &mut MoveConfirmationState,
) {
    let Some(value) = world.fleets.get(team.fleet) else {
        return;
    };
    let destination = target.system();
    let days = if value.location == destination {
        0
    } else {
        rebellion_core::movement::fleet_transit_ticks(value, world, value.location, destination)
            .unwrap_or(0)
    };
    let lines = team
        .ships
        .iter()
        .filter_map(|&index| {
            let ship = value.capital_ships.get(index)?;
            let class = world.capital_ship_classes.get(ship.class)?;
            let name = world.ship_name(team.fleet, index).unwrap_or(&class.name);
            Some((name.to_owned(), days))
        })
        .collect();
    let join = match target {
        ReleaseTarget::Fleet { fleet, .. } => Some(fleet),
        ReleaseTarget::System(_) => None,
    };
    macroquad::logging::info!("[interface] ship_move confirm days={days}");
    move_confirmation_state.open(MoveConfirmation {
        products: Vec::new(),
        order: ConfirmedOrder::Move,
        faction,
        fleets: Vec::new(),
        characters: Vec::new(),
        special_forces: Vec::new(),
        troops: Vec::new(),
        squadrons: Vec::new(),
        ships: Some(team),
        missions: Vec::new(),
        scrap: Vec::new(),
        blockaded: false,
        destination,
        join,
        lines,
    });
}

/// Whether `FUN_00487cc0` asks before a single unit's Move (`0x201`): when
/// the member's container is a system with the blockade bit (`+0x88` bit
/// `0x20`) of the member's side (read `0x00487cc0..0x00487e13`), as
/// `issue_team_move` asks for a window selection. A unit on a planet has
/// the system as its container; one aboard a fleet (`container` `None`)
/// never asks. The confirmation then shows the evacuation warning 0x7056
/// (`FUN_0049a350`). Every order goes through `FUN_00487740` ->
/// `FUN_00487cc0` (`move-order.md`), the menu's Move of a character,
/// special force, regiment or squadron too.
pub(super) fn move_out_of_blockade_asks(
    world: &GameWorld,
    blockaded: &HashSet<SystemKey>,
    container: Option<SystemKey>,
    faction: MissionFaction,
) -> bool {
    container.is_some_and(|key| {
        blockaded.contains(&key)
            && world
                .systems
                .get(key)
                .is_some_and(|value| value.control.is_controlled_by(Faction::from(faction)))
    })
}

/// A regiment's name in a move's confirmation: its class's (`FUN_0053c2e0`
/// lists each member by `FUN_004f6270`), wherever it stands, on a planet or
/// aboard a fleet.
pub(super) fn troop_name(world: &GameWorld, troop: rebellion_core::ids::TroopKey) -> Option<String> {
    let unit = world.troops.get(troop)?;
    Some(world.buildable_classes.get(&unit.class_dat_id)?.name.clone())
}

/// A regiment's Confirmed Move (`0x202`): the confirmation lists the
/// regiment's days (`FUN_0053c2e0`) and warns of an evacuation when it
/// stands on a blockaded planet (`FUN_0049a350`).
#[allow(clippy::too_many_arguments)]
pub(super) fn confirm_troop_move(
    world: &GameWorld,
    movement: &MovementState,
    transport: &TroopTransportState,
    blockaded_systems: &std::collections::HashSet<rebellion_core::ids::SystemKey>,
    faction: MissionFaction,
    troop: rebellion_core::ids::TroopKey,
    (system, target): (rebellion_core::ids::SystemKey, RegimentTarget),
    tick: u64,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
) {
    let days = transport.move_days(world, movement, blockaded_systems, troop, target);
    let name = troop_name(world, troop);
    let (Ok(days), Some(name)) = (days, name) else {
        macroquad::logging::info!("[interface] troop_move confirm refused");
        msg_log.refuse(
            GameMessage::new(tick, "Move refused".to_string(), MessageCategory::Event),
            UNTRACED_REFUSAL,
        );
        return;
    };
    let blockaded = world
        .systems
        .values()
        .find(|value| value.ground_units.contains(&troop))
        .is_some_and(|value| {
            rebellion_core::blockade::BlockadeSystem::system_is_blockaded(world, value)
        });
    macroquad::logging::info!(
        "[interface] troop_move confirm days={} blockaded={}",
        days,
        blockaded
    );
    let join = match target {
        RegimentTarget::Fleet(fleet) => Some(fleet),
        RegimentTarget::System(_) => None,
    };
    move_confirmation_state.open(MoveConfirmation {
        order: ConfirmedOrder::Move,
        faction,
        fleets: Vec::new(),
        characters: Vec::new(),
        special_forces: Vec::new(),
        troops: vec![troop],
        squadrons: Vec::new(),
        ships: None,
        missions: Vec::new(),
        scrap: Vec::new(),
        blockaded,
        destination: system,
        join,
        products: Vec::new(),
        lines: vec![(name, days)],
    });
}

/// The headquarters' Move to `destination`. `FUN_00487cc0` asks for a
/// Confirmed Move, and for a Move out of a blockaded system of its side;
/// otherwise the order goes at once. The confirmation lists the trip's days
/// (`FUN_0053d200`) under 0x7057, or the evacuation warning 0x7056
/// (`FUN_0049a350`, the headquarters being no fleet).
pub(super) fn order_headquarters_move(
    world: &GameWorld,
    transport: &TroopTransportState,
    faction: MissionFaction,
    destination: rebellion_core::ids::SystemKey,
    confirmed: bool,
    tick: u64,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
    panel_actions: &mut Vec<PanelAction>,
) {
    use rebellion_core::headquarters_move;
    let alliance = faction == MissionFaction::Alliance;
    let (origin, days) = match headquarters_move::plan(world, transport, alliance, destination) {
        Ok(plan) => plan,
        Err(error) => {
            macroquad::logging::info!(
                "[interface] command=0x201 headquarters status=refused reason={error}"
            );
            // port: FUN_00487c90's advisor reaction; this line stands in.
            msg_log.refuse(
                GameMessage::new(
                    tick,
                    format!("Headquarters move rejected: {error}"),
                    MessageCategory::Event,
                ),
                UNTRACED_REFUSAL,
            );
            return;
        }
    };
    let blockaded = world.systems.get(origin).is_some_and(|value| {
        rebellion_core::blockade::BlockadeSystem::system_is_blockaded(world, value)
    });
    if !confirmed && !blockaded {
        panel_actions.push(PanelAction::MoveHeadquarters { destination });
        return;
    }
    let name = headquarters_move::headquarters(world)
        .and_then(|(_, key)| world.manufacturing_facilities.get(key))
        .and_then(|facility| world.buildable_classes.get(&facility.class_dat_id))
        .map_or_else(String::new, |class| class.name.clone());
    macroquad::logging::info!(
        "[interface] headquarters_move confirm days={} blockaded={}",
        days,
        blockaded
    );
    move_confirmation_state.open(MoveConfirmation {
        order: ConfirmedOrder::MoveHeadquarters,
        faction,
        fleets: Vec::new(),
        characters: Vec::new(),
        special_forces: Vec::new(),
        troops: Vec::new(),
        squadrons: Vec::new(),
        ships: None,
        missions: Vec::new(),
        scrap: Vec::new(),
        blockaded,
        destination,
        join: None,
        products: Vec::new(),
        lines: vec![(name, days)],
    });
}

/// A squadron's Confirmed Move (`0x202`): the confirmation lists the
/// squadron's days (`FUN_0053c2e0`) and warns of an evacuation when it
/// stands on a blockaded planet (`FUN_0049a350`).
#[allow(clippy::too_many_arguments)]
pub(super) fn confirm_squadron_move(
    world: &GameWorld,
    transits: &rebellion_core::squadrons::SquadronTransits,
    faction: MissionFaction,
    (class, from): (rebellion_core::ids::FighterKey, SquadronPlace),
    target: ReleaseTarget,
    tick: u64,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
) {
    let alliance = faction == MissionFaction::Alliance;
    let days = transits.move_days(world, alliance, class, 1, from, target.squadron_place());
    let name = world
        .fighter_classes
        .get(class)
        .map(|value| value.name.clone());
    let (Ok(days), Some(name)) = (days, name) else {
        macroquad::logging::info!("[interface] squadron_move confirm refused");
        msg_log.refuse(
            GameMessage::new(tick, "Move refused".to_string(), MessageCategory::Event),
            UNTRACED_REFUSAL,
        );
        return;
    };
    let blockaded = match from {
        SquadronPlace::Planet(system) => world.systems.get(system).is_some_and(|value| {
            rebellion_core::blockade::BlockadeSystem::system_is_blockaded(world, value)
        }),
        SquadronPlace::Fleet(_) => false,
    };
    macroquad::logging::info!(
        "[interface] squadron_move confirm days={} blockaded={}",
        days,
        blockaded
    );
    let join = match target {
        ReleaseTarget::Fleet { fleet, .. } => Some(fleet),
        ReleaseTarget::System(_) => None,
    };
    move_confirmation_state.open(MoveConfirmation {
        order: ConfirmedOrder::Move,
        faction,
        fleets: Vec::new(),
        characters: Vec::new(),
        special_forces: Vec::new(),
        troops: Vec::new(),
        squadrons: vec![(class, from)],
        ships: None,
        missions: Vec::new(),
        scrap: Vec::new(),
        blockaded,
        destination: target.system(),
        join,
        products: Vec::new(),
        lines: vec![(name, days)],
    });
}

/// A special force's Confirmed Move (`0x202`), as a character's: the Move's
/// check, then the confirmation with the unit's class name and the trip's
/// days (`FUN_0049a350`, `FUN_004f4850`).
pub(super) fn confirm_special_force_move(
    world: &GameWorld,
    faction: MissionFaction,
    unit: rebellion_core::ids::SpecialForceKey,
    system: rebellion_core::ids::SystemKey,
    fleet: Option<rebellion_core::ids::FleetKey>,
    tick: u64,
    msg_log: &mut MessageLog,
    move_confirmation_state: &mut MoveConfirmationState,
) {
    let member = MissionMember::SpecialForce(unit);
    let days = rebellion_core::missions::member_move_days(world, member, system);
    let (Some(value), Some(days), true) = (
        world.special_forces.get(unit),
        days,
        rebellion_core::missions::move_place_allowed(world, faction, system, fleet),
    ) else {
        macroquad::logging::info!("[interface] special_force_move confirm refused");
        msg_log.refuse(
            GameMessage::new(tick, "Move refused".to_string(), MessageCategory::Event),
            UNTRACED_REFUSAL,
        );
        return;
    };
    // The first member is not a fleet; its container is its system, or a
    // fleet, which takes no warning.
    let blockaded = world.systems.values().any(|sys| {
        sys.special_forces.contains(&unit)
            && rebellion_core::blockade::BlockadeSystem::system_is_blockaded(world, sys)
    });
    let name = world
        .buildable_classes
        .get(&value.class_dat_id)
        .map_or_else(String::new, |class| class.name.clone());
    macroquad::logging::info!(
        "[interface] special_force_move confirm days={} blockaded={}",
        days,
        blockaded
    );
    move_confirmation_state.open(MoveConfirmation {
        order: ConfirmedOrder::Move,
        faction,
        fleets: Vec::new(),
        characters: Vec::new(),
        special_forces: vec![unit],
        missions: Vec::new(),
        scrap: Vec::new(),
        troops: Vec::new(),
        squadrons: Vec::new(),
        ships: None,
        blockaded,
        destination: system,
        join: fleet,
        products: Vec::new(),
        lines: vec![(name, days)],
    });
}

/// Whether each order of `selection`'s menu passes its own rule (`+0x18`).
pub(super) fn order_gates(
    selection: Option<MenuObject>,
    world: &GameWorld,
    mission_state: &MissionState,
    movement_state: &MovementState,
    troop_transport_state: &TroopTransportState,
    delivery_state: &DeliveryState,
    manufacturing: &ManufacturingState,
    research: &rebellion_core::research::ResearchState,
    player_faction: MissionFaction,
    conditions_word: u32,
) -> OrderGates {
    let is_alliance = player_faction == MissionFaction::Alliance;
    let side = if is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    // port: a band's manager is its system's holder's; the orders' own
    // +0x18 rules (FUN_0052ae30's class) are untraced.
    let holds = |system| {
        world
            .systems
            .get(system)
            .is_some_and(|value| value.control.is_controlled_by(side))
    };
    OrderGates {
        mission: selection
            .and_then(MenuObject::mission_member)
            .is_some_and(|member| {
                mission_state.mission_order_enabled(world, player_faction, &[member])
            }),
        fleet_move: match selection {
            Some(MenuObject::Fleet(fleet)) => {
                fleet_move_enabled(movement_state, world, fleet, is_alliance)
            }
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::Fleets,
            }) => fleets_move_enabled(
                movement_state,
                world,
                &system_side_fleets(movement_state, world, system, is_alliance),
                is_alliance,
            ),
            _ => false,
        },
        // FUN_0053c4b0 resolves the character; the 0x241's +0x18
        // (FUN_0051fe20) builds command 0x250 (FUN_004f4a00), whose +0x18
        // (FUN_0054d1a0 -> FUN_005429e0 -> FUN_0054c110, member slot
        // +0x1c8) refuses a member on a mission or travelling. The side is
        // the character's, a prisoner's being its captor's (FUN_00504e60).
        character_move: match selection {
            Some(MenuObject::Character(character)) => mission_state.move_order_enabled(
                world,
                player_faction,
                &[MissionMember::Character(character)],
            ),
            _ => false,
        },
        // FUN_00536bc0's Move on a unit, as a character's: the unit is the
        // player's.
        // As a character's: the 0x241's +0x18 reaches FUN_0054c110, whose
        // member slot +0x1c8 for a unit (FUN_00533ce0) refuses one of
        // another side, on a mission or travelling.
        special_force_move: match selection {
            Some(MenuObject::SpecialForce(unit)) => mission_state.move_order_enabled(
                world,
                player_faction,
                &[MissionMember::SpecialForce(unit)],
            ),
            _ => false,
        },
        troop_move: match selection {
            Some(MenuObject::Troop(troop)) => {
                troop_transport_state.regiment_move_enabled(world, troop, is_alliance)
            }
            Some(MenuObject::PendingTroop {
                product: Some(product),
                ..
            }) => product_move_enabled(world, product, is_alliance),
            _ => false,
        },
        headquarters_move: matches!(selection, Some(MenuObject::Headquarters(_)))
            && rebellion_core::headquarters_move::move_enabled(
                world,
                troop_transport_state,
                is_alliance,
            ),
        defenses_move: match selection {
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::Defenses,
            }) => defenses_move_enabled(
                world,
                mission_state,
                troop_transport_state,
                delivery_state,
                player_faction,
                system,
            ),
            _ => false,
        },
        squadron_move: match selection {
            Some(MenuObject::Fighter { fleet, index }) => world
                .fleets
                .get(fleet)
                .and_then(|value| value.fighters.get(index))
                .is_some_and(|entry| {
                    rebellion_core::squadrons::move_enabled(
                        world,
                        is_alliance,
                        entry.class,
                        SquadronPlace::Fleet(fleet),
                    )
                }),
            Some(MenuObject::PlanetFighter { system, class }) => {
                rebellion_core::squadrons::move_enabled(
                    world,
                    is_alliance,
                    class,
                    SquadronPlace::Planet(system),
                )
            }
            // An unfinished squadron of the player's (A0 2026-11-15 "u-3").
            Some(MenuObject::PendingFighter {
                product: Some(product),
                ..
            }) => product_move_enabled(world, product, is_alliance),
            _ => false,
        },
        // FUN_004f9860: the ship's fleet is the player's and in orbit. An
        // unfinished ship's: it is the player's (A0 2026-11-15 "u-2").
        ship_move: match selection {
            Some(MenuObject::PendingShip { product }) => {
                product_move_enabled(world, product, is_alliance)
            }
            Some(MenuObject::Ship { fleet, index, .. }) => {
                fleet_move_enabled(movement_state, world, fleet, is_alliance)
                    && world
                        .fleets
                        .get(fleet)
                        .and_then(|value| value.capital_ships.get(index))
                        .is_some_and(|ship| ship.alive)
            }
            _ => false,
        },
        // port: a system's production areas are its manufacturing
        // facilities' (the 0xa0 managers are not modelled), the player's.
        destination: match selection {
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::System,
            }) => facility_icon_has_managers(world, system, side),
            Some(MenuObject::Producer { system, .. }) => holds(system),
            _ => false,
        },
        // A band's Reserved (0x216): as Destination; checked while its
        // manager is Reserved (A0 menus3/30-33). A facility icon's: its team
        // is Destination's (FUN_00512700 kind 4), checked while any manager
        // is Reserved (FUN_0051fd30).
        reserved: match selection {
            Some(MenuObject::Producer { system, .. }) => holds(system),
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::System,
            }) => facility_icon_has_managers(world, system, side),
            _ => false,
        },
        reserved_checked: match selection {
            Some(MenuObject::Producer { system, area }) => manufacturing.is_reserved(system, area),
            Some(MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::System,
            }) => manufacturing.any_reserved(system),
            _ => false,
        },
        // Build opens Build Selection on what the band's manager lists
        // (FUN_0052e580). Its validation (+0x1bc, FUN_00527d00) refuses a
        // manager with no usable yard: A0 menus3/30, 31 grey Build on
        // Coruscant's ship and troop bands (0:0 yards), not on its
        // facilities band (1:1).
        build: match selection {
            Some(MenuObject::Producer { system, area }) => {
                holds(system)
                    && rebellion_core::ai_module1::has_yard(
                        world,
                        rebellion_core::dat::Faction::of_alliance(is_alliance),
                        system,
                        rebellion_core::build_selection::area_family(area),
                    )
                    && !rebellion_core::build_selection::listed_classes(
                        world,
                        research,
                        area,
                        is_alliance,
                    )
                    .is_empty()
            }
            _ => false,
        },
        // Stop clears a band that is building (manual p. 84).
        stop: match selection {
            Some(MenuObject::Producer { system, area }) => {
                holds(system)
                    && manufacturing
                        .queue(system, area)
                        .is_some_and(|queue| !queue.is_empty())
            }
            _ => false,
        },
        // port: the player names its own fleets and ships; FUN_004f6e60
        // refuses a destroyed object (FUN_0053a000). The order's own +0x18
        // rule is untraced.
        // hyp: Rename's +0x18 also runs the object check FUN_004f9860,
        // which refuses an object en route: A0 greys Rename on a fleet in
        // hyperspace and on its ship (a0-vm/menus/22, 29 and 31).
        rename: match selection {
            // An unfinished ship of the player's, not on its trip (A0
            // 2026-11-15 "u-2": Rename white).
            Some(MenuObject::PendingShip { product }) => {
                product_move_enabled(world, product, is_alliance)
            }
            Some(MenuObject::Fleet(fleet)) => {
                world
                    .fleets
                    .get(fleet)
                    .is_some_and(|value| value.is_alliance == is_alliance)
                    && !movement_state.is_in_transit(fleet)
            }
            Some(MenuObject::Ship { fleet, index, .. }) => {
                world.fleets.get(fleet).is_some_and(|value| {
                    value.is_alliance == is_alliance
                        && value
                            .capital_ships
                            .get(index)
                            .is_some_and(|ship| ship.alive)
                }) && !movement_state.is_in_transit(fleet)
            }
            _ => false,
        },
        bombard: bombard_team(movement_state, world, selection, is_alliance).map_or(
            [false; 4],
            |(team, system)| {
                [0x220, 0x221, 0x222, 0x223].map(|kind| {
                    rebellion_core::bombardment::order_enabled(
                        world,
                        movement_state,
                        is_alliance,
                        &team,
                        system,
                        kind,
                    )
                })
            },
        ),
        assault: bombard_team(movement_state, world, selection, is_alliance).is_some_and(|(team, system)| {
            rebellion_core::assault::order_enabled(
                world,
                movement_state,
                troop_transport_state,
                is_alliance,
                &team,
                system,
            )
        }),
        // FUN_004ed750 for each of 0x260..0x263.
        command: command_rows(world, mission_state, selection, is_alliance),
        // FUN_004ed460 (the Retire order's +0x18 through event 0x230).
        // Set where a mission icon is chosen, with the player's fog
        // (`abort_gate`).
        abort: false,
        retire: selection
            .and_then(MenuObject::mission_member)
            .and_then(|member| retire_check(world, mission_state, member, is_alliance))
            .is_some_and(|result| result.is_ok()),
        // The Scrap command's +0x18 is each member's slot +0x5c
        // (FUN_005796c0; scrap::check, a fleet en route refused); an
        // icon's team is its kind's objects of the side (FUN_00512700),
        // and an empty team refuses (FUN_0053c4b0). A0 menus3/06-13.
        // A defenses icon's team also holds the side's regiments and
        // squadrons flying in, whose scrap command refuses while they
        // travel (surface_arrival_pending; A0 2026-11-11-icon-menu-topic/30).
        scrap: selection.is_some_and(|object| {
            if let MenuObject::SystemIcon {
                system,
                quadrant: Quadrant::Defenses,
            } = object
            {
                // Its unfinished or travelling products too (FUN_004f9860
                // status 2 and 4; A0 2026-11-19-edited-saves-10 c-01, c-05).
                if troop_transport_state.surface_arrival_pending(world, system, is_alliance)
                    || delivery_state.surface_arrival_pending(system, side)
                    || rebellion_core::products::defenses_scrap_refused(world, system, is_alliance)
                {
                    return false;
                }
            }
            let team = object.scrap_team(movement_state, world, side);
            !team.is_empty()
                && team.iter().all(|&target| {
                    rebellion_core::scrap::check_with_movement(world, movement_state, side, target)
                        .is_ok()
                })
        }),
        // FUN_0051de80: the game is over (the victory conditions word).
        orders_stopped: conditions_word & rebellion_core::victory::WORD_GAME_OVER != 0,
    }
}

/// The Command submenu's rows on `selection`: which of `0x260..0x263`
/// `FUN_004ed750` allows. None is checked (`object_menu.rs`,
/// `ObjectMenuRow::entry`).
pub(super) fn command_rows(
    world: &GameWorld,
    missions: &MissionState,
    selection: Option<MenuObject>,
    is_alliance: bool,
) -> [bool; 4] {
    use rebellion_core::character_command::{order_allowed, CommandKind};
    let Some(MenuObject::Character(key)) = selection else {
        return [false; 4];
    };
    let Some(c) = world.characters.get(key) else {
        return [false; 4];
    };
    let en_route = missions.is_en_route(rebellion_core::missions::MissionMember::Character(key));
    [0x260, 0x261, 0x262, 0x263].map(|order| {
        let kind = CommandKind::of_order(order).unwrap_or(CommandKind::None);
        order_allowed(c, kind, is_alliance, en_route).is_ok()
    })
}

/// The Retire order's check on `member` (`FUN_004ed460` for a character,
/// `FUN_00533ba0` for a special force), with the name its confirmation
/// lists: a character's own, a special force's class name. `None` when the
/// member is gone.
pub(super) fn retire_check(
    world: &GameWorld,
    missions: &MissionState,
    member: MissionMember,
    is_alliance: bool,
) -> Option<Result<String, rebellion_core::character_retire::Refusal>> {
    use rebellion_core::character_retire::{order_allowed, special_force_allowed};
    let en_route = missions.is_en_route(member);
    match member {
        MissionMember::Character(key) => {
            let c = world.characters.get(key)?;
            Some(order_allowed(c, is_alliance, en_route).map(|()| c.name.clone()))
        }
        MissionMember::SpecialForce(key) => {
            let unit = world.special_forces.get(key)?;
            let name = world
                .buildable_classes
                .get(&unit.class_dat_id)
                .map_or_else(String::new, |class| class.name.clone());
            Some(special_force_allowed(unit, is_alliance, en_route).map(|()| name))
        }
    }
}

/// The team and system of a Planetary Bombardment order on `selection`: a
/// fleet at its system, or a fleet icon's system with its fleets of the
/// player's side (`FUN_00512700`, kind `0x10`).
pub(super) fn bombard_team(
    movement: &MovementState,
    world: &GameWorld,
    selection: Option<MenuObject>,
    is_alliance: bool,
) -> Option<(Vec<FleetKey>, SystemKey)> {
    match selection? {
        MenuObject::Fleet(fleet) => Some((vec![fleet], world.fleets.get(fleet)?.location)),
        MenuObject::SystemIcon {
            system,
            quadrant: Quadrant::Fleets,
        } => Some((
            rebellion_core::movement::system_side_fleets(movement, world, system, is_alliance),
            system,
        )),
        _ => None,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Keep explicit state and UI inputs at this existing integration boundary."
)]
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
#[expect(
    clippy::cast_precision_loss,
    reason = "Retain the existing simulation rounding, saturation and fixed-width arithmetic semantics."
)]
/// An order the validator refused (`FUN_00487c90`): its two status words,
/// whether its team was left empty with decoys and no target, and the
/// port's line for a game without the advisor's messages.
/// hyp: the validator's two words are not read for this refusal; any words
/// but the advisor's special ones take its generic reaction (`FUN_004c4390`,
/// `FUN_004c2230`: one of two messages).
pub(super) const UNTRACED_REFUSAL: (u32, u32) = (0, 0);
/// `FUN_004f4b60`: a movement kind's target of another side (`1`/`0x28`),
/// whose reaction is the advisor's message `+8` (`FUN_004c3bb0`,
/// `FUN_004c19f0`).
pub(super) const OTHER_SIDE_REFUSAL: (u32, u32) = (1, 0x28);

pub(super) struct RefusedOrder {
    pub(super) status: (u32, u32),
    pub(super) decoys_without_target: bool,
    pub(super) text: String,
}

/// A refused mission's two status words and whether the original's decoy
/// reaction applies (`FUN_004c2940`, `FUN_004c0870`: `0x40`/`0x91` with no
/// target object and a decoy list). The validators' words: `FUN_0054bb90`'s
/// `0x40`/`0x91`, `FUN_0056a110`'s `0x40`/`0x28` (and `FUN_005744c0`'s
/// `0x29`), `FUN_00583320`'s `0x40`/`0x01`. hyp: `FUN_00522b30`'s word is
/// not read; `0x40`/`0x91` stands in, without the decoy reaction.
pub(super) fn refusal_status(
    refused: rebellion_core::missions::MissionRefusal,
    has_target: bool,
    has_decoys: bool,
) -> ((u32, u32), bool) {
    use rebellion_core::missions::MissionRefusal;
    match refused {
        MissionRefusal::EmptyTeam => ((0x40, 0x91), !has_target && has_decoys),
        MissionRefusal::MemberUnavailable(_) => ((0x40, 0x91), false),
        MissionRefusal::TargetUnavailable => ((0x40, 0x28), false),
        MissionRefusal::MembersNotAllowed => ((0x40, 0x01), false),
    }
}
