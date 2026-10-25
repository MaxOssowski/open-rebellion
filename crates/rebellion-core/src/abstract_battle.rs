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
/// (`FUN_005b1bc0`) and the engines have power (slot `+0x90`), for a
/// capital ship or a fighter group alike (`FUN_005d0f50`, `FUN_005d14d0`).
fn hyperdrive_works(session: &BattleSession, unit: AbstractUnit) -> bool {
    match unit {
        AbstractUnit::Capital(index) => {
            let ship = &session.ships[index];
            ship.subsystem_capacity.hyperdrive > ship.subsystem_damage.hyperdrive
                && original_tactical_mobility(&session.ships, &session.field_effects, index)
                    .is_some_and(|mobility| mobility.effective_engine_power != 0.0)
        }
        AbstractUnit::Fighter(index) => {
            let fighter = &session.fighters[index];
            // port: a group's engine power is its sublight rating.
            fighter.has_hyperdrive && fighter.sublight_engine_power != 0.0
        }
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
    /// fighter group or capital ship has a working hyperdrive; for the Empire, an operational
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

    /// `FUN_005d14d0` step 5: the side leaves ([`leave`]).
    fn retreat(&mut self, alliance: bool) {
        leave(&mut self.session, alliance);
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

/// `FUN_005d14d0` step 5, the side leaving: each fighter group and capital
/// ship with a working hyperdrive escapes (`FUN_005af910`, state 9); the
/// others are destroyed (`FUN_005af7a0`). hyp: a fighter group without one
/// escapes when its side has an escaping carrier (`FUN_005afe40` recovers
/// it), and is destroyed otherwise. The tactical battle has its own
/// commanders (`FUN_005be160`) and withdraws through `FUN_005c8780`.
pub fn leave(session: &mut BattleSession, alliance: bool) {
    let units = units_of(session, alliance);
    let carrier_escapes = units.iter().any(|&unit| match unit {
        AbstractUnit::Capital(index) => {
            session.ships[index].fighter_capacity > 0 && hyperdrive_works(session, unit)
        }
        AbstractUnit::Fighter(_) => false,
    });
    for unit in units {
        let escapes = match unit {
            AbstractUnit::Capital(_) => hyperdrive_works(session, unit),
            AbstractUnit::Fighter(_) => hyperdrive_works(session, unit) || carrier_escapes,
        };
        match (unit, escapes) {
            (AbstractUnit::Capital(index), true) => {
                session.ships[index].retreated = true;
            }
            (AbstractUnit::Fighter(index), true) => {
                session.fighters[index].alive = false;
            }
            (AbstractUnit::Capital(index), false) => {
                let ship = &mut session.ships[index];
                ship.hull_current = 0;
                ship.shield = 0;
                ship.alive = false;
            }
            (AbstractUnit::Fighter(index), false) => {
                let fighter = &mut session.fighters[index];
                fighter.hull_current = 0.0;
                fighter.refresh_craft_count();
            }
        }
    }
}

/// `FUN_005d2190`: a task force's target is its first member's current
/// attack target, even when a later member has one and the first has none.
fn task_force_target(
    session: &BattleSession,
    members: &[AbstractUnit],
) -> Option<crate::tactical::TacticalAttackTarget> {
    members.first().and_then(|&member| match member {
        AbstractUnit::Capital(index) => session.ships[index].attack_target,
        AbstractUnit::Fighter(index) => session.fighters[index].attack_target,
    })
}

/// The tactical commander's test in `FUN_005c8780`: with its retreat order
/// and a way out (`FUN_005c7e80`), a side withdraws when the enemy's
/// strength is above 0 (`_DAT_0066cbb4`) and its own is under 0.33 (`+0x24`)
/// of it, or 0. The strengths are `FUN_005c7f70`: `FUN_005d0ce0`'s sum (the
/// same per-unit terms and Death Star bonus) over the tactical task forces,
/// whose target is its first member's current attack target:
/// `FUN_005d2190` returns the first node of the group's member list
/// (`+0x10`, then the node's `+0x10`). port: members are in the port's
/// list order, and no arrivals are pending (`FUN_005ae1d0`).
#[must_use]
pub fn tactical_side_withdraws(session: &BattleSession, alliance: bool) -> bool {
    let mut probe = AbstractBattle::new(session.clone());
    probe.retreat_allowed = session.retreat_allowed;
    probe.retreat_done = session.retreat_done;
    for side in [true, false] {
        for (group, members) in probe.groups(side) {
            let target = task_force_target(session, &members);
            let target = match target {
                Some(crate::tactical::TacticalAttackTarget::CapitalShip(index)) => {
                    AbstractUnit::Capital(index)
                }
                Some(crate::tactical::TacticalAttackTarget::FighterGroup(index)) => {
                    AbstractUnit::Fighter(index)
                }
                None => continue,
            };
            probe.targets.insert(group, target);
        }
    }
    if !probe.can_leave(alliance) {
        return false;
    }
    let enemy = probe.side_strength(!alliance);
    if enemy <= 0.0 {
        return false;
    }
    let own = probe.side_strength(alliance);
    own / enemy < ABSTRACT_RETREAT_RATIO || own == 0.0
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
        TacticalAttackTarget, TacticalSubsystemDamage, TacticalTactic, TacticalVoiceAnnouncer, TacticalWeaponRanges, TacticalWorldPosition,
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
            has_hyperdrive: false,
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
            retreating: false,
            retreat_progress: 0.0,
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
            turn_events: Vec::new(),
            trench_run_outcome: None,
            trench_run_remaining_ms: None,
            trench_run_group: None,
            trench_run_success_rating: 1,
            trench_run_pending_outcome: None,
            trench_run_participants: Vec::new(),
            trench_run_saved: Vec::new(),
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
            voice_announcer: TacticalVoiceAnnouncer::default(),
            field_effects: Vec::new(),
            subsystem_repairs: Vec::new(),
            paused: false,
            combat_speed: 1,
            step_accumulator: 0.0,
            winner: None,
            retreat_allowed: [false; 2],
            retreat_done: [false; 2],
            withdrawal_ms: None,
            ai_withdrawal_ms: None,
            diagnostic_log: Vec::new(),
        }
    }

    // FUN_005b4c60: the arc with the greatest total, the first on a tie.
    // Source: FUN_005d2190 returns the first node of the task force's
    // member list; FUN_005c7f70 takes that unit's target.
    #[test]
    fn a_task_force_takes_its_first_member_s_target_only() {
        let arc = TacticalWeaponArc::default();
        let mut session = session(
            vec![ship(true, arc, 1.0), ship(true, arc, 1.0), ship(false, arc, 1.0)],
            Vec::new(),
        );
        session.ships[1].attack_target = Some(TacticalAttackTarget::CapitalShip(2));
        let members = [AbstractUnit::Capital(0), AbstractUnit::Capital(1)];
        assert_eq!(task_force_target(&session, &members), None);
        session.ships[0].attack_target = Some(TacticalAttackTarget::CapitalShip(2));
        assert_eq!(
            task_force_target(&session, &members),
            Some(TacticalAttackTarget::CapitalShip(2))
        );
        assert_eq!(task_force_target(&session, &[]), None);
    }

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

    /// Steps until `done` holds and returns the combat step it held on.
    fn step_until(tactical: &mut BattleSession, done: impl Fn(&BattleSession) -> bool) -> u32 {
        for _ in 0..1_000 {
            if done(tactical) {
                return tactical.combat_tick;
            }
            tactical.step();
        }
        panic!("the condition never held");
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

    // The tactical battle's own commanders (FUN_005be160) step through
    // FUN_005c8780: the AI side with its retreat order and a way out
    // (FUN_005c7e80) withdraws through FUN_005ad530 like the player's once
    // its strength falls under 0.33 of the enemy's. Its ships with a
    // working hyperdrive head for their edge and jump after its 30 s; one
    // without fights on (FUN_005b2720). The player's side never withdraws
    // on its own. port: the rule is the abstract one's (FUN_005d19a0).
    #[test]
    fn the_ai_side_of_a_tactical_battle_withdraws_like_the_player() {
        let weak = |alliance: bool| {
            let mut fast = jumper(alliance, TacticalWeaponArc::new(0, 0, 1), 1.0);
            fast.subsystem_condition.hyperdrive = 100;
            let mut slow = ship(alliance, TacticalWeaponArc::new(0, 0, 1), 1.0);
            slow.task_force = 1;
            let mut strong = ship(!alliance, TacticalWeaponArc::new(0, 0, 4), 30.0);
            strong.hull_current = 1_000_000;
            strong.hull_max = 1_000_000;
            vec![fast, slow, strong]
        };
        // The player has the Alliance (the attacker); the AI the Empire.
        let mut tactical = session(weak(false), Vec::new());
        tactical.retreat_allowed = [true, true];
        // The commander's first review is at turn 1, step 4.
        for _ in 0..4 {
            assert!(!tactical.step());
        }
        assert_eq!(tactical.retreat_done, [false, true]);
        assert_eq!(tactical.ai_withdrawal_ms, Some(30_000.0));
        assert_eq!(tactical.withdrawal_ms, None);
        // FUN_005ad530 gives every capital ship order 3.
        assert!(tactical.ships[0].retreating && tactical.ships[0].alive);
        assert!(tactical.ships[1].retreating);
        // At its edge when the time ends, it jumps for 4000 ms and leaves;
        // then the ship without a hyperdrive is destroyed (event 0x2f,
        // FUN_005b2720 -> FUN_005b2d20). The time ends at turn 2, step 8;
        // the frame's order step decides the jump at step 12, stamped 5
        // (0x005ad4a0); it runs at turn 6, step 24 (FUN_005a96b0); the
        // flight ends on step 39, and its event 0x2e, stamped 12, runs at
        // turn 13, step 52.
        tactical.ships[0].source_position.z = tactical.withdrawal_exit_z(false);
        tactical.ai_withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        assert_eq!(step_until(&mut tactical, |t| t.ships[0].jumping()), 24);
        assert_eq!(step_until(&mut tactical, |t| !t.ships[0].alive), 52);
        assert!(tactical.ships[0].retreated && !tactical.ships[0].alive);
        assert!(!tactical.ships[1].retreated && !tactical.ships[1].alive);
        assert_eq!(tactical.ai_withdrawal_ms, None);

        // Without its order the AI side fights on.
        let mut ordered = session(weak(false), Vec::new());
        ordered.retreat_allowed = [true, false];
        // Through the first review, turn 1.
        for _ in 0..4 {
            ordered.step();
        }
        assert!(!ordered.ships[0].retreated);
        assert_eq!(ordered.retreat_done, [false, false]);

        // The player's own losing side does not leave by this rule.
        let mut player = session(weak(true), Vec::new());
        player.retreat_allowed = [true, true];
        for _ in 0..4 {
            player.step();
        }
        assert!(!player.ships[0].retreated);
        assert_eq!(player.retreat_done, [false, false]);
    }

    // FUN_005ad530: the player's ships with a working hyperdrive take
    // order 3, and the side's time (+0x7c) becomes at least 30000 ms
    // (DAT_0066c240), or 150000 (DAT_0066c234) for the Empire with an
    // operational Death Star. FUN_005c8780 counts it down by the step; at 0
    // a ship at its exit line starts its jump (FUN_005cc300) and leaves when
    // the jump's 4000 ms (_DAT_0066c24c) end (FUN_005a6b40, event 0x2e).
    #[test]
    fn the_players_withdrawal_ends_after_the_sides_time() {
        let mut stranded = ship(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        stranded.task_force = 1;
        let mut tactical = session(
            vec![
                jumper(true, TacticalWeaponArc::new(0, 0, 1), 1.0),
                stranded,
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
            ],
            Vec::new(),
        );
        // FUN_005b1bc0: the hyperdrive is ready (its condition too).
        tactical.ships[0].subsystem_condition.hyperdrive = 100;
        // FUN_005ad530 gives every capital ship order 3, a damaged or a
        // missing hyperdrive too.
        let mut damaged = jumper(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        damaged.subsystem_condition.hyperdrive = 0;
        let mut missing = ship(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        missing.subsystem_condition.hyperdrive = 100;
        tactical.ships.push(damaged);
        tactical.ships.push(missing);
        tactical.ships[0].source_position.z = tactical.withdrawal_exit_z(true);
        tactical.begin_player_withdrawal();
        assert!(tactical.ships[0].retreating && tactical.ships[1].retreating);
        assert!(tactical.ships[3].retreating && tactical.ships[4].retreating);
        assert!(!tactical.ships[2].retreating);
        assert_eq!(tactical.withdrawal_ms, Some(30_000.0));
        // 30000 ms is 120 steps of 250 ms.
        for _ in 0..119 {
            tactical.step();
        }
        assert!(tactical.ships[0].alive && !tactical.ships[0].retreated);
        assert_eq!(tactical.ships[0].retreat_progress, 0.0);
        tactical.step();
        // The jump is decided on step 120, a turn's first step, so it is
        // stamped 32 (+0x87c before it advances, 0x005bedf6) and starts at
        // turn 33, step 132 (FUN_005a96b0). Until then the ship stays in
        // the fight (state 6), and the side waits on it (state 3).
        assert_eq!(
            tactical.turn_events,
            vec![crate::tactical::TacticalQueuedEvent {
                stamp: 32,
                event: crate::tactical::TacticalTurnEvent::JumpStart(
                    crate::tactical::TacticalUnitRef::Ship(0)
                ),
            }]
        );
        assert!(tactical.ships[0].alive && !tactical.ships[0].jumping());
        assert_eq!(tactical.withdrawal_ms, Some(0.0));
        assert_eq!(step_until(&mut tactical, |t| t.ships[0].jumping()), 132);
        // 4000 ms is 16 steps from the first: the flight ends on step 147,
        // and its event 0x2e (stamped 39) runs at turn 40, step 160.
        assert_eq!(
            step_until(&mut tactical, |t| t.ships[0].retreat_progress
                >= crate::tactical::ORIGINAL_JUMP_MILLISECONDS),
            147
        );
        assert!(tactical.ships[0].alive && !tactical.ships[0].retreated);
        assert_eq!(step_until(&mut tactical, |t| !t.ships[0].alive), 160);
        assert!(!tactical.ships[0].alive && tactical.ships[0].retreated);
        // The side's end (state 4): those that cannot jump are destroyed
        // (event 0x2f, FUN_005b2720 -> FUN_005b2d20).
        for index in [1, 3, 4] {
            let ship = &tactical.ships[index];
            assert!(!ship.alive && !ship.retreated && ship.hull_current == 0);
        }
        assert!(tactical.ships[2].alive);
        assert_eq!(tactical.withdrawal_ms, None);

        // A second order keeps the larger time left.
        tactical.withdrawal_ms = Some(40_000.0);
        tactical.start_withdrawal_time();
        assert_eq!(tactical.withdrawal_ms, Some(40_000.0));

        // The Empire with an operational Death Star: 150000 ms.
        let mut imperial = session(Vec::new(), Vec::new());
        imperial.player_is_attacker = false;
        imperial.death_star = Some(crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: crate::tactical::TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        imperial.start_withdrawal_time();
        assert_eq!(imperial.withdrawal_ms, Some(150_000.0));
        imperial.withdrawal_ms = None;
        imperial.death_star.as_mut().unwrap().destroyed = true;
        imperial.start_withdrawal_time();
        assert_eq!(imperial.withdrawal_ms, Some(30_000.0));
    }

    // FUN_005c8780 state 4: when the side's last jump ends, each unit of the
    // side without a working hyperdrive still in the fight is destroyed
    // (event 0x2f, FUN_005b2720 -> FUN_005b2d20): a group still on its way
    // to the carrier that escaped, one on its way to the carrier that
    // stays, one only awaiting a carrier, and the carrier without a
    // hyperdrive.
    #[test]
    fn units_left_behind_are_destroyed_at_the_sides_end() {
        use crate::tactical::TacticalFighterRecoveryState as Recovery;
        let mut carrier = jumper(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        carrier.subsystem_condition.hyperdrive = 100;
        carrier.fighter_capacity = 2;
        let mut grounded = ship(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        grounded.fighter_capacity = 1;
        let enemy = ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0);
        let group = |state, target| {
            let mut value = fighter(true, TacticalWeaponArc::default());
            value.recovery_state = state;
            value.recovery_target = Some(target);
            value
        };
        let mut tactical = session(
            vec![carrier, grounded, enemy],
            vec![
                group(Recovery::Returning, 0),
                group(Recovery::Returning, 1),
                group(Recovery::AwaitingCarrier, 0),
            ],
        );
        tactical.ships[0].retreating = true;
        tactical.ships[0].source_position.z = tactical.withdrawal_exit_z(true);
        // The groups bound for a carrier have no engine power, and the one
        // that stays is far off, so both are still on their way at the
        // side's end.
        tactical.ships[1].source_position.z = 200.0;
        tactical.fighters[0].sublight_engine_power = 0.0;
        tactical.fighters[1].sublight_engine_power = 0.0;
        tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        // The time ends at turn 1 (step 4); the carrier's jump is decided
        // at step 12, runs at turn 6 (step 24), flies 4000 ms and leaves at
        // turn 13 (step 52).
        assert_eq!(step_until(&mut tactical, |t| t.ships[0].retreated), 52);
        for index in 0..3 {
            assert_eq!(tactical.fighters[index].squad_count, 0);
        }
        assert!(!tactical.ships[1].alive && !tactical.ships[1].retreated);
        assert_eq!(tactical.withdrawal_ms, None);
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

    // hyp (FUN_005afe40): fighters without a hyperdrive leave only with a
    // carrier that both has room and jumps; otherwise they are destroyed.
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

    // FUN_005d14d0 step 5 tests each fighter group with FUN_005b1b70 and
    // sends one with a working hyperdrive away itself (FUN_005af910), with
    // no carrier; FUN_005d0f50 lets such a group alone take its side away.
    #[test]
    fn a_fighter_group_with_a_hyperdrive_leaves_without_a_carrier() {
        let run = |has_hyperdrive: bool| {
            let mut own = fighter(true, TacticalWeaponArc::default());
            own.has_hyperdrive = has_hyperdrive;
            let mut battle = AbstractBattle::new(session(
                vec![ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0)],
                vec![own],
            ));
            battle.retreat_allowed[0] = true;
            let can_leave = battle.can_leave(true);
            battle.retreat(true);
            (can_leave, battle.session.fighters[0].squad_count > 0)
        };
        assert_eq!(run(true), (true, true));
        assert_eq!(run(false), (false, false));
    }

    // FUN_005b1b70 also needs engine power (slot +0x90).
    #[test]
    fn a_fighter_group_without_engine_power_cannot_jump() {
        let mut own = fighter(true, TacticalWeaponArc::default());
        own.has_hyperdrive = true;
        own.sublight_engine_power = 0.0;
        let mut battle = AbstractBattle::new(session(
            vec![ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0)],
            vec![own],
        ));
        battle.retreat_allowed[0] = true;
        assert!(!battle.can_leave(true));
    }

    // FUN_005b1bc0 counts the installed hyperdrives (+0x3ec); hyp: a fighter
    // group's are its FIGHTSD primary and backup ratings.
    #[test]
    fn a_battle_gives_a_fighter_group_its_class_hyperdrive() {
        let has = |primary: u32, backup: u32| {
            let (mut world, system, alliance, ships, squadrons) = world_with_battle();
            let class = world.fleets[alliance].fighters[0].class;
            world.fighter_classes[class].hyperdrive = primary;
            world.fighter_classes[class].hyperdrive_if_damaged = backup;
            let FleetBattle { session, .. } = BattleSession::new_for_fleets(
                &world,
                system,
                &[alliance],
                &[ships, squadrons],
                true,
                3,
                7,
            );
            session.fighters[0].has_hyperdrive
        };
        assert!(has(60, 0));
        assert!(has(0, 60));
        assert!(has(1, 1));
        assert!(!has(0, 0));
    }

    // FUN_005c6610 (order 3): when the player's time ends, the player's
    // withdrawing fighter groups with a working hyperdrive (FUN_005b1b70:
    // engine power too) at their exit line jump and leave 4000 ms later
    // with their craft; one without engine power is destroyed at the side's
    // end (FUN_005b2720 -> FUN_005b2d20); the other side's do not jump.
    #[test]
    fn the_players_fighters_with_a_hyperdrive_jump_when_the_time_ends() {
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut tactical = session(vec![enemy], Vec::new());
        let exit = tactical.withdrawal_exit_z(true);
        let group = |alliance: bool, power: f32| {
            let mut value = fighter(alliance, TacticalWeaponArc::default());
            value.has_hyperdrive = true;
            value.retreating = alliance;
            value.sublight_engine_power = power;
            value.source_position.z = exit;
            value
        };
        tactical.fighters = vec![group(true, 10.0), group(true, 0.0), group(false, 10.0)];
        tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        // Decided at step 12, the first frame step (state 6, stamped 5),
        // the jump runs at turn 6, step 24 (0x005ad4a0, FUN_005a96b0); its
        // 0x2e, stamped 12 on step 39, runs at turn 13, step 52.
        for _ in 0..12 {
            tactical.step();
        }
        assert!(tactical.fighters[0].alive && !tactical.fighters[0].jumping());
        assert!(tactical.fighters[1].alive && tactical.fighters[1].retreating);
        assert_eq!(step_until(&mut tactical, |t| t.fighters[0].jumping()), 24);
        assert_eq!(step_until(&mut tactical, |t| !t.fighters[0].alive), 52);
        assert!(!tactical.fighters[0].alive && tactical.fighters[0].squad_count > 0);
        // It could not jump: destroyed at the side's end (FUN_005b2720).
        assert!(!tactical.fighters[1].alive && tactical.fighters[1].squad_count == 0);
        assert!(tactical.fighters[2].alive);
        assert_eq!(tactical.withdrawal_ms, None);
    }

    // FUN_005c6610 heads a withdrawing fighter force for its exit line; it
    // jumps only within 8.0 (_DAT_0066cb0c) of it once the time has ended
    // (_DAT_0066cb18). port: straight at the sublight rating per second.
    #[test]
    fn a_withdrawing_fighter_group_flies_to_its_edge_before_it_jumps() {
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut wing = fighter(true, TacticalWeaponArc::default());
        wing.has_hyperdrive = true;
        wing.retreating = true;
        wing.sublight_engine_power = 4.0;
        let mut tactical = session(vec![enemy], vec![wing]);
        let exit = tactical.withdrawal_exit_z(true);
        tactical.fighters[0].source_position.z = exit + 9.0;
        tactical.withdrawal_ms = Some(2.0 * crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        // The task force's order step runs in the frame (slot +0x14,
        // 0x005a9ff8), from turn 3: 4.0 a second, one unit a 250 ms step
        // from step 12.
        for _ in 0..11 {
            tactical.step();
        }
        assert_eq!(tactical.fighters[0].source_position.z, exit + 9.0);
        tactical.step();
        assert_eq!(tactical.fighters[0].source_position.z, exit + 8.0);
        // Within 8.0 of the line but still moving (0x005ec0d0 returns 1
        // beyond its 1.0 radius), so FUN_005c6610 tries no jump yet.
        assert!(tactical.turn_events.is_empty());
        for _ in 0..7 {
            tactical.step();
        }
        assert_eq!(tactical.fighters[0].source_position.z, exit + 1.0);
        assert!(tactical.turn_events.is_empty());
        // At step 20, a turn's first, the move finds it arrived (|d| = 1.0):
        // its jump is decided (state 6, event 0x2d stamped 7, 0x005ad4a0)
        // and it holds there until the event runs at turn 8, step 32.
        tactical.step();
        assert_eq!(tactical.combat_tick, 20);
        assert_eq!(tactical.fighters[0].source_position.z, exit + 1.0);
        assert_eq!(tactical.turn_events.len(), 1);
        assert_eq!(tactical.turn_events[0].stamp, 7);
        assert!(!tactical.fighters[0].jumping());
        assert_eq!(step_until(&mut tactical, |t| t.fighters[0].jumping()), 32);
        // Then the flight moves it on in that step: k = 25 (DAT_006bc47c)
        // for 250 ms (FUN_005a6b40).
        assert_eq!(
            tactical.fighters[0].source_position.z,
            exit + 1.0 - 25.0 * 0.25 * 0.25
        );
    }

    // 0x005ec0d0 moves a full step at the force's speed, not clamped to
    // the distance left, and stops only within its 1.0 radius.
    #[test]
    fn a_withdrawing_fighter_group_overshoots_its_line_by_a_full_step() {
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let run = |power: f32| {
            let mut wing = fighter(true, TacticalWeaponArc::default());
            wing.retreating = true;
            wing.sublight_engine_power = power;
            let mut tactical = session(vec![enemy.clone()], vec![wing]);
            let exit = tactical.withdrawal_exit_z(true);
            tactical.fighters[0].source_position.z = exit + 2.0;
            tactical.withdrawal_ms = Some(100_000.0);
            // One move a frame step from step 12 (0x005a9ff8).
            for _ in 0..11 {
                tactical.step();
            }
            let mut at = Vec::new();
            for _ in 0..3 {
                tactical.step();
                at.push(tactical.fighters[0].source_position.z - exit);
            }
            at
        };
        // 10 a second, 2.5 a step: past the line by 0.5, within the radius,
        // so it stays.
        assert_eq!(run(10.0), [-0.5, -0.5, -0.5]);
        // 2.5 a second: 0.625 a step until within the radius.
        assert_eq!(run(2.5), [1.375, 0.75, 0.75]);
    }

    // FUN_005be160 gives the Alliance (0, 0, 1) and the Empire (0, 0, -1);
    // FUN_005cc300 heads for it times -1.0 and half the extent +0x8cc.
    #[test]
    fn each_side_withdraws_to_its_own_edge() {
        let tactical = session(Vec::new(), Vec::new());
        let half = tactical.source_layout.battle_extent * 0.5;
        assert_eq!(tactical.withdrawal_exit_z(true), -half);
        assert_eq!(tactical.withdrawal_exit_z(false), half);
    }

    // FUN_005ad530 drops a withdrawing ship's attack (FUN_005a8d30) and
    // gives it order 3; FUN_005cc300 heads it for its exit line each step.
    #[test]
    fn a_withdrawing_ship_drops_its_attack_and_heads_for_its_exit() {
        let mut own = jumper(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        own.subsystem_condition.hyperdrive = 100;
        own.order = TacticalOrder::LeftHook;
        own.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(1));
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut tactical = session(vec![own, enemy], Vec::new());
        tactical.begin_player_withdrawal();
        let ship = &tactical.ships[0];
        assert_eq!(ship.order, TacticalOrder::None);
        assert_eq!(ship.attack_target, None);
        // The frame's moves start at turn 3, step 12 (0x005bef1d).
        for _ in 0..11 {
            tactical.step();
        }
        assert_eq!(tactical.ships[0].source_waypoint, None);
        tactical.step();
        let ship = &tactical.ships[0];
        let exit = tactical.withdrawal_exit_z(true);
        assert_eq!(ship.source_waypoint.map(|waypoint| waypoint.z), Some(exit));
        assert_eq!(
            ship.source_waypoint
                .map(|waypoint| (waypoint.x, waypoint.y)),
            Some((ship.source_position.x, ship.source_position.y))
        );
    }

    // FUN_005cc300: after the side's time a ship jumps only within 8.0
    // (_DAT_0066cff0) of its exit line; the side waits on one still on its
    // way. FUN_005b1e40 -> FUN_005b2720: one whose hyperdrive was lost
    // returns to the fight.
    #[test]
    fn after_the_time_a_ship_jumps_only_near_its_exit() {
        let run = |offset: f32, hyperdrive: u8| {
            let mut own = jumper(true, TacticalWeaponArc::default(), 0.0);
            own.subsystem_condition.hyperdrive = hyperdrive;
            let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
            enemy.hull_current = 1_000_000;
            enemy.hull_max = 1_000_000;
            let mut tactical = session(vec![own, enemy], Vec::new());
            tactical.ships[0].retreating = true;
            tactical.ships[0].source_position.z = tactical.withdrawal_exit_z(true) + offset;
            tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
            // The commander ends the time at turn 1; the frame's order step
            // decides from turn 3, step 12.
            for _ in 0..12 {
                tactical.step();
            }
            let ship = &tactical.ships[0];
            // The jump is decided (state 6): event 0x2d is queued.
            (
                ship.retreating,
                tactical.turn_event_pending(&crate::tactical::TacticalTurnEvent::JumpStart(
                    crate::tactical::TacticalUnitRef::Ship(0),
                )),
                tactical.withdrawal_ms,
            )
        };
        assert_eq!(run(0.5, 100), (true, true, Some(0.0)));
        // 0x005ec0d0 / 0x005eb1a0: arrived when |d| is not above the radius.
        assert_eq!(run(1.0, 100), (true, true, Some(0.0)));
        // Within 8.0 but beyond the move's 1.0 radius the force is still
        // moving (0x005cc3ef), on either side of its line.
        assert_eq!(run(7.5, 100), (true, false, Some(0.0)));
        assert_eq!(run(-30.0, 100), (true, false, Some(0.0)));
        assert_eq!(run(8.5, 100), (true, false, Some(0.0)));
        assert_eq!(run(0.0, 0), (false, false, None));
        // The Empire's line is on the other edge.
        let mut own = jumper(false, TacticalWeaponArc::default(), 0.0);
        own.subsystem_condition.hyperdrive = 100;
        let mut enemy = ship(true, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut tactical = session(vec![own, enemy], Vec::new());
        tactical.player_is_attacker = false;
        tactical.ships[0].retreating = true;
        tactical.ships[0].source_position.z = tactical.withdrawal_exit_z(false) - 0.5;
        tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        let decided = |t: &BattleSession| {
            t.turn_event_pending(&crate::tactical::TacticalTurnEvent::JumpStart(
                crate::tactical::TacticalUnitRef::Ship(0),
            ))
        };
        for _ in 0..12 {
            tactical.step();
        }
        assert!(decided(&tactical));
        // Away from it, toward the field's middle, it does not.
        tactical.turn_events.clear();
        tactical.ships[0].source_position.z = tactical.withdrawal_exit_z(false) - 30.0;
        tactical.step();
        assert!(!decided(&tactical));
    }

    // FUN_005cc300 tries only its force's first member (0x005c6070), which
    // stays first through its jump until it is gone (FUN_005a90c0 ->
    // FUN_005aa760); a first member that cannot jump holds the others.
    #[test]
    fn a_task_force_jumps_its_ships_one_after_another() {
        let run = |first_hyperdrive: u8| {
            let mut first = jumper(true, TacticalWeaponArc::default(), 0.0);
            first.subsystem_condition.hyperdrive = first_hyperdrive;
            let mut second = jumper(true, TacticalWeaponArc::default(), 0.0);
            second.subsystem_condition.hyperdrive = 100;
            let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
            enemy.hull_current = 1_000_000;
            enemy.hull_max = 1_000_000;
            let mut tactical = session(vec![first, second, enemy], Vec::new());
            let exit = tactical.withdrawal_exit_z(true);
            for index in [0, 1] {
                tactical.ships[index].retreating = true;
                tactical.ships[index].source_position.z = exit;
            }
            tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
            // Decided in the first frame step, step 12, stamped 5.
            for _ in 0..12 {
                tactical.step();
            }
            tactical
        };
        let decided = |t: &BattleSession, index: usize| {
            t.turn_event_pending(&crate::tactical::TacticalTurnEvent::JumpStart(
                crate::tactical::TacticalUnitRef::Ship(index),
            ))
        };
        let mut tactical = run(100);
        assert!(decided(&tactical, 0));
        assert!(!decided(&tactical, 1));
        // The first jumps at turn 6 (step 24) and leaves at turn 13 (step
        // 52, event 0x2e); until then the second is not tried.
        let gone = step_until(&mut tactical, |t| {
            if !t.ships[0].alive {
                return true;
            }
            assert!(!decided(t, 1) && !t.ships[1].jumping());
            false
        });
        assert_eq!(gone, 52);
        assert!(tactical.ships[0].retreated);
        // Now the second is first. Back at its line (port: it followed
        // the first's flight in formation), it is decided on the next step,
        // 53, stamped 16, and jumps at turn 17, step 68.
        tactical.ships[1].source_position.z = tactical.withdrawal_exit_z(true);
        assert_eq!(step_until(&mut tactical, |t| decided(t, 1)), 53);
        assert_eq!(step_until(&mut tactical, |t| t.ships[1].jumping()), 68);

        let mut held = run(0);
        for _ in 0..60 {
            held.step();
        }
        assert!(!held.ships[0].jumping() && !held.ships[1].jumping());
        assert!(!decided(&held, 0) && !decided(&held, 1));
    }

    // FUN_005c8780 and FUN_005cc300 act on the withdrawing side's order-3
    // units only: a ship that was not withdrawing does not jump, a jumping
    // ship is not steered again (FUN_005a6b40 owns it), and a fighter group
    // with a hyperdrive that appears after the time ended stays (port: the
    // groups leave when the time ends).
    #[test]
    fn only_the_withdrawing_units_leave() {
        let mut staying = jumper(true, TacticalWeaponArc::default(), 0.0);
        staying.subsystem_condition.hyperdrive = 100;
        let mut jumping = jumper(true, TacticalWeaponArc::default(), 0.0);
        jumping.subsystem_condition.hyperdrive = 100;
        // Its own task force: 0x005c6070 tries a force's first member only.
        jumping.task_force = 1;
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut tactical = session(vec![staying, jumping, enemy], Vec::new());
        let exit = tactical.withdrawal_exit_z(true);
        tactical.ships[0].source_position.z = exit;
        tactical.ships[1].source_position.z = exit;
        tactical.ships[1].retreating = true;
        tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        tactical.step();
        assert!(!tactical.ships[0].retreating);
        assert_eq!(tactical.ships[0].retreat_progress, 0.0);
        // Decided at step 12, its event 0x2d runs at turn 6, step 24.
        assert_eq!(step_until(&mut tactical, |t| t.ships[1].jumping()), 24);
        assert_eq!(tactical.ships[0].retreat_progress, 0.0);
        let mut late = fighter(true, TacticalWeaponArc::default());
        late.has_hyperdrive = true;
        tactical.fighters.push(late);
        tactical.step();
        assert_eq!(tactical.ships[1].source_waypoint, None);
        assert!(tactical.fighters[0].alive);
    }

    // FUN_005ad530 for the AI side's fighter groups (class 4): one with a
    // hyperdrive takes order 3 and leaves when the AI's time ends; one
    // without takes order 2 to a carrier of its own side (FUN_005cf980).
    // The player's groups and time are untouched. The Empire's floor is
    // 150000 ms (DAT_0066c234) while its Death Star is operational.
    #[test]
    fn the_ai_sides_fighters_withdraw_by_their_hyperdrive() {
        use crate::tactical::TacticalFighterRecoveryState as Recovery;
        let mut carrier = ship(false, TacticalWeaponArc::default(), 0.0);
        carrier.fighter_capacity = 1;
        let mut enemy = ship(true, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut defender = fighter(false, TacticalWeaponArc::default());
        defender.has_hyperdrive = true;
        defender.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(1));
        let tie = fighter(false, TacticalWeaponArc::default());
        let mut own = fighter(true, TacticalWeaponArc::default());
        own.has_hyperdrive = true;
        let mut tactical = session(vec![carrier, enemy], vec![defender, tie, own]);
        tactical.death_star = Some(crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: crate::tactical::TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        tactical.begin_withdrawal(false);
        tactical.withdraw_fighters(false);
        assert_eq!(tactical.ai_withdrawal_ms, Some(150_000.0));
        assert_eq!(tactical.withdrawal_ms, None);
        assert_eq!(tactical.fighters[0].order, TacticalOrder::None);
        assert_eq!(tactical.fighters[0].attack_target, None);
        assert_eq!(tactical.fighters[1].order, TacticalOrder::Recover);
        assert_eq!(tactical.fighters[1].recovery_target, Some(0));
        assert_eq!(tactical.fighters[1].recovery_state, Recovery::Returning);
        assert_eq!(tactical.fighters[2].order, TacticalOrder::None);
        assert!(tactical.fighters[0].retreating && !tactical.fighters[2].retreating);
        tactical.fighters[0].source_position.z = tactical.withdrawal_exit_z(false);
        tactical.ai_withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        // Decided at step 12, it jumps at turn 6 and leaves at turn 13, step
        // 52 (events 0x2d and 0x2e, FUN_005a96b0).
        assert_eq!(step_until(&mut tactical, |t| !t.fighters[0].alive), 52);
        assert!(!tactical.fighters[0].alive && tactical.fighters[0].squad_count > 0);
        assert!(tactical.fighters[2].alive);
    }

    // FUN_005cf980 reserves a living carrier of the group's side with room;
    // a group already returning holds its place, a stale target outside
    // the recovery states does not, and one place serves one group. A
    // player's group with no room and no colour (4) queues no voice.
    #[test]
    fn a_recovery_reserves_one_free_place_on_a_living_carrier() {
        use crate::tactical::TacticalFighterRecoveryState as Recovery;
        let mut dead = ship(true, TacticalWeaponArc::default(), 0.0);
        dead.fighter_capacity = 2;
        dead.alive = false;
        let mut carrier = ship(true, TacticalWeaponArc::default(), 0.0);
        carrier.fighter_capacity = 2;
        let mut returning = fighter(true, TacticalWeaponArc::default());
        returning.recovery_state = Recovery::Returning;
        returning.recovery_target = Some(1);
        let mut stale = fighter(true, TacticalWeaponArc::default());
        stale.recovery_state = Recovery::AwaitingCarrier;
        stale.recovery_target = Some(1);
        let mut first = fighter(true, TacticalWeaponArc::default());
        first.fighter_group = 4;
        let mut second = fighter(true, TacticalWeaponArc::default());
        second.fighter_group = 4;
        let mut tactical = session(vec![dead, carrier], vec![returning, stale, first, second]);
        tactical.begin_original_fighter_recovery(&[2, 3]);
        assert_eq!(tactical.fighters[2].recovery_target, Some(1));
        assert_eq!(tactical.fighters[2].recovery_state, Recovery::Returning);
        assert_eq!(tactical.fighters[3].recovery_target, None);
        assert_eq!(
            tactical.fighters[3].recovery_state,
            Recovery::AwaitingCarrier
        );
        assert!(tactical.take_pending_voice_cues().is_empty());
    }

    // FUN_005c7f70 (as FUN_005d0ce0) leaves out a capital ship whose enemy
    // task force out-damages its shield recharge at it (FUN_005d2190,
    // FUN_005cb650 against slot +0x64). In the tactical battle that target
    // is the units' current attack target. FUN_005c8780 then wants the
    // enemy's strength above 0.
    #[test]
    fn a_tactical_side_counts_the_targets_of_the_enemys_task_forces() {
        let mut big = ship(false, TacticalWeaponArc::new(0, 0, 4), 30.0);
        big.task_force = 1;
        let mut small = jumper(false, TacticalWeaponArc::new(0, 0, 1), 1.0);
        small.task_force = 2;
        let player = ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0);
        let mut tactical = session(vec![big, small, player], Vec::new());
        tactical.retreat_allowed = [false, true];
        assert!(!tactical_side_withdraws(&tactical, false));
        tactical.ships[2].attack_target =
            Some(crate::tactical::TacticalAttackTarget::CapitalShip(0));
        assert!(tactical_side_withdraws(&tactical, false));

        // No strength on either side: the enemy's is not above 0.
        let mut idle = jumper(false, TacticalWeaponArc::default(), 0.0);
        idle.subsystem_condition.hyperdrive = 100;
        let mut tactical = session(
            vec![idle, ship(true, TacticalWeaponArc::default(), 0.0)],
            Vec::new(),
        );
        tactical.retreat_allowed = [false, true];
        assert!(!tactical_side_withdraws(&tactical, false));
    }

    // FUN_005c8780: the AI commander reviews the battle on the first turn
    // and every sixth after (+0x40 from 0, -1.0 a call, reset to 6.0; one
    // call a turn through slot +0x80, 0x005a9d10), and not while a trench
    // run is under way (FUN_005d0ae0).
    #[test]
    fn the_ai_commander_reviews_every_sixth_turn() {
        let weak = || {
            let mut fast = jumper(false, TacticalWeaponArc::new(0, 0, 1), 1.0);
            fast.subsystem_condition.hyperdrive = 100;
            let mut strong = ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0);
            strong.hull_current = 1_000_000;
            strong.hull_max = 1_000_000;
            vec![fast, strong]
        };
        let mut tactical = session(weak(), Vec::new());
        // Turn 1 (step 4) reviews without the retreat order.
        for _ in 0..4 {
            tactical.step();
        }
        tactical.retreat_allowed = [false, true];
        // The next review is at turn 7, step 28.
        for _ in 0..23 {
            tactical.step();
            assert!(!tactical.retreat_done[1]);
        }
        tactical.step();
        assert_eq!(tactical.combat_tick, 28);
        assert!(tactical.retreat_done[1]);
        // A side already withdrawing (+0x84 = 2) is not ordered again: the
        // next review leaves its time running down.
        for _ in 0..24 {
            tactical.step();
        }
        assert!(tactical.ai_withdrawal_ms.is_some_and(|left| left < 30_000.0));

        let mut running = session(weak(), Vec::new());
        running.retreat_allowed = [false, true];
        running.trench_run_remaining_ms = Some(60_000.0);
        running.trench_run_pending_outcome =
            Some(crate::tactical::TacticalTrenchRunOutcome::Failure);
        for _ in 0..4 {
            running.step();
        }
        assert!(!running.retreat_done[1]);
        // Nor during the run's approach, which is under way too.
        let mut approaching = session(weak(), Vec::new());
        approaching.retreat_allowed = [false, true];
        approaching.trench_run_group = Some(0);
        for _ in 0..28 {
            approaching.step();
        }
        assert!(!approaching.retreat_done[1]);
    }

    // FUN_005c6610 on each side: a group flies toward its own edge (the
    // Alliance's at -z, the Empire's at +z) from wherever it is, and once
    // the time has ended jumps only within 8.0 of that line on its side.
    #[test]
    fn withdrawing_fighters_head_for_their_own_edge() {
        let group = |alliance: bool, z: f32, power: f32| {
            let mut value = fighter(alliance, TacticalWeaponArc::default());
            value.has_hyperdrive = true;
            value.retreating = true;
            value.sublight_engine_power = power;
            value.source_position.z = z;
            value
        };
        let mut tactical = session(Vec::new(), Vec::new());
        let alliance_exit = tactical.withdrawal_exit_z(true);
        let empire_exit = tactical.withdrawal_exit_z(false);
        tactical.fighters = vec![
            // Past the field's middle: it turns back toward -z.
            group(true, 60.0, 4.0),
            // Just outside 8.0 after its move: no jump.
            group(true, alliance_exit + 8.5, 0.4),
            // Far inside the field: no jump.
            group(true, alliance_exit + 20.0, 0.4),
            // The Empire's group well short of its +z edge: no jump.
            group(false, 20.0, 0.4),
        ];
        tactical.withdrawal_ms = Some(0.0);
        tactical.ai_withdrawal_ms = Some(0.0);
        // The task forces' order steps run in the frame, from turn 3, step
        // 12 (0x005a9ff8, 0x005bef1d), with its 250 ms.
        for _ in 0..11 {
            tactical.step();
        }
        assert_eq!(tactical.fighters[0].source_position.z, 60.0);
        tactical.step();
        let fighters = &tactical.fighters;
        assert_eq!(fighters[0].source_position.z, 59.0);
        assert!((fighters[1].source_position.z - (alliance_exit + 8.4)).abs() < 1e-4);
        assert!((fighters[3].source_position.z - 20.1).abs() < 1e-4);
        assert!(empire_exit > 20.1);
        // No jump is decided (event 0x2d).
        assert!(tactical.turn_events.is_empty());
    }

    // FUN_005cdf50 takes a unit in state 2 or 4, not 6 (FUN_005ad870): a
    // withdrawing ship stays a target until its jump starts.
    #[test]
    fn a_withdrawing_ship_can_be_hit_until_its_jump_starts() {
        let run = |jumping: bool| {
            let mut own = jumper(true, TacticalWeaponArc::default(), 0.0);
            own.subsystem_condition.hyperdrive = 100;
            own.sublight_engine_power = 0.0;
            own.retreating = true;
            if jumping {
                own.retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
            }
            let mut enemy = ship(false, TacticalWeaponArc::new(0, 0, 4), 30.0);
            enemy.order = TacticalOrder::AttackCapitalShips;
            enemy.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(0));
            enemy.weapon_ranges = TacticalWeaponRanges {
                laser_cannon: 100.0,
                ion_cannon: 100.0,
                turbolaser: 100.0,
            };
            enemy.source_position.z = 10.0;
            enemy.weapon_arcs = [TacticalWeaponArc::new(0, 0, 4); 4];
            let mut tactical = session(vec![own, enemy], Vec::new());
            tactical.withdrawal_ms = Some(if jumping { 0.0 } else { 30_000.0 });
            // Fire decided at turn 1 hits at turn 6, step 24 (FUN_005b3f10's
            // hit event at +0x87c + 2.0). The turn's recharge after the
            // events may refill a small hit on the shield, so the hit is read
            // from its impact.
            for _ in 0..24 {
                tactical.step();
            }
            tactical.impact_effects.iter().any(|impact| impact.target == 0)
        };
        assert!(run(false));
        assert!(!run(true));
    }

    // FUN_005cdf50, FUN_005b3a40 and FUN_005b3f10: a dead ship and one whose
    // jump has started neither fire nor are fired at, by capital ships or
    // fighter groups; an attack order on one moves to a live ship
    // (FUN_005d0bb0), and a jumping ship's own order is dropped.
    #[test]
    fn dead_and_jumping_ships_drop_out_of_the_fight() {
        let ranges = TacticalWeaponRanges {
            laser_cannon: 100.0,
            ion_cannon: 100.0,
            turbolaser: 100.0,
        };
        let armed = |alliance: bool, z: f32| {
            let mut value = ship(alliance, TacticalWeaponArc::new(2, 0, 4), 30.0);
            value.weapon_arcs = [TacticalWeaponArc::new(2, 0, 4); 4];
            value.weapon_ranges = ranges;
            value.hull_current = 1_000_000;
            value.hull_max = 1_000_000;
            value.source_position.z = z;
            value
        };
        let mut shooter = armed(true, 0.0);
        shooter.order = TacticalOrder::AttackCapitalShips;
        shooter.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(1));
        let mut jumping = armed(false, 10.0);
        jumping.retreating = true;
        jumping.retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
        let mut dead = armed(false, 10.0);
        dead.alive = false;
        let live = armed(false, 12.0);
        let mut own_jumping = armed(true, 2.0);
        own_jumping.retreating = true;
        own_jumping.retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
        own_jumping.order = TacticalOrder::AttackCapitalShips;
        own_jumping.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(3));
        let mut wing = fighter(true, TacticalWeaponArc::new(4, 0, 0));
        wing.weapon_ranges = ranges;
        wing.source_position.z = 6.0;
        wing.order = TacticalOrder::AttackCapitalShips;
        let mut tactical = session(vec![shooter, jumping, dead, live, own_jumping], vec![wing]);
        let before = |tactical: &BattleSession, index: usize| {
            let ship = &tactical.ships[index];
            (ship.hull_current, ship.shield)
        };
        let jumping_before = before(&tactical, 1);
        let dead_before = before(&tactical, 2);
        let live_before = before(&tactical, 3);
        // Fire decided at turn 1 hits at turn 6, step 24 (FUN_005b3f10's
        // hit event at +0x87c + 2.0).
        for _ in 0..24 {
            tactical.step();
        }
        assert_eq!(before(&tactical, 1), jumping_before);
        assert_eq!(before(&tactical, 2), dead_before);
        assert_ne!(before(&tactical, 3), live_before);
        assert_eq!(
            tactical.ships[0].attack_target,
            Some(crate::tactical::TacticalAttackTarget::CapitalShip(3))
        );
        assert_eq!(tactical.ships[4].attack_target, None);
        assert_ne!(
            tactical.fighters[0].attack_target,
            Some(crate::tactical::TacticalAttackTarget::CapitalShip(1))
        );
    }

    // FUN_005ad870 and FUN_005cdf50 leave a unit in its jump (state 6 on)
    // out of the fight: a jumping fighter group picks no target, has no
    // strength against one, and is no one's target.
    #[test]
    fn a_jumping_fighter_group_drops_out_of_the_fight() {
        use crate::tactical::{
            first_original_attack_target, manual_attack_target_is_eligible,
            tactical_attack_target_is_eligible, OriginalFighterTarget, TacticalAttackTarget,
        };
        let ranges = TacticalWeaponRanges {
            laser_cannon: 100.0,
            ion_cannon: 100.0,
            turbolaser: 100.0,
        };
        let mut own = fighter(true, TacticalWeaponArc::new(4, 0, 0));
        own.weapon_ranges = ranges;
        let mut enemy = fighter(false, TacticalWeaponArc::new(4, 0, 0));
        enemy.weapon_ranges = ranges;
        enemy.source_position.z = 5.0;
        let tactical = session(Vec::new(), vec![own, enemy]);
        let target = TacticalAttackTarget::FighterGroup(1);
        let check = |tactical: &BattleSession| {
            (
                tactical_attack_target_is_eligible(
                    tactical,
                    true,
                    target,
                    TacticalOrder::AttackFighters,
                ),
                manual_attack_target_is_eligible(tactical, true, target),
                first_original_attack_target(tactical, true, TacticalOrder::AttackFighters),
                tactical.fighter_target(1).is_some(),
                tactical.fighter_target_strength(1, OriginalFighterTarget::Fighter(0)) > 0.0,
                tactical.fighter_target_strength(0, OriginalFighterTarget::Fighter(1)) > 0.0,
            )
        };
        assert_eq!(
            check(&tactical),
            (true, true, Some(target), true, true, true)
        );
        let mut jumping = tactical.clone();
        jumping.fighters[1].retreating = true;
        jumping.fighters[1].retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
        assert_eq!(check(&jumping), (false, false, None, false, false, false));
    }

    // FUN_005ad530 posts event 0x3c (state 8) for a withdrawing fighter
    // group on the trench run (order 6): FUN_005cfec0 ends the run without
    // a result and gives each participant back the hull and shields that
    // FUN_005d08d0 kept when the run began; the group then withdraws.
    #[test]
    fn withdrawing_aborts_the_trench_run() {
        let mut ds_ship = ship(false, TacticalWeaponArc::default(), 0.0);
        ds_ship.hull_current = 1_000_000;
        ds_ship.hull_max = 1_000_000;
        let mut runner = fighter(true, TacticalWeaponArc::default());
        runner.order = TacticalOrder::AttackDeathStar;
        runner.has_hyperdrive = true;
        // Maneuverability 0: the run's maneuver draw always hits it.
        runner.maneuverability = 0;
        let mut tactical = session(vec![ds_ship], vec![runner]);
        tactical.death_star = Some(crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: crate::tactical::TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        let hull = tactical.fighters[0].hull_current;
        tactical.trench_run_remaining_ms =
            Some(crate::tactical::ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);
        tactical.prepare_original_trench_run();
        assert!(tactical.trench_run_pending_outcome.is_some());
        assert!(tactical.fighters[0].hull_current < hull);
        // The other side's withdrawal leaves the run alone, even with the
        // runner still flying it.
        let mut flying = tactical.clone();
        flying.trench_run_participants.clear();
        flying.fighters[0].alive = true;
        flying.fighters[0].squad_count = 1;
        flying.withdraw_fighters(false);
        assert!(flying.trench_run_remaining_ms.is_some());
        // With no list, a live runner of the side still marks its run.
        flying.withdraw_fighters(true);
        assert_eq!(flying.trench_run_remaining_ms, None);
        tactical.begin_player_withdrawal();
        tactical.withdraw_fighters(true);
        assert_eq!(tactical.trench_run_remaining_ms, None);
        assert_eq!(tactical.trench_run_pending_outcome, None);
        assert!(tactical.trench_run_participants.is_empty());
        let runner = &tactical.fighters[0];
        assert_eq!(runner.hull_current, hull);
        assert!(runner.alive && runner.retreating);
        assert_eq!(runner.order, TacticalOrder::None);
        // Nothing resolves the run afterwards.
        for _ in 0..4 {
            tactical.step();
        }
        assert!(!tactical.death_star.unwrap().destroyed);
        assert_eq!(tactical.trench_run_outcome, None);
    }

    // FUN_005a6b40 draws a jumping unit at z - side_z * k * (0.001 t)^2 over
    // its 4000 ms, k by (id + slot +0x24) & 3 (port: the list index), and
    // queues the jump's sound on its first frame: event 0x1f (WAVE 13065)
    // for a capital ship, 0x1d (13063) for a fighter group.
    #[test]
    fn a_jump_flies_its_unit_away_and_sounds_once() {
        use crate::tactical::{TacticalAudioCue, TacticalAudioEvent, ORIGINAL_JUMP_ACCELERATIONS};
        let mut own = jumper(true, TacticalWeaponArc::default(), 0.0);
        own.subsystem_condition.hyperdrive = 100;
        own.retreating = true;
        let mut wing = fighter(true, TacticalWeaponArc::default());
        wing.has_hyperdrive = true;
        wing.retreating = true;
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut tactical = session(vec![own, enemy], vec![wing]);
        let exit = tactical.withdrawal_exit_z(true);
        tactical.ships[0].source_position.z = exit;
        tactical.fighters[0].source_position.z = exit;
        tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        // The time ends at turn 1; the jumps are decided in the first frame
        // step, step 12 (state 6, no sound yet, stamped 5); their events 0x2d
        // run at turn 6, step 24, where the flights start.
        for _ in 0..12 {
            tactical.step();
        }
        assert!(tactical.take_pending_audio_cues().is_empty());
        assert_eq!(step_until(&mut tactical, |t| t.ships[0].jumping()), 24);
        assert_eq!(
            tactical.take_pending_audio_cues(),
            vec![
                TacticalAudioCue::from_variant(TacticalAudioEvent::FighterJump, 0),
                TacticalAudioCue::from_variant(TacticalAudioEvent::CapitalJump, 0),
            ]
        );
        assert_eq!(TacticalAudioEvent::CapitalJump.resource_base(), 13_065);
        assert_eq!(TacticalAudioEvent::FighterJump.resource_base(), 13_063);
        let mut last = tactical.ships[0].source_position.z;
        for _ in 0..15 {
            tactical.step();
            let z = tactical.ships[0].source_position.z;
            assert!(z < last);
            last = z;
        }
        // State 8 holds where the flight ended until its event 0x2e
        // (stamped 12 on step 39) runs at turn 13, step 52.
        assert_eq!(step_until(&mut tactical, |t| t.ships[0].retreated), 52);
        assert_eq!(tactical.ships[0].source_position.z, last);
        assert!(tactical.take_pending_audio_cues().is_empty());
        // 4 s at k = 25 (DAT_006bc47c = 400 / 4^2): 400 units toward -z.
        assert_eq!(ORIGINAL_JUMP_ACCELERATIONS[0], 25.0);
        assert!((tactical.ships[0].source_position.z - (exit - 400.0)).abs() < 1e-3);
        assert!(!tactical.fighters[0].alive && tactical.fighters[0].squad_count > 0);
        assert!((tactical.fighters[0].source_position.z - (exit - 400.0)).abs() < 1e-3);
    }

    // 0x5a69b0..0x5a6a7c: each factor carries its unit its distance by the
    // end of its time, 400 by 4000 ms, 575 by 3750, 650 by 3925 and 325 by
    // 3725 (_DAT_0066c24c less 0, 250, 75, 275). One variant each (hyp).
    #[test]
    fn the_jump_factors_reach_their_distances() {
        use crate::tactical::{TacticalAudioEvent, ORIGINAL_JUMP_ACCELERATIONS};
        for (factor, (distance, ms)) in ORIGINAL_JUMP_ACCELERATIONS.into_iter().zip([
            (400.0, 4000.0),
            (575.0, 3750.0),
            (650.0, 3925.0),
            (325.0, 3725.0),
        ]) {
            let seconds: f32 = ms * 0.001;
            assert!(
                (factor * seconds * seconds - distance).abs() < 1e-2,
                "{factor}"
            );
        }
        assert_eq!(TacticalAudioEvent::FighterJump.variant_count(), 1);
        assert_eq!(TacticalAudioEvent::CapitalJump.variant_count(), 1);
    }

    // FUN_005b1b70 wants engine power (slot +0x90) as well as a hyperdrive:
    // a ship without it cannot jump and is destroyed at the side's end
    // (event 0x2f); one already in its jump is no longer in the fight
    // (FUN_005b2720 asks FUN_005ad870) and finishes it. One that loses it
    // while its event 0x2d waits is destroyed by that event (FUN_005b1e40
    // -> FUN_005b2720).
    #[test]
    fn a_ship_needs_engine_power_to_jump() {
        let decided = |t: &BattleSession| {
            t.turn_event_pending(&crate::tactical::TacticalTurnEvent::JumpStart(
                crate::tactical::TacticalUnitRef::Ship(1),
            ))
        };
        let run = |power_after_start: bool| {
            let mut stalled = jumper(true, TacticalWeaponArc::default(), 0.0);
            stalled.subsystem_condition.hyperdrive = 100;
            stalled.retreating = true;
            stalled.sublight_engine_power = 0.0;
            let mut mover = jumper(true, TacticalWeaponArc::default(), 0.0);
            mover.subsystem_condition.hyperdrive = 100;
            mover.retreating = true;
            // Its own task force: 0x005c6070 tries a force's first member only.
            mover.task_force = 1;
            let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
            enemy.hull_current = 1_000_000;
            enemy.hull_max = 1_000_000;
            let mut tactical = session(vec![stalled, mover, enemy], Vec::new());
            let exit = tactical.withdrawal_exit_z(true);
            tactical.ships[0].source_position.z = exit;
            tactical.ships[1].source_position.z = exit;
            tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
            // Decided at step 12 (stamped 5), it jumps at turn 6, step 24.
            for _ in 0..12 {
                tactical.step();
            }
            assert_eq!(tactical.ships[0].retreat_progress, 0.0);
            assert!(decided(&tactical));
            assert_eq!(step_until(&mut tactical, |t| t.ships[1].jumping()), 24);
            if !power_after_start {
                tactical.ships[1].sublight_engine_power = 0.0;
            }
            assert_eq!(step_until(&mut tactical, |t| !t.ships[1].alive), 52);
            tactical
        };
        for power_after_start in [true, false] {
            let tactical = run(power_after_start);
            assert!(!tactical.ships[0].alive && !tactical.ships[0].retreated);
            assert!(tactical.ships[1].retreated && tactical.ships[1].hull_current > 0);
            assert_eq!(tactical.withdrawal_ms, None);
        }

        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut lost = jumper(true, TacticalWeaponArc::default(), 0.0);
        lost.subsystem_condition.hyperdrive = 100;
        lost.retreating = true;
        // A force still on its way keeps the side waiting (state 3).
        let mut far = jumper(true, TacticalWeaponArc::default(), 0.0);
        far.subsystem_condition.hyperdrive = 100;
        far.retreating = true;
        far.task_force = 2;
        let mut tactical = session(vec![enemy, lost, far], Vec::new());
        tactical.ships[1].source_position.z = tactical.withdrawal_exit_z(true);
        tactical.ships[2].source_position.z = tactical.withdrawal_exit_z(true) + 300.0;
        tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        for _ in 0..12 {
            tactical.step();
        }
        assert!(decided(&tactical));
        tactical.ships[1].sublight_engine_power = 0.0;
        assert_eq!(step_until(&mut tactical, |t| !t.ships[1].alive), 24);
        assert!(!tactical.ships[1].retreated && tactical.ships[1].hull_current == 0);
        assert!(!tactical.ships[1].jumping());
    }

    // port: a side stays in the battle while it has a ship, a fighter group
    // with craft, or a Death Star not destroyed; the side left with them
    // wins, and with none on either side it is a draw.
    #[test]
    fn the_battle_ends_when_a_side_has_nothing_left_in_it() {
        use crate::tactical::CombatWinner;
        let death_star = |is_attacker: bool, destroyed: bool| crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker,
            is_alliance: false,
            source_position: crate::tactical::TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed,
            action_committed: false,
        };
        let wing = |alliance: bool, craft: u32| {
            let mut value = fighter(alliance, TacticalWeaponArc::default());
            value.is_attacker = alliance;
            value.squad_count = craft;
            value
        };
        let defender = || {
            let mut value = ship(false, TacticalWeaponArc::default(), 0.0);
            value.is_attacker = false;
            value
        };
        let attacker = || {
            let mut value = ship(true, TacticalWeaponArc::default(), 0.0);
            value.is_attacker = true;
            value
        };
        let outcome = |mut tactical: BattleSession| {
            tactical.step();
            tactical.winner
        };
        // The attacker's fighter group with craft keeps it in.
        assert_eq!(outcome(session(vec![defender()], vec![wing(true, 3)])), None);
        // One without craft does not: the defender wins.
        assert_eq!(
            outcome(session(vec![defender()], vec![wing(true, 0)])),
            Some(CombatWinner::Defender)
        );
        assert_eq!(outcome(session(vec![attacker()], vec![wing(false, 3)])), None);
        assert_eq!(
            outcome(session(vec![attacker()], vec![wing(false, 0)])),
            Some(CombatWinner::Attacker)
        );
        // A Death Star keeps its side in until it is destroyed.
        for is_attacker in [true, false] {
            let other = if is_attacker { defender() } else { attacker() };
            let mut kept = session(vec![other.clone()], Vec::new());
            kept.death_star = Some(death_star(is_attacker, false));
            assert_eq!(outcome(kept), None);
            let mut lost = session(vec![other], Vec::new());
            lost.death_star = Some(death_star(is_attacker, true));
            let winner = if is_attacker {
                CombatWinner::Defender
            } else {
                CombatWinner::Attacker
            };
            assert_eq!(outcome(lost), Some(winner));
        }
    }

    // The ready voice (0x113) sounds on the player's not-ready to ready
    // edge only: not each turn while the laser stays full.
    #[test]
    fn a_full_laser_does_not_repeat_its_ready_voice() {
        let mut tactical = session(
            vec![ship(true, TacticalWeaponArc::default(), 0.0), ship(false, TacticalWeaponArc::default(), 0.0)],
            Vec::new(),
        );
        tactical.death_star = Some(crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: tactical.player_is_attacker,
            is_alliance: false,
            source_position: crate::tactical::TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 100.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        for _ in 0..8 {
            tactical.step();
        }
        assert!(!tactical
            .take_pending_voice_cues()
            .contains(&crate::tactical::TacticalVoiceCue::death_star_ready()));
    }

    fn trench_death_star(is_alliance: bool) -> crate::tactical::TacticalDeathStar {
        crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance,
            source_position: crate::tactical::TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        }
    }

    // FUN_005c6610's approach moves the force's members (alive, with craft,
    // of the run's group, on order 6) by one step at the slowest member's
    // power toward its point, (-side.z, 0, side.x) * 0.5 extent: -x for the
    // Alliance (side vector (0, 0, 1)), +x for the Empire.
    #[test]
    fn a_trench_run_approach_moves_only_its_group_toward_its_side_s_point() {
        for alliance in [true, false] {
            let mut enemy = ship(!alliance, TacticalWeaponArc::default(), 0.0);
            enemy.hull_current = 1_000_000;
            enemy.hull_max = 1_000_000;
            let member = |power: f32| {
                let mut value = fighter(alliance, TacticalWeaponArc::default());
                value.order = TacticalOrder::AttackDeathStar;
                value.fighter_group = 1;
                value.sublight_engine_power = power;
                value
            };
            let mut other_group = member(1.0);
            other_group.fighter_group = 2;
            let mut empty = member(1.0);
            empty.squad_count = 0;
            let mut unordered = member(1.0);
            unordered.order = TacticalOrder::None;
            let mut dead = member(1.0);
            dead.alive = false;
            let mut tactical = session(
                vec![enemy],
                vec![member(1.0), member(2.0), other_group, empty, unordered, dead],
            );
            tactical.death_star = Some(trench_death_star(!alliance));
            let half = tactical.source_layout.battle_extent * 0.5;
            let point_x = if alliance { -half } else { half };
            let start = crate::tactical::TacticalWorldPosition {
                x: point_x + 2.0,
                y: 2.0,
                z: 1.0,
            };
            for fighter in &mut tactical.fighters {
                fighter.source_position = start;
            }
            // The approach is under way (state 2 has run); the frame's step
            // runs from turn 3, step 12.
            tactical.trench_run_group = Some(1);
            tactical.combat_tick = 11;
            tactical.step();
            // Three from the point at speed 1: 250 ms along (-2, -2, -1) / 3,
            // in 60 Hz frames (port).
            assert!((crate::tactical::PORT_MOVER_FRAME_MILLISECONDS * 60.0 - 1000.0).abs() < 1e-3);
            let moved = crate::tactical::TacticalWorldPosition {
                x: point_x + 2.0 - 0.25 * 2.0 / 3.0,
                y: 2.0 - 0.25 * 2.0 / 3.0,
                z: 1.0 - 0.25 / 3.0,
            };
            for index in [0, 1] {
                let at = tactical.fighters[index].source_position;
                assert!(
                    (at.x - moved.x).abs() < 1e-4
                        && (at.y - moved.y).abs() < 1e-4
                        && (at.z - moved.z).abs() < 1e-4,
                    "{alliance} {at:?}"
                );
            }
            for index in 2..6 {
                assert_eq!(tactical.fighters[index].source_position, start);
            }
        }
    }

    // FUN_005d0340: the order is heard by the Alliance's player for its own
    // group, or by the Empire's for its own Death Star; otherwise the port
    // starts no run. FUN_005cfec0 state 8 / FUN_005d0950: a withdrawal
    // during the approach ends the run with its queued states, and the AI
    // commander does not withdraw while it is under way (FUN_005d0ae0).
    #[test]
    fn a_trench_run_order_is_heard_by_its_side_and_ends_with_a_withdrawal() {
        use crate::tactical::TacticalTurnEvent;
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        // The Alliance player (the attacker) and the Imperial Death Star.
        let mut runner = fighter(true, TacticalWeaponArc::default());
        runner.order = TacticalOrder::AttackDeathStar;
        let mut tactical = session(vec![enemy.clone()], vec![runner]);
        tactical.death_star = Some(trench_death_star(false));
        assert!(!tactical.begin_original_trench_run(0, false));
        assert!(!tactical.trench_run_under_way());
        assert!(tactical.begin_original_trench_run(0, true));
        assert!(tactical.trench_run_under_way());
        // The Empire's player hears it for its own Death Star only.
        let mut imperial = session(vec![enemy.clone()], Vec::new());
        imperial.player_is_attacker = false;
        imperial.death_star = Some(trench_death_star(false));
        assert!(imperial.begin_original_trench_run(0, true));
        assert_eq!(
            imperial.take_pending_voice_cues(),
            vec![crate::tactical::TacticalVoiceCue::death_star_under_attack()]
        );
        let mut foreign = session(vec![enemy], Vec::new());
        foreign.player_is_attacker = false;
        let mut owned = trench_death_star(false);
        owned.is_attacker = true;
        foreign.death_star = Some(owned);
        assert!(!foreign.begin_original_trench_run(0, true));

        // A withdrawal during the approach ends it and its queued state.
        tactical.withdraw_fighters(tactical.player_is_attacker);
        assert!(!tactical.trench_run_under_way());
        assert!(!tactical.turn_event_pending(&TacticalTurnEvent::TrenchRunState(2)));
    }

    // FUN_005c6610's order 6: once the run's state 2 has run (posted at
    // the run's creation, 0x005d0884), the force flies each turn toward
    // (-side.z, 0, side.x) * 0.5 extent with the move's 0.2 radius
    // (0x005c68f0); on arrival it posts state 5 (0x005c6928), whose turn
    // starts the 120 s run and its follow-up line (FUN_005cfec0).
    #[test]
    fn a_trench_run_flies_its_approach_before_the_run_begins() {
        use crate::tactical::{TacticalTurnEvent, TacticalVoiceCue};
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut wing = fighter(true, TacticalWeaponArc::default());
        wing.order = TacticalOrder::AttackDeathStar;
        wing.fighter_group = 2;
        wing.sublight_engine_power = 1.0;
        let mut tactical = session(vec![enemy], vec![wing]);
        tactical.death_star = Some(crate::tactical::TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: crate::tactical::TacticalWorldPosition::ORIGIN,
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        });
        let half = tactical.source_layout.battle_extent * 0.5;
        tactical.fighters[0].source_position = crate::tactical::TacticalWorldPosition {
            x: -half + 2.0,
            y: 0.0,
            z: 0.0,
        };
        assert!(tactical.begin_original_trench_run(2, true));
        assert_eq!(
            tactical.take_pending_voice_cues(),
            vec![TacticalVoiceCue::trench_run_started(2)]
        );
        assert!(tactical.trench_run_under_way());
        // State 2, stamped 3, runs at turn 4 (step 16); the frame's order
        // step then flies 1.0 a second.
        for _ in 0..15 {
            tactical.step();
        }
        assert_eq!(tactical.fighters[0].source_position.x, -half + 2.0);
        tactical.step();
        assert!((tactical.fighters[0].source_position.x - (-half + 1.75)).abs() < 1e-3);
        // Within 0.2 after 1.8 units, 108 frames: during step 23, stamped 8
        // (turn 5 plus the lead), so state 5 runs at turn 9, step 36.
        for _ in 0..6 {
            tactical.step();
        }
        assert!(!tactical.turn_event_pending(&TacticalTurnEvent::TrenchRunState(5)));
        tactical.step();
        assert_eq!(tactical.combat_tick, 23);
        assert!(tactical.turn_event_pending(&TacticalTurnEvent::TrenchRunState(5)));
        assert_eq!(tactical.turn_events[0].stamp, 8);
        assert_eq!(tactical.trench_run_remaining_ms, None);
        tactical.take_pending_voice_cues();
        for _ in 0..13 {
            tactical.step();
        }
        assert_eq!(tactical.combat_tick, 36);
        assert!(tactical.trench_run_remaining_ms.is_some());
        assert!(tactical
            .take_pending_voice_cues()
            .contains(&TacticalVoiceCue::trench_run_committed(2)));
    }

    // The frame's slot +0x98 (0x005a9ec0) runs only once +0x878 has
    // reached +0x8ac (0x005bef1d): a ship given a place to go holds still
    // for the first three turns and moves from step 12.
    #[test]
    fn nothing_moves_for_the_first_three_turns() {
        let mut mover = jumper(true, TacticalWeaponArc::default(), 0.0);
        mover.sublight_engine_power = 10.0;
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut tactical = session(vec![mover, enemy], Vec::new());
        tactical.ships[0].source_waypoint = Some(crate::tactical::TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 40.0,
        });
        let start = tactical.ships[0].source_position;
        for _ in 0..11 {
            tactical.step();
        }
        assert_eq!(tactical.ships[0].source_position, start);
        tactical.step();
        assert_eq!(tactical.event_turn(), 3);
        assert_ne!(tactical.ships[0].source_position, start);
    }

    // 0x005a9ec0 (0x005aa089..0x005aa17a) holds each unit in the fight
    // within +-2/3 of the battle extent on each axis (_DAT_0066c290,
    // _DAT_0066c294); a unit in its jump flight (state 7) is not held.
    #[test]
    fn a_unit_in_the_fight_is_held_within_the_field() {
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let outside = ship(true, TacticalWeaponArc::default(), 0.0);
        let mut flying = jumper(true, TacticalWeaponArc::default(), 0.0);
        flying.retreat_progress = 1_000.0;
        let wing = fighter(true, TacticalWeaponArc::default());
        let mut tactical = session(vec![enemy, outside, flying], vec![wing]);
        // The float at 0x66c294.
        assert_eq!(
            crate::tactical::ORIGINAL_FIELD_LIMIT_EXTENT_SCALE.to_bits(),
            0x3f2a_aaab
        );
        let limit =
            tactical.source_layout.battle_extent * crate::tactical::ORIGINAL_FIELD_LIMIT_EXTENT_SCALE;
        let far = crate::tactical::TacticalWorldPosition {
            x: 3.0 * limit,
            y: -3.0 * limit,
            z: 3.0 * limit,
        };
        tactical.ships[1].source_position = far;
        tactical.ships[2].source_position = far;
        tactical.fighters[0].source_position = far;
        for _ in 0..12 {
            tactical.step();
        }
        let held = crate::tactical::TacticalWorldPosition {
            x: limit,
            y: -limit,
            z: limit,
        };
        assert_eq!(tactical.ships[1].source_position, held);
        assert_eq!(tactical.fighters[0].source_position, held);
        let flown = tactical.ships[2].source_position;
        assert!(flown.x > limit && flown.y < -limit && flown.z > limit);
    }

    // FUN_005b3f10 posts each shot's fire event at +0x87c + 1.0 and its hit
    // at + 2.0; the fire event (FUN_005b1f60) needs both units in the fight
    // (FUN_005ad870), the hit (0x005b2ac0) the target.
    #[test]
    fn a_shot_fires_and_hits_on_its_turns_while_its_units_fight() {
        use crate::tactical::{
            OriginalFighterTarget, TacticalShot, TacticalTurnEvent, TacticalUnitRef, WeaponKind,
        };
        let mut shooter = ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0);
        shooter.weapon_arcs = [TacticalWeaponArc::new(0, 0, 4); 4];
        shooter.weapon_ranges = TacticalWeaponRanges {
            laser_cannon: 100.0,
            ion_cannon: 100.0,
            turbolaser: 100.0,
        };
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        target.source_position.z = 10.0;
        target.hull_current = 1_000_000;
        target.hull_max = 1_000_000;
        let mut tactical = session(vec![shooter, target], Vec::new());
        for _ in 0..4 {
            tactical.step();
        }
        // Decided in turn 1's updates: +0x87c is 3 there.
        let stamps = tactical
            .turn_events
            .iter()
            .map(|queued| match queued.event {
                TacticalTurnEvent::WeaponFire(_) => ("fire", queued.stamp),
                TacticalTurnEvent::WeaponHit(_) => ("hit", queued.stamp),
                _ => ("other", queued.stamp),
            })
            .collect::<Vec<_>>();
        assert_eq!(stamps, [("fire", 4), ("hit", 5)]);

        let shot = TacticalShot {
            source: TacticalUnitRef::Ship(0),
            target: OriginalFighterTarget::Capital(1),
            kind: WeaponKind::Turbolaser,
            strength: 50.0,
        };
        // A source out of the fight shows no projectile.
        let mut gone = session(vec![ship(true, TacticalWeaponArc::default(), 0.0), ship(false, TacticalWeaponArc::default(), 0.0)], Vec::new());
        gone.ships[0].alive = false;
        gone.fire_weapon_shot(shot);
        assert!(gone.weapon_effects.is_empty());
        assert!(gone.pending_audio_cues.is_empty());
        // A target out of the fight takes no hit.
        let mut jumped = session(vec![ship(true, TacticalWeaponArc::default(), 0.0), ship(false, TacticalWeaponArc::default(), 0.0)], Vec::new());
        jumped.ships[1].retreat_progress = 1_000.0;
        let before = (jumped.ships[1].hull_current, jumped.ships[1].shield);
        jumped.hit_weapon_shot(shot);
        assert_eq!((jumped.ships[1].hull_current, jumped.ships[1].shield), before);
        assert!(jumped.impact_effects.is_empty());
        // In the fight, both apply.
        let mut fighting = session(vec![ship(true, TacticalWeaponArc::default(), 0.0), ship(false, TacticalWeaponArc::default(), 0.0)], Vec::new());
        fighting.ships[1].source_position.z = 10.0;
        fighting.fire_weapon_shot(shot);
        assert_eq!(fighting.weapon_effects.len(), 1);
        fighting.hit_weapon_shot(shot);
        assert_eq!(fighting.impact_effects.len(), 1);
    }

    // The hit event (0x005b2a20 and its siblings) calls the target's slot
    // +0xe0 only while it is in the fight (FUN_005ad870): a fighter group
    // alive with craft and not jumping, a Death Star not destroyed with
    // hull. A capital source hits a fighter group with its maneuverability
    // + 1 (FUN_005b3f10); a torpedo is 8.5/16 a point (FUN_005b2c70) and its
    // impact plays variant 0 without a draw; every sound is one of its
    // event's variants (FUN_005bae60).
    #[test]
    fn shot_events_reach_each_kind_of_target_while_it_fights() {
        use crate::tactical::{
            apply_original_fighter_damage, OriginalFighterTarget, TacticalShot, TacticalUnitRef,
            WeaponKind, ORIGINAL_FIGHTER_TORPEDO_STRENGTH,
        };
        let shot = |source, target, kind, strength| TacticalShot {
            source,
            target,
            kind,
            strength,
        };
        // Fighter targets.
        let mut capital = ship(true, TacticalWeaponArc::default(), 0.0);
        capital.maneuverability = 3;
        let wing = || {
            let mut value = fighter(false, TacticalWeaponArc::default());
            value.shield = 0.0;
            value.shield_max = 0.0;
            value
        };
        let mut empty = wing();
        empty.squad_count = 0;
        let mut jumping = wing();
        jumping.retreat_progress = 1_000.0;
        let mut tactical = session(vec![capital], vec![wing(), empty, jumping]);
        let mut expected = tactical.fighters.clone();
        apply_original_fighter_damage(&mut expected, 0, 30.0, 4.0);
        for index in 0..3 {
            tactical.hit_weapon_shot(shot(
                TacticalUnitRef::Ship(0),
                OriginalFighterTarget::Fighter(index),
                WeaponKind::LaserCannon,
                30.0,
            ));
        }
        assert_eq!(tactical.fighters[0].hull_current, expected[0].hull_current);
        assert!(tactical.fighters[0].hull_current < 96.0);
        assert_eq!(tactical.fighters[1].hull_current, 96.0);
        assert_eq!(tactical.fighters[2].hull_current, 96.0);
        // Each sound draws its variant from 0 through its count - 1, fire
        // then impact (FUN_005bae60's counts).
        tactical.pending_audio_cues.clear();
        let mut draws = tactical.tactical_rng;
        let expected = (0..3)
            .flat_map(|_| {
                [
                    crate::tactical::TacticalAudioEvent::LaserFire,
                    crate::tactical::TacticalAudioEvent::LaserImpact,
                ]
            })
            .map(|event| {
                let variant = draws.inclusive(0, u32::from(event.variant_count() - 1));
                event.resource_base() + variant
            })
            .collect::<Vec<_>>();
        for _ in 0..3 {
            tactical.fire_weapon_shot(shot(
                TacticalUnitRef::Ship(0),
                OriginalFighterTarget::Fighter(0),
                WeaponKind::LaserCannon,
                0.0,
            ));
            tactical.hit_weapon_shot(shot(
                TacticalUnitRef::Ship(0),
                OriginalFighterTarget::Fighter(0),
                WeaponKind::LaserCannon,
                0.0,
            ));
        }
        assert_eq!(
            tactical.pending_audio_cues.iter().map(|cue| cue.resource_id).collect::<Vec<_>>(),
            expected
        );
        // Every sound is one of its event's variants.
        for _ in 0..40 {
            tactical.fire_weapon_shot(shot(
                TacticalUnitRef::Ship(0),
                OriginalFighterTarget::Fighter(0),
                WeaponKind::LaserCannon,
                0.001,
            ));
            tactical.hit_weapon_shot(shot(
                TacticalUnitRef::Ship(0),
                OriginalFighterTarget::Fighter(0),
                WeaponKind::LaserCannon,
                0.001,
            ));
        }
        assert!(tactical.pending_audio_cues.iter().all(|cue| {
            let base = cue.event.resource_base();
            cue.resource_id >= base && cue.resource_id < base + u32::from(cue.event.variant_count())
        }));

        // The Death Star.
        let mut star = session(vec![ship(false, TacticalWeaponArc::default(), 0.0)], vec![fighter(true, TacticalWeaponArc::default())]);
        star.death_star = Some(trench_death_star(false));
        let torpedo = shot(
            TacticalUnitRef::Fighter(0),
            OriginalFighterTarget::DeathStar,
            WeaponKind::FighterAttack,
            30.0,
        );
        let rng = star.tactical_rng;
        star.hit_weapon_shot(torpedo);
        assert_eq!(star.death_star.unwrap().hull, 70.0);
        assert_eq!(star.tactical_rng, rng);
        assert_eq!(star.pending_audio_cues.last().unwrap().resource_id, crate::tactical::TacticalAudioEvent::TorpedoImpact.resource_base());
        star.hit_weapon_shot(TacticalShot { strength: 70.0, ..torpedo });
        assert!(star.death_star.unwrap().destroyed);
        assert_eq!(star.death_star.unwrap().hull, 0.0);
        // Out of the fight: destroyed, or without hull.
        star.hit_weapon_shot(torpedo);
        assert_eq!(star.death_star.unwrap().hull, 0.0);
        let mut empty_star = session(Vec::new(), vec![fighter(true, TacticalWeaponArc::default())]);
        let mut hulk = trench_death_star(false);
        hulk.hull = 0.0;
        empty_star.death_star = Some(hulk);
        empty_star.hit_weapon_shot(torpedo);
        assert!(!empty_star.death_star.unwrap().destroyed);

        // A torpedo's strength is its rating times 8.5 / 16.
        let mut bomber = fighter(true, TacticalWeaponArc::new(8, 6, 0));
        bomber.weapon_ranges = TacticalWeaponRanges {
            laser_cannon: 100.0,
            ion_cannon: 100.0,
            turbolaser: 0.0,
        };
        bomber.torpedo_strength = 16;
        bomber.torpedo_range = 100.0;
        bomber.order = TacticalOrder::AttackCapitalShips;
        bomber.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(0));
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        target.shield = 0;
        target.source_position.z = 5.0;
        let mut run = session(vec![target], vec![bomber]);
        let mut shots = Vec::new();
        run.fighter_step(&mut shots);
        assert_eq!(
            shots.iter().find(|shot| shot.kind == WeaponKind::FighterAttack).map(|shot| shot.strength),
            Some(16.0 * ORIGINAL_FIGHTER_TORPEDO_STRENGTH)
        );
    }

    // FUN_005b5f50 (the hit's slot +0xe0) takes the shield and hull at once
    // and posts each subsystem hit as event 0x1e..0x22 through slot +0x28;
    // its handler (0x005b28d0 and siblings) calls slot +0xa0.
    #[test]
    fn a_hit_posts_its_subsystem_damage_through_the_queue() {
        use crate::tactical::{
            OriginalFighterTarget, TacticalShot, TacticalSubsystemKind, TacticalTurnEvent,
            TacticalUnitRef, WeaponKind,
        };
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        target.shield = 10;
        target.shield_max = 10;
        target.hull_current = 1_000;
        target.hull_max = 1_000;
        target.subsystem_capacity.shields = true;
        target.refresh_subsystem_condition();
        let mut tactical = session(vec![ship(true, TacticalWeaponArc::default(), 0.0), target], Vec::new());
        tactical.combat_tick = 20;
        tactical.hit_weapon_shot(TacticalShot {
            source: TacticalUnitRef::Ship(0),
            target: OriginalFighterTarget::Capital(1),
            kind: WeaponKind::LaserCannon,
            strength: 10.0,
        });
        // The shield is gone at once; its subsystem hit waits in the queue,
        // stamped +0x87c within the turn's events (turn 5 + 2).
        assert_eq!(tactical.ships[1].shield, 0);
        assert_eq!(tactical.ships[1].subsystem_damage.shields, 0);
        assert_eq!(
            tactical.turn_events,
            vec![crate::tactical::TacticalQueuedEvent {
                stamp: 7,
                event: TacticalTurnEvent::SubsystemHit {
                    ship: 1,
                    kind: TacticalSubsystemKind::Shields,
                },
            }]
        );
        tactical.hit_subsystem(1, TacticalSubsystemKind::Shields);
        assert_eq!(tactical.ships[1].subsystem_damage.shields, 1);
        assert!(tactical.ships[1].subsystem_condition.shields < 100);
    }

    // FUN_005ab650 places a side's units at x 0, 5, -5, -10, 10, 15, ...
    // (DAT_0066c27c, DAT_0066c2a4), capital ships at -+0.5 extent and fighter
    // groups 20 inside (DAT_0066c29c, DAT_0066c2a8, DAT_0066c2ac).
    #[test]
    fn units_take_the_original_slots_and_lanes() {
        use crate::tactical::OriginalTacticalSlots;
        assert_eq!(
            OriginalTacticalSlots::default().take(9).collect::<Vec<_>>(),
            [0.0, 5.0, -5.0, -10.0, 10.0, 15.0, -15.0, -20.0, 20.0]
        );
        let layout = crate::tactical::OriginalTacticalLayout::from_active_counts(1, 1);
        assert_eq!(layout.battle_extent, 103.0);
        assert_eq!(layout.outer_negative_z, -51.5);
        assert_eq!(layout.outer_positive_z, 51.5);
        assert_eq!(layout.inner_negative_z, -31.5);
        assert_eq!(layout.inner_positive_z, 31.5);
    }

    // FUN_005ab650: the extent starts at 100.0 (0x005ab683) and grows by
    // 3.0 (-DAT_0066c2a0) for each unit of the larger side.
    #[test]
    fn the_battle_extent_grows_by_three_a_unit_of_the_larger_side() {
        use crate::tactical::OriginalTacticalLayout;
        assert_eq!(OriginalTacticalLayout::from_active_counts(0, 0).battle_extent, 100.0);
        assert_eq!(OriginalTacticalLayout::from_active_counts(4, 7).battle_extent, 121.0);
        assert_eq!(OriginalTacticalLayout::from_active_counts(9, 2).battle_extent, 127.0);
    }

    // FUN_005a96b0 runs at a turn's start (every fourth step) the events
    // stamped +0x878 - 1, only once +0x878 has reached +0x8ac (3), and
    // drops older ones (0x005bec3c, 0x005bec7e).
    #[test]
    fn queued_events_run_at_their_turn_from_the_third() {
        use crate::tactical::{TacticalTurnEvent, TacticalUnitRef};
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let own = || ship(true, TacticalWeaponArc::default(), 0.0);
        let mut tactical = session(vec![enemy, own(), own(), own(), own()], Vec::new());
        for (stamp, index) in [(0, 1), (1, 2), (2, 3), (3, 4)] {
            tactical.post_turn_event(stamp, TacticalTurnEvent::JumpEnd(TacticalUnitRef::Ship(index)));
        }
        let alive = |t: &BattleSession| (1..5).map(|i| t.ships[i].alive).collect::<Vec<_>>();
        for _ in 0..11 {
            tactical.step();
        }
        assert_eq!(alive(&tactical), [true; 4]);
        // Turn 3, step 12: stamp 2 runs; stamps 0 and 1 are dropped.
        tactical.step();
        assert_eq!(alive(&tactical), [true, true, false, true]);
        assert_eq!(tactical.turn_events.len(), 1);
        // Stamp 3 waits for turn 4, step 16.
        for _ in 0..3 {
            tactical.step();
        }
        assert!(tactical.ships[4].alive);
        tactical.step();
        assert!(!tactical.ships[4].alive);
        assert!(tactical.turn_events.is_empty());
    }

    // The manoeuvre executors (FUN_005ca6d0 -> FUN_005a8f70) aim at the
    // opponent furthest along the side vector of the opponents' centre:
    // turned left (Left Hook), right (Right Hook), mirrored in y (Hammer) or
    // straight (Anvil); the first of equal scores wins.
    #[test]
    fn each_manoeuvre_aims_at_the_opponent_furthest_along_its_side() {
        use crate::tactical::{original_maneuver_target, TacticalOrder};
        let at = |x: f32, y: f32, z: f32| TacticalWorldPosition { x, y, z };
        let mut own = ship(true, TacticalWeaponArc::default(), 0.0);
        own.source_position = at(-4.0, -4.0, -50.0);
        let mut ships = vec![own];
        for position in [
            at(17.0, 22.0, 67.0),
            at(16.0, -10.0, 64.0),
            at(20.0, -24.0, 43.0),
            at(-26.0, 28.0, 37.0),
            at(17.0, 22.0, 67.0),
        ] {
            let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
            enemy.source_position = position;
            ships.push(enemy);
        }
        let target = |order| original_maneuver_target(&ships, 0, order);
        assert_eq!(target(TacticalOrder::LeftHook), Some(at(-26.0, 28.0, 37.0)));
        assert_eq!(
            target(TacticalOrder::RightHook),
            Some(at(20.0, -24.0, 43.0))
        );
        assert_eq!(target(TacticalOrder::Hammer), Some(at(16.0, -10.0, 64.0)));
        assert_eq!(target(TacticalOrder::Anvil), Some(at(17.0, 22.0, 67.0)));
        assert_eq!(target(TacticalOrder::HoldPosition), None);

        // Two opponents at the same score: the first is kept.
        let mut level = vec![ships[0].clone()];
        for position in [
            at(-10.0, 5.0, 10.0),
            at(-10.0, -5.0, 10.0),
            at(10.0, 5.0, 10.0),
            at(10.0, -5.0, 10.0),
        ] {
            let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
            enemy.source_position = position;
            level.push(enemy);
        }
        level[0].source_position = at(0.0, 0.0, 0.0);
        assert_eq!(
            original_maneuver_target(&level, 0, TacticalOrder::LeftHook),
            Some(at(-10.0, 5.0, 10.0))
        );
    }

    // DAT_0066d094 turns a hook's leg by pi/8 and DAT_0066d07c scales it by
    // 0.75 (FUN_005a8f70).
    #[test]
    fn a_left_hook_turns_its_leg_an_eighth_of_pi() {
        use crate::tactical::{original_maneuver_waypoint, TacticalOrder};
        let at = |x: f32, y: f32, z: f32| TacticalWorldPosition { x, y, z };
        let waypoint = original_maneuver_waypoint(
            TacticalOrder::LeftHook,
            at(0.0, 0.0, 0.0),
            at(0.0, 0.0, 10.0),
        )
        .unwrap();
        assert!((waypoint.x + 2.870_126).abs() < 1e-4, "{waypoint:?}");
        assert!((waypoint.z - 6.929_096).abs() < 1e-4, "{waypoint:?}");
    }

    // port: an order's executor (event 0xa) reaches only the units still in
    // the battle on its turn.
    #[test]
    fn an_order_s_executor_skips_units_gone_from_the_battle() {
        use crate::tactical::{TacticalAttackTarget, TacticalOrder};
        let enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        let mut gone = fighter(true, TacticalWeaponArc::default());
        gone.alive = false;
        let mut empty = fighter(true, TacticalWeaponArc::default());
        empty.squad_count = 0;
        let flying = fighter(true, TacticalWeaponArc::default());
        let mut dead = ship(true, TacticalWeaponArc::default(), 0.0);
        dead.alive = false;
        let mut tactical = session(vec![enemy, dead], vec![gone, empty, flying]);
        tactical.post_order_executor(TacticalOrder::AttackCapitalShips, vec![1], vec![0, 1, 2]);
        tactical.run_posted_order_executors();
        assert_eq!(tactical.ships[1].attack_target, None);
        assert_eq!(tactical.fighters[0].attack_target, None);
        assert_eq!(tactical.fighters[1].attack_target, None);
        assert_eq!(
            tactical.fighters[2].attack_target,
            Some(TacticalAttackTarget::CapitalShip(0))
        );
    }

    // FUN_005d0820 (order 6's executor) starts the run for the first given
    // group still flying against a Death Star that still stands.
    #[test]
    fn the_death_star_attack_takes_the_first_flying_group_against_a_standing_station() {
        use crate::tactical::{begin_original_attack_order, TacticalOrder};
        let enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        let group = |number: u8| {
            let mut wing = fighter(true, TacticalWeaponArc::default());
            wing.fighter_group = number;
            wing
        };
        let mut gone = group(0);
        gone.alive = false;
        let mut empty = group(1);
        empty.squad_count = 0;
        let mut friendly = group(2);
        friendly.is_attacker = false;
        let mut tactical = session(vec![enemy], vec![gone, empty, friendly, group(3)]);
        let mut wrecked = trench_death_star(false);
        wrecked.hull = 0.0;
        tactical.death_star = Some(wrecked);
        begin_original_attack_order(
            &mut tactical,
            &[],
            &[0, 1, 2, 3],
            TacticalOrder::AttackDeathStar,
        );
        assert_eq!(tactical.trench_run_group, None);
        let mut destroyed = trench_death_star(false);
        destroyed.destroyed = true;
        tactical.death_star = Some(destroyed);
        begin_original_attack_order(
            &mut tactical,
            &[],
            &[0, 1, 2, 3],
            TacticalOrder::AttackDeathStar,
        );
        assert_eq!(tactical.trench_run_group, None);
        tactical.death_star = Some(trench_death_star(false));
        begin_original_attack_order(
            &mut tactical,
            &[],
            &[0, 1, 2, 3],
            TacticalOrder::AttackDeathStar,
        );
        assert_eq!(tactical.trench_run_group, Some(3));
    }

    // SetOrder (0x005ca530) sets the order at once and posts event 0xa at
    // the command stamp (+0x87c, three turns ahead); its handler
    // (0x005caba0) installs the executor (FUN_005ca6d0) only then.
    #[test]
    fn an_order_s_executor_waits_for_its_command_turn() {
        use crate::tactical::TacticalOrder;
        let mut own = ship(true, TacticalWeaponArc::default(), 0.0);
        own.source_position.z = -50.0;
        own.order = TacticalOrder::LeftHook;
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        enemy.source_position.z = 50.0;
        let mut tactical = session(vec![own, enemy], Vec::new());
        tactical.post_order_executor(TacticalOrder::LeftHook, vec![0], Vec::new());
        // Posted on turn 0 for stamp 3, which runs at turn 4's start.
        for _ in 0..15 {
            tactical.step();
        }
        assert_eq!(tactical.ships[0].source_waypoint, None);
        tactical.step();
        assert!(tactical.ships[0].source_waypoint.is_some());
        assert!(tactical.turn_events.is_empty());
    }

    // FUN_005b1e40 starts the jump only of a unit still in the fight
    // (FUN_005ad870): not of one destroyed or already jumping.
    #[test]
    fn a_jump_event_skips_a_unit_out_of_the_fight() {
        use crate::tactical::{TacticalTurnEvent, TacticalUnitRef};
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut dead = jumper(true, TacticalWeaponArc::default(), 0.0);
        dead.subsystem_condition.hyperdrive = 100;
        dead.alive = false;
        let mut wing = fighter(true, TacticalWeaponArc::default());
        wing.has_hyperdrive = true;
        wing.alive = false;
        let mut flying = jumper(true, TacticalWeaponArc::default(), 0.0);
        flying.subsystem_condition.hyperdrive = 100;
        flying.retreat_progress = 1_000.0;
        let mut flying_wing = fighter(true, TacticalWeaponArc::default());
        flying_wing.has_hyperdrive = true;
        flying_wing.retreat_progress = 1_000.0;
        let mut tactical = session(vec![enemy, dead, flying], vec![wing, flying_wing]);
        for unit in [
            TacticalUnitRef::Ship(1),
            TacticalUnitRef::Ship(2),
            TacticalUnitRef::Fighter(0),
            TacticalUnitRef::Fighter(1),
        ] {
            tactical.post_turn_event(3, TacticalTurnEvent::JumpStart(unit));
        }
        for _ in 0..16 {
            tactical.step();
        }
        assert_eq!(tactical.ships[1].retreat_progress, 0.0);
        assert_eq!(tactical.fighters[0].retreat_progress, 0.0);
        // Not withdrawing, the flying units are not advanced by the step;
        // the event does not restart their flight.
        assert_eq!(tactical.ships[2].retreat_progress, 1_000.0);
        assert_eq!(tactical.fighters[1].retreat_progress, 1_000.0);
        assert!(tactical.take_pending_audio_cues().is_empty());
    }

    // FUN_005c6610 decides a member's jump only after the side's time
    // (_DAT_0066cb18), with a working hyperdrive, and once: a member in
    // state 6 is not tried again.
    #[test]
    fn a_fighter_group_decides_its_jump_once_after_the_time() {
        use crate::tactical::{TacticalTurnEvent, TacticalUnitRef};
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut tactical = session(vec![enemy], Vec::new());
        let exit = tactical.withdrawal_exit_z(true);
        let group = |hyperdrive: bool| {
            let mut value = fighter(true, TacticalWeaponArc::default());
            value.has_hyperdrive = hyperdrive;
            value.retreating = true;
            value.sublight_engine_power = 10.0;
            value.source_position.z = exit;
            value
        };
        tactical.fighters = vec![group(true), group(false)];
        tactical.withdrawal_ms = Some(1_500.0);
        let queued = |t: &BattleSession, index: usize| {
            t.turn_events
                .iter()
                .filter(|queued| {
                    queued.event == TacticalTurnEvent::JumpStart(TacticalUnitRef::Fighter(index))
                })
                .count()
        };
        // Turn 1 (step 4) leaves 500 ms; turn 2 (step 8) ends the time; the
        // frame's order step (from turn 3, step 12) decides the jump once.
        let mut counts = Vec::new();
        for _ in 0..3 {
            for _ in 0..4 {
                tactical.step();
            }
            counts.push(queued(&tactical, 0));
        }
        assert_eq!(counts, [0, 0, 1]);
        assert_eq!(queued(&tactical, 1), 0);
    }

    // FUN_005a0240 warns (voice 0x95 / 0x10e) when a capital ship of the
    // withdrawing player's side cannot jump (FUN_005b1b70: a hyperdrive and
    // engine power).
    #[test]
    fn the_withdrawal_warns_of_a_ship_that_cannot_jump() {
        let mut ready = jumper(true, TacticalWeaponArc::default(), 0.0);
        ready.subsystem_condition.hyperdrive = 100;
        let mut tactical = session(vec![ready.clone()], Vec::new());
        assert!(!tactical.queue_withdrawal_hyperdrive_warning());
        let mut stalled = ready;
        stalled.sublight_engine_power = 0.0;
        tactical.ships.push(stalled);
        assert!(tactical.queue_withdrawal_hyperdrive_warning());
        assert_eq!(tactical.take_pending_voice_cues().len(), 1);
    }
}
