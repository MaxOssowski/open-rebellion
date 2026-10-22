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
//! Each step the commanders give idle task forces targets (`FUN_005c9200`,
//! `FUN_005ec740`, `FUN_005ed170`). A side whose answer to the battle
//! choice orders a retreat (`retreat_allowed`) may leave (slots `+0x0c`,
//! `+0x2c`). At the battle's end
//! the loser's fleets leave on Autorouting ([`withdraw_loser`],
//! `FUN_00516ef0`). The Death Star charges and the Alliance may make the
//! trench run ([`AbstractBattle::step`], `FUN_005aefc0`).

use std::collections::HashMap;

use crate::combat::{CombatSide, FighterLossEvent, ShipDamageEvent, SpaceCombatResult};
use crate::ids::{FleetKey, SystemKey};
use crate::world::GameWorld;

use crate::tactical::{
    apply_original_capital_damage, original_engine_mode_bonus, original_tactical_damage_rolls,
    original_tactical_mobility, BattlePhase, BattleSession, CombatWinner, FleetBattle,
    TacticalWeaponArc, ORIGINAL_CAPITAL_MANEUVERABILITY_MAX, ORIGINAL_CAPITAL_WEAPON_STRENGTH,
    ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE, ORIGINAL_FIGHTER_TORPEDO_STRENGTH,
    ORIGINAL_ION_VS_FIGHTER, ORIGINAL_LASER_VS_CAPITAL, ORIGINAL_LASER_VS_FIGHTER,
    ORIGINAL_TURBOLASER_VS_FIGHTER,
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
/// `DAT_0066cbac`: the score limit `FUN_005c9200` passes to the pairing.
pub const ABSTRACT_PAIR_SCORE_LIMIT: f32 = 180.0;
/// `DAT_0066da84`: the combined score under which a task force keeps its
/// target (`FUN_005ed5f0`).
pub const ABSTRACT_KEEP_TARGET_SCORE: f32 = 300.0;
/// `DAT_0066da9c`: the score of a target the rate cannot wear down.
pub const ABSTRACT_NO_SCORE: f32 = -1.0;
/// `DAT_0066da58`: the base key of a task force that cannot pair alone.
pub const ABSTRACT_UNPAIRED_KEY: f32 = 100_000.0;
/// `DAT_0066da8c`: the weight of hull in a target's hit points.
pub const ABSTRACT_SCORE_HULL_WEIGHT: f32 = 0.65;
/// `DAT_0066da98`: the weight of the travel time in a score.
pub const ABSTRACT_SCORE_TRAVEL_WEIGHT: f32 = 0.001;
/// `DAT_0066da7c`: the weight of a target's strength in its priority.
pub const ABSTRACT_PRIORITY_STRENGTH_WEIGHT: f32 = 15.0;
/// `DAT_0066daa0`: the weight of its shield and hull ratios.
pub const ABSTRACT_PRIORITY_RATIO_WEIGHT: f32 = 0.01;
/// `+0x24` of a side commander (`FUN_005c7cb0`, `0x3ea8f5c3`): the share of
/// the enemy's strength under which a side that may leave does.
pub const ABSTRACT_RETREAT_RATIO: f32 = f32::from_bits(0x3ea8_f5c3);
/// port: a bound on the steps, far past any stalemate, so a battle that
/// cannot end does not hang the campaign.
pub const ABSTRACT_STEP_LIMIT: u32 = 100_000;
/// `FUN_005ba7f0`: the superlaser charge a step adds, dt (`DAT_0066badc`,
/// 1.0) x 1/3 (`DAT_0066c708`) x 0.001 (`DAT_0066c718`). The display passes
/// milliseconds; the abstract step passes 1.0, so a full charge of 100
/// takes 300,000 steps.
pub const ABSTRACT_DEATH_STAR_CHARGE_PER_STEP: f32 = ABSTRACT_STEP_SECONDS * (1.0 / 3.0) * 0.001;
/// `FUN_005aefc0`: the trench run's visits before every member left is
/// destroyed (`0x5af047`).
pub const TRENCH_RUN_VISITS: u32 = 100;
/// `FUN_005aefc0`: hull per hit unit (`DAT_0066c3dc`, 8.0).
pub const TRENCH_RUN_HULL_UNIT: f32 = 8.0;
/// `FUN_005afc00`: the commander term's floor (`DAT_0066c3f4`), its value
/// with no commanding person.
pub const TRENCH_RUN_COMMANDER_FLOOR: f32 = 1.0;
/// `FUN_005afc00`: the commander term's cap (`DAT_0066c3ec`).
pub const TRENCH_RUN_COMMANDER_CAP: f32 = 9.0;
/// `FUN_0040d680`: a battle person's `+0x50` is its combat (slot `+0x1f0`)
/// over this integer (`DAT_006a80e8`, 20).
pub const BATTLE_PERSON_COMBAT_DIVISOR: f32 = 20.0;
/// `FUN_005aefc0`: the success chance's scale (`DAT_0066c3f0`, 0.5).
pub const TRENCH_RUN_SUCCESS_SCALE: f32 = 0.5;

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

/// What a trench run (`FUN_005aefc0`) came to: the member visits it made
/// and whether the Death Star fell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrenchRun {
    pub visits: u32,
    pub success: bool,
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
    /// `+0x80` of each side's commander: the side may leave the battle.
    /// `FUN_00599c90` copies it from the battle object's `+0x10` and
    /// `+0x14` (`FUN_005afb30`), the sides' retreat words.
    pub retreat_allowed: [bool; 2],
    /// `+0x84` = 5: the side's retreat has run.
    pub retreat_done: [bool; 2],
    /// `+0xb0`: the stalemate marked the side to leave.
    pub retreat_marked: [bool; 2],
    /// `FUN_005afc00(0)`: the Alliance commander's term in the trench run
    /// ([`trench_commander_term`]).
    pub trench_commander: f32,
    /// The system's last arrived side (`+0x78` bits 6..7,
    /// [`crate::world::System::last_arrival`]), which `FUN_0040c780` asks
    /// when both retreat words are set.
    pub last_arrival: Option<crate::dat::Faction>,
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

/// Policy slot `+0x10` (`FUN_005ed550`): an active unit with some weapon
/// recharge (slot `+0x60`). The retreat branch is not ported.
fn target_eligible(session: &BattleSession, unit: AbstractUnit) -> bool {
    unit_active(session, unit) && weapon_recharge(session, unit) != 0.0
}

/// Policy slot `+0x08` (`FUN_005ed8e0`): 15 × the unit's strength less
/// 0.01 × its shield-generator ratio (`FUN_005b1710`) and hull ratio
/// (`FUN_005b1640`). The game-list terms are empty without display.
fn target_priority(session: &BattleSession, unit: AbstractUnit) -> f32 {
    let shield_ratio = match unit {
        AbstractUnit::Capital(index) => {
            let ship = &session.ships[index];
            if ship.shield_recharge_rate == 0.0 {
                0.0
            } else {
                ship.effective_shield_recharge() / ship.shield_recharge_rate
            }
        }
        // hyp: a fighter group's +0x3e8 base is its full-hull recharge.
        AbstractUnit::Fighter(_) => hull_ratio(session, unit),
    };
    unit_strength(session, unit) * ABSTRACT_PRIORITY_STRENGTH_WEIGHT
        - shield_ratio * ABSTRACT_PRIORITY_RATIO_WEIGHT
        - hull_ratio(session, unit) * ABSTRACT_PRIORITY_RATIO_WEIGHT
}

/// A unit's retained-mode position (`+0x30`).
fn position(session: &BattleSession, unit: AbstractUnit) -> (f32, f32, f32) {
    let at = match unit {
        AbstractUnit::Capital(index) => session.ships[index].source_position,
        AbstractUnit::Fighter(index) => session.fighters[index].source_position,
    };
    (at.x, at.y, at.z)
}

/// hyp: the speed (slot `+0x34`) of a task force's object `+0x54`, taken as
/// its leader's sublight engine power.
fn engine_speed(session: &BattleSession, unit: AbstractUnit) -> f32 {
    match unit {
        AbstractUnit::Capital(index) => session.ships[index].sublight_engine_power,
        AbstractUnit::Fighter(index) => session.fighters[index].sublight_engine_power,
    }
}

/// `FUN_005b1b70`: installed hyperdrives outnumber hyperdrive hits
/// (`FUN_005b1bc0`) and the engines have power (slot `+0x90`). The port's
/// fighter groups carry no hyperdrive.
fn hyperdrive_works(session: &BattleSession, unit: AbstractUnit) -> bool {
    match unit {
        AbstractUnit::Capital(index) => {
            let ship = &session.ships[index];
            ship.subsystem_capacity.hyperdrive > ship.subsystem_damage.hyperdrive
                && original_tactical_mobility(&session.ships, &session.field_effects, index)
                    .is_some_and(|mobility| mobility.effective_engine_power != 0.0)
        }
        AbstractUnit::Fighter(_) => false,
    }
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
            retreat_allowed: [false; 2],
            retreat_done: [false; 2],
            retreat_marked: [false; 2],
            trench_commander: TRENCH_RUN_COMMANDER_FLOOR,
            last_arrival: None,
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

    /// `FUN_005d1a70` (the commander's list `+0x90`): the enemy's active
    /// units, highest priority first.
    fn ranked_enemies(&self, alliance: bool) -> Vec<AbstractUnit> {
        let mut ranked: Vec<(f32, AbstractUnit)> = units_of(&self.session, !alliance)
            .into_iter()
            .map(|unit| (-target_priority(&self.session, unit), unit))
            .collect();
        ranked.sort_by(|left, right| left.0.total_cmp(&right.0));
        ranked.into_iter().map(|(_, unit)| unit).collect()
    }

    /// `FUN_005c9200` (after slot `+0x08`, `FUN_005d1a70`): each side's
    /// commander gives its idle task forces targets.
    fn assign_targets(&mut self, alliance: bool) {
        let groups = self.groups(alliance);
        let ranked = self.ranked_enemies(alliance);
        // `FUN_005c9200`: fighters, then ships, for laser-armed leaders;
        // ships, then fighters, then ships again, for the others.
        let fighters: Vec<AbstractUnit> = ranked
            .iter()
            .copied()
            .filter(|unit| unit.is_fighter())
            .collect();
        let ships: Vec<AbstractUnit> = ranked
            .iter()
            .copied()
            .filter(|unit| !unit.is_fighter())
            .collect();
        let fighter_list: Vec<AbstractUnit> = fighters.iter().chain(&ships).copied().collect();
        let ship_list: Vec<AbstractUnit> = ships.iter().chain(&fighter_list).copied().collect();

        let mut ship_groups = Vec::new();
        let mut laser_groups = Vec::new();
        for (group, members) in &groups {
            if self.keeps_target(*group, &groups) {
                continue;
            }
            // `FUN_005b7ce0`: the leader carries laser cannons in some arc.
            let lasers = arcs(&self.session, members[0])
                .iter()
                .any(|arc| arc.laser_cannons > 0);
            if lasers {
                laser_groups.push((*group, members.clone()));
            } else {
                ship_groups.push((*group, members.clone()));
            }
        }
        self.pair(ship_groups, &ship_list);
        self.pair(laser_groups, &fighter_list);
    }

    /// Policy slot `+0x0c` (`FUN_005ed5f0`): a task force keeps an eligible
    /// target while it and the side's other task forces on it score under
    /// 300 (`DAT_0066da84`) and not -1.
    fn keeps_target(
        &self,
        group: AbstractGroup,
        groups: &[(AbstractGroup, Vec<AbstractUnit>)],
    ) -> bool {
        let Some(&target) = self.targets.get(&group) else {
            return false;
        };
        if !target_eligible(&self.session, target) {
            return false;
        }
        let members = |key: AbstractGroup| {
            groups
                .iter()
                .find(|(other, _)| *other == key)
                .map(|(_, members)| members.as_slice())
                .unwrap_or(&[])
        };
        let mut running = 0.0;
        let mut score = self.score(target, members(group), &mut running);
        if score >= ABSTRACT_KEEP_TARGET_SCORE || score == ABSTRACT_NO_SCORE {
            for (other, other_members) in groups {
                if *other != group && self.targets.get(other) == Some(&target) {
                    score = self.score(target, other_members, &mut running);
                }
            }
            if score >= ABSTRACT_KEEP_TARGET_SCORE || score == ABSTRACT_NO_SCORE {
                return false;
            }
        }
        true
    }

    /// Policy slot `+0x04` (`FUN_005ed170`): the seconds the running rate
    /// needs to wear the target down, plus a small travel term. `running`
    /// grows by the task force's rate (`FUN_005cb650`).
    fn score(&self, target: AbstractUnit, members: &[AbstractUnit], running: &mut f32) -> f32 {
        if members.is_empty() {
            return ABSTRACT_NO_SCORE;
        }
        *running += self.group_rate(members, target);
        let (hull, shield) = match target {
            AbstractUnit::Capital(index) => {
                let ship = &self.session.ships[index];
                (ship.hull_current as f32, ship.shield as f32)
            }
            AbstractUnit::Fighter(index) => {
                let fighter = &self.session.fighters[index];
                (fighter.hull_current, fighter.shield)
            }
        };
        let points = (hull * ABSTRACT_SCORE_HULL_WEIGHT + shield).max(1.0);
        let recharge = shield_recharge(&self.session, target);
        let margin = *running - recharge;
        if margin <= 0.0 {
            return ABSTRACT_NO_SCORE;
        }
        let mut score = points / margin;
        let leader = members[0];
        let from = position(&self.session, leader);
        let to = position(&self.session, target);
        let (dx, dy, dz) = (to.0 - from.0, to.1 - from.1, to.2 - from.2);
        if dx != 0.0 || dy != 0.0 || dz != 0.0 {
            let speed = engine_speed(&self.session, leader);
            let travel = if speed == 0.0 {
                0.0
            } else {
                dx.hypot(dy).hypot(dz) / speed
            };
            score += travel.max(1.0) * ABSTRACT_SCORE_TRAVEL_WEIGHT;
        }
        score
    }

    /// Strategy `0x66cbe8` slot `+0x04` (`FUN_005ec740`): pair idle task
    /// forces with eligible targets in list order, alone where one scores
    /// 0..180 (`DAT_0066cbac`), otherwise combined from the weakest up.
    fn pair(&mut self, groups: Vec<(AbstractGroup, Vec<AbstractUnit>)>, targets: &[AbstractUnit]) {
        let mut idle = groups;
        if idle.is_empty() || targets.is_empty() {
            return;
        }
        for &target in targets {
            if !target_eligible(&self.session, target) {
                continue;
            }
            // Each idle task force scored alone; a score outside 0..180
            // sorts after, by 100,000 (`DAT_0066da58`) less its rate.
            let mut scored: Vec<(f32, usize)> = idle
                .iter()
                .enumerate()
                .map(|(index, (_, members))| {
                    let mut running = 0.0;
                    let score = self.score(target, members, &mut running);
                    let key = if (0.0..=ABSTRACT_PAIR_SCORE_LIMIT).contains(&score) {
                        score
                    } else {
                        ABSTRACT_UNPAIRED_KEY - running
                    };
                    (key, index)
                })
                .collect();
            scored.sort_by(|left, right| left.0.total_cmp(&right.0));
            let Some(&(_, head)) = scored.first() else {
                return;
            };
            if scored.len() == 1 {
                let (group, _) = idle.remove(head);
                self.targets.insert(group, target);
                return;
            }
            // From the tail toward the head, add each task force to the
            // head's rate; the first combined score in 0..180 links the tail
            // through that one. The head itself is not linked.
            let mut running = self.group_rate(&idle[head].1, target);
            let mut linked = None;
            for position in (1..scored.len()).rev() {
                let index = scored[position].1;
                let score = self.score(target, &idle[index].1, &mut running);
                if (0.0..=ABSTRACT_PAIR_SCORE_LIMIT).contains(&score) {
                    linked = Some(position);
                    break;
                }
            }
            if let Some(from) = linked {
                let mut chosen: Vec<usize> =
                    scored[from..].iter().map(|&(_, index)| index).collect();
                for &index in &chosen {
                    self.targets.insert(idle[index].0, target);
                }
                chosen.sort_unstable();
                for index in chosen.into_iter().rev() {
                    idle.remove(index);
                }
            }
        }
        // Every task force still idle takes the first eligible target, or
        // the first of the list.
        let fallback = targets
            .iter()
            .copied()
            .find(|&target| target_eligible(&self.session, target))
            .unwrap_or(targets[0]);
        for (group, _) in idle {
            self.targets.insert(group, fallback);
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
        self.death_star_step(alliance);
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
        if self.should_retreat(alliance) {
            self.retreat(alliance);
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

    /// Slot `+0x0c` (`FUN_005d0f50`): the side may leave, has not yet, and a
    /// capital ship has a working hyperdrive; for the Empire, an operational
    /// Death Star also lets it. No arrivals are pending (`FUN_005c7e40`).
    pub fn can_leave(&self, alliance: bool) -> bool {
        let side = side_index(alliance);
        if !self.retreat_allowed[side] || self.retreat_done[side] {
            return false;
        }
        let jumps = units_of(&self.session, alliance)
            .into_iter()
            .any(|unit| hyperdrive_works(&self.session, unit));
        jumps || (!alliance && self.death_star_operational())
    }

    /// Slot `+0x2c` (`FUN_005d19a0`): a side that can leave does when the
    /// stalemate marked it, when it has no strength left, or when its
    /// strength falls under 0.33 (`+0x24`) of the enemy's.
    pub fn should_retreat(&self, alliance: bool) -> bool {
        if !self.can_leave(alliance) {
            return false;
        }
        if self.retreat_marked[side_index(alliance)] {
            return true;
        }
        let own = self.side_strength(alliance);
        if own == 0.0 {
            return true;
        }
        let enemy = self.side_strength(!alliance);
        enemy > 0.0 && own / enemy < ABSTRACT_RETREAT_RATIO
    }

    /// `FUN_005d14d0` step 5: each capital ship with a working hyperdrive
    /// escapes (`FUN_005af910`, state 9); the others are destroyed
    /// (`FUN_005af7a0`). hyp: a fighter group escapes when its side has an
    /// escaping carrier (`FUN_005afe40` recovers it), and is destroyed
    /// otherwise.
    fn retreat(&mut self, alliance: bool) {
        let units = units_of(&self.session, alliance);
        let carrier_escapes = units.iter().any(|&unit| match unit {
            AbstractUnit::Capital(index) => {
                self.session.ships[index].fighter_capacity > 0
                    && hyperdrive_works(&self.session, unit)
            }
            AbstractUnit::Fighter(_) => false,
        });
        for unit in units {
            let escapes = match unit {
                AbstractUnit::Capital(_) => hyperdrive_works(&self.session, unit),
                AbstractUnit::Fighter(_) => carrier_escapes,
            };
            match (unit, escapes) {
                (AbstractUnit::Capital(index), true) => {
                    self.session.ships[index].retreated = true;
                }
                (AbstractUnit::Fighter(index), true) => {
                    self.session.fighters[index].alive = false;
                }
                (AbstractUnit::Capital(index), false) => {
                    let ship = &mut self.session.ships[index];
                    ship.hull_current = 0;
                    ship.shield = 0;
                    ship.alive = false;
                }
                (AbstractUnit::Fighter(index), false) => {
                    let fighter = &mut self.session.fighters[index];
                    fighter.hull_current = 0.0;
                    fighter.refresh_craft_count();
                }
            }
        }
        self.retreat_done[side_index(alliance)] = true;
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
            self.destroy(unit);
        }
    }

    /// `FUN_005af7a0`: the unit is destroyed (state `0xb`).
    fn destroy(&mut self, unit: AbstractUnit) {
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

    /// `FUN_005d14d0`'s first part: the Death Star. The Empire charges the
    /// superlaser (`FUN_005ba7f0`) and, at full charge, destroys the first
    /// capital ship of its enemy list (`+0x90`, `FUN_005d1a70`) and empties
    /// the charge (`FUN_005ba710`). The Alliance, with more fighter units
    /// than the Empire (`+0x64`, `FUN_005c5eb0`) and an attackable Death
    /// Star (`FUN_005ba450`), sends its first fighter task force on the
    /// trench run (`FUN_005aefc0`).
    ///
    /// hyp: the leader kind 4 the Alliance looks for (slot `+0x04`) is a
    /// fighter group, as the port's task forces of fighters.
    fn death_star_step(&mut self, alliance: bool) {
        let Some(death_star) = self.session.death_star.filter(|ds| !ds.is_alliance) else {
            return;
        };
        if !alliance {
            if !death_star.operational() {
                return;
            }
            // FUN_005ba7f0's own hull and +0x74 tests are within
            // operational (FUN_005ba420).
            let ds = self.session.death_star.as_mut().expect("checked above");
            if ds.laser_charge != ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE {
                ds.laser_charge = (ds.laser_charge + ABSTRACT_DEATH_STAR_CHARGE_PER_STEP)
                    .min(ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE);
            }
            if ds.laser_charge == ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE {
                let target = self
                    .ranked_enemies(false)
                    .into_iter()
                    .find(|unit| !unit.is_fighter());
                if let Some(target) = target {
                    if let Some(ds) = self.session.death_star.as_mut() {
                        ds.laser_charge = 0.0;
                    }
                    self.destroy(target);
                }
            }
            return;
        }
        if !death_star.attackable() {
            return;
        }
        let fighters = |alliance| {
            units_of(&self.session, alliance)
                .into_iter()
                .filter(|unit| unit.is_fighter())
                .count()
        };
        if fighters(false) >= fighters(true) {
            return;
        }
        if let Some((_, members)) = self
            .groups(true)
            .into_iter()
            .find(|(group, _)| group.fighters)
        {
            let members: Vec<usize> = members
                .into_iter()
                .filter_map(|unit| match unit {
                    AbstractUnit::Fighter(index) => Some(index),
                    AbstractUnit::Capital(_) => None,
                })
                .collect();
            self.trench_run(&members);
        }
    }

    /// `FUN_005aefc0`: the trench run of a fighter task force. Its members
    /// are visited in turn, at most [`TRENCH_RUN_VISITS`] times, after which
    /// every member left is destroyed. A member whose maneuverability (slot
    /// `+0xbc`) is under a roll of 1..10 is hit for (1..hull / 8) x 8, and
    /// destroyed when that leaves it 1 or less to roll or no hull. A member
    /// left with hull rolls 1..100 against (`FUN_005b23a0` + the commander
    /// term) x 0.5; at or under it, the Death Star's hull and charge empty
    /// (`FUN_005ba8a0`).
    ///
    /// The commander term is [`AbstractBattle::trench_commander`]
    /// ([`trench_commander_term`]). port: the units the Death Star
    /// carries (its lists `+0x38`, `+0x3c`) are not known, so none is
    /// destroyed with it; the port's Death Star capital ship takes the
    /// Death Star's fall.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "The run truncates its floats as the original's __ftol."
    )]
    fn trench_run(&mut self, members: &[usize]) -> TrenchRun {
        let mut left: Vec<usize> = members.to_vec();
        let mut at = 0;
        let mut visits = 0;
        while !left.is_empty() {
            if visits == TRENCH_RUN_VISITS {
                for index in std::mem::take(&mut left) {
                    self.destroy(AbstractUnit::Fighter(index));
                }
                break;
            }
            visits += 1;
            let index = left[at];
            let unit = AbstractUnit::Fighter(index);
            let maneuverability = maneuverability(&self.session, unit) as u32;
            let mut destroyed = false;
            if self.session.tactical_rng.inclusive(1, 10) > maneuverability {
                let hits =
                    (self.session.fighters[index].hull_current / TRENCH_RUN_HULL_UNIT) as u32;
                if hits <= 1 {
                    destroyed = true;
                } else {
                    let roll = self.session.tactical_rng.inclusive(1, hits);
                    self.deal(unit, roll as f32 * TRENCH_RUN_HULL_UNIT);
                    destroyed = self.session.fighters[index].hull_current <= 0.0;
                }
            }
            if destroyed {
                self.destroy(unit);
                left.remove(at);
                if !left.is_empty() {
                    at %= left.len();
                }
                continue;
            }
            let fighter = &self.session.fighters[index];
            // FUN_005b1770: the rating scaled by hull over full hull. A
            // member still here has hull, so its full hull is above 0.
            let torpedoes = fighter.hull_current
                * fighter.torpedo_strength as f32
                * ORIGINAL_FIGHTER_TORPEDO_STRENGTH
                / fighter.hull_max;
            let chance = ((torpedoes + self.trench_commander) * TRENCH_RUN_SUCCESS_SCALE) as u32;
            if self.session.tactical_rng.inclusive(1, 100) <= chance {
                if let Some(ds) = self.session.death_star.as_mut() {
                    ds.hull = 0.0;
                    ds.laser_charge = 0.0;
                }
                let empire = !self.session.attacker_is_alliance;
                for ship in &mut self.session.ships {
                    if ship.death_star_resource.is_some() && ship.is_attacker == empire {
                        ship.hull_current = 0;
                        ship.shield = 0;
                        ship.alive = false;
                    }
                }
                return TrenchRun {
                    visits,
                    success: true,
                };
            }
            at = (at + 1) % left.len();
        }
        TrenchRun {
            visits,
            success: false,
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
                // A side that can leave (slot `+0x0c`) is marked `+0xb0`
                // instead; the Alliance's mark ends the step.
                if empire <= alliance {
                    if self.can_leave(false) {
                        self.retreat_marked[1] = true;
                    } else {
                        self.destroy_side(false);
                    }
                }
                if alliance <= empire {
                    if self.can_leave(true) {
                        self.retreat_marked[0] = true;
                        return None;
                    }
                    self.destroy_side(true);
                }
            }
            return None;
        }
        Some(self.finish())
    }

    /// Whether a unit of the side escaped (state 9, `FUN_005af910`): a
    /// capital ship that jumped, or a fighter group that left with it and
    /// kept its craft.
    fn side_escaped(&self, alliance: bool) -> bool {
        let session = &self.session;
        let ships = (0..session.ships.len()).map(AbstractUnit::Capital);
        let fighters = (0..session.fighters.len()).map(AbstractUnit::Fighter);
        ships.chain(fighters).any(|unit| {
            unit_is_alliance(session, unit) == alliance
                && match unit {
                    AbstractUnit::Capital(index) => {
                        let ship = &session.ships[index];
                        ship.alive && ship.retreated
                    }
                    AbstractUnit::Fighter(index) => {
                        let fighter = &session.fighters[index];
                        !fighter.alive && fighter.squad_count > 0
                    }
                }
        })
    }

    /// The seven result words (`FUN_005ae9c0`, `0x5aec52`) and the winner
    /// `FUN_0040c780` takes from them (`battle-flow.md`, "The winner and
    /// the loser's withdrawal"):
    ///
    /// - words 0 and 2: the Alliance's or the Empire's retreat ran
    ///   (`+0x84` == 5) and one of its units escaped;
    /// - word 1: the Empire has no active unit, word 0 is clear and no
    ///   Death Star is operational (`FUN_005ba420`); word 3: the Alliance
    ///   has no active unit and word 2 is clear; both set clears both;
    /// - the winner is the Alliance on word 1 alone and the Empire on
    ///   word 3 alone; failing that, the Alliance on word 0 alone and the
    ///   Empire on word 2 alone;
    /// - with words 0 and 2 both set, the system's last arrived side
    ///   (`+0x78` bits 6..7): the Alliance for 1, else the Empire.
    ///
    /// port: the Death Star's `+0x78` in word 2 is not modelled.
    fn finish(&mut self) -> AbstractOutcome {
        let words_0_2 = [
            self.retreat_done[0] && self.side_escaped(true),
            self.retreat_done[1] && self.side_escaped(false),
        ];
        let mut word_1 = !words_0_2[0]
            && units_of(&self.session, false).is_empty()
            && !self.death_star_operational();
        let mut word_3 = !words_0_2[1] && units_of(&self.session, true).is_empty();
        if word_1 && word_3 {
            word_1 = false;
            word_3 = false;
        }
        let alliance_won = match (word_1, word_3) {
            (true, false) => Some(true),
            (false, true) => Some(false),
            _ => match words_0_2 {
                [true, false] => Some(true),
                [false, true] => Some(false),
                [true, true] => Some(self.last_arrival == Some(crate::dat::Faction::Alliance)),
                [false, false] => None,
            },
        };
        let winner = match alliance_won {
            Some(alliance) if alliance == self.session.attacker_is_alliance => {
                CombatWinner::Attacker
            }
            Some(_) => CombatWinner::Defender,
            None => CombatWinner::Draw,
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

/// port: the seed of the battle object's stream (`FUN_005af420`) taken from
/// the campaign's first combat roll.
#[must_use]
pub fn abstract_seed(rolls: &[f64]) -> u32 {
    rolls.first().map_or(0, |roll| {
        (roll.clamp(0.0, 1.0) * f64::from(u32::MAX)) as u32
    })
}

/// A strategic space battle resolved without display (`FUN_0040a050`
/// state 8): every fleet of both sides at the system fights one abstract
/// battle, and `FUN_0040c780` carries each hull and squadron back to its
/// fleet. The result names the first fleet of each side.
///
/// `retreat_allowed` (Alliance, Empire) is each side's answer to the
/// battle choice: its retreat word, non-zero for a retreat order
/// (`FUN_004362b0`), which reaches the side commander's `+0x80`
/// (`FUN_005afb30`).
#[must_use]
pub fn resolve_space_battle(
    world: &GameWorld,
    system: SystemKey,
    attackers: &[FleetKey],
    defenders: &[FleetKey],
    seed: u32,
    tick: u64,
    retreat_allowed: [bool; 2],
) -> (SpaceCombatResult, AbstractOutcome) {
    let FleetBattle {
        session,
        ship_fleets,
        fighter_fleets,
    } = BattleSession::new_for_fleets(world, system, attackers, defenders, true, tick, seed);
    let mut battle = AbstractBattle::at_system(world, system, session, retreat_allowed);
    let outcome = battle.run();
    let session = battle.session;

    let mut ship_damage = Vec::new();
    for (ship, &fleet_key) in session.ships.iter().zip(&ship_fleets) {
        let Some(fleet) = world.fleets.get(fleet_key) else {
            continue;
        };
        let Some(before) = fleet.capital_ships.get(ship.fleet_ship_index) else {
            continue;
        };
        let after = if ship.alive {
            ship.hull_current.max(0)
        } else {
            0
        };
        let hits_after = world
            .capital_ship_classes
            .get(before.class)
            .map_or(before.subsystem_hits, |class| ship.strategic_hits(class));
        if !ship_changed(before, after, hits_after) {
            continue;
        }
        // `ship_index` counts the fleet's live hulls.
        let ship_index = fleet.capital_ships[..ship.fleet_ship_index]
            .iter()
            .filter(|other| other.alive)
            .count();
        ship_damage.push(ShipDamageEvent {
            fleet: fleet_key,
            ship_index,
            hull_before: before.hull_current,
            hull_after: after,
            hits_after,
        });
    }

    // `apply_space_combat_result_inner` counts live hulls as it marks them
    // dead, so each fleet's events run from the last hull to the first.
    ship_damage.sort_by(|left, right| right.ship_index.cmp(&left.ship_index));

    let mut survivors: Vec<((FleetKey, usize), u32)> = Vec::new();
    for (fighter, &fleet_key) in session.fighters.iter().zip(&fighter_fleets) {
        let key = (fleet_key, fighter.fleet_fighter_index);
        // An escaped group keeps its craft.
        let alive = u32::from(fighter.squad_count > 0);
        match survivors.iter_mut().find(|(entry, _)| *entry == key) {
            Some((_, count)) => *count += alive,
            None => survivors.push((key, alive)),
        }
    }
    let fighter_losses = survivors
        .into_iter()
        .filter_map(|((fleet_key, fighter_index), after)| {
            let before = world
                .fleets
                .get(fleet_key)?
                .fighters
                .get(fighter_index)?
                .count;
            (after != before).then_some(FighterLossEvent {
                fleet: fleet_key,
                fighter_index,
                squads_before: before,
                squads_after: after,
            })
        })
        .collect();

    let winner = match outcome.winner {
        CombatWinner::Attacker => CombatSide::Attacker,
        CombatWinner::Defender => CombatSide::Defender,
        CombatWinner::Draw => CombatSide::Draw,
    };
    (
        SpaceCombatResult {
            attacker_fleet: attackers[0],
            defender_fleet: defenders[0],
            system,
            winner,
            ship_damage,
            fighter_losses,
            tick,
        },
        outcome,
    )
}

/// One fleet of a battle's loser sent away (`FUN_00516ef0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Withdrawal {
    pub fleet: FleetKey,
    pub destination: SystemKey,
    pub transit_ticks: u32,
}

/// `FUN_00516ef0` (state 9, the battle's end): each fleet of the loser at
/// the system is set Autorouting (`FUN_004f78e0(1)`), and the event `0x307`
/// handler (`FUN_00515480`) moves it (`FUN_00556390`) to the autoroute
/// search's destination (`FUN_005545e0`, [`crate::missions`]'s
/// `nearest_place`). A battle with no winner, both sides still present,
/// goes to the system's last arrived side (`+0x78` bits 6..7); with one
/// side gone nobody leaves.
///
/// port: the search leaves out the battle's system, as the port's escape
/// search does for the place a character leaves. port: a fleet with no
/// destination, or one that cannot move (speed 0 or no hyperdrive), stays;
/// the original destroys a unit that cannot move to another system
/// (`FUN_00555410`, slot `+0xac(5)`).
///
/// The loser's squadrons on the planet (`FUN_00503a50(system, 1, side)`)
/// are set Autorouting too and move the same way, on `day`. port: a
/// squadron with no destination, or whose destination planet another side
/// holds (`squadrons`' route check), stays. A squadron's speed is never 0
/// in the port (`movement::fighter_speed`).
/// port: the winner's planet squadrons go to `FUN_00556710` when
/// `FUN_00506e60` (a game flag, `+0xbc`) allows; that is not read.
pub fn withdraw_loser(
    world: &mut GameWorld,
    movement: &mut crate::movement::MovementState,
    squadrons: &mut crate::squadrons::SquadronTransits,
    system: SystemKey,
    winner: Option<crate::dat::Faction>,
    day: u64,
) -> Vec<Withdrawal> {
    use crate::dat::Faction;

    // With no winner and both sides still present, the system's last
    // arrived side wins (+0x78 bits 6..7: the Alliance for 1, else the
    // Empire).
    let winner = winner.or_else(|| {
        let (alliance, empire) = crate::squadrons::sides_present(world, system);
        (alliance && empire).then(|| {
            match world.systems.get(system).and_then(|sys| sys.last_arrival) {
                Some(Faction::Alliance) => Faction::Alliance,
                _ => Faction::Empire,
            }
        })
    });
    let loser_is_alliance = match winner {
        Some(Faction::Alliance) => false,
        Some(Faction::Empire) => true,
        Some(Faction::Neutral) | None => return Vec::new(),
    };
    let Some(origin) = world
        .systems
        .get(system)
        .map(|sys| (i64::from(sys.x), i64::from(sys.y)))
    else {
        return Vec::new();
    };
    let side = if loser_is_alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    let fleets: Vec<FleetKey> = world.systems[system]
        .fleets
        .iter()
        .copied()
        .filter(|&fleet| {
            world
                .fleets
                .get(fleet)
                .is_some_and(|value| value.is_alliance == loser_is_alliance)
                && !movement.is_in_transit(fleet)
        })
        .collect();
    let mut withdrawals = Vec::new();
    for fleet in fleets {
        let Some(destination) = crate::missions::nearest_place(world, side, origin, Some(system))
        else {
            continue;
        };
        let Some(value) = world.fleets.get(fleet) else {
            continue;
        };
        if crate::movement::fleet_speed(value, world).is_none_or(|speed| speed == 0) {
            continue;
        }
        let Some(transit_ticks) =
            crate::movement::fleet_transit_ticks(value, world, system, destination)
        else {
            continue;
        };
        if crate::movement::begin_fleet_transit(movement, world, fleet, destination, transit_ticks)
        {
            withdrawals.push(Withdrawal {
                fleet,
                destination,
                transit_ticks,
            });
        }
    }
    // The squadrons' move refuses the winner's entries (another side) and
    // emptied ones; it moves the loser's.
    let planet: Vec<crate::world::FighterEntry> = world.systems[system].fighters.clone();
    for entry in planet {
        let Some(destination) = crate::missions::nearest_place(world, side, origin, Some(system))
        else {
            continue;
        };
        let _ = squadrons.move_squadrons(
            world,
            loser_is_alliance,
            entry.class,
            entry.count,
            crate::squadrons::SquadronPlace::Planet(system),
            crate::squadrons::SquadronPlace::Planet(destination),
            day,
        );
    }
    withdrawals
}

/// Whether the player's Battle Alert offers Retreat (`FUN_004865b0` passes
/// `FUN_004fcf50` -> `FUN_00516730`'s answer to the dialog, which enables
/// button `0x67` when it is set). All three must hold:
///
/// - no capital ship of the enemy's fleets at the system has a gravity
///   well projector (slot `+0x254`, class `+0xcc`, CAPSHPSD `+0xa4`, less
///   slot `+0x280`, which is 0);
/// - one of the side's units there can move (slot `+0x34`): a capital ship
///   in its fleets ([`crate::movement::ship_speed`]), else a squadron of
///   its on the planet ([`crate::movement::fighter_speed`]);
/// - the autoroute search (`FUN_005545e0` with no container candidate)
///   finds that unit a system.
///
/// port: the search is [`crate::missions::nearest_place`], leaving out
/// the battle's system; it does not depend on the unit.
#[must_use]
pub fn retreat_offered(world: &GameWorld, system: SystemKey, alliance: bool) -> bool {
    use crate::dat::Faction;
    let Some(place) = world.systems.get(system) else {
        return false;
    };
    let ships = |side: bool| {
        place
            .fleets
            .iter()
            .filter_map(|&key| world.fleets.get(key))
            .filter(move |fleet| fleet.is_alliance == side)
            .flat_map(|fleet| fleet.capital_ships.iter())
            .filter(|ship| ship.alive)
            .filter_map(|ship| Some((ship, world.capital_ship_classes.get(ship.class)?)))
    };
    if ships(!alliance).any(|(_, class)| class.gravity_well_projector != 0) {
        return false;
    }
    let ship_moves =
        ships(alliance).any(|(ship, class)| crate::movement::ship_speed(world, class, ship) != 0);
    let squadron_moves = || {
        place.fighters.iter().any(|entry| {
            entry.count > 0
                && world.fighter_classes.get(entry.class).is_some_and(|class| {
                    class.is_alliance == alliance
                        && crate::movement::fighter_speed(world, class) != 0
                })
        })
    };
    if !ship_moves && !squadron_moves() {
        return false;
    }
    let side = if alliance {
        Faction::Alliance
    } else {
        Faction::Empire
    };
    crate::missions::nearest_place(
        world,
        side,
        (i64::from(place.x), i64::from(place.y)),
        Some(system),
    )
    .is_some()
}

impl AbstractBattle {
    /// The battle at `system` (`FUN_0040b0e0`): each side's retreat order
    /// (`FUN_005afb30`) and the system's last arrived side.
    #[must_use]
    pub fn at_system(
        world: &GameWorld,
        system: SystemKey,
        session: BattleSession,
        retreat_allowed: [bool; 2],
    ) -> Self {
        let mut battle = Self::new(session);
        battle.retreat_allowed = retreat_allowed;
        battle.last_arrival = world.systems.get(system).and_then(|sys| sys.last_arrival);
        battle.trench_commander = trench_commander_term(world, system);
        battle
    }
}

/// `FUN_005afc00(0)`: the trench run's commander term for the Alliance at
/// `system`. The battle's `+0x1c` is `FUN_00509530(system, 1)`:
///
/// - first the system's own side's Commander (`+0x96` = 1) not aboard a
///   fleet (`FUN_00509330`, [`crate::character_command::system_holder`]);
///   that side is the system's, not the battle side's;
/// - then each Alliance fleet at the system (`FUN_004ffef0(system, 1, 1)`):
///   its Commander (`FUN_004fd790`, `FUN_005006f0`) replaces the one held
///   when its combat (slot `+0x1f0`) is higher.
///
/// The battle person matching `+0x1c` is registered at `+0x254`
/// (`FUN_0040c200` `+0x58`, `FUN_005af440`), and its `+0x50` is combat / 20
/// (`FUN_0040d680`). The term is that minus -1.0 (`DAT_0066c3e4`), clamped to
/// 1.0..9.0; with no such person it is 1.0. `FUN_0040c200` flags only an
/// Alliance person against `+0x1c` (an Imperial one against `+0x24`), so
/// an Imperial pick gives 1.0 too.
///
/// hyp: every character at the battle has a battle person (`FUN_0040c200`'s
/// caller is not read), so the commander always counts.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "FUN_0040d680 loads the integer combat onto the FPU (fildl)."
)]
pub fn trench_commander_term(world: &GameWorld, system: SystemKey) -> f32 {
    use crate::character_command::{fleet_holder, system_holder, CommandKind};

    let combat = |key| {
        world
            .characters
            .get(key)
            .map_or(0, |c| c.effective_skill(crate::world::Skill::Combat))
    };
    let mut held = system_holder(world, system, CommandKind::Commander);
    let fleets = world
        .systems
        .get(system)
        .map_or(&[][..], |sys| &sys.fleets[..]);
    for &fleet in fleets {
        if !world
            .fleets
            .get(fleet)
            .is_some_and(|value| value.is_alliance)
        {
            continue;
        }
        if let Some(candidate) = fleet_holder(world, fleet, CommandKind::Commander) {
            if held.is_none_or(|current| combat(candidate) > combat(current)) {
                held = Some(candidate);
            }
        }
    }
    // FUN_0040c200 sets +0x58 only on an Alliance person matching +0x1c, so
    // an Imperial pick there leaves the Alliance without a commander.
    held.filter(|&key| world.characters.get(key).is_some_and(|c| c.is_alliance))
        .map_or(TRENCH_RUN_COMMANDER_FLOOR, |key| {
            (combat(key) as f32 / BATTLE_PERSON_COMBAT_DIVISOR + 1.0)
                .clamp(TRENCH_RUN_COMMANDER_FLOOR, TRENCH_RUN_COMMANDER_CAP)
        })
}

/// Whether a battle changed `before`: its hull, or its hit counters
/// (`FUN_0040cfc0` writes both). An unchanged ship has no event.
fn ship_changed(before: &crate::world::ShipInstance, hull_after: i32, hits_after: u32) -> bool {
    hull_after != before.hull_current || hits_after != before.subsystem_hits
}

/// The fleets of `lead`'s side at `system`, `lead` first: a battle takes
/// every fleet of both sides there (`FUN_0040c780` applies its result to
/// each, `FUN_004ffe70`), as the headless campaign's
/// `resolve_system_space_combat` does.
#[must_use]
pub fn side_fleets(world: &GameWorld, system: SystemKey, lead: FleetKey) -> Vec<FleetKey> {
    let Some(side) = world.fleets.get(lead).map(|fleet| fleet.is_alliance) else {
        return Vec::new();
    };
    let mut fleets = vec![lead];
    if let Some(value) = world.systems.get(system) {
        fleets.extend(value.fleets.iter().copied().filter(|&key| {
            key != lead
                && world
                    .fleets
                    .get(key)
                    .is_some_and(|fleet| fleet.is_alliance == side)
        }));
    }
    fleets
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
            fleet: FleetKey::default(),
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
            fleet: FleetKey::default(),
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

    fn group(side: u8, id: u8) -> AbstractGroup {
        AbstractGroup {
            side,
            fighters: false,
            id,
        }
    }

    // FUN_005ed170: hit points (hull × 0.65 + shields) over the running
    // rate less the target's shield recharge; -1 when the rate cannot win.
    #[test]
    fn a_score_is_the_seconds_to_wear_the_target_down() {
        let battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        ));
        let attackers = [AbstractUnit::Capital(0)];
        let target = AbstractUnit::Capital(1);
        let rate = damage_rate(&battle.session, attackers[0], target);
        let mut running = 0.0;
        let score = battle.score(target, &attackers, &mut running);
        assert!((running - rate).abs() < 1e-6);
        // 100 hull × 0.65 + 50 shield, recharge 1.0; both at the origin.
        assert!((score - 115.0 / (rate - 1.0)).abs() < 1e-3, "{score}");
        let mut weak = 0.5;
        let mut empty = 0.0;
        assert_eq!(battle.score(target, &[], &mut empty), ABSTRACT_NO_SCORE);
        let mut ion_only = battle.clone();
        ion_only.session.ships[0].weapon_arcs[0] = TacticalWeaponArc::default();
        assert_eq!(
            ion_only.score(target, &attackers, &mut weak),
            ABSTRACT_NO_SCORE
        );
    }

    // FUN_005ed170: a distant target adds 0.001 × its travel seconds, at
    // least one.
    #[test]
    fn a_distant_target_adds_its_travel_time() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        ));
        let attackers = [AbstractUnit::Capital(0)];
        let target = AbstractUnit::Capital(1);
        let near = battle.score(target, &attackers, &mut 0.0);
        battle.session.ships[1].source_position.z = 40.0;
        let far = battle.score(target, &attackers, &mut 0.0);
        // 40 units at engine power 2: 20 seconds.
        assert!((far - near - 20.0 * 0.001).abs() < 1e-5, "{far} {near}");
        battle.session.ships[1].source_position.z = 1.0;
        let close = battle.score(target, &attackers, &mut 0.0);
        assert!((close - near - 0.001).abs() < 1e-5, "{close} {near}");
    }

    // FUN_005d1a70 and FUN_005ed8e0: targets run from the highest priority,
    // 15 × strength; FUN_005ed550 skips an unarmed one.
    #[test]
    fn the_strongest_armed_enemy_is_taken_first() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 5.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        ));
        for index in 1..4 {
            battle.session.ships[index].task_force = index as u8;
        }
        assert!(
            target_priority(&battle.session, AbstractUnit::Capital(2))
                > target_priority(&battle.session, AbstractUnit::Capital(1))
        );
        assert!(!target_eligible(&battle.session, AbstractUnit::Capital(3)));
        battle.assign_targets(true);
        assert_eq!(battle.targets[&group(0, 0)], AbstractUnit::Capital(2));
    }

    // FUN_005ed550: an unarmed ship is skipped for the next eligible target
    // even on the ship list; with only unarmed ships, FUN_005ec740 falls back
    // to the first of the list. A fighter group is armed by its hull
    // (FUN_005b9c00).
    #[test]
    fn an_unarmed_ship_is_skipped_and_only_a_fallback() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            vec![fighter(false, TacticalWeaponArc::default())],
        ));
        assert!(target_eligible(&battle.session, AbstractUnit::Fighter(0)));
        battle.assign_targets(true);
        assert_eq!(battle.targets[&group(0, 0)], AbstractUnit::Fighter(0));

        let mut only_ships = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        ));
        only_ships.assign_targets(true);
        assert_eq!(only_ships.targets[&group(0, 0)], AbstractUnit::Capital(1));
    }

    // FUN_005ec740: three task forces too weak alone combine. The walk from
    // the tail links the tail and the middle once the head's rate plus theirs
    // scores under 180; the head is counted but not linked, so it stays idle
    // and, alone for the next target, is linked to it.
    #[test]
    fn weak_task_forces_combine_and_the_head_moves_on() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 1.2),
                ship(true, TacticalWeaponArc::new(0, 0, 4), 1.1),
                ship(true, TacticalWeaponArc::new(0, 0, 4), 1.0),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
            ],
            Vec::new(),
        ));
        battle.session.ships[1].task_force = 1;
        battle.session.ships[2].task_force = 2;
        for index in [3, 4] {
            battle.session.ships[index].shield_recharge_rate = 0.0;
        }
        // Rates 0.32, 0.293 and 0.267 per second against 115 points: alone
        // over 180 s; head and tail 196 s; all three 131 s.
        let groups = battle.groups(true);
        battle.pair(
            groups,
            &[AbstractUnit::Capital(3), AbstractUnit::Capital(4)],
        );
        assert_eq!(battle.targets[&group(0, 1)], AbstractUnit::Capital(3));
        assert_eq!(battle.targets[&group(0, 2)], AbstractUnit::Capital(3));
        assert_eq!(battle.targets[&group(0, 0)], AbstractUnit::Capital(4));
    }

    // FUN_005ed5f0: a task force keeps an eligible target while its score
    // stays under 300; a target with no weapons is dropped.
    #[test]
    fn a_task_force_keeps_its_target_while_the_score_is_under_300() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
            ],
            Vec::new(),
        ));
        battle.targets.insert(group(0, 0), AbstractUnit::Capital(1));
        let groups = battle.groups(true);
        assert!(battle.keeps_target(group(0, 0), &groups));
        battle.session.ships[1].hull_current = 100_000;
        assert!(!battle.keeps_target(group(0, 0), &groups));
        battle.session.ships[1].hull_current = 100;
        battle.session.ships[1].weapon_recharge_rate = 0.0;
        assert!(!battle.keeps_target(group(0, 0), &groups));
    }

    // FUN_005c9200 with FUN_005b7ce0: a laser-armed leader takes the
    // fighter list, any other leader the ship list.
    #[test]
    fn laser_leaders_take_the_fighter_list_and_others_the_ship_list() {
        let mut lasers = ship(true, TacticalWeaponArc::new(1, 0, 4), 30.0);
        lasers.task_force = 1;
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                lasers,
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
            ],
            vec![fighter(false, TacticalWeaponArc::new(2, 0, 0))],
        ));
        battle.assign_targets(true);
        assert_eq!(battle.targets[&group(0, 0)], AbstractUnit::Capital(2));
        assert_eq!(battle.targets[&group(0, 1)], AbstractUnit::Fighter(0));
    }

    // FUN_005ed8e0: 15 (DAT_0066da7c) × strength less 0.01 (DAT_0066daa0) ×
    // the shield-generator ratio (FUN_005b1710: recharge over its base, 0
    // without one) and the hull ratio (FUN_005b1640).
    #[test]
    fn the_priority_is_fifteen_times_strength_less_the_ratios() {
        let mut battle = session(
            vec![
                ship(false, TacticalWeaponArc::new(0, 0, 4), 10.0),
                ship(false, TacticalWeaponArc::new(0, 0, 4), 10.0),
            ],
            vec![fighter(false, TacticalWeaponArc::new(2, 0, 0))],
        );
        battle.ships[0].shield_recharge_rate = 2.0;
        battle.ships[0].hull_current = 50;
        battle.ships[0].subsystem_damage.shields = 1;
        battle.ships[1].shield_recharge_rate = 0.0;
        battle.fighters[0].hull_current = 48.0;
        let unit = AbstractUnit::Capital(0);
        let shield_ratio = battle.ships[0].effective_shield_recharge() / 2.0;
        let expected = 15.0 * unit_strength(&battle, unit) - 0.01 * shield_ratio - 0.01 * 0.5;
        assert!((target_priority(&battle, unit) - expected).abs() < 1e-5);
        let bare = AbstractUnit::Capital(1);
        let expected = 15.0 * unit_strength(&battle, bare) - 0.01;
        assert!((target_priority(&battle, bare) - expected).abs() < 1e-5);
        let squadron = AbstractUnit::Fighter(0);
        let expected = 15.0 * unit_strength(&battle, squadron) - 0.01;
        assert!((target_priority(&battle, squadron) - expected).abs() < 1e-5);
        assert!(shield_ratio > 0.0 && shield_ratio < 0.5);
    }

    // FUN_005ed5f0: a score of 300 or more alone is kept when the side's
    // other task forces on the same target bring it under 300; task forces
    // on another target do not count.
    #[test]
    fn partners_on_the_same_target_keep_it() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 1.0),
                ship(true, TacticalWeaponArc::new(0, 0, 4), 10.0),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
            ],
            Vec::new(),
        ));
        battle.session.ships[1].task_force = 1;
        battle.session.ships[2].shield_recharge_rate = 0.0;
        battle.session.ships[3].task_force = 1;
        battle.targets.insert(group(0, 0), AbstractUnit::Capital(2));
        battle.targets.insert(group(0, 1), AbstractUnit::Capital(3));
        let groups = battle.groups(true);
        // Alone: 115 / 0.267 > 300.
        assert!(!battle.keeps_target(group(0, 0), &groups));
        battle.targets.insert(group(0, 1), AbstractUnit::Capital(2));
        assert!(battle.keeps_target(group(0, 0), &groups));
    }

    // FUN_005ed170: the travel term measures from the task force to the
    // target along every axis.
    #[test]
    fn the_travel_term_measures_from_the_leader_on_every_axis() {
        let battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        ));
        let attackers = [AbstractUnit::Capital(0)];
        let target = AbstractUnit::Capital(1);
        let base = battle.score(target, &attackers, &mut 0.0);
        for axis in 0..3 {
            let mut moved = battle.clone();
            let set = |at: &mut TacticalWorldPosition, value: f32| match axis {
                0 => at.x = value,
                1 => at.y = value,
                _ => at.z = value,
            };
            set(&mut moved.session.ships[0].source_position, 6.0);
            set(&mut moved.session.ships[1].source_position, 10.0);
            // 4 units at engine power 2: 2 seconds.
            let score = moved.score(target, &attackers, &mut 0.0);
            assert!((score - base - 0.002).abs() < 1e-5, "axis {axis}: {score}");
        }
        assert_eq!(ABSTRACT_NO_SCORE, -1.0);
    }

    fn world_with_battle() -> (GameWorld, SystemKey, FleetKey, FleetKey, FleetKey) {
        use crate::dat::ExplorationStatus;
        use crate::ids::SectorKey;
        use crate::world::{
            CapitalShipClass, ControlKind, FighterClass, FighterEntry, Fleet, ShipInstance, System,
        };
        let mut world = GameWorld::default();
        let system = world.systems.insert(System {
            last_arrival: None,
            dat_id: DatId::new(0x9000_0001),
            name: "Test".into(),
            sector: SectorKey::default(),
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
        let strong = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x3000_0001),
            is_alliance: true,
            hull: 2_000,
            turbolaser_fore: 10,
            laser_cannon_fore: 10,
            weapon_recharge_rate: 40,
            ..CapitalShipClass::default()
        });
        let weak = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x3000_0002),
            is_empire: true,
            hull: 40,
            turbolaser_fore: 1,
            weapon_recharge_rate: 1,
            ..CapitalShipClass::default()
        });
        let craft = world.fighter_classes.insert(FighterClass {
            dat_id: DatId::new(0x2000_0001),
            is_empire: true,
            squadron_size: 1,
            laser_cannon_fore: 1,
            ..FighterClass::default()
        });
        let escort = world.fighter_classes.insert(FighterClass {
            dat_id: DatId::new(0x2000_0002),
            is_alliance: true,
            squadron_size: 12,
            ..FighterClass::default()
        });
        let mut dead = ShipInstance::new(weak, 0, false);
        dead.alive = false;
        let alliance = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![ShipInstance::new(strong, 2_000, true)],
            fighters: vec![FighterEntry {
                class: escort,
                count: 2,
            }],
            characters: vec![],
            is_alliance: true,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        let ships = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![
                dead,
                ShipInstance::new(weak, 40, false),
                ShipInstance::new(weak, 40, false),
            ],
            fighters: vec![],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        let squadrons = world.fleets.insert(Fleet {
            location: system,
            capital_ships: vec![],
            fighters: vec![FighterEntry {
                class: craft,
                count: 2,
            }],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets = vec![alliance, ships, squadrons];
        (world, system, alliance, ships, squadrons)
    }

    // FUN_0040c780: each lost hull and squadron goes back to its own fleet.
    // A hull's index counts the fleet's live hulls, last first, and an
    // untouched hull has no event.
    #[test]
    fn a_strategic_battle_reports_each_fleets_losses() {
        let (world, system, alliance, ships, squadrons) = world_with_battle();
        let (result, outcome) = resolve_space_battle(
            &world,
            system,
            &[alliance],
            &[ships, squadrons],
            7,
            3,
            [false, false],
        );
        assert_eq!(outcome.winner, CombatWinner::Attacker);
        assert_eq!(result.winner, CombatSide::Attacker);
        assert_eq!(result.attacker_fleet, alliance);
        assert_eq!(result.defender_fleet, ships);
        assert_eq!(result.tick, 3);
        let lost = |fleet: FleetKey| -> Vec<(usize, i32, i32)> {
            result
                .ship_damage
                .iter()
                .filter(|event| event.fleet == fleet)
                .map(|event| (event.ship_index, event.hull_before, event.hull_after))
                .collect()
        };
        assert_eq!(lost(ships), [(1, 40, 0), (0, 40, 0)]);
        let hit = lost(alliance);
        assert_eq!(hit.len(), 1);
        assert!(hit[0].0 == 0 && hit[0].2 < hit[0].1 && hit[0].2 > 0);
        // The Alliance's two squadrons both survive: no event.
        assert_eq!(result.fighter_losses.len(), 1);
        let squadron = &result.fighter_losses[0];
        assert_eq!(
            (
                squadron.fleet,
                squadron.fighter_index,
                squadron.squads_before,
                squadron.squads_after
            ),
            (squadrons, 0, 2, 0)
        );
    }

    // FUN_004362b0 -> FUN_005afb30: the side's answer reaches its commander's
    // +0x80. Allowed to leave, the beaten Empire's jumping hulls escape
    // (FUN_005d14d0 step 5) and keep their hull; without it they are
    // destroyed.
    #[test]
    fn a_retreat_order_reaches_the_battle() {
        let (mut world, system, alliance, ships, squadrons) = world_with_battle();
        for class in world.capital_ship_classes.values_mut() {
            if class.is_empire {
                class.hyperdrive = 1;
                class.sub_light_engine = 1;
            }
        }
        let lost = |allowed: [bool; 2]| {
            let (result, outcome) = resolve_space_battle(
                &world,
                system,
                &[alliance],
                &[ships, squadrons],
                7,
                3,
                allowed,
            );
            assert_eq!(outcome.winner, CombatWinner::Attacker);
            result
                .ship_damage
                .iter()
                .filter(|event| event.fleet == ships && event.hull_after == 0)
                .count()
        };
        assert_eq!(lost([false, false]), 2);
        assert!(lost([false, true]) < 2);
    }

    // F-024: no recovered battle code reads the Emperor (record 0x34000280).
    // The abstract step (FUN_005ae9c0, FUN_005d14d0, FUN_005b7780) and the
    // battle's build and result (FUN_0040b0e0, FUN_0040c780) have no
    // character term but the trench run's commander, so the Emperor aboard
    // an Imperial fleet changes nothing.
    #[test]
    fn the_emperor_aboard_changes_no_battle() {
        let (world, system, alliance, ships, squadrons) = world_with_battle();
        let fight = |world: &GameWorld| {
            resolve_space_battle(
                world,
                system,
                &[alliance],
                &[ships, squadrons],
                7,
                3,
                [false, false],
            )
        };
        let (without, outcome_without) = fight(&world);
        let mut with_emperor = world.clone();
        let emperor = with_emperor.characters.insert(crate::world::Character {
            dat_id: DatId::new(0x3400_0280),
            name: "Emperor Palpatine".into(),
            current_fleet: Some(ships),
            ..crate::world::Character::default()
        });
        with_emperor.fleets[ships].characters.push(emperor);
        let (with, outcome_with) = fight(&with_emperor);
        assert_eq!(outcome_with, outcome_without);
        let hulls = |result: &SpaceCombatResult| {
            result
                .ship_damage
                .iter()
                .map(|e| (e.fleet, e.ship_index, e.hull_before, e.hull_after))
                .collect::<Vec<_>>()
        };
        let squads = |result: &SpaceCombatResult| {
            result
                .fighter_losses
                .iter()
                .map(|e| (e.fleet, e.fighter_index, e.squads_before, e.squads_after))
                .collect::<Vec<_>>()
        };
        assert_eq!(hulls(&with), hulls(&without));
        assert_eq!(squads(&with), squads(&without));
        assert!(!hulls(&without).is_empty());
    }

    // FUN_0040b8a0 -> FUN_005b05c0: a battle unit starts from its ship's
    // +0x64 hit counters, the hyperdrive's as primary + backup; FUN_0040cfc0
    // writes them back with the hull.
    #[test]
    fn a_battle_carries_the_ships_hit_counters_both_ways() {
        use crate::world::Subsystem;
        let (mut world, system, alliance, ships, squadrons) = world_with_battle();
        {
            let ship = &mut world.fleets[alliance].capital_ships[0];
            ship.set_hits(Subsystem::Shield, 2);
            ship.set_hits(Subsystem::Weapon, 1);
            ship.set_hits(Subsystem::Speed, 3);
            ship.set_hits(Subsystem::PrimaryHyperdrive, 1);
            ship.set_hits(Subsystem::BackupHyperdrive, 1);
        }
        let FleetBattle { session, .. } = BattleSession::new_for_fleets(
            &world,
            system,
            &[alliance],
            &[ships, squadrons],
            true,
            3,
            7,
        );
        let unit = &session.ships[0];
        assert_eq!(
            (
                unit.subsystem_damage.shields,
                unit.subsystem_damage.weapons,
                unit.subsystem_damage.tractor,
                unit.subsystem_damage.engines,
                unit.subsystem_damage.hyperdrive
            ),
            (2, 1, 0, 3, 2)
        );
        let (result, _) = resolve_space_battle(
            &world,
            system,
            &[alliance],
            &[ships, squadrons],
            7,
            3,
            [false, false],
        );
        let event = result
            .ship_damage
            .iter()
            .find(|event| event.fleet == alliance)
            .expect("the Alliance ship took damage");
        let mut after = world.fleets[alliance].capital_ships[0].clone();
        after.subsystem_hits = event.hits_after;
        assert!(after.hits(Subsystem::Shield) >= 2);
        assert!(after.hits(Subsystem::Weapon) >= 1);
        assert!(after.hits(Subsystem::Speed) >= 3);
    }

    // FUN_0040cfc0 writes a ship's hull and counters; a ship whose hull and
    // counters are unchanged has no event.
    #[test]
    fn a_ship_changes_with_its_hull_or_its_hit_counters() {
        let mut ship = crate::world::ShipInstance::new(CapitalShipKey::default(), 50, true);
        ship.subsystem_hits = 0x10;
        assert!(!ship_changed(&ship, 50, 0x10));
        assert!(ship_changed(&ship, 40, 0x10));
        assert!(ship_changed(&ship, 50, 0x11));
        assert!(ship_changed(&ship, 40, 0x11));
    }

    // FUN_0040cfc0: the hyperdrive's hits fill the primary (if the class has
    // a hyperdrive, FUN_00500680), then the backup (FUN_00500690), one each.
    #[test]
    fn hyperdrive_hits_go_to_the_primary_then_the_backup() {
        use crate::world::{CapitalShipClass, ShipInstance, Subsystem};
        let split = |hyperdrive, hyperdrive_if_damaged, hits| {
            let class = CapitalShipClass {
                hyperdrive,
                hyperdrive_if_damaged,
                ..CapitalShipClass::default()
            };
            let mut unit = ship(true, TacticalWeaponArc::default(), 0.0);
            unit.subsystem_damage.hyperdrive = hits;
            unit.subsystem_damage.tractor = 2;
            let mut ship = ShipInstance::new(crate::ids::CapitalShipKey::default(), 1, true);
            ship.subsystem_hits = unit.strategic_hits(&class);
            assert_eq!(ship.hits(Subsystem::Tractor), 2);
            (
                ship.hits(Subsystem::PrimaryHyperdrive),
                ship.hits(Subsystem::BackupHyperdrive),
            )
        };
        assert_eq!(split(80, 120, 0), (0, 0));
        assert_eq!(split(80, 120, 1), (1, 0));
        assert_eq!(split(80, 120, 2), (1, 1));
        assert_eq!(split(0, 120, 1), (0, 1));
        assert_eq!(split(80, 0, 2), (1, 0));
    }

    // port: the seed comes from the first combat roll.
    #[test]
    fn the_seed_is_the_first_roll_scaled_to_32_bits() {
        assert_eq!(
            abstract_seed(&[0.5, 0.9]),
            (0.5 * f64::from(u32::MAX)) as u32
        );
        assert_eq!(abstract_seed(&[]), 0);
        assert_eq!(abstract_seed(&[2.0]), u32::MAX);
    }

    fn jumper(alliance: bool, fore: TacticalWeaponArc, recharge: f32) -> TacticalShip {
        let mut ship = ship(alliance, fore, recharge);
        ship.subsystem_capacity.hyperdrive = 1;
        ship
    }

    // FUN_005d19a0 and FUN_005d14d0 step 5: a side that may leave and falls
    // under 0.33 of the enemy's strength leaves; a ship with a working
    // hyperdrive escapes (state 9), one without is destroyed (FUN_005af7a0).
    #[test]
    fn a_losing_side_that_may_leave_jumps_out() {
        let mut slow = ship(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        slow.task_force = 1;
        let mut battle = AbstractBattle::new(session(
            vec![
                jumper(true, TacticalWeaponArc::new(0, 0, 1), 1.0),
                slow,
                ship(false, TacticalWeaponArc::new(0, 0, 4), 30.0),
            ],
            Vec::new(),
        ));
        battle.retreat_allowed[0] = true;
        assert!(battle.should_retreat(true));
        let outcome = battle.step().expect("the Alliance left");
        assert_eq!(outcome.winner, CombatWinner::Defender);
        assert!(battle.session.ships[0].retreated && battle.session.ships[0].alive);
        assert!(!battle.session.ships[1].alive);
        assert!(battle.retreat_done[0]);
        assert!(!battle.can_leave(true));
    }

    // FUN_005d0f50: without +0x80 a side cannot leave and fights on.
    #[test]
    fn a_side_not_allowed_to_leave_fights_on() {
        let mut battle = AbstractBattle::new(session(
            vec![
                jumper(true, TacticalWeaponArc::new(0, 0, 1), 1.0),
                ship(false, TacticalWeaponArc::new(0, 0, 4), 30.0),
            ],
            Vec::new(),
        ));
        assert!(!battle.can_leave(true));
        assert_eq!(battle.step(), None);
        assert!(!battle.session.ships[0].retreated);
        battle.retreat_allowed[0] = true;
        battle.session.ships[0].subsystem_capacity.hyperdrive = 0;
        assert!(!battle.can_leave(true));
    }

    // FUN_005d19a0 with +0x24 = 0x3ea8f5c3 (0.33): a ratio under it leaves.
    #[test]
    fn the_retreat_line_is_a_third_of_the_enemy_strength() {
        let line = |recharge: f32| {
            let mut battle = AbstractBattle::new(session(
                vec![
                    jumper(true, TacticalWeaponArc::new(0, 0, 1), recharge),
                    ship(false, TacticalWeaponArc::new(0, 0, 1), 10.0),
                ],
                Vec::new(),
            ));
            battle.retreat_allowed[0] = true;
            battle.should_retreat(true)
        };
        assert!(line(3.2));
        assert!(!line(3.4));
        assert!((ABSTRACT_RETREAT_RATIO - 0.33).abs() < 1e-6);
    }

    // FUN_005ae9c0: at the stalemate a weaker side that can leave is marked
    // (+0xb0) instead of destroyed, and leaves on the next step; its fighter
    // group goes with the carrier. Ion fire only reaches the shields, so
    // neither strength falls (0.27 against 0.53, above the 0.33 line).
    #[test]
    fn a_stalled_side_that_can_leave_withdraws_instead_of_dying() {
        let mut carrier = jumper(true, TacticalWeaponArc::new(0, 1, 0), 1.0);
        carrier.fighter_capacity = 1;
        let mut battle = AbstractBattle::new(session(
            vec![carrier, ship(false, TacticalWeaponArc::new(0, 1, 0), 2.0)],
            vec![fighter(true, TacticalWeaponArc::default())],
        ));
        battle.retreat_allowed[0] = true;
        assert!(!battle.should_retreat(true));
        battle.stalls = [1199.0, 1199.0];
        assert_eq!(battle.step(), None);
        assert!(battle.retreat_marked[0]);
        assert!(battle.session.ships[0].alive && !battle.session.ships[0].retreated);
        let outcome = battle.step().expect("the Alliance left");
        assert_eq!(outcome.winner, CombatWinner::Defender);
        assert!(battle.session.ships[0].retreated && battle.session.ships[0].alive);
        assert!(battle.session.fighters[0].squad_count > 0);
        assert!(battle.session.ships[1].alive);
    }

    // FUN_005d0f50: an operational Death Star lets the Empire leave without a
    // hyperdrive; it does nothing for the Alliance.
    #[test]
    fn the_death_star_lets_only_the_empire_leave() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::default(), 0.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        ));
        battle.retreat_allowed = [true, true];
        assert!(!battle.can_leave(false));
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
        assert!(battle.can_leave(false));
        assert!(!battle.can_leave(true));
    }

    /// One side's end state for [`the_result_words_name_the_winner`].
    #[derive(Clone, Copy)]
    struct End {
        /// `+0x84` == 5: the side's retreat ran.
        ran: bool,
        /// Its first capital ship escaped (state 9); else it was destroyed.
        jumped: bool,
        /// Its fighter group escaped with its craft (else it was destroyed).
        fighters_left: bool,
        /// Its second capital ship is still active (state 2).
        active: bool,
    }

    const GONE: End = End {
        ran: false,
        jumped: false,
        fighters_left: false,
        active: false,
    };
    const FIGHTING: End = End {
        active: true,
        ..GONE
    };
    const LEFT: End = End {
        ran: true,
        jumped: true,
        ..GONE
    };

    fn finish_with(alliance: End, empire: End, death_star: bool) -> CombatWinner {
        ended(alliance, empire, death_star).finish().winner
    }

    fn ended(alliance: End, empire: End, death_star: bool) -> AbstractBattle {
        let mut battle = AbstractBattle::new(session(
            vec![
                jumper(true, TacticalWeaponArc::new(0, 0, 1), 1.0),
                ship(true, TacticalWeaponArc::new(0, 0, 1), 1.0),
                jumper(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
            ],
            vec![
                fighter(true, TacticalWeaponArc::default()),
                fighter(false, TacticalWeaponArc::default()),
            ],
        ));
        for (side, end) in [(0, alliance), (1, empire)] {
            let first = side * 2;
            battle.retreat_done[side] = end.ran;
            if end.jumped {
                battle.session.ships[first].retreated = true;
            } else {
                battle.session.ships[first].alive = false;
            }
            battle.session.ships[first + 1].alive = end.active;
            let fighter = &mut battle.session.fighters[side];
            assert!(fighter.squad_count > 0);
            fighter.alive = false;
            if !end.fighters_left {
                fighter.squad_count = 0;
            }
        }
        if death_star {
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
        }
        battle
    }

    // FUN_005ae9c0 (0x5aec52) and FUN_0040c780: the result words decide the
    // winner (battle-flow.md, "The winner and the loser's withdrawal"). The
    // Alliance is the attacker here.
    #[test]
    fn the_result_words_name_the_winner() {
        use CombatWinner::{Attacker, Defender, Draw};
        // Word 1 alone: the Empire has no active unit.
        assert_eq!(finish_with(FIGHTING, GONE, false), Attacker);
        // A side that left has no active unit: word 3, the Empire wins.
        assert_eq!(finish_with(LEFT, FIGHTING, false), Defender);
        // The Alliance left and the Empire was destroyed: word 0 blocks
        // word 1, so word 3 alone gives the Empire the battle.
        assert_eq!(finish_with(LEFT, GONE, false), Defender);
        // Both destroyed: words 1 and 3 clear each other.
        assert_eq!(finish_with(GONE, GONE, false), Draw);
        // An operational Death Star keeps word 1 clear.
        assert_eq!(finish_with(FIGHTING, GONE, true), Draw);
        // The second pass: a side whose retreat ran with an escapee but a
        // unit still active wins on its word alone.
        let stayed = End {
            active: true,
            ..LEFT
        };
        assert_eq!(finish_with(stayed, FIGHTING, false), Attacker);
        assert_eq!(finish_with(FIGHTING, stayed, false), Defender);
        // Both retreats ran: the system's last arrived side (+0x78 bits
        // 6..7) wins, the Empire when none is recorded.
        assert_eq!(finish_with(LEFT, LEFT, false), Defender);
        for (arrival, winner) in [
            (Some(crate::dat::Faction::Alliance), Attacker),
            (Some(crate::dat::Faction::Empire), Defender),
            (None, Defender),
        ] {
            let mut battle = ended(LEFT, LEFT, false);
            battle.last_arrival = arrival;
            assert_eq!(battle.finish().winner, winner);
        }
    }

    // FUN_005ae9c0: words 0 and 2 need both the side's retreat (+0x84 == 5)
    // and an escaped unit (state 9), by the unit's side +0x168. An escaped
    // fighter group counts; an active ship or a destroyed group does not.
    #[test]
    fn a_retreat_word_needs_the_retreat_and_an_escapee() {
        use CombatWinner::{Attacker, Defender, Draw};
        let ran_no_escapee = End {
            ran: true,
            active: true,
            ..GONE
        };
        assert_eq!(finish_with(ran_no_escapee, FIGHTING, false), Draw);
        assert_eq!(finish_with(FIGHTING, ran_no_escapee, false), Draw);
        let escaped_no_retreat = End {
            jumped: true,
            active: true,
            ..GONE
        };
        assert_eq!(finish_with(escaped_no_retreat, FIGHTING, false), Draw);
        assert_eq!(finish_with(FIGHTING, escaped_no_retreat, false), Draw);
        let fighters_escaped = End {
            ran: true,
            fighters_left: true,
            active: true,
            ..GONE
        };
        assert_eq!(finish_with(fighters_escaped, FIGHTING, false), Attacker);
        assert_eq!(finish_with(FIGHTING, fighters_escaped, false), Defender);
    }

    fn withdrawal_world() -> (GameWorld, [SystemKey; 3], FleetKey, FleetKey) {
        use crate::dat::{ExplorationStatus, Faction};
        use crate::world::{CapitalShipClass, ControlKind, Fleet, ShipInstance, System};
        let mut world = GameWorld::default();
        let mut place = |x: u16, control: ControlKind| {
            world.systems.insert(System {
                last_arrival: None,
                dat_id: DatId::new(0x9000_0000 | u32::from(x)),
                name: format!("S{x}"),
                sector: crate::ids::SectorKey::default(),
                x,
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
                control,
            })
        };
        let battle = place(0, ControlKind::Uncontrolled);
        let near = place(30, ControlKind::Controlled(Faction::Empire));
        let far = place(90, ControlKind::Controlled(Faction::Empire));
        let class = world.capital_ship_classes.insert(CapitalShipClass {
            dat_id: DatId::new(0x3000_0003),
            hull: 10,
            ..CapitalShipClass::default()
        });
        let fleet = |world: &mut GameWorld, is_alliance: bool| {
            let key = world.fleets.insert(Fleet {
                location: battle,
                capital_ships: vec![ShipInstance::new(class, 10, false)],
                fighters: vec![],
                characters: vec![],
                is_alliance,
                has_death_star: false,
                special_forces: Vec::new(),
            });
            world.systems[battle].fleets.push(key);
            key
        };
        let alliance = fleet(&mut world, true);
        let empire = fleet(&mut world, false);
        (world, [battle, near, far], alliance, empire)
    }

    // FUN_00516730 (FUN_004865b0 -> FUN_00488d20 +0x50 -> FUN_0044f860):
    // the Battle Alert offers Retreat unless an enemy ship has a gravity
    // well projector (slot +0x254, class +0xcc), no unit of the side can
    // move, or the autoroute search finds no system.
    #[test]
    fn retreat_is_offered_unless_an_interdictor_or_nowhere_to_go_bars_it() {
        use crate::world::{CapitalShipClass, FighterClass, FighterEntry, ShipInstance};
        let (mut world, [battle, near, far], alliance, empire) = withdrawal_world();
        assert!(retreat_offered(&world, battle, true));
        assert!(retreat_offered(&world, battle, false));

        // An Interdictor in the Alliance's fleet holds the Empire only.
        let interdictor = world.capital_ship_classes.insert(CapitalShipClass {
            gravity_well_projector: 1,
            hull: 10,
            ..CapitalShipClass::default()
        });
        world.fleets[alliance]
            .capital_ships
            .push(ShipInstance::new(interdictor, 10, false));
        assert!(!retreat_offered(&world, battle, false));
        assert!(retreat_offered(&world, battle, true));
        // A destroyed Interdictor holds nobody.
        world.fleets[alliance].capital_ships[1].alive = false;
        assert!(retreat_offered(&world, battle, false));

        // With no ship of its own there, only a squadron on the planet can
        // leave.
        world.fleets[empire].capital_ships.clear();
        assert!(!retreat_offered(&world, battle, false));
        let tie = world.fighter_classes.insert(FighterClass {
            is_alliance: false,
            ..FighterClass::default()
        });
        // port: an emptied entry is no squadron.
        world.systems[battle].fighters.push(FighterEntry {
            class: tie,
            count: 0,
        });
        assert!(!retreat_offered(&world, battle, false));
        world.systems[battle].fighters[0].count = 1;
        assert!(retreat_offered(&world, battle, false));
        // The Alliance's tab does not count the Empire's squadron.
        world.fleets[alliance].capital_ships.clear();
        assert!(!retreat_offered(&world, battle, true));

        // With no other system, the search finds nowhere to go.
        world.systems.remove(near);
        world.systems.remove(far);
        assert!(!retreat_offered(&world, battle, false));
        assert!(!retreat_offered(&world, SystemKey::default(), false));
    }

    // FUN_00516ef0 sets the loser's fleets Autorouting; FUN_00515480 moves
    // each to the autoroute search's destination (FUN_005545e0): the
    // nearest system of its side. The winner stays.
    #[test]
    fn the_losers_fleets_leave_for_their_nearest_system() {
        let (mut world, [battle, near, _], alliance, empire) = withdrawal_world();
        let mut movement = crate::movement::MovementState::new();
        let sent = withdraw_loser(
            &mut world,
            &mut movement,
            &mut Default::default(),
            battle,
            Some(crate::dat::Faction::Alliance),
            0,
        );
        assert_eq!(sent.len(), 1);
        assert_eq!((sent[0].fleet, sent[0].destination), (empire, near));
        assert_eq!(
            movement.get(empire).map(|order| order.destination),
            Some(near)
        );
        assert!(!movement.is_in_transit(alliance));
        assert_eq!(world.systems[battle].fleets, [alliance]);
    }

    // FUN_0040b0e0 / FUN_005afb30: a battle carries the sides' orders and its
    // system's last arrived side.
    #[test]
    fn a_battle_takes_its_systems_last_arrived_side() {
        let (mut world, [battle, ..], alliance, _) = withdrawal_world();
        world.systems[battle].last_arrival = Some(crate::dat::Faction::Alliance);
        officer(
            &mut world,
            true,
            crate::character_command::CommandKind::Commander,
            100,
            Some(alliance),
            battle,
        );
        let fought = AbstractBattle::at_system(
            &world,
            battle,
            session(Vec::new(), Vec::new()),
            [true, false],
        );
        assert_eq!(fought.last_arrival, Some(crate::dat::Faction::Alliance));
        assert_eq!(fought.retreat_allowed, [true, false]);
        assert_eq!(fought.trench_commander, 6.0);
    }

    // FUN_00516ef0: the loser's squadrons on the planet (FUN_00503a50) are
    // set Autorouting and leave for their side's nearest system; the
    // winner's stay.
    #[test]
    fn the_losers_planet_squadrons_leave_too() {
        let (mut world, [battle, near, _], _, _) = withdrawal_world();
        let mut squadron = |is_alliance: bool| {
            let class = world.fighter_classes.insert(crate::world::FighterClass {
                is_alliance,
                is_empire: !is_alliance,
                ..crate::world::FighterClass::default()
            });
            crate::squadrons::add(&mut world.systems[battle].fighters, class, 2);
            class
        };
        let tie = squadron(false);
        let x_wing = squadron(true);
        let mut movement = crate::movement::MovementState::new();
        let mut squadrons = crate::squadrons::SquadronTransits::default();
        withdraw_loser(
            &mut world,
            &mut movement,
            &mut squadrons,
            battle,
            Some(crate::dat::Faction::Alliance),
            5,
        );
        assert_eq!(crate::squadrons::on_planet(&world, battle, false), 0);
        assert_eq!(crate::squadrons::on_planet(&world, battle, true), 2);
        let (arrived, lost) = squadrons.advance(&mut world, u64::MAX);
        assert!(lost.is_empty());
        assert_eq!(arrived.len(), 1);
        assert_eq!((arrived[0].class, arrived[0].count), (tie, 2));
        assert_eq!(crate::squadrons::on_planet(&world, near, false), 2);
        assert_ne!(tie, x_wing);
    }

    // port: a squadron cannot land on a planet another side holds, so with
    // only such places left it stays.
    #[test]
    fn a_loser_squadron_with_only_enemy_planets_stays() {
        let (mut world, [battle, near, far], _, _) = withdrawal_world();
        for system in [near, far] {
            world.systems[system].control =
                crate::world::ControlKind::Controlled(crate::dat::Faction::Alliance);
        }
        let tie = world.fighter_classes.insert(crate::world::FighterClass {
            is_empire: true,
            ..crate::world::FighterClass::default()
        });
        crate::squadrons::add(&mut world.systems[battle].fighters, tie, 1);
        let mut squadrons = crate::squadrons::SquadronTransits::default();
        withdraw_loser(
            &mut world,
            &mut crate::movement::MovementState::new(),
            &mut squadrons,
            battle,
            Some(crate::dat::Faction::Alliance),
            5,
        );
        assert_eq!(crate::squadrons::on_planet(&world, battle, false), 1);
    }

    // FUN_00516ef0: with no winner and both sides present, the system's
    // last arrived side (+0x78 bits 6..7) holds it and the other leaves;
    // the Empire holds it when none is recorded.
    #[test]
    fn without_a_winner_the_last_arrived_side_holds_the_system() {
        for (arrival, leaver) in [
            (Some(crate::dat::Faction::Alliance), false),
            (Some(crate::dat::Faction::Empire), true),
            (None, true),
        ] {
            let (mut world, [battle, ..], alliance, empire) = withdrawal_world();
            world.systems[battle].last_arrival = arrival;
            let mut movement = crate::movement::MovementState::new();
            let sent = withdraw_loser(
                &mut world,
                &mut movement,
                &mut Default::default(),
                battle,
                None,
                0,
            );
            let gone = if leaver { alliance } else { empire };
            assert_eq!(sent.iter().map(|w| w.fleet).collect::<Vec<_>>(), [gone]);
        }
    }

    // FUN_00516ef0: with no winner and one side gone, nobody leaves.
    #[test]
    fn a_battle_without_a_winner_or_a_second_side_sends_nobody_away() {
        let (mut world, [battle, ..], alliance, empire) = withdrawal_world();
        world.systems[battle]
            .fleets
            .retain(|&fleet| fleet != empire);
        let mut movement = crate::movement::MovementState::new();
        assert!(withdraw_loser(
            &mut world,
            &mut movement,
            &mut Default::default(),
            battle,
            None,
            0
        )
        .is_empty());
        assert!(movement.is_empty());
        assert_eq!(world.systems[battle].fleets, [alliance]);
    }

    fn imperial_death_star(shielded: bool) -> crate::tactical::TacticalDeathStar {
        crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition::ORIGIN,
            hull: ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE,
            laser_charge: 0.0,
            shielded,
            destroyed: false,
            action_committed: false,
        }
    }

    /// The Empire's Death Star (also a capital ship, as the port keeps it)
    /// against Alliance fighter groups of the given maneuverability and
    /// torpedo rating; the Empire has one fighter group.
    fn trench_battle(
        alliance_groups: usize,
        maneuverability: u32,
        torpedoes: u32,
    ) -> AbstractBattle {
        let mut death_star_ship = ship(false, TacticalWeaponArc::default(), 0.0);
        death_star_ship.death_star_resource =
            Some(crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE);
        let mut fighters = vec![fighter(false, TacticalWeaponArc::default())];
        for _ in 0..alliance_groups {
            let mut group = fighter(true, TacticalWeaponArc::default());
            group.maneuverability = maneuverability;
            group.torpedo_strength = torpedoes;
            fighters.push(group);
        }
        let mut battle = AbstractBattle::new(session(vec![death_star_ship], fighters));
        battle.session.death_star = Some(imperial_death_star(false));
        battle
    }

    // FUN_005ba7f0 with the abstract dt of 1.0 (DAT_0066badc): the charge
    // grows by 1/3 x 0.001 a step; at 100 (DAT_0066d0f4) the Empire destroys
    // the first capital ship of its enemy list (+0x90) and the charge empties
    // (FUN_005ba710).
    #[test]
    fn the_superlaser_charges_a_three_thousandth_a_step_and_fires_when_full() {
        let mut battle = AbstractBattle::new(session(
            vec![
                ship(true, TacticalWeaponArc::default(), 0.0),
                ship(false, TacticalWeaponArc::default(), 0.0),
            ],
            Vec::new(),
        ));
        battle.session.death_star = Some(imperial_death_star(true));
        battle.death_star_step(false);
        let charge = battle.session.death_star.unwrap().laser_charge;
        assert!((charge - 1.0 / 3000.0).abs() < 1e-9);
        assert!((ABSTRACT_DEATH_STAR_CHARGE_PER_STEP - 1.0 / 3000.0).abs() < 1e-9);
        assert!(battle.session.ships[0].alive);
        battle.session.death_star.as_mut().unwrap().laser_charge =
            ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE - ABSTRACT_DEATH_STAR_CHARGE_PER_STEP / 2.0;
        battle.death_star_step(false);
        assert!(!battle.session.ships[0].alive);
        assert!(battle.session.ships[1].alive);
        assert_eq!(battle.session.death_star.unwrap().laser_charge, 0.0);
        // The Alliance's step charges nothing.
        battle.death_star_step(true);
        assert_eq!(battle.session.death_star.unwrap().laser_charge, 0.0);
    }

    // FUN_005aefc0: a member that dodges (maneuverability 9 against 1..10)
    // and carries enough torpedoes always hits: the Death Star's hull and
    // charge empty (FUN_005ba8a0), and the port's Death Star ship falls.
    #[test]
    fn a_trench_run_that_hits_empties_the_death_star() {
        let mut battle = trench_battle(2, 9, 400);
        battle
            .session
            .ships
            .push(ship(false, TacticalWeaponArc::default(), 0.0));
        battle.death_star_step(true);
        let death_star = battle.session.death_star.unwrap();
        assert_eq!(death_star.hull, 0.0);
        assert!(!death_star.operational());
        assert!(!battle.session.ships[0].alive);
        // Only the Death Star falls.
        assert!(battle.session.ships[1].alive);
        assert!(battle.session.fighters[1..].iter().all(|f| f.alive));
    }

    // FUN_005aefc0: with no torpedoes the chance is (0 + 1.0) x 0.5, 0 as an
    // integer, so the run never hits; after 100 visits (0x5af047) every
    // member left is destroyed.
    #[test]
    fn a_trench_run_that_never_hits_loses_its_task_force() {
        let mut battle = trench_battle(2, 9, 0);
        battle.death_star_step(true);
        assert_eq!(
            battle.session.death_star.unwrap().hull,
            ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
        );
        assert!(battle.session.ships[0].alive);
        assert!(battle.session.fighters[0].alive);
        assert!(battle.session.fighters[1..].iter().all(|f| !f.alive));
    }

    // FUN_005aefc0: a member under the 1..10 roll is hit; with hull / 8 at
    // most 1 it is destroyed outright.
    #[test]
    fn a_trench_run_member_hit_with_little_hull_is_destroyed() {
        let mut battle = trench_battle(2, 0, 400);
        for fighter in &mut battle.session.fighters[1..] {
            fighter.hull_current = 15.0;
        }
        // Maneuverability 0 reads 1.0 (slot +0xbc), so only a roll of 1
        // spares a member; this stream's rolls do not.
        battle.death_star_step(true);
        assert!(battle.session.fighters[1..].iter().all(|f| !f.alive));
        assert_eq!(
            battle.session.death_star.unwrap().hull,
            ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
        );
    }

    /// The first draws of a stream seeded `seed`, each from 1 to its bound.
    fn draws(seed: u32, bounds: &[u32]) -> Vec<u32> {
        let mut rng = OriginalTacticalRng::new(seed);
        bounds
            .iter()
            .map(|&bound| rng.inclusive(1, bound))
            .collect()
    }

    /// The first seed whose draws pass `keep`.
    fn seed_where(bounds: &[u32], keep: impl Fn(&[u32]) -> bool) -> u32 {
        (1..10_000)
            .find(|&seed| keep(&draws(seed, bounds)))
            .expect("a seed")
    }

    /// One Alliance fighter group (index 1) set for a trench run.
    fn runner(seed: u32, maneuverability: u32, hull: f32, torpedoes: u32) -> AbstractBattle {
        let mut battle = trench_battle(1, maneuverability, torpedoes);
        battle.session.tactical_rng = OriginalTacticalRng::new(seed);
        let fighter = &mut battle.session.fighters[1];
        fighter.hull_current = hull;
        fighter.shield = 0.0;
        battle
    }

    // FUN_005aefc0 with FUN_005b23a0 / FUN_005b1770: the shot succeeds at or
    // under (torpedoes x 0.5333 x hull / full hull + 1.0) x 0.5 (DAT_0066c4cc,
    // DAT_0066c3f4, DAT_0066c3f0), as an integer, on a roll of 1..100.
    #[test]
    fn the_trench_run_shot_is_torpedoes_by_hull_share_plus_one_halved() {
        // Maneuverability 8 reads 9 (slot +0xbc): a roll under 10 spares.
        let seed = seed_where(&[10, 100], |d| d[0] <= 9 && (5..=90).contains(&d[1]));
        let shot = draws(seed, &[10, 100])[1];
        // Half the full hull of 96 halves the torpedo term.
        let chance = |torpedoes: u32| {
            ((48.0 * torpedoes as f32 * ORIGINAL_FIGHTER_TORPEDO_STRENGTH / 96.0 + 1.0) * 0.5)
                as u32
        };
        // The fewest torpedoes that give the chance: the 1.0 term decides it.
        let fewest = |wanted: u32| (0..5000).find(|&t| chance(t) == wanted).unwrap();
        let mut battle = runner(seed, 8, 48.0, fewest(shot));
        assert_eq!(
            battle.trench_run(&[1]),
            TrenchRun {
                visits: 1,
                success: true
            }
        );
        assert_eq!(battle.session.death_star.unwrap().hull, 0.0);
        let mut battle = runner(seed, 8, 48.0, fewest(shot) - 1);
        assert!(battle.trench_run(&[1]).visits > 1);
    }

    // FUN_005aefc0: a roll of 1..10 over the maneuverability hits; equal to it
    // spares. A hit member with hull / 8 at most 1 is destroyed.
    #[test]
    fn a_trench_run_roll_over_the_maneuverability_hits() {
        let seed = seed_where(&[10], |d| (3..=9).contains(&d[0]));
        let roll = draws(seed, &[10])[0];
        // Slot +0xbc reads the rating plus one.
        let mut battle = runner(seed, roll - 1, 15.0, 1000);
        assert_eq!(
            battle.trench_run(&[1]),
            TrenchRun {
                visits: 1,
                success: true
            }
        );
        let mut battle = runner(seed, roll - 2, 15.0, 1000);
        assert_eq!(
            battle.trench_run(&[1]),
            TrenchRun {
                visits: 1,
                success: false
            }
        );
        assert!(!battle.session.fighters[1].alive);
    }

    // FUN_005aefc0: a hit with more than 1 to roll deals (1..hull / 8) x 8
    // (DAT_0066c3dc) through FUN_005b54d0; a member left with hull goes on to
    // its shot.
    #[test]
    fn a_trench_run_hit_deals_eights_of_hull() {
        // Rolls: 10 (a hit against 9), the damage 1 of 1..2.
        let seed = seed_where(&[10, 2], |d| d[0] == 10 && d[1] == 1);
        let mut battle = runner(seed, 8, 16.0, 1000);
        let run = battle.trench_run(&[1]);
        assert!(run.success);
        assert_eq!(battle.session.fighters[1].hull_current, 8.0);
        let seed = seed_where(&[10, 2], |d| d[0] == 10 && d[1] == 2);
        let mut battle = runner(seed, 8, 16.0, 1000);
        assert!(!battle.trench_run(&[1]).success);
        assert!(!battle.session.fighters[1].alive);
    }

    // FUN_005aefc0 (0x5af047): after 100 visits every member left is
    // destroyed.
    #[test]
    fn a_trench_run_ends_after_a_hundred_visits() {
        let mut battle = runner(1, 8, 1.0e9, 0);
        battle.session.fighters[1].hull_max = 1.0e9;
        assert_eq!(
            battle.trench_run(&[1]),
            TrenchRun {
                visits: TRENCH_RUN_VISITS,
                success: false
            }
        );
        assert!(!battle.session.fighters[1].alive);
    }

    // FUN_005aefc0: the members take their turns in order.
    #[test]
    fn a_trench_run_visits_its_members_in_turn() {
        let mut battle = trench_battle(2, 8, 0);
        for fighter in &mut battle.session.fighters[1..] {
            fighter.hull_current = 1.0e9;
            fighter.hull_max = 1.0e9;
            fighter.shield = 0.0;
        }
        battle.session.fighters[2].torpedo_strength = 1000;
        assert_eq!(
            battle.trench_run(&[1, 2]),
            TrenchRun {
                visits: 2,
                success: true
            }
        );
    }

    /// A character of `side` with `command` and `combat`, aboard `fleet` or
    /// standing at `system`.
    fn officer(
        world: &mut GameWorld,
        alliance: bool,
        command: crate::character_command::CommandKind,
        combat: u32,
        fleet: Option<FleetKey>,
        system: SystemKey,
    ) -> crate::ids::CharacterKey {
        let key = world.characters.insert(crate::world::Character {
            is_alliance: alliance,
            is_empire: !alliance,
            command,
            combat: crate::world::SkillPair {
                base: combat,
                variance: 0,
            },
            current_fleet: fleet,
            current_system: Some(system),
            ..crate::world::Character::default()
        });
        if let Some(fleet) = fleet {
            world.fleets[fleet].characters.push(key);
        }
        key
    }

    // FUN_005afc00(0) over FUN_00509530 and FUN_0040d680: the Alliance's
    // best Commander (+0x96 = 1) by combat (slot +0x1f0) gives combat / 20
    // + 1, clamped to 1..9; with none, 1.
    #[test]
    fn the_trench_commander_is_the_best_alliance_commander_by_combat() {
        use crate::character_command::CommandKind::{Admiral, Commander};
        let (mut world, [battle, ..], alliance, _) = withdrawal_world();
        assert_eq!(trench_commander_term(&world, battle), 1.0);
        officer(&mut world, true, Admiral, 160, Some(alliance), battle);
        assert_eq!(trench_commander_term(&world, battle), 1.0);
        officer(&mut world, true, Commander, 100, Some(alliance), battle);
        assert_eq!(trench_commander_term(&world, battle), 6.0);
        let second = world.fleets.insert(world.fleets[alliance].clone());
        world.fleets[second].characters.clear();
        world.systems[battle].fleets.push(second);
        officer(&mut world, true, Commander, 120, Some(second), battle);
        assert_eq!(trench_commander_term(&world, battle), 7.0);
        officer(&mut world, true, Commander, 400, Some(second), battle);
        assert_eq!(trench_commander_term(&world, battle), 7.0);
        let third = world.fleets.insert(world.fleets[alliance].clone());
        world.fleets[third].characters.clear();
        world.systems[battle].fleets.push(third);
        officer(&mut world, true, Commander, 400, Some(third), battle);
        assert_eq!(
            trench_commander_term(&world, battle),
            TRENCH_RUN_COMMANDER_CAP
        );
    }

    #[test]
    fn a_sides_fleets_at_the_system_follow_its_lead_fleet() {
        // FUN_0040c780 (FUN_004ffe70): every fleet of the side fights.
        let (mut world, [battle, ..], alliance, empire) = withdrawal_world();
        let second = world.fleets.insert(world.fleets[alliance].clone());
        world.systems[battle].fleets.insert(0, second);
        assert_eq!(side_fleets(&world, battle, alliance), [alliance, second]);
        assert_eq!(side_fleets(&world, battle, empire), [empire]);
        assert!(side_fleets(&world, battle, FleetKey::default()).is_empty());
    }

    #[test]
    fn a_tactical_session_takes_the_alliance_commanders_rating() {
        // FUN_005ad7e0 reads the commander slot +0x950 that FUN_005a93e0
        // fills from the person FUN_0040c200 flags (+0x58): +0x50 + 1.0,
        // clamped to 1..9, else 1. The trench run's integer draw drops the
        // fraction.
        use crate::character_command::CommandKind::Commander;
        let (mut world, [battle, ..], alliance, empire) = withdrawal_world();
        let rating = |world: &GameWorld| {
            BattleSession::new(world, battle, alliance, empire, true, 0, 1)
                .trench_run_success_rating
        };
        assert_eq!(rating(&world), 1);
        officer(&mut world, true, Commander, 107, Some(alliance), battle);
        assert_eq!(rating(&world), 6);
    }

    // FUN_00509330 starts from the system's own side's Commander; a fleet's
    // replaces it only with more combat. FUN_0040c200 flags only an Alliance
    // person against +0x1c, so an Imperial pick leaves the term at 1.
    #[test]
    fn an_imperial_system_commander_can_hold_the_alliance_slot() {
        use crate::character_command::CommandKind::Commander;
        let (mut world, [battle, ..], alliance, _) = withdrawal_world();
        world.systems[battle].control =
            crate::world::ControlKind::Controlled(crate::dat::Faction::Empire);
        officer(&mut world, false, Commander, 300, None, battle);
        // A tie keeps the one held: FUN_00509530 replaces only on more.
        officer(&mut world, true, Commander, 300, Some(alliance), battle);
        assert_eq!(trench_commander_term(&world, battle), 1.0);
        let second = world.fleets.insert(world.fleets[alliance].clone());
        world.fleets[second].characters.clear();
        world.systems[battle].fleets.push(second);
        officer(&mut world, true, Commander, 320, Some(second), battle);
        assert_eq!(trench_commander_term(&world, battle), 9.0);
    }

    // FUN_005aefc0: the shot's chance adds the commander term.
    #[test]
    fn the_trench_run_shot_adds_the_commander_term() {
        let seed = seed_where(&[10, 100], |d| d[0] <= 9 && d[1] <= 4);
        let mut battle = runner(seed, 8, 96.0, 0);
        battle.trench_commander = 9.0;
        assert!(battle.trench_run(&[1]).success);
        let mut battle = runner(seed, 8, 96.0, 0);
        assert_eq!(battle.trench_commander, 1.0);
        assert!(battle.trench_run(&[1]).visits > 1);
    }

    // FUN_005d14d0: the Death Star comes first in each side's step.
    #[test]
    fn a_battle_step_runs_the_death_star_part() {
        let mut battle = trench_battle(2, 9, 400);
        let _ = battle.step();
        assert_eq!(battle.session.death_star.unwrap().hull, 0.0);
    }

    // FUN_005d14d0: no run against a shielded Death Star (FUN_005ba450) or
    // without more fighter units than the Empire (FUN_005c5eb0).
    #[test]
    fn the_alliance_makes_no_trench_run_when_shielded_or_outnumbered() {
        let mut battle = trench_battle(2, 9, 400);
        battle.session.death_star = Some(imperial_death_star(true));
        battle.death_star_step(true);
        assert!(battle.session.ships[0].alive);
        let mut battle = trench_battle(1, 9, 400);
        battle.death_star_step(true);
        assert!(battle.session.ships[0].alive);
        assert_eq!(
            battle.session.death_star.unwrap().hull,
            ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
        );
    }

    // hyp (FUN_005afe40): fighters leave only with a carrier that both has
    // room and jumps; otherwise they are destroyed.
    #[test]
    fn fighters_leave_only_with_a_jumping_carrier() {
        let run = |capacity: u32, hyperdrive: u8| {
            let mut own = ship(true, TacticalWeaponArc::default(), 0.0);
            own.fighter_capacity = capacity;
            own.subsystem_capacity.hyperdrive = hyperdrive;
            let mut battle = AbstractBattle::new(session(
                vec![own, ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0)],
                vec![fighter(true, TacticalWeaponArc::default())],
            ));
            battle.retreat_allowed[0] = true;
            battle.retreat(true);
            battle.session.fighters[0].squad_count > 0
        };
        assert!(run(1, 1));
        assert!(!run(0, 1));
        assert!(!run(1, 0));
    }
}
