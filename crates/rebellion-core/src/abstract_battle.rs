//! The abstract battle: a tactical battle resolved without display.
//!
//! `FUN_0059a530` steps an abstract battle through `FUN_005ae9c0` at
//! `DAT_0066badc` = 1.0, one source second per step (the unit recharge
//! `FUN_005b6320` takes seconds: displayed play passes milliseconds times
//! `DAT_0066c26c` = 0.001, `FUN_005a9de0`). Each side is a commander object
//! whose step `FUN_005d14d0` lets every task force with a target deal its
//! summed damage rate (`FUN_005cb650`, `FUN_005b7780`) to that target, then
//! recharges its units. Units do not move. A side with no active unit is
//! beaten. When neither side's strength (`FUN_005d0ce0`, `FUN_005b7a80`)
//! has fallen for 1200 seconds (`DAT_0066c3e0`), the weaker side is
//! destroyed (`FUN_005af7a0`). `ghidra/notes/abstract-battle.md`.
//!
//! Not yet ported: the commanders' target scoring (`FUN_005c9200`,
//! `FUN_005ec740`), the retreat decision (slots `+0x0c`, `+0x2c`), and the
//! Death Star's abstract actions.

use std::collections::HashMap;

use crate::tactical::{
    apply_original_capital_damage, original_engine_mode_bonus, original_tactical_damage_rolls,
    original_tactical_mobility, BattlePhase, BattleSession, CombatWinner, TacticalWeaponArc,
    ORIGINAL_CAPITAL_MANEUVERABILITY_MAX, ORIGINAL_CAPITAL_WEAPON_STRENGTH,
    ORIGINAL_FIGHTER_TORPEDO_STRENGTH, ORIGINAL_ION_VS_FIGHTER, ORIGINAL_LASER_VS_CAPITAL,
    ORIGINAL_LASER_VS_FIGHTER, ORIGINAL_TURBOLASER_VS_FIGHTER,
};

/// `DAT_0066badc`: the time `FUN_0059a530` passes to each abstract step.
pub const ABSTRACT_STEP_SECONDS: f32 = 1.0;
/// `DAT_0066c3e0`: the stall after which `FUN_005ae9c0` ends the weaker side.
pub const ABSTRACT_STALEMATE_SECONDS: f32 = 1200.0;
/// `_DAT_0066d0f0`: what an operational Death Star adds to side 1's
/// strength in `FUN_005d0ce0`.
pub const ABSTRACT_DEATH_STAR_STRENGTH: f32 = 20_000_000.0;
/// `DAT_0066c4d4`: the divisor of the source's maneuverability in
/// `FUN_005b7a80`.
pub const ABSTRACT_STRENGTH_MANEUVER_DIVISOR: f32 = 6.0;
/// `_DAT_0066c518`: the floor of the maneuverability ratio in
/// `FUN_005b7780` and `FUN_005b7a80`.
pub const ABSTRACT_MANEUVER_RATIO_FLOOR: f32 = 0.1;
/// port: a bound on the steps, far past any stalemate, so a battle that
/// cannot end does not hang the campaign.
pub const ABSTRACT_STEP_LIMIT: u32 = 100_000;

/// One unit of the battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AbstractUnit {
    Capital(usize),
    Fighter(usize),
}

impl AbstractUnit {
    /// Slot `+0x20`: 0 for a capital ship (`FUN_006158b0`), 1 for a fighter
    /// group (`FUN_0040f340`).
    const fn is_fighter(self) -> bool {
        matches!(self, Self::Fighter(_))
    }
}

/// One task force: capital ships by task force, fighters by group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AbstractGroup {
    /// Side 0 is the Alliance (`+0x24c`, `FUN_005a5b60`), side 1 the Empire
    /// (`+0x250`, `FUN_005a66f0`).
    pub side: u8,
    pub fighters: bool,
    pub id: u8,
}

/// The result of a finished abstract battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbstractOutcome {
    pub winner: CombatWinner,
    pub steps: u32,
    pub stalemate: bool,
}

/// An abstract battle over a tactical session.
#[derive(Debug, Clone)]
pub struct AbstractBattle {
    pub session: BattleSession,
    /// Each task force's target (`FUN_005d2190`).
    pub targets: HashMap<AbstractGroup, AbstractUnit>,
    /// `+0x264`, `+0x268`: each side's lowest strength so far.
    pub lows: [f32; 2],
    /// `+0x26c`, `+0x270`: each side's time since its strength last fell.
    pub stalls: [f32; 2],
    /// `+0x00`: elapsed time.
    pub elapsed: f32,
    /// port: damage owed to a capital ship whose hull and shields the port
    /// keeps as integers.
    pub capital_carry: HashMap<usize, f32>,
    pub steps: u32,
    pub stalemate: bool,
}

const fn side_index(alliance: bool) -> usize {
    if alliance {
        0
    } else {
        1
    }
}

fn unit_is_alliance(session: &BattleSession, unit: AbstractUnit) -> bool {
    let attacker = match unit {
        AbstractUnit::Capital(index) => session.ships[index].is_attacker,
        AbstractUnit::Fighter(index) => session.fighters[index].is_attacker,
    };
    attacker == session.attacker_is_alliance
}

/// State 2 (active): alive and still in the battle.
fn unit_active(session: &BattleSession, unit: AbstractUnit) -> bool {
    match unit {
        AbstractUnit::Capital(index) => session
            .ships
            .get(index)
            .is_some_and(|ship| ship.alive && !ship.retreated && ship.hull_current > 0),
        AbstractUnit::Fighter(index) => session
            .fighters
            .get(index)
            .is_some_and(|fighter| fighter.alive && fighter.squad_count > 0),
    }
}

