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
    /// A ship outside the group slots is a force of its own
    /// (`FUN_005aded0`): its index plus one here, else 0.
    pub own_force: u16,
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
pub(crate) fn family_strengths(vs_fighter: bool) -> [f32; 3] {
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
                AbstractUnit::Capital(index) => {
                    let id = self.session.ships[index].task_force;
                    AbstractGroup {
                        side,
                        fighters: false,
                        id,
                        own_force: if id == crate::tactical::TACTICAL_UNGROUPED_TASK_FORCE {
                            u16::try_from(index + 1).unwrap_or(u16::MAX)
                        } else {
                            0
                        },
                    }
                }
                AbstractUnit::Fighter(index) => AbstractGroup {
                    side,
                    fighters: true,
                    id: self.session.fighters[index].fighter_group,
                    own_force: 0,
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

    /// Policy slot `+0x0c` (`FUN_005ed5f0`): a task force on Recover (order
    /// 2), withdrawing (3) or on the Death Star run (6) is left alone (its
    /// live order `+0x64`, the port's first member's). Otherwise it keeps an
    /// eligible target while it and the side's other task forces on it score
    /// under 300 (`DAT_0066da84`) and not -1. port: a fighter force's members
    /// in states 1 or 3 are not mapped.
    fn keeps_target(
        &self,
        group: AbstractGroup,
        groups: &[(AbstractGroup, Vec<AbstractUnit>)],
    ) -> bool {
        let busy = groups
            .iter()
            .find(|(other, _)| *other == group)
            .and_then(|(_, members)| members.first())
            .is_some_and(|&lead| {
                let (order, retreating) = match lead {
                    AbstractUnit::Capital(index) => {
                        let ship = &self.session.ships[index];
                        (ship.order, ship.retreating)
                    }
                    AbstractUnit::Fighter(index) => {
                        let fighter = &self.session.fighters[index];
                        (fighter.order, fighter.retreating)
                    }
                };
                retreating
                    || matches!(
                        order,
                        crate::tactical::TacticalOrder::Recover
                            | crate::tactical::TacticalOrder::AttackDeathStar
                    )
            });
        if busy {
            return true;
        }
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
        let Some(destination) = withdrawal_destination(world, system, loser_is_alliance) else {
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

/// Where `alliance`'s fleets withdraw to from `system`: [`withdraw_loser`]'s
/// Autorouting search (`FUN_005545e0`), leaving out the battle's system.
/// The battle report names it (`+0x48`, set when `FUN_0049b720` builds the
/// report; RCDATA `0x700d` / `0x7011`).
#[must_use]
pub fn withdrawal_destination(
    world: &GameWorld,
    system: SystemKey,
    alliance: bool,
) -> Option<SystemKey> {
    let place = world.systems.get(system)?;
    let side = if alliance {
        crate::dat::Faction::Alliance
    } else {
        crate::dat::Faction::Empire
    };
    crate::missions::nearest_place(
        world,
        side,
        (i64::from(place.x), i64::from(place.y)),
        Some(system),
    )
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

/// `FUN_005d14d0` step 5, the side leaving. First its fighter groups (the
/// side's list `+0x64`), in order:
/// - one with a working hyperdrive (`FUN_005b1b70`) escapes
///   (`FUN_005af910`, state 9);
/// - one without docks on a carrier (`FUN_005afe40`): its home carrier
///   (`+0x164`) when that ship is still in the field (state 2,
///   `FUN_005afc80`) with room in its hangar (`FUN_005b84a0`: `+8` below
///   `+0x10`), else the first such ship of its side (the list `+0x58`);
/// - one that finds none is destroyed (`FUN_005af7a0`).
///
/// Then its capital ships (`+0x58`): one with a working hyperdrive escapes,
/// and the squadrons inside it go too; the others are destroyed, and
/// `FUN_005af7a0` destroys the squadrons inside them (the hangar's lists
/// `+0x434` and `+0x438`). A ship escapes after the groups dock, so a group
/// can board a carrier that is then lost.
///
/// port: the Empire's Death Star, `FUN_005afe40`'s last try, takes no
/// squadron (the port's Death Star has no hangar). hyp: a squadron already
/// inside a hangar (state 1) stays with its carrier; whether the list
/// `+0x64` holds it is untraced (in it, `FUN_005afe40` would refuse it, as
/// its state is not 2, and it would be destroyed). The tactical battle has
/// its own commanders (`FUN_005be160`) and withdraws through
/// `FUN_005c8780`.
pub fn leave(session: &mut BattleSession, alliance: bool) {
    let units = units_of(session, alliance);
    for &unit in &units {
        let AbstractUnit::Fighter(index) = unit else {
            continue;
        };
        if hyperdrive_works(session, unit) {
            session.fighters[index].alive = false;
            continue;
        }
        let in_field = |session: &BattleSession, carrier: usize| {
            unit_active(session, AbstractUnit::Capital(carrier))
                && unit_is_alliance(session, AbstractUnit::Capital(carrier)) == alliance
                && session.carrier_room(carrier) > 0
        };
        let home = session.fighters[index].home_carrier;
        let carrier = home
            .into_iter()
            .chain(0..session.ships.len())
            .find(|&carrier| in_field(session, carrier));
        match carrier {
            Some(carrier) => {
                let fighter = &mut session.fighters[index];
                fighter.alive = false;
                fighter.in_hangar = true;
                fighter.home_carrier = Some(carrier);
                session.ships[carrier].hangar.push(index);
            }
            None => {
                let fighter = &mut session.fighters[index];
                fighter.hull_current = 0.0;
                fighter.refresh_craft_count();
            }
        }
    }
    for unit in units {
        let AbstractUnit::Capital(index) = unit else {
            continue;
        };
        if hyperdrive_works(session, unit) {
            session.ships[index].retreated = true;
            continue;
        }
        let ship = &mut session.ships[index];
        ship.hull_current = 0;
        ship.shield = 0;
        ship.alive = false;
        for squadron in std::mem::take(&mut ship.hangar) {
            let fighter = &mut session.fighters[squadron];
            fighter.hull_current = 0.0;
            fighter.refresh_craft_count();
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

/// `FUN_005c9200` in the live battle, each review of `FUN_005c8780`: the
/// side's commander pairs its task forces that do not keep their target
/// (`FUN_005ed5f0`) with the enemy's units in priority order
/// (`FUN_005c8e10`, `FUN_005ec740`), exactly as the abstract battle does.
/// A task force's current target is its first member's (`FUN_005d2190`).
/// Returns every member's target after the review. port: the port keeps a
/// target per unit, so each member takes its force's.
#[must_use]
pub fn commander_targets(
    session: &BattleSession,
    alliance: bool,
) -> Vec<(AbstractUnit, crate::tactical::TacticalAttackTarget)> {
    use crate::tactical::TacticalAttackTarget;
    let mut battle = AbstractBattle::new(session.clone());
    let groups = battle.groups(alliance);
    for (group, members) in &groups {
        if let Some(target) = task_force_target(session, members) {
            battle.targets.insert(
                *group,
                match target {
                    TacticalAttackTarget::CapitalShip(index) => AbstractUnit::Capital(index),
                    TacticalAttackTarget::FighterGroup(index) => AbstractUnit::Fighter(index),
                },
            );
        }
    }
    battle.assign_targets(alliance);
    let mut targets = Vec::new();
    for (group, members) in &groups {
        let Some(&target) = battle.targets.get(group) else {
            continue;
        };
        let target = match target {
            AbstractUnit::Capital(index) => TacticalAttackTarget::CapitalShip(index),
            AbstractUnit::Fighter(index) => TacticalAttackTarget::FighterGroup(index),
        };
        targets.extend(members.iter().map(|&member| (member, target)));
    }
    targets
}

/// `0x5c8bbc`..`0x5c8c33` of `FUN_005c8780`: the Empire's commander with a
/// full superlaser fires at the first unit of its target list `+0x90`
/// (priority order, `FUN_005c8e10`) that is not a fighter and is still in
/// the fight (`FUN_005ad870`; the list holds only units in the fight).
#[must_use]
pub fn commander_superlaser_target(session: &BattleSession, alliance: bool) -> Option<usize> {
    let battle = AbstractBattle::new(session.clone());
    battle
        .ranked_enemies(alliance)
        .into_iter()
        .find_map(|unit| match unit {
            AbstractUnit::Capital(index) => Some(index),
            AbstractUnit::Fighter(_) => None,
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
            tractor_beam_range: 0.0,
            tractor_request_pending: false,
            ion_engine_stun: 0.0,
            hangar: Vec::new(),
            hangar_launch_stamp: 0.0,
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
            navigation_route: Vec::new(),
            home_carrier: None,
            in_hangar: false,
            retreating: false,
            retreat_progress: 0.0,
        }
    }

    /// The first stream whose next weapon draws (`FUN_005b3a40`'s 0 to 2)
    /// come up as `draws`.
    fn stream_with_weapon_draws(draws: &[u32]) -> OriginalTacticalRng {
        (0..)
            .map(OriginalTacticalRng::new)
            .find(|rng| {
                let mut rng = *rng;
                draws.iter().all(|&draw| {
                    rng.inclusive(0, crate::tactical::ORIGINAL_WEAPON_TURN_DRAW_MAX) == draw
                })
            })
            .expect("a stream gives the draws")
    }

    /// A stream whose next `count` weapon draws all come up 0, so the units
    /// fire on their first turns.
    fn stream_opening_weapon_turns(count: usize) -> OriginalTacticalRng {
        stream_with_weapon_draws(&vec![0; count])
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
            withdrawal_executors: Vec::new(),
            task_force_selection: None,
            task_force_formations: [[TacticalTactic::StandOff; 8]; 2],
            engagement_bands: Vec::new(),
            force_movers: Vec::new(),
            member_states: Vec::new(),
            force_detours: Vec::new(),
            player_observing: false,
            observer_review_countdown: 0,
            commanders_regrouped: [false; 2],
            recovery_executors: Vec::new(),
            alliance_raid_sent: false,
            fighter_orbit_angles: Vec::new(),
            fighter_movers: Vec::new(),
            fighter_frames: Vec::new(),
            trench_run_outcome: None,
            trench_run_remaining_ms: None,
            trench_run_group: None,
            trench_run_success_rating: 1,
            alliance_commander_term: 1.0,
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
            weapon_draws: 0,
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
            own_force: 0,
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
                carrier: None,
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
                carrier: None,
            }],
            characters: vec![],
            is_alliance: false,
            has_death_star: false,
            special_forces: Vec::new(),
        });
        world.systems[system].fleets = vec![alliance, ships, squadrons];
        (world, system, alliance, ships, squadrons)
    }

    /// `world_with_battle`'s Alliance carrier (id 5, room 4) with two
    /// squadrons aboard without a hyperdrive, one aboard with one, and one
    /// on no ship; the battle from it.
    fn carrier_battle() -> BattleSession {
        use crate::world::{FighterClass, FighterEntry};
        let (mut world, system, alliance, ships, squadrons) = world_with_battle();
        let jumper = world.fighter_classes.insert(FighterClass {
            dat_id: DatId::new(0x2000_0003),
            is_alliance: true,
            squadron_size: 12,
            hyperdrive: 1,
            sub_light_engine: 10,
            ..FighterClass::default()
        });
        let escort = world.fleets[alliance].fighters[0].class;
        let ship = world.fleets[alliance].capital_ships[0].class;
        world.capital_ship_classes[ship].fighter_capacity = 4;
        world.fleets[alliance].capital_ships[0].id = 5;
        world.fleets[alliance].fighters = vec![
            FighterEntry { class: escort, count: 2, carrier: Some(5) },
            FighterEntry { class: jumper, count: 1, carrier: Some(5) },
            FighterEntry { class: escort, count: 1, carrier: None },
        ];
        let FleetBattle { session, .. } = BattleSession::new_for_fleets(
            &world,
            system,
            &[alliance],
            &[ships, squadrons],
            true,
            3,
            7,
        );
        session
    }

    // 0x40bf0b / FUN_005964c0: a squadron aboard a carrier and without a
    // working hyperdrive (FUN_005b1b70) starts inside its hangar (state 1,
    // FUN_005b69d0; FUN_005b8570 sets +0x164 and the list); one with a
    // hyperdrive starts deployed, its carrier its home; one on no ship
    // deployed with no home. FUN_005b8570 sets the unset timer to the stamp
    // 3 plus a draw of 0 to 8 plus 5.0.
    #[test]
    fn carried_squadrons_without_a_hyperdrive_start_in_the_hangar() {
        let session = carrier_battle();
        let ours = session
            .fighters
            .iter()
            .enumerate()
            .filter(|(_, fighter)| fighter.identity.is_alliance)
            .map(|(index, fighter)| (index, fighter.in_hangar, fighter.alive, fighter.home_carrier))
            .collect::<Vec<_>>();
        assert_eq!(
            ours.iter().map(|&(_, hangar, alive, home)| (hangar, alive, home)).collect::<Vec<_>>(),
            [(true, false, Some(0)), (true, false, Some(0)), (false, true, Some(0)), (false, true, None)]
        );
        assert_eq!(session.ships[0].hangar, [ours[0].0, ours[1].0]);
        let stamp = session.ships[0].hangar_launch_stamp;
        assert!((8.0..=16.0).contains(&stamp) && stamp.fract() == 0.0, "{stamp}");
        assert_eq!(
            session.ships.iter().skip(1).map(|ship| ship.hangar_launch_stamp).collect::<Vec<_>>(),
            vec![0.0; session.ships.len() - 1]
        );
    }

    // FUN_005b8800 (a turn of the carrier's, after its repair): at its
    // timer it posts event 0x19 for the first squadron inside, and sets the
    // timer again (stamp + draw 0..8 + 5); FUN_005b8b10 / FUN_005b8c70
    // launch it three turns later at the carrier's underside, its home that
    // carrier, into the fight.
    #[test]
    fn a_hangar_launches_its_squadrons_one_at_a_time() {
        use crate::tactical::TacticalTurnEvent;
        let mut session = carrier_battle();
        session.paused = false;
        let first = session.ships[0].hangar[0];
        let second = session.ships[0].hangar[1];
        let timer = session.ships[0].hangar_launch_stamp;
        let posted = step_until(&mut session, |t| {
            t.turn_events.iter().any(|queued| {
                queued.event == TacticalTurnEvent::FighterLaunch { carrier: 0, fighter: first }
            })
        });
        // Posted in the updates of the turn whose stamp (+0x87c, the turn
        // plus 2) reaches the timer, on that turn's first step.
        assert_eq!(posted, (timer as u32 - 2) * 4);
        let next = session.ships[0].hangar_launch_stamp;
        assert!(next >= timer + 5.0 && next <= timer + 13.0, "{next}");
        let launched = step_until(&mut session, |t| t.fighters[first].alive);
        assert_eq!(launched, posted + 12);
        let fighter = &session.fighters[first];
        assert!(!fighter.in_hangar);
        assert_eq!(fighter.home_carrier, Some(0));
        assert_eq!(session.ships[0].hangar, [second]);
        assert!(!session.fighters[second].alive);
    }

    // FUN_005b8b10: no launch while the squadron's group is recovering
    // (order 2), nor with its carrier out of the fight.
    #[test]
    fn a_squadron_stays_inside_while_recovering_or_its_carrier_is_lost() {
        use crate::tactical::TacticalTurnEvent;
        let launch = |recover: bool, lost: bool| {
            let mut session = carrier_battle();
            let first = session.ships[0].hangar[0];
            if recover {
                session.fighters[first].order = TacticalOrder::Recover;
            }
            if lost {
                session.ships[0].alive = false;
            }
            session.run_turn_event(TacticalTurnEvent::FighterLaunch { carrier: 0, fighter: first });
            session.fighters[first].in_hangar
        };
        assert!(!launch(false, false));
        assert!(launch(true, false));
        assert!(launch(false, true));
    }

    // FUN_005b8800 with the timer unset: the stamp (+0x87c) plus a draw of 0
    // to 8 plus 5.0, and no launch yet.
    #[test]
    fn an_unset_hangar_timer_is_the_stamp_plus_a_draw_plus_five() {
        let mut session = carrier_battle();
        session.ships[0].hangar_launch_stamp = 0.0;
        session.tactical_rng = (0..)
            .map(OriginalTacticalRng::new)
            .find(|rng| {
                let mut rng = *rng;
                rng.inclusive(0, crate::tactical::ORIGINAL_HANGAR_LAUNCH_DRAW_MAX) == 3
            })
            .expect("a stream gives the draw");
        let stamp = session.step_event_stamp() as f32;
        session.hangar_turn(0);
        assert_eq!(session.ships[0].hangar_launch_stamp, stamp + 3.0 + 5.0);
        assert!(session.turn_events.is_empty());
    }

    // 0x5c7060: a member in state 0 flies to its home carrier only while that
    // carrier lives on its side; else to its place.
    #[test]
    fn a_waiting_member_flies_to_a_live_home_carrier_of_its_side() {
        use crate::tactical::TacticalWorldPosition;
        let flown = |alive: bool, alliance: bool| {
            let mut tactical = recovery(0, 1);
            let mut home = ship(alliance, TacticalWeaponArc::default(), 0.0);
            home.source_position.x = 30.0;
            home.alive = alive;
            tactical.ships.push(home);
            tactical.fighters[0].home_carrier = Some(2);
            tactical.sync_fighter_movers();
            for mover in &mut tactical.fighter_movers {
                mover.anchor = TacticalWorldPosition { x: -30.0, y: 0.0, z: 0.0 };
            }
            tactical.advance_recovering_fighters(250.0);
            tactical.fighters[0].source_position.x
        };
        assert!(flown(true, true) > 0.0);
        assert!(flown(false, true) < 0.0);
        assert!(flown(true, false) < 0.0);
    }

    // FUN_005cf980 tries a member's home carrier (+0x164) before its side's
    // ships, and the hangar's squadrons fill its room (+0xc counts them).
    #[test]
    fn a_recovery_tries_the_home_carrier_first() {
        use crate::tactical::TacticalFighterRecoveryState as Recovery;
        let mut tactical = recovery(1, 1);
        let mut home = ship(true, TacticalWeaponArc::default(), 0.0);
        home.fighter_capacity = 1;
        tactical.ships.push(home);
        tactical.fighters[0].home_carrier = Some(2);
        tactical.advance_recovery_executors();
        assert_eq!(tactical.fighters[0].recovery_state, Recovery::Reserved);
        assert_eq!(tactical.fighters[0].recovery_target, Some(2));

        let mut full = recovery(1, 1);
        full.ships[0].hangar.push(5);
        full.advance_recovery_executors();
        assert_eq!(full.fighters[0].recovery_state, Recovery::AwaitingCarrier);
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

    /// Installs the order-3 executor (event 0xa, FUN_005ca6d0) of each
    /// unit already given the order, as in a withdrawal under way.
    fn install_withdrawal_executors(tactical: &mut BattleSession) {
        use crate::tactical::TacticalUnitRef;
        for (index, ship) in tactical.ships.iter().enumerate() {
            if ship.retreating {
                tactical
                    .withdrawal_executors
                    .push(TacticalUnitRef::Ship(index));
            }
        }
        for (index, fighter) in tactical.fighters.iter().enumerate() {
            if fighter.retreating {
                tactical
                    .withdrawal_executors
                    .push(TacticalUnitRef::Fighter(index));
            }
        }
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
        // FUN_005b2720 -> FUN_005b2d20). The review's SetOrder(3)
        // (0x005ad6b8) posts the executor (event 0xa) with the step's stamp
        // 3; it is installed at turn 4, step 16 (FUN_005ca6d0). The time
        // has ended at step 8; at its edge from then, the frame's order
        // step (FUN_005cc300) decides the jump at step 17, stamped 7
        // (0x005ad4a0); it runs at turn 8, step 32 (FUN_005a96b0); the
        // flight ends on step 47, and its event 0x2e, stamped 14, runs at
        // turn 15, step 60.
        tactical.ai_withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        assert_eq!(
            step_until(&mut tactical, |t| t
                .withdrawal_executors
                .contains(&crate::tactical::TacticalUnitRef::Ship(0))),
            16
        );
        tactical.ships[0].source_position.z = tactical.withdrawal_exit_z(false);
        assert_eq!(step_until(&mut tactical, |t| t.ships[0].jumping()), 32);
        assert_eq!(step_until(&mut tactical, |t| !t.ships[0].alive), 60);
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
        // Its hyperdrive takes every hit it can (FUN_005b1970, case 4), so a
        // hit elsewhere that refreshes every ship's condition keeps it at 0.
        let mut damaged = jumper(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        damaged.subsystem_damage.hyperdrive = damaged.subsystem_capacity.hyperdrive;
        damaged.refresh_subsystem_condition();
        assert_eq!(damaged.subsystem_condition.hyperdrive, 0);
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
        install_withdrawal_executors(&mut tactical);
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
            carrier: None,
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

    // FUN_005afe40: a fighter group without a hyperdrive docks on a ship
    // of its side with room; FUN_005af7a0 destroys a ship that cannot jump
    // with the squadrons inside it.
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

    // FUN_005afe40: a group tries its home carrier (+0x164) first, then its
    // side's ships in order (+0x58); FUN_005b84a0 docks it only while the
    // hangar's +8 is below its +0x10. A group that finds no room is
    // destroyed (FUN_005af7a0).
    #[test]
    fn a_leaving_group_docks_on_its_home_carrier_first_and_needs_room() {
        let carrier = |capacity: u32| {
            let mut own = ship(true, TacticalWeaponArc::default(), 0.0);
            own.fighter_capacity = capacity;
            own.subsystem_capacity.hyperdrive = 1;
            own
        };
        let mut groups = vec![
            fighter(true, TacticalWeaponArc::default()),
            fighter(true, TacticalWeaponArc::default()),
            fighter(true, TacticalWeaponArc::default()),
        ];
        groups[0].home_carrier = Some(1);
        let mut battle = AbstractBattle::new(session(
            vec![
                carrier(1),
                carrier(1),
                ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0),
            ],
            groups,
        ));
        battle.retreat_allowed[0] = true;
        battle.retreat(true);
        let session = &battle.session;
        assert_eq!(session.ships[1].hangar, [0]);
        assert_eq!(session.ships[0].hangar, [1]);
        assert!(session.fighters[0].in_hangar && session.fighters[0].squad_count > 0);
        assert!(session.fighters[1].in_hangar && session.fighters[1].squad_count > 0);
        // No room left for the third.
        assert_eq!(session.fighters[2].squad_count, 0);
        assert!(session.ships[0].retreated && session.ships[1].retreated);
    }

    // FUN_005af7a0 walks a destroyed ship's hangar lists (+0x434, +0x438)
    // and destroys the squadrons inside, those there before the side left
    // as well as those that docked as it left.
    #[test]
    fn a_ship_that_cannot_jump_takes_the_squadrons_inside_with_it() {
        let mut own = ship(true, TacticalWeaponArc::default(), 0.0);
        own.fighter_capacity = 2;
        own.subsystem_capacity.hyperdrive = 0;
        own.hangar = vec![1];
        let mut inside = fighter(true, TacticalWeaponArc::default());
        inside.alive = false;
        inside.in_hangar = true;
        inside.home_carrier = Some(0);
        let mut battle = AbstractBattle::new(session(
            vec![own, ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0)],
            vec![fighter(true, TacticalWeaponArc::default()), inside],
        ));
        battle.retreat_allowed[0] = true;
        battle.retreat(true);
        let session = &battle.session;
        assert!(!session.ships[0].alive);
        assert!(session.ships[0].hangar.is_empty());
        assert_eq!(session.fighters[0].squad_count, 0);
        assert_eq!(session.fighters[1].squad_count, 0);
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

    // FUN_005b1bc0 counts the installed hyperdrives (+0x3ec); FUN_0040be90
    // gives a fighter group its FIGHTSD primary and backup ratings, one
    // each when not 0 (slots +0x1ec and +0x1f0, class +0x6c and +0x70).
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
        install_withdrawal_executors(&mut tactical);
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
        // 3 plus the commander term 1: 4.
        wing.sublight_engine_power = 3.0;
        let mut tactical = session(vec![enemy], vec![wing]);
        let exit = tactical.withdrawal_exit_z(true);
        tactical.fighters[0].source_position.z = exit + 9.0;
        install_withdrawal_executors(&mut tactical);
        face_exits(&mut tactical);
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
        // beyond its 1.0 radius), so FUN_005c6610 tries no jump yet. (The
        // computer commander's target links, 0x36, may be queued.)
        assert!(events_besides_target_links(&tactical) == 0);
        for _ in 0..7 {
            tactical.step();
        }
        assert_eq!(tactical.fighters[0].source_position.z, exit + 1.0);
        assert!(tactical.turn_events.is_empty());
        // At step 20, a turn's first, the move finds its anchor arrived
        // (|d| = 1.0): the jump is decided (state 6, event 0x2d stamped 7,
        // 0x005ad4a0). The squadron's own frame flies on along z to the
        // line itself (force slot +0x18 on order 3) and holds there until
        // the event runs at turn 8, step 32.
        tactical.step();
        assert_eq!(tactical.combat_tick, 20);
        assert_eq!(tactical.fighters[0].source_position.z, exit);
        assert_eq!(tactical.turn_events.len(), 1);
        assert_eq!(tactical.turn_events[0].stamp, 7);
        assert!(!tactical.fighters[0].jumping());
        assert_eq!(step_until(&mut tactical, |t| t.fighters[0].jumping()), 32);
        // Then the flight moves it on in that step: k = 25 (DAT_006bc47c)
        // for 250 ms (FUN_005a6b40).
        // It jumps from the line, where it held.
        assert_eq!(
            tactical.fighters[0].source_position.z,
            exit - 25.0 * 0.25 * 0.25
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
            install_withdrawal_executors(&mut tactical);
            tactical.withdrawal_ms = Some(100_000.0);
            // One move a frame step from step 12 (0x005a9ff8).
            for _ in 0..11 {
                tactical.step();
            }
            let mut at = Vec::new();
            for _ in 0..3 {
                tactical.step();
                // The step moves the group's anchor.
                at.push(tactical.fighter_mover_of(0).expect("a mover").anchor.z - exit);
            }
            at
        };
        // 10 a second, 2.5 a step: past the line by 0.5, within the radius,
        // so it stays.
        // A rating of 9 plus the commander term 1 (0x5ba190): 10 a second.
        assert_eq!(run(9.0), [-0.5, -0.5, -0.5]);
        // 2.5 a second: 0.625 a step until within the radius.
        assert_eq!(run(1.5), [1.375, 0.75, 0.75]);
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
    // gives it order 3 (SetOrder, 0x005ad6b8); FUN_005cc300, installed on
    // the order's event 0xa (FUN_005ca6d0), heads it for its exit line each
    // step.
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
        // The order, given on turn 0, is stamped 3; its executor is
        // installed at turn 4, step 16, after the frame's moves have begun
        // (turn 3, step 12, 0x005bef1d).
        for _ in 0..15 {
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
            install_withdrawal_executors(&mut tactical);
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
        install_withdrawal_executors(&mut tactical);
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

    // FUN_005cc300 tries its force's first member (0x005c6070), but
    // FUN_005ad530 gives each capital ship a force of its own
    // (FUN_005aded0 -> 0x005cbf50): ships of one group decide their jumps
    // together, and one that cannot jump holds no other.
    #[test]
    fn each_withdrawing_ship_decides_its_own_jump() {
        let run = |first_hyperdrive: u8| {
            let mut first = jumper(true, TacticalWeaponArc::default(), 0.0);
            first.subsystem_condition.hyperdrive = first_hyperdrive;
            let mut second = jumper(true, TacticalWeaponArc::default(), 0.0);
            second.subsystem_condition.hyperdrive = 100;
            let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
            enemy.hull_current = 1_000_000;
            enemy.hull_max = 1_000_000;
            let mut tactical = session(vec![first, second, enemy], Vec::new());
            assert_eq!(tactical.ships[0].task_force, tactical.ships[1].task_force);
            let exit = tactical.withdrawal_exit_z(true);
            for index in [0, 1] {
                tactical.ships[index].retreating = true;
                tactical.ships[index].source_position.z = exit;
            }
            install_withdrawal_executors(&mut tactical);
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
        assert!(decided(&tactical, 0) && decided(&tactical, 1));
        // Both jump at turn 6 (step 24).
        assert_eq!(
            step_until(&mut tactical, |t| t.ships[0].jumping()
                && t.ships[1].jumping()),
            24
        );

        let held = run(0);
        assert!(!decided(&held, 0));
        assert!(decided(&held, 1));
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
        install_withdrawal_executors(&mut tactical);
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
        assert_eq!(tactical.fighters[2].order, TacticalOrder::None);
        assert!(tactical.fighters[0].retreating && !tactical.fighters[2].retreating);
        // The orders' executors (event 0xa, stamped 3 on turn 0) come at
        // turn 4, step 16 (FUN_005ca6d0): the recovery reserves its carrier
        // then (FUN_005cf980).
        assert_eq!(tactical.fighters[1].recovery_target, None);
        assert_eq!(
            step_until(&mut tactical, |t| t.fighters[1].recovery_target.is_some()),
            16
        );
        assert_eq!(tactical.fighters[1].recovery_target, Some(0));
        // Reserved (state 1, FUN_005b84e0) on the turn's update, it flies
        // its frame to the carrier's underside already (0x5c7060 with
        // +0x164, 0x5b8f70). Its event 0x2c (FUN_005b1e10) runs at turn 7
        // (step 28) and gives it state 2; that turn's executor finds it
        // within 2.0 and docks it (state 3, event 0x1a), whose handler
        // takes it in at turn 10 (step 40, FUN_005b8bd0).
        assert_eq!(tactical.fighters[1].recovery_state, Recovery::Reserved);
        let mut docking = tactical.clone();
        assert_eq!(
            step_until(&mut docking, |t| t.fighters[1].recovery_state != Recovery::Reserved),
            28
        );
        assert_eq!(docking.fighters[1].recovery_state, Recovery::Docking);
        assert_eq!(
            step_until(&mut docking, |t| t.fighters[1].recovery_state == Recovery::Recovered),
            40
        );
        tactical.fighters[0].source_position.z = tactical.withdrawal_exit_z(false);
        // Its group's anchor goes with it (FUN_005c72b0's +0x3c).
        let exit_z = tactical.withdrawal_exit_z(false);
        for mover in &mut tactical.fighter_movers {
            if mover.slots.iter().any(|(member, _)| *member == 0) {
                mover.anchor.z = exit_z;
            }
        }
        tactical.ai_withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        // From its executor, it jumps at turn 8, step 32, and leaves at
        // turn 15, step 60 (events 0x2d and 0x2e, FUN_005a96b0).
        assert_eq!(step_until(&mut tactical, |t| t.fighters[0].jumping()), 32);
        assert_eq!(step_until(&mut tactical, |t| !t.fighters[0].alive), 60);
        assert!(!tactical.fighters[0].alive && tactical.fighters[0].squad_count > 0);
        assert!(tactical.fighters[2].alive);
    }

    // FUN_005cf980, on the executor's turn, reserves a living carrier of the
    // group's side with room (FUN_005b84e0: +0xc below +0x10); a squadron
    // already on its way holds its place, a stale target in state 0 does
    // not, and one place serves one squadron. A member finding every ship
    // full sets the group's order to 0 (SetOrder, 0x005ca530) once, for
    // the whole group. A player's group with no colour (4) queues no voice.
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
        first.order = TacticalOrder::Recover;
        let mut second = fighter(true, TacticalWeaponArc::default());
        second.fighter_group = 4;
        second.order = TacticalOrder::Recover;
        let mut tactical = session(vec![dead, carrier], vec![returning, stale, first, second]);
        tactical.begin_original_fighter_recovery(&[2, 3]);
        assert_eq!(tactical.fighters[2].recovery_state, Recovery::AwaitingCarrier);
        tactical.advance_recovery_executors();
        assert_eq!(tactical.fighters[2].recovery_target, Some(1));
        assert_eq!(tactical.fighters[2].recovery_state, Recovery::Reserved);
        assert_eq!(tactical.fighters[3].recovery_target, None);
        assert_eq!(
            tactical.fighters[3].recovery_state,
            Recovery::AwaitingCarrier
        );
        assert!(tactical.take_pending_voice_cues().is_empty());
        assert_eq!(tactical.fighters[2].order, TacticalOrder::None);
        assert_eq!(tactical.fighters[3].order, TacticalOrder::None);
        assert!(tactical.turn_events.iter().any(|queued| matches!(
            &queued.event,
            crate::tactical::TacticalTurnEvent::OrderExecutor { order: TacticalOrder::None, fighters, .. }
                if fighters == &vec![2, 3]
        )));
    }

    // 0x5c7060: a recovering member's distance is the raw one to its
    // carrier's +0x418 sub-object, whose vt +4 (0x5b0b80) is the hull's
    // underside (y less the vertical radius +0x8); 0x5b8f70 then turns and
    // flies it at its speed and alignment.
    #[test]
    fn a_returning_squadron_flies_its_frame_to_the_carrier_underside() {
        use crate::tactical::{
            original_capital_turn_rate, turn_original_fighter, OriginalTacticalCollisionEnvelope,
            TacticalFighterRecoveryState as Recovery, TacticalWorldPosition,
        };
        let mut carrier = ship(true, TacticalWeaponArc::default(), 0.0);
        carrier.source_position = TacticalWorldPosition {
            x: 4.0,
            y: 2.0,
            z: 10.0,
        };
        carrier.source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 3.0,
            planar_diameter: 0.0,
        });
        let mut wing = fighter(true, TacticalWeaponArc::default());
        wing.order = TacticalOrder::Recover;
        wing.recovery_state = Recovery::Returning;
        wing.recovery_target = Some(0);
        let mut tactical = session(vec![carrier], vec![wing]);
        let speed = tactical.original_fighter_speed(0);
        tactical.advance_recovering_fighters(250.0);
        // Toward (4, -1, 10), from the side's forward +z.
        let heading = vector(4.0, -1.0, 10.0).normalized();
        let rate = original_capital_turn_rate(tactical.fighters[0].maneuverability);
        let forward = turn_original_fighter(vector(0.0, 0.0, 1.0), heading, rate, 250.0);
        let travel = speed * forward.dot(heading).max(0.25) * 0.25;
        let at = tactical.fighters[0].source_position;
        assert!((at.x - forward.x * travel).abs() < 1e-5, "{at:?}");
        assert!((at.y - forward.y * travel).abs() < 1e-5, "{at:?}");
        assert!((at.z - forward.z * travel).abs() < 1e-5, "{at:?}");
        assert!(at.y < 0.0, "{at:?}");
        // Its force's anchor stands.
        assert_eq!(
            tactical.fighter_mover_of(0).expect("a mover").anchor,
            TacticalWorldPosition::ORIGIN
        );
    }

    // FUN_005cf980's second loop, on the executor's turn: only in state 2,
    // a member within 2.0 (DAT_0066d088) of its carrier's underside enters
    // state 3.
    #[test]
    fn a_squadron_docks_within_two_of_the_carrier_underside() {
        use crate::tactical::{
            OriginalTacticalCollisionEnvelope, TacticalFighterRecoveryState as Recovery,
            TacticalWorldPosition,
        };
        let dock = |state: Recovery, x: f32, y: f32| {
            let mut carrier = ship(true, TacticalWeaponArc::default(), 0.0);
            carrier.source_position.z = 10.0;
            carrier.source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
                vertical_radius: 3.0,
                planar_diameter: 0.0,
            });
            let mut wing = fighter(true, TacticalWeaponArc::default());
            wing.order = TacticalOrder::Recover;
            wing.recovery_state = state;
            wing.recovery_target = Some(0);
            wing.source_position = TacticalWorldPosition { x, y, z: 10.0 };
            let mut tactical = session(vec![carrier], vec![wing]);
            tactical.begin_original_fighter_recovery(&[0]);
            tactical.fighters[0].recovery_state = state;
            tactical.fighters[0].recovery_target = Some(0);
            tactical.advance_recovery_executors();
            tactical.fighters[0].recovery_state
        };
        // 1.5 below the underside (-3), 4.5 below the centre.
        assert_eq!(dock(Recovery::Returning, 0.0, -4.5), Recovery::Docking);
        // At the centre, 3 above the underside.
        assert_eq!(dock(Recovery::Returning, 0.0, 0.0), Recovery::Returning);
        // Off to the side: 1.9 and 2.1 from it.
        assert_eq!(dock(Recovery::Returning, 1.9, -3.0), Recovery::Docking);
        assert_eq!(dock(Recovery::Returning, 2.1, -3.0), Recovery::Returning);
        // Exactly 2.0 is not within it.
        assert_eq!(dock(Recovery::Returning, 2.0, -3.0), Recovery::Returning);
        // Not yet in state 2.
        assert_eq!(dock(Recovery::Reserved, 0.0, -3.0), Recovery::Reserved);
    }

    // The squadrons a recovery flies (0x5c7060): on their way with a live
    // carrier of their own side, and not withdrawing. Within 2.0 a squadron
    // still flies; its docking comes on the executor's turn (FUN_005cf980,
    // run from the force's turn slot +0x10, not from the frame). A carrier
    // of the other side is no carrier: the squadron does not fly, and its
    // state stays.
    #[test]
    fn a_recovery_flies_only_squadrons_still_on_their_way() {
        use crate::tactical::{TacticalFighterRecoveryState as Recovery, TacticalWorldPosition};
        let flies = |carrier_side: bool, start_z: f32, retreating: bool| {
            let mut carrier = ship(carrier_side, TacticalWeaponArc::default(), 0.0);
            carrier.source_position.z = 10.0;
            let mut wing = fighter(true, TacticalWeaponArc::default());
            wing.order = TacticalOrder::Recover;
            wing.recovery_state = Recovery::Returning;
            wing.recovery_target = Some(0);
            wing.retreating = retreating;
            let start = TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: start_z,
            };
            wing.source_position = start;
            let mut tactical = session(vec![carrier], vec![wing]);
            tactical.begin_original_fighter_recovery(&[0]);
            tactical.fighters[0].recovery_state = Recovery::Returning;
            tactical.fighters[0].recovery_target = Some(0);
            tactical.advance_recovering_fighters(250.0);
            let moved = tactical.fighters[0].source_position != start;
            (moved, tactical.fighters[0].recovery_state)
        };
        assert_eq!(flies(true, 0.0, false), (true, Recovery::Returning));
        assert_eq!(flies(false, 0.0, false), (false, Recovery::Returning));
        assert!(flies(true, 8.0, false).0);
        assert!(flies(true, 8.5, false).0);
        assert_eq!(flies(true, 0.0, true).0, false);
    }

    // 0x5cb050 for a recovering member with no carrier: its place round the
    // force's anchor, which stands (order 2's install clears its target).
    #[test]
    fn a_squadron_with_no_carrier_flies_to_its_place() {
        use crate::tactical::{TacticalAttackTarget, TacticalWorldPosition};
        let mut wing = fighter(true, TacticalWeaponArc::default());
        wing.order = TacticalOrder::Recover;
        wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        let mut far = ship(false, TacticalWeaponArc::default(), 0.0);
        far.source_position.z = 50.0;
        let mut tactical = session(vec![far], vec![wing]);
        tactical.sync_fighter_movers();
        tactical.fighter_movers[0].anchor = TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: -20.0,
        };
        tactical.advance_recovering_fighters(250.0);
        let at = tactical.fighters[0].source_position;
        // It faces +z and its place is behind: it still flies a quarter.
        assert!(at.z > 0.0, "{at:?}");
        assert_eq!(tactical.fighter_movers[0].anchor.z, -20.0);
        // A squadron not recovering is left to its force's step.
        let mut free = session(
            vec![ship(false, TacticalWeaponArc::default(), 0.0)],
            vec![fighter(true, TacticalWeaponArc::default())],
        );
        free.advance_recovering_fighters(250.0);
        assert_eq!(free.fighters[0].source_position, TacticalWorldPosition::ORIGIN);
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
            group(true, 60.0, 3.0),
            // Just outside 8.0 after its move: no jump.
            group(true, alliance_exit + 8.5, 0.4),
            // Far inside the field: no jump.
            group(true, alliance_exit + 20.0, 0.4),
            // The Empire's group well short of its +z edge: no jump.
            group(false, 20.0, 0.4),
        ];
        // Each in a group of its own: a group is one force on one anchor
        // (FUN_005adfa0).
        for (group, fighter) in tactical.fighters.iter_mut().enumerate() {
            fighter.fighter_group = u8::try_from(group).unwrap_or(0);
        }
        install_withdrawal_executors(&mut tactical);
        tactical.withdrawal_ms = Some(0.0);
        tactical.ai_withdrawal_ms = Some(0.0);
        face_exits(&mut tactical);
        // The task forces' order steps run in the frame, from turn 3, step
        // 12 (0x005a9ff8, 0x005bef1d), with its 250 ms.
        for _ in 0..11 {
            tactical.step();
        }
        assert_eq!(tactical.fighters[0].source_position.z, 60.0);
        tactical.step();
        let fighters = &tactical.fighters;
        assert_eq!(fighters[0].source_position.z, 59.0);
        // 0.4 plus the commander term 1 (0x5ba190): 1.4 a second, 0.35 a
        // step.
        assert!((fighters[1].source_position.z - (alliance_exit + 8.15)).abs() < 1e-4);
        assert!((fighters[3].source_position.z - 20.35).abs() < 1e-4);
        assert!(empire_exit > 20.35);
        // No jump is decided (event 0x2d).
        assert_eq!(events_besides_target_links(&tactical), 0);
    }

    /// Turns every squadron to face its side's exit, so that its flight
    /// (0x5b8f70) needs no turn and moves at its full speed.
    fn face_exits(tactical: &mut BattleSession) {
        tactical.fighter_frames = tactical
            .fighters
            .iter()
            .map(|fighter| crate::tactical::TacticalFighterFrame {
                forward: TacticalWorldVector {
                    x: 0.0,
                    y: 0.0,
                    z: if fighter.identity.is_alliance { -1.0 } else { 1.0 },
                },
                velocity: TacticalWorldVector::ZERO,
            })
            .collect();
    }

    /// The queued events other than the computer commander's target links
    /// (event 0x36, FUN_005a8fc0).
    fn events_besides_target_links(tactical: &BattleSession) -> usize {
        tactical
            .turn_events
            .iter()
            .filter(|queued| !matches!(queued.event, crate::tactical::TacticalTurnEvent::TaskForceTarget { .. }))
            .count()
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
            // The enemy's draw opens its turn 1 (FUN_005b3a40).
            tactical.tactical_rng = stream_opening_weapon_turns(1);
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
        // The shooter's, the live ship's and the wing's draws open turn 1
        // (FUN_005b3a40); the dead and jumping ships do not draw.
        tactical.tactical_rng = stream_opening_weapon_turns(3);
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
            tactical_attack_target_is_eligible, TacticalAttackTarget,
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
        // Each group's nearest enemy is the other (FUN_005cb6e0).
        let fires = |tactical: &BattleSession, index: usize| {
            let mut shots = Vec::new();
            tactical
                .clone()
                .fire_unit_weapons(crate::tactical::TacticalUnitRef::Fighter(index), &mut shots);
            !shots.is_empty()
        };
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
                fires(tactical, 1),
                fires(tactical, 0),
            )
        };
        assert_eq!(check(&tactical), (true, true, Some(target), true, true));
        let mut jumping = tactical.clone();
        jumping.fighters[1].retreating = true;
        jumping.fighters[1].retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
        assert_eq!(check(&jumping), (false, false, None, false, false));
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
        // An Alliance ship keeps its side in the battle until the abort's
        // turn: the run's draws have taken the runner's craft.
        let mut escort = ship(true, TacticalWeaponArc::default(), 0.0);
        escort.hull_current = 1_000_000;
        escort.hull_max = 1_000_000;
        let mut tactical = session(vec![ds_ship, escort], vec![runner]);
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
        let abort = crate::tactical::TacticalTurnEvent::TrenchRunState(8);
        flying.withdraw_fighters(false);
        assert!(!flying.turn_event_pending(&abort));
        // With no list, a live runner of the side still marks its run.
        flying.withdraw_fighters(true);
        assert!(flying.turn_event_pending(&abort));
        tactical.begin_player_withdrawal();
        tactical.withdraw_fighters(true);
        // The state 8, posted on turn 0 with the command stamp 3
        // (0x005ad5e7), ends the run at turn 4, step 16.
        assert!(tactical.trench_run_remaining_ms.is_some());
        assert_eq!(
            step_until(&mut tactical, |t| t.trench_run_remaining_ms.is_none()),
            16
        );
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
        install_withdrawal_executors(&mut tactical);
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
            install_withdrawal_executors(&mut tactical);
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
        install_withdrawal_executors(&mut tactical);
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

    // FUN_005c6610's approach steps the force's anchor at its slowest
    // member's speed toward its point, (-side.z, 0, side.x) * 0.5 extent:
    // -x for the Alliance (side vector (0, 0, 1)), +x for the Empire; the
    // mover faces that way. The members on order 6 (alive, with craft, of
    // the run's group) fly their frames to their places.
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
            // An earlier group, whose mover comes first.
            other_group.fighter_group = 0;
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
            // Three from the point at the slowest speed (the commander term
            // 1 plus 1): 250 ms along (-2, -2, -1) / 3, in 60 Hz frames
            // (port).
            assert!((crate::tactical::PORT_MOVER_FRAME_MILLISECONDS * 60.0 - 1000.0).abs() < 1e-3);
            let speed = tactical.original_fighter_speed(0);
            assert!(speed < tactical.original_fighter_speed(1));
            let travel = 0.25 * speed / 3.0;
            let mover = tactical.fighter_mover_of(0).expect("a mover");
            let anchor = mover.anchor;
            let expected = crate::tactical::TacticalWorldPosition {
                x: point_x + 2.0 - 2.0 * travel,
                y: 2.0 - 2.0 * travel,
                z: 1.0 - travel,
            };
            assert!(
                (anchor.x - expected.x).abs() < 1e-4
                    && (anchor.y - expected.y).abs() < 1e-4
                    && (anchor.z - expected.z).abs() < 1e-4,
                "{alliance} {anchor:?}"
            );
            let side_x = if alliance { -1.0 } else { 1.0 };
            assert_eq!(mover.forward, vector(side_x, 0.0, 0.0));
            for index in [0, 1] {
                assert_ne!(tactical.fighters[index].source_position, start, "{alliance}");
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

        // A withdrawal during the approach posts state 8 (0x005ad5e7) with
        // the command stamp 3, after the run's state 2 of the same stamp;
        // at turn 4, step 16, it ends the run.
        tactical.withdraw_fighters(tactical.player_is_attacker);
        assert!(tactical.trench_run_under_way());
        assert!(tactical.turn_event_pending(&TacticalTurnEvent::TrenchRunState(8)));
        assert_eq!(step_until(&mut tactical, |t| !t.trench_run_under_way()), 16);
        assert!(tactical
            .turn_events
            .iter()
            .all(|queued| !matches!(queued.event, TacticalTurnEvent::TrenchRunState(_))));
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
        // step then moves the anchor at the squadron's speed, the commander
        // term 1 plus 1 (0x5ba190): 2.0 a second.
        let anchor_x = |tactical: &BattleSession| {
            tactical.fighter_mover_of(0).expect("a mover").anchor.x
        };
        for _ in 0..15 {
            tactical.step();
        }
        assert_eq!(anchor_x(&tactical), -half + 2.0);
        assert_eq!(tactical.fighters[0].source_position.x, -half + 2.0);
        tactical.step();
        assert!((anchor_x(&tactical) - (-half + 1.5)).abs() < 1e-3);
        // Within 0.2 after 1.8 units, 54 frames: during step 19, stamped 7
        // (turn 4 plus the lead), so state 5 runs at turn 8, step 32.
        for _ in 0..2 {
            tactical.step();
        }
        assert!(!tactical.turn_event_pending(&TacticalTurnEvent::TrenchRunState(5)));
        tactical.step();
        assert_eq!(tactical.combat_tick, 19);
        assert!(tactical.turn_event_pending(&TacticalTurnEvent::TrenchRunState(5)));
        assert_eq!(tactical.turn_events[0].stamp, 7);
        assert_eq!(tactical.trench_run_remaining_ms, None);
        // It is posted once.
        tactical.step();
        assert_eq!(
            tactical
                .turn_events
                .iter()
                .filter(|event| event.event == TacticalTurnEvent::TrenchRunState(5))
                .count(),
            1
        );
        tactical.take_pending_voice_cues();
        for _ in 0..11 {
            tactical.step();
        }
        assert_eq!(tactical.combat_tick, 31);
        assert!(tactical.trench_run_remaining_ms.is_none());
        tactical.step();
        assert_eq!(tactical.combat_tick, 32);
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

    /// A capital ship 10 ahead of an unarmed target, and a fighter group
    /// beside it with a target of its own, all in range, after turn 1's
    /// updates (step 4) from `rng`: the shots decided, and the stream.
    fn weapon_turn(
        order: TacticalOrder,
        ready: bool,
        rng: OriginalTacticalRng,
    ) -> (Vec<crate::tactical::TacticalUnitRef>, OriginalTacticalRng, u32) {
        use crate::tactical::{TacticalAttackTarget, TacticalTurnEvent};
        let ranges = TacticalWeaponRanges {
            laser_cannon: 100.0,
            ion_cannon: 100.0,
            turbolaser: 100.0,
        };
        let mut shooter = ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0);
        shooter.weapon_arcs = [TacticalWeaponArc::new(0, 0, 4); 4];
        shooter.weapon_ranges = ranges;
        shooter.order = order;
        shooter.attack_target = (order == TacticalOrder::AttackCapitalShips)
            .then_some(TacticalAttackTarget::CapitalShip(1));
        shooter.escort_target = (order == TacticalOrder::Escort).then_some(2);
        if !ready {
            for arc in &mut shooter.weapon_arcs {
                arc.ready = false;
            }
        }
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        target.source_position.z = 10.0;
        target.hull_current = 1_000_000;
        target.hull_max = 1_000_000;
        let mut escorted = ship(true, TacticalWeaponArc::default(), 0.0);
        escorted.source_position.x = 30.0;
        let mut tactical = session(vec![shooter, target, escorted], Vec::new());
        tactical.tactical_rng = rng;
        for _ in 0..4 {
            tactical.step();
        }
        let sources = tactical
            .turn_events
            .iter()
            .filter_map(|queued| match queued.event {
                TacticalTurnEvent::WeaponFire(shot) => Some(shot.source),
                _ => None,
            })
            .collect();
        (sources, tactical.tactical_rng, tactical.weapon_draws)
    }

    /// `rng` after `count` weapon draws.
    fn after_draws(mut rng: OriginalTacticalRng, count: usize) -> OriginalTacticalRng {
        for _ in 0..count {
            rng.inclusive(0, crate::tactical::ORIGINAL_WEAPON_TURN_DRAW_MAX);
        }
        rng
    }

    // FUN_005b3a40 (0x5b3a92): each turn a unit with a ready arc draws 0 to
    // 2 (FUN_005a8a70) and fires only on 0.
    #[test]
    fn a_unit_fires_only_when_its_draw_comes_up_zero() {
        use crate::tactical::TacticalUnitRef;
        for draw in 0..=2 {
            let rng = stream_with_weapon_draws(&[draw]);
            let (sources, after, draws) =
                weapon_turn(TacticalOrder::AttackCapitalShips, true, rng);
            let fires = if draw == 0 { vec![TacticalUnitRef::Ship(0)] } else { Vec::new() };
            assert_eq!(sources, fires, "draw {draw}");
            assert_eq!(after, after_draws(rng, 1), "draw {draw}");
            assert_eq!(draws, 1, "draw {draw}");
        }
    }

    // FUN_005b3a40 (0x5b3a6b): a unit with no ready arc (+0x1e0, one arc
    // record 0x74 apart) returns before its draw.
    #[test]
    fn a_unit_with_no_ready_arc_does_not_draw() {
        let rng = stream_opening_weapon_turns(1);
        let (sources, after, draws) = weapon_turn(TacticalOrder::AttackCapitalShips, false, rng);
        assert!(sources.is_empty());
        assert_eq!(after, rng);
        assert_eq!(draws, 0);
    }

    // FUN_005b3a40 (0x5b3ae2): a force on Escort (order +0x64 = 1) returns
    // after the draw, before any target or fire.
    #[test]
    fn an_escort_draws_but_holds_its_fire() {
        let rng = stream_opening_weapon_turns(1);
        let (sources, after, draws) = weapon_turn(TacticalOrder::Escort, true, rng);
        assert!(sources.is_empty());
        assert_eq!(after, after_draws(rng, 1));
        assert_eq!(draws, 1);
        let (sources, _, _) = weapon_turn(TacticalOrder::None, true, rng);
        assert_eq!(sources, [crate::tactical::TacticalUnitRef::Ship(0)]);
    }

    // 0x005a9e43 (FUN_005ad870) and FUN_005b3a40 (0x5b3a5e): a unit out of
    // the fight, docked or in its jump takes no weapon turn, so it does not
    // draw. Each unit below fails one condition only.
    #[test]
    fn a_unit_out_of_the_fight_takes_no_weapon_turn() {
        use crate::tactical::TacticalFighterRecoveryState;
        let armed = || {
            let mut value = ship(true, TacticalWeaponArc::new(0, 0, 4), 30.0);
            value.weapon_arcs = [TacticalWeaponArc::new(0, 0, 4); 4];
            value
        };
        let mut dead = armed();
        dead.alive = false;
        let mut jumping = armed();
        jumping.retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
        let wing = || fighter(true, TacticalWeaponArc::new(4, 0, 0));
        let mut lost = wing();
        lost.alive = false;
        let mut empty = wing();
        empty.squad_count = 0;
        let mut leaving = wing();
        leaving.retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
        let mut docked = wing();
        docked.recovery_state = TacticalFighterRecoveryState::Recovered;
        // An unarmed enemy keeps the battle going and has no arc to draw for.
        let enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        let mut tactical = session(vec![dead, jumping, enemy], vec![lost, empty, leaving, docked]);
        let rng = stream_opening_weapon_turns(1);
        tactical.tactical_rng = rng;
        for _ in 0..4 {
            tactical.step();
        }
        assert_eq!(tactical.phase, crate::tactical::BattlePhase::Combat);
        assert_eq!(tactical.tactical_rng, rng);
    }

    // FUN_005b3a40 is slot +0x88 of the fighter class too (0x66c028): a
    // group draws after the capital ships (port order) and fires only on 0.
    #[test]
    fn a_fighter_group_takes_the_same_weapon_turn() {
        use crate::tactical::{TacticalAttackTarget, TacticalTurnEvent, TacticalUnitRef};
        let run = |draws: &[u32]| {
            let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
            target.source_position.z = 3.0;
            target.hull_current = 1_000_000;
            target.hull_max = 1_000_000;
            let mut wing = fighter(true, TacticalWeaponArc::new(4, 0, 0));
            wing.weapon_ranges = TacticalWeaponRanges {
                laser_cannon: 100.0,
                ion_cannon: 100.0,
                turbolaser: 100.0,
            };
            wing.order = TacticalOrder::AttackCapitalShips;
            wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
            let mut tactical = session(vec![target], vec![wing]);
            let rng = stream_with_weapon_draws(draws);
            tactical.tactical_rng = rng;
            for _ in 0..4 {
                tactical.step();
            }
            let fired = tactical.turn_events.iter().any(|queued| {
                matches!(
                    queued.event,
                    TacticalTurnEvent::WeaponFire(shot) if shot.source == TacticalUnitRef::Fighter(0)
                )
            });
            (fired, tactical.tactical_rng == after_draws(rng, 1))
        };
        assert_eq!(run(&[0]), (true, true));
        assert_eq!(run(&[1]), (false, true));
    }

    /// Every family's range at 100.
    fn long_ranges() -> TacticalWeaponRanges {
        TacticalWeaponRanges {
            laser_cannon: 100.0,
            ion_cannon: 100.0,
            turbolaser: 100.0,
        }
    }

    /// A capital ship with `arc` in all four arcs and every range at 100.
    fn gunship(alliance: bool, arc: TacticalWeaponArc) -> TacticalShip {
        let mut value = ship(alliance, arc, 30.0);
        value.weapon_arcs = [arc; 4];
        value.weapon_ranges = long_ranges();
        value.hull_current = 1_000_000;
        value.hull_max = 1_000_000;
        value
    }

    /// A fighter group with lasers and every range at 100, at `z`.
    fn wing_at(alliance: bool, z: f32) -> TacticalFighter {
        let mut value = fighter(alliance, TacticalWeaponArc::new(4, 0, 0));
        value.weapon_ranges = long_ranges();
        value.source_position.z = z;
        value
    }

    /// The shots one unit's weapons decide (`FUN_005b3a40` past its gate).
    fn unit_shots(
        tactical: &mut BattleSession,
        unit: crate::tactical::TacticalUnitRef,
    ) -> Vec<crate::tactical::TacticalShot> {
        let mut shots = Vec::new();
        tactical.fire_unit_weapons(unit, &mut shots);
        shots
    }

    fn shot_targets(shots: &[crate::tactical::TacticalShot]) -> Vec<crate::tactical::OriginalFighterTarget> {
        shots.iter().map(|shot| shot.target).collect()
    }

    // FUN_005cb6e0 (FUN_005aa970): with no target, a force whose first
    // member has only ion cannons or turbolasers fires at the nearest enemy
    // capital ship; one with laser cannons at the nearest enemy fighter
    // group, and, with none, at no capital ship at all (FUN_005b3a40 searches
    // the fighter tree +0x900 only).
    #[test]
    fn an_idle_force_fires_only_at_its_nearest_enemy() {
        use crate::tactical::{OriginalFighterTarget, TacticalUnitRef};
        let enemies = || {
            let mut far = gunship(false, TacticalWeaponArc::default());
            far.source_position.z = 50.0;
            let mut near = gunship(false, TacticalWeaponArc::default());
            near.source_position.z = 10.0;
            [far, near]
        };
        let [far, near] = enemies();
        let mut turbolasers = session(
            vec![gunship(true, TacticalWeaponArc::new(0, 0, 4)), far, near],
            Vec::new(),
        );
        let shots = unit_shots(&mut turbolasers, TacticalUnitRef::Ship(0));
        assert_eq!(shot_targets(&shots), [OriginalFighterTarget::Capital(2)]);

        let [far, near] = enemies();
        let mut lasers = session(
            vec![gunship(true, TacticalWeaponArc::new(4, 0, 0)), far, near],
            Vec::new(),
        );
        assert!(unit_shots(&mut lasers, TacticalUnitRef::Ship(0)).is_empty());
        lasers.fighters.push(wing_at(false, 40.0));
        lasers.fighters.push(wing_at(false, 20.0));
        let shots = unit_shots(&mut lasers, TacticalUnitRef::Ship(0));
        assert_eq!(shot_targets(&shots), [OriginalFighterTarget::Fighter(1)]);

        let [far, near] = enemies();
        let mut unarmed = session(
            vec![gunship(true, TacticalWeaponArc::default()), far, near],
            vec![wing_at(false, 20.0)],
        );
        assert!(unit_shots(&mut unarmed, TacticalUnitRef::Ship(0)).is_empty());
    }

    // FUN_005cb6e0 measures from the force's mover anchor (+0x54 + 0x3c,
    // FUN_005a2260), not from the unit; the first of equal distances stays
    // (FUN_005aa970's strict <).
    #[test]
    fn the_nearest_enemy_is_measured_from_the_forces_anchor() {
        use crate::tactical::{OriginalFighterTarget, TacticalUnitRef};
        let mut by_the_unit = gunship(false, TacticalWeaponArc::default());
        by_the_unit.source_position.z = 10.0;
        let mut by_the_anchor = gunship(false, TacticalWeaponArc::default());
        by_the_anchor.source_position.z = 45.0;
        let mut tactical = session(
            vec![
                gunship(true, TacticalWeaponArc::new(0, 0, 4)),
                by_the_unit,
                by_the_anchor,
            ],
            Vec::new(),
        );
        tactical.update_force_movers();
        let key = tactical.force_key(0);
        for mover in &mut tactical.force_movers {
            if mover.force == key {
                mover.anchor.z = 40.0;
            }
        }
        let shots = unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        assert_eq!(shot_targets(&shots), [OriginalFighterTarget::Capital(2)]);

        let mut tied = session(
            vec![gunship(true, TacticalWeaponArc::new(0, 0, 4))],
            Vec::new(),
        );
        for x in [-10.0, 10.0] {
            let mut enemy = gunship(false, TacticalWeaponArc::default());
            enemy.source_position.x = x;
            tied.ships.push(enemy);
        }
        let shots = unit_shots(&mut tied, TacticalUnitRef::Ship(0));
        assert_eq!(shot_targets(&shots), [OriginalFighterTarget::Capital(1)]);
    }

    // FUN_005b6530 returns 2 beyond the longest range (FUN_005b18d0), and
    // FUN_005b3a40 then tries the force's nearest enemy; its 1 (the arc not
    // ready) stops the turn there.
    #[test]
    fn a_target_out_of_range_falls_to_the_nearest_enemy() {
        use crate::tactical::{OriginalFighterTarget, TacticalAttackTarget, TacticalUnitRef};
        let mut far = gunship(false, TacticalWeaponArc::default());
        far.source_position.z = 150.0;
        let mut beside = gunship(false, TacticalWeaponArc::default());
        beside.source_position.x = 30.0;
        let mut shooter = gunship(true, TacticalWeaponArc::new(0, 0, 4));
        shooter.order = TacticalOrder::AttackCapitalShips;
        shooter.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        let mut tactical = session(vec![shooter, far, beside], Vec::new());
        let shots = unit_shots(&mut tactical.clone(), TacticalUnitRef::Ship(0));
        assert_eq!(shot_targets(&shots), [OriginalFighterTarget::Capital(2)]);

        // The target in the fore arc at 30, that arc spent: no fallback.
        tactical.ships[1].source_position.z = 30.0;
        tactical.ships[2].source_position.x = 20.0;
        tactical.ships[0].weapon_arcs[0].ready = false;
        assert!(unit_shots(&mut tactical, TacticalUnitRef::Ship(0)).is_empty());
    }

    // FUN_005b3a40 (0x5b3afc): when no arc can hurt the target's class
    // (slot +0x84, FUN_005b4c60, returns 4), the turn ends before any other
    // shot; ion cannons do not hurt fighters.
    #[test]
    fn no_weapon_against_the_targets_class_ends_the_turn() {
        use crate::tactical::{OriginalFighterTarget, TacticalAttackTarget, TacticalUnitRef};
        let mut ions = gunship(true, TacticalWeaponArc::new(0, 4, 0));
        ions.order = TacticalOrder::AttackFighters;
        ions.attack_target = Some(TacticalAttackTarget::FighterGroup(0));
        let mut capital = gunship(false, TacticalWeaponArc::default());
        capital.source_position.z = 10.0;
        let mut tactical = session(vec![ions, capital], vec![wing_at(false, 10.0)]);
        assert!(unit_shots(&mut tactical.clone(), TacticalUnitRef::Ship(0)).is_empty());
        tactical.ships[0].order = TacticalOrder::None;
        tactical.ships[0].attack_target = None;
        let shots = unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        assert_eq!(shot_targets(&shots), [OriginalFighterTarget::Capital(1)]);
    }

    // FUN_005b3a40 (0x5b3b83): on Attack Fighters, after the direct shot at
    // its target, the unit searches the groups nearby with its other ready
    // arcs; a fired arc keeps the energy of the families it did not fire
    // (FUN_005b3f10).
    #[test]
    fn attack_fighters_fires_at_its_target_then_at_groups_nearby() {
        use crate::tactical::{OriginalFighterTarget, TacticalAttackTarget, TacticalUnitRef};
        let mut shooter = gunship(true, TacticalWeaponArc::new(4, 2, 0));
        shooter.order = TacticalOrder::AttackFighters;
        shooter.attack_target = Some(TacticalAttackTarget::FighterGroup(0));
        let mut tactical = session(
            vec![shooter],
            vec![wing_at(false, 10.0), wing_at(false, -10.0)],
        );
        let shots = unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        assert_eq!(
            shot_targets(&shots),
            [OriginalFighterTarget::Fighter(0), OriginalFighterTarget::Fighter(1)]
        );
        let ship = &tactical.ships[0];
        assert_eq!(ship.weapon_recharge_queue, vec![0, 2]);
        assert_eq!(ship.weapon_arcs[0].energy, 2.0);
        assert!(ship.weapon_arcs[1].ready && ship.weapon_arcs[3].ready);
    }

    // FUN_005b3a40's search (FUN_005cdf50, then 0x5b3cf0..): a family keeps
    // a group while its kept strength is below its full strength, and keeps
    // it scaled by the hull (FUN_005b1770). A whole unit keeps the first
    // group it walks to, a damaged one the last. The nearest group, behind,
    // takes the aft arc's direct shot first (FUN_005cb6e0, FUN_005b6530).
    #[test]
    fn the_search_keeps_the_first_group_when_whole_and_the_last_when_damaged() {
        use crate::tactical::{OriginalFighterTarget, TacticalUnitRef};
        let run = |hull: i32| {
            let mut shooter = gunship(true, TacticalWeaponArc::new(4, 0, 0));
            shooter.order = TacticalOrder::AttackFighters;
            shooter.hull_current = hull;
            let behind = wing_at(false, -5.0);
            let mut left = wing_at(false, 10.0);
            left.source_position.x = -1.0;
            let mut right = wing_at(false, 10.0);
            right.source_position.x = 1.0;
            let mut tactical = session(vec![shooter], vec![behind, left, right]);
            let shots = unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
            (shot_targets(&shots), shots.get(1).map(|shot| shot.strength))
        };
        let (whole, whole_strength) = run(1_000_000);
        assert_eq!(
            whole,
            [OriginalFighterTarget::Fighter(0), OriginalFighterTarget::Fighter(1)]
        );
        let (damaged, damaged_strength) = run(500_000);
        assert_eq!(
            damaged,
            [OriginalFighterTarget::Fighter(0), OriginalFighterTarget::Fighter(2)]
        );
        assert!((damaged_strength.unwrap() * 2.0 - whole_strength.unwrap()).abs() < 0.000_1);
    }

    // FUN_005cdf50 and FUN_005b3a40's search: enemy groups in the fight
    // (not in their jump) within a family's range of a ready arc. The
    // nearest group ahead is a decoy: the fore arc has no guns, so the direct
    // shot at a fighter target stops at 1 (FUN_005b6530), and Attack
    // Fighters still searches (0x5b3b83).
    #[test]
    fn the_search_takes_enemy_groups_in_range_of_a_ready_arc() {
        use crate::tactical::{OriginalFighterTarget, TacticalUnitRef};
        let run = |x: f32, alliance: bool, jumping: bool| {
            let mut shooter = gunship(true, TacticalWeaponArc::default());
            shooter.weapon_arcs[1] = TacticalWeaponArc::new(4, 0, 0);
            shooter.order = TacticalOrder::AttackFighters;
            let decoy = wing_at(false, 5.0);
            let mut wing = wing_at(alliance, 0.0);
            wing.source_position.x = x;
            if jumping {
                wing.retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
            }
            let mut tactical = session(vec![shooter], vec![decoy, wing]);
            shot_targets(&unit_shots(&mut tactical, TacticalUnitRef::Ship(0)))
        };
        assert_eq!(run(60.0, false, false), [OriginalFighterTarget::Fighter(1)]);
        assert_eq!(run(100.0, false, false), [OriginalFighterTarget::Fighter(1)]);
        assert!(run(100.5, false, false).is_empty());
        assert!(run(60.0, true, false).is_empty());
        assert!(run(60.0, false, true).is_empty());
    }

    // FUN_005b3a40 (0x5b3b83): the search runs on Attack Fighters, or with
    // laser cannons when the direct shots came to 2 (out of range); a
    // fired laser ship, or one without lasers out of range, does not search.
    #[test]
    fn only_lasers_out_of_range_or_attack_fighters_search() {
        use crate::tactical::{OriginalFighterTarget, TacticalUnitRef};
        let mut lasers = session(
            vec![gunship(true, TacticalWeaponArc::new(4, 0, 0))],
            vec![wing_at(false, 10.0), wing_at(false, -20.0)],
        );
        let shots = unit_shots(&mut lasers, TacticalUnitRef::Ship(0));
        assert_eq!(shot_targets(&shots), [OriginalFighterTarget::Fighter(0)]);

        let mut far = gunship(false, TacticalWeaponArc::default());
        far.source_position.z = 150.0;
        let mut turbolasers = session(
            vec![gunship(true, TacticalWeaponArc::new(0, 0, 4)), far],
            vec![wing_at(false, 10.0)],
        );
        assert!(unit_shots(&mut turbolasers, TacticalUnitRef::Ship(0)).is_empty());
    }

    // The force's target (+0x8c) is an enemy in the fight (FUN_005d2190,
    // FUN_005ad870): a friend or a dead enemy draws no shot.
    #[test]
    fn a_friend_or_a_dead_ship_is_no_target() {
        use crate::tactical::{TacticalAttackTarget, TacticalUnitRef};
        for (alliance, alive) in [(true, true), (false, false)] {
            let mut shooter = gunship(true, TacticalWeaponArc::new(0, 0, 4));
            shooter.order = TacticalOrder::AttackCapitalShips;
            shooter.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
            let mut target = gunship(alliance, TacticalWeaponArc::default());
            target.source_position.z = 10.0;
            target.alive = alive;
            let mut tactical = session(vec![shooter, target], Vec::new());
            assert!(unit_shots(&mut tactical, TacticalUnitRef::Ship(0)).is_empty());
        }
    }

    // FUN_005aa970 and FUN_005b6530 measure squared three-dimensional
    // distances, each axis squared.
    #[test]
    fn distances_take_every_axis_squared() {
        use crate::tactical::{OriginalFighterTarget, TacticalUnitRef};
        let at = |x: f32, y: f32, z: f32| TacticalWorldPosition { x, y, z };
        let nearest = |first: TacticalWorldPosition, second: TacticalWorldPosition| {
            let mut tactical = session(
                vec![gunship(true, TacticalWeaponArc::new(0, 0, 4))],
                Vec::new(),
            );
            for position in [first, second] {
                let mut enemy = gunship(false, TacticalWeaponArc::default());
                enemy.source_position = position;
                tactical.ships.push(enemy);
            }
            shot_targets(&unit_shots(&mut tactical, TacticalUnitRef::Ship(0)))
        };
        assert_eq!(
            nearest(at(0.0, 0.0, 10.0), at(0.0, 6.0, 8.2)),
            [OriginalFighterTarget::Capital(1)]
        );
        assert_eq!(
            nearest(at(10.0, 0.0, 0.0), at(0.0, 0.0, 10.1)),
            [OriginalFighterTarget::Capital(1)]
        );
        assert_eq!(
            nearest(at(0.0, 0.0, 10.0), at(6.0, 6.0, 0.0)),
            [OriginalFighterTarget::Capital(2)]
        );
        assert_eq!(
            nearest(at(10.0, 0.0, 0.0), at(0.0, 0.0, 9.9)),
            [OriginalFighterTarget::Capital(2)]
        );
        for position in [at(100.5, 0.0, 0.0), at(0.0, 100.5, 0.0), at(0.0, 0.0, 100.5), at(60.0, 60.0, 60.0)] {
            let mut tactical = session(
                vec![gunship(true, TacticalWeaponArc::new(0, 0, 4))],
                Vec::new(),
            );
            let mut enemy = gunship(false, TacticalWeaponArc::default());
            enemy.source_position = position;
            tactical.ships.push(enemy);
            assert!(
                unit_shots(&mut tactical, TacticalUnitRef::Ship(0)).is_empty(),
                "{position:?}"
            );
        }
    }

    // FUN_005cdf50 walks the x-sorted tree out from the unit: a capital
    // ship's search point sorts after groups of equal x (the walk goes down
    // through them, the last first), and a group walks from its own node.
    // With the tie rule, a whole unit keeps the first group it walks to.
    #[test]
    fn the_search_walks_out_from_the_unit_by_x() {
        use crate::tactical::{OriginalFighterTarget, TacticalUnitRef};
        let capital = |hull: i32| {
            let mut shooter = gunship(true, TacticalWeaponArc::default());
            shooter.weapon_arcs[0] = TacticalWeaponArc::new(4, 0, 0);
            shooter.order = TacticalOrder::AttackFighters;
            shooter.hull_current = hull;
            let mut behind = wing_at(false, -10.0);
            behind.source_position.x = -1.0;
            let mut tactical = session(
                vec![shooter],
                vec![wing_at(false, 10.0), wing_at(false, 12.0), behind, wing_at(false, -3.0)],
            );
            shot_targets(&unit_shots(&mut tactical, TacticalUnitRef::Ship(0)))
        };
        assert_eq!(capital(1_000_000), [OriginalFighterTarget::Fighter(1)]);
        assert_eq!(capital(500_000), [OriginalFighterTarget::Fighter(0)]);

        let group = |hull: f32| {
            let mut shooter = wing_at(true, 0.0);
            shooter.order = TacticalOrder::AttackFighters;
            shooter.hull_current = hull;
            let mut tactical = session(
                Vec::new(),
                vec![wing_at(false, 10.0), shooter, wing_at(false, 12.0), wing_at(false, -150.0)],
            );
            tactical.sync_fighter_movers();
            for mover in &mut tactical.fighter_movers {
                if mover.slots.iter().any(|(member, _)| *member == 1) {
                    mover.anchor.z = -200.0;
                }
            }
            let shots = unit_shots(&mut tactical, TacticalUnitRef::Fighter(1));
            (shot_targets(&shots), shots.first().map(|shot| shot.strength))
        };
        let (whole, whole_strength) = group(96.0);
        assert_eq!(whole, [OriginalFighterTarget::Fighter(0)]);
        let (damaged, damaged_strength) = group(48.0);
        assert_eq!(damaged, [OriginalFighterTarget::Fighter(2)]);
        // The group's own guns scale with its hull (FUN_005b9c00), and the
        // search scales them once more (FUN_005b1770).
        assert!((damaged_strength.unwrap() * 4.0 - whole_strength.unwrap()).abs() < 0.000_1);
    }

    // FUN_005b3f10 (FUN_005b23a0): a group's torpedoes follow its guns only
    // at a capital target without shields, within torpedo range, with a
    // torpedo rating.
    #[test]
    fn torpedoes_need_no_shields_range_and_a_rating() {
        use crate::tactical::{TacticalAttackTarget, TacticalUnitRef, WeaponKind};
        let run = |shield: i32, torpedoes: u32, z: f32| {
            let mut bomber = wing_at(true, 0.0);
            bomber.torpedo_strength = torpedoes;
            bomber.torpedo_range = 7.0;
            bomber.order = TacticalOrder::AttackCapitalShips;
            bomber.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
            let mut target = gunship(false, TacticalWeaponArc::default());
            target.shield = shield;
            target.source_position.z = z;
            let mut tactical = session(vec![target], vec![bomber]);
            unit_shots(&mut tactical, TacticalUnitRef::Fighter(0))
                .iter()
                .any(|shot| shot.kind == WeaponKind::FighterAttack)
        };
        assert!(run(0, 16, 7.0));
        assert!(!run(10, 16, 5.0));
        assert!(!run(0, 0, 5.0));
        assert!(!run(0, 16, 7.5));
    }

    /// A turbolaser ship with a tractor beam of power 10 and range 20, and
    /// an enemy at `z` as its force's target.
    fn tractor_pair(z: f32) -> BattleSession {
        use crate::tactical::TacticalAttackTarget;
        let mut source = gunship(true, TacticalWeaponArc::new(0, 0, 4));
        source.tractor_beam_power = 10.0;
        source.tractor_beam_range = 20.0;
        source.subsystem_capacity.tractor = true;
        source.order = TacticalOrder::AttackCapitalShips;
        source.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        let mut target = gunship(false, TacticalWeaponArc::default());
        target.source_position.z = z;
        target.sublight_engine_power = 50.0;
        session(vec![source, target], Vec::new())
    }

    /// The tractor events a session has queued.
    fn tractor_events(tactical: &BattleSession) -> Vec<crate::tactical::TacticalTurnEvent> {
        use crate::tactical::TacticalTurnEvent;
        tactical
            .turn_events
            .iter()
            .map(|queued| queued.event.clone())
            .filter(|event| {
                matches!(
                    event,
                    TacticalTurnEvent::TractorOn { .. } | TacticalTurnEvent::TractorOff { .. }
                )
            })
            .collect()
    }

    // FUN_005b2140 from the weapon turn: a ship with power and its target
    // within range posts event 0x28 and waits (+0x3ac); its handler
    // FUN_005b2630 (slot +0x40, FUN_005b2570) holds the target, whose
    // engines then lose the beam's power (FUN_005b17f0, FUN_005b1790).
    #[test]
    fn a_ship_takes_its_target_into_its_tractor_beam_through_event_0x28() {
        use crate::tactical::{original_tactical_mobility, TacticalTurnEvent, TacticalUnitRef};
        let mut tactical = tractor_pair(10.0);
        let free = original_tactical_mobility(&tactical.ships, &tactical.field_effects, 1)
            .unwrap()
            .effective_engine_power;
        unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        let on = TacticalTurnEvent::TractorOn { source: 0, target: crate::tactical::TacticalUnitRef::Ship(1) };
        assert_eq!(tractor_events(&tactical), [on.clone()]);
        assert!(tactical.ships[0].tractor_request_pending);
        assert_eq!(tactical.tractor_target_of(0), None);
        // Waiting, it asks nothing more.
        unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        assert_eq!(tractor_events(&tactical).len(), 1);

        tactical.run_turn_event(on);
        assert_eq!(tactical.tractor_target_of(0), Some(crate::tactical::TacticalUnitRef::Ship(1)));
        assert!(!tactical.ships[0].tractor_request_pending);
        let held = original_tactical_mobility(&tactical.ships, &tactical.field_effects, 1)
            .unwrap()
            .effective_engine_power;
        assert_eq!(held, free - 10.0);
        // Held within range, it keeps it and asks nothing.
        tactical.turn_events.clear();
        unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        assert!(tractor_events(&tactical).is_empty());
    }

    // FUN_005b2140: beyond the beam's range (slot +0x5c, +0x39c) or without
    // a beam (+0x3b8), a ship asks nothing; FUN_005b2630 holds nothing when
    // either ship has left the fight, and the request stays on its way.
    #[test]
    fn a_tractor_beam_needs_range_power_and_both_ships_in_the_fight() {
        use crate::tactical::{TacticalTurnEvent, TacticalUnitRef};
        let mut far = tractor_pair(20.5);
        unit_shots(&mut far, TacticalUnitRef::Ship(0));
        assert!(tractor_events(&far).is_empty());
        let mut edge = tractor_pair(20.0);
        unit_shots(&mut edge, TacticalUnitRef::Ship(0));
        assert_eq!(tractor_events(&edge).len(), 1);
        let mut unpowered = tractor_pair(10.0);
        unpowered.ships[0].tractor_beam_power = 0.0;
        unit_shots(&mut unpowered, TacticalUnitRef::Ship(0));
        assert!(tractor_events(&unpowered).is_empty());
        let mut broken = tractor_pair(10.0);
        broken.ships[0].subsystem_damage.tractor = 4;
        unit_shots(&mut broken, TacticalUnitRef::Ship(0));
        assert!(tractor_events(&broken).is_empty());

        let mut gone = tractor_pair(10.0);
        unit_shots(&mut gone, TacticalUnitRef::Ship(0));
        gone.ships[1].alive = false;
        gone.run_turn_event(TacticalTurnEvent::TractorOn { source: 0, target: crate::tactical::TacticalUnitRef::Ship(1) });
        assert_eq!(gone.tractor_target_of(0), None);
        assert!(gone.ships[0].tractor_request_pending);
    }

    // FUN_005b2140: a held ship beyond range, out of the fight, or no
    // longer the force's target is let go (event 0x29); a ship holding
    // another lets that one go first. FUN_005b26a0 (slot +0x48,
    // FUN_005b2440) ends the hold and the wait.
    #[test]
    fn a_tractor_beam_lets_go_through_event_0x29() {
        use crate::tactical::{TacticalTurnEvent, TacticalUnitRef};
        let held = |z: f32| {
            let mut tactical = tractor_pair(z);
            assert!(tactical.set_tractor_field(0, 1, true));
            tactical
        };
        let off = TacticalTurnEvent::TractorOff { source: 0, target: crate::tactical::TacticalUnitRef::Ship(1) };
        let mut beyond = held(25.0);
        unit_shots(&mut beyond, TacticalUnitRef::Ship(0));
        assert_eq!(tractor_events(&beyond), [off.clone()]);
        assert!(beyond.ships[0].tractor_request_pending);
        beyond.run_turn_event(off.clone());
        assert_eq!(beyond.tractor_target_of(0), None);
        assert!(!beyond.ships[0].tractor_request_pending);

        let mut untargeted = held(10.0);
        untargeted.ships[0].attack_target = None;
        untargeted.ships[0].order = TacticalOrder::None;
        unit_shots(&mut untargeted, TacticalUnitRef::Ship(0));
        assert_eq!(tractor_events(&untargeted), [off.clone()]);

        let mut other = held(10.0);
        let mut second = gunship(false, TacticalWeaponArc::default());
        second.source_position.z = 12.0;
        other.ships.push(second);
        other.ships[0].attack_target =
            Some(crate::tactical::TacticalAttackTarget::CapitalShip(2));
        unit_shots(&mut other, TacticalUnitRef::Ship(0));
        assert_eq!(tractor_events(&other), [off]);

        let mut grouped = held(10.0);
        grouped.fighters.push(wing_at(false, 5.0));
        grouped.ships[0].order = TacticalOrder::AttackFighters;
        grouped.ships[0].attack_target =
            Some(crate::tactical::TacticalAttackTarget::FighterGroup(0));
        unit_shots(&mut grouped, TacticalUnitRef::Ship(0));
        assert_eq!(
            tractor_events(&grouped),
            [TacticalTurnEvent::TractorOff { source: 0, target: crate::tactical::TacticalUnitRef::Ship(1) }]
        );
    }

    // FUN_005b3a40 (0x5b3b05): a unit whose arcs cannot hurt its target's
    // class returns before FUN_005b2140, so an ion ship ordered against a
    // group keeps the ship it holds.
    #[test]
    fn a_turn_ended_by_the_targets_class_keeps_the_tractor_hold() {
        use crate::tactical::{TacticalAttackTarget, TacticalUnitRef};
        let mut tactical = tractor_pair(10.0);
        tactical.ships[0].weapon_arcs = [TacticalWeaponArc::new(0, 4, 0); 4];
        assert!(tactical.set_tractor_field(0, 1, true));
        tactical.fighters.push(wing_at(false, 5.0));
        tactical.ships[0].order = TacticalOrder::AttackFighters;
        tactical.ships[0].attack_target = Some(TacticalAttackTarget::FighterGroup(0));
        unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        assert!(tractor_events(&tactical).is_empty());
        // With a laser it reaches FUN_005b2140, which lets the ship go.
        tactical.ships[0].weapon_arcs[0] = TacticalWeaponArc::new(4, 4, 0);
        unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        assert_eq!(tractor_events(&tactical).len(), 1);
    }

    // FUN_005b2140 takes any unit (FUN_005a6340): a ship on Attack Fighters
    // holds its target group through 0x28 (FUN_005b2630, slot +0x40 of
    // the ship, FUN_005b2570 on the group's +0x360), and the group's speed
    // (0x5ba190) loses the beam's power (FUN_005b1790).
    #[test]
    fn a_ship_holds_a_fighter_group_in_its_tractor_beam() {
        use crate::tactical::{TacticalAttackTarget, TacticalTurnEvent, TacticalUnitRef};
        let mut tactical = tractor_pair(10.0);
        tactical.ships[0].weapon_arcs = [TacticalWeaponArc::new(4, 0, 0); 4];
        tactical.ships[0].order = TacticalOrder::AttackFighters;
        tactical.ships[0].attack_target = Some(TacticalAttackTarget::FighterGroup(0));
        tactical.fighters.push(wing_at(false, 15.0));
        let free = tactical.original_fighter_speed(0);
        unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        let on = TacticalTurnEvent::TractorOn {
            source: 0,
            target: TacticalUnitRef::Fighter(0),
        };
        assert_eq!(tractor_events(&tactical), [on.clone()]);

        let mut leaving = tactical.clone();
        leaving.fighters[0].retreat_progress = crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS;
        leaving.run_turn_event(on.clone());
        assert_eq!(leaving.tractor_target_of(0), None);

        tactical.run_turn_event(on);
        assert_eq!(tactical.tractor_target_of(0), Some(TacticalUnitRef::Fighter(0)));
        assert_eq!(tactical.original_fighter_speed(0), free - 10.0);
        // Out of range, the group is let go.
        tactical.fighters[0].source_position.z = 25.0;
        tactical.turn_events.clear();
        unit_shots(&mut tactical, TacticalUnitRef::Ship(0));
        assert_eq!(
            tractor_events(&tactical),
            [TacticalTurnEvent::TractorOff {
                source: 0,
                target: TacticalUnitRef::Fighter(0),
            }]
        );
    }

    // FUN_005b1970 case 2: a tractor hit that leaves the beam no power
    // (FUN_005b1790) posts 0x29; slot +0x68 (0x005b0ad0) on a ship's
    // destruction does the same.
    #[test]
    fn a_tractor_beam_without_power_or_ship_lets_go() {
        use crate::tactical::{TacticalSubsystemKind, TacticalTurnEvent};
        let off = TacticalTurnEvent::TractorOff { source: 0, target: crate::tactical::TacticalUnitRef::Ship(1) };
        let mut hit = tractor_pair(10.0);
        assert!(hit.set_tractor_field(0, 1, true));
        hit.ships[0].subsystem_damage.tractor = 3;
        hit.hit_subsystem(0, TacticalSubsystemKind::Tractor);
        assert_eq!(tractor_events(&hit), [off.clone()]);
        assert!(hit.ships[0].tractor_request_pending);

        let mut weakened = tractor_pair(10.0);
        assert!(weakened.set_tractor_field(0, 1, true));
        weakened.hit_subsystem(0, TacticalSubsystemKind::Tractor);
        assert!(tractor_events(&weakened).is_empty());

        let mut destroyed = tractor_pair(10.0);
        assert!(destroyed.set_tractor_field(0, 1, true));
        destroyed.ships[0].hull_current = 1;
        destroyed.ships[0].shield = 0;
        destroyed.hit_weapon_shot(crate::tactical::TacticalShot {
            source: crate::tactical::TacticalUnitRef::Ship(1),
            target: crate::tactical::OriginalFighterTarget::Capital(0),
            kind: crate::tactical::WeaponKind::Turbolaser,
            strength: 100.0,
        });
        assert!(!destroyed.ships[0].alive);
        assert_eq!(tractor_events(&destroyed), [off]);
    }

    /// An enemy ship with shields `shield` and 100 hull, hit by an ion
    /// shot of `strength` from a stream that yields `draws` as (minimum,
    /// maximum, value) triples.
    fn ion_hit(shield: i32, strength: f32, draws: &[(u32, u32, u32)]) -> BattleSession {
        use crate::tactical::{OriginalFighterTarget, TacticalShot, TacticalUnitRef, WeaponKind};
        let shooter = gunship(true, TacticalWeaponArc::new(0, 4, 0));
        let mut target = gunship(false, TacticalWeaponArc::new(0, 0, 4));
        target.shield = shield;
        target.shield_max = shield.max(1);
        target.hull_current = 100;
        target.hull_max = 100;
        let mut tactical = session(vec![shooter, target], Vec::new());
        tactical.tactical_rng = (0..)
            .map(OriginalTacticalRng::new)
            .find(|rng| {
                let mut rng = *rng;
                draws
                    .iter()
                    .all(|&(minimum, maximum, value)| rng.inclusive(minimum, maximum) == value)
            })
            .expect("a stream gives the draws");
        tactical.hit_weapon_shot(TacticalShot {
            source: TacticalUnitRef::Ship(0),
            target: OriginalFighterTarget::Capital(1),
            kind: WeaponKind::IonCannon,
            strength,
        });
        tactical
    }

    // FUN_005b5b60 (event 0x12, slot +0xdc): ions never touch the hull.
    // While the shields hold they lose the damage, and a draw of 0 to 100
    // at most the share lost (at least 1) posts a shield hit (0x1e).
    #[test]
    fn ions_drain_the_shields_and_never_the_hull() {
        use crate::tactical::{TacticalSubsystemKind, TacticalTurnEvent};
        let shield_hit = |tactical: &BattleSession| {
            tactical.turn_events.iter().any(|queued| {
                queued.event
                    == TacticalTurnEvent::SubsystemHit {
                        ship: 1,
                        kind: TacticalSubsystemKind::Shields,
                    }
            })
        };
        let held = ion_hit(20, 5.0, &[(0, 100, 25)]);
        assert_eq!((held.ships[1].shield, held.ships[1].hull_current), (15, 100));
        assert!(shield_hit(&held));
        let missed = ion_hit(20, 5.0, &[(0, 100, 26)]);
        assert_eq!(missed.ships[1].shield, 15);
        assert!(!shield_hit(&missed));
        // Half a percent lost still counts as 1.
        let small = ion_hit(400, 2.0, &[(0, 100, 1)]);
        assert!(shield_hit(&small));
    }

    // FUN_005b5b60's overflow: shields to 0, hull untouched, and per point
    // beyond them a draw of 1 to 10: 1 a dead timer (+0x3b0, 30 to 50), 2
    // the engines' stun (+0x3b4, 30 to 50), 3 to 10 arc (draw - 3) / 2;
    // event 0x16 carries the stun and the arcs.
    #[test]
    fn an_ion_overflow_stuns_engines_and_knocks_out_arcs() {
        use crate::tactical::TacticalTurnEvent;
        let tactical = ion_hit(
            2,
            6.0,
            &[(1, 10, 2), (30, 50, 41), (1, 10, 1), (30, 50, 33), (1, 10, 4), (1, 10, 10)],
        );
        assert_eq!((tactical.ships[1].shield, tactical.ships[1].hull_current), (0, 100));
        let events = tactical
            .turn_events
            .iter()
            .filter(|queued| matches!(queued.event, TacticalTurnEvent::IonDisable { .. }))
            .map(|queued| queued.event.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            events,
            [TacticalTurnEvent::IonDisable {
                ship: 1,
                engine_stun: 41.0,
                arcs: [true, false, false, true],
            }]
        );
    }

    // Event 0x16 (FUN_005b2340): the stun grows and the arcs are emptied
    // (FUN_005b6980) and queued (FUN_005b64e0); a stunned ship's speed is 0
    // (FUN_005b17f0); the stun runs down a second a turn (FUN_005b0330);
    // a queued arc with no guns is ready once recharged (FUN_005b6320).
    #[test]
    fn an_ion_stun_stops_the_engines_for_its_seconds() {
        use crate::tactical::{original_tactical_mobility, TacticalTurnEvent};
        let mut tactical = ion_hit(10, 1.0, &[(0, 100, 100)]);
        tactical.ships[1].sublight_engine_power = 50.0;
        tactical.ships[1].weapon_arcs[1] = TacticalWeaponArc::default();
        tactical.run_turn_event(TacticalTurnEvent::IonDisable {
            ship: 1,
            engine_stun: 2.0,
            arcs: [true, true, false, false],
        });
        let ship = &tactical.ships[1];
        assert_eq!(ship.ion_engine_stun, 2.0);
        assert!(!ship.weapon_arcs[0].ready && ship.weapon_arcs[0].energy == 0.0);
        assert_eq!(ship.weapon_recharge_queue, vec![0, 1]);
        let speed = |tactical: &BattleSession| {
            original_tactical_mobility(&tactical.ships, &tactical.field_effects, 1)
                .unwrap()
                .effective_engine_power
        };
        assert_eq!(speed(&tactical), 0.0);
        for _ in 0..4 {
            tactical.step();
        }
        assert_eq!(tactical.ships[1].ion_engine_stun, 1.0);
        assert_eq!(speed(&tactical), 0.0);
        for _ in 0..4 {
            tactical.step();
        }
        assert_eq!(tactical.ships[1].ion_engine_stun, 0.0);
        assert!(speed(&tactical) > 0.0);
        assert!(tactical.ships[1].weapon_arcs[1].ready);
    }

    /// The computer's (Empire's) ships after its first review (turn 1):
    /// each ship's task force.
    fn regrouped(ai: Vec<TacticalShip>) -> Vec<u8> {
        let mut ships = vec![ship(true, TacticalWeaponArc::default(), 0.0)];
        ships.extend(ai);
        let mut tactical = session(ships, Vec::new());
        for _ in 0..4 {
            tactical.step();
        }
        assert_eq!(tactical.commanders_regrouped, [false, true]);
        tactical.ships[1..].iter().map(|ship| ship.task_force).collect()
    }

    fn armed(arc: TacticalWeaponArc, range: f32) -> TacticalShip {
        let mut value = ship(false, arc, 0.0);
        value.weapon_ranges = TacticalWeaponRanges {
            laser_cannon: range,
            ion_cannon: range,
            turbolaser: range,
        };
        value.hull_current = 1_000_000;
        value.hull_max = 1_000_000;
        value
    }

    // FUN_005c8780's first review runs slot +0x18 (FUN_005c8300) once:
    // FUN_005c8660 makes a list of c ships the largest k forces with
    // k * k <= c; every (c / k)-th ship starts one (FUN_005aded0), the
    // others join the last (FUN_005ade50). The laser list (+0xa0) follows
    // the armed list (+0x98), the unarmed list (+0xa8) comes last.
    #[test]
    fn the_first_review_regroups_the_computers_ships() {
        let lasers = || armed(TacticalWeaponArc::new(4, 0, 0), 10.0);
        // Five laser ships: two forces, ships 0-1 and 2-4.
        assert_eq!(regrouped(vec![lasers(), lasers(), lasers(), lasers(), lasers()]), [0, 0, 1, 1, 1]);
        // Turbolasers (no lasers) first, then lasers, then the unarmed.
        let turbolasers = || armed(TacticalWeaponArc::new(0, 0, 4), 10.0);
        let unarmed = armed(TacticalWeaponArc::default(), 0.0);
        assert_eq!(
            regrouped(vec![lasers(), unarmed, turbolasers(), lasers()]),
            [1, 2, 0, 1]
        );
        // Six ships make two forces (2 * 2 <= 6 < 3 * 3).
        assert_eq!(regrouped(vec![turbolasers(); 6]), [0, 0, 0, 1, 1, 1]);
        // After one armed ship, sixteen laser ships make four forces in
        // slots 1 to 4: the search may reach k + 3 there, and the cap is
        // the list's four slots.
        let mut mixed = vec![turbolasers()];
        mixed.extend((0..16).map(|_| lasers()));
        let slots = regrouped(mixed);
        assert_eq!(slots[0], 0);
        assert_eq!(*slots.iter().max().unwrap(), 4);
        // Nine ships make three forces of three.
        assert_eq!(regrouped(vec![turbolasers(); 9]), [0, 0, 0, 1, 1, 1, 2, 2, 2]);
        // The search for k stops past the list's last slot (3), so the
        // first list makes at most three forces: 25 ships, 8 a force.
        let many = regrouped(vec![turbolasers(); 25]);
        assert_eq!(*many.iter().max().unwrap(), 2);
        assert_eq!(many.iter().filter(|&&slot| slot == 0).count(), 8);
    }

    // FUN_005c81d0 sorts each list by -(speed + 10000 x longest range)
    // (DAT_0066cbb0, FUN_005b18d0), the first of equal keys first
    // (FUN_005f4f10): the longest-ranged ships lead the first force.
    #[test]
    fn the_regroup_puts_the_longest_ranges_first() {
        let turbolasers = |range: f32| armed(TacticalWeaponArc::new(0, 0, 4), range);
        let mut fast = turbolasers(10.0);
        fast.sublight_engine_power = 50.0;
        assert_eq!(
            regrouped(vec![turbolasers(5.0), turbolasers(20.0), fast, turbolasers(10.0)]),
            [1, 0, 0, 1]
        );
        // Range outweighs speed: a fast short-ranged ship comes after slower
        // longer-ranged ones.
        let mut quick = turbolasers(5.0);
        quick.sublight_engine_power = 50.0;
        assert_eq!(
            regrouped(vec![turbolasers(20.0), quick, turbolasers(10.0), turbolasers(1.0)]),
            [0, 1, 0, 1]
        );
    }

    /// A carrier with room for `capacity` and one recovering group of
    /// `members`, its executor installed (FUN_005ca6d0 case 2).
    fn recovery(capacity: u32, members: usize) -> BattleSession {
        let mut carrier = ship(true, TacticalWeaponArc::default(), 0.0);
        carrier.fighter_capacity = capacity;
        let enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        let wings = (0..members)
            .map(|_| {
                let mut wing = fighter(true, TacticalWeaponArc::default());
                wing.order = TacticalOrder::Recover;
                wing
            })
            .collect::<Vec<_>>();
        let mut tactical = session(vec![carrier, enemy], wings);
        tactical.begin_original_fighter_recovery(&(0..members).collect::<Vec<_>>());
        tactical
    }

    // FUN_005b84e0 reserves (state 1) and posts 0x2c; its handler
    // FUN_005b1e10 gives state 2 with the carrier (+0x164).
    #[test]
    fn a_reserved_squadron_heads_for_its_carrier_through_event_0x2c() {
        use crate::tactical::{TacticalFighterRecoveryState as Recovery, TacticalTurnEvent};
        let mut tactical = recovery(1, 1);
        tactical.advance_recovery_executors();
        assert_eq!(tactical.fighters[0].recovery_state, Recovery::Reserved);
        let reserved = TacticalTurnEvent::FighterReserved { fighter: 0, carrier: 0 };
        assert!(tactical.turn_events.iter().any(|queued| queued.event == reserved));
        tactical.run_turn_event(reserved);
        assert_eq!(tactical.fighters[0].recovery_state, Recovery::Returning);
        assert_eq!(tactical.fighters[0].recovery_target, Some(0));
    }

    // +0xc counts the squadrons a carrier took in as well as those it
    // reserved (FUN_005b8630 keeps the reservation); FUN_005b8bd0 takes a
    // squadron in only while its carrier is in the fight.
    #[test]
    fn a_carrier_full_of_recovered_squadrons_takes_no_more() {
        use crate::tactical::{TacticalFighterRecoveryState as Recovery, TacticalTurnEvent};
        let mut tactical = recovery(1, 2);
        tactical.fighters[1].order = TacticalOrder::None;
        tactical.advance_recovery_executors();
        tactical.run_turn_event(TacticalTurnEvent::FighterReserved { fighter: 0, carrier: 0 });
        tactical.fighters[0].recovery_state = Recovery::Docking;
        let mut lost = tactical.clone();
        lost.ships[0].alive = false;
        lost.run_turn_event(TacticalTurnEvent::FighterDocked { fighter: 0, carrier: 0 });
        assert_eq!(lost.fighters[0].recovery_state, Recovery::Docking);
        tactical.run_turn_event(TacticalTurnEvent::FighterDocked { fighter: 0, carrier: 0 });
        assert_eq!(tactical.fighters[0].recovery_state, Recovery::Recovered);
        assert!(!tactical.fighters[0].alive);
        // The other group asks now: no room.
        tactical.fighters[1].order = TacticalOrder::Recover;
        tactical.begin_original_fighter_recovery(&[1]);
        tactical.advance_recovery_executors();
        assert_eq!(tactical.fighters[1].recovery_state, Recovery::AwaitingCarrier);
    }

    // FUN_005b8630: the player's group's last member docked plays its
    // "recovered" voice; a group without a colour (4) has none.
    #[test]
    fn the_last_docked_member_of_a_player_group_plays_its_voice() {
        use crate::tactical::{TacticalTurnEvent, TacticalVoiceCue, TacticalVoiceFaction};
        let voices = |group: u8| {
            let mut tactical = recovery(2, 2);
            for fighter in &mut tactical.fighters {
                fighter.fighter_group = group;
            }
            tactical.run_turn_event(TacticalTurnEvent::FighterDocked { fighter: 0, carrier: 0 });
            let first = tactical.take_pending_voice_cues();
            tactical.run_turn_event(TacticalTurnEvent::FighterDocked { fighter: 1, carrier: 0 });
            (first, tactical.take_pending_voice_cues())
        };
        assert_eq!(
            voices(3),
            (
                Vec::new(),
                vec![TacticalVoiceCue::fighter_recovery_complete(TacticalVoiceFaction::Alliance, 3)]
            )
        );
        assert_eq!(voices(4), (Vec::new(), Vec::new()));
    }

    // FUN_005cf980 with every ship full while the side withdraws (+0x84):
    // event 0x2f for each member still in state 0, which FUN_005b2720
    // destroys; once (+4).
    #[test]
    fn a_withdrawing_recovery_with_no_room_leaves_its_squadrons_behind() {
        use crate::tactical::TacticalTurnEvent;
        let mut tactical = recovery(0, 2);
        tactical.withdrawal_ms = Some(30_000.0);
        tactical.advance_recovery_executors();
        let behind = tactical
            .turn_events
            .iter()
            .filter(|queued| matches!(queued.event, TacticalTurnEvent::FighterLeftBehind(_)))
            .count();
        assert_eq!(behind, 2);
        assert_eq!(tactical.fighters[0].order, TacticalOrder::Recover);
        tactical.advance_recovery_executors();
        assert_eq!(
            tactical
                .turn_events
                .iter()
                .filter(|queued| matches!(queued.event, TacticalTurnEvent::FighterLeftBehind(_)))
                .count(),
            2
        );
        tactical.run_turn_event(TacticalTurnEvent::FighterLeftBehind(0));
        assert!(!tactical.fighters[0].alive);
        assert_eq!(tactical.fighters[0].squad_count, 0);
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
        // The shooter's draw opens its turn 1 (FUN_005b3a40).
        tactical.tactical_rng = stream_opening_weapon_turns(1);
        for _ in 0..4 {
            tactical.step();
        }
        // Decided in turn 1's updates: +0x87c is 3 there.
        let stamps = tactical
            .turn_events
            .iter()
            .filter(|queued| !matches!(queued.event, TacticalTurnEvent::TaskForceTarget { .. }))
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
        run.fire_unit_weapons(crate::tactical::TacticalUnitRef::Fighter(0), &mut shots);
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
        assert_eq!(events_besides_target_links(&tactical), 1);
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

    // FUN_005cc300 and FUN_005c6610 run for a force whose order-3 executor
    // is installed (FUN_005ca6d0 on event 0xa): a unit of the withdrawing
    // side, given the order and still in the battle, decides its jump; a
    // destroyed one, one not given the order, or one of the other side does
    // not, nor one whose executor's event has not yet run. The event
    // installs no executor for a unit already gone.
    #[test]
    fn only_a_withdrawing_unit_with_its_executor_decides_its_jump() {
        use crate::tactical::{TacticalTurnEvent, TacticalUnitRef};
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let forces = (0..4)
            .map(|force| {
                let mut unit = jumper(true, TacticalWeaponArc::default(), 0.0);
                unit.subsystem_condition.hyperdrive = 100;
                unit.task_force = force;
                unit.retreating = true;
                unit
            })
            .collect::<Vec<_>>();
        let mut other = jumper(false, TacticalWeaponArc::default(), 0.0);
        other.subsystem_condition.hyperdrive = 100;
        other.retreating = true;
        let mut ships = forces;
        ships.push(other);
        ships.push(enemy);
        let mut gone = fighter(true, TacticalWeaponArc::default());
        gone.has_hyperdrive = true;
        gone.retreating = true;
        let mut wing = gone.clone();
        wing.fighter_group = 1;
        let mut tactical = session(ships, vec![gone, wing]);
        let exit = tactical.withdrawal_exit_z(true);
        for ship in &mut tactical.ships[..4] {
            ship.source_position.z = exit;
        }
        tactical.ships[4].source_position.z = tactical.withdrawal_exit_z(false);
        for fighter in &mut tactical.fighters {
            fighter.source_position.z = exit;
        }
        install_withdrawal_executors(&mut tactical);
        tactical.ships[1].alive = false;
        tactical.ships[2].retreating = false;
        tactical.fighters[0].alive = false;
        // Given the order, but its executor's event has not yet run.
        tactical
            .withdrawal_executors
            .retain(|unit| *unit != TacticalUnitRef::Ship(3));
        tactical.withdrawal_ms = Some(crate::tactical::ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        for _ in 0..12 {
            tactical.step();
        }
        let decided =
            |t: &BattleSession, unit| t.turn_event_pending(&TacticalTurnEvent::JumpStart(unit));
        assert!(decided(&tactical, TacticalUnitRef::Ship(0)));
        assert!(decided(&tactical, TacticalUnitRef::Fighter(1)));
        for unit in [
            TacticalUnitRef::Ship(1),
            TacticalUnitRef::Ship(2),
            TacticalUnitRef::Ship(3),
            TacticalUnitRef::Ship(4),
            TacticalUnitRef::Fighter(0),
        ] {
            assert!(!decided(&tactical, unit), "{unit:?}");
        }

        // The event skips a destroyed unit and installs each once.
        let mut posted = session(
            vec![jumper(true, TacticalWeaponArc::default(), 0.0)],
            Vec::new(),
        );
        posted.ships.push(posted.ships[0].clone());
        posted.ships[1].alive = false;
        let units = vec![TacticalUnitRef::Ship(0), TacticalUnitRef::Ship(1)];
        posted.post_turn_event(0, TacticalTurnEvent::WithdrawalExecutor(units.clone()));
        posted.post_turn_event(0, TacticalTurnEvent::WithdrawalExecutor(units));
        posted.run_posted_order_executors();
        assert_eq!(posted.withdrawal_executors, [TacticalUnitRef::Ship(0)]);
    }

    // FUN_005aded0 gives a ship a force of its own outside the eight group
    // slots: the commander counts it as its own task force, and its voices
    // take the generic line before task force one (FUN_005a5cf0: 0x55,
    // 0x62 when 0x005a0300 finds no slot).
    #[test]
    fn a_ship_outside_the_group_slots_is_a_force_of_its_own() {
        use crate::tactical::{
            capital_group_offset, task_force_of_slot, task_force_slot,
            TACTICAL_UNGROUPED_TASK_FORCE as OWN,
        };
        assert_eq!(task_force_slot(OWN), 8);
        assert_eq!(task_force_slot(3), 3);
        assert_eq!(task_force_slot(9), 7);
        assert_eq!(task_force_of_slot(8), OWN);
        assert_eq!(task_force_of_slot(7), 7);
        assert_eq!(task_force_of_slot(0), 0);
        assert_eq!(capital_group_offset(2, OWN), 1);
        assert_eq!(capital_group_offset(2, 0), 2);
        assert_eq!(capital_group_offset(2, 6), 8);
        assert_eq!(capital_group_offset(2, 9), 9);

        let mut own = |index: usize| {
            let mut unit = ship(true, TacticalWeaponArc::default(), 0.0);
            unit.task_force = if index < 2 { OWN } else { 0 };
            unit
        };
        let ships = (0..4).map(&mut own).collect::<Vec<_>>();
        let battle = AbstractBattle::new(session(ships, Vec::new()));
        let groups = battle.groups(true);
        assert_eq!(groups.len(), 3);
        let own_forces = groups
            .iter()
            .filter(|(key, _)| key.id == OWN)
            .map(|(key, members)| (key.own_force, members.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            own_forces,
            [
                (1, vec![AbstractUnit::Capital(0)]),
                (2, vec![AbstractUnit::Capital(1)])
            ]
        );
        assert!(groups.iter().any(|(key, members)| key.id == 0
            && key.own_force == 0
            && members == &[AbstractUnit::Capital(2), AbstractUnit::Capital(3)]));
    }

    // 0x005eb8d0 (run in an emulator for indices 0..45): world-axis
    // directions by index % 22, normalized, of length 12.5 (DAT_006bd88c)
    // growing by a quarter every 22 (DAT_0066d8a8); cases 15 and 19 match.
    #[test]
    fn the_surround_slots_follow_the_original_table() {
        use crate::tactical::original_surround_slot as slot;
        let close = |v: crate::tactical::TacticalWorldVector, x: f32, y: f32, z: f32| {
            assert!(
                (v.x - x).abs() < 1e-4 && (v.y - y).abs() < 1e-4 && (v.z - z).abs() < 1e-4,
                "{v:?}"
            );
        };
        close(slot(0), 8.838_835, -8.838_835, 0.0);
        close(slot(1), 0.0, 0.0, 12.5);
        close(slot(2), 0.0, 0.0, -12.5);
        close(slot(3), 12.5, 0.0, 0.0);
        close(slot(7), 7.216_878, 7.216_878, 7.216_878);
        close(slot(21), -8.838_835, -8.838_835, 0.0);
        close(slot(23), 0.0, 0.0, 15.625);
        close(slot(45), 0.0, 0.0, 18.75);
        assert_eq!(slot(15), slot(19));
        // Every entry at length 1, as the emulator printed it for 0..21
        // (r = 0.707107, t = 0.57735).
        let (r, t) = (std::f32::consts::FRAC_1_SQRT_2, 0.577_350_3);
        let table = [
            (r, -r, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, -1.0),
            (1.0, 0.0, 0.0),
            (-1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, -1.0, 0.0),
            (t, t, t),
            (-t, -t, -t),
            (-t, t, t),
            (t, -t, -t),
            (t, -t, t),
            (-t, t, -t),
            (-t, -t, t),
            (t, t, -t),
            (0.0, -r, -r),
            (0.0, -r, r),
            (-r, r, 0.0),
            (r, r, 0.0),
            (0.0, -r, -r),
            (0.0, r, r),
            (-r, -r, 0.0),
        ];
        for (index, (x, y, z)) in table.into_iter().enumerate() {
            close(slot(index), x * 12.5, y * 12.5, z * 12.5);
        }
    }

    // 0x005cb1a0 builds a Surround force's mover (0x005eb860): each member
    // stands at the anchor less its slot (0x005eb040), the lead unit being
    // index 1. port: anchored on the lead unit.
    #[test]
    fn a_surround_force_places_its_members_by_their_slots() {
        use crate::tactical::{TacticalTactic, TacticalWorldPosition};
        let mut ships = (0..5)
            .map(|_| ship(true, TacticalWeaponArc::default(), 0.0))
            .collect::<Vec<_>>();
        ships[0].source_position = TacticalWorldPosition {
            x: 10.0,
            y: 2.0,
            z: -40.0,
        };
        let mut formations = [[TacticalTactic::StandOff; 8]; 2];
        formations[1][0] = TacticalTactic::Surround;
        BattleSession::refresh_original_task_force_geometry(&mut ships, &formations);
        let at = |x: f32, y: f32, z: f32| TacticalWorldPosition { x, y, z };
        assert_eq!(ships[0].source_position, at(10.0, 2.0, -40.0));
        assert_eq!(ships[1].source_position, at(10.0, 2.0, -15.0));
        assert_eq!(ships[2].source_position, at(-2.5, 2.0, -27.5));
        // Index 5 points up (0, 12.5, 0): the fifth member stands below.
        assert_eq!(ships[4].source_position, at(10.0, -10.5, -27.5));
        // The other side's slot 0 is unaffected.
        formations[1][0] = TacticalTactic::StandOff;
        formations[0][0] = TacticalTactic::Surround;
        BattleSession::refresh_original_task_force_geometry(&mut ships, &formations);
        assert_eq!(ships[1].source_position.y, 2.0);
        assert_ne!(ships[1].source_position, at(10.0, 2.0, -15.0));
    }

    fn duelists(own_range: f32, target_range: f32) -> BattleSession {
        use crate::tactical::{TacticalAttackTarget, TacticalWorldPosition};
        let mut own = ship(true, TacticalWeaponArc::new(2, 0, 0), 1.0);
        own.weapon_ranges.laser_cannon = own_range;
        own.hull_current = 1_000_000;
        own.hull_max = 1_000_000;
        own.order = TacticalOrder::AttackCapitalShips;
        own.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        own.source_position = TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: -50.0,
        };
        let mut enemy = ship(false, TacticalWeaponArc::new(2, 0, 0), 1.0);
        enemy.weapon_ranges.laser_cannon = target_range;
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        enemy.source_position.z = 50.0;
        enemy.task_force = 1;
        session(vec![own, enemy], Vec::new())
    }

    // FUN_005edf80: Stand Off's band is the worthiest band the force
    // reaches. The force's lasers (40) add, the target's (30) subtract
    // (FUN_005b49e0), so [30, 40) wins; min is at least 5 (DAT_0066daa8).
    // An unarmed force keeps max(target range x 1.2, 10) to 1e6; Surround
    // keeps 0 to 2.5 (0x005edf00).
    #[test]
    fn a_force_s_band_follows_its_tactic_and_the_weapons_on_both_sides() {
        use crate::tactical::{TacticalEngagementBand, TacticalTactic};
        let band = |t: &BattleSession| t.engagement_bands.clone();
        let mut tactical = duelists(40.0, 30.0);
        for _ in 0..4 {
            tactical.step();
        }
        assert_eq!(
            band(&tactical),
            [TacticalEngagementBand {
                leader: crate::tactical::TacticalUnitRef::Ship(0),
                min: 30.0,
                max: 40.0,
                step: 40.0 - 30.0
            }]
        );
        let mut close = duelists(4.0, 30.0);
        close.ships[1].weapon_arcs[0] = TacticalWeaponArc::default();
        for _ in 0..4 {
            close.step();
        }
        assert_eq!(close.engagement_bands[0].min, 5.0);
        assert_eq!(close.engagement_bands[0].max, 4.0);

        let mut unarmed = duelists(40.0, 30.0);
        unarmed.ships[0].weapon_arcs[0] = TacticalWeaponArc::default();
        unarmed.ships[0].weapon_ranges.laser_cannon = 0.0;
        for _ in 0..4 {
            unarmed.step();
        }
        assert_eq!(unarmed.engagement_bands[0].min, 36.0);
        assert_eq!(unarmed.engagement_bands[0].max, 1_000_000.0);

        let mut surround = duelists(40.0, 30.0);
        surround.task_force_formations[1][0] = TacticalTactic::Surround;
        for _ in 0..4 {
            surround.step();
        }
        assert_eq!(surround.engagement_bands[0].min, 0.0);
        assert_eq!(surround.engagement_bands[0].max, 2.5);
    }

    // FUN_005cc300: from turn 3 a force with a target closes on it while
    // r + 2 lies outside its band, then stops inside it; holding a
    // position, it does not move.
    #[test]
    fn a_force_closes_on_its_target_and_stops_in_its_band() {
        let distance = |t: &BattleSession| {
            let a = t.ships[0].source_position;
            let b = t.ships[1].source_position;
            (b.z - a.z).hypot(b.x - a.x).hypot(b.y - a.y)
        };
        let mut tactical = duelists(40.0, 30.0);
        // The computer's ship, given its own target, would close too; held
        // without engines it leaves the distances to the player's force.
        tactical.ships[1].subsystem_capacity.engines = false;
        tactical.ships[1].sublight_engine_power = 0.0;
        for _ in 0..12 {
            tactical.step();
        }
        assert_eq!(distance(&tactical), 100.0);
        step_until(&mut tactical, |t| distance(t) + 2.0 <= 40.0);
        let stopped = distance(&tactical);
        for _ in 0..40 {
            tactical.step();
        }
        assert!((distance(&tactical) - stopped).abs() < 1.0, "{stopped}");
        assert!(distance(&tactical) + 2.0 >= 30.0);

        // Holding, it stays (the AI's ship may close on it).
        let mut holding = duelists(40.0, 30.0);
        holding.ships[0].order = TacticalOrder::HoldPosition;
        // As its executor leaves it (FUN_005ca6d0, order 11): no heading.
        holding.ships[0].source_desired_forward = crate::tactical::TacticalWorldVector::ZERO;
        for _ in 0..60 {
            holding.step();
        }
        assert_eq!(holding.ships[0].source_position.z, -50.0);
    }

    // FUN_005cc300: a Stand Off force closer than its band's min backs off
    // along its side (min x -1.1, DAT_0066d00c), unless the target attacks
    // one of its members (0x005ca4b0); Surround steps 5 toward the target.
    #[test]
    fn a_stand_off_force_backs_off_unless_its_target_attacks_it() {
        use crate::tactical::{TacticalAttackTarget, TacticalTactic};
        let mut tactical = duelists(40.0, 30.0);
        tactical.ships[1].source_position.z = -40.0;
        for _ in 0..12 {
            tactical.step();
        }
        let waypoint = tactical.ships[0].source_waypoint.expect("backing off");
        assert!(waypoint.z < tactical.ships[0].source_position.z);

        let mut held = duelists(40.0, 30.0);
        held.ships[1].source_position.z = -40.0;
        held.ships[1].attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        for _ in 0..12 {
            held.step();
        }
        assert_eq!(held.ships[0].source_waypoint, None);

        let mut surround = duelists(40.0, 30.0);
        surround.task_force_formations[1][0] = TacticalTactic::Surround;
        for _ in 0..12 {
            surround.step();
        }
        // The goal is set 5 ahead before the step's move (slot +0x14 before
        // +0x8), so the moved ship is just under 5 short of it.
        let waypoint = surround.ships[0].source_waypoint.expect("closing");
        let position = surround.ships[0].source_position;
        let ahead = waypoint.z - position.z;
        assert!(ahead > 4.0 && ahead <= 5.0, "{waypoint:?} {position:?}");
    }

    // FUN_005b49e0: each family of the strongest arc, if it has weapons,
    // is a band to its range worth count x slot +0x60 x strength; the
    // target's count against the force.
    #[test]
    fn a_unit_s_weapons_are_bands_worth_their_firepower() {
        use crate::tactical::TacticalWeaponRanges;
        let ranges = TacticalWeaponRanges {
            laser_cannon: 30.0,
            ion_cannon: 10.0,
            turbolaser: 20.0,
        };
        let arcs = [
            TacticalWeaponArc::new(2, 3, 4),
            TacticalWeaponArc::default(),
        ];
        let mut bands = Vec::new();
        BattleSession::weapon_bands(&arcs, ranges, 2.0, false, -1.0, &mut bands);
        let s = family_strengths(false);
        let expected = [
            (10.0, -(3.0 * 2.0 * s[1])),
            (20.0, -(4.0 * 2.0 * s[2])),
            (30.0, -(2.0 * 2.0 * s[0])),
        ];
        assert_eq!(bands.len(), 3);
        for ((range, worth), (want_range, want_worth)) in bands.iter().zip(expected) {
            assert_eq!(*range, want_range);
            assert!((worth - want_worth).abs() < 1e-6, "{worth} {want_worth}");
        }
        // A family without weapons adds no band; nor does an empty unit.
        let mut lean = Vec::new();
        BattleSession::weapon_bands(
            &[TacticalWeaponArc::new(2, 0, 0)],
            ranges,
            2.0,
            true,
            1.0,
            &mut lean,
        );
        assert_eq!(lean.len(), 1);
        assert!((lean[0].1 - 2.0 * 2.0 * family_strengths(true)[0]).abs() < 1e-6);
        let mut none = Vec::new();
        BattleSession::weapon_bands(
            &[TacticalWeaponArc::default()],
            ranges,
            2.0,
            false,
            1.0,
            &mut none,
        );
        assert!(none.is_empty());
    }

    // FUN_005edf80's choice among split bands: the worthiest band within
    // the force's reach, the later of equals (fcomp >=), a zero-range
    // weapon adding no band (FUN_005b7e50 drops [0, 0)).
    #[test]
    fn stand_off_takes_the_worthiest_band_within_reach() {
        use crate::tactical::{TacticalAttackTarget, TacticalUnitRef::Ship};
        let pair = |(min, max, _): (f32, f32, f32)| (min, max);
        // Turbolasers to 20 outweigh lasers to 40; the target's lasers
        // reach 30. [0, 20) has both families less the target's.
        let mut tactical = duelists(40.0, 30.0);
        tactical.ships[0].weapon_arcs[0] = TacticalWeaponArc::new(2, 0, 40);
        tactical.ships[0].weapon_ranges.turbolaser = 20.0;
        let target = TacticalAttackTarget::CapitalShip(1);
        assert_eq!(
            pair(tactical.stand_off_band(&[Ship(0)], target)),
            (5.0, 20.0)
        );
        // Beyond the force's own reach a band is not taken: the target's
        // turbolasers to 60 make [40, 60) its least bad band.
        let mut outranged = duelists(40.0, 30.0);
        outranged.ships[1].weapon_arcs[0] = TacticalWeaponArc::new(2, 0, 1);
        outranged.ships[1].weapon_ranges.turbolaser = 60.0;
        assert_eq!(
            pair(outranged.stand_off_band(&[Ship(0)], target)),
            (30.0, 40.0)
        );
        // Equal bands: a second ship's ions to 20 with no recharge split
        // [0, 40) into two of equal worth; the later wins.
        let mut even = duelists(40.0, 30.0);
        even.ships[1].weapon_arcs[0] = TacticalWeaponArc::default();
        let mut quiet = ship(true, TacticalWeaponArc::new(0, 1, 0), 0.0);
        quiet.weapon_ranges.ion_cannon = 20.0;
        even.ships.push(quiet);
        assert_eq!(
            pair(even.stand_off_band(&[Ship(0), Ship(2)], target)),
            (20.0, 40.0)
        );
        // A zero-range weapon adds no band: [0, 20) of the turbolasers stays.
        let mut zero = duelists(0.0, 30.0);
        zero.ships[1].weapon_arcs[0] = TacticalWeaponArc::default();
        zero.ships[0].weapon_arcs[0] = TacticalWeaponArc::new(5, 0, 1);
        zero.ships[0].weapon_ranges.turbolaser = 20.0;
        assert_eq!(pair(zero.stand_off_band(&[Ship(0)], target)), (5.0, 20.0));
    }

    // The forces in the fight: each slot's ships of a side, and each ship
    // of a force of its own alone; destroyed, jumping and withdrawing ships
    // are not in one. A target is a place only while it is in the fight.
    #[test]
    fn the_forces_and_their_targets_are_those_in_the_fight() {
        use crate::tactical::{TacticalAttackTarget, TACTICAL_UNGROUPED_TASK_FORCE as OWN};
        let unit = |alliance: bool, task_force: u8| {
            let mut unit = ship(alliance, TacticalWeaponArc::default(), 0.0);
            unit.task_force = task_force;
            unit
        };
        let mut ships = vec![
            unit(true, 0),
            unit(true, 0),
            unit(false, 0),
            unit(true, OWN),
            unit(true, OWN),
            unit(true, 1),
            unit(true, 2),
            unit(true, 3),
        ];
        ships[5].alive = false;
        ships[6].retreat_progress = 1.0;
        ships[7].retreating = true;
        let mut wing = fighter(false, TacticalWeaponArc::default());
        wing.squad_count = 0;
        let mut jumping = fighter(false, TacticalWeaponArc::default());
        jumping.retreat_progress = 1.0;
        let live = fighter(false, TacticalWeaponArc::default());
        let tactical = session(ships, vec![wing, jumping, live]);
        let mut forces = tactical.capital_forces();
        for force in &mut forces {
            force.sort_unstable();
        }
        assert_eq!(forces, [vec![0, 1], vec![2], vec![3], vec![4]]);
        let place = |target| tactical.attack_target_position(target);
        assert!(place(TacticalAttackTarget::CapitalShip(0)).is_some());
        assert!(place(TacticalAttackTarget::CapitalShip(5)).is_none());
        assert!(place(TacticalAttackTarget::CapitalShip(6)).is_none());
        assert!(place(TacticalAttackTarget::FighterGroup(0)).is_none());
        assert!(place(TacticalAttackTarget::FighterGroup(1)).is_none());
        assert!(place(TacticalAttackTarget::FighterGroup(2)).is_some());
    }

    // FUN_005cc300's step, exactly: the goal is the anchor plus the unit
    // direction times the step (Stand Off: min + k (max - min), k = 0.5,
    // 0.75, 0.9 by size); r + 2 inside the band stops it; a zero direction
    // takes the side's vector times -5; a Stand Off force at exactly its
    // min, outside a narrow band, still closes.
    #[test]
    fn the_approach_step_aims_exactly() {
        use crate::tactical::{TacticalEngagementBand, TacticalWorldPosition};
        let at = |x: f32, y: f32, z: f32| TacticalWorldPosition { x, y, z };
        let prepared = |members: usize, band: (f32, f32), target: TacticalWorldPosition| {
            let mut tactical = duelists(40.0, 30.0);
            tactical.ships[0].source_position = at(0.0, 0.0, 0.0);
            tactical.ships[1].source_position = target;
            for _ in 1..members {
                let mut extra = ship(true, TacticalWeaponArc::default(), 0.0);
                extra.sublight_engine_power = 0.0;
                tactical.ships.push(extra);
            }
            tactical.engagement_bands = vec![TacticalEngagementBand {
                leader: crate::tactical::TacticalUnitRef::Ship(0),
                min: band.0,
                max: band.1,
                step: 5.0,
            }];
            tactical.advance_engagement_approach();
            tactical.ships[0].source_waypoint
        };
        let close = |goal: Option<TacticalWorldPosition>, want: TacticalWorldPosition| {
            let goal = goal.expect("a goal");
            assert!(
                (goal.x - want.x).abs() < 1e-4
                    && (goal.y - want.y).abs() < 1e-4
                    && (goal.z - want.z).abs() < 1e-4,
                "{goal:?} {want:?}"
            );
        };
        // 3-4-0 direction (0.6, 0.8, 0), band (30, 40): steps 35, 37.5, 39.
        close(
            prepared(1, (30.0, 40.0), at(60.0, 80.0, 0.0)),
            at(21.0, 28.0, 0.0),
        );
        close(
            prepared(3, (30.0, 40.0), at(60.0, 80.0, 0.0)),
            at(22.5, 30.0, 0.0),
        );
        close(
            prepared(6, (30.0, 40.0), at(60.0, 80.0, 0.0)),
            at(23.4, 31.2, 0.0),
        );
        // A step is at least 2.
        close(
            prepared(1, (0.0, 1.0), at(0.0, 0.0, 50.0)),
            at(0.0, 0.0, 2.0),
        );
        // r = 29, r + 2 = 31 inside (30, 40): it stops.
        assert_eq!(prepared(1, (30.0, 40.0), at(0.0, 0.0, 29.0)), None);
        // r = 2.5, r + 2 = 4.5 below (5, 40): still out, so it backs off.
        let goal = prepared(1, (5.0, 40.0), at(0.0, 0.0, 2.5)).expect("backing off");
        assert!(goal.z < 0.0, "{goal:?}");
        // On the target: the side's vector times -5, so back toward z < 0.
        close(
            prepared(1, (0.0, 1.0), at(0.0, 0.0, 0.0)),
            at(0.0, 0.0, -2.0),
        );
        // r = 5 = min of the band (5, 4): out of it, and not closer than
        // min, so it closes.
        close(
            prepared(1, (5.0, 4.0), at(0.0, 0.0, 5.0)),
            at(0.0, 0.0, 4.5),
        );
    }

    // The Empire's force backs off toward its own side (+z): the side's
    // vector times min x -1.1.
    #[test]
    fn an_imperial_stand_off_force_backs_off_toward_its_side() {
        use crate::tactical::{TacticalAttackTarget, TacticalEngagementBand};
        let mut tactical = duelists(40.0, 30.0);
        tactical.ships.swap(0, 1);
        tactical.ships[0].attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        tactical.ships[0].order = TacticalOrder::AttackCapitalShips;
        tactical.ships[1].attack_target = None;
        tactical.ships[0].source_position.z = 0.0;
        tactical.ships[1].source_position.z = -10.0;
        tactical.engagement_bands = vec![TacticalEngagementBand {
            leader: crate::tactical::TacticalUnitRef::Ship(0),
            min: 30.0,
            max: 40.0,
            step: 5.0,
        }];
        tactical.advance_engagement_approach();
        let goal = tactical.ships[0].source_waypoint.expect("backing off");
        assert!(goal.z > 0.0, "{goal:?}");
    }

    fn dogfight() -> BattleSession {
        use crate::tactical::{TacticalAttackTarget, TacticalWorldPosition};
        let mut target = ship(false, TacticalWeaponArc::new(2, 0, 0), 1.0);
        target.weapon_ranges.laser_cannon = 30.0;
        target.hull_current = 1_000_000;
        target.hull_max = 1_000_000;
        target.source_position.z = 50.0;
        let mut wing = fighter(true, TacticalWeaponArc::new(5, 0, 0));
        wing.weapon_ranges.laser_cannon = 12.0;
        // 10 plus the Alliance's commander term 1 (0x5ba190): 11 a second.
        wing.sublight_engine_power = 10.0;
        wing.order = TacticalOrder::AttackCapitalShips;
        wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        wing.source_position = TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: -50.0,
        };
        session(vec![target], vec![wing])
    }

    // FUN_005c6610: a route (+0x74) comes before the target; the mover
    // flies to the first point (slot +0xc) at its speed.
    #[test]
    fn a_fighter_route_comes_before_its_target() {
        use crate::tactical::TacticalWorldPosition;
        let mut tactical = dogfight();
        let point = TacticalWorldPosition { x: 40.0, y: 0.0, z: -50.0 };
        tactical.fighters[0].navigation_route = vec![point];
        tactical.advance_fighter_approach();
        let anchor = tactical.fighter_mover_of(0).expect("a mover").anchor;
        assert!(anchor.x > 0.0, "{anchor:?}");
        assert!((anchor.z + 50.0).abs() < 1e-5, "{anchor:?}");
        assert_eq!(tactical.fighters[0].navigation_route, vec![point]);
    }

    // FUN_005c6610: within 1.0 (0x3f800000) the point becomes the anchor
    // (+0x3c) and leaves the route (FUN_005c6200); the player's group hears
    // its maneuver-complete line (0x37 plus the slot). The target follows.
    #[test]
    fn a_fighter_force_takes_its_route_point_and_then_its_target() {
        use crate::tactical::TacticalWorldPosition;
        let mut tactical = dogfight();
        tactical.sync_fighter_movers();
        let start = tactical.fighter_mover_of(0).expect("a mover").anchor;
        let near = TacticalWorldPosition { x: start.x + 0.9, ..start };
        let far = TacticalWorldPosition { x: start.x + 30.0, ..start };
        tactical.fighters[0].navigation_route = vec![near, far];
        let cues = tactical.pending_voice_cues.len();
        tactical.advance_fighter_approach();
        assert_eq!(tactical.fighter_mover_of(0).expect("a mover").anchor, near);
        assert_eq!(tactical.fighters[0].navigation_route, vec![far]);
        // The route goes on, so no line yet.
        assert_eq!(tactical.pending_voice_cues.len(), cues);
        tactical.fighters[0].navigation_route = vec![near];
        tactical.advance_fighter_approach();
        assert!(tactical.fighters[0].navigation_route.is_empty());
        assert_eq!(tactical.pending_voice_cues.len(), cues + 1);
    }

    // FUN_005c6610: a fighter force heads one +0xc step toward its target
    // each step from turn 3, at its speed. Its Stand Off +0xc against a
    // capital that outranges it is a third into its best band
    // (FUN_005edf80: [0, 12) gives 4, DAT_0066dab8).
    #[test]
    fn a_fighter_force_flies_at_its_target() {
        use crate::tactical::{TacticalAttackTarget, TacticalUnitRef};
        let tactical = dogfight();
        let (min, max, step) = tactical.stand_off_band(
            &[TacticalUnitRef::Fighter(0)],
            TacticalAttackTarget::CapitalShip(0),
        );
        assert_eq!((min, max), (5.0, 12.0));
        assert!((step - 4.0).abs() < 1e-6, "{step}");
        // Nothing moves before turn 3, step 12 (0x005bef1d).
        let mut tactical = dogfight();
        for _ in 0..11 {
            tactical.step();
        }
        let z = tactical.fighters[0].source_position.z;
        assert_eq!(z, -50.0);
        tactical.step();
        assert_eq!(tactical.fighters[0].source_position.z, -50.0 + 11.0 * 0.25);
    }

    // FUN_005c6610 within the move's radius (2.0): the force circles its
    // target; the angle grows by the step's 250 ms x pi x 0.0002 and the
    // Empire's is negated each time; the radius is the target's size x
    // 2.4, at least 1.5, with half of it above the target.
    #[test]
    fn a_fighter_force_in_reach_circles_its_target() {
        use crate::tactical::{TacticalEngagementBand, TacticalUnitRef};
        let orbit = |alliance: bool, steps: usize| {
            let mut tactical = dogfight();
            tactical.fighters[0].identity.is_alliance = alliance;
            tactical.engagement_bands = vec![TacticalEngagementBand {
                leader: TacticalUnitRef::Fighter(0),
                min: 0.0,
                max: 0.0,
                step: 1.0,
            }];
            for _ in 0..steps {
                tactical.advance_fighter_approach();
            }
            // The circling point is the force's anchor.
            (
                tactical.fighter_orbit_angles[0],
                tactical.fighter_mover_of(0).expect("a mover").anchor,
            )
        };
        let delta = 250.0 * std::f32::consts::PI * 0.0002;
        let (angle, at) = orbit(true, 1);
        assert!((angle - delta).abs() < 1e-6);
        let (sin, cos) = delta.sin_cos();
        assert!((at.x - (cos * 1.5 - sin * 1.5)).abs() < 1e-5, "{at:?}");
        assert!((at.y - 0.75).abs() < 1e-6);
        assert!(
            (at.z - (50.0 + cos * 1.5 + sin * 1.5)).abs() < 1e-4,
            "{at:?}"
        );
        assert!((orbit(true, 2).0 - 2.0 * delta).abs() < 1e-6);
        assert!((orbit(false, 1).0 + delta).abs() < 1e-6);
        assert!(orbit(false, 2).0.abs() < 1e-6);
        // It wraps at a full turn: 40 steps of pi / 20 come back to 0.
        assert!(orbit(true, 40).0.abs() < 1e-3);
        // A target of size 2 gives a radius of 4.8.
        let mut tactical = dogfight();
        tactical.ships[0].source_collision_envelope =
            Some(crate::tactical::OriginalTacticalCollisionEnvelope {
                vertical_radius: 1.0,
                planar_diameter: 2.0,
            });
        tactical.engagement_bands = vec![TacticalEngagementBand {
            leader: TacticalUnitRef::Fighter(0),
            min: 0.0,
            max: 0.0,
            step: 1.0,
        }];
        tactical.advance_fighter_approach();
        assert!((tactical.fighter_mover_of(0).expect("a mover").anchor.y - 2.4).abs() < 1e-5);
    }

    // FUN_005edf80's +0xc a third into a band that starts above 0: the
    // target's heavy short lasers (to 4) spoil [0, 4), its turbolasers
    // outrange the force, so [4, 12) is best and the step is 4 + 8/3.
    #[test]
    fn stand_off_steps_a_third_into_its_band() {
        use crate::tactical::{TacticalAttackTarget, TacticalUnitRef};
        let mut tactical = dogfight();
        tactical.ships[0].weapon_arcs[0] = TacticalWeaponArc::new(40, 0, 1);
        tactical.ships[0].weapon_ranges.laser_cannon = 4.0;
        tactical.ships[0].weapon_ranges.turbolaser = 30.0;
        let (min, max, step) = tactical.stand_off_band(
            &[TacticalUnitRef::Fighter(0)],
            TacticalAttackTarget::CapitalShip(0),
        );
        assert_eq!((min, max), (5.0, 12.0));
        assert!((step - (4.0 + 8.0 / 3.0)).abs() < 1e-5, "{step}");
    }

    // FUN_005c6610's flight on all three axes, a step of exactly 2 already
    // within the move's radius (it circles), and an Imperial force on its
    // target taking its side's vector (-z).
    #[test]
    fn a_fighter_force_flies_in_three_dimensions() {
        use crate::tactical::{TacticalEngagementBand, TacticalUnitRef, TacticalWorldPosition};
        let at = |x: f32, y: f32, z: f32| TacticalWorldPosition { x, y, z };
        let fly = |target: TacticalWorldPosition, step: f32, alliance: bool| {
            let mut tactical = dogfight();
            tactical.ships[0].source_position = target;
            tactical.fighters[0].source_position = at(0.0, 0.0, 0.0);
            tactical.fighters[0].identity.is_alliance = alliance;
            tactical.engagement_bands = vec![TacticalEngagementBand {
                leader: TacticalUnitRef::Fighter(0),
                min: 0.0,
                max: 0.0,
                step,
            }];
            tactical.advance_fighter_approach();
            // The order step moves the force's anchor (FUN_005c72b0's +0x3c).
            tactical.fighter_mover_of(0).expect("a mover").anchor
        };
        // Toward (12, 9, 20) / 25: a 2.75 step (11 per second for 250 ms).
        let moved = fly(at(12.0, 9.0, 20.0), 4.0, true);
        let want = [12.0 / 25.0 * 2.75, 9.0 / 25.0 * 2.75, 20.0 / 25.0 * 2.75];
        assert!((moved.x - want[0]).abs() < 1e-5, "{moved:?}");
        assert!((moved.y - want[1]).abs() < 1e-5, "{moved:?}");
        assert!((moved.z - want[2]).abs() < 1e-5, "{moved:?}");
        // A step of 2 is within the radius: it circles at 1.5.
        let circling = fly(at(0.0, 0.0, 30.0), 2.0, true);
        assert!((circling.y - 0.75).abs() < 1e-6, "{circling:?}");
        // On its target an Imperial force heads -z, away by 2.75.
        let imperial = fly(at(0.0, 0.0, 0.0), 4.0, false);
        assert!((imperial.z + 2.75).abs() < 1e-5, "{imperial:?}");
    }

    fn mobility(tactical: &BattleSession, index: usize) -> f32 {
        crate::tactical::original_tactical_mobility(&tactical.ships, &tactical.field_effects, index)
            .map_or(0.0, |mobility| mobility.effective_engine_power)
    }

    // FUN_005ea740: the mover's speed +0x124 is the slowest positive member
    // speed; 0x005eb1a0 moves the anchor at 0.95 of it (DAT_0066d84c), and
    // FUN_005b2f30 caps each member at it.
    #[test]
    fn a_formation_moves_at_its_slowest_member_less_five_percent() {
        let mut fast = ship(true, TacticalWeaponArc::default(), 0.0);
        fast.sublight_engine_power = 4.0;
        let mut slow = ship(true, TacticalWeaponArc::default(), 0.0);
        slow.sublight_engine_power = 2.0;
        slow.source_position.x = 30.0;
        let goal = crate::tactical::TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 100.0,
        };
        fast.source_waypoint = Some(goal);
        slow.source_waypoint = Some(goal);
        let mut tactical = session(vec![fast, slow], Vec::new());
        let slowest = mobility(&tactical, 1);
        assert!(slowest > 0.0 && slowest < mobility(&tactical, 0));
        // A frame of 1 s: within 1.0 of its place (DAT_0066d850) a member
        // counts as there, so a shorter step would leave the lead standing.
        tactical.advance_original_tactical_movement(1000.0);
        let mover = tactical.force_movers[0];
        assert_eq!(mover.speed, slowest);
        assert!((mover.anchor.z - 0.95 * slowest).abs() < 1e-5, "{mover:?}");
        // The lead (whichever started on the anchor) snaps onto it: it
        // moved no faster than the force.
        let lead = (0..2)
            .find(|&index| tactical.ships[index].source_position.x == mover.anchor.x)
            .expect("a lead on the anchor");
        assert!((tactical.ships[lead].source_position.z - mover.anchor.z).abs() < 1e-5);
    }

    // 0x005eb1a0 (Stand Off): a heading behind flips the forward at once
    // and leaves the slots; within pi / 12 (DAT_006bd884) it does not turn,
    // and a still formation that did not turn takes the velocity (0, 1, 0).
    #[test]
    fn a_stand_off_formation_flips_without_turning_its_slots() {
        use crate::tactical::{TacticalForceKey, TacticalForceMover, TacticalWorldPosition};
        let mut mover = TacticalForceMover {
            force: TacticalForceKey::Ship(0),
            tactic: TacticalTactic::StandOff,
            anchor: TacticalWorldPosition::ORIGIN,
            forward: TacticalWorldVector::SOURCE_FORWARD,
            heading: TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            lateral: TacticalWorldVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            velocity: TacticalWorldVector::ZERO,
            last_delta: TacticalWorldVector::ZERO,
            speed: 4.0,
            turn_rate: std::f32::consts::PI,
            still_turns: 0,
            slot_turn: 0.0,
            last_turn_anchor: TacticalWorldPosition::ORIGIN,
        };
        let behind = TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: -50.0,
        };
        assert!(!BattleSession::step_force_anchor(&mut mover, behind, 1.0, 250.0));
        assert_eq!(mover.forward.z, -1.0);
        assert_eq!((mover.lateral.x, mover.lateral.z), (1.0, 0.0));
        assert!((mover.anchor.z + 0.95).abs() < 1e-6);
        // 10 degrees off, within reach: no turn, velocity (0, 1, 0).
        let angle = 10_f32.to_radians();
        mover.heading = TacticalWorldVector {
            x: angle.sin(),
            y: 0.0,
            z: -angle.cos(),
        };
        let near = mover.anchor;
        assert!(BattleSession::step_force_anchor(&mut mover, near, 1.0, 250.0));
        assert_eq!(mover.forward.z, -1.0);
        assert_eq!(
            (mover.velocity.x, mover.velocity.y, mover.velocity.z),
            (0.0, 1.0, 0.0)
        );
        // Beyond pi / 12 it turns at its rate, and the slots turn with it.
        let point = TacticalWorldPosition {
            x: 50.0,
            y: 0.0,
            z: mover.anchor.z,
        };
        mover.heading = TacticalWorldVector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        BattleSession::step_force_anchor(&mut mover, point, 1.0, 250.0);
        let turned = std::f32::consts::PI * 0.25;
        assert!((source_angle(mover.forward) - (std::f32::consts::PI - turned)).abs() < 1e-4);
        assert!((mover.lateral.x.hypot(mover.lateral.z) - 1.0).abs() < 1e-5);
        assert!(mover.lateral.z.abs() > 0.5, "{mover:?}");
    }

    fn source_angle(vector: TacticalWorldVector) -> f32 {
        vector.x.atan2(vector.z).abs()
    }

    // 0x005eb540 (Surround): it turns toward its own level movement with no
    // dead band (FUN_005eae90), and its slots stay on the world axes
    // (0x005ead70 turns only the forward); its members stand at the anchor
    // less their slot, the lead at slot 1 (0x005eb040, 0x005eb8d0).
    #[test]
    fn a_surround_formation_keeps_its_slots_on_the_world_axes() {
        use crate::tactical::{
            original_surround_slot, TacticalForceKey, TacticalForceMover, TacticalWorldPosition,
        };
        let tactical = session(vec![ship(true, TacticalWeaponArc::default(), 0.0)], Vec::new());
        let mut mover = TacticalForceMover {
            force: TacticalForceKey::Ship(0),
            tactic: TacticalTactic::Surround,
            anchor: TacticalWorldPosition::ORIGIN,
            forward: TacticalWorldVector::SOURCE_FORWARD,
            heading: TacticalWorldVector::SOURCE_FORWARD,
            lateral: TacticalWorldVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            velocity: TacticalWorldVector::ZERO,
            last_delta: TacticalWorldVector::ZERO,
            speed: 4.0,
            turn_rate: 0.4,
            still_turns: 0,
            slot_turn: 0.0,
            last_turn_anchor: TacticalWorldPosition::ORIGIN,
        };
        let before = tactical.force_member_place(&mover, &[0], 0);
        let point = TacticalWorldPosition {
            x: 50.0,
            y: 5.0,
            z: 0.0,
        };
        BattleSession::step_force_anchor(&mut mover, point, 2.0, 250.0);
        // 0.1 rad, below Stand Off's pi / 12; a heading square to the
        // forward does not flip it.
        assert!((mover.forward.x - 0.1_f32.sin()).abs() < 1e-5, "{mover:?}");
        assert!((mover.forward.z - 0.1_f32.cos()).abs() < 1e-5, "{mover:?}");
        assert_eq!((mover.heading.x, mover.heading.y), (1.0, 0.0));
        let after = tactical.force_member_place(&mover, &[0], 0);
        let slot = original_surround_slot(1);
        assert_eq!(after.x - mover.anchor.x, -slot.x);
        assert_eq!(after.z - mover.anchor.z, -slot.z);
        assert_eq!(before.z, -slot.z);
    }

    // FUN_005b4d40: a member at its place in a still formation (velocity
    // zero, +0x134 at least 5) turns its strongest arc against capital
    // ships (slot +0x84) to its target: the bearing turned by arc x pi / 2.
    #[test]
    fn a_still_formation_turns_its_strongest_arc_to_the_target() {
        let mut gunner = ship(true, TacticalWeaponArc::default(), 0.0);
        gunner.weapon_arcs[1] = TacticalWeaponArc::new(0, 4, 0);
        gunner.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(1));
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        target.source_position.x = 50.0;
        target.task_force = 1;
        let mut tactical = session(vec![gunner, target], Vec::new());
        tactical.advance_original_tactical_movement(250.0);
        // Not yet still for 5 turns: it keeps its forward.
        assert_eq!(tactical.ships[0].source_desired_forward.z, 1.0);
        tactical.force_movers[0].still_turns = 5;
        tactical.advance_original_tactical_movement(250.0);
        let wanted = tactical.ships[0].source_desired_forward;
        // Bearing (1, 0, 0) turned by pi / 2: (0, 0, 1).
        assert!(wanted.x.abs() < 1e-5 && (wanted.z - 1.0).abs() < 1e-5, "{wanted:?}");
        tactical.ships[0].weapon_arcs[1] = TacticalWeaponArc::default();
        tactical.ships[0].weapon_arcs[0] = TacticalWeaponArc::new(0, 4, 0);
        tactical.advance_original_tactical_movement(250.0);
        let wanted = tactical.ships[0].source_desired_forward;
        assert!((wanted.x - 1.0).abs() < 1e-5, "{wanted:?}");
    }

    // FUN_005cb1a0 rebuilds a force's mover on its tactic's event 0x3d: the
    // new Surround mover starts on the lead unit, whose place is the anchor
    // less slot 1, so the lead moves to it.
    #[test]
    fn a_new_tactic_rebuilds_the_mover_on_the_lead_unit() {
        let lead = ship(true, TacticalWeaponArc::default(), 0.0);
        let mut tactical = session(vec![lead], Vec::new());
        tactical.advance_original_tactical_movement(250.0);
        assert_eq!(tactical.force_movers[0].tactic, TacticalTactic::StandOff);
        tactical.ships[0].source_position.x = 7.0;
        tactical.task_force_formations[1][0] = TacticalTactic::Surround;
        tactical.advance_original_tactical_movement(250.0);
        let mover = tactical.force_movers[0];
        assert_eq!(mover.tactic, TacticalTactic::Surround);
        assert_eq!(mover.anchor.x, 7.0);
        let slot = crate::tactical::original_surround_slot(1);
        let place_z = mover.anchor.z - slot.z;
        let start = (0.0 - place_z).abs();
        for _ in 0..40 {
            tactical.advance_original_tactical_movement(250.0);
        }
        let moved = (tactical.ships[0].source_position.z - place_z).abs();
        assert!(moved < start, "{:?} {place_z}", tactical.ships[0].source_position);
    }

    fn test_mover(tactic: TacticalTactic) -> crate::tactical::TacticalForceMover {
        use crate::tactical::{TacticalForceKey, TacticalForceMover, TacticalWorldPosition};
        TacticalForceMover {
            force: TacticalForceKey::Ship(0),
            tactic,
            anchor: TacticalWorldPosition::ORIGIN,
            forward: TacticalWorldVector::SOURCE_FORWARD,
            heading: TacticalWorldVector::SOURCE_FORWARD,
            lateral: TacticalWorldVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            velocity: TacticalWorldVector::ZERO,
            last_delta: TacticalWorldVector::ZERO,
            speed: 4.0,
            turn_rate: 0.0,
            still_turns: 0,
            slot_turn: 0.0,
            last_turn_anchor: TacticalWorldPosition::ORIGIN,
        }
    }

    fn vector(x: f32, y: f32, z: f32) -> TacticalWorldVector {
        TacticalWorldVector { x, y, z }
    }

    fn near(left: TacticalWorldVector, right: TacticalWorldVector) -> bool {
        (left.x - right.x).abs() < 1e-5
            && (left.y - right.y).abs() < 1e-5
            && (left.z - right.z).abs() < 1e-5
    }

    // FUN_005cd460 is one turn for the mover's forward and a unit's: the
    // vector turn gives the ship turn's result, and returns the signed angle.
    #[test]
    fn the_formation_turn_is_the_unit_turn() {
        use crate::tactical::{
            original_capital_turn_rate, turn_original_tactical_ship, turn_original_vector,
        };
        let cases = [
            (vector(0.6, 0.2, 0.77), vector(-0.3, 0.1, 0.9)),
            (vector(0.6, 0.2, 0.77), vector(0.9, -0.4, -0.1)),
            (vector(-0.5, 0.5, -0.7), vector(0.2, 0.6, -0.8)),
            (vector(0.0, 0.0, 1.0), vector(1.0, 0.0, 0.0)),
            // Cases whose turn direction hangs on the y term.
            (vector(0.1, 0.9, 0.42), vector(0.3, 0.9, -0.3)),
            (vector(0.1, -0.2, 0.42), vector(0.3, 0.9, -0.3)),
        ];
        for (from, to) in cases {
            for (maneuverability, milliseconds) in [(0, 250.0), (4, 250.0), (9, 40.0), (20, 5000.0)] {
                let mut unit = ship(true, TacticalWeaponArc::default(), 0.0);
                unit.maneuverability = maneuverability;
                unit.source_forward = from.normalized();
                unit.source_desired_forward = to;
                turn_original_tactical_ship(&mut unit, milliseconds);
                let mut forward = from.normalized();
                let applied = turn_original_vector(
                    &mut forward,
                    to,
                    original_capital_turn_rate(maneuverability),
                    milliseconds,
                );
                assert!(near(forward, unit.source_forward), "{from:?} {to:?} {forward:?}");
                let angle = crate::tactical::source_direction_angle(from, to);
                let step = original_capital_turn_rate(maneuverability) * milliseconds * 0.001;
                assert!((applied.abs() - step.min(angle)).abs() < 1e-5, "{applied}");
            }
        }
        let mut forward = TacticalWorldVector::SOURCE_FORWARD;
        assert!(turn_original_vector(&mut forward, vector(1.0, 0.0, 0.0), 1.0, 250.0) < 0.0);
        assert!(turn_original_vector(&mut forward, vector(-1.0, 0.0, 0.0), 1.0, 250.0) > 0.0);
    }

    // Group slots 0..7 (+0x164) are forces of a side; any other task force
    // is a force of its own (FUN_005aded0).
    #[test]
    fn group_slots_and_lone_ships_key_their_forces() {
        use crate::tactical::TacticalForceKey;
        let mut ships = (0..4)
            .map(|_| ship(true, TacticalWeaponArc::default(), 0.0))
            .collect::<Vec<_>>();
        ships[0].task_force = 3;
        ships[1].task_force = 7;
        ships[2].task_force = 8;
        ships[3].task_force = 255;
        let tactical = session(ships, Vec::new());
        assert_eq!(
            tactical.force_key(0),
            TacticalForceKey::Slot {
                attacker: true,
                slot: 3
            }
        );
        assert_eq!(
            tactical.force_key(1),
            TacticalForceKey::Slot {
                attacker: true,
                slot: 7
            }
        );
        assert_eq!(tactical.force_key(2), TacticalForceKey::Ship(2));
        assert_eq!(tactical.force_key(3), TacticalForceKey::Ship(3));
    }

    // FUN_005ea740: the speed skips members that cannot move; +0x134 counts
    // the turns the anchor stood still (against +0xc8) and resets when it
    // moved.
    #[test]
    fn the_formation_speed_skips_stopped_members_and_counts_still_turns() {
        let moving = ship(true, TacticalWeaponArc::default(), 0.0);
        let mut stopped = ship(true, TacticalWeaponArc::default(), 0.0);
        stopped.subsystem_capacity.engines = false;
        stopped.sublight_engine_power = 0.0;
        stopped.source_position.x = 30.0;
        let mut tactical = session(vec![moving, stopped], Vec::new());
        assert_eq!(mobility(&tactical, 1), 0.0);
        tactical.update_force_movers();
        let speed = mobility(&tactical, 0);
        assert!(speed > 0.0);
        assert_eq!(tactical.force_movers[0].speed, speed);
        assert_eq!(tactical.force_movers[0].still_turns, 1);
        tactical.update_force_movers();
        assert_eq!(tactical.force_movers[0].still_turns, 2);
        tactical.force_movers[0].anchor.z = 5.0;
        tactical.update_force_movers();
        assert_eq!(tactical.force_movers[0].still_turns, 0);
        assert_eq!(tactical.force_movers[0].last_turn_anchor.z, 5.0);
        // A turn rate change reaches the mover once a turn.
        tactical.ships[0].maneuverability = 20;
        tactical.ships[1].maneuverability = 20;
        tactical.update_force_movers();
        assert_eq!(
            tactical.force_movers[0].turn_rate,
            crate::tactical::original_capital_turn_rate(20)
        );
    }

    // FUN_005ec400: Stand Off's places alternate along the slot axis,
    // spaced by the largest footprint x 1.2 (at least 5); 0x005eb040 and
    // 0x005eb8d0: Surround's stand at the anchor less slot k + 1.
    #[test]
    fn formation_places_follow_their_slots() {
        use crate::tactical::{original_surround_slot, OriginalTacticalCollisionEnvelope};
        let mut ships = (0..12)
            .map(|_| ship(true, TacticalWeaponArc::default(), 0.0))
            .collect::<Vec<_>>();
        ships[1].source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 10.0,
        });
        let tactical = session(ships, Vec::new());
        let members = (0..12).collect::<Vec<_>>();
        let mut mover = test_mover(TacticalTactic::StandOff);
        mover.anchor = crate::tactical::TacticalWorldPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        mover.lateral = vector(0.6, 0.0, 0.8);
        let place = |ordinal| tactical.force_member_place(&mover, &members, ordinal);
        assert_eq!(place(0), mover.anchor);
        let first = place(1);
        assert!((first.x - (1.0 + 0.6 * 12.0)).abs() < 1e-5, "{first:?}");
        assert!((first.z - (3.0 + 0.8 * 12.0)).abs() < 1e-5, "{first:?}");
        assert_eq!(first.y, 2.0);
        let second = place(2);
        assert!((second.x - (1.0 - 0.6 * 12.0)).abs() < 1e-5, "{second:?}");
        assert!((second.z - (3.0 - 0.8 * 12.0)).abs() < 1e-5, "{second:?}");
        mover.tactic = TacticalTactic::Surround;
        for ordinal in [0, 1, 11] {
            let slot = original_surround_slot(ordinal + 1);
            let at = tactical.force_member_place(&mover, &members, ordinal);
            assert!((at.x - (1.0 - slot.x)).abs() < 1e-5, "{ordinal} {at:?}");
            assert!((at.y - (2.0 - slot.y)).abs() < 1e-5, "{ordinal} {at:?}");
            assert!((at.z - (3.0 - slot.z)).abs() < 1e-5, "{ordinal} {at:?}");
        }
        let slot = original_surround_slot(12);
        assert!(slot.x != 0.0 && slot.y != 0.0 && slot.z != 0.0);
    }

    // 0x005eb1a0 / 0x005eb540: the anchor climbs at 0.4 of its speed
    // (DAT_0066d844); Stand Off's flip negates all three axes, Surround's
    // x and z; Stand Off's turn turns its slot axis by the same angle
    // (FUN_005ead90).
    #[test]
    fn the_anchor_climbs_flips_and_turns_its_slots() {
        use crate::tactical::TacticalWorldPosition;
        let mut mover = test_mover(TacticalTactic::StandOff);
        let point = TacticalWorldPosition {
            x: 0.0,
            y: 30.0,
            z: 40.0,
        };
        mover.heading = vector(0.0, 0.6, 0.8);
        BattleSession::step_force_anchor(&mut mover, point, 1.0, 250.0);
        assert!(near(mover.velocity, vector(0.0, 0.6 * 3.8 * 0.4, 0.8 * 3.8)), "{mover:?}");
        assert!((mover.anchor.y - 0.6 * 3.8 * 0.4 * 0.25).abs() < 1e-6);
        assert!((mover.anchor.z - 0.8 * 3.8 * 0.25).abs() < 1e-6);

        let mut flip = test_mover(TacticalTactic::StandOff);
        flip.forward = vector(0.48, 0.6, 0.64);
        flip.heading = vector(-1.0, 0.0, 0.0);
        let behind = TacticalWorldPosition {
            x: -50.0,
            y: 0.0,
            z: 0.0,
        };
        BattleSession::step_force_anchor(&mut flip, behind, 1.0, 250.0);
        assert!(near(flip.forward, vector(-0.48, -0.6, -0.64)), "{flip:?}");

        let mut surround = test_mover(TacticalTactic::Surround);
        surround.forward = vector(0.6, 0.0, 0.8);
        BattleSession::step_force_anchor(&mut surround, behind, 1.0, 250.0);
        assert!(near(surround.forward, vector(-0.6, 0.0, -0.8)), "{surround:?}");
        assert!(near(surround.heading, vector(-1.0, 0.0, 0.0)), "{surround:?}");

        let mut turning = test_mover(TacticalTactic::StandOff);
        turning.lateral = vector(0.6, 0.0, 0.8);
        turning.turn_rate = std::f32::consts::FRAC_PI_2;
        turning.heading = vector(1.0, 0.0, 0.0);
        let aside = TacticalWorldPosition {
            x: 50.0,
            y: 0.0,
            z: 0.0,
        };
        BattleSession::step_force_anchor(&mut turning, aside, 1.0, 250.0);
        let (sin, cos) = (-std::f32::consts::FRAC_PI_8).sin_cos();
        assert!(
            near(turning.lateral, vector(0.6 * cos - sin * 0.8, 0.0, 0.6 * sin + cos * 0.8)),
            "{turning:?}"
        );
    }

    // FUN_005b2f30: a member below the mover's speed keeps its own; off its
    // heading it moves along its turned level forward by speed x alignment.
    #[test]
    fn a_member_moves_by_its_alignment_at_no_more_than_its_own_speed() {
        use crate::tactical::turn_original_tactical_ship;
        let mut member = ship(true, TacticalWeaponArc::default(), 0.0);
        member.source_forward = vector(0.6, 0.0, 0.8);
        member.source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
        let mut tactical = session(vec![member], Vec::new());
        tactical.advance_original_tactical_movement(250.0);
        let own = mobility(&tactical, 0);
        tactical.force_movers[0].speed = own * 10.0;
        tactical.ships[0].source_position.z = -40.0;
        tactical.ships[0].source_forward = vector(0.6, 0.0, 0.8);
        tactical.ships[0].source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
        let mut expected = tactical.ships[0].clone();
        expected.source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
        turn_original_tactical_ship(&mut expected, 250.0);
        tactical.advance_original_tactical_movement(250.0);
        let forward = expected.source_forward;
        let velocity = vector(forward.x * own * 0.8, 0.0, forward.z * own * 0.8);
        assert!(near(tactical.ships[0].source_velocity, velocity), "{:?}", tactical.ships[0]);
        assert!((tactical.ships[0].source_position.x - velocity.x * 0.25).abs() < 1e-5);
        assert!((tactical.ships[0].source_position.z - (-40.0 + velocity.z * 0.25)).abs() < 1e-5);
    }

    // FUN_005b4d40: the bearing turns by the arc (here from (30, 0, 40));
    // a formation that is not still (velocity (0, 1, 0) after an arrival
    // without a turn, 0x005eb1a0) keeps its forward.
    #[test]
    fn only_a_still_formation_turns_an_arc_to_the_target() {
        let mut gunner = ship(true, TacticalWeaponArc::default(), 0.0);
        gunner.weapon_arcs[1] = TacticalWeaponArc::new(0, 4, 0);
        gunner.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(1));
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        target.source_position.x = 30.0;
        target.source_position.z = 40.0;
        target.task_force = 1;
        let mut tactical = session(vec![gunner, target], Vec::new());
        tactical.advance_original_tactical_movement(250.0);
        tactical.force_movers[0].still_turns = 5;
        tactical.advance_original_tactical_movement(250.0);
        let wanted = tactical.ships[0].source_desired_forward;
        assert!(near(wanted, vector(-0.8, 0.0, 0.6)), "{wanted:?}");
        // Arc 2 turns the bearing by pi.
        tactical.ships[0].weapon_arcs[1] = TacticalWeaponArc::default();
        tactical.ships[0].weapon_arcs[2] = TacticalWeaponArc::new(0, 4, 0);
        tactical.advance_original_tactical_movement(250.0);
        let wanted = tactical.ships[0].source_desired_forward;
        assert!(near(wanted, vector(-0.6, 0.0, -0.8)), "{wanted:?}");
        // An arrival within reach, straight ahead: no turn, velocity (0, 1, 0).
        let mut fresh = session(tactical.ships.clone(), Vec::new());
        fresh.ships[0].source_forward = TacticalWorldVector::SOURCE_FORWARD;
        fresh.ships[0].source_desired_forward = TacticalWorldVector::SOURCE_FORWARD;
        fresh.advance_original_tactical_movement(250.0);
        fresh.force_movers[0].still_turns = 5;
        fresh.ships[0].source_waypoint = Some(crate::tactical::TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 0.5,
        });
        fresh.advance_original_tactical_movement(250.0);
        assert_eq!(fresh.force_movers[0].velocity.y, 1.0);
        assert!(near(fresh.ships[0].source_desired_forward, TacticalWorldVector::SOURCE_FORWARD));
    }

    // FUN_005cc300: closing on a target arrives within 2.0, a route, a
    // maneuver or an order without a target within 1.0.
    #[test]
    fn closing_on_a_target_arrives_within_two() {
        let goal = crate::tactical::TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: 1.5,
        };
        let arrives = |route: bool, target: bool, order: TacticalOrder| {
            let mut lead = ship(true, TacticalWeaponArc::default(), 0.0);
            lead.order = order;
            lead.source_waypoint = Some(goal);
            if route {
                lead.navigation_route = vec![goal];
            }
            if target {
                lead.attack_target = Some(crate::tactical::TacticalAttackTarget::CapitalShip(1));
            }
            let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
            enemy.source_position.z = 80.0;
            let mut tactical = session(vec![lead, enemy], Vec::new());
            tactical.advance_original_tactical_movement(10.0);
            tactical.ships[0].source_waypoint.is_none()
        };
        assert!(arrives(false, true, TacticalOrder::AttackCapitalShips));
        assert!(!arrives(true, true, TacticalOrder::AttackCapitalShips));
        assert!(!arrives(false, false, TacticalOrder::AttackCapitalShips));
        assert!(!arrives(false, true, TacticalOrder::LeftHook));
        assert!(!arrives(false, true, TacticalOrder::Escort));
    }

    // FUN_005b4d40, order 3: a withdrawing member heads away from its side,
    // toward its exit line.
    #[test]
    fn a_withdrawing_member_heads_for_its_exit() {
        let mut leaving = ship(true, TacticalWeaponArc::default(), 0.0);
        leaving.retreating = true;
        leaving.task_force = 255;
        let mut tactical = session(vec![leaving], Vec::new());
        let exit = tactical.withdrawal_exit_z(true);
        assert!(exit < 0.0);
        // Beyond the field's middle, on the far side.
        tactical.ships[0].source_position.z = -exit + 10.0;
        tactical.advance_original_tactical_movement(250.0);
        assert!(near(tactical.ships[0].source_desired_forward, vector(0.0, 0.0, -1.0)));
        // Between the middle and its own exit.
        let mut nearer = session(tactical.ships.clone(), Vec::new());
        nearer.ships[0].source_position.z = exit * 0.5;
        nearer.ships[0].source_desired_forward = vector(0.0, 0.0, -1.0);
        nearer.advance_original_tactical_movement(250.0);
        assert!(near(nearer.ships[0].source_desired_forward, vector(0.0, 0.0, -1.0)));
    }

    fn neighbour(y: f32, half_height: f32, id: usize) -> crate::tactical::TacticalHeightNeighbour {
        crate::tactical::TacticalHeightNeighbour { y, half_height, id }
    }

    // FUN_005b2f30's vertical step between the nearby ships' floor (+0x404)
    // and ceiling (+0x408). Heights sit off zero so that sums and
    // differences of heights differ.
    #[test]
    fn a_member_climbs_within_its_height_band() {
        use crate::tactical::original_height_band_step as band;
        let own = neighbour(10.0, 1.0, 5);
        // Free: the step's length.
        assert_eq!(band(own, &[], 5.0, 2.0), (12.0, false));
        assert_eq!(band(own, &[], -0.5, 2.0), (9.5, false));
        assert_eq!(band(own, &[], 0.0, 2.0), (10.0, false));
        // A ship above at 13: the ceiling is 13 - 1 - 1 = 11.
        assert_eq!(band(own, &[neighbour(13.0, 1.0, 1)], 5.0, 2.0), (11.0, false));
        // Diving is not limited by a ship above.
        assert_eq!(band(own, &[neighbour(13.0, 1.0, 1)], -5.0, 2.0), (8.0, false));
        // A ship below at 7.5: the floor is 9.5.
        assert_eq!(band(own, &[neighbour(7.5, 1.0, 1)], -5.0, 2.0), (9.5, false));
        // Climbing is not limited by a ship below.
        assert_eq!(band(own, &[neighbour(7.5, 1.0, 1)], 5.0, 2.0), (12.0, false));
        // A floor exactly at its height is not inside it: it climbs.
        assert_eq!(band(own, &[neighbour(8.0, 1.0, 1)], 5.0, 1.0), (11.0, false));
        // The ceiling at its own height holds the climb.
        assert_eq!(band(own, &[neighbour(12.0, 1.0, 1)], 5.0, 2.0), (10.0, true));
        // Inside the ceiling (9.5) it steps down, at most the step.
        assert_eq!(band(own, &[neighbour(11.5, 1.0, 1)], 5.0, 0.3), (9.7, false));
        assert_eq!(band(own, &[neighbour(11.5, 1.0, 1)], 5.0, 2.0), (9.5, false));
        // Inside the floor (10.5) it steps up.
        assert_eq!(band(own, &[neighbour(8.5, 1.0, 1)], -5.0, 0.3), (10.3, false));
        assert_eq!(band(own, &[neighbour(8.5, 1.0, 1)], -5.0, 2.0), (10.5, false));
        // Pinned inside both: held (inside both, the ceiling is below the
        // floor, so the code's "fits" branch never runs).
        assert_eq!(
            band(own, &[neighbour(11.5, 1.0, 1), neighbour(8.5, 1.0, 2)], 5.0, 2.0),
            (10.0, true)
        );
        // Pinned low, under a tall ship: still held.
        assert_eq!(
            band(
                neighbour(-20.0, 1.0, 5),
                &[neighbour(-19.9, 20.0, 1), neighbour(-20.1, 1.0, 2)],
                5.0,
                2.0
            ),
            (-20.0, true)
        );
        // At the same height: a higher id goes above, a lower or equal one
        // below; either way it is inside and steps out.
        assert_eq!(band(own, &[neighbour(10.0, 1.0, 9)], 0.0, 0.5), (9.5, false));
        assert_eq!(band(own, &[neighbour(10.0, 1.0, 5)], 0.0, 0.5), (10.5, false));
        // With only a ship above, a same-height one with a lower id goes above.
        assert_eq!(
            band(own, &[neighbour(20.0, 1.0, 9), neighbour(10.0, 1.0, 1)], 0.0, 0.5),
            (9.5, false)
        );
        // With only a ship below, a higher id goes below.
        assert_eq!(
            band(own, &[neighbour(0.0, 1.0, 1), neighbour(10.0, 1.0, 9)], 0.0, 0.5),
            (10.5, false)
        );
        // With both, it goes above only when the room below (y - floor)
        // exceeds the room above (ceiling - y). Here 1 against 6: below.
        assert_eq!(
            band(
                own,
                &[neighbour(18.0, 1.0, 1), neighbour(7.0, 1.0, 2), neighbour(10.0, 1.0, 9)],
                0.0,
                0.5
            ),
            (10.5, false)
        );
        // Equal room (3 and 3): below.
        assert_eq!(
            band(
                own,
                &[neighbour(15.0, 1.0, 1), neighbour(5.0, 1.0, 2), neighbour(10.0, 1.0, 9)],
                0.0,
                0.5
            ),
            (10.5, false)
        );
        // 8 against 6: above; 3 against 5: below.
        assert_eq!(
            band(
                own,
                &[neighbour(18.0, 1.0, 1), neighbour(0.0, 1.0, 2), neighbour(10.0, 1.0, 9)],
                0.0,
                0.5
            ),
            (9.5, false)
        );
        assert_eq!(
            band(
                own,
                &[neighbour(17.0, 1.0, 1), neighbour(5.0, 1.0, 2), neighbour(10.0, 1.0, 9)],
                0.0,
                0.5
            ),
            (10.5, false)
        );
        // And 6 against 1: above.
        assert_eq!(
            band(
                own,
                &[neighbour(13.0, 1.0, 1), neighbour(2.0, 1.0, 2), neighbour(10.0, 1.0, 9)],
                0.0,
                0.5
            ),
            (9.5, false)
        );
    }

    // FUN_005cdbf0: neighbours along x until the first beyond both reaches
    // (half the speed plus 5/9 of the planar diameter, DAT_0066c534), kept
    // when z is within the reaches and y within both half-heights plus 0.4
    // of the frame's travel (DAT_0066d03c).
    #[test]
    fn nearby_capitals_stop_at_the_first_beyond_reach() {
        use crate::tactical::OriginalTacticalCollisionEnvelope;
        let envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 9.0,
        });
        // The ship stands off the origin, at (3, 7, -2).
        let at = |x: f32, y: f32, z: f32| {
            let mut unit = ship(true, TacticalWeaponArc::default(), 0.0);
            unit.source_collision_envelope = envelope;
            unit.source_position = crate::tactical::TacticalWorldPosition {
                x: 3.0 + x,
                y: 7.0 + y,
                z: -2.0 + z,
            };
            unit
        };
        // Reach 5 each, 10 together; travel 2 gives a vertical limit of
        // 1 + 1 + 0.8.
        let ships = vec![
            at(0.0, 0.0, 0.0),
            at(10.0, 0.0, 0.0),
            at(11.0, 0.0, 0.0),
            at(15.0, 0.0, 0.0),
            at(-9.0, 0.0, 10.5),
            at(-8.0, 2.7, -10.0),
            at(-7.0, 2.9, 0.0),
            at(-6.0, -2.9, 0.0),
        ];
        let mut tactical = session(ships, Vec::new());
        let mut nearby = tactical.nearby_capitals(0, 2.0);
        nearby.sort_unstable();
        // 10 away exactly is within; 11 stops the walk, so 15 is never
        // reached; -9 is too far in z; 2.9 above or below is too high.
        assert_eq!(nearby, vec![1, 5]);
        // Speed widens the reach: at 4 the reach is 7, 12 together.
        tactical.ships[0].source_velocity = vector(0.0, 0.0, 4.0);
        let mut nearby = tactical.nearby_capitals(0, 2.0);
        nearby.sort_unstable();
        assert_eq!(nearby, vec![1, 2, 4, 5]);
        // Destroyed, jumping or envelope-less ships are not in the tree.
        tactical.ships[1].alive = false;
        tactical.ships[2].retreat_progress = 0.5;
        tactical.ships[4].source_collision_envelope = None;
        assert_eq!(tactical.nearby_capitals(0, 2.0), vec![5]);
        // A ship without an envelope has no neighbours.
        tactical.ships[0].source_collision_envelope = None;
        assert!(tactical.nearby_capitals(0, 2.0).is_empty());
    }

    // FUN_005b2f30: +0x414 records a vertical step that the band held or
    // that overlapped a nearby ship; a member with nothing to cover clears it.
    #[test]
    fn a_held_climb_marks_the_member_blocked() {
        use crate::tactical::OriginalTacticalCollisionEnvelope;
        let envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 4.0,
        });
        let mut climber = ship(true, TacticalWeaponArc::default(), 0.0);
        climber.source_collision_envelope = envelope;
        climber.task_force = 255;
        let mut roof = ship(false, TacticalWeaponArc::default(), 0.0);
        roof.source_collision_envelope = envelope;
        roof.subsystem_capacity.engines = false;
        roof.sublight_engine_power = 0.0;
        roof.task_force = 255;
        roof.source_position.y = 2.0;
        roof.source_position.x = 1.0;
        let mut tactical = session(vec![climber, roof], Vec::new());
        tactical.advance_original_tactical_movement(250.0);
        // Its place is 10 above: the roof's ceiling at 0 holds it.
        let key = tactical.force_key(0);
        for mover in &mut tactical.force_movers {
            if mover.force == key {
                mover.anchor.y = 10.0;
            }
        }
        tactical.advance_original_tactical_movement(250.0);
        assert_eq!(tactical.ships[0].source_position.y, 0.0);
        assert!(tactical.member_states[0].blocked);
        // Without the roof it climbs and is not blocked.
        tactical.ships[1].source_position.x = 100.0;
        tactical.advance_original_tactical_movement(250.0);
        assert!(tactical.ships[0].source_position.y > 0.0);
        assert!(!tactical.member_states[0].blocked);
    }

    // FUN_005ea740: once a turn, a moving member held in its climb (+0x414)
    // in a formation that stood still five turns (+0x134), more than 12
    // turns (DAT_0066d858) after its last detour (+0x410 against +0x87c),
    // takes detour (id + 1 + last) & 3; unblocked, its record goes after
    // those 12 turns.
    #[test]
    fn a_held_member_of_a_still_formation_takes_a_detour() {
        use crate::tactical::ORIGINAL_TACTICAL_TURN_STEPS as STEPS;
        let mut ships = (0..6)
            .map(|_| ship(true, TacticalWeaponArc::default(), 0.0))
            .collect::<Vec<_>>();
        for (index, unit) in ships.iter_mut().enumerate() {
            unit.source_position.x = 40.0 * index as f32;
            unit.task_force = 255;
        }
        // Ships 0 and 4 share group slot 0.
        ships[0].task_force = 0;
        ships[4].task_force = 0;
        let mut tactical = session(ships, Vec::new());
        // +0x87c is 13 on turn 11's updates.
        tactical.combat_tick = 11 * STEPS;
        assert_eq!(tactical.step_event_stamp(), 13);
        tactical.update_force_movers();
        for held in [0, 2, 3, 5] {
            tactical.member_states[held].blocked = true;
        }
        for _ in 0..4 {
            tactical.update_force_movers();
        }
        // The count reaches 5 on this last update, after its loop.
        assert_eq!(tactical.force_movers[0].still_turns, 5);
        assert!(tactical.force_detours.is_empty());
        tactical.update_force_movers();
        let detour = |tactical: &BattleSession, ship: usize| {
            tactical
                .force_detours
                .iter()
                .find(|detour| detour.ship == ship)
                .map(|detour| (detour.offset.x, detour.offset.y, detour.offset.z))
        };
        // Ship 0 (id 1): index (1 + 1 + 0) & 3 = 2, (5, 0, 20).
        assert_eq!(detour(&tactical, 0), Some((5.0, 0.0, 20.0)));
        assert_eq!(tactical.member_states[0].detour_stamp, 13.0);
        // Ship 2 (id 3): index (3 + 1 + 0) & 3 = 0, (20, 0, 0).
        assert_eq!(detour(&tactical, 2), Some((20.0, 0.0, 0.0)));
        // Ship 3 (id 4): (4 + 1) & 3 = 1, (-20, 10, 5); ship 5 (id 6): 3,
        // (0, -10, -20).
        assert_eq!(detour(&tactical, 3), Some((-20.0, 10.0, 5.0)));
        assert_eq!(detour(&tactical, 5), Some((0.0, -10.0, -20.0)));
        // Ship 1 is not held.
        assert_eq!(detour(&tactical, 1), None);
        // Ship 4, in ship 0's force, is held from turn 23 on.
        tactical.member_states[4].blocked = true;
        // 12 turns later nothing changes; 13 turns later the next detour.
        tactical.combat_tick = 23 * STEPS;
        tactical.update_force_movers();
        assert_eq!(detour(&tactical, 0), Some((5.0, 0.0, 20.0)));
        tactical.combat_tick = 24 * STEPS;
        tactical.update_force_movers();
        // Ship 4 took (5 + 1 + 0) & 3 = 2 on turn 23.
        assert_eq!(detour(&tactical, 4), Some((5.0, 0.0, 20.0)));
        // Ship 0: (1 + 1 + 2) & 3 = 0; ship 2 (id 3) keeps index 0;
        // ship 3: (4 + 1 + 1) & 3 = 2.
        assert_eq!(detour(&tactical, 0), Some((20.0, 0.0, 0.0)));
        assert_eq!(detour(&tactical, 2), Some((20.0, 0.0, 0.0)));
        assert_eq!(detour(&tactical, 3), Some((5.0, 0.0, 20.0)));
        assert_eq!(tactical.force_detours.len(), 5);
        // Unblocked, the record stays until 12 turns have passed.
        tactical.member_states[0].blocked = false;
        tactical.combat_tick = 36 * STEPS;
        tactical.update_force_movers();
        assert!(detour(&tactical, 0).is_some());
        tactical.combat_tick = 37 * STEPS;
        tactical.update_force_movers();
        assert_eq!(detour(&tactical, 0), None);
        // Its force's other member keeps its own.
        assert!(detour(&tactical, 4).is_some());
        // A moving formation, or a member that cannot move, takes none.
        tactical.member_states[0].blocked = true;
        tactical.combat_tick = 60 * STEPS;
        let key = tactical.force_key(0);
        for mover in &mut tactical.force_movers {
            if mover.force == key {
                mover.velocity = vector(0.0, 0.0, 1.0);
            }
        }
        tactical.update_force_movers();
        assert_eq!(detour(&tactical, 0), None);
        tactical.ships[1].subsystem_capacity.engines = false;
        tactical.ships[1].sublight_engine_power = 0.0;
        tactical.member_states[1].blocked = true;
        tactical.update_force_movers();
        assert_eq!(detour(&tactical, 1), None);
    }

    // 0x5eb040: a detour's raw offset replaces the distance to the place;
    // Stand Off's turn turns it with the slots (FUN_005ead90); a rebuilt
    // mover drops the force's records.
    #[test]
    fn a_detour_steers_the_member_and_turns_with_the_slots() {
        use crate::tactical::{TacticalForceDetour, TacticalWorldPosition};
        let mut member = ship(true, TacticalWeaponArc::default(), 0.0);
        member.source_forward = vector(1.0, 0.0, 0.0);
        member.source_desired_forward = vector(1.0, 0.0, 0.0);
        // A second member of the same force, at its place 5 along +x.
        let mut beside = member.clone();
        beside.source_position.x = 5.0;
        let mut tactical = session(vec![member, beside], Vec::new());
        tactical.advance_original_tactical_movement(250.0);
        let key = tactical.force_key(0);
        tactical.force_detours.push(TacticalForceDetour {
            force: key,
            ship: 0,
            offset: vector(20.0, 4.0, 0.0),
        });
        tactical.ships[0].source_forward = vector(1.0, 0.0, 0.0);
        tactical.ships[0].source_desired_forward = vector(1.0, 0.0, 0.0);
        let speed = mobility(&tactical, 0);
        tactical.advance_original_tactical_movement(250.0);
        // At its place, it still moves along +x and climbs; the other
        // member, with no record, stays at its place.
        assert!((tactical.ships[0].source_position.x - speed * 0.25).abs() < 1e-5);
        assert!((tactical.ships[0].source_position.y - speed * 0.4 * 0.25).abs() < 1e-5);
        assert_eq!(tactical.ships[1].source_position.x, 5.0);
        // A Stand Off turn turns the offset by the same angle.
        tactical.force_detours[0].offset = vector(20.0, 4.0, 10.0);
        tactical.ships[0].source_waypoint = Some(TacticalWorldPosition {
            x: 50.0,
            y: 0.0,
            z: 0.0,
        });
        tactical.advance_original_tactical_movement(250.0);
        let turn = tactical.force_movers[0].slot_turn;
        assert!(turn != 0.0);
        let (sin, cos) = turn.sin_cos();
        let offset = tactical.force_detours[0].offset;
        assert!(
            near(offset, vector(20.0 * cos - sin * 10.0, 4.0, 20.0 * sin + cos * 10.0)),
            "{offset:?}"
        );
        // A new tactic rebuilds the mover and drops the record.
        tactical.task_force_formations[1][0] = TacticalTactic::Surround;
        tactical.advance_original_tactical_movement(250.0);
        assert!(tactical.force_detours.is_empty());
    }

    // FUN_005c8780's review (turn 1 and every sixth after) runs FUN_005c9200:
    // the computer's task forces are paired with the enemy's units, and each
    // link (FUN_005a8fc0) is event 0x36 at the command stamp, taking effect
    // three turns later (FUN_005cae10). A laser-armed leader (FUN_005b7ce0)
    // takes the fighter list first; the player's side gets nothing.
    #[test]
    fn the_computer_commander_links_its_forces_to_targets_on_its_command_turn() {
        use crate::tactical::{TacticalAttackTarget, TacticalTurnEvent, TacticalUnitRef};
        let mut player_ship = ship(true, TacticalWeaponArc::new(0, 0, 2), 1.0);
        player_ship.source_position.z = -50.0;
        player_ship.weapon_ranges.turbolaser = 20.0;
        let player_wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        // The computer's cruiser carries only turbolasers, its TIE group lasers.
        let mut cruiser = ship(false, TacticalWeaponArc::new(0, 0, 2), 1.0);
        cruiser.source_position.z = 50.0;
        cruiser.weapon_ranges.turbolaser = 30.0;
        cruiser.task_force = 1;
        let mut ties = fighter(false, TacticalWeaponArc::new(2, 0, 0));
        ties.source_position.z = 30.0;
        let mut tactical = session(vec![player_ship, cruiser], vec![player_wing, ties]);
        for _ in 0..4 {
            tactical.step();
        }
        // Turn 1's review queued the links, none in effect yet.
        let links = tactical
            .turn_events
            .iter()
            .filter_map(|queued| match &queued.event {
                TacticalTurnEvent::TaskForceTarget { units, target } => {
                    Some((queued.stamp, units.clone(), *target))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(links.len(), 2, "{links:?}");
        assert!(links.iter().all(|(stamp, _, _)| *stamp == 3));
        assert!(tactical.ships[1].attack_target.is_none());
        assert!(tactical.fighters[1].attack_target.is_none());
        // The player's units get none.
        assert!(links.iter().all(|(_, units, _)| !units.iter().any(|unit| matches!(
            unit,
            TacticalUnitRef::Ship(0) | TacticalUnitRef::Fighter(0)
        ))));
        // Turn 4: the cruiser takes the player's ship, the TIEs the
        // player's fighters.
        for _ in 0..12 {
            tactical.step();
        }
        assert_eq!(tactical.ships[1].attack_target, Some(TacticalAttackTarget::CapitalShip(0)));
        assert_eq!(tactical.fighters[1].attack_target, Some(TacticalAttackTarget::FighterGroup(0)));
        assert!(tactical.ships[0].attack_target.is_none());
        // A kept target posts no new link at the next review (turn 7).
        for _ in 0..12 {
            tactical.step();
        }
        assert!(!tactical
            .turn_events
            .iter()
            .any(|queued| matches!(queued.event, TacticalTurnEvent::TaskForceTarget { .. })));
        // And the computer's ship closes on its target.
        let gap = |t: &BattleSession| t.ships[1].source_position.z - t.ships[0].source_position.z;
        let before = gap(&tactical);
        for _ in 0..8 {
            tactical.step();
        }
        assert!(gap(&tactical) < before, "{before} {}", gap(&tactical));
    }

    // FUN_005ed5f0: a force on Recover (2), withdrawing (3) or on the Death
    // Star run (6) is left out of the commander's pairing; an idle one is
    // paired.
    #[test]
    fn busy_computer_forces_keep_out_of_the_pairing() {
        let mut player = ship(true, TacticalWeaponArc::new(0, 0, 2), 1.0);
        player.source_position.z = -50.0;
        let idle = |task_force: u8| {
            let mut unit = ship(false, TacticalWeaponArc::new(0, 0, 2), 1.0);
            unit.source_position.z = 50.0;
            unit.task_force = task_force;
            unit
        };
        let mut leaving = idle(2);
        leaving.retreating = true;
        let mut recovering = fighter(false, TacticalWeaponArc::new(2, 0, 0));
        recovering.order = TacticalOrder::Recover;
        recovering.fighter_group = 1;
        let mut raiding = fighter(false, TacticalWeaponArc::new(2, 0, 0));
        raiding.order = TacticalOrder::AttackDeathStar;
        raiding.fighter_group = 2;
        let tactical = session(
            vec![player, idle(1), leaving],
            vec![recovering, raiding],
        );
        let units = commander_targets(&tactical, false)
            .into_iter()
            .map(|(unit, _)| unit)
            .collect::<Vec<_>>();
        assert_eq!(units, vec![AbstractUnit::Capital(1)]);
    }

    // Command 0x7b (FUN_0059fd40) from Observe Battle puts the player's
    // commander in mode 1 (FUN_005c7e10): its count starts at 0, so it
    // reviews on the next turn's updates and every sixth after; Take Command
    // stops it.
    #[test]
    fn observing_hands_the_player_side_to_its_commander() {
        use crate::tactical::{
            TacticalTurnEvent, TacticalUnitRef, ORIGINAL_TACTICAL_TURN_STEPS as STEPS,
        };
        let mut player = ship(true, TacticalWeaponArc::new(0, 0, 2), 1.0);
        player.source_position.z = -50.0;
        let mut enemy = ship(false, TacticalWeaponArc::new(0, 0, 2), 1.0);
        enemy.source_position.z = 50.0;
        enemy.task_force = 1;
        let mut tactical = session(vec![player, enemy], Vec::new());
        let player_links = |t: &BattleSession| {
            t.turn_events
                .iter()
                .filter(|queued| match &queued.event {
                    TacticalTurnEvent::TaskForceTarget { units, .. } => {
                        units.contains(&TacticalUnitRef::Ship(0))
                    }
                    _ => false,
                })
                .count()
        };
        for _ in 0..(2 * STEPS) {
            tactical.step();
        }
        assert_eq!(player_links(&tactical), 0);
        tactical.set_player_observing(true);
        // The next turn's updates (turn 2, between the computer's reviews).
        for _ in 0..STEPS {
            tactical.step();
        }
        assert_eq!(player_links(&tactical), 1);
        assert_eq!(tactical.observer_review_countdown, 5);
        // Observe pressed again while observing keeps the count.
        tactical.set_player_observing(true);
        assert_eq!(tactical.observer_review_countdown, 5);
        // Taking command back stops the reviews; switching it on again
        // restarts the count.
        tactical.set_player_observing(false);
        tactical.ships[0].attack_target = None;
        tactical.turn_events.clear();
        for _ in 0..(8 * STEPS) {
            tactical.step();
        }
        assert_eq!(player_links(&tactical), 0);
        tactical.set_player_observing(true);
        assert_eq!(tactical.observer_review_countdown, 0);
    }

    // 0x5c8bbc..0x5c8c33 of FUN_005c8780: on a review, the Empire's
    // commander with its Death Star operational and charged to exactly 100
    // (DAT_0066cbd8) fires at the first capital ship of its priority list
    // still in the fight (FUN_005ba720, event 0x44).
    #[test]
    fn the_computer_empire_fires_a_full_superlaser_at_its_first_priority_ship() {
        use crate::tactical::{TacticalDeathStar, TacticalTurnEvent, TacticalWorldPosition};
        let mut small = ship(true, TacticalWeaponArc::new(0, 0, 1), 1.0);
        small.source_position.z = -40.0;
        let mut big = ship(true, TacticalWeaponArc::new(0, 0, 8), 1.0);
        big.source_position.z = -45.0;
        big.source_position.x = 20.0;
        big.hull_max = 5_000;
        big.hull_current = 5_000;
        // The big ship (index 0) outranks the small one (FUN_005ed8e0).
        let mut tactical = session(vec![big, small], Vec::new());
        let death_star = |charge: f32| TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 60.0,
            },
            hull: 100.0,
            laser_charge: charge,
            shielded: false,
            destroyed: false,
            action_committed: false,
        };
        let beams = |t: &BattleSession| {
            t.turn_events
                .iter()
                .filter_map(|queued| match queued.event {
                    TacticalTurnEvent::DeathStarBeam { target } => Some((queued.stamp, target)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // Short of full: no shot at turn 1's review.
        let mut early = tactical.clone();
        early.death_star = Some(death_star(10.0));
        for _ in 0..4 {
            early.step();
        }
        assert!(early.death_star.unwrap().laser_charge < 100.0);
        assert!(beams(&early).is_empty());
        // Full: the first-priority capital ship still in the fight.
        tactical.death_star = Some(death_star(100.0));
        assert_eq!(commander_superlaser_target(&tactical, false), Some(0));
        let mut fallback = tactical.clone();
        fallback.ships[0].alive = false;
        assert_eq!(commander_superlaser_target(&fallback, false), Some(1));
        // Destroyed or Alliance-owned, it does not fire.
        for broken in [
            TacticalDeathStar { destroyed: true, ..death_star(100.0) },
            TacticalDeathStar { is_alliance: true, ..death_star(100.0) },
        ] {
            let mut quiet = tactical.clone();
            quiet.death_star = Some(broken);
            for _ in 0..4 {
                quiet.step();
            }
            assert!(beams(&quiet).is_empty());
        }
        for _ in 0..4 {
            tactical.step();
        }
        assert_eq!(beams(&tactical), vec![(3, 0)]);
        assert_eq!(tactical.death_star.unwrap().laser_charge, 0.0);
        // The firing voice belongs to the player's fire control only.
        assert!(!tactical
            .take_pending_voice_cues()
            .contains(&crate::tactical::TacticalVoiceCue::death_star_firing()));
        // An Alliance computer has no superlaser branch.
        let mut alliance = session(vec![ship(false, TacticalWeaponArc::new(0, 0, 1), 1.0)], Vec::new());
        alliance.player_is_attacker = false;
        alliance.death_star = Some(death_star(100.0));
        for _ in 0..4 {
            alliance.step();
        }
        assert!(beams(&alliance).is_empty());
    }

    // 0x5c8c47..0x5c8cb0 of FUN_005c8780: with an operational Death Star in
    // the battle, the Alliance's commander sends the group of its first
    // fighter on the Death Star run (order 6, once: +0x18) when FUN_005ad930
    // holds (attackable, more Alliance fighter groups in the fight than the
    // Empire's, none already on order 6); with no fighters it withdraws.
    #[test]
    fn the_computer_alliance_sends_its_first_group_on_the_death_star_run_once() {
        use crate::tactical::{TacticalDeathStar, TacticalTurnEvent, TacticalWorldPosition};
        let death_star = TacticalDeathStar {
            resource: crate::tactical_resources::DEATH_STAR_TACTICAL_RESOURCE,
            is_attacker: false,
            is_alliance: false,
            source_position: TacticalWorldPosition {
                x: 0.0,
                y: 0.0,
                z: 60.0,
            },
            hull: 100.0,
            laser_charge: 0.0,
            shielded: false,
            destroyed: false,
            action_committed: false,
        };
        // The Alliance attacks under the computer; the player is the Empire.
        let build = |alliance_groups: &[u8], empire_groups: usize| {
            let mut fighters = alliance_groups
                .iter()
                .map(|&group| {
                    let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
                    wing.fighter_group = group;
                    wing
                })
                .collect::<Vec<_>>();
            fighters.extend((0..empire_groups).map(|_| fighter(false, TacticalWeaponArc::new(2, 0, 0))));
            let mut tactical = session(
                vec![
                    ship(true, TacticalWeaponArc::new(0, 0, 2), 1.0),
                    ship(false, TacticalWeaponArc::new(0, 0, 2), 1.0),
                ],
                fighters,
            );
            tactical.player_is_attacker = false;
            tactical.death_star = Some(death_star);
            tactical
        };
        let executors = |t: &BattleSession| {
            t.turn_events
                .iter()
                .filter_map(|queued| match &queued.event {
                    TacticalTurnEvent::OrderExecutor { order, fighters, .. }
                        if *order == TacticalOrder::AttackDeathStar =>
                    {
                        Some((queued.stamp, fighters.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // Three Alliance wings (groups 1, 1, 2) against one TIE group: the
        // first wing's group, both its wings, at turn 1's review.
        let mut tactical = build(&[1, 1, 2], 1);
        for _ in 0..4 {
            tactical.step();
        }
        assert_eq!(executors(&tactical), vec![(3, vec![0, 1])]);
        assert_eq!(tactical.fighters[0].order, TacticalOrder::AttackDeathStar);
        assert_eq!(tactical.fighters[2].order, TacticalOrder::None);
        assert!(tactical.alliance_raid_sent);
        // Once only: even with the order taken back, the next review sends
        // none.
        tactical.fighters[0].order = TacticalOrder::None;
        tactical.fighters[1].order = TacticalOrder::None;
        tactical.turn_events.clear();
        for _ in 0..24 {
            tactical.step();
        }
        assert!(executors(&tactical).is_empty());
        // As many TIE groups as Alliance wings: no run.
        let mut even = build(&[1, 2], 2);
        for _ in 0..4 {
            even.step();
        }
        assert!(executors(&even).is_empty());
        assert!(!even.alliance_raid_sent);
        // A shielded Death Star cannot be attacked.
        let mut shielded = build(&[1, 1, 2], 1);
        shielded.death_star.as_mut().unwrap().shielded = true;
        for _ in 0..4 {
            shielded.step();
        }
        assert!(executors(&shielded).is_empty());
        // No Alliance fighters: the computer withdraws, where it may leave;
        // without a Death Star it does not.
        let mut bare = build(&[], 1);
        bare.retreat_allowed = [true, true];
        for _ in 0..4 {
            bare.step();
        }
        assert!(bare.retreat_done[0]);
        // The Alliance's ship leaves, not the player's.
        assert!(bare.ships[0].retreating && !bare.ships[1].retreating);
        let mut no_death_star = build(&[], 1);
        no_death_star.retreat_allowed = [true, true];
        no_death_star.death_star = None;
        for _ in 0..4 {
            no_death_star.step();
        }
        assert!(!no_death_star.retreat_done[0]);
        // Nor with a destroyed one, nor where it may not leave, nor when it
        // has already left; nor while it still has fighters.
        let withdrawals = |t: &BattleSession| {
            t.turn_events
                .iter()
                .filter(|queued| matches!(queued.event, TacticalTurnEvent::WithdrawalExecutor(_)))
                .count()
        };
        let mut wrecked = build(&[], 1);
        wrecked.retreat_allowed = [true, true];
        wrecked.death_star.as_mut().unwrap().destroyed = true;
        let mut barred = build(&[], 1);
        let mut gone = build(&[], 1);
        gone.retreat_allowed = [true, true];
        gone.retreat_done = [true, false];
        let mut armed = build(&[1, 2], 2);
        armed.retreat_allowed = [true, true];
        for case in [&mut wrecked, &mut barred, &mut gone, &mut armed] {
            for _ in 0..4 {
                case.step();
            }
            assert_eq!(withdrawals(case), 0);
        }
        assert!(!wrecked.retreat_done[0] && !barred.retreat_done[0] && !armed.retreat_done[0]);
        // An emptied wing is not in the fight: one Alliance wing against one
        // TIE group is no majority.
        let mut emptied = build(&[1, 2], 1);
        emptied.fighters[1].squad_count = 0;
        for _ in 0..4 {
            emptied.step();
        }
        assert!(executors(&emptied).is_empty());
        // Observing as the Alliance while a run is under way marks it sent;
        // as the Empire it does not.
        let mut observer = build(&[1, 1, 2], 1);
        observer.player_is_attacker = true;
        observer.trench_run_remaining_ms = Some(1_000.0);
        observer.set_player_observing(true);
        assert!(observer.alliance_raid_sent);
        let mut imperial = build(&[1, 1, 2], 1);
        imperial.trench_run_remaining_ms = Some(1_000.0);
        imperial.set_player_observing(true);
        assert!(!imperial.alliance_raid_sent);
    }

    // 0x5ebee0 (the FUN_005c72b0 mover's slot +0x30): the lead has no
    // offset; the member making the count n takes the traced unit offset,
    // which repeats every 30 and flips with a forward of (0, 0, -1).
    #[test]
    fn fighter_slots_follow_the_traced_offsets() {
        use crate::tactical::original_fighter_slot_offset as slot;
        assert_eq!(slot(1, 1.0), TacticalWorldVector::ZERO);
        let close = |n: usize, z: f32, expected: (f32, f32)| {
            let offset = slot(n, z);
            assert!(
                (offset.x - expected.0).abs() < 1e-3 && (offset.y - expected.1).abs() < 1e-3,
                "{n}: {offset:?}"
            );
            assert_eq!(offset.z, 0.0);
        };
        close(2, 1.0, (-0.809, -0.588));
        close(3, 1.0, (-0.5, 0.866));
        close(4, 1.0, (-0.105, -0.995));
        close(5, 1.0, (0.309, 0.951));
        close(8, 1.0, (1.0, 0.0));
        close(32, 1.0, (-0.809, -0.588));
        close(2, -1.0, (0.809, 0.588));
        // The traced table is a ring of unit vectors in the plane, mirrored:
        // entry i equals entry 27 - i, and the last two are equal.
        let table = crate::tactical::ORIGINAL_FIGHTER_SLOT_OFFSETS;
        for (i, [x, y, z]) in table.iter().enumerate() {
            assert!((x.hypot(*y) - 1.0).abs() < 1e-5 && *z == 0.0, "{i}");
            if i <= 27 {
                assert_eq!(table[i], table[27 - i], "{i}");
            }
        }
        assert_eq!(table[28], table[29]);
    }

    // 0x005ec0d0: beyond the radius the anchor moves by the unit delta x
    // speed x ms x 0.001 in three dimensions, unclamped; within it, or on
    // the point, the velocity clears and the step returns 0.
    #[test]
    fn the_fighter_anchor_steps_in_three_dimensions_and_can_overshoot() {
        use crate::tactical::{step_fighter_anchor, TacticalFighterMover, TacticalWorldPosition};
        let mut mover = TacticalFighterMover {
            alliance: true,
            group: 0,
            anchor: TacticalWorldPosition::ORIGIN,
            forward: TacticalWorldVector::SOURCE_FORWARD,
            velocity: TacticalWorldVector::ZERO,
            speed: 40.0,
            slots: Vec::new(),
        };
        let point = TacticalWorldPosition {
            x: 0.0,
            y: 3.0,
            z: 4.0,
        };
        assert!(step_fighter_anchor(&mut mover, point, 2.0, 250.0));
        // 10 along (0, 0.6, 0.8): past the point 5 away.
        assert!((mover.anchor.y - 6.0).abs() < 1e-5 && (mover.anchor.z - 8.0).abs() < 1e-5);
        assert!((mover.velocity.z - 32.0).abs() < 1e-5);
        let near = TacticalWorldPosition {
            x: 0.0,
            y: 6.0,
            z: 9.5,
        };
        assert!(!step_fighter_anchor(&mut mover, near, 2.0, 250.0));
        assert_eq!(mover.velocity, TacticalWorldVector::ZERO);
        let here = mover.anchor;
        assert!(!step_fighter_anchor(&mut mover, here, 0.0, 250.0));
    }

    // FUN_005adfa0: one fighter force per side and group, its mover on the
    // first member; the anchor moves at the slowest member's speed and each
    // squadron stands at the anchor less its slot.
    #[test]
    fn a_fighter_group_moves_as_one_force() {
        use crate::tactical::{original_fighter_slot_offset, TacticalAttackTarget};
        let mut fast = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        // 11 and 7 plus the commander term 1 (0x5ba190): 12 and 8.
        fast.sublight_engine_power = 11.0;
        fast.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        let mut slow = fast.clone();
        slow.sublight_engine_power = 7.0;
        slow.source_position.x = 3.0;
        let mut other_group = fast.clone();
        other_group.fighter_group = 2;
        let mut enemy_wing = fighter(false, TacticalWeaponArc::new(2, 0, 0));
        enemy_wing.source_position.z = 40.0;
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        target.source_position.z = 100.0;
        let mut tactical = session(vec![target], vec![fast, slow, other_group, enemy_wing]);
        tactical.advance_fighter_approach();
        assert_eq!(tactical.fighter_movers.len(), 3);
        let mover = tactical.fighter_mover_of(0).expect("a mover").clone();
        assert_eq!(mover.slots.iter().map(|(index, _)| *index).collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(mover.speed, 8.0);
        // From the first member's position, 8 x 0.25 toward the target.
        assert!((mover.anchor.z - 2.0).abs() < 1e-5, "{mover:?}");
        let second = original_fighter_slot_offset(2, 1.0);
        // The lead, within 2.0 of its place, heads along the mover's
        // velocity (0x5ebe30) and flies at its own speed, 12 x 0.25, not
        // the force's (0x5b8f70).
        assert!((tactical.fighters[0].source_position.z - 3.0).abs() < 1e-5);
        // The second flies toward its place, the anchor less its slot.
        let place = |t: &BattleSession| {
            let anchor = t.fighter_mover_of(0).expect("a mover").anchor;
            (anchor.x - second.x, anchor.y - second.y, anchor.z - second.z)
        };
        let gap = |t: &BattleSession| {
            let (x, y, z) = place(t);
            let at = t.fighters[1].source_position;
            (x - at.x).hypot(y - at.y).hypot(z - at.z)
        };
        let start = (place(&tactical).0 - 3.0).hypot(place(&tactical).1).hypot(place(&tactical).2);
        assert!(gap(&tactical) < start, "{:?}", tactical.fighters[1].source_position);
        // The mover of a squadron in another group is that group's.
        assert_eq!(tactical.fighter_mover_of(2).map(|mover| mover.group), Some(2));
        assert_eq!(tactical.fighter_mover_of(3).map(|mover| mover.alliance), Some(false));
        // A member gone frees the count: a new one takes slot 2 again.
        tactical.fighters[1].alive = false;
        tactical.fighters.push({
            let mut late = tactical.fighters[0].clone();
            late.source_position.x = -4.0;
            late
        });
        tactical.sync_fighter_movers();
        let mover = tactical.fighter_mover_of(0).expect("a mover");
        assert_eq!(mover.slots.len(), 2);
        assert_eq!(mover.slots[1], (4, second));
        // An emptied squadron leaves; a squadron with no sublight rating
        // still flies at its side's commander term, 1 (0x5ba190), which
        // then sets the force's speed.
        tactical.fighters[4].squad_count = 0;
        tactical.fighters[0].sublight_engine_power = 0.0;
        tactical.fighters[2].fighter_group = 0;
        tactical.update_force_movers();
        let mover = tactical.fighter_mover_of(0).expect("a mover");
        assert_eq!(mover.slots.iter().map(|(index, _)| *index).collect::<Vec<_>>(), vec![0, 2]);
        assert_eq!(mover.speed, 1.0);
        // The Alliance's group 2, now empty, loses its mover; the Empire's
        // group 0 keeps its own.
        assert!(!tactical.fighter_movers.iter().any(|mover| mover.alliance && mover.group == 2));
        assert!(tactical.fighter_movers.iter().any(|mover| !mover.alliance && mover.group == 0));
        assert_eq!(tactical.fighter_movers.len(), 2);
        // Group 1 on both sides: the Alliance's emptied, the Empire's kept.
        let mut ours = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        ours.fighter_group = 1;
        let mut theirs = fighter(false, TacticalWeaponArc::new(2, 0, 0));
        theirs.fighter_group = 1;
        let mut pair = session(Vec::new(), vec![ours, theirs]);
        pair.sync_fighter_movers();
        assert_eq!(pair.fighter_movers.len(), 2);
        pair.fighters[0].alive = false;
        pair.sync_fighter_movers();
        assert_eq!(pair.fighter_movers.len(), 1);
        assert!(!pair.fighter_movers[0].alliance);
    }

    // FUN_005c6610, order 3, on the group's mover: the anchor heads for its
    // own x and y at the exit line's z (0x005ec0d0, the arrival radius), its
    // members stand at their places, and the jumps are tried only once the
    // move has arrived (it returns 0).
    #[test]
    fn a_withdrawing_fighter_group_flies_on_its_anchor() {
        use crate::tactical::{TacticalTurnEvent, TacticalUnitRef};
        let wing = |z: f32| {
            let mut wing = fighter(true, TacticalWeaponArc::default());
            wing.has_hyperdrive = true;
            wing.retreating = true;
            // 3 plus the commander term 1: 4.
            wing.sublight_engine_power = 3.0;
            wing.source_position.z = z;
            wing
        };
        // An enemy far off keeps the battle going.
        let mut enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        enemy.source_position.z = 60.0;
        enemy.hull_current = 1_000_000;
        enemy.hull_max = 1_000_000;
        let mut tactical = session(vec![enemy], Vec::new());
        let exit = tactical.withdrawal_exit_z(true);
        // The lead far from the line, the second member already at it.
        tactical.fighters = vec![wing(exit + 30.0), wing(exit)];
        install_withdrawal_executors(&mut tactical);
        tactical.withdrawal_ms = Some(0.0);
        for _ in 0..12 {
            tactical.step();
        }
        let mover = tactical.fighter_mover_of(0).expect("a mover").clone();
        // One step of 4 x 0.25 from the lead's start toward the line.
        assert!((mover.anchor.z - (exit + 29.0)).abs() < 1e-4, "{mover:?}");
        // The member already at the line has no distance along z to fly
        // (force slot +0x18 on order 3), so it holds.
        assert_eq!(tactical.fighters[1].source_position.z, exit);
        assert_eq!(tactical.fighters[1].source_position.x, 0.0);
        // The anchor has not arrived, so no member jumps yet.
        assert!(!tactical.turn_events.iter().any(|queued| matches!(
            queued.event,
            TacticalTurnEvent::JumpStart(TacticalUnitRef::Fighter(_))
        )));
    }

    // 0x5b97a0: q = rate x ms / (angle x 1000); at 1 or more it snaps;
    // below, the level part turns by acos(level dot) x q, signed by
    // h.z f.x - h.x f.z, and the height moves q of the way. No dead band.
    #[test]
    fn a_squadron_turns_by_its_share_of_the_angle() {
        use crate::tactical::turn_original_fighter;
        let north = TacticalWorldVector::SOURCE_FORWARD;
        let east = vector(1.0, 0.0, 0.0);
        // pi/2 away at pi/2 a second for 1 s: q = 1, a snap.
        let snapped = turn_original_fighter(north, east, std::f32::consts::FRAC_PI_2, 1000.0);
        assert!(near(snapped, east), "{snapped:?}");
        // For 250 ms: q = 0.25, a level turn of pi/8, signed by
        // h.z f.x - h.x f.z = -1 < 0.
        let quarter = turn_original_fighter(north, east, std::f32::consts::FRAC_PI_2, 250.0);
        let (sin, cos) = (-std::f32::consts::FRAC_PI_8).sin_cos();
        assert!(near(quarter, vector(-sin, 0.0, cos)), "{quarter:?}");
        // The other way, toward -x, turns the other sign.
        let west = turn_original_fighter(north, vector(-1.0, 0.0, 0.0), std::f32::consts::FRAC_PI_2, 250.0);
        assert!(near(west, vector(sin, 0.0, cos)), "{west:?}");
        // Straight up: the height moves q of the way; the level part still
        // turns by acos(0) x q = pi/8 (the routine takes the level dot as
        // it stands), positive as the sign term is 0; then it renormalizes.
        let up = turn_original_fighter(north, vector(0.0, 1.0, 0.0), std::f32::consts::FRAC_PI_2, 250.0);
        let (sin, cos) = std::f32::consts::FRAC_PI_8.sin_cos();
        assert!(near(up, vector(-sin, 0.25, cos).normalized()), "{up:?}");
        // A tiny angle still turns (no dead band), and nothing turns at
        // rate 0.
        let slight = vector(0.01, 0.0, 1.0).normalized();
        let turned = turn_original_fighter(north, slight, 0.1, 10.0);
        assert!(turned.x > 0.0 && turned.x < slight.x, "{turned:?}");
        assert_eq!(turn_original_fighter(north, east, 0.0, 250.0), north);
    }

    // 0x5b8f70: a squadron moves along its own forward at its speed times
    // its alignment, never under 0.25 (DAT_0066c6c4), even facing away;
    // within 2.0 of its place (DAT_0066d864) its heading is its mover's
    // velocity (0x5ebe30).
    #[test]
    fn a_squadron_flies_its_forward_at_no_less_than_a_quarter_speed() {
        use crate::tactical::{TacticalAttackTarget, TacticalFighterFrame};
        let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        wing.sublight_engine_power = 7.0;
        wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        // Behind it, at -z, while it faces +z.
        target.source_position.z = -100.0;
        // 7 plus the commander term 1: 8.
        let mut tactical = session(vec![target], vec![wing]);
        tactical.advance_fighter_approach();
        // Facing away, it still moves forward: 8 x 0.25 x 0.25 along +z, and
        // starts its slow turn (pi/12 a second).
        let at = tactical.fighters[0].source_position;
        assert!(at.z > 0.0 && (at.z - 0.5).abs() < 0.05, "{at:?}");
        // Within 2.0 of its place, it heads along the mover's velocity.
        let mut close = session(
            vec![{
                let mut far = ship(false, TacticalWeaponArc::default(), 0.0);
                far.source_position.z = 100.0;
                far
            }],
            vec![{
                let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
                wing.sublight_engine_power = 7.0;
                wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
                wing
            }],
        );
        close.fighter_frames = vec![TacticalFighterFrame {
            forward: TacticalWorldVector::SOURCE_FORWARD,
            velocity: TacticalWorldVector::ZERO,
        }];
        close.advance_fighter_approach();
        // The anchor moved 2 along +z (8 x 0.25): within 2.0, so the heading
        // is the mover's velocity (+z), and it flies a full 2.
        let at = close.fighters[0].source_position;
        assert!((at.z - 2.0).abs() < 1e-5, "{at:?}");
        assert!((close.fighter_frames[0].velocity.z - 8.0).abs() < 1e-5);
    }

    // 0x5b97a0 and 0x5b8f70 off the axes, against values worked by hand
    // (outside the port's code) from the same routine: a turn from
    // (0.6, 0, 0.8), and a member flying a diagonal to its slot place.
    #[test]
    fn a_squadron_turns_and_flies_off_the_axes() {
        use crate::tactical::{
            turn_original_fighter, TacticalAttackTarget, TacticalFighterFrame, TacticalWorldPosition,
        };
        let from = vector(0.6, 0.0, 0.8);
        let up = turn_original_fighter(from, vector(0.0, 0.6, 0.8), 1.0, 250.0);
        assert!(near(up, vector(0.377_927_5, 0.168_720_66, 0.910_331_9)), "{up:?}");
        let left = turn_original_fighter(from, vector(-0.8, 0.0, 0.6), 0.5, 250.0);
        assert!(near(left, vector(0.495_578_8, 0.0, 0.868_563)), "{left:?}");
        // A turn whose sign the h.x f.z term decides, and turns with height.
        let right = turn_original_fighter(vector(0.8, 0.0, 0.6), vector(0.6, 0.0, 0.8), 0.1, 250.0);
        assert!(near(right, vector(0.784_751_6, 0.0, 0.619_810_4)), "{right:?}");
        let level = turn_original_fighter(vector(0.48, 0.6, 0.64), vector(0.0, 0.6, 0.8), 1.0, 250.0);
        assert!(near(level, vector(0.110_443_75, 0.6, 0.792_339_7)), "{level:?}");
        let dive = turn_original_fighter(vector(0.48, 0.6, 0.64), vector(0.0, -0.6, 0.8), 1.0, 250.0);
        assert!(near(dive, vector(0.400_447_25, 0.436_807_1, 0.805_507)), "{dive:?}");
        // A turn of nothing, or for no time, keeps the forward.
        assert_eq!(turn_original_fighter(from, from, 1.0, 250.0), from);
        assert_eq!(turn_original_fighter(from, vector(-0.8, 0.0, 0.6), 1.0, 0.0), from);

        let mut lead = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        // 7 and 9 plus the commander term 1: 8 and 10.
        lead.sublight_engine_power = 7.0;
        lead.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        let mut member = lead.clone();
        member.sublight_engine_power = 9.0;
        member.maneuverability = 4;
        member.source_position = TacticalWorldPosition {
            x: -6.0,
            y: 3.0,
            z: -4.0,
        };
        let mut target = ship(false, TacticalWeaponArc::default(), 0.0);
        target.source_position = TacticalWorldPosition {
            x: 40.0,
            y: 10.0,
            z: 60.0,
        };
        let mut tactical = session(vec![target], vec![lead, member]);
        tactical.fighter_frames = vec![
            TacticalFighterFrame {
                forward: TacticalWorldVector::SOURCE_FORWARD,
                velocity: TacticalWorldVector::ZERO,
            },
            TacticalFighterFrame {
                forward: from,
                velocity: TacticalWorldVector::ZERO,
            },
        ];
        tactical.advance_fighter_approach();
        let anchor = tactical.fighter_mover_of(0).expect("a mover").anchor;
        assert!((anchor.x - 1.098_884_5).abs() < 1e-4 && (anchor.z - 1.648_326_8).abs() < 1e-4);
        let velocity = tactical.fighter_frames[1].velocity;
        assert!(near_within(velocity, vector(7.469_824_6, -1.234_270_3, 6.409_224_7), 1e-3), "{velocity:?}");
        let at = tactical.fighters[1].source_position;
        assert!((at.x + 4.132_543_9).abs() < 1e-3, "{at:?}");
        assert!((at.y - 2.691_432_4).abs() < 1e-3, "{at:?}");
        assert!((at.z + 2.397_693_8).abs() < 1e-3, "{at:?}");

        // 0x5ebe30 at its edge: exactly 2.0 from its place it takes the
        // mover's velocity; just beyond, its distance.
        let mut mover = tactical.fighter_mover_of(0).expect("a mover").clone();
        mover.anchor = TacticalWorldPosition::ORIGIN;
        mover.velocity = vector(0.0, 0.0, 8.0);
        tactical.fighters[0].source_position = TacticalWorldPosition {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(tactical.fighter_place_delta(&mover, 0), vector(0.0, 0.0, 8.0));
        tactical.fighters[0].source_position.x = 2.5;
        assert_eq!(tactical.fighter_place_delta(&mover, 0), vector(-2.5, 0.0, 0.0));
        // A member's place is the anchor less its slot, on every axis.
        mover.anchor = TacticalWorldPosition {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        tactical.fighters[1].source_position = TacticalWorldPosition {
            x: 10.0,
            y: 10.0,
            z: 10.0,
        };
        let (_, offset) = mover.slots[1];
        let delta = tactical.fighter_place_delta(&mover, 1);
        assert!(near(delta, vector(1.0 - offset.x - 10.0, 2.0 - offset.y - 10.0, 3.0 - offset.z - 10.0)));
    }

    fn near_within(left: TacticalWorldVector, right: TacticalWorldVector, tolerance: f32) -> bool {
        (left.x - right.x).abs() < tolerance
            && (left.y - right.y).abs() < tolerance
            && (left.z - right.z).abs() < tolerance
    }

    // 0x5ba190 (slot +0x90 of the fighter class 0x66c028): the side's
    // commander term (FUN_005ad7e0) plus the sublight rating (+0x3f8), at
    // least 0; the Empire, with no flagged commander (FUN_0040c200), adds 1.
    #[test]
    fn a_squadron_s_speed_adds_its_commander_term() {
        let mut ours = fighter(true, TacticalWeaponArc::default());
        ours.sublight_engine_power = 6.0;
        let mut theirs = fighter(false, TacticalWeaponArc::default());
        theirs.sublight_engine_power = 6.0;
        let mut broken = fighter(false, TacticalWeaponArc::default());
        broken.sublight_engine_power = -3.0;
        let mut tactical = session(Vec::new(), vec![ours, theirs, broken]);
        tactical.alliance_commander_term = 5.0;
        assert_eq!(tactical.original_fighter_speed(0), 11.0);
        assert_eq!(tactical.original_fighter_speed(1), 7.0);
        assert_eq!(tactical.original_fighter_speed(2), 0.0);
    }

    // 0x5b8f70's band block: a squadron leaves a band it is inside, at the
    // step (speed x 0.4, DAT_0066c6b4), and otherwise keeps its y (no d.y
    // tracking, no limit, no blocked flag).
    #[test]
    fn a_squadron_only_steps_out_of_a_height_band() {
        use crate::tactical::original_fighter_band_step as band;
        let own = neighbour(10.0, 0.0, 9);
        assert_eq!(band(own, &[], 2.0), 10.0);
        // A ship above at 11 (half-height 2): ceiling 9; inside it, it steps
        // down by at most the step.
        assert_eq!(band(own, &[neighbour(11.0, 2.0, 1)], 0.5), 9.5);
        assert_eq!(band(own, &[neighbour(11.0, 2.0, 1)], 3.0), 9.0);
        // A ship below at 9 (half-height 2): floor 11; it steps up.
        assert_eq!(band(own, &[neighbour(9.0, 2.0, 1)], 0.5), 10.5);
        assert_eq!(band(own, &[neighbour(9.0, 2.0, 1)], 3.0), 11.0);
        // Outside both limits it keeps its y, even near them.
        assert_eq!(band(own, &[neighbour(13.0, 2.0, 1), neighbour(7.0, 2.0, 2)], 1.0), 10.0);
        // Pinned inside both: held.
        assert_eq!(band(own, &[neighbour(11.0, 2.0, 1), neighbour(9.0, 2.0, 2)], 1.0), 10.0);
    }

    // FUN_005cdbf0 for a squadron (0x5b9102): only the capital tree, the
    // squadron entered at its x; its reach is half its last velocity.
    #[test]
    fn a_squadron_s_neighbours_are_the_ships_near_it() {
        use crate::tactical::{OriginalTacticalCollisionEnvelope, TacticalFighterFrame, TacticalWorldPosition};
        let envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 9.0,
        });
        let at = |x: f32, y: f32, z: f32| {
            let mut unit = ship(true, TacticalWeaponArc::default(), 0.0);
            unit.source_collision_envelope = envelope;
            unit.source_position = TacticalWorldPosition { x, y, z };
            unit
        };
        // Each ship reaches 5; the squadron 0 at rest.
        let ships = vec![
            at(-4.0, 0.0, 0.0),
            at(4.5, 0.5, 0.0),
            at(6.0, 0.0, 0.0),
            at(3.0, 5.0, 0.0),
            // Exactly at the reach: within.
            at(-5.0, 0.0, 0.0),
            // Beyond a reach of 7.
            at(7.5, 0.0, 0.0),
        ];
        let mut wing = fighter(true, TacticalWeaponArc::default());
        wing.source_position = TacticalWorldPosition::ORIGIN;
        let other_wing = fighter(true, TacticalWeaponArc::default());
        let mut tactical = session(ships, vec![wing, other_wing]);
        let mut nearby = tactical.nearby_capitals_of_squadron(0, 1.0);
        nearby.sort_unstable();
        // 6 is beyond 5 in x and stops the walk; 3 at 5 high is beyond 1 +
        // 0.4; -5 lies exactly at the reach.
        assert_eq!(nearby, vec![0, 1, 4]);
        // Moving at 4 a second, its reach grows by half that, 2: 6 is within
        // 7, 7.5 beyond.
        tactical.fighter_frames = vec![
            TacticalFighterFrame {
                forward: TacticalWorldVector::SOURCE_FORWARD,
                velocity: vector(0.0, 0.0, 4.0),
            },
            TacticalFighterFrame {
                forward: TacticalWorldVector::SOURCE_FORWARD,
                velocity: TacticalWorldVector::ZERO,
            },
        ];
        let mut nearby = tactical.nearby_capitals_of_squadron(0, 1.0);
        nearby.sort_unstable();
        assert_eq!(nearby, vec![0, 1, 2, 4]);
    }

    // FUN_005cdbf0 for a squadron off the origin: the ships' reach takes
    // their velocity; destroyed, jumping and envelope-less ships are not in
    // the tree; the walk goes on past ships within reach on both sides.
    #[test]
    fn a_squadron_s_neighbour_walk_off_the_origin() {
        use crate::tactical::{OriginalTacticalCollisionEnvelope, TacticalWorldPosition};
        let envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 9.0,
        });
        let (ox, oy, oz) = (10.0, 20.0, -5.0);
        let at = |x: f32, y: f32, z: f32| {
            let mut unit = ship(true, TacticalWeaponArc::default(), 0.0);
            unit.source_collision_envelope = envelope;
            unit.source_position = TacticalWorldPosition {
                x: ox + x,
                y: oy + y,
                z: oz + z,
            };
            unit
        };
        let mut moving = at(7.0, 0.0, 0.0);
        // Reach 5 + 0.5 x 6: 8 beyond 7.
        moving.source_velocity = vector(6.0, 0.0, 0.0);
        let mut dead = at(1.0, 0.0, 0.0);
        dead.alive = false;
        let mut jumping = at(-1.0, 0.0, 0.0);
        jumping.retreat_progress = 0.5;
        let mut bare = at(2.0, 0.0, 0.0);
        bare.source_collision_envelope = None;
        let ships = vec![
            at(-3.0, 1.2, 4.0),  // 0: within on the left
            at(-4.9, -1.3, 0.0), // 1: within on the left, past 0
            at(3.0, 0.0, -4.9),  // 2: within on the right
            at(4.0, 0.0, 0.0),   // 3: within on the right, past 2
            moving,              // 4: within by its speed
            at(-6.0, 0.0, 0.0),  // 5: beyond on the left
            at(3.5, 1.5, 0.0),   // 6: too high for 1 + 0.4 x 1
            at(3.6, 0.0, 5.5),   // 7: too far in z
            dead,
            jumping,
            bare,
        ];
        let mut wing = fighter(true, TacticalWeaponArc::default());
        wing.source_position = TacticalWorldPosition { x: ox, y: oy, z: oz };
        let tactical = session(ships, vec![wing]);
        let mut nearby = tactical.nearby_capitals_of_squadron(0, 1.0);
        nearby.sort_unstable();
        assert_eq!(nearby, vec![0, 1, 2, 3, 4]);
        // A longer travel raises the vertical slack: 6 at 1.5 joins at 2.
        let mut nearby = tactical.nearby_capitals_of_squadron(0, 2.0);
        nearby.sort_unstable();
        assert_eq!(nearby, vec![0, 1, 2, 3, 4, 6]);
    }

    // 0x5b8f70's band at the same height: the squadron's id (after every
    // ship's, port) puts a level ship below it, so it steps up; the step
    // search travels speed x alignment, which here keeps a ship 1.15 below
    // in the band.
    #[test]
    fn a_squadron_level_with_a_ship_steps_up() {
        use crate::tactical::{OriginalTacticalCollisionEnvelope, TacticalAttackTarget, TacticalWorldPosition};
        let envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 1.0,
            planar_diameter: 9.0,
        });
        let at = |x: f32, y: f32, z: f32| {
            let mut unit = ship(false, TacticalWeaponArc::default(), 0.0);
            unit.source_collision_envelope = envelope;
            unit.source_position = TacticalWorldPosition { x, y, z };
            unit
        };
        let mut far = ship(false, TacticalWeaponArc::default(), 0.0);
        far.source_position.z = 300.0;
        let level_run = |wing_index: usize| {
            let mut wings = (0..=wing_index)
                .map(|offset| {
                    let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
                    wing.sublight_engine_power = 7.0;
                    wing.fighter_group = u8::try_from(offset).unwrap_or(0);
                    wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
                    wing.source_position = TacticalWorldPosition {
                        x: 100.0 * offset as f32,
                        y: 0.0,
                        z: 0.0,
                    };
                    wing
                })
                .collect::<Vec<_>>();
            wings[wing_index].source_position.x = 200.0;
            let mut tactical = session(
                vec![far.clone(), at(-500.0, 0.0, 0.0), at(200.0, 0.0, 0.0)],
                wings,
            );
            tactical.advance_fighter_approach();
            tactical.fighters[wing_index].source_position.y
        };
        // Floor 0 + 1 + 0 = 1: up by 8 x 0.4 x 0.25 = 0.8.
        assert!((level_run(0) - 0.8).abs() < 1e-4);
        assert!((level_run(2) - 0.8).abs() < 1e-4);
        // A ship 1.15 below is a neighbour for a travel of 2 (1 + 0.4 x 2
        // = 1.8) but not for 0.3 (1.12).
        let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        wing.sublight_engine_power = 7.0;
        wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
        let mut close = session(vec![far.clone(), at(0.0, -1.15, 0.0)], vec![wing]);
        close.fighter_frames = vec![crate::tactical::TacticalFighterFrame {
            forward: TacticalWorldVector::SOURCE_FORWARD,
            velocity: TacticalWorldVector::ZERO,
        }];
        close.advance_fighter_approach();
        // Floor -1.15 + 1 = -0.15, below its 0: not inside, y kept by the
        // band; its move along +z leaves y at 0.
        assert!(close.fighters[0].source_position.y.abs() < 1e-5);
        assert!(close.nearby_capitals_of_squadron(0, 2.0).contains(&1));
        assert!(!close.nearby_capitals_of_squadron(0, 0.3).contains(&1));
    }

    // 0x5b8f70: under a ship, inside its ceiling, a squadron steps down
    // while it flies; its 3D move follows.
    #[test]
    fn a_squadron_under_a_ship_steps_down_as_it_flies() {
        use crate::tactical::{OriginalTacticalCollisionEnvelope, TacticalAttackTarget, TacticalWorldPosition};
        let mut roof = ship(false, TacticalWeaponArc::default(), 0.0);
        roof.source_collision_envelope = Some(OriginalTacticalCollisionEnvelope {
            vertical_radius: 2.0,
            planar_diameter: 9.0,
        });
        roof.source_position = TacticalWorldPosition {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let mut far = ship(false, TacticalWeaponArc::default(), 0.0);
        far.source_position.z = 200.0;
        let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        // 7 plus the commander term 1: 8.
        wing.sublight_engine_power = 7.0;
        wing.attack_target = Some(TacticalAttackTarget::CapitalShip(1));
        let mut tactical = session(vec![roof, far], vec![wing]);
        tactical.advance_fighter_approach();
        // Ceiling 1 - 2 - 0 = -1 below its 0: down by 8 x 0.4 x 0.25 = 0.8.
        let at = tactical.fighters[0].source_position;
        assert!((at.y + 0.8).abs() < 1e-4, "{at:?}");
        assert!(at.z > 0.0);
    }

    // FUN_005ab650 0x005ab921 copies each unit's side vector (FUN_005b5ad0,
    // FUN_005be160 +0x48) into its forward: the Alliance faces +z, the
    // Empire -z, each toward the other's lane.
    #[test]
    fn each_side_starts_facing_the_other() {
        let mut ships = vec![
            ship(true, TacticalWeaponArc::default(), 0.0),
            ship(false, TacticalWeaponArc::default(), 0.0),
        ];
        let layout = crate::tactical::BattleSession::assign_original_world_positions(
            &mut ships,
            &mut Vec::new(),
        );
        let alliance = ships[0].source_forward;
        let empire = ships[1].source_forward;
        assert_eq!((alliance.x, alliance.y, alliance.z), (0.0, 0.0, 1.0));
        assert_eq!((empire.x, empire.y, empire.z), (0.0, 0.0, -1.0));
        assert!(ships[0].source_position.z < 0.0 && layout.outer_negative_z < 0.0);
        assert!(ships[1].source_position.z > 0.0);
    }

    // 0x005ccdda: a computer side's Stand Off force of more than 7 members,
    // out of its band, takes Surround (0x005c9be0(1), event 0x3d); one of
    // 7, or the player's, keeps Stand Off.
    #[test]
    fn a_large_computer_force_switches_to_surround() {
        use crate::tactical::{TacticalAttackTarget, TacticalTactic, TacticalTurnEvent};
        let run = |count: usize, player_side: bool| {
            let mut ships = (0..count)
                .map(|_| {
                    let mut unit = ship(player_side, TacticalWeaponArc::default(), 0.0);
                    unit.task_force = 3;
                    unit.order = TacticalOrder::AttackCapitalShips;
                    unit.source_desired_forward = crate::tactical::TacticalWorldVector::ZERO;
                    unit
                })
                .collect::<Vec<_>>();
            let mut target = ship(!player_side, TacticalWeaponArc::default(), 0.0);
            target.source_position.z = 80.0;
            target.task_force = 6;
            ships.push(target);
            for unit in &mut ships[..count] {
                unit.attack_target = Some(TacticalAttackTarget::CapitalShip(count));
            }
            let mut tactical = session(ships, Vec::new());
            tactical.advance_engagement_approach();
            let switched = tactical.ships[0].tactic == TacticalTactic::Surround;
            let posted = tactical.turn_events.iter().any(|queued| {
                queued.event
                    == TacticalTurnEvent::TaskForceTactic {
                        attacker: player_side,
                        task_force: 3,
                        tactic: TacticalTactic::Surround,
                    }
            });
            (switched, posted)
        };
        assert_eq!(run(8, false), (true, true));
        assert_eq!(run(7, false), (false, false));
        assert_eq!(run(8, true), (false, false));
    }

    // The fighter forces that close: not recovering, holding, withdrawing,
    // on the trench run, emptied or gone. An escort closes on its ship
    // (FUN_005c6610 on +0x8c; order 1's executor, 0x5ae1e0, is empty).
    #[test]
    fn only_free_fighter_forces_close_on_targets() {
        use crate::tactical::TacticalFighterRecoveryState;
        let wing = || fighter(true, TacticalWeaponArc::default());
        let mut wings = (0..9).map(|_| wing()).collect::<Vec<_>>();
        wings[1].order = TacticalOrder::Recover;
        wings[2].order = TacticalOrder::Escort;
        wings[3].order = TacticalOrder::HoldPosition;
        wings[4].retreating = true;
        wings[5].recovery_state = TacticalFighterRecoveryState::Returning;
        wings[6].squad_count = 0;
        wings[7].alive = false;
        wings[8].order = TacticalOrder::AttackDeathStar;
        let mut tactical = session(vec![ship(false, TacticalWeaponArc::default(), 0.0)], wings);
        tactical.fighters.push(wing());
        tactical.trench_run_participants = vec![9];
        let mut flying = wing();
        flying.retreat_progress = 1.0;
        tactical.fighters.push(flying);
        assert_eq!(tactical.fighter_forces(), [0, 2]);
    }

    // FUN_005cc300 closes a capital force on its +0x8c target alike for an
    // attack and an escort (FUN_005cb600 sets the escorted ship there): the
    // escort's waypoint is the band step toward its ship, as an attack on
    // the same ship would take, and its arrival radius is 2.0
    // (0x40000000).
    #[test]
    fn a_capital_escort_closes_on_its_ship_like_an_attack() {
        use crate::tactical::TacticalAttackTarget;
        let run = |escort: bool| {
            let mut guarded = ship(true, TacticalWeaponArc::default(), 0.0);
            guarded.source_position.z = 60.0;
            guarded.source_position.x = 3.0;
            guarded.task_force = 1;
            let mut escort_ship = ship(true, TacticalWeaponArc::new(0, 0, 4), 0.0);
            if escort {
                escort_ship.order = TacticalOrder::Escort;
                escort_ship.escort_target = Some(0);
            } else {
                escort_ship.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
            }
            let mut tactical = session(vec![guarded, escort_ship], Vec::new());
            // Surround's band (0 to 12.5) and step are the port's known ones.
            tactical.task_force_formations[1][0] = crate::tactical::TacticalTactic::Surround;
            tactical.update_engagement_bands();
            tactical.update_force_movers();
            tactical.advance_engagement_approach();
            tactical.ships[1].source_waypoint
        };
        let escort = run(true);
        assert!(escort.is_some());
        assert_eq!(escort, run(false));
    }

    // FUN_005cc300 / FUN_005c6610: when the escorted ship leaves the fight
    // the force latches +0xb0 and stops closing, but order 1 and its target
    // stay (only 0x5cb600 / 0x5cb4d0 clear them); an escort holds its fire
    // (FUN_005b3a40), so it stands.
    #[test]
    fn an_escort_keeps_its_order_when_its_ship_is_lost() {
        let mut guarded = ship(true, TacticalWeaponArc::default(), 0.0);
        guarded.source_position.z = 60.0;
        guarded.alive = false;
        guarded.task_force = 1;
        let mut escort_ship = ship(true, TacticalWeaponArc::new(0, 0, 4), 0.0);
        escort_ship.order = TacticalOrder::Escort;
        escort_ship.escort_target = Some(0);
        let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        wing.order = TacticalOrder::Escort;
        wing.escort_target = Some(0);
        let enemy = ship(false, TacticalWeaponArc::default(), 0.0);
        let mut tactical = session(vec![guarded, escort_ship, enemy], vec![wing]);
        tactical.sync_fighter_movers();
        let anchor = tactical.fighter_movers[0].anchor;
        for _ in 0..20 {
            tactical.step();
        }
        assert_eq!(tactical.ships[1].order, TacticalOrder::Escort);
        assert_eq!(tactical.ships[1].escort_target, Some(0));
        assert_eq!(tactical.fighters[0].order, TacticalOrder::Escort);
        assert_eq!(tactical.fighters[0].escort_target, Some(0));
        assert_eq!(tactical.fighter_movers[0].anchor, anchor);
        assert_eq!(tactical.ships[1].source_waypoint, None);
        // A friendly target is required: an enemy as the target clears.
        tactical.ships[1].escort_target = Some(2);
        crate::tactical::refresh_original_escort_targets(&mut tactical);
        assert_eq!(tactical.ships[1].order, TacticalOrder::None);
    }

    // FUN_005c6610 takes the escorted ship from +0x8c like any target: the
    // force's anchor steps toward it, and with a step within the move's
    // radius (2.0) it goes onto the circle round the ship (radius 1.5,
    // DAT_0066cb08, half of it above; angle 250 x pi x 0.0002).
    #[test]
    fn an_escorting_fighter_group_closes_on_and_circles_its_ship() {
        use crate::tactical::{TacticalAttackTarget, TacticalEngagementBand, TacticalUnitRef};
        let escort = |z: f32, step: Option<f32>| {
            let mut guarded = ship(true, TacticalWeaponArc::default(), 0.0);
            guarded.source_position.z = z;
            guarded.source_position.x = 3.0;
            let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
            wing.order = TacticalOrder::Escort;
            wing.escort_target = Some(0);
            let mut tactical = session(vec![guarded], vec![wing]);
            if let Some(step) = step {
                tactical.engagement_bands = vec![TacticalEngagementBand {
                    leader: TacticalUnitRef::Fighter(0),
                    min: 0.0,
                    max: 0.0,
                    step,
                }];
            }
            tactical.advance_fighter_approach();
            tactical
        };
        let attack = |z: f32| {
            let mut other = ship(true, TacticalWeaponArc::default(), 0.0);
            other.source_position.z = z;
            other.source_position.x = 3.0;
            let mut wing = fighter(true, TacticalWeaponArc::new(2, 0, 0));
            wing.attack_target = Some(TacticalAttackTarget::CapitalShip(0));
            let mut tactical = session(vec![other], vec![wing]);
            tactical.advance_fighter_approach();
            tactical
        };
        // Far off, it moves as an attack on the same ship would.
        let far = escort(60.0, None);
        let anchor = far.fighter_movers[0].anchor;
        assert!(anchor.z > 0.0 && anchor.x > 0.0, "{anchor:?}");
        assert_eq!(anchor, attack(60.0).fighter_movers[0].anchor);
        assert_ne!(far.fighters[0].source_position, TacticalWorldPosition::ORIGIN);
        // Close by, it goes onto the circle.
        let near_ship = escort(1.0, Some(1.0));
        let angle = 250.0 * std::f32::consts::PI * 0.000_2;
        let (sin, cos) = angle.sin_cos();
        let anchor = near_ship.fighter_movers[0].anchor;
        assert!((anchor.x - (3.0 + 1.5 * cos - 1.5 * sin)).abs() < 1e-5, "{anchor:?}");
        assert!((anchor.y - 0.75).abs() < 1e-5, "{anchor:?}");
        assert!((anchor.z - (1.0 + 1.5 * cos + 1.5 * sin)).abs() < 1e-5, "{anchor:?}");
        // Its band is measured on the escorted ship.
        let mut banded = escort(60.0, None);
        banded.update_engagement_bands();
        assert!(banded
            .engagement_bands
            .iter()
            .any(|band| band.leader == TacticalUnitRef::Fighter(0)));
        // Without a ship to escort, it stands.
        let mut lost = fighter(true, TacticalWeaponArc::new(2, 0, 0));
        lost.order = TacticalOrder::Escort;
        let mut tactical = session(vec![ship(true, TacticalWeaponArc::default(), 0.0)], vec![lost]);
        tactical.advance_fighter_approach();
        assert_eq!(tactical.fighters[0].source_position, TacticalWorldPosition::ORIGIN);
    }

    // FUN_005a8f70 ends a maneuver at its last point; an attack order
    // outlives its navigation route.
    #[test]
    fn an_attack_order_outlives_its_route() {
        use crate::tactical::TacticalWorldPosition;
        let mut tactical = duelists(40.0, 30.0);
        let point = TacticalWorldPosition {
            x: 0.0,
            y: 0.0,
            z: -49.0,
        };
        tactical.ships[0].navigation_route = vec![point];
        tactical.ships[0].source_waypoint = Some(point);
        step_until(&mut tactical, |t| t.ships[0].navigation_route.is_empty());
        assert_eq!(tactical.ships[0].order, TacticalOrder::AttackCapitalShips);
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
        install_withdrawal_executors(&mut tactical);
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