fn units_of(session: &BattleSession, alliance: bool) -> Vec<AbstractUnit> {
    let capitals = (0..session.ships.len()).map(AbstractUnit::Capital);
    let fighters = (0..session.fighters.len()).map(AbstractUnit::Fighter);
    capitals
        .chain(fighters)
        .filter(|&unit| unit_is_alliance(session, unit) == alliance && unit_active(session, unit))
        .collect()
}

/// The weapon records of a unit: a capital ship's four arcs, a fighter
/// group's one (`FUN_005b05c0`, `FUN_005b9c60`).
fn arcs(session: &BattleSession, unit: AbstractUnit) -> Vec<TacticalWeaponArc> {
    match unit {
        AbstractUnit::Capital(index) => session.ships[index].weapon_arcs.to_vec(),
        AbstractUnit::Fighter(index) => vec![session.fighters[index].weapon_arc],
    }
}

/// `FUN_005d2360` strengths with the `FUN_005d24e0` (laser), `FUN_005d2530`
/// (ion) and `FUN_005d2490` (turbolaser) target modifiers, per weapon, as
/// laser, ion and turbolaser. Ion cannons do nothing against fighters.
fn family_strengths(vs_fighter: bool) -> [f32; 3] {
    let modifiers = if vs_fighter {
        [
            ORIGINAL_LASER_VS_FIGHTER,
            ORIGINAL_ION_VS_FIGHTER,
            ORIGINAL_TURBOLASER_VS_FIGHTER,
        ]
    } else {
        [ORIGINAL_LASER_VS_CAPITAL, 1.0, 1.0]
    };
    modifiers.map(|modifier| ORIGINAL_CAPITAL_WEAPON_STRENGTH * modifier)
}

/// `FUN_005b4c60` (slot `+0x84`): the arc with the most firepower against
/// the target type, the first on a tie, none when all are empty.
pub fn strongest_arc(arcs: &[TacticalWeaponArc], vs_fighter: bool) -> Option<usize> {
    let strengths = family_strengths(vs_fighter);
    let mut best = None;
    let mut best_total = 0.0_f32;
    for (index, arc) in arcs.iter().enumerate() {
        let ion = if vs_fighter {
            0.0
        } else {
            arc.ion_cannons as f32 * strengths[1]
        };
        let total =
            arc.laser_cannons as f32 * strengths[0] + ion + arc.turbolasers as f32 * strengths[2];
        if best_total < total {
            best = Some(index);
            best_total = total;
        }
    }
    best
}

/// `FUN_005b1770`: a value scaled by current over maximum hull.
fn hull_ratio(session: &BattleSession, unit: AbstractUnit) -> f32 {
    match unit {
        AbstractUnit::Capital(index) => session.ships[index].hull_ratio(),
        AbstractUnit::Fighter(index) => {
            let fighter = &session.fighters[index];
            if fighter.hull_max > 0.0 {
                fighter.hull_current.max(0.0) / fighter.hull_max
            } else {
                0.0
            }
        }
    }
}

/// Slot `+0x60`: weapon recharge per second, `FUN_005b0400` for a capital
/// ship and `FUN_005b9c00` for a fighter group.
fn weapon_recharge(session: &BattleSession, unit: AbstractUnit) -> f32 {
    match unit {
        AbstractUnit::Capital(index) => session.ships[index].effective_weapon_recharge(),
        AbstractUnit::Fighter(index) => session.fighters[index].weapon_hull_factor(),
    }
}

/// Slot `+0x64`: shield recharge per second, `FUN_005b0460` for a capital
/// ship and `FUN_005b9c30` for a fighter group.
fn shield_recharge(session: &BattleSession, unit: AbstractUnit) -> f32 {
    match unit {
        AbstractUnit::Capital(index) => session.ships[index].effective_shield_recharge(),
        AbstractUnit::Fighter(index) => session.fighters[index].shield_recharge_rate(),
    }
}

/// Slot `+0xbc`: maneuverability. A capital ship (`FUN_005b0be0`) has 1.0
/// with no engine power, otherwise its abstract mode contribution
/// (`FUN_005afb70`) plus its rating, at most 9. A fighter group uses
/// `FUN_005ba270`.
fn maneuverability(session: &BattleSession, unit: AbstractUnit) -> f32 {
    match unit {
        AbstractUnit::Capital(index) => {
            let ship = &session.ships[index];
            let engine = original_tactical_mobility(&session.ships, &session.field_effects, index)
                .map_or(0.0, |mobility| mobility.effective_engine_power);
            if engine == 0.0 {
                1.0
            } else {
                (original_engine_mode_bonus(ship.maneuver_state_value)
                    + ship.maneuverability as f32)
                    .min(ORIGINAL_CAPITAL_MANEUVERABILITY_MAX)
            }
        }
        AbstractUnit::Fighter(index) => session.fighters[index].effective_maneuverability(),
    }
}

/// The shared core of `FUN_005b7780` and `FUN_005b7a80`: the chosen arc's
/// per-family firepower over its weapon count per recharged second.
/// `maneuver_factor` multiplies laser and turbolaser fire at fighters;
/// `extra` (torpedoes) joins the sum before the division.
fn arc_rate(
    session: &BattleSession,
    source: AbstractUnit,
    vs_fighter: bool,
    maneuver_factor: f32,
    torpedo: f32,
) -> f32 {
    let recharge = weapon_recharge(session, source);
    if recharge == 0.0 {
        return 0.0;
    }
    let arcs = arcs(session, source);
    let Some(arc) = strongest_arc(&arcs, vs_fighter).map(|index| arcs[index]) else {
        return 0.0;
    };
    let fighter_source = source.is_fighter();
    let hull = hull_ratio(session, source);
    let scale = |value: f32| if fighter_source { value * hull } else { value };
    let mut count = arc.laser_cannons + arc.ion_cannons + arc.turbolasers;
    if fighter_source {
        // `FUN_005b1770` then `__ftol`: the scaled count is truncated.
        count = (count as f32 * hull) as u32;
    }
    let per_second = count as f32 / recharge;
    if per_second <= 0.0 {
        return 0.0;
    }
    let strengths = family_strengths(vs_fighter);
    let ion = if vs_fighter {
        0.0
    } else {
        scale(arc.ion_cannons as f32 * strengths[1])
    };
    let laser = scale(arc.laser_cannons as f32 * strengths[0] * maneuver_factor);
    let turbolaser = scale(arc.turbolasers as f32 * strengths[2] * maneuver_factor);
    (ion + laser + turbolaser + scale(torpedo)) / per_second
}

/// `FUN_005b7780`: what `source` deals `target` per second.
pub fn damage_rate(session: &BattleSession, source: AbstractUnit, target: AbstractUnit) -> f32 {
    if !unit_active(session, source) {
        return 0.0;
    }
    let vs_fighter = target.is_fighter();
    let maneuver_factor = if vs_fighter {
        (maneuverability(session, source) / maneuverability(session, target))
            .max(ABSTRACT_MANEUVER_RATIO_FLOOR)
    } else {
        1.0
    };
    // `FUN_005b23a0` torpedoes join when a fighter group fires at a capital
    // ship whose shields are down (`FUN_005b1650`).
    let torpedo = match (source, target) {
        (AbstractUnit::Fighter(fighter), AbstractUnit::Capital(ship))
            if session.ships[ship].shield <= 0 =>
        {
            session.fighters[fighter].torpedo_strength as f32 * ORIGINAL_FIGHTER_TORPEDO_STRENGTH
        }
        _ => 0.0,
    };
    arc_rate(session, source, vs_fighter, maneuver_factor, torpedo)
}

/// `FUN_005b7a80` with the strength argument 0 that `FUN_005d0ce0` passes:
/// a unit's firepower against capital ships.
pub fn unit_strength(session: &BattleSession, unit: AbstractUnit) -> f32 {
    if !unit_active(session, unit) {
        return 0.0;
    }
    arc_rate(session, unit, false, 1.0, 0.0)
}

impl AbstractBattle {
    /// `FUN_005ae460`: the battle starts with each side's strength as its
    /// low.
    #[must_use]
    pub fn new(session: BattleSession) -> Self {
        let mut battle = Self {
            session,
            targets: HashMap::new(),
            lows: [0.0; 2],
            stalls: [0.0; 2],
            elapsed: 0.0,
            capital_carry: HashMap::new(),
            steps: 0,
            stalemate: false,
        };
        battle.lows = [battle.side_strength(true), battle.side_strength(false)];
        battle
    }

    fn death_star_operational(&self) -> bool {
        self.session
            .death_star
            .is_some_and(|death_star| death_star.operational() && !death_star.is_alliance)
    }

    /// The side's task forces in a stable order: capital task forces, then
    /// fighter groups (the commander's list `+0x70`).
    fn groups(&self, alliance: bool) -> Vec<(AbstractGroup, Vec<AbstractUnit>)> {
        let side = side_index(alliance) as u8;
        let mut groups: Vec<(AbstractGroup, Vec<AbstractUnit>)> = Vec::new();
        for unit in units_of(&self.session, alliance) {
            let group = match unit {
                AbstractUnit::Capital(index) => AbstractGroup {
                    side,
                    fighters: false,
                    id: self.session.ships[index].task_force,
                },
                AbstractUnit::Fighter(index) => AbstractGroup {
                    side,
                    fighters: true,
                    id: self.session.fighters[index].fighter_group,
                },
            };
            match groups.iter_mut().find(|(key, _)| *key == group) {
                Some((_, members)) => members.push(unit),
                None => groups.push((group, vec![unit])),
            }
        }
        groups.sort_by_key(|(key, _)| *key);
        groups
    }

    /// hyp: until `FUN_005c9200` and `FUN_005ec740` are ported, a task force
    /// keeps its live target and otherwise takes the first enemy in stable
    /// order. A leader with laser cannons (`FUN_005b7ce0`) looks at fighter
    /// groups first, others at capital ships first.
    fn assign_targets(&mut self, alliance: bool) {
        let enemies = units_of(&self.session, !alliance);
        for (group, members) in self.groups(alliance) {
            if self
                .targets
                .get(&group)
                .is_some_and(|&target| unit_active(&self.session, target))
            {
                continue;
            }
            let leader_has_lasers = arcs(&self.session, members[0])
                .iter()
                .any(|arc| arc.laser_cannons > 0);
            let target = enemies
                .iter()
                .copied()
                .find(|unit| unit.is_fighter() == leader_has_lasers)
                .or_else(|| enemies.first().copied());
            match target {
                Some(target) => {
                    self.targets.insert(group, target);
                }
                None => {
                    self.targets.remove(&group);
                }
            }
        }
    }

    /// `FUN_005cb650`: a task force's summed damage rate at its target.
    fn group_rate(&self, members: &[AbstractUnit], target: AbstractUnit) -> f32 {
        members
            .iter()
            .map(|&member| damage_rate(&self.session, member, target))
            .sum()
    }

    /// `FUN_005b54d0` with `rate × dt`. A capital ship keeps integer hull and
    /// shields in the port, so the fraction carries to its next hit.
    fn deal(&mut self, target: AbstractUnit, damage: f32) {
        if damage <= 0.0 {
            return;
        }
        match target {
            AbstractUnit::Capital(index) => {
                let owed = self.capital_carry.entry(index).or_insert(0.0);
                *owed += damage;
                let whole = owed.floor();
                if whole < 1.0 {
                    return;
                }
                *owed -= whole;
                let whole = whole as i32;
                let rolls = original_tactical_damage_rolls(
                    &self.session.ships,
                    index,
                    whole,
                    &mut self.session.tactical_rng,
                );
                apply_original_capital_damage(
                    &mut self.session.ships,
                    &mut self.session.field_effects,
                    index,
                    whole,
                    rolls,
                );
            }
            AbstractUnit::Fighter(index) => {
                // hyp: the non-virtual `FUN_005b54d0` reads a fighter group's
                // shield and hull at the capital offsets: shields first,
                // overflow to hull with at least one point (`_DAT_0066c500`).
                // Its two draws come from the same stream.
                let rng = &mut self.session.tactical_rng;
                let fighter = &mut self.session.fighters[index];
                let shield_before = fighter.shield;
                fighter.shield -= damage;
                if shield_before > 0.0 {
                    rng.inclusive(0, 100);
                }
                if fighter.shield < 0.0 {
                    let overflow = fighter.shield.min(-1.0);
                    fighter.shield = 0.0;
                    fighter.hull_current = (fighter.hull_current + overflow).max(0.0);
                    rng.inclusive(0, 100);
                }
                fighter.refresh_craft_count();
            }
        }
    }

    /// `FUN_005d14d0` (slot `+0x28`): one side's step. Returns true when the
    /// side is beaten.
    fn side_step(&mut self, alliance: bool) -> bool {
        self.assign_targets(alliance);
        for (group, members) in self.groups(alliance) {
            let Some(&target) = self.targets.get(&group) else {
                continue;
            };
            if !unit_active(&self.session, target) {
                continue;
            }
            let damage = self.group_rate(&members, target) * ABSTRACT_STEP_SECONDS;
            self.deal(target, damage);
        }
        // `FUN_005b6320`: shield and weapon recharge of every unit.
        for unit in units_of(&self.session, alliance) {
            match unit {
                AbstractUnit::Capital(index) => {
                    self.session.ships[index].advance_original_recharge(ABSTRACT_STEP_SECONDS);
                }
                AbstractUnit::Fighter(index) => {
                    self.session.fighters[index].advance_original_recharge(ABSTRACT_STEP_SECONDS);
                }
            }
        }
        let active = units_of(&self.session, alliance).len();
        active == 0 && (alliance || !self.death_star_operational())
    }

    /// `FUN_005d0ce0` (slot `+0x14`): the side's strength. A capital ship
    /// that an enemy task force out-damages (its rate at least the ship's
    /// shield recharge) does not count.
    pub fn side_strength(&self, alliance: bool) -> f32 {
        let enemy_groups = self.groups(!alliance);
        let mut strength = 0.0;
        for unit in units_of(&self.session, alliance) {
            if let AbstractUnit::Capital(_) = unit {
                let overwhelmed = enemy_groups.iter().any(|(group, members)| {
                    self.targets.get(group) == Some(&unit)
                        && shield_recharge(&self.session, unit) <= self.group_rate(members, unit)
                });
                if overwhelmed {
                    continue;
                }
            }
            strength += unit_strength(&self.session, unit);
        }
        if !alliance && self.death_star_operational() {
            strength += ABSTRACT_DEATH_STAR_STRENGTH;
        }
        strength
    }

    /// `FUN_005af7a0`: every unit of the side is destroyed.
    fn destroy_side(&mut self, alliance: bool) {
        for unit in units_of(&self.session, alliance) {
            match unit {
                AbstractUnit::Capital(index) => {
                    let ship = &mut self.session.ships[index];
                    ship.hull_current = 0;
                    ship.shield = 0;
                    ship.alive = false;
                }
                AbstractUnit::Fighter(index) => {
                    let fighter = &mut self.session.fighters[index];
                    fighter.hull_current = 0.0;
                    fighter.refresh_craft_count();
                }
            }
        }
    }

    /// `FUN_005ae9c0`: one step of the battle. Returns the outcome once a
    /// side is beaten.
    pub fn step(&mut self) -> Option<AbstractOutcome> {
        self.steps += 1;
        self.elapsed += ABSTRACT_STEP_SECONDS;
        // Side 1 (`+0x250`) steps before side 0 (`+0x24c`); both beaten
        // tests are taken before either answer is used.
        let empire_beaten = self.side_step(false);
        let alliance_beaten = self.side_step(true);
        let strengths = [self.side_strength(true), self.side_strength(false)];
        for ((strength, low), stall) in strengths
            .iter()
            .zip(self.lows.iter_mut())
            .zip(self.stalls.iter_mut())
        {
            if *strength < *low {
                *low = *strength;
                *stall = 0.0;
            } else {
                *stall += ABSTRACT_STEP_SECONDS;
            }
        }
        if !alliance_beaten && !empire_beaten {
            if self.stalls[0].min(self.stalls[1]) >= ABSTRACT_STALEMATE_SECONDS {
                self.stalemate = true;
                let alliance = if self.death_star_operational() {
                    -1.0
                } else {
                    strengths[0]
                };
                let empire = strengths[1];
                // The side with the lower strength ends; on a tie both do.
                if empire <= alliance {
                    self.destroy_side(false);
                }
                if alliance <= empire {
                    self.destroy_side(true);
                }
            }
            return None;
        }
        Some(self.finish())
    }

    /// `FUN_0040c780`: the winner is the only side with survivors.
    fn finish(&mut self) -> AbstractOutcome {
        let alliance_left = !units_of(&self.session, true).is_empty();
        let empire_left = !units_of(&self.session, false).is_empty();
        let attacker_left = if self.session.attacker_is_alliance {
            alliance_left
        } else {
            empire_left
        };
        let defender_left = if self.session.attacker_is_alliance {
            empire_left
        } else {
            alliance_left
        };
        let winner = match (attacker_left, defender_left) {
            (true, false) => CombatWinner::Attacker,
            (false, true) => CombatWinner::Defender,
            _ => CombatWinner::Draw,
        };
        self.session.winner = Some(winner);
        self.session.phase = BattlePhase::Results;
        AbstractOutcome {
            winner,
            steps: self.steps,
            stalemate: self.stalemate,
        }
    }

    /// Step until the battle ends, or `ABSTRACT_STEP_LIMIT` steps pass.
    pub fn run(&mut self) -> AbstractOutcome {
        while self.steps < ABSTRACT_STEP_LIMIT {
            if let Some(outcome) = self.step() {
                return outcome;
            }
        }
        self.finish()
    }
}

/// Resolve a battle session without display.
#[must_use]
pub fn resolve_abstract(session: BattleSession) -> (BattleSession, AbstractOutcome) {
    let mut battle = AbstractBattle::new(session);
    let outcome = battle.run();
    (battle.session, outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CapitalShipKey, DatId, FighterKey, FleetKey, SystemKey};
    use crate::tactical::{
        original_navigation_point_sets, OriginalTacticalLayout, OriginalTacticalRng,
        TacticalFighter, TacticalFighterRecoveryState, TacticalObjectIdentity, TacticalOrder,
        TacticalShip, TacticalSubsystemCapacity, TacticalSubsystemCondition,
        TacticalSubsystemDamage, TacticalTactic, TacticalWeaponRanges, TacticalWorldPosition,
        TacticalWorldVector, ORIGINAL_TRENCH_RUN_SILENT_EVENT,
    };

    fn ship(alliance: bool, fore: TacticalWeaponArc, recharge: f32) -> TacticalShip {
        TacticalShip {
            class_key: CapitalShipKey::default(),
            identity: TacticalObjectIdentity {
                class_dat_id: DatId::new(64),
                fleet_roster_index: 0,
                is_alliance: alliance,
            },
            tactical_resource: None,
            death_star_resource: None,
            source_position: TacticalWorldPosition::ORIGIN,
            source_forward: TacticalWorldVector::SOURCE_FORWARD,
            source_desired_forward: TacticalWorldVector::SOURCE_FORWARD,
            source_velocity: TacticalWorldVector::ZERO,
            source_waypoint: None,
            navigation_route: Vec::new(),
            source_collision_envelope: None,
            name: "ship".into(),
            x: 0.0,
            y: 0.0,
            hull_current: 100,
            hull_max: 100,
            shield: 50,
            shield_max: 50,
            subsystem_condition: TacticalSubsystemCondition {
                shields: 100,
                weapons: 100,
                tractor: 0,
                engines: 100,
                hyperdrive: 0,
            },
            subsystem_capacity: TacticalSubsystemCapacity {
                shields: true,
                weapons: true,
                tractor: false,
                engines: true,
                hyperdrive: 0,
            },
            subsystem_damage: TacticalSubsystemDamage::default(),
            sublight_engine_power: 2.0,
            maneuverability: 2,
            tractor_beam_power: 0.0,
            fighter_capacity: 0,
            maneuver_state_value: None,
            damage_control: 0,
            is_attacker: alliance,
            alive: true,
            selected: false,
            order: TacticalOrder::None,
            tactic: TacticalTactic::StandOff,
            task_force: 0,
            fleet_ship_index: 0,
            sprite_id: None,
            weapon_arcs: [
                fore,
                TacticalWeaponArc::default(),
                TacticalWeaponArc::default(),
                TacticalWeaponArc::default(),
            ],
            weapon_ranges: TacticalWeaponRanges::default(),
            weapon_recharge_rate: recharge,
            shield_recharge_rate: 1.0,
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

    fn fighter(alliance: bool, arc: TacticalWeaponArc) -> TacticalFighter {
        TacticalFighter {
            class_key: FighterKey::default(),
            identity: TacticalObjectIdentity {
                class_dat_id: DatId::new(128),
                fleet_roster_index: 0,
                is_alliance: alliance,
            },
            tactical_resource: None,
            source_position: TacticalWorldPosition::ORIGIN,
            sublight_engine_power: 10.0,
            fleet_fighter_index: 0,
            fleet_squadron_index: 0,
            name: "fighter".into(),
            x: 0.0,
            y: 0.0,
            squad_count: 12,
            hull_current: 96.0,
            hull_max: 96.0,
            shield: 0.0,
            shield_max: 0.0,
            weapon_arc: arc,
            weapon_ranges: TacticalWeaponRanges::default(),
            torpedo_strength: 0,
            torpedo_range: 0.0,
            maneuverability: 5,
            is_attacker: alliance,
            alive: true,
            fighter_group: 0,
            selected: false,
            order: TacticalOrder::None,
            tactic: TacticalTactic::StandOff,
            attack_target: None,
            escort_target: None,
            recovery_state: TacticalFighterRecoveryState::AwaitingCarrier,
            recovery_target: None,
        }
    }

    fn session(ships: Vec<TacticalShip>, fighters: Vec<TacticalFighter>) -> BattleSession {
        BattleSession {
            system: SystemKey::default(),
            system_name: "Test".into(),
            system_picture_id: 1,
            attacker_fleet: FleetKey::default(),
            defender_fleet: FleetKey::default(),
            attacker_is_alliance: true,
            player_is_attacker: true,
            phase: BattlePhase::Combat,
            ships,
            fighters,
            death_star: None,
            death_star_beam: None,
            trench_run_outcome: None,
            trench_run_remaining_ms: None,
            trench_run_group: None,
            trench_run_success_rating: 1,
            trench_run_pending_outcome: None,
            trench_run_participants: Vec::new(),
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
            tactical_rng: OriginalTacticalRng::new(7),
            weapon_effects: Vec::new(),
            impact_effects: Vec::new(),
            pending_audio_cues: Vec::new(),
            pending_voice_cues: Vec::new(),
            field_effects: Vec::new(),
            subsystem_repairs: Vec::new(),
            paused: false,
            combat_speed: 1,
            step_accumulator: 0.0,
            winner: None,
            diagnostic_log: Vec::new(),
        }
    }

    // FUN_005b4c60: the arc with the greatest total, the first on a tie.
    #[test]
    fn the_strongest_arc_wins_and_a_tie_keeps_the_first() {
        let arcs = [
            TacticalWeaponArc::new(0, 0, 2),
            TacticalWeaponArc::new(0, 0, 3),
            TacticalWeaponArc::new(0, 0, 3),
            TacticalWeaponArc::default(),
        ];
        assert_eq!(strongest_arc(&arcs, false), Some(1));
        assert_eq!(
            strongest_arc(&[TacticalWeaponArc::default(); 4], false),
            None
        );
    }

    // FUN_005b4c60 and FUN_005b7780: ion cannons count for nothing against
    // fighters.
    #[test]
    fn ion_cannons_neither_aim_nor_fire_at_fighters() {
        let arcs = [
            TacticalWeaponArc::new(0, 5, 0),
            TacticalWeaponArc::new(1, 0, 0),
            TacticalWeaponArc::default(),
            TacticalWeaponArc::default(),
        ];
        assert_eq!(strongest_arc(&arcs, true), Some(1));
        let battle = session(
            vec![ship(true, TacticalWeaponArc::new(0, 5, 0), 10.0)],
            vec![fighter(false, TacticalWeaponArc::default())],
        );
        assert_eq!(
            damage_rate(&battle, AbstractUnit::Capital(0), AbstractUnit::Fighter(0)),
            0.0
        );
    }

    // FUN_005b7780: the arc's firepower over its weapon count per recharged
    // second; turbolasers keep a 1.0 modifier against capital ships.
    #[test]
    fn a_capital_ship_deals_its_mean_weapon_strength_per_recharged_second() {
        let battle = session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 10.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        );
        let rate = damage_rate(&battle, AbstractUnit::Capital(0), AbstractUnit::Capital(1));
        assert!((rate - ORIGINAL_CAPITAL_WEAPON_STRENGTH * 10.0).abs() < 1e-5);
    }

    // FUN_005d14d0: a task force deals its rate times the 1.0 step to its
    // target every step.
    #[test]
    fn each_step_takes_one_second_of_fire_off_the_target() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        ));
        let rate = damage_rate(
            &battle.session,
            AbstractUnit::Capital(0),
            AbstractUnit::Capital(1),
        );
        assert_eq!(battle.step(), None);
        let lost = 50 - battle.session.ships[1].shield;
        assert_eq!(lost, rate.floor() as i32);
    }

    // FUN_005ae9c0 with DAT_0066c3e0 = 1200: when neither strength falls for
    // 1200 seconds the weaker side is destroyed (FUN_005af7a0).
    #[test]
    fn a_stalled_battle_destroys_the_weaker_side_after_1200_seconds() {
        let mut battle = AbstractBattle::new(session(
            vec![ship(true, TacticalWeaponArc::new(0, 5, 0), 10.0)],
            vec![fighter(false, TacticalWeaponArc::default())],
        ));
        for _ in 0..1199 {
            assert_eq!(battle.step(), None);
        }
        assert!(battle.session.fighters[0].alive);
        assert_eq!(battle.step(), None);
        assert!(!battle.session.fighters[0].alive);
        let outcome = battle.step().expect("the Empire has no unit left");
        assert_eq!(outcome.winner, CombatWinner::Attacker);
        assert!(outcome.stalemate);
        assert!(battle.session.ships[0].alive);
    }

    // FUN_005ae9c0: on equal strength both sides end.
    #[test]
    fn a_stall_between_equal_sides_destroys_both() {
        let mut battle = AbstractBattle::new(session(
            Vec::new(),
            vec![
                fighter(true, TacticalWeaponArc::default()),
                fighter(false, TacticalWeaponArc::default()),
            ],
        ));
        let outcome = battle.run();
        assert_eq!(outcome.winner, CombatWinner::Draw);
        assert_eq!(outcome.steps, 1201);
    }

    fn fighter_rate_fixture(hull: f32) -> BattleSession {
        let mut source = fighter(true, TacticalWeaponArc::new(5, 0, 0));
        source.hull_current = hull;
        let mut target = fighter(false, TacticalWeaponArc::default());
        target.maneuverability = 2;
        session(Vec::new(), vec![source, target])
    }

    // FUN_005b7780 for a fighter group: its terms and its weapon count scale
    // with hull (FUN_005b1770), the count is truncated (__ftol), the
    // recharge is FUN_005b9c00, and fire at fighters takes the source over
    // target maneuverability (FUN_005ba270: 6 over 3).
    #[test]
    fn a_fighter_group_rate_scales_with_hull_and_truncates_its_weapon_count() {
        let full = fighter_rate_fixture(96.0);
        let rate = damage_rate(&full, AbstractUnit::Fighter(0), AbstractUnit::Fighter(1));
        let recharge = 96.0 * 3.751 / 8.0;
        let expected = 5.0 * ORIGINAL_CAPITAL_WEAPON_STRENGTH * 2.0 / (5.0 / recharge);
        assert!((rate - expected).abs() < 1e-3, "{rate} vs {expected}");

        let half = fighter_rate_fixture(48.0);
        let rate = damage_rate(&half, AbstractUnit::Fighter(0), AbstractUnit::Fighter(1));
        let recharge = 48.0 * 3.751 / 8.0;
        // 5 lasers at half hull count as trunc(2.5) = 2.
        let expected = 5.0 * ORIGINAL_CAPITAL_WEAPON_STRENGTH * 2.0 * 0.5 / (2.0 / recharge);
        assert!((rate - expected).abs() < 1e-3, "{rate} vs {expected}");
    }

    // FUN_005b7780: torpedoes (FUN_005b23a0) join only against a capital
    // ship whose shields are down (FUN_005b1650).
    #[test]
    fn torpedoes_join_only_against_a_ship_with_its_shields_down() {
        let mut source = fighter(true, TacticalWeaponArc::new(0, 0, 4));
        source.torpedo_strength = 6;
        let mut battle = session(
            vec![ship(false, TacticalWeaponArc::default(), 0.0)],
            vec![source],
        );
        let shielded = damage_rate(&battle, AbstractUnit::Fighter(0), AbstractUnit::Capital(0));
        battle.ships[0].shield = 0;
        let open = damage_rate(&battle, AbstractUnit::Fighter(0), AbstractUnit::Capital(0));
        let recharge = 96.0 * 3.751 / 8.0;
        let torpedo = 6.0 * ORIGINAL_FIGHTER_TORPEDO_STRENGTH;
        assert!((open - shielded - torpedo / (4.0 / recharge)).abs() < 1e-3);
        assert!(shielded > 0.0);
    }

    // FUN_005b0be0: a ship's maneuverability is 1.0 without engine power,
    // otherwise FUN_005afb70's 1.0 plus its rating, at most 9. Turbolaser
    // fire at fighters takes it over the fighter's (FUN_005b7780).
    #[test]
    fn a_ship_firing_at_fighters_uses_its_maneuverability_over_theirs() {
        let mut battle = session(
            vec![ship(true, TacticalWeaponArc::new(0, 0, 4), 10.0)],
            vec![fighter(false, TacticalWeaponArc::default())],
        );
        let rate = damage_rate(&battle, AbstractUnit::Capital(0), AbstractUnit::Fighter(0));
        // (1 + 2) / 6 for a fighter with rating 5.
        let expected =
            ORIGINAL_CAPITAL_WEAPON_STRENGTH * ORIGINAL_TURBOLASER_VS_FIGHTER * 0.5 * 10.0;
        assert!((rate - expected).abs() < 1e-4, "{rate} vs {expected}");
        battle.ships[0].maneuverability = 20;
        let capped = damage_rate(&battle, AbstractUnit::Capital(0), AbstractUnit::Fighter(0));
        assert!((capped - expected * 3.0).abs() < 1e-4, "{capped}");
        battle.ships[0].sublight_engine_power = 0.0;
        let stopped = damage_rate(&battle, AbstractUnit::Capital(0), AbstractUnit::Fighter(0));
        // 1.0 / 6 is above the 0.1 floor.
        assert!((stopped - expected / 3.0).abs() < 1e-4, "{stopped}");
    }

    // FUN_005d0ce0: a capital ship whose attackers out-damage its shield
    // recharge (slot +0x64) adds nothing to its side's strength.
    #[test]
    fn an_out_damaged_ship_adds_no_strength() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 10.0),
                ship(false, TacticalWeaponArc::new(0, 0, 4), 10.0),
            ],
            Vec::new(),
        ));
        let alone = battle.side_strength(false);
        assert!((alone - unit_strength(&battle.session, AbstractUnit::Capital(1))).abs() < 1e-6);
        battle.assign_targets(true);
        assert_eq!(battle.side_strength(false), 0.0);
        battle.session.ships[1].shield_recharge_rate = 100.0;
        assert!((battle.side_strength(false) - alone).abs() < 1e-6);
    }

    // FUN_005d0ce0: an operational Empire Death Star adds 20,000,000.
    #[test]
    fn an_operational_death_star_adds_to_the_empire_strength() {
        let mut battle = AbstractBattle::new(session(Vec::new(), Vec::new()));
        battle.session.death_star = Some(crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 1.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        assert_eq!(battle.side_strength(false), ABSTRACT_DEATH_STAR_STRENGTH);
        assert_eq!(battle.side_strength(true), 0.0);
        // FUN_005d14d0: side 1 with its Death Star is not beaten.
        assert!(!battle.side_step(false));
        assert!(battle.side_step(true));
    }

    // port: a capital ship keeps integer hull and shields, so a fraction of
    // a point carries to the next step.
    #[test]
    fn fractions_of_a_point_carry_to_the_next_step() {
        let mut battle = AbstractBattle::new(session(
            vec![ship(false, TacticalWeaponArc::default(), 0.0)],
            Vec::new(),
        ));
        battle.deal(AbstractUnit::Capital(0), 0.6);
        assert_eq!(battle.session.ships[0].shield, 50);
        battle.deal(AbstractUnit::Capital(0), 0.6);
        assert_eq!(battle.session.ships[0].shield, 49);
        assert!((battle.capital_carry[&0] - 0.2).abs() < 1e-5);
    }

    // FUN_005b54d0: shield overflow reaches hull with at least one point
    // (_DAT_0066c500 = -1).
    #[test]
    fn a_fighter_group_loses_at_least_one_hull_point_when_its_shield_breaks() {
        let mut battle = AbstractBattle::new(session(
            Vec::new(),
            vec![fighter(false, TacticalWeaponArc::default())],
        ));
        battle.session.fighters[0].shield = 0.5;
        battle.deal(AbstractUnit::Fighter(0), 0.6);
        assert_eq!(battle.session.fighters[0].shield, 0.0);
        assert!((battle.session.fighters[0].hull_current - 95.0).abs() < 1e-5);
        battle.deal(AbstractUnit::Fighter(0), 10.0);
        assert!((battle.session.fighters[0].hull_current - 85.0).abs() < 1e-5);
        assert_eq!(battle.session.fighters[0].squad_count, 11);
    }

    // FUN_005ae9c0: a fall below a side's low resets its clock.
    #[test]
    fn a_falling_strength_restarts_the_stall_clock() {
        let mut battle = AbstractBattle::new(session(
            vec![ship(true, TacticalWeaponArc::new(0, 5, 0), 10.0)],
            vec![fighter(false, TacticalWeaponArc::default())],
        ));
        for _ in 0..10 {
            battle.step();
        }
        assert_eq!(battle.stalls, [10.0, 10.0]);
        battle.session.ships[0].hull_current = 50;
        battle.step();
        assert_eq!(battle.stalls, [0.0, 11.0]);
        assert!(battle.lows[0] < unit_strength(&battle.session, AbstractUnit::Capital(0)) + 1e-6);
    }

    // FUN_0040c780: the only side with survivors wins; the Empire defends.
    #[test]
    fn the_side_left_standing_wins() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::default(), 0.0),
                ship(false, TacticalWeaponArc::new(0, 0, 8), 40.0),
            ],
            Vec::new(),
        ));
        let outcome = battle.run();
        assert_eq!(outcome.winner, CombatWinner::Defender);
        assert!(!outcome.stalemate);
        assert!(!battle.session.ships[0].alive);
        assert_eq!(battle.session.winner, Some(CombatWinner::Defender));
        assert_eq!(battle.session.phase, BattlePhase::Results);
    }

    // hyp targeting: a laser-armed leader picks fighters first, others pick
    // capital ships first; a live target is kept, each task force its own.
    #[test]
    fn laser_leaders_take_fighters_and_others_take_ships() {
        let mut lasers = ship(true, TacticalWeaponArc::new(2, 0, 0), 10.0);
        lasers.task_force = 1;
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 2), 10.0),
                lasers,
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            vec![fighter(false, TacticalWeaponArc::default())],
        ));
        battle.assign_targets(true);
        let group = |id| AbstractGroup {
            side: 0,
            fighters: false,
            id,
        };
        assert_eq!(battle.targets[&group(0)], AbstractUnit::Capital(2));
        assert_eq!(battle.targets[&group(1)], AbstractUnit::Fighter(0));
        battle.targets.insert(group(1), AbstractUnit::Capital(2));
        battle.assign_targets(true);
        assert_eq!(battle.targets[&group(1)], AbstractUnit::Capital(2));
        battle.session.ships[2].alive = false;
        battle.assign_targets(true);
        assert_eq!(battle.targets[&group(1)], AbstractUnit::Fighter(0));
        assert_eq!(battle.targets[&group(0)], AbstractUnit::Fighter(0));
    }

    // State 2: a ship that left or has no hull is out of the battle.
    #[test]
    fn a_ship_that_left_or_has_no_hull_is_inactive() {
        let mut battle = session(
            vec![ship(true, TacticalWeaponArc::default(), 0.0)],
            vec![fighter(true, TacticalWeaponArc::default())],
        );
        assert!(unit_active(&battle, AbstractUnit::Capital(0)));
        battle.ships[0].retreated = true;
        assert!(!unit_active(&battle, AbstractUnit::Capital(0)));
        battle.ships[0].retreated = false;
        battle.ships[0].hull_current = 0;
        assert!(!unit_active(&battle, AbstractUnit::Capital(0)));
        battle.fighters[0].squad_count = 0;
        assert!(!unit_active(&battle, AbstractUnit::Fighter(0)));
    }

    // FUN_005b4c60 weighs each family by its strength and modifier: three
    // lasers (0x3e2b020c against ships) lose to one turbolaser, one ion
    // cannon to two turbolasers, one turbolaser to two ion cannons.
    #[test]
    fn the_arc_choice_weighs_each_family_by_its_modifier() {
        let arc = TacticalWeaponArc::new;
        let none = TacticalWeaponArc::default();
        assert_eq!(
            strongest_arc(&[arc(3, 0, 0), arc(0, 0, 1), none, none], false),
            Some(1)
        );
        assert_eq!(
            strongest_arc(&[arc(0, 0, 2), arc(0, 1, 0), none, none], false),
            Some(0)
        );
        assert_eq!(
            strongest_arc(&[arc(0, 0, 1), arc(0, 2, 0), none, none], false),
            Some(1)
        );
    }

    // FUN_005ba420: a destroyed Death Star adds nothing; neither does an
    // Alliance one.
    #[test]
    fn only_a_standing_imperial_death_star_counts() {
        let mut battle = AbstractBattle::new(session(Vec::new(), Vec::new()));
        let mut death_star = crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: 1.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: true,
            action_committed: false,
        };
        battle.session.death_star = Some(death_star);
        assert_eq!(battle.side_strength(false), 0.0);
        death_star.destroyed = false;
        death_star.is_alliance = true;
        battle.session.death_star = Some(death_star);
        assert_eq!(battle.side_strength(false), 0.0);
    }

    // FUN_005b54d0: a hit that leaves the shield at exactly zero does not
    // reach the hull; it draws once for the shield, and once more on an
    // overflow.
    #[test]
    fn a_shield_emptied_exactly_spares_the_hull_and_draws_once() {
        let mut battle = AbstractBattle::new(session(
            Vec::new(),
            vec![fighter(false, TacticalWeaponArc::default())],
        ));
        battle.session.fighters[0].shield = 1.0;
        let mut expected = battle.session.tactical_rng;
        battle.deal(AbstractUnit::Fighter(0), 1.0);
        expected.inclusive(0, 100);
        assert_eq!(battle.session.tactical_rng, expected);
        assert!((battle.session.fighters[0].hull_current - 96.0).abs() < 1e-6);
        battle.deal(AbstractUnit::Fighter(0), 2.0);
        assert_eq!(battle.session.tactical_rng, {
            let mut rng = expected;
            rng.inclusive(0, 100);
            rng
        });
    }

    // FUN_005ae9c0: +0x00 counts the elapsed seconds.
    #[test]
    fn the_battle_counts_its_elapsed_seconds() {
        let mut battle = AbstractBattle::new(session(
            vec![ship(true, TacticalWeaponArc::new(0, 5, 0), 10.0)],
            vec![fighter(false, TacticalWeaponArc::default())],
        ));
        battle.step();
        battle.step();
        assert_eq!(battle.elapsed, 2.0);
    }

    // FUN_005b7780: ion cannons keep a 1.0 modifier against capital ships
    // (FUN_005d2530).
    #[test]
    fn ion_cannons_deal_their_full_strength_to_ships() {
        let battle = session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 3, 0), 10.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        );
        let rate = damage_rate(&battle, AbstractUnit::Capital(0), AbstractUnit::Capital(1));
        assert!((rate - ORIGINAL_CAPITAL_WEAPON_STRENGTH * 10.0).abs() < 1e-5);
    }
}
