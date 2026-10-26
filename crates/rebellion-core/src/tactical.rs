//! Tactical battle simulation: the state and step of one space battle.
//!
//! Moved unchanged from `rebellion-render::tactical_view` so a battle can be
//! stepped without display (`ghidra/notes/battle-flow.md`: `FUN_0059a530`
//! runs the same tactical module for an abstract battle). Drawing, input,
//! and the battle windows stay in the render crate.

use crate::ids::{CapitalShipKey, DatId, FighterKey, FleetKey, SystemKey};
use crate::pictures::planet_picture_id;
use crate::tactical_resources::{
    capital_ship_tactical_resource, death_star_tactical_resource, fighter_tactical_resource,
    TacticalCapitalShipResource, TacticalDeathStarResource, TacticalFighterResource,
    DEATH_STAR_TACTICAL_RESOURCE,
};
use crate::world::GameWorld;

/// Battlefield dimensions (logical units).
///
/// port: the 2D fallback battlefield; the original lays the battle out in 3D (`FUN_005ec400`).
pub const ARENA_WIDTH: f32 = 1200.0;
/// port: the 2D fallback battlefield height.
pub const ARENA_HEIGHT: f32 = 800.0;

/// Deployment zone width (fraction of arena width per side).
///
/// port: the 2D fallback deployment zone.
pub const DEPLOY_ZONE_FRACTION: f32 = 0.3;

/// `FUN_005e45f0` and `FUN_005e7540` place the five condition indicators at
/// panel-local x positions 10, 37, 64, 91, and 118 with y 103.
pub const TACTICAL_SUBSYSTEM_RESOURCE_BASES: [u32; 5] = [1201, 1206, 1211, 1216, 1221];
/// `_DAT_0066d154`, used by both `FUN_005d3cc0` and `FUN_005d41a0`.
pub const ORIGINAL_TACTICAL_EFFECT_FRAME_SECONDS: f32 = 0.1;
/// `FUN_005b1ea0` laser threshold (`ghidra/notes/tactical-projectile-field-path.md`).
pub const ORIGINAL_LASER_PROJECTILE_THRESHOLD: f32 = 28.8;
/// `FUN_005b1f60` turbolaser threshold.
pub const ORIGINAL_TURBOLASER_PROJECTILE_THRESHOLD: f32 = 34.666_668;
/// `FUN_005b2080` ion-cannon threshold.
pub const ORIGINAL_ION_PROJECTILE_THRESHOLD: f32 = 32.0;
/// `FUN_005b2c70` torpedo and fighter-attack threshold.
pub const ORIGINAL_TORPEDO_PROJECTILE_THRESHOLD: f32 = 12.8;
pub const ORIGINAL_PROJECTILE_DURATION_SHORT: f32 = 1.0;
/// `FUN_005ee590` variant three lives two seconds.
pub const ORIGINAL_PROJECTILE_DURATION_LONG: f32 = 2.0;
/// `FUN_005b0330` resets the subsystem-repair timer to 50 * 1000 ms; it
/// runs once a turn (`0x005a9de0`, slot `+0x9c`), so every 50 turns.
pub const ORIGINAL_SUBSYSTEM_REPAIR_INTERVAL_TURNS: u32 = 50;
/// `FUN_005b05c0` converts the CAPSHPSD sublight rating by 0.4 * 0.6.
pub const ORIGINAL_SUBLIGHT_ENGINE_SCALE: f32 = 0.24;
/// `FUN_005b9c60` converts the FIGHTSD sublight rating by 0.4 without the
/// capital-ship 0.6 multiplier.
pub const ORIGINAL_FIGHTER_SUBLIGHT_ENGINE_SCALE: f32 = 0.4;
/// Tactical combat advances four authoritative simulation steps per second.
///
/// port: the port steps four times per second; `FUN_005cd640` integrates whatever elapsed milliseconds it is given.
pub const ORIGINAL_TACTICAL_STEP_MILLISECONDS: f32 = 250.0;
/// `FUN_005ad750` and `FUN_005afb70` clamp the active mode contribution.
pub const ORIGINAL_ENGINE_MODE_MIN_BONUS: f32 = 1.0;
/// `FUN_005ad750` and `FUN_005afb70` upper clamp.
pub const ORIGINAL_ENGINE_MODE_MAX_BONUS: f32 = 9.0;
/// `_DAT_0066d088`, consumed by `FUN_005cf980` before recovery state 3.
pub const ORIGINAL_FIGHTER_DOCKING_DISTANCE: f32 = 2.0;
/// `FUN_005ba270` clamps the effective capital maneuverability at nine.
pub const ORIGINAL_CAPITAL_MANEUVERABILITY_MAX: f32 = 9.0;
/// The fallback turn rate installed by `FUN_005b9c60` is pi / 12 radians/s.
pub const ORIGINAL_CAPITAL_TURN_RATE_FALLBACK: f32 = std::f32::consts::PI / 12.0;
/// `FUN_005d2360` initializes every capital-ship weapon family with this
/// event-strength coefficient.
pub const ORIGINAL_CAPITAL_WEAPON_STRENGTH: f32 = f32::from_bits(0x3e88_8889);
/// `FUN_005d24e0` makes laser cannon fire deliberately weak against capital
/// ships. Its exact immediate is `0x3e2b020c`.
pub const ORIGINAL_LASER_VS_CAPITAL: f32 = f32::from_bits(0x3e2b_020c);
/// `FUN_005b05c0` converts the CAPSHPSD weapon recharge rating by 3.75.
pub const ORIGINAL_WEAPON_RECHARGE_SCALE: f32 = 3.75;
/// One fighter craft contributes eight floating hull points.
///
/// Live craft = ceil(hull / 8) in the fighter receivers (`FUN_005b5100`; `ghidra/notes/tactical-fighter-combat.md`).
pub const ORIGINAL_FIGHTER_HULL_PER_CRAFT: f32 = 8.0;
/// `FUN_005b9c00` scales a fighter group's live hull into weapon output.
pub const ORIGINAL_FIGHTER_WEAPON_HULL_SCALE: f32 = 3.751;
/// `FUN_005b9c30` scales live fighter hull into shield recharge per second.
pub const ORIGINAL_FIGHTER_SHIELD_HULL_SCALE: f32 = 0.025;
/// `FUN_005b23a0` applies this coefficient to the FIGHTSD torpedo rating.
pub const ORIGINAL_FIGHTER_TORPEDO_STRENGTH: f32 = f32::from_bits(0x3f08_8889);
/// `FUN_005d2490` and `FUN_005d24e0` target modifiers.
pub const ORIGINAL_ION_VS_FIGHTER: f32 = 0.0;
pub const ORIGINAL_LASER_VS_FIGHTER: f32 = 1.0;
/// `FUN_005d2490` and `FUN_005d24e0` turbolaser modifier.
pub const ORIGINAL_TURBOLASER_VS_FIGHTER: f32 = 0.5;
/// port: ORIGINAL_TACTICAL_STEP_MILLISECONDS in seconds.
pub const ORIGINAL_TACTICAL_STEP_SECONDS: f32 = ORIGINAL_TACTICAL_STEP_MILLISECONDS / 1000.0;
/// The battle manager's event turn, `+0x8a8 = 0x3e8` ms (`0x005bea9c`,
/// `0x005c0545`). At each turn the manager advances `+0x878` and runs the
/// events stamped `+0x878 - 1` (`0x005bec3c`, `FUN_005a96b0`).
///
/// port: the turn boundary falls on every fourth 250 ms step, counted from
/// the first combat step.
pub const ORIGINAL_TACTICAL_TURN_STEPS: u32 = 4;
/// `+0x8ac = 3` (`0x005be233`, `0x005bf929`): a new event is stamped
/// `+0x87c`, which runs this many turns ahead of `+0x878` (`0x005a877b`,
/// slot `+0x28` = `0x005ad4a0`) in both managers, `FUN_005be160` and
/// `FUN_005bf830`.
pub const ORIGINAL_TACTICAL_EVENT_LEAD_TURNS: u32 = 3;
/// The time a turn's updates are given: the turn length `+0x8a8` (1000 ms)
/// for the commanders (`0x005bed3e`), the task forces (`0x005a9d40`) and
/// the units (`0x005a9de0`, which scales it by `DAT_0066c26c` = 0.001 for
/// the recharge `FUN_005b6320`).
pub const ORIGINAL_TACTICAL_TURN_MILLISECONDS: f32 = 1000.0;
/// `DAT_0066c294` (and `DAT_0066c290` = -2/3), read by `0x005a9ec0`: the
/// field's half-width on each axis, as a share of the battle extent
/// `+0x8cc`.
pub const ORIGINAL_FIELD_LIMIT_EXTENT_SCALE: f32 = 2.0 / 3.0;
/// port: ORIGINAL_TACTICAL_TURN_MILLISECONDS in seconds.
pub const ORIGINAL_TACTICAL_TURN_SECONDS: f32 = ORIGINAL_TACTICAL_TURN_MILLISECONDS / 1000.0;

/// `FUN_005ad530` sets the withdrawing side's time (`+0x7c`) to at least
/// `DAT_0066c240`, 30000 ms; `FUN_005c8780` counts it down by the step and
/// ends the withdrawal (state 2 -> 3) at 0.
pub const ORIGINAL_WITHDRAWAL_MILLISECONDS: f32 = 30_000.0;
/// `DAT_0066c234`: the Empire's withdrawal time while its Death Star is
/// operational (`FUN_005a9330`, `FUN_005ba420`).
pub const ORIGINAL_DEATH_STAR_WITHDRAWAL_MILLISECONDS: f32 = 150_000.0;
/// `FUN_005b1b70` for a tactical ship: an installed hyperdrive that is not
/// knocked out (`FUN_005b1bc0`) and engines with power (slot `+0x90`,
/// [`original_tactical_mobility`]).
pub fn original_ship_hyperdrive_works(
    ships: &[TacticalShip],
    field_effects: &[TacticalFieldEffect],
    index: usize,
) -> bool {
    ships.get(index).is_some_and(|ship| {
        ship.subsystem_capacity.hyperdrive > 0 && ship.subsystem_condition.hyperdrive > 0
    }) && original_tactical_mobility(ships, field_effects, index)
        .is_some_and(|mobility| mobility.effective_engine_power != 0.0)
}

/// `FUN_005b1b70` for a fighter group: a hyperdrive and engine power (port:
/// its sublight rating).
fn original_fighter_hyperdrive_works(fighter: &TacticalFighter) -> bool {
    fighter.has_hyperdrive && fighter.sublight_engine_power != 0.0
}

/// `FUN_005be160` gives the tactical battle's Alliance commander (side 0,
/// `+0x08`) the vector `+0x48` = (0, 0, 1) and the Empire's (`+0x0c`)
/// (0, 0, -1); `FUN_005af5f0` gives the abstract ones the same. `FUN_005cc300` heads a
/// withdrawing capital-ship force for it times `-1.0` (`_DAT_0066cff8`) and
/// half (`_DAT_0066cffc`) the battle extent `+0x8cc`.
pub const ORIGINAL_WITHDRAWAL_EXIT_EXTENT_SCALE: f32 = 0.5;
/// `_DAT_0066cff0` in `FUN_005cc300`: after the side's time a unit within
/// 8.0 of its exit line (along the side vector) starts its jump.
pub const ORIGINAL_JUMP_EXIT_DISTANCE: f32 = 8.0;
/// `FUN_005c6610` (`0x005c66b7`) and `FUN_005cc300` (`0x005cc3ba`) pass 1.0
/// as the arrival radius of the force's move (the mover's slot `+0xc`:
/// `0x005ec0d0`, or a formation's `0x005eb1a0` / `0x005eb540`): within it the
/// move ends (returns 0), and only then are the members' jumps tried.
pub const ORIGINAL_WITHDRAWAL_ARRIVAL_RADIUS: f32 = 1.0;
/// `_DAT_0066c24c`: `FUN_005a6b40` ends the jump (state 7 -> 8) when its
/// clock `+0x50` reaches 4000 ms; event `0x2e` then removes the unit.
pub const ORIGINAL_JUMP_MILLISECONDS: f32 = 4_000.0;
/// `FUN_005a6b40` picks the jump factor by `(type + id) & 3`: the unit's
/// vtable `+0x24` (its tactical ordinal, the index `FUN_005ab0e0` uses for
/// the `+0xa50` resource table) plus its object id `+0x3b`. hyp: the port's
/// list index stands for the object id.
#[must_use]
pub const fn original_jump_factor_index(tactical_ordinal: u8, object_index: usize) -> usize {
    (tactical_ordinal as usize + object_index) & 3
}

/// `FUN_005a6b40` draws a jumping unit at `z - side_z * k * (0.001 t)^2`.
/// The four factors `k` are set at start-up (`0x5a69b0`..`0x5a6a7c`) as a
/// distance over the squared seconds left of the 4000 ms jump:
/// `DAT_006bc47c` = 400 / 4^2, `DAT_006bc488` = 575 / 3.75^2,
/// `DAT_006bc484` = 650 / 3.925^2 and `DAT_006bc494` = 325 / 3.725^2, picked
/// by `(id + slot +0x24) & 3` in that order.
pub const ORIGINAL_JUMP_ACCELERATIONS: [f32; 4] = [
    400.0 / (4.0 * 4.0),
    575.0 / (3.75 * 3.75),
    650.0 / (3.925 * 3.925),
    325.0 / (3.725 * 3.725),
];

/// How far a jump of `factor` draws its unit from `from` ms to `to` ms of
/// its clock `+0x50` (`FUN_005a6b40`).
fn original_jump_travel(factor: f32, from: f32, to: f32) -> f32 {
    let (from, to) = (from * 0.001, to * 0.001);
    factor * (to * to - from * from)
}
/// `FUN_005c8780`: a tactical commander reviews the battle when its count
/// `+0x40`, falling by 1.0 a call (`_DAT_0066cbd4`), reaches 0, then sets
/// it to 6.0 (`DAT_0066cba4`). It is called once a turn (slot `+0x80`,
/// `0x005a9d10`), so it reviews every sixth turn.
pub const ORIGINAL_COMMANDER_REVIEW_TURNS: u32 = 6;
/// `FUN_005ba7f0` advances the tactical Death Star laser by elapsed
/// milliseconds times one third times 0.001, capped at 100.
pub const ORIGINAL_DEATH_STAR_LASER_CHARGE_PER_SECOND: f32 = 1.0 / 3.0;
/// `FUN_005ba7f0` caps the charge at 100.
pub const ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE: f32 = 100.0;
/// The Death Star shield's family (DEFFACSD 4, TEXTSTRA 8832; `FUN_0051c0c0`).
pub const DEATH_STAR_SHIELD_FAMILY: u8 = 0x25;
/// `FUN_005cfec0` arms the Death Star attack state with a 120,000 ms window.
pub const ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS: f32 = 120_000.0;
/// port: the frame in which the trench run's approach is flown, a 60 Hz
/// frame. The original flies it each rendered frame (`0x005a9ff8`).
pub const PORT_MOVER_FRAME_MILLISECONDS: f32 = 1000.0 / 60.0;
/// The approach move's arrival radius, 0.2 (`FUN_005c6610` pushes
/// `0x3e4ccccd` at `0x005c68f0` for the mover's slot `+0xc`).
pub const ORIGINAL_TRENCH_RUN_APPROACH_RADIUS: f32 = 0.2;
/// `_DAT_0066d078` scales the maneuver-failure damage draw in `FUN_005d04e0`.
pub const ORIGINAL_TRENCH_RUN_DAMAGE_SCALE: f32 = 8.0;
/// `FUN_005d04e0` silent chatter slot.
pub const ORIGINAL_TRENCH_RUN_SILENT_EVENT: u16 = 0x13d;
/// `FUN_005d04e0` success chatter, one slot per 10% of the timer.
pub const ORIGINAL_TRENCH_RUN_SUCCESS_CHATTER: [u16; 9] = [
    0x11e, 0x11f, 0x120, 0x128, 0x121, 0x122, 0x129, 0x13d, 0x13d,
];
/// `FUN_005d04e0` damaged-success chatter.
pub const ORIGINAL_TRENCH_RUN_DAMAGED_SUCCESS_CHATTER: [u16; 9] = [
    0x11e, 0x123, 0x124, 0x125, 0x12a, 0x12b, 0x126, 0x13d, 0x13d,
];
/// `FUN_005d04e0` failure chatter.
pub const ORIGINAL_TRENCH_RUN_FAILURE_CHATTER: [u16; 9] = [
    0x11e, 0x123, 0x124, 0x125, 0x12a, 0x12b, 0x126, 0x12c, 0x127,
];

/// Spacing between auto-placed ships.
///
/// port: 2D fallback placement spacing.
pub const SHIP_SPACING: f32 = 60.0;

/// `FUN_005ab650` starts the battle extent `+0x8cc` at 100.0
/// (`0x005ab683`, `0x42c80000`).
pub const ORIGINAL_BATTLE_BASE_EXTENT: f32 = 100.0;
/// `FUN_005ab650` counts each side's units as it places them, leaving out
/// those in state 1 (`0x005b69f0`), and subtracts `DAT_0066c2a0` (-3.0)
/// from the extent each time a side's count passes the other's
/// (`0x005abb11`..`0x005abb4b`): 3.0 a unit of the larger side.
///
/// port: what puts a unit in state 1 at placement is not traced; every
/// living ship and fighter group with craft counts.
pub const ORIGINAL_BATTLE_OBJECT_INCREMENT: f32 = 3.0;
/// `FUN_005ab650` places each side's capital ships (list `+0x58`) at
/// `z = extent * 0.5` (`DAT_0066c29c`) for the Empire and `* -0.5`
/// (`DAT_0066c2a8`) for the Alliance (`0x005abb66`..`0x005abc5d`).
pub const ORIGINAL_BATTLE_OUTER_LANE_SCALE: f32 = 0.5;
/// `FUN_005ab650` places the fighter groups (list `+0x64`) 20.0
/// (`DAT_0066c2ac`) inside their side's capital lane: `20 - 0.5 extent`
/// for the Alliance, `0.5 extent - 20` for the Empire
/// (`0x005abcc6`..`0x005abd71`).
pub const ORIGINAL_BATTLE_INNER_LANE_OFFSET: f32 = 20.0;
/// `FUN_005ab650`'s x slots: an even unit turns the last x round (times
/// `DAT_0066c27c`, -1.0); an odd one widens the slot by 5.0
/// (`-DAT_0066c2a4`) on the last x's side: 0, 5, -5, -10, 10, 15, ...
pub const ORIGINAL_BATTLE_SLOT_SPACING: f32 = 5.0;
/// `FUN_005c81d0` multiplies maximum weapon range by 10,000 before adding
/// available sublight power and negating the ordered-list key.
pub const ORIGINAL_FORMATION_RANGE_PRIORITY: f32 = 10_000.0;
/// `FUN_005ea250` expands each member footprint by 1.2 before retaining the
/// largest value for `FUN_005ec400`'s alternating follower slots.
pub const ORIGINAL_FORMATION_FOOTPRINT_SCALE: f32 = 1.2;

/// port: a ship's `task_force` when its force holds none of the eight group
/// slots (`+0x164`). `FUN_005aded0` gives a ship such a force, and every
/// such force holds that one ship: a single selected unit's order
/// (`0x005a1a60`, `0x005a1ac0`), a selection with no free slot
/// (`0x005a2c70`, `0x005a2a40`), and each withdrawing ship
/// (`FUN_005ad530`).
pub const TACTICAL_UNGROUPED_TASK_FORCE: u8 = u8::MAX;

/// A completion array's index for a task force: the eight group slots,
/// then one for the forces without a slot. hyp: their completion lines
/// take the generic variant, as the destruction lines do.
pub(crate) fn task_force_slot(task_force: u8) -> usize {
    if task_force == TACTICAL_UNGROUPED_TASK_FORCE {
        8
    } else {
        usize::from(task_force.min(7))
    }
}

/// port: the slot index back to a `task_force`, `TACTICAL_UNGROUPED_TASK_FORCE`
/// for the ninth.
pub(crate) fn task_force_of_slot(slot: usize) -> u8 {
    u8::try_from(slot)
        .ok()
        .filter(|slot| *slot < 8)
        .unwrap_or(TACTICAL_UNGROUPED_TASK_FORCE)
}

/// A capital family's event offset: its generic line for a force without a
/// group slot, as `FUN_005a5cf0` uses `0x55`/`0x62` when `0x005a0300` finds
/// none, else the slot's line after it.
pub(crate) const fn capital_group_offset(task_force_one: u16, group: u8) -> u16 {
    if group == TACTICAL_UNGROUPED_TASK_FORCE {
        task_force_one - 1
    } else {
        task_force_one + if group < 7 { group as u16 } else { 7 }
    }
}

/// Source-coordinate tactical envelope recovered from `FUN_005ab650`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalTacticalLayout {
    pub first_active_objects: u16,
    pub second_active_objects: u16,
    pub battle_extent: f32,
    pub outer_positive_z: f32,
    pub outer_negative_z: f32,
    pub inner_negative_z: f32,
    pub inner_positive_z: f32,
}

impl OriginalTacticalLayout {
    pub fn from_active_counts(first_active_objects: usize, second_active_objects: usize) -> Self {
        let first_active_objects = u16::try_from(first_active_objects).unwrap_or(u16::MAX);
        let second_active_objects = u16::try_from(second_active_objects).unwrap_or(u16::MAX);
        let widest_force = first_active_objects.max(second_active_objects);
        let battle_extent = ORIGINAL_BATTLE_OBJECT_INCREMENT
            .mul_add(f32::from(widest_force), ORIGINAL_BATTLE_BASE_EXTENT);
        let outer_positive_z = battle_extent * ORIGINAL_BATTLE_OUTER_LANE_SCALE;
        let outer_negative_z = -outer_positive_z;
        let inner_negative_z = ORIGINAL_BATTLE_INNER_LANE_OFFSET - outer_positive_z;
        let inner_positive_z = outer_positive_z - ORIGINAL_BATTLE_INNER_LANE_OFFSET;
        Self {
            first_active_objects,
            second_active_objects,
            battle_extent,
            outer_positive_z,
            outer_negative_z,
            inner_negative_z,
            inner_positive_z,
        }
    }
}

impl Default for OriginalTacticalLayout {
    fn default() -> Self {
        Self::from_active_counts(0, 0)
    }
}

/// Build the four far-to-clustered navigation sets described by the original
/// manual. Their interaction contract is authoritative; the deterministic
/// coordinates are A1 fixtures scaled to the recovered battle envelope until
/// the executable's point producer and A0 positions can be compared.
pub fn original_navigation_point_sets(
    layout: OriginalTacticalLayout,
) -> [Vec<TacticalWorldPosition>; 4] {
    let extent = layout.battle_extent.max(ORIGINAL_BATTLE_BASE_EXTENT);
    let point = |x: f32, y: f32, z: f32, scale: f32| TacticalWorldPosition {
        x: x * extent * scale,
        y: y * extent * scale,
        z: z * extent * scale,
    };
    [
        vec![
            point(-0.82, 0.22, -0.78, 1.0),
            point(0.78, -0.18, -0.70, 1.0),
            point(-0.70, -0.25, 0.75, 1.0),
            point(0.84, 0.20, 0.68, 1.0),
            point(0.05, 0.48, -0.92, 1.0),
            point(-0.12, -0.44, 0.90, 1.0),
        ],
        vec![
            point(-0.72, 0.18, -0.58, 0.82),
            point(0.66, -0.16, -0.52, 0.82),
            point(-0.58, -0.22, 0.62, 0.82),
            point(0.70, 0.20, 0.55, 0.82),
            point(0.02, 0.36, 0.04, 0.82),
        ],
        vec![
            point(-0.52, 0.15, -0.42, 0.62),
            point(0.48, -0.12, -0.36, 0.62),
            point(-0.42, -0.15, 0.44, 0.62),
            point(0.50, 0.14, 0.40, 0.62),
            point(0.0, 0.26, 0.0, 0.62),
        ],
        vec![
            point(-0.24, 0.10, -0.20, 0.42),
            point(0.22, -0.08, -0.18, 0.42),
            point(-0.18, -0.10, 0.22, 0.42),
            point(0.24, 0.09, 0.20, 0.42),
        ],
    ]
}

/// Stable production identity for one battle participant. Slotmap keys remain
/// runtime-only; the original DAT identity and fleet roster index survive any
/// tactical render reordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalObjectIdentity {
    pub class_dat_id: DatId,
    pub fleet_roster_index: usize,
    pub is_alliance: bool,
}

/// Initial retained-mode world coordinate recovered from the original battle
/// setup. The existing 2D simulation coordinates remain separate until its
/// movement and scale rules are recovered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalWorldPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Source retained-mode direction or velocity vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalWorldVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl TacticalWorldVector {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// `FUN_005b0f70` initializes the current vector with `FUN_0059fb10`.
    pub const SOURCE_FORWARD: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    pub fn normalized(self) -> Self {
        let magnitude = self.x.hypot(self.y).hypot(self.z);
        if magnitude > f32::EPSILON {
            Self {
                x: self.x / magnitude,
                y: self.y / magnitude,
                z: self.z / magnitude,
            }
        } else {
            Self::ZERO
        }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x
            .mul_add(other.x, self.y.mul_add(other.y, self.z * other.z))
    }
}

impl TacticalWorldPosition {
    pub const ORIGIN: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct OriginalTacticalSlots {
    pub index: usize,
    pub magnitude: f32,
    pub current: f32,
}

impl Default for OriginalTacticalSlots {
    fn default() -> Self {
        Self {
            index: 0,
            magnitude: 0.0,
            current: 0.0,
        }
    }
}

impl Iterator for OriginalTacticalSlots {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index & 1 == 0 {
            self.current = -self.current;
        } else {
            let prior_was_negative = self.current < 0.0;
            self.magnitude += ORIGINAL_BATTLE_SLOT_SPACING;
            self.current = if prior_was_negative {
                -self.magnitude
            } else {
                self.magnitude
            };
        }
        self.index = self.index.saturating_add(1);
        Some(self.current)
    }
}

/// Current phase of the tactical battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePhase {
    /// Player positions ships in deployment zone.
    Placement,
    /// Real-time combat in progress (Task #8).
    Combat,
    /// Battle over, showing results (Task #10).
    Results,
}

/// One command from the original unified tactical order field at source offset `+0x68`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TacticalOrder {
    #[default]
    None,
    /// Direct right-click protection order. The selected units follow one
    /// friendly capital ship and keep autonomous opportunity fire enabled.
    Escort,
    Recover,
    AttackFighters,
    AttackCapitalShips,
    AttackDeathStar,
    LeftHook,
    RightHook,
    Hammer,
    Anvil,
    HoldPosition,
}

/// Target identity retained by the original tactical attack-order executors.
/// Capital ships and fighter groups share one source object list, while the
/// reconstruction keeps their typed Rust collections separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalAttackTarget {
    CapitalShip(usize),
    FighterGroup(usize),
}

impl TacticalOrder {
    /// Exact numeric value consumed by the original tactical runtime.
    ///
    /// `FUN_005ca6d0` dispatches these order codes.
    #[must_use]
    pub const fn source_code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Escort => 1,
            Self::Recover => 2,
            Self::AttackFighters => 4,
            Self::AttackCapitalShips => 5,
            Self::AttackDeathStar => 6,
            Self::LeftHook => 7,
            Self::RightHook => 8,
            Self::Hammer => 9,
            Self::Anvil => 10,
            Self::HoldPosition => 11,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "No Orders",
            Self::Escort => "Escort",
            Self::Recover => "Recover",
            Self::AttackFighters => "Attack Fighters",
            Self::AttackCapitalShips => "Attack Capital Ships",
            Self::AttackDeathStar => "Attack Death Star",
            Self::LeftHook => "Left Hook",
            Self::RightHook => "Right Hook",
            Self::Hammer => "Hammer",
            Self::Anvil => "Anvil",
            Self::HoldPosition => "Hold Position",
        }
    }
}

/// The source tactical behavior toggle at offset `+0x38`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TacticalTactic {
    Surround,
    #[default]
    StandOff,
}

impl TacticalTactic {
    /// hyp: Surround 1 / Stand Off 2 per the P58F9 evidence (`2026-09-17-tactical-command-assignment.md`); the `+0x38` writer is not re-located.
    #[must_use]
    pub const fn source_code(self) -> u8 {
        match self {
            Self::Surround => 1,
            Self::StandOff => 2,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Surround => "Surround",
            Self::StandOff => "Stand Off",
        }
    }
}

// ---------------------------------------------------------------------------
// Ship instance (per-hull in the battle)
// ---------------------------------------------------------------------------

/// One individual ship hull participating in the battle.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "These independent flags preserve the existing state and serialization model."
)]
pub struct TacticalShip {
    /// Which capital ship class this hull belongs to.
    pub class_key: CapitalShipKey,
    /// Stable DAT and roster identity for production-to-render joins.
    pub identity: TacticalObjectIdentity,
    /// Exact original tactical graphic identity, when this is an original class.
    pub tactical_resource: Option<TacticalCapitalShipResource>,
    /// Separate original Death Star resource path, when applicable.
    pub death_star_resource: Option<TacticalDeathStarResource>,
    /// Original retained-mode position at battle initialization.
    pub source_position: TacticalWorldPosition,
    /// Current retained-mode forward direction at offsets `+0x10` through `+0x18`.
    pub source_forward: TacticalWorldVector,
    /// Desired retained-mode direction at offsets `+0x20` through `+0x28`.
    pub source_desired_forward: TacticalWorldVector,
    /// Latest source velocity stored at offsets `+0x6c` through `+0x74`.
    pub source_velocity: TacticalWorldVector,
    /// Active executor waypoint assigned through `FUN_005a8f70`.
    pub source_waypoint: Option<TacticalWorldPosition>,
    /// Ordered navigation points assigned through the tactical display. The
    /// first entry is mirrored by `source_waypoint` while it is active.
    pub navigation_route: Vec<TacticalWorldPosition>,
    /// Mesh-derived source collision volume installed by `FUN_005ab0e0`.
    /// It becomes available when the participant's close/base mesh family is
    /// decoded and remains invariant across visual LOD changes.
    pub source_collision_envelope: Option<OriginalTacticalCollisionEnvelope>,
    /// Display name (from `CapitalShipClass`).
    pub name: String,
    /// Position on the battlefield (logical coords).
    pub x: f32,
    pub y: f32,
    /// Current hull points.
    pub hull_current: i32,
    /// Maximum hull points.
    pub hull_max: i32,
    /// Shield strength (0-100 scale).
    pub shield: i32,
    pub shield_max: i32,
    /// Shield, weapon, tractor, engine, and hyperdrive condition percentages.
    /// Values are independent of the live hull and shield-energy meters.
    pub subsystem_condition: TacticalSubsystemCondition,
    /// Source component availability used to derive the five condition values.
    pub subsystem_capacity: TacticalSubsystemCapacity,
    /// Source damage counters. The first four cap at four hits; hyperdrive
    /// caps at the number of installed normal and damaged-drive components.
    pub subsystem_damage: TacticalSubsystemDamage,
    /// Normalized maximum sublight power stored at source offset `+0x3f8`.
    pub sublight_engine_power: f32,
    /// Raw `CAPSHPSD.DAT` maneuverability consumed by `FUN_005ba270`.
    pub maneuverability: u32,
    /// Maximum tractor power stored at source offset `+0x3b8`.
    pub tractor_beam_power: f32,
    /// Number of fighter squadrons this hull can recover.
    pub fighter_capacity: u32,
    /// Active state-record value at offset `+0x4c`, or no active record.
    pub maneuver_state_value: Option<f32>,
    /// Inclusive 1 through 100 repair chance used by `FUN_005b1490`.
    pub damage_control: u8,
    /// True if this ship belongs to the attacker side.
    pub is_attacker: bool,
    /// True if the ship is still alive.
    pub alive: bool,
    /// Whether this ship is currently selected by the player.
    pub selected: bool,
    /// Unified source order code assigned through the command panel.
    pub order: TacticalOrder,
    /// Source Surround or Stand Off behavior assigned with a maneuver.
    pub tactic: TacticalTactic,
    /// Zero-based task-force assignment shown in the eight source HUD slots,
    /// or `TACTICAL_UNGROUPED_TASK_FORCE` for a force of its own.
    pub task_force: u8,
    /// The strategic fleet the hull came from: a battle takes every fleet
    /// of both sides at the system (`FUN_0040c780`).
    pub fleet: FleetKey,
    /// Index into the fleet's `capital_ships` for damage application.
    pub fleet_ship_index: usize,
    /// Sprite resource ID in TACTICAL.DLL (if known).
    pub sprite_id: Option<u32>,
    /// Four source weapon records in executable order: fore, starboard, aft,
    /// and port. Each record owns readiness and its current energy reserve.
    pub weapon_arcs: [TacticalWeaponArc; 4],
    /// Laser, ion, and turbolaser engagement ranges from CAPSHPSD.DAT.
    pub weapon_ranges: TacticalWeaponRanges,
    /// Base energy restored per source second after the constructor's 3.75
    /// conversion.
    pub weapon_recharge_rate: f32,
    /// Base shield energy restored per source second.
    pub shield_recharge_rate: f32,
    /// Fractional recharge retained while the public shield meter remains an
    /// integer value.
    pub shield_recharge_carry: f32,
    /// Unique FIFO of fired arcs, matching `FUN_005b64e0` and `FUN_005b6320`.
    pub weapon_recharge_queue: Vec<u8>,
    /// Active target selected by a source attack-order executor or focus input.
    pub attack_target: Option<TacticalAttackTarget>,
    /// Player-authored target list. The original completes navigation points
    /// before attacking these targets in selection order.
    pub manual_targets: Vec<TacticalAttackTarget>,
    /// Friendly capital-ship index assigned by the original direct Escort
    /// command (order code 1), or no active escort target.
    pub escort_target: Option<usize>,
    /// Personnel and troops assigned to this physical hull for the selected-
    /// ship contents monitor.
    pub contents: Vec<TacticalShipContent>,
    /// True if this ship withdraws with its side (`FUN_005ad530` gives it
    /// order 3).
    pub retreating: bool,
    /// The jump's clock in milliseconds (`+0x50`, `FUN_005a6b40`); above 0
    /// once the ship has started its jump (states 6 and 7).
    pub retreat_progress: f32,
    /// True if this ship successfully retreated (survived, not destroyed).
    pub retreated: bool,
}

/// Capital-ship collision volume consumed by `FUN_005b2e60`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalTacticalCollisionEnvelope {
    pub vertical_radius: f32,
    pub planar_diameter: f32,
}

/// One compact GOKRES-backed entry in the selected ship's contents monitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TacticalShipContent {
    pub label: String,
    pub resource_id: u32,
}

/// Resolve the compact 61x25 GOKRES assignment image used by original system
/// and tactical contents windows. These are separate from the 80x80 officer
/// portraits and preserve the executable's major/minor resource bands.
#[must_use]
pub fn tactical_character_content_resource(dat_id: DatId, is_major: bool) -> Option<u32> {
    crate::pictures::character_mini_resource_id(dat_id, is_major)
}

/// One of the four 0x74-byte capital-ship battery records initialized by
/// `FUN_005b0e50` and consumed by `FUN_005b3f10`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TacticalWeaponArc {
    pub laser_cannons: u32,
    pub ion_cannons: u32,
    pub turbolasers: u32,
    pub ready: bool,
    pub energy: f32,
    pub energy_capacity: f32,
}

impl TacticalWeaponArc {
    pub fn new(laser_cannons: u32, ion_cannons: u32, turbolasers: u32) -> Self {
        let energy_capacity = (laser_cannons + ion_cannons + turbolasers) as f32;
        Self {
            laser_cannons,
            ion_cannons,
            turbolasers,
            ready: energy_capacity > 0.0,
            energy: energy_capacity,
            energy_capacity,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TacticalWeaponRanges {
    pub laser_cannon: f32,
    pub ion_cannon: f32,
    pub turbolaser: f32,
}

/// Source fighter recovery state stored at object offset `+0x35c`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TacticalFighterRecoveryState {
    #[default]
    AwaitingCarrier,
    Reserved,
    Returning,
    Docking,
    Recovered,
}

impl TacticalFighterRecoveryState {
    /// `FUN_005ba2f0`/`FUN_005cf940` state 0, `FUN_005b84e0` 1, `FUN_005b1e10` 2, `FUN_005cf980` 3, `FUN_005b8630` 4.
    #[must_use]
    pub const fn source_code(self) -> u8 {
        match self {
            Self::AwaitingCarrier => 0,
            Self::Reserved => 1,
            Self::Returning => 2,
            Self::Docking => 3,
            Self::Recovered => 4,
        }
    }
}

/// The five selected-capital condition values consumed by the original HUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalSubsystemCondition {
    pub shields: u8,
    pub weapons: u8,
    pub tractor: u8,
    pub engines: u8,
    pub hyperdrive: u8,
}

/// Installed subsystem components passed to the original tactical constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalSubsystemCapacity {
    pub shields: bool,
    pub weapons: bool,
    pub tractor: bool,
    pub engines: bool,
    pub hyperdrive: u8,
}

impl TacticalSubsystemCapacity {
    /// Hit limits of the routine at `0x005b1970` (`FUN_005b1970`): four per component, the hyperdrive count from `FUN_005b05c0`.
    #[must_use]
    pub const fn hit_limits(self) -> [u8; 5] {
        [
            if self.shields { 4 } else { 0 },
            if self.weapons { 4 } else { 0 },
            if self.tractor { 4 } else { 0 },
            if self.engines { 4 } else { 0 },
            self.hyperdrive,
        ]
    }
}

/// Per-subsystem hit counters used by the original combat object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TacticalSubsystemDamage {
    pub shields: u8,
    pub weapons: u8,
    pub tractor: u8,
    pub engines: u8,
    pub hyperdrive: u8,
}

impl TacticalShip {
    /// The ship has started its jump out (states 6, 7 and 8): it no longer
    /// fires or can be targeted (`FUN_005ad870`, `FUN_005cdf50`). Until
    /// then a withdrawing ship stays in the fight.
    #[must_use]
    pub fn jumping(&self) -> bool {
        self.retreat_progress > 0.0
    }

    /// `FUN_0040cfc0`: the unit's hit counts as the ship's `+0x64` counters.
    /// The hyperdrive's hits fill the primary (one, if the class has a
    /// `hyperdrive`, `FUN_00500680`), then the backup (one, if it has a
    /// `hyperdrive_if_damaged`, `FUN_00500690`).
    #[must_use]
    pub fn strategic_hits(&self, class: &crate::world::CapitalShipClass) -> u32 {
        use crate::world::Subsystem;
        let damage = self.subsystem_damage;
        let primary = damage.hyperdrive.min(u8::from(class.hyperdrive > 0));
        let backup = (damage.hyperdrive - primary).min(u8::from(class.hyperdrive_if_damaged > 0));
        let mut ship = crate::world::ShipInstance::new(self.class_key, 0, false);
        for (subsystem, hits) in [
            (Subsystem::Shield, damage.shields),
            (Subsystem::Weapon, damage.weapons),
            (Subsystem::Tractor, damage.tractor),
            (Subsystem::Speed, damage.engines),
            (Subsystem::PrimaryHyperdrive, primary),
            (Subsystem::BackupHyperdrive, backup),
        ] {
            ship.set_hits(subsystem, hits);
        }
        ship.subsystem_hits
    }
}

impl TacticalSubsystemDamage {
    #[must_use]
    pub const fn hits(self) -> [u8; 5] {
        [
            self.shields,
            self.weapons,
            self.tractor,
            self.engines,
            self.hyperdrive,
        ]
    }

    #[must_use]
    pub const fn total_hits(self) -> u16 {
        self.shields as u16
            + self.weapons as u16
            + self.tractor as u16
            + self.engines as u16
            + self.hyperdrive as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalSubsystemKind {
    Shields,
    Weapons,
    Tractor,
    Engines,
    Hyperdrive,
}

impl TacticalSubsystemKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Shields => "shields",
            Self::Weapons => "weapons",
            Self::Tractor => "tractor",
            Self::Engines => "engines",
            Self::Hyperdrive => "hyperdrive",
        }
    }
}

/// One source-selected subsystem repair applied during live tactical combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalSubsystemRepair {
    pub ship: usize,
    pub kind: TacticalSubsystemKind,
    pub hits_before: u8,
    pub hits_after: u8,
}

/// Effective sublight state after engine damage and active tractor sources.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalSubsystemMobility {
    pub base_engine_power: f32,
    pub engine_mode_bonus: f32,
    pub active_tractor_power: f32,
    pub effective_engine_power: f32,
    pub engine_percent: u8,
}

impl TacticalSubsystemCondition {
    #[must_use]
    pub const fn percentages(self) -> [u8; 5] {
        [
            self.shields,
            self.weapons,
            self.tractor,
            self.engines,
            self.hyperdrive,
        ]
    }

    #[must_use]
    pub fn resource_ids(self) -> [u32; 5] {
        let percentages = self.percentages();
        std::array::from_fn(|index| {
            TACTICAL_SUBSYSTEM_RESOURCE_BASES[index]
                + u32::from(original_tactical_condition_band(percentages[index]))
        })
    }

    pub fn from_source_state(
        hull_current: i32,
        hull_max: i32,
        capacity: TacticalSubsystemCapacity,
        damage: TacticalSubsystemDamage,
    ) -> Self {
        fn hull_bound_percent(available: bool, hull_current: i32, hull_max: i32, hits: u8) -> u8 {
            if !available || hull_max <= 0 {
                return 0;
            }
            let hull_percent = hull_current.max(0) as f32 * 100.0 / hull_max as f32;
            (hull_percent - f32::from(hits) * 25.0).clamp(0.0, 100.0) as u8
        }

        let hyperdrive = if capacity.hyperdrive == 0 {
            0
        } else {
            let remaining = capacity.hyperdrive.saturating_sub(damage.hyperdrive);
            (f32::from(remaining) * 100.0 / f32::from(capacity.hyperdrive)) as u8
        };
        Self {
            shields: hull_bound_percent(capacity.shields, hull_current, hull_max, damage.shields),
            weapons: hull_bound_percent(capacity.weapons, hull_current, hull_max, damage.weapons),
            tractor: hull_bound_percent(capacity.tractor, hull_current, hull_max, damage.tractor),
            // FUN_005b17f0 does not apply the hull ratio to sublight power.
            // Active tractor sources and maneuver mode are applied by the
            // session-wide refresh after this local baseline is formed.
            engines: if capacity.engines {
                100_u8.saturating_sub(damage.engines.saturating_mul(25))
            } else {
                0
            },
            hyperdrive,
        }
    }
}

impl TacticalShip {
    pub fn refresh_subsystem_condition(&mut self) {
        self.subsystem_condition = TacticalSubsystemCondition::from_source_state(
            self.hull_current,
            self.hull_max,
            self.subsystem_capacity,
            self.subsystem_damage,
        );
    }

    /// `FUN_005b1970` (vtable `+0xa0`) increments one hit counter up to its limit.

    pub fn add_subsystem_hit(&mut self, kind: TacticalSubsystemKind) -> bool {
        match kind {
            TacticalSubsystemKind::Shields => {
                self.subsystem_damage.shields =
                    self.subsystem_damage.shields.saturating_add(1).min(4);
            }
            TacticalSubsystemKind::Weapons => {
                self.subsystem_damage.weapons =
                    self.subsystem_damage.weapons.saturating_add(1).min(4);
                if self.subsystem_damage.weapons == 4 {
                    self.disable_weapon_arcs();
                }
            }
            TacticalSubsystemKind::Tractor => {
                self.subsystem_damage.tractor =
                    self.subsystem_damage.tractor.saturating_add(1).min(4);
            }
            TacticalSubsystemKind::Engines => {
                self.subsystem_damage.engines =
                    self.subsystem_damage.engines.saturating_add(1).min(4);
            }
            TacticalSubsystemKind::Hyperdrive => {
                self.subsystem_damage.hyperdrive = self
                    .subsystem_damage
                    .hyperdrive
                    .saturating_add(1)
                    .min(self.subsystem_capacity.hyperdrive);
            }
        }
        self.refresh_subsystem_condition();
        kind == TacticalSubsystemKind::Tractor && self.subsystem_condition.tractor == 0
    }

    /// port: queue every battery for recharge when weapons reach four hits (`FUN_005b1970`); index 3 bounds the four batteries of `FUN_005b0e50`.

    pub fn disable_weapon_arcs(&mut self) {
        for (index, arc) in self.weapon_arcs.iter_mut().enumerate() {
            if arc.energy_capacity <= 0.0 {
                continue;
            }
            arc.ready = false;
            arc.energy = 0.0;
            let index = u8::try_from(index).unwrap_or(3);
            if !self.weapon_recharge_queue.contains(&index) {
                self.weapon_recharge_queue.push(index);
            }
        }
    }

    pub fn hull_ratio(&self) -> f32 {
        if self.hull_max <= 0 {
            0.0
        } else {
            self.hull_current.max(0) as f32 / self.hull_max as f32
        }
    }

    /// `FUN_005b1770` hull scaling less 25% per weapons hit (`FUN_005b1680`).

    pub fn effective_weapon_recharge(&self) -> f32 {
        (self.hull_ratio() * self.weapon_recharge_rate
            - self.weapon_recharge_rate * 0.25 * f32::from(self.subsystem_damage.weapons))
        .max(0.0)
    }

    /// `FUN_005b1770` hull scaling less 25% per shield hit (`FUN_005b1710`).

    pub fn effective_shield_recharge(&self) -> f32 {
        (self.hull_ratio() * self.shield_recharge_rate
            - self.shield_recharge_rate * 0.25 * f32::from(self.subsystem_damage.shields))
        .max(0.0)
    }

    pub fn advance_original_recharge(&mut self, seconds: f32) {
        if !self.alive || seconds <= 0.0 {
            return;
        }

        if self.shield < self.shield_max {
            self.shield_recharge_carry += self.effective_shield_recharge() * seconds;
            let whole = self.shield_recharge_carry.floor() as i32;
            if whole > 0 {
                let before = self.shield;
                self.shield = (self.shield + whole).min(self.shield_max);
                self.shield_recharge_carry -= (self.shield - before) as f32;
                if self.shield == self.shield_max {
                    self.shield_recharge_carry = 0.0;
                }
            }
        } else {
            self.shield_recharge_carry = 0.0;
        }

        let mut recharge = self.effective_weapon_recharge() * seconds;
        for &arc_index in &self.weapon_recharge_queue {
            if recharge <= 0.0 {
                break;
            }
            let Some(arc) = self.weapon_arcs.get_mut(usize::from(arc_index)) else {
                continue;
            };
            let missing = (arc.energy_capacity - arc.energy).max(0.0);
            let restored = recharge.min(missing);
            arc.energy += restored;
            recharge -= restored;
            if arc.energy >= arc.energy_capacity {
                arc.energy = arc.energy_capacity;
                arc.ready = arc.energy_capacity > 0.0;
            }
        }
        self.weapon_recharge_queue.retain(|&arc_index| {
            self.weapon_arcs
                .get(usize::from(arc_index))
                .is_some_and(|arc| !arc.ready)
        });
    }

    pub fn remove_subsystem_hit(&mut self, kind: TacticalSubsystemKind) -> Option<(u8, u8)> {
        let hits = match kind {
            TacticalSubsystemKind::Shields => &mut self.subsystem_damage.shields,
            TacticalSubsystemKind::Weapons => &mut self.subsystem_damage.weapons,
            TacticalSubsystemKind::Tractor => &mut self.subsystem_damage.tractor,
            TacticalSubsystemKind::Engines => &mut self.subsystem_damage.engines,
            TacticalSubsystemKind::Hyperdrive => &mut self.subsystem_damage.hyperdrive,
        };
        if *hits == 0 {
            return None;
        }
        let before = *hits;
        *hits -= 1;
        let after = *hits;
        self.refresh_subsystem_condition();
        Some((before, after))
    }
}

/// `FUN_005e77c0` quantizes a condition percentage to resource offsets 0–4.

pub fn original_tactical_condition_band(percent: u8) -> u8 {
    match percent {
        0 => 0,
        1..=24 => 1,
        25..=49 => 2,
        50..=74 => 3,
        75..=u8::MAX => 4,
    }
}

/// One fighter squadron in the battle.
#[derive(Debug, Clone)]
pub struct TacticalFighter {
    pub class_key: FighterKey,
    /// Stable DAT and roster identity for production-to-render joins.
    pub identity: TacticalObjectIdentity,
    /// Exact original side-dependent tactical graphic identity.
    pub tactical_resource: Option<TacticalFighterResource>,
    /// Original retained-mode position at battle initialization.
    pub source_position: TacticalWorldPosition,
    /// Normalized maximum sublight power stored at source offset `+0x3f8`.
    pub sublight_engine_power: f32,
    /// The strategic fleet the squadron came from.
    pub fleet: FleetKey,
    /// Index into the originating fleet's aggregate fighter roster. Classes
    /// may repeat in separate slots.
    pub fleet_fighter_index: usize,
    /// Zero-based squadron within the aggregate strategic roster entry.
    pub fleet_squadron_index: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    /// Live craft count derived from the source floating hull pool.
    pub squad_count: u32,
    /// Source fighter hull pool. Every craft contributes exactly eight points.
    pub hull_current: f32,
    pub hull_max: f32,
    /// Source fighter shield pool and its constructor maximum.
    pub shield: f32,
    pub shield_max: f32,
    /// Fore-only battery record used by all eight original FIGHTSD classes.
    pub weapon_arc: TacticalWeaponArc,
    /// Laser, ion, and turbolaser ranges promoted from FIGHTSD.DAT.
    pub weapon_ranges: TacticalWeaponRanges,
    /// FIGHTSD torpedo rating and range. The rating is not consumed as ammo.
    pub torpedo_strength: u32,
    pub torpedo_range: f32,
    /// Raw FIGHTSD maneuverability consumed by the fighter damage ratio.
    pub maneuverability: u32,
    /// The group has a hyperdrive: `FUN_005b1bc0` (installed `+0x3ec` less
    /// hits `+0x374`) is above 0. hyp: installed counts the FIGHTSD primary
    /// and backup ratings as for a capital ship; port: fighters take no
    /// hyperdrive hits.
    pub has_hyperdrive: bool,
    pub is_attacker: bool,
    pub alive: bool,
    /// Zero-based red, blue, green, or gold fighter group; 4 is unassigned.
    pub fighter_group: u8,
    /// Fighter groups are selected as a unit; individual fighters are not.
    pub selected: bool,
    /// Unified source order code assigned through the command panel.
    pub order: TacticalOrder,
    /// Source Surround or Stand Off behavior assigned with a maneuver.
    pub tactic: TacticalTactic,
    /// Active target selected by a source attack-order executor.
    pub attack_target: Option<TacticalAttackTarget>,
    /// Friendly capital-ship index assigned by the direct Escort command.
    pub escort_target: Option<usize>,
    /// Current `FUN_005cf980` recovery executor state.
    pub recovery_state: TacticalFighterRecoveryState,
    /// Capital-ship index reserved as this squadron's recovery carrier.
    pub recovery_target: Option<usize>,
    /// True if the group withdraws with its side (`FUN_005ad530` gives a
    /// group with a hyperdrive order 3).
    pub retreating: bool,
    /// The jump's clock in milliseconds (`FUN_005a6b40`); above 0 once the
    /// group has started its jump.
    pub retreat_progress: f32,
}

impl TacticalFighter {
    /// The group has started its jump out: like a jumping ship
    /// ([`TacticalShip::jumping`]) it no longer fires or can be targeted
    /// (`FUN_005ad870`, `FUN_005cdf50`).
    #[must_use]
    pub fn jumping(&self) -> bool {
        self.retreat_progress > 0.0
    }

    pub fn effective_maneuverability(&self) -> f32 {
        (self.maneuverability as f32 + 1.0).clamp(1.0, ORIGINAL_CAPITAL_MANEUVERABILITY_MAX)
    }

    pub fn weapon_hull_factor(&self) -> f32 {
        self.hull_current.max(0.0) * ORIGINAL_FIGHTER_WEAPON_HULL_SCALE
            / ORIGINAL_FIGHTER_HULL_PER_CRAFT
    }

    pub fn shield_recharge_rate(&self) -> f32 {
        self.hull_current.max(0.0) * ORIGINAL_FIGHTER_SHIELD_HULL_SCALE
            / ORIGINAL_FIGHTER_HULL_PER_CRAFT
    }

    pub fn refresh_craft_count(&mut self) {
        self.hull_current = self.hull_current.clamp(0.0, self.hull_max.max(0.0));
        self.squad_count = if self.hull_current <= 0.0 {
            0
        } else {
            (self.hull_current / ORIGINAL_FIGHTER_HULL_PER_CRAFT).ceil() as u32
        };
        if self.squad_count == 0 {
            self.alive = false;
            self.shield = 0.0;
            self.weapon_arc.ready = false;
            self.weapon_arc.energy = 0.0;
        }
    }

    pub fn advance_original_recharge(&mut self, seconds: f32) {
        if !self.alive || seconds <= 0.0 {
            return;
        }
        if self.shield < self.shield_max {
            self.shield =
                (self.shield + self.shield_recharge_rate() * seconds).min(self.shield_max);
        }

        if !self.weapon_arc.ready && self.weapon_arc.energy_capacity > 0.0 {
            self.weapon_arc.energy = (self.weapon_arc.energy + self.weapon_hull_factor() * seconds)
                .min(self.weapon_arc.energy_capacity);
            if self.weapon_arc.energy >= self.weapon_arc.energy_capacity {
                self.weapon_arc.energy = self.weapon_arc.energy_capacity;
                self.weapon_arc.ready = true;
            }
        }
    }

    pub fn consume_weapon_arc(&mut self) {
        self.weapon_arc.energy = 0.0;
        self.weapon_arc.ready = false;
    }
}

/// The Death Star is a manager-owned tactical object rather than a member of
/// the 29-entry capital mesh table. `FUN_005ab0e0` and `FUN_005ab650` select
/// its 5010 or 5020 sprite independently from normal capital participants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalDeathStar {
    pub resource: TacticalDeathStarResource,
    pub is_attacker: bool,
    pub is_alliance: bool,
    pub source_position: TacticalWorldPosition,
    /// Source field `+0x68`; zero makes `FUN_005ba420` non-operational.
    pub hull: f32,
    /// Source field `+0x6c`, filled by `FUN_005ba7f0` from zero to 100.
    pub laser_charge: f32,
    /// Source field `+0x70`: the battle's system holds a usable Death Star
    /// shield (DEFFACSD 4, family `0x25`), counted by `FUN_0040bcd0`
    /// (`FUN_0051c270`) and passed through `FUN_00595ef0` -> `FUN_005af4e0`.
    pub shielded: bool,
    /// Source field `+0x74`; selects resource 5020 when set.
    pub destroyed: bool,
    /// Source field `+0x78`; set when the Death Star action is committed.
    pub action_committed: bool,
}

/// One committed superlaser shot. `0x005ba720` posts two events through the
/// manager's turn queue:
/// - event `0x44`, stamped `+0x87c`, whose handler `0x005ba520` starts the
///   beam (`0x005c12d0`);
/// - a type-3 event, stamped `+0x87c + 1.0` (`DAT_0066c70c`), whose handler
///   `0x005b2b60` destroys the target if it is still in the fight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalDeathStarBeam {
    pub target: usize,
    pub source_position: TacticalWorldPosition,
    pub target_position: TacticalWorldPosition,
    /// Set once the beam has started.
    pub fired: bool,
    /// Seconds since the beam started.
    pub elapsed: f32,
}

/// A unit that a queued event names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalUnitRef {
    Ship(usize),
    Fighter(usize),
}

/// One weapon family's shot, decided by `FUN_005b3f10` in the turn's unit
/// update: its fire event (`0x0d`..`0x10`) is stamped `+0x87c + 1.0`, its
/// hit event (`0x11`..`0x14`) `+0x87c + 2.0` (`0x005b40c3`..`0x005b41ae`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalShot {
    pub source: TacticalUnitRef,
    pub target: OriginalFighterTarget,
    pub kind: WeaponKind,
    pub strength: f32,
}

/// An event of the battle manager's turn queue, by its dispatcher type
/// (`0x005a9780`, table `0x5a9bc4`).
#[derive(Debug, Clone, PartialEq)]
pub enum TacticalTurnEvent {
    /// `0x44`, `0x005ba520`: the superlaser's beam starts (`0x005c12d0`).
    DeathStarBeam { target: usize },
    /// Type 3 from the Death Star, `0x005b2b60`: the target dies.
    DeathStarKill { target: usize },
    /// `0x2d`, `FUN_005b1e40`: a unit still in the fight starts its jump
    /// flight (state 7), or is destroyed if its hyperdrive no longer works
    /// (`FUN_005b2720`).
    JumpStart(TacticalUnitRef),
    /// `0x2e`, `FUN_005b26d0` -> `FUN_005a90c0`: the jumped unit is gone
    /// (state 9).
    JumpEnd(TacticalUnitRef),
    /// `0x3c`, `FUN_005caf20` -> `FUN_005caf50` -> `FUN_005cfec0`: the
    /// trench run's result state, posted by `FUN_005cfe80`: 2 (the
    /// approach) from the run's creation (`0x005d0884`), 5 (the run) on
    /// the approach's arrival (`0x005c6928`).
    TrenchRunState(u8),
    /// `0x0d`..`0x10` (`FUN_005b1ea0`, `FUN_005b1f60`, `FUN_005b2080`,
    /// `FUN_005b2c70`): with both units in the fight, the projectile
    /// (`0x005c12d0`).
    WeaponFire(TacticalShot),
    /// `0x11`..`0x14` (`0x005b2a20`, `0x005b2a70`, `0x005b2ac0`,
    /// `0x005b2b10`): with the target in the fight, its damage (slot
    /// `+0xe0`).
    WeaponHit(TacticalShot),
    /// `0x1e`..`0x22` (`0x005b28d0`, `0x005b28a0`, `0x005b2870`,
    /// `0x005b2840`, `0x005b2900`): the ship's slot `+0xa0` with the
    /// subsystem 0..4 (shields, weapons, tractor, engines, hyperdrive).
    SubsystemHit { ship: usize, kind: TacticalSubsystemKind },
    /// `0xa`, posted by the task force's `SetOrder` (`0x005ca530`, at
    /// `0x005ca688`) with the order it was given; its handler
    /// (`0x005caba0`) calls slot `+0x48`, `FUN_005ca6d0`, which installs
    /// that order's executor. One event carries the units given the order
    /// together.
    OrderExecutor {
        order: TacticalOrder,
        capitals: Vec<usize>,
        fighters: Vec<usize>,
    },
    /// `0xa` with order 3 from the withdrawal (`FUN_005ad530`,
    /// `0x005ad60f` and `0x005ad6b8`): `FUN_005ca6d0` installs the
    /// withdrawal executor (`FUN_005cc300` for a capital ship's force,
    /// `FUN_005c6610` for a fighter group).
    WithdrawalExecutor(Vec<TacticalUnitRef>),
    /// `0x3d`, posted by SetTactic (`0x005c9be0`): its handler
    /// (`0x005c9c50`) rebuilds a capital force's mover with the tactic's
    /// formation (`0x005cb1a0`).
    TaskForceTactic {
        attacker: bool,
        task_force: u8,
        tactic: TacticalTactic,
    },
    /// `0x36`, posted by `FUN_005a8fc0` when a commander links a task force
    /// to a target: its handler (`FUN_005cae10`, slot `+0x1c` =
    /// `FUN_005cb600`) replaces the force's live target list `+0x8c`.
    TaskForceTarget {
        units: Vec<TacticalUnitRef>,
        target: TacticalAttackTarget,
    },
}

/// One queued event and its turn stamp (`+0x20`, `0x005ce500`).
#[derive(Debug, Clone, PartialEq)]
pub struct TacticalQueuedEvent {
    pub stamp: u32,
    pub event: TacticalTurnEvent,
}

impl TacticalDeathStar {
    #[must_use]
    pub fn operational(self) -> bool {
        !self.destroyed && self.hull > 0.0 && !self.action_committed
    }

    /// `FUN_005ba450`: fighters may attack it, operational and with no
    /// Death Star shield on its system (manual p. 124: "the only way to
    /// protect a Death Star from attack by Alliance ships and fighters").
    #[must_use]
    pub fn attackable(self) -> bool {
        self.operational() && !self.shielded
    }

    #[must_use]
    pub fn resource_id(self) -> u32 {
        if self.destroyed {
            self.resource.resource_base_with_flag
        } else {
            self.resource.resource_base_without_flag
        }
    }

    pub fn advance_laser_charge(&mut self, seconds: f32) {
        if self.hull > 0.0 && !self.destroyed && seconds > 0.0 {
            self.laser_charge = (self.laser_charge
                + seconds * ORIGINAL_DEATH_STAR_LASER_CHARGE_PER_SECOND)
                .min(ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE);
        }
    }
}

// ---------------------------------------------------------------------------
// BattleSession
// ---------------------------------------------------------------------------

/// All state for one tactical combat encounter.
#[derive(Debug, Clone)]
pub struct BattleSession {
    /// System where the battle takes place.
    pub system: SystemKey,
    pub system_name: String,
    /// Original `SYSTEMSD.picture_id`; selects the active tactical palette.
    pub system_picture_id: u8,
    /// Attacker fleet key (in `GameWorld`).
    pub attacker_fleet: FleetKey,
    /// Defender fleet key (in `GameWorld`).
    pub defender_fleet: FleetKey,
    /// Faction identity is independent of the attacking role.
    pub attacker_is_alliance: bool,
    /// True if the player controls the attacker side.
    pub player_is_attacker: bool,
    /// Current battle phase.
    pub phase: BattlePhase,
    /// Individual ship hulls on the battlefield.
    pub ships: Vec<TacticalShip>,
    /// Fighter squadrons on the battlefield.
    pub fighters: Vec<TacticalFighter>,
    /// Separate manager-owned Death Star state, when either fleet carries it.
    pub death_star: Option<TacticalDeathStar>,
    /// Active superlaser beam and delayed target resolution.
    pub death_star_beam: Option<TacticalDeathStarBeam>,
    /// The manager's turn queue (`+0x908` per side, `FUN_005a96b0`).
    /// port: one queue in posting order for both sides.
    pub turn_events: Vec<TacticalQueuedEvent>,
    /// The withdrawing units whose order-3 executor is installed
    /// (`+0x64` = 3, `FUN_005ca6d0` on event `0xa`); until then a unit given
    /// the order (`retreating`, `+0x68`) does not yet head for its exit.
    pub withdrawal_executors: Vec<TacticalUnitRef>,
    /// The task force chosen whole (its group button or key): the input
    /// state `0x66bc48`, whose orders go to the force (`0x005a3630`,
    /// `0x005a3740`). `None` when single units are selected.
    pub task_force_selection: Option<u8>,
    /// Each side's group slots' formation (the mover `+0x54`, built from
    /// the tactic by `0x005cb1a0` when event `0x3d` runs, `0x005c9c50`).
    /// Index: `[attacker as usize][slot]`. A new force starts Stand Off
    /// (`0x005cbf50` builds `0x005ec310`).
    pub task_force_formations: [[TacticalTactic; 8]; 2],
    /// Each force's band of distance to its target, by its lead unit: the
    /// tactic object's `+0x30` and `+0x34` (slot `+0x10`), recomputed once
    /// a turn through the force's slot `+0xc` (`0x005c9bd0`).
    pub engagement_bands: Vec<TacticalEngagementBand>,
    /// Each fighter force's circling angle (`+0xc0`, `FUN_005c6610`).
    pub fighter_orbit_angles: Vec<f32>,
    /// The fighter forces' movers, one per side and group.
    pub fighter_movers: Vec<TacticalFighterMover>,
    /// Each squadron's forward (`+0x10`) and velocity (`+0x6c`), by
    /// fighter index.
    pub fighter_frames: Vec<TacticalFighterFrame>,
    /// Each capital force's mover (`+0x54`): its anchor, facing and rates.
    pub force_movers: Vec<TacticalForceMover>,
    /// Each ship's own movement state, by ship index.
    pub member_states: Vec<TacticalMemberState>,
    /// The forces' detour records (`+0xec`).
    pub force_detours: Vec<TacticalForceDetour>,
    /// The player's commander in mode 1 (`+0x08`, `FUN_005c7e10`), set by
    /// command `0x7b` from Observe Battle (`FUN_0059fd40`): the commander
    /// then reviews the player's side as well.
    pub player_observing: bool,
    /// The player's commander's count `+0x40`, in turns to its next review.
    pub observer_review_countdown: u32,
    /// The Alliance commander's `+0x18`: it has sent a group on the Death
    /// Star run (or one was under way when it took command).
    pub alliance_raid_sent: bool,
    /// Resolved Death Star trench-run result, if one has occurred.
    pub trench_run_outcome: Option<TacticalTrenchRunOutcome>,
    /// Remaining source milliseconds in the active state-5 trench run.
    pub trench_run_remaining_ms: Option<f32>,
    /// RGBY group that entered the active run and owns its ordered voice path.
    pub trench_run_group: Option<u8>,
    /// Source `FUN_005ad7e0` commander-rating result, clamped to 1 through 9:
    /// the person in the side's commander slot `+0x950` (set by
    /// `FUN_005a93e0` from a person flagged `+0x58`, the flag
    /// `FUN_0040c200` gives the Alliance's battle commander), its `+0x50`
    /// minus -1.0 (`DAT_0066c27c`), or 1.0 with none. These are the abstract
    /// battle's person and constants, so a session takes
    /// [`crate::abstract_battle::trench_commander_term`]. The trench run
    /// compares an integer 1..100 draw with it, so the fraction drops.
    pub trench_run_success_rating: u8,
    /// `FUN_005ad7e0(0)`: the Alliance's commander term, 1 through 9, as a
    /// fighter's speed adds it (`0x5ba190`). The Empire's side has no
    /// flagged commander (`FUN_0040c200`), so its term is 1.0.
    pub alliance_commander_term: f32,
    /// Result selected by `FUN_005d04e0` when state 5 begins. State 6 or 7
    /// applies it only after the two-minute presentation timer completes.
    pub trench_run_pending_outcome: Option<TacticalTrenchRunOutcome>,
    /// Stable fighter indices retained by the source result manager.
    pub trench_run_participants: Vec<usize>,
    /// Each participant's hull and shields when the run began
    /// (`FUN_005d08d0` copies `+0x3cc` to `+0x3d0` and `+0x3c0` to
    /// `+0x3c4`); an aborted run restores them (`FUN_005cfec0` state 8).
    pub trench_run_saved: Vec<(usize, f32, f32)>,
    /// Nine threshold slots selected by `FUN_005d04e0`; `0x13d` is silent.
    pub trench_run_chatter: [u16; 9],
    /// Next threshold slot consumed by `FUN_005d03f0`.
    pub trench_run_chatter_cursor: usize,
    /// Presentation edge consumed once by the app-level cutscene router.
    pub trench_run_cinematic_pending: bool,
    /// Four independently visible navigation-point sets. Their spread follows
    /// the manual's far-to-clustered ordering and scales with the recovered
    /// battle envelope.
    pub navigation_sets: [Vec<TacticalWorldPosition>; 4],
    /// Immutable source battle envelope used to place the initial participants.
    pub source_layout: OriginalTacticalLayout,
    /// Index of currently selected ship (in `ships`), if any.
    pub selected_ship: Option<usize>,
    /// Selected player fighter group, if the HUD is displaying a squadron.
    pub selected_fighter_group: Option<u8>,
    /// Whether the placement phase is confirmed (player clicked "Begin Battle").
    pub placement_confirmed: bool,
    /// Game tick when the battle started.
    pub start_tick: u64,
    /// Combat sub-tick counter (increments each combat step).
    pub combat_tick: u32,
    /// One source-compatible tactical random stream. The campaign contributes
    /// its seed at battle entry; every combat consumer advances it in order.
    pub tactical_rng: OriginalTacticalRng,
    /// Active weapon fire visual effects (source ship idx -> target ship idx).
    pub weapon_effects: Vec<WeaponEffect>,
    /// Target-attached original type-303 hit, damage, and destruction sequences.
    pub impact_effects: Vec<TacticalImpactEffect>,
    /// Source audio events waiting for the application audio backend. The
    /// resource identity is fixed when the combat event is emitted so frame
    /// scheduling cannot change variant selection.
    pub pending_audio_cues: Vec<TacticalAudioCue>,
    /// Source-identified VOICEFXA/VOICEFXE tactical lines waiting for the
    /// application audio backend.
    pub pending_voice_cues: Vec<TacticalVoiceCue>,
    /// `FUN_005ada10`'s opening and Death Star announcer.
    pub voice_announcer: TacticalVoiceAnnouncer,
    /// Target-attached tractor/gravity field selected through one shared slot.
    pub field_effects: Vec<TacticalFieldEffect>,
    /// Source-selected subsystem repairs emitted by the latest combat step.
    pub subsystem_repairs: Vec<TacticalSubsystemRepair>,
    /// Whether combat is paused.
    pub paused: bool,
    /// Combat speed multiplier (1 = normal, 2 = fast, 4 = faster).
    pub combat_speed: u32,
    /// Accumulator for sub-frame combat stepping.
    pub step_accumulator: f32,
    /// Winner once battle ends (None during combat).
    pub winner: Option<CombatWinner>,
    /// Each side's retreat order from the battle choice (Alliance, Empire;
    /// `FUN_004362b0`'s `+0x3c` word). The battle start (`FUN_00599c90`)
    /// gives it to the abstract commanders (`FUN_005afb30`) and to the
    /// tactical ones (`FUN_005a8af0`, `+0x80`), so in the tactical battle
    /// too an AI side with the order withdraws (`FUN_005c8780`).
    pub retreat_allowed: [bool; 2],
    /// Whether each side has left (Alliance, Empire).
    pub retreat_done: [bool; 2],
    /// The player's side withdrawing: the milliseconds left (`+0x7c`,
    /// `FUN_005ad530`, `FUN_005c8780`; state 2), then 0 while its ships
    /// jump out (state 3).
    pub withdrawal_ms: Option<f32>,
    /// The AI side's withdrawal time, as `withdrawal_ms`.
    pub ai_withdrawal_ms: Option<f32>,
    /// Diagnostic lines the simulation produced since the host last drained
    /// them; the host prints each one on its own console.
    pub diagnostic_log: Vec<String>,
}

/// A battle session with the fleet of each ship and fighter squadron.
#[derive(Debug, Clone)]
pub struct FleetBattle {
    pub session: BattleSession,
    pub ship_fleets: Vec<FleetKey>,
    pub fighter_fleets: Vec<FleetKey>,
}

/// A visual weapon fire effect between two ships.
#[derive(Debug, Clone)]
pub struct WeaponEffect {
    /// Index of the firing capital ship in `ships`, or `usize::MAX` for a
    /// fighter source. Rendering uses the captured source position.
    pub source: usize,
    /// Index of the target capital ship in `ships`, or `usize::MAX` for a
    /// fighter target. Rendering uses the captured target position.
    pub target: usize,
    /// Weapon type determines color and rendering.
    pub kind: WeaponKind,
    /// Source point is captured once, matching the retained projectile frame.
    pub source_position: TacticalWorldPosition,
    /// Existing 2D simulation source used only when the original camera path is unavailable.
    pub fallback_source: [f32; 2],
    /// Target point captured once for capital and fighter projectiles alike.
    pub target_position: TacticalWorldPosition,
    /// Existing 2D simulation target used only when the original camera path is unavailable.
    pub fallback_target: [f32; 2],
    /// Source constructor scale along the projectile's local Z axis.
    pub longitudinal_scale: f32,
    /// One of the three exact triangle families built by `FUN_005ee590`.
    pub shape_variant: u8,
    /// Direct3DRM material selector: 0 red, 1 green, 2 blue.
    pub color_selector: u8,
    /// Source lifecycle timer and exact one- or two-second duration.
    pub elapsed: f32,
    pub duration: f32,
}

impl WeaponEffect {
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }
}

/// One target's shared tractor/gravity field slot from `FUN_005d3ac0`.
#[derive(Debug, Clone, PartialEq)]
pub struct TacticalFieldEffect {
    pub target: usize,
    pub tractor_sources: Vec<usize>,
    pub gravity_sources: Vec<usize>,
    pub frame: u8,
    pub frame_elapsed: f32,
}

/// The source field selector uses bit 1 for tractor and bit 2 for gravity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalTacticalFieldKind {
    Tractor = 1,
    Gravity = 2,
}

impl TacticalFieldEffect {
    pub fn visible_kind(&self) -> Option<OriginalTacticalFieldKind> {
        if !self.gravity_sources.is_empty() {
            Some(OriginalTacticalFieldKind::Gravity)
        } else if !self.tractor_sources.is_empty() {
            Some(OriginalTacticalFieldKind::Tractor)
        } else {
            None
        }
    }

    /// `FUN_005d3cc0` field frames: gravity 3520–3527, tractor 3620–3627.
    #[must_use]
    pub fn resource_id(&self) -> Option<u32> {
        self.visible_kind().map(|kind| match kind {
            OriginalTacticalFieldKind::Gravity => 3520 + u32::from(self.frame),
            OriginalTacticalFieldKind::Tractor => 3620 + u32::from(self.frame),
        })
    }

    #[must_use]
    pub fn source_counts(&self) -> (u8, u8) {
        (
            u8::try_from(self.tractor_sources.len()).unwrap_or(u8::MAX),
            u8::try_from(self.gravity_sources.len()).unwrap_or(u8::MAX),
        )
    }

    #[must_use]
    pub fn source_ids(&self) -> (&[usize], &[usize]) {
        (&self.tractor_sources, &self.gravity_sources)
    }
}

/// One target-attached type-303 impact sequence selected by the original
/// tactical effect dispatcher at `FUN_005d3e90`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalImpactEffect {
    pub target: usize,
    pub sequence: OriginalTacticalEffectSequence,
    pub frame: u8,
    pub frame_elapsed: f32,
}

/// Original target sprite states. The discriminants preserve the priority and
/// mutual-exclusion bits tested by `FUN_005d3e90`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalTacticalEffectSequence {
    IonHit = 0x04,
    StandardHit = 0x08,
    IonDamage = 0x10,
    StandardDamage = 0x20,
    TurbolaserDamage = 0x40,
    Destroyed = 0x80,
}

impl OriginalTacticalEffectSequence {
    /// `FUN_005d3e90` type-303 bases.
    #[must_use]
    pub const fn resource_base(self) -> u32 {
        match self {
            Self::IonHit => 3180,
            Self::StandardHit => 3060,
            Self::IonDamage => 3240,
            Self::StandardDamage => 3120,
            Self::TurbolaserDamage => 3300,
            Self::Destroyed => 3360,
        }
    }

    /// `FUN_005d3e90` frame counts.
    #[must_use]
    pub const fn frame_count(self) -> u8 {
        match self {
            Self::IonHit | Self::StandardHit => 6,
            Self::IonDamage | Self::Destroyed => 16,
            Self::StandardDamage | Self::TurbolaserDamage => 7,
        }
    }

    /// `FUN_005d3e90` draw sizes.
    #[must_use]
    pub const fn source_size(self) -> (u16, u16) {
        match self {
            Self::IonHit | Self::StandardHit | Self::StandardDamage | Self::TurbolaserDamage => {
                (32, 32)
            }
            Self::IonDamage => (64, 32),
            Self::Destroyed => (64, 64),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalTacticalImpactStage {
    Hit = 0,
    Damage = 1,
    Destroyed = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalVoiceGroup {
    Capital(u8),
    Fighter(u8),
}

/// Route `FUN_005a5cf0`'s destruction callback from the local player's
/// perspective. A local attacker names its own firing group in the target-
/// destroyed family; a local casualty names the group that was lost.
pub fn queue_original_destruction_voice(
    voice_cues: &mut Vec<TacticalVoiceCue>,
    faction: TacticalVoiceFaction,
    player_is_attacker: bool,
    source_is_attacker: bool,
    source_group: TacticalVoiceGroup,
    target_is_attacker: bool,
    target_group: TacticalVoiceGroup,
    stage: OriginalTacticalImpactStage,
) {
    if stage != OriginalTacticalImpactStage::Destroyed {
        return;
    }
    let cue = if source_is_attacker == player_is_attacker
        && target_is_attacker != player_is_attacker
    {
        match source_group {
            TacticalVoiceGroup::Capital(group) => {
                TacticalVoiceCue::target_destroyed_by_capital(faction, group)
            }
            TacticalVoiceGroup::Fighter(group) => {
                TacticalVoiceCue::target_destroyed_by_fighter(faction, group)
            }
        }
    } else if target_is_attacker == player_is_attacker && source_is_attacker != player_is_attacker {
        match target_group {
            TacticalVoiceGroup::Capital(group) => {
                TacticalVoiceCue::capital_destroyed(faction, group)
            }
            TacticalVoiceGroup::Fighter(group) => TacticalVoiceCue::fighter_losses(faction, group),
        }
    } else {
        return;
    };
    voice_cues.push(cue);
}

/// Type of weapon for visual rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponKind {
    Turbolaser,
    IonCannon,
    LaserCannon,
    FighterAttack,
}

/// The eight weapon audio events dispatched by `FUN_005a7500` through the
/// original tactical sound manager. `FUN_005bae60` assigns each event a
/// contiguous TACTICAL.DLL WAVE family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalAudioEvent {
    LaserFire,
    TurbolaserFire,
    IonFire,
    TorpedoFire,
    LaserImpact,
    IonImpact,
    TurbolaserImpact,
    TorpedoImpact,
    /// A fighter group's jump out (`FUN_005a6b40`, slot `+0x20` = 1).
    FighterJump,
    /// A capital ship's jump out (`FUN_005a6b40`, slot `+0x20` = 0).
    CapitalJump,
}

impl TacticalAudioEvent {
    /// `FUN_005ba8b0` weapon events from `FUN_005b5100`, `FUN_005b5750`, `FUN_005b5b60`, `FUN_005b5f50`.
    #[must_use]
    pub const fn event_id(self) -> u8 {
        match self {
            Self::LaserFire => 0x0d,
            Self::TurbolaserFire => 0x0e,
            Self::IonFire => 0x0f,
            Self::TorpedoFire => 0x10,
            Self::LaserImpact => 0x11,
            Self::IonImpact => 0x12,
            Self::TurbolaserImpact => 0x13,
            Self::TorpedoImpact => 0x14,
            Self::FighterJump => 0x1d,
            Self::CapitalJump => 0x1f,
        }
    }

    /// `FUN_005bae60` TACTICAL.DLL WAVE bases.
    #[must_use]
    pub const fn resource_base(self) -> u32 {
        match self {
            Self::LaserFire => 13_033,
            Self::TurbolaserFire => 13_036,
            Self::IonFire => 13_039,
            Self::TorpedoFire => 13_042,
            Self::LaserImpact => 13_045,
            Self::IonImpact => 13_048,
            Self::TurbolaserImpact => 13_051,
            Self::TorpedoImpact => 13_054,
            // The table's event 0 is 13000 at `+0x28e` (two words an
            // event), so `0x1d` and `0x1f` read 13063 and 13065.
            Self::FighterJump => 13_063,
            Self::CapitalJump => 13_065,
        }
    }

    /// `FUN_005bae60` per-event variant counts (`+0x50` + 4 * event): 1 by
    /// default, then 4 for events 0..2, 2 for 3..5, 1 for 6..8, 3 for
    /// 9..0x13 and 1 for 0x14..0x1f.
    #[must_use]
    pub const fn variant_count(self) -> u8 {
        original_audio_variant_count(self.event_id())
    }

    pub const fn fire_for(kind: WeaponKind) -> Self {
        match kind {
            WeaponKind::LaserCannon => Self::LaserFire,
            WeaponKind::Turbolaser => Self::TurbolaserFire,
            WeaponKind::IonCannon => Self::IonFire,
            WeaponKind::FighterAttack => Self::TorpedoFire,
        }
    }

    pub const fn impact_for(kind: WeaponKind) -> Self {
        match kind {
            WeaponKind::LaserCannon => Self::LaserImpact,
            WeaponKind::Turbolaser => Self::TurbolaserImpact,
            WeaponKind::IonCannon => Self::IonImpact,
            WeaponKind::FighterAttack => Self::TorpedoImpact,
        }
    }
}

/// `FUN_005bae60`'s `+0x50` table: how many WAVE variants an event has.
#[must_use]
pub const fn original_audio_variant_count(event: u8) -> u8 {
    match event {
        0x00..=0x02 => 4,
        0x03..=0x05 => 2,
        0x09..=0x13 => 3,
        _ => 1,
    }
}

/// One concrete source event and selected original WAVE resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalAudioCue {
    pub event: TacticalAudioEvent,
    pub resource_id: u32,
}

/// Faction-specific voice bank selected by the tactical sound manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TacticalVoiceFaction {
    Alliance,
    Empire,
}

impl TacticalVoiceFaction {
    /// `FUN_005bae60` faction event bases.
    #[must_use]
    pub const fn source_event_base(self) -> u16 {
        match self {
            Self::Alliance => 0x20,
            Self::Empire => 0x9a,
        }
    }

    /// `FUN_005bae60` VOICEFXA/VOICEFXE WAVE bases.
    #[must_use]
    pub const fn resource_base(self) -> u32 {
        match self {
            Self::Alliance => 14_001,
            Self::Empire => 15_001,
        }
    }
}

/// Source family that selected a faction tactical voice line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalVoiceEvent {
    SourceMapped,
    BattleReady,
    /// Alliance events 0x90-0x94 (`FUN_005ada10`).
    DeathStarWarning,
    /// Empire events 0x11c-0x11d (`FUN_005ada10`, `FUN_005cfec0`).
    DeathStarFighterScreen,
    TargetTaskForceRejected,
    CapitalManeuver,
    FighterManeuver,
    ManeuverComplete,
    CapitalAttack,
    FighterAttack,
    OrdersComplete,
    TargetDestroyed,
    CapitalDestroyed,
    FighterLosses,
    CapitalFormation,
    CapitalMission,
    FighterMission,
    FighterRecoveryComplete,
    FighterRecoveryCapacity,
    WithdrawalStarted,
    WithdrawalHyperdriveWarning,
    WithdrawalComplete,
    BattleWonWithdrawal,
    BattleWonDestroyed,
    BattleLost,
    DeathStarFiring,
    DeathStarReady,
    DeathStarUnderAttack,
    DeathStarAttackBrokenOff,
    DeathStarDestroyed,
    TrenchRunStarted,
    TrenchRunCommitted,
    TrenchRunChatter,
    TrenchRunFailed,
    TrenchRunSucceeded,
}

/// One exact tactical voice event and its original WAVE resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalVoiceCue {
    pub event: TacticalVoiceEvent,
    pub faction: TacticalVoiceFaction,
    pub source_event: u16,
    pub resource_id: u32,
}

/// `FUN_005ada10`'s state (`+0xe94`): 0 to 3 walk the opening lines
/// (`+0xe74..+0xe80`), 1000 waits, 4 watches the Attack Death Star window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TacticalVoiceAnnouncerState {
    /// Not armed yet: the player has not taken command.
    #[default]
    Idle,
    Opening(u8),
    Waiting,
    Watching,
}

/// `FUN_005ada10`'s fields: state `+0xe94`, countdown `+0xe6c` (ms) and the
/// last Attack Death Star result `+0xe68`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TacticalVoiceAnnouncer {
    pub state: TacticalVoiceAnnouncerState,
    pub countdown_ms: f32,
    pub attack_death_star_open: bool,
}

/// `DAT_0066c238`: the announcer's gap between lines.
pub const ORIGINAL_VOICE_ANNOUNCER_GAP_MS: f32 = 6000.0;

/// `FUN_005ada10`'s Alliance opening (`+0xe74..+0xe80`): battle ready, then
/// the Death Star warnings approach, shield and fighter screen.
const ALLIANCE_OPENING_EVENTS: [u16; 4] = [0x20, 0x90, 0x93, 0x94];

impl TacticalVoiceCue {
    /// `FUN_005bae60` builds the three banks and `FUN_005bad50` resolves an event to its WAVE.
    pub const fn from_source_event(event: TacticalVoiceEvent, source_event: u16) -> Self {
        let (faction, event_base, resource_base) = match source_event {
            0x20..=0x99 => (TacticalVoiceFaction::Alliance, 0x20, 14_001),
            0x9a..=0x11d => (TacticalVoiceFaction::Empire, 0x9a, 15_001),
            0x11e..=0x13c => (TacticalVoiceFaction::Alliance, 0x11e, 15_133),
            _ => panic!("source event is outside the recovered tactical voice banks"),
        };
        Self {
            event,
            faction,
            source_event,
            resource_id: resource_base + (source_event - event_base) as u32,
        }
    }

    pub const fn from_offset(
        faction: TacticalVoiceFaction,
        event: TacticalVoiceEvent,
        offset: u16,
    ) -> Self {
        Self::from_source_event(event, faction.source_event_base() + offset)
    }

    #[must_use]
    pub const fn battle_ready(faction: TacticalVoiceFaction) -> Self {
        Self::from_offset(faction, TacticalVoiceEvent::BattleReady, 0)
    }

    /// Queue the faction-specific rejection used when an enemy target command
    /// cannot resolve the selected capitals to one task-force ordinal.
    ///
    /// `FUN_005c55b0` events 0x84 and 0x102.
    #[must_use]
    pub const fn target_task_force_rejected(faction: TacticalVoiceFaction) -> Self {
        let source_event = match faction {
            TacticalVoiceFaction::Alliance => 0x84,
            TacticalVoiceFaction::Empire => 0x102,
        };
        Self::from_source_event(TacticalVoiceEvent::TargetTaskForceRejected, source_event)
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn capital_maneuver(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::CapitalManeuver,
            capital_group_offset(2, group),
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn fighter_maneuver(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::FighterManeuver,
            10 + group.min(3) as u16,
        )
    }

    /// `FUN_005bae60` events 0x2e..0x3a and 0xa8..0xb4. The generic line is
    /// followed by task forces one through eight and RGBY fighter groups.
    #[must_use]
    pub fn capital_maneuver_complete(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::ManeuverComplete,
            capital_group_offset(15, group),
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn fighter_maneuver_complete(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::ManeuverComplete,
            23 + group.min(3) as u16,
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn capital_attack(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::CapitalAttack,
            capital_group_offset(28, group),
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn fighter_attack(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::FighterAttack,
            36 + group.min(3) as u16,
        )
    }

    /// `FUN_005bae60` events 0x48..0x54 and 0xc2..0xce.
    #[must_use]
    pub fn capital_orders_complete(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::OrdersComplete,
            capital_group_offset(41, group),
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn fighter_orders_complete(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::OrdersComplete,
            49 + group.min(3) as u16,
        )
    }

    /// `FUN_005a5cf0` selects these variants from the local firing group when
    /// its target is destroyed.
    #[must_use]
    pub fn target_destroyed_by_capital(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::TargetDestroyed,
            capital_group_offset(54, group),
        )
    }

    /// `FUN_005a5cf0`; `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn target_destroyed_by_fighter(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::TargetDestroyed,
            62 + group.min(3) as u16,
        )
    }

    /// `FUN_005a5cf0` selects loss variants from the local destroyed group.
    #[must_use]
    pub fn capital_destroyed(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::CapitalDestroyed,
            capital_group_offset(67, group),
        )
    }

    /// `FUN_005a5cf0`; `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn fighter_losses(faction: TacticalVoiceFaction, group: u8) -> Self {
        Self::from_offset(
            faction,
            TacticalVoiceEvent::FighterLosses,
            75 + group.min(3) as u16,
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn capital_formation(faction: TacticalVoiceFaction, group: u8) -> Self {
        let base = match faction {
            TacticalVoiceFaction::Alliance => 79,
            TacticalVoiceFaction::Empire => 83,
        };
        Self::from_offset(
            faction,
            TacticalVoiceEvent::CapitalFormation,
            base + group.min(7) as u16,
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn capital_mission(faction: TacticalVoiceFaction, group: u8) -> Self {
        let base = match faction {
            TacticalVoiceFaction::Alliance => 88,
            TacticalVoiceFaction::Empire => 92,
        };
        Self::from_offset(
            faction,
            TacticalVoiceEvent::CapitalMission,
            capital_group_offset(base, group),
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn fighter_mission(faction: TacticalVoiceFaction, group: u8) -> Self {
        let base = match faction {
            TacticalVoiceFaction::Alliance => 96,
            TacticalVoiceFaction::Empire => 100,
        };
        Self::from_offset(
            faction,
            TacticalVoiceEvent::FighterMission,
            base + group.min(3) as u16,
        )
    }

    /// Queue the source group acknowledgement emitted when the final fighters
    /// in a recovering group enter the carrier.
    ///
    /// `FUN_005b8630` events 0x87–0x8a and 0x105–0x108.
    #[must_use]
    pub fn fighter_recovery_complete(faction: TacticalVoiceFaction, group: u8) -> Self {
        let source_event = match faction {
            TacticalVoiceFaction::Alliance => 0x87,
            TacticalVoiceFaction::Empire => 0x105,
        } + u16::from(group.min(3));
        Self::from_source_event(TacticalVoiceEvent::FighterRecoveryComplete, source_event)
    }

    /// Queue the source group warning used when no surviving friendly carrier
    /// has another fighter docking slot.
    ///
    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub fn fighter_recovery_capacity(faction: TacticalVoiceFaction, group: u8) -> Self {
        let source_event = match faction {
            TacticalVoiceFaction::Alliance => 0x8b,
            TacticalVoiceFaction::Empire => 0x109,
        } + u16::from(group.min(3));
        Self::from_source_event(TacticalVoiceEvent::FighterRecoveryCapacity, source_event)
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub const fn withdrawal_started(faction: TacticalVoiceFaction) -> Self {
        let source_event = match faction {
            TacticalVoiceFaction::Alliance => 0x8f,
            TacticalVoiceFaction::Empire => 0x10d,
        };
        Self::from_source_event(TacticalVoiceEvent::WithdrawalStarted, source_event)
    }

    /// Queue the original warning that ships without an operational
    /// hyperdrive will be left behind by a fleet withdrawal.
    ///
    /// `FUN_005a0240` events 0x95 and 0x10e.
    #[must_use]
    pub const fn withdrawal_hyperdrive_warning(faction: TacticalVoiceFaction) -> Self {
        let source_event = match faction {
            TacticalVoiceFaction::Alliance => 0x95,
            TacticalVoiceFaction::Empire => 0x10e,
        };
        Self::from_source_event(
            TacticalVoiceEvent::WithdrawalHyperdriveWarning,
            source_event,
        )
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub const fn withdrawal_complete(faction: TacticalVoiceFaction) -> Self {
        let source_event = match faction {
            TacticalVoiceFaction::Alliance => 0x96,
            TacticalVoiceFaction::Empire => 0x118,
        };
        Self::from_source_event(TacticalVoiceEvent::WithdrawalComplete, source_event)
    }

    /// `FUN_005a9ec0` events 0x97/0x98 and 0x119/0x11a.
    #[must_use]
    pub const fn battle_won(faction: TacticalVoiceFaction, opponent_withdrew: bool) -> Self {
        let source_event = match (faction, opponent_withdrew) {
            (TacticalVoiceFaction::Alliance, true) => 0x97,
            (TacticalVoiceFaction::Alliance, false) => 0x98,
            (TacticalVoiceFaction::Empire, true) => 0x119,
            (TacticalVoiceFaction::Empire, false) => 0x11a,
        };
        Self::from_source_event(
            if opponent_withdrew {
                TacticalVoiceEvent::BattleWonWithdrawal
            } else {
                TacticalVoiceEvent::BattleWonDestroyed
            },
            source_event,
        )
    }

    /// `FUN_005a9ec0`; `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub const fn battle_lost(faction: TacticalVoiceFaction) -> Self {
        let source_event = match faction {
            TacticalVoiceFaction::Alliance => 0x99,
            TacticalVoiceFaction::Empire => 0x11b,
        };
        Self::from_source_event(TacticalVoiceEvent::BattleLost, source_event)
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub const fn death_star_firing() -> Self {
        Self::from_source_event(TacticalVoiceEvent::DeathStarFiring, 0x111)
    }

    /// `FUN_005ba7f0` event 0x113.
    #[must_use]
    pub const fn death_star_ready() -> Self {
        Self::from_source_event(TacticalVoiceEvent::DeathStarReady, 0x113)
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub const fn death_star_under_attack() -> Self {
        Self::from_source_event(TacticalVoiceEvent::DeathStarUnderAttack, 0x115)
    }

    /// `FUN_005cfec0` state 5 for the Empire's player: event 0x11d, the
    /// attack goes on (TACTICAL.DLL 15132, A0 TAC-01-C012 at 53.72 s).
    #[must_use]
    pub const fn death_star_attack_continuing() -> Self {
        Self::from_source_event(TacticalVoiceEvent::DeathStarFighterScreen, 0x11d)
    }

    /// `FUN_005d0340` event 0x115.
    #[must_use]
    pub const fn death_star_attack_broken_off() -> Self {
        Self::from_source_event(TacticalVoiceEvent::DeathStarAttackBrokenOff, 0x116)
    }

    /// `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub const fn death_star_destroyed() -> Self {
        Self::from_source_event(TacticalVoiceEvent::DeathStarDestroyed, 0x117)
    }

    /// `FUN_005d0340` group events 0x139, 0x12d, 0x131, 0x135.

    pub const fn trench_run_source_event(group: u8) -> u16 {
        match group {
            0 => 0x139,
            1 => 0x12d,
            2 => 0x131,
            _ => 0x135,
        }
    }

    #[must_use]
    pub const fn trench_run_started(group: u8) -> Self {
        Self::from_source_event(
            TacticalVoiceEvent::TrenchRunStarted,
            Self::trench_run_source_event(group),
        )
    }

    /// `FUN_005cfec0` queues the group-specific follow-up immediately after
    /// the tactical result manager enters the timed trench-run state.
    #[must_use]
    pub const fn trench_run_committed(group: u8) -> Self {
        Self::from_source_event(
            TacticalVoiceEvent::TrenchRunCommitted,
            Self::trench_run_source_event(group) + 1,
        )
    }

    #[must_use]
    pub const fn trench_run_chatter(source_event: u16) -> Self {
        Self::from_source_event(TacticalVoiceEvent::TrenchRunChatter, source_event)
    }

    /// `FUN_005d03f0`; `FUN_005bae60` event table (`FUN_005ba8b0` queues it); family span from `docs/reference/asset-library/tactical-voice-map.json`.
    #[must_use]
    pub const fn trench_run_outcome(group: u8, outcome: TacticalTrenchRunOutcome) -> Self {
        let (event, offset) = match outcome {
            TacticalTrenchRunOutcome::Failure => (TacticalVoiceEvent::TrenchRunFailed, 2),
            TacticalTrenchRunOutcome::Success => (TacticalVoiceEvent::TrenchRunSucceeded, 3),
        };
        Self::from_source_event(event, Self::trench_run_source_event(group) + offset)
    }

    pub const fn fixture_source_event(source_event: u16) -> Self {
        Self::from_source_event(TacticalVoiceEvent::SourceMapped, source_event)
    }
}

impl TacticalAudioCue {
    #[must_use]
    pub const fn from_variant(event: TacticalAudioEvent, variant: u8) -> Self {
        let bounded = variant % event.variant_count();
        Self {
            event,
            resource_id: event.resource_base() + bounded as u32,
        }
    }

    #[must_use]
    pub const fn from_seed(event: TacticalAudioEvent, seed: u32) -> Self {
        Self::from_variant(event, (seed % event.variant_count() as u32) as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalTacticalWeaponArc {
    Fore = 0,
    Starboard = 1,
    Aft = 2,
    Port = 3,
}

impl OriginalTacticalWeaponArc {
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Reproduce the two rotated-forward tests in `FUN_005b6530`. Equality at the
/// forward boundaries belongs to the fore arc; equality at the aft boundaries
/// remains in the corresponding lateral arc.
pub fn original_tactical_weapon_arc(
    source: &TacticalShip,
    target: &TacticalShip,
) -> OriginalTacticalWeaponArc {
    original_tactical_weapon_arc_to_position(source, target.source_position)
}

pub fn original_tactical_weapon_arc_to_position(
    source: &TacticalShip,
    target: TacticalWorldPosition,
) -> OriginalTacticalWeaponArc {
    let delta = source_delta(target, source.source_position);
    let forward = source.source_forward;
    let forward_dot = delta.x.mul_add(forward.x, delta.z * forward.z);
    let starboard_dot = delta.x.mul_add(forward.z, -delta.z * forward.x);
    let lateral = starboard_dot.abs();
    if forward_dot >= lateral {
        OriginalTacticalWeaponArc::Fore
    } else if -forward_dot > lateral {
        OriginalTacticalWeaponArc::Aft
    } else if starboard_dot > 0.0 {
        OriginalTacticalWeaponArc::Starboard
    } else {
        OriginalTacticalWeaponArc::Port
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OriginalTacticalWeaponCandidate {
    pub strength: f32,
    pub target: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OriginalTacticalArcCandidates {
    pub laser: OriginalTacticalWeaponCandidate,
    pub ion: OriginalTacticalWeaponCandidate,
    pub turbolaser: OriginalTacticalWeaponCandidate,
}

impl OriginalTacticalArcCandidates {
    pub fn total(self) -> f32 {
        self.laser.strength + self.ion.strength + self.turbolaser.strength
    }
}

pub fn original_capital_weapon_strength(
    ship: &TacticalShip,
    count: u32,
    kind: WeaponKind,
    scale_for_hull: bool,
) -> f32 {
    let target_modifier = match kind {
        WeaponKind::LaserCannon => ORIGINAL_LASER_VS_CAPITAL,
        WeaponKind::IonCannon | WeaponKind::Turbolaser | WeaponKind::FighterAttack => 1.0,
    };
    let strength = count as f32 * ORIGINAL_CAPITAL_WEAPON_STRENGTH * target_modifier;
    if scale_for_hull {
        strength * ship.hull_ratio()
    } else {
        strength
    }
}

pub fn original_weapon_range(ship: &TacticalShip, kind: WeaponKind) -> f32 {
    match kind {
        WeaponKind::LaserCannon => ship.weapon_ranges.laser_cannon,
        WeaponKind::IonCannon => ship.weapon_ranges.ion_cannon,
        WeaponKind::Turbolaser => ship.weapon_ranges.turbolaser,
        WeaponKind::FighterAttack => 0.0,
    }
}

pub fn original_weapon_count(arc: TacticalWeaponArc, kind: WeaponKind) -> u32 {
    match kind {
        WeaponKind::LaserCannon => arc.laser_cannons,
        WeaponKind::IonCannon => arc.ion_cannons,
        WeaponKind::Turbolaser => arc.turbolasers,
        WeaponKind::FighterAttack => 0,
    }
}

impl WeaponKind {
    /// `SHIP_TAKE_*_HIT` codes delivered to the original render dispatcher.
    ///
    /// `FUN_005a7500` registers these codes 1–4.
    pub const fn original_impact_code(self) -> u8 {
        match self {
            Self::LaserCannon | Self::FighterAttack => 1,
            Self::IonCannon => 2,
            Self::Turbolaser => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalProjectileProfile {
    pub longitudinal_scale: f32,
    pub shape_variant: u8,
    pub color_selector: u8,
    pub duration: f32,
}

/// `FUN_005ee590` scales, variants, and colours; thresholds from `FUN_005b1ea0`, `FUN_005b1f60`, `FUN_005b2080`, `FUN_005b2c70`.

pub fn original_projectile_profile(
    weapon: WeaponKind,
    event_strength: f32,
    source_tactical_ordinal: Option<u8>,
    source_is_alliance: bool,
) -> OriginalProjectileProfile {
    let strength = if event_strength.is_finite() {
        event_strength.max(0.0)
    } else {
        0.0
    };
    let (longitudinal_scale, shape_variant) = match weapon {
        WeaponKind::LaserCannon if strength >= ORIGINAL_LASER_PROJECTILE_THRESHOLD => (0.5, 2),
        WeaponKind::LaserCannon => (0.5, 1),
        WeaponKind::Turbolaser if strength < ORIGINAL_TURBOLASER_PROJECTILE_THRESHOLD => (0.65, 1),
        WeaponKind::Turbolaser
            if matches!(source_tactical_ordinal, Some(11 | 13 | 14 | 22 | 25)) =>
        {
            (0.75, 2)
        }
        WeaponKind::Turbolaser => (1.0, 3),
        WeaponKind::IonCannon if strength >= ORIGINAL_ION_PROJECTILE_THRESHOLD => (1.0, 3),
        WeaponKind::IonCannon => (0.4, 1),
        WeaponKind::FighterAttack if strength >= ORIGINAL_TORPEDO_PROJECTILE_THRESHOLD => (1.0, 3),
        WeaponKind::FighterAttack => (0.2, 1),
    };
    let color_selector = if weapon == WeaponKind::IonCannon {
        2
    } else if source_is_alliance {
        0
    } else {
        1
    };
    OriginalProjectileProfile {
        longitudinal_scale,
        shape_variant,
        color_selector,
        duration: if shape_variant == 3 {
            ORIGINAL_PROJECTILE_DURATION_LONG
        } else {
            ORIGINAL_PROJECTILE_DURATION_SHORT
        },
    }
}

/// Reproduce the resource branch in `FUN_005d3e90`. Its first argument uses
/// the `SHIP_TAKE_*_HIT` ordering recovered from `FUN_005a7500`; its second is
/// the target effect stage returned immediately after hit resolution.
pub fn original_tactical_effect_sequence(
    weapon: WeaponKind,
    stage: OriginalTacticalImpactStage,
) -> OriginalTacticalEffectSequence {
    match stage {
        OriginalTacticalImpactStage::Hit if weapon.original_impact_code() == 2 => {
            OriginalTacticalEffectSequence::IonHit
        }
        OriginalTacticalImpactStage::Hit => OriginalTacticalEffectSequence::StandardHit,
        OriginalTacticalImpactStage::Damage if weapon.original_impact_code() == 2 => {
            OriginalTacticalEffectSequence::IonDamage
        }
        OriginalTacticalImpactStage::Damage if weapon.original_impact_code() == 3 => {
            OriginalTacticalEffectSequence::TurbolaserDamage
        }
        OriginalTacticalImpactStage::Damage => OriginalTacticalEffectSequence::StandardDamage,
        OriginalTacticalImpactStage::Destroyed => OriginalTacticalEffectSequence::Destroyed,
    }
}

/// `FUN_005d3e90` state bits: `0x04`/`0x08` and `0x10`/`0x20`/`0x40` are exclusive groups.

pub fn queue_original_tactical_impact(
    effects: &mut Vec<TacticalImpactEffect>,
    target: usize,
    weapon: WeaponKind,
    stage: OriginalTacticalImpactStage,
) -> bool {
    let sequence = original_tactical_effect_sequence(weapon, stage);
    let incoming = sequence as u8;
    if let Some(current) = effects.iter_mut().find(|effect| effect.target == target) {
        let active = current.sequence as u8;
        if incoming <= active
            || (incoming & 0x70 != 0 && active & 0x70 != 0)
            || (incoming & 0x0c != 0 && active & 0x0c != 0)
        {
            return false;
        }
        *current = TacticalImpactEffect {
            target,
            sequence,
            frame: 0,
            frame_elapsed: 0.0,
        };
    } else {
        effects.push(TacticalImpactEffect {
            target,
            sequence,
            frame: 0,
            frame_elapsed: 0.0,
        });
    }
    true
}

pub fn queue_original_tactical_projectile(
    effects: &mut Vec<WeaponEffect>,
    ships: &[TacticalShip],
    source: usize,
    target: usize,
    weapon: WeaponKind,
    event_strength: f32,
) -> bool {
    let (Some(source_ship), Some(target_ship)) = (ships.get(source), ships.get(target)) else {
        return false;
    };
    let profile = original_projectile_profile(
        weapon,
        event_strength,
        source_ship
            .tactical_resource
            .map(|resource| resource.tactical_ordinal),
        source_ship.identity.is_alliance,
    );
    effects.push(WeaponEffect {
        source,
        target,
        kind: weapon,
        source_position: source_ship.source_position,
        fallback_source: [source_ship.x, source_ship.y],
        target_position: target_ship.source_position,
        fallback_target: [target_ship.x, target_ship.y],
        longitudinal_scale: profile.longitudinal_scale,
        shape_variant: profile.shape_variant,
        color_selector: profile.color_selector,
        elapsed: 0.0,
        duration: profile.duration,
    });
    true
}

pub fn queue_original_fighter_projectile(
    effects: &mut Vec<WeaponEffect>,
    fighters: &[TacticalFighter],
    source: usize,
    target_ship: Option<(usize, &TacticalShip)>,
    target_fighter: Option<(usize, &TacticalFighter)>,
    weapon: WeaponKind,
    event_strength: f32,
) -> bool {
    let Some(source_fighter) = fighters.get(source) else {
        return false;
    };
    let (target, target_position, fallback_target) = if let Some((index, ship)) = target_ship {
        (index, ship.source_position, [ship.x, ship.y])
    } else if let Some((_index, fighter)) = target_fighter {
        (usize::MAX, fighter.source_position, [fighter.x, fighter.y])
    } else {
        return false;
    };
    let profile = original_projectile_profile(
        weapon,
        event_strength,
        source_fighter
            .tactical_resource
            .map(|resource| resource.tactical_ordinal),
        source_fighter.identity.is_alliance,
    );
    effects.push(WeaponEffect {
        source: usize::MAX,
        target,
        kind: weapon,
        source_position: source_fighter.source_position,
        fallback_source: [source_fighter.x, source_fighter.y],
        target_position,
        fallback_target,
        longitudinal_scale: profile.longitudinal_scale,
        shape_variant: profile.shape_variant,
        color_selector: profile.color_selector,
        elapsed: 0.0,
        duration: profile.duration,
    });
    true
}

/// port: the 2D fallback target is the arena centre; the source position comes from `FUN_005d3de0`.

pub fn queue_original_fighter_to_death_star_projectile(
    effects: &mut Vec<WeaponEffect>,
    fighters: &[TacticalFighter],
    source: usize,
    death_star: TacticalDeathStar,
    weapon: WeaponKind,
    event_strength: f32,
) -> bool {
    let Some(source_fighter) = fighters.get(source) else {
        return false;
    };
    let profile = original_projectile_profile(
        weapon,
        event_strength,
        source_fighter
            .tactical_resource
            .map(|resource| resource.tactical_ordinal),
        source_fighter.identity.is_alliance,
    );
    effects.push(WeaponEffect {
        source: usize::MAX,
        target: usize::MAX,
        kind: weapon,
        source_position: source_fighter.source_position,
        fallback_source: [source_fighter.x, source_fighter.y],
        target_position: death_star.source_position,
        fallback_target: [ARENA_WIDTH * 0.5, ARENA_HEIGHT * 0.5],
        longitudinal_scale: profile.longitudinal_scale,
        shape_variant: profile.shape_variant,
        color_selector: profile.color_selector,
        elapsed: 0.0,
        duration: profile.duration,
    });
    true
}

/// `FUN_005d3ac0` shared field slot; `FUN_005b24d0` four gravity targets, one tractor target.

pub fn set_original_tactical_field(
    effects: &mut Vec<TacticalFieldEffect>,
    source: usize,
    target: usize,
    kind: OriginalTacticalFieldKind,
    active: bool,
) -> bool {
    if active {
        let duplicate = effects.iter().any(|effect| {
            effect.target == target
                && match kind {
                    OriginalTacticalFieldKind::Tractor => effect.tractor_sources.contains(&source),
                    OriginalTacticalFieldKind::Gravity => effect.gravity_sources.contains(&source),
                }
        });
        if duplicate {
            return false;
        }
        let source_assignments = effects
            .iter()
            .filter(|effect| match kind {
                OriginalTacticalFieldKind::Tractor => effect.tractor_sources.contains(&source),
                OriginalTacticalFieldKind::Gravity => effect.gravity_sources.contains(&source),
            })
            .count();
        let source_limit = match kind {
            OriginalTacticalFieldKind::Tractor => 1,
            OriginalTacticalFieldKind::Gravity => 4,
        };
        if source_assignments >= source_limit {
            return false;
        }
    }
    let existing = effects.iter().position(|effect| effect.target == target);
    if existing.is_none() && !active {
        return false;
    }
    let index = existing.unwrap_or_else(|| {
        effects.push(TacticalFieldEffect {
            target,
            tractor_sources: Vec::new(),
            gravity_sources: Vec::new(),
            frame: 0,
            frame_elapsed: 0.0,
        });
        effects.len() - 1
    });
    let effect = &mut effects[index];
    let previous = effect.visible_kind();
    let sources = match kind {
        OriginalTacticalFieldKind::Tractor => &mut effect.tractor_sources,
        OriginalTacticalFieldKind::Gravity => &mut effect.gravity_sources,
    };
    if active {
        sources.push(source);
    } else {
        let Some(source_index) = sources.iter().position(|candidate| *candidate == source) else {
            return false;
        };
        sources.remove(source_index);
    }
    let current = effect.visible_kind();
    if current != previous {
        effect.frame = 0;
        effect.frame_elapsed = 0.0;
    }
    if current.is_none() {
        effects.remove(index);
    }
    true
}

pub fn remove_tractor_source(effects: &mut Vec<TacticalFieldEffect>, source: usize) {
    for effect in effects.iter_mut() {
        let previous = effect.visible_kind();
        effect
            .tractor_sources
            .retain(|candidate| *candidate != source);
        if effect.visible_kind() != previous {
            effect.frame = 0;
            effect.frame_elapsed = 0.0;
        }
    }
    effects.retain(|effect| effect.visible_kind().is_some());
}

/// `FUN_005b1770` hull scaling less 25% per tractor hit (`FUN_005b1740`).

pub fn original_effective_tractor_power(ship: &TacticalShip) -> f32 {
    if !ship.alive || !ship.subsystem_capacity.tractor || ship.tractor_beam_power <= 0.0 {
        return 0.0;
    }
    let hull_ratio = if ship.hull_max > 0 {
        ship.hull_current.max(0) as f32 / ship.hull_max as f32
    } else {
        0.0
    };
    (ship.tractor_beam_power * hull_ratio
        - ship.tractor_beam_power * 0.25 * f32::from(ship.subsystem_damage.tractor))
    .max(0.0)
}

/// Recover the active maneuver contribution returned by `FUN_005ad750` and
/// `FUN_005afb70`. Missing state records return one. Active records return
/// `9 - value`, clamped to the inclusive source range one through nine.
pub fn original_engine_mode_bonus(maneuver_state_value: Option<f32>) -> f32 {
    maneuver_state_value.map_or(ORIGINAL_ENGINE_MODE_MIN_BONUS, |value| {
        (ORIGINAL_ENGINE_MODE_MAX_BONUS - value).clamp(
            ORIGINAL_ENGINE_MODE_MIN_BONUS,
            ORIGINAL_ENGINE_MODE_MAX_BONUS,
        )
    })
}

/// `FUN_005b2f30` multiplies effective engine power by the non-negative dot
/// product between the current and desired retained-mode direction vectors.
pub fn original_movement_alignment(ship: &TacticalShip) -> f32 {
    ship.source_forward
        .normalized()
        .dot(ship.source_desired_forward.normalized())
        .clamp(0.0, 1.0)
}

/// Exact `FUN_005b2e60` overlap test. The source first compares the absolute Y
/// separation against the sum of vertical radii, then compares squared X/Z
/// distance against half the sum of planar diameters.
pub fn original_tactical_objects_overlap(first: &TacticalShip, second: &TacticalShip) -> bool {
    let (Some(first_envelope), Some(second_envelope)) = (
        first.source_collision_envelope,
        second.source_collision_envelope,
    ) else {
        return false;
    };
    if (first.source_position.y - second.source_position.y).abs()
        >= first_envelope.vertical_radius + second_envelope.vertical_radius
    {
        return false;
    }
    let x = first.source_position.x - second.source_position.x;
    let z = first.source_position.z - second.source_position.z;
    let planar_radius = (first_envelope.planar_diameter + second_envelope.planar_diameter) * 0.5;
    x.mul_add(x, z * z) < planar_radius * planar_radius
}

/// Formation ordering recovered from `FUN_005c81d0`. The source stores the
/// negative of this value in an ascending ordered list, so the effective order
/// is greatest maximum weapon range first and greatest available sublight
/// power second. Stable fleet order resolves exact ties.
pub fn original_formation_capability_score(ship: &TacticalShip) -> f32 {
    let maximum_weapon_range = ship
        .weapon_ranges
        .laser_cannon
        .max(ship.weapon_ranges.ion_cannon)
        .max(ship.weapon_ranges.turbolaser);
    let undamaged_power =
        ship.sublight_engine_power + original_engine_mode_bonus(ship.maneuver_state_value);
    let available_sublight = (undamaged_power
        - undamaged_power * 0.25 * f32::from(ship.subsystem_damage.engines))
    .max(0.0);
    maximum_weapon_range.mul_add(ORIGINAL_FORMATION_RANGE_PRIORITY, available_sublight)
}

/// A force's band of distance to its target (`+0x30`, `+0x34` of its
/// tactic object), kept by its lead unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalEngagementBand {
    pub leader: TacticalUnitRef,
    pub min: f32,
    pub max: f32,
    /// The object's `+0xc`: Surround's 5.0, Stand Off's reach beyond the
    /// target's or a third into its band. A fighter force steps by it
    /// (`FUN_005c6610`).
    pub step: f32,
}

/// Which capital force a mover belongs to: a group slot (`+0x164`) of a
/// side, or a ship's force of its own (`FUN_005aded0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalForceKey {
    Slot { attacker: bool, slot: u8 },
    Ship(usize),
}

/// A capital force's mover (`+0x54`, built by `FUN_005cb1a0`): Stand Off's
/// `FUN_005ec310` (class `0x66da08`) or Surround's `FUN_005eb860` (class
/// `0x66d9a0`), both on the base `FUN_005ea030`. Its anchor moves; the
/// members steer to their places round it (`FUN_005b2f30`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalForceMover {
    pub force: TacticalForceKey,
    pub tactic: TacticalTactic,
    /// `+0x3c`: set to the lead unit's position when the mover is built.
    pub anchor: TacticalWorldPosition,
    /// `+0x1c`.
    pub forward: TacticalWorldVector,
    /// `+0x2c`: the heading the step turns toward.
    pub heading: TacticalWorldVector,
    /// Stand Off's slot axis: the slots' offsets turn with each applied turn
    /// (`FUN_005ead90`); flips leave them (`0x005eb1a0`).
    pub lateral: TacticalWorldVector,
    /// `+0x78`: zero when the goal is cleared or reached.
    pub velocity: TacticalWorldVector,
    /// `+0x68`: the last step's raw delta to its point.
    pub last_delta: TacticalWorldVector,
    /// `+0x124`: the slowest positive member speed, once a turn.
    pub speed: f32,
    /// `+0xa8`: the shortest member turn rate, once a turn.
    pub turn_rate: f32,
    /// `+0x134`: turns in a row with the anchor unchanged (`+0xc8`).
    pub still_turns: u32,
    /// The angle Stand Off's last step turned its slots by (`FUN_005ead90`),
    /// 0 when it did not turn.
    pub slot_turn: f32,
    pub last_turn_anchor: TacticalWorldPosition,
}

/// `DAT_0066d84c`: the anchor moves at 0.95 of the force's speed, so the
/// members catch up.
pub const ORIGINAL_FORMATION_SPEED_SHARE: f32 = 0.95;
/// `DAT_0066d844`: the anchor's and a member's vertical share of speed.
pub const ORIGINAL_FORMATION_VERTICAL_SHARE: f32 = 0.4;
/// `DAT_006bd884` (`0x5e9de0`): Stand Off turns its formation only beyond
/// pi / 12.
pub const ORIGINAL_STAND_OFF_TURN_DEAD_BAND: f32 = std::f32::consts::PI / 12.0;
/// `DAT_006bc4b0` (`0x5b0c70`): a member turns only beyond pi / 48.
pub const ORIGINAL_MEMBER_TURN_DEAD_BAND: f32 = std::f32::consts::PI / 48.0;
/// `DAT_0066c508`: a member snaps to its place only when it faces its
/// heading this well.
pub const ORIGINAL_MEMBER_SNAP_ALIGNMENT: f32 = 0.707;
/// `DAT_0066d850`: a member within 1.0 (squared) of its place is there.
pub const ORIGINAL_MEMBER_PLACE_RADIUS_SQUARED: f32 = 1.0;
/// `FUN_005cc300`'s step radius for a route or the withdrawal.
pub const ORIGINAL_ROUTE_ARRIVAL_RADIUS: f32 = 1.0;
/// `FUN_005cc300`'s step radius when closing on a target.
pub const ORIGINAL_CLOSING_ARRIVAL_RADIUS: f32 = 2.0;
/// `FUN_005ea740`'s starting turn rate, above any member's.
pub const ORIGINAL_FORMATION_TURN_RATE_START: f32 = 99.0;
/// `+0x134` at which a still formation's members face their target with
/// their strongest arc (`FUN_005b4d40`).
pub const ORIGINAL_STILL_TURNS_TO_FACE: u32 = 5;

/// A member's own movement state (unit `+0x40c`..`+0x414`, zeroed by
/// `FUN_005b0f70`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TacticalMemberState {
    /// `+0x414`: its last vertical step was held (`FUN_005b2f30`).
    pub blocked: bool,
    /// `+0x40c`: the last detour's index (`FUN_005ea740`).
    pub detour_index: u8,
    /// `+0x410`: the turn of its last detour.
    pub detour_stamp: f32,
}

/// A member's detour record (the mover's `+0xec` list, `FUN_005e9cc0`): a
/// world offset it steers along in place of its slot (`0x5eb040`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalForceDetour {
    pub force: TacticalForceKey,
    pub ship: usize,
    pub offset: TacticalWorldVector,
}

/// `FUN_005ea740`'s four detour offsets, by index.
pub const ORIGINAL_DETOUR_OFFSETS: [[f32; 3]; 4] = [
    [20.0, 0.0, 0.0],
    [-20.0, 10.0, 5.0],
    [5.0, 0.0, 20.0],
    [0.0, -10.0, -20.0],
];
/// `DAT_0066d858`: a detour lasts more than 12 turns.
pub const ORIGINAL_DETOUR_TURNS: f32 = 12.0;

/// A nearby capital ship for the height bands: its height, vertical radius
/// (`+0x8`) and object id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalHeightNeighbour {
    pub y: f32,
    pub half_height: f32,
    pub id: usize,
}

/// `DAT_0066c534`: an object's reach in the neighbour search is half its
/// speed plus 5/9 of its planar diameter (vt `+0x8c`).
pub const ORIGINAL_NEIGHBOUR_DIAMETER_SHARE: f32 = 0.555_555_6;
/// `DAT_0066d03c`: the search's vertical slack, 0.4 of a frame's travel.
pub const ORIGINAL_NEIGHBOUR_VERTICAL_SLACK: f32 = 0.4;

/// `FUN_005b2f30`'s vertical step between the nearby ships' bands
/// (`+0x404` floor, `+0x408` ceiling): ships above lower the ceiling to
/// their y less both half-heights, ships below raise the floor; one at the
/// same height goes above or below by id and by the room on each side.
/// Pinned inside both, the member is held when it does not fit, else it
/// steps by `(ceiling - floor) / 2 - y` (the code's own sum); inside one it
/// steps out; otherwise it moves toward `dy`, no farther than the band.
/// Returns the new y and whether the step was held.
pub fn original_height_band_step(
    own: TacticalHeightNeighbour,
    neighbours: &[TacticalHeightNeighbour],
    dy: f32,
    step: f32,
) -> (f32, bool) {
    let band = original_height_band(own, neighbours);
    let y = own.y;
    if band.inside_ceiling && band.inside_floor {
        if 2.0 * own.half_height > band.ceiling - band.floor {
            return (y, true);
        }
        return (y + ((band.ceiling - band.floor) * 0.5 - y).clamp(-step, step), false);
    }
    if band.inside_ceiling {
        return (y - (band.ceiling - y).abs().min(step), false);
    }
    if band.inside_floor {
        return (y + (y - band.floor).abs().min(step), false);
    }
    if dy == 0.0 {
        return (y, false);
    }
    let mut travel = dy.abs().min(step);
    if dy > 0.0 && band.above {
        travel = travel.min(band.ceiling - y);
    } else if dy < 0.0 && band.below {
        travel = travel.min(y - band.floor);
    }
    (y + travel.copysign(dy), travel == 0.0)
}

/// The nearby ships' floor (`+0x404`) and ceiling (`+0x408`) of a unit, and
/// which of them it is inside (`FUN_005b2f30`, `0x5b8f70`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalHeightBand {
    pub floor: f32,
    pub ceiling: f32,
    pub above: bool,
    pub below: bool,
    pub inside_ceiling: bool,
    pub inside_floor: bool,
}

/// The band both frames build: ships above lower the ceiling to their y
/// less both half-heights, ships below raise the floor; one at the same
/// height goes above or below by id and by the room on each side.
pub fn original_height_band(
    own: TacticalHeightNeighbour,
    neighbours: &[TacticalHeightNeighbour],
) -> TacticalHeightBand {
    let mut band = TacticalHeightBand {
        floor: -f32::MAX,
        ceiling: f32::MAX,
        above: false,
        below: false,
        inside_ceiling: false,
        inside_floor: false,
    };
    let put = |band: &mut TacticalHeightBand, neighbour: TacticalHeightNeighbour, up: bool| {
        if up {
            band.above = true;
            band.ceiling = band
                .ceiling
                .min(neighbour.y - neighbour.half_height - own.half_height);
            if own.y > band.ceiling {
                band.inside_ceiling = true;
            }
        } else {
            band.below = true;
            band.floor = band
                .floor
                .max(neighbour.y + neighbour.half_height + own.half_height);
            if own.y < band.floor {
                band.inside_floor = true;
            }
        }
    };
    let mut level = Vec::new();
    for &neighbour in neighbours {
        let rise = neighbour.y - own.y;
        if rise > 0.0 {
            put(&mut band, neighbour, true);
        } else if rise < 0.0 {
            put(&mut band, neighbour, false);
        } else {
            level.push(neighbour);
        }
    }
    for neighbour in level {
        let up = if neighbour.id > own.id && !band.below {
            true
        } else if neighbour.id <= own.id && !band.above {
            false
        } else if band.above && band.below {
            own.y - band.floor > band.ceiling - own.y
        } else {
            band.above
        };
        put(&mut band, neighbour, up);
    }
    band
}

/// `0x5b8f70`'s y step (`0x5b949c`..`0x5b969d`, the frame of the
/// `FUN_005b9c60` class): a squadron only leaves a
/// band it is inside, at `step`: pinned inside both it holds (the fit path
/// is dead, as in the capital code); inside the ceiling it steps down,
/// inside the floor up; otherwise its y is left to its move.
pub fn original_fighter_band_step(
    own: TacticalHeightNeighbour,
    neighbours: &[TacticalHeightNeighbour],
    step: f32,
) -> f32 {
    let band = original_height_band(own, neighbours);
    let y = own.y;
    if band.inside_ceiling && band.inside_floor {
        if 2.0 * own.half_height > band.ceiling - band.floor {
            return y;
        }
        return y + ((band.ceiling - band.floor) * 0.5 - y).clamp(-step, step);
    }
    if band.inside_ceiling {
        return y - (y - band.ceiling).min(step);
    }
    if band.inside_floor {
        return y + (band.floor - y).min(step);
    }
    y
}

/// The fighter mover's slot offsets (`0x5ebee0`, slot `+0x30`) for the
/// member that makes the count `n` = 2..31, with forward (0, 0, 1); they
/// repeat every 30. Read from an x87 trace of the routine: `k = 1 - m ×
/// (n >> 1)` with `m = ftol(±3 × -1.5)`, `θ = kπ`, the forward turned by
/// `2θ/3`, `θ/2` and `0.4θ` (`DAT_0066d8b0`, `DAT_0066d86c`,
/// `DAT_0066d8b4`). A forward of (0, 0, -1) negates them.
pub const ORIGINAL_FIGHTER_SLOT_OFFSETS: [[f32; 3]; 30] = [
    [-0.8090170, -0.5877852, 0.0],
    [-0.5000000, 0.8660254, 0.0],
    [-0.1045285, -0.9945219, 0.0],
    [0.3090170, 0.9510565, 0.0],
    [0.6691306, -0.7431448, 0.0],
    [0.9135454, 0.4067366, 0.0],
    [1.0000000, 0.0, 0.0],
    [0.9135454, -0.4067366, 0.0],
    [0.6691306, 0.7431448, 0.0],
    [0.3090170, -0.9510565, 0.0],
    [-0.1045285, 0.9945219, 0.0],
    [-0.5000000, -0.8660254, 0.0],
    [-0.8090170, 0.5877852, 0.0],
    [-0.9781476, -0.2079117, 0.0],
    [-0.9781476, -0.2079117, 0.0],
    [-0.8090170, 0.5877852, 0.0],
    [-0.5000000, -0.8660254, 0.0],
    [-0.1045285, 0.9945219, 0.0],
    [0.3090170, -0.9510565, 0.0],
    [0.6691306, 0.7431448, 0.0],
    [0.9135454, -0.4067366, 0.0],
    [1.0000000, 0.0, 0.0],
    [0.9135454, 0.4067366, 0.0],
    [0.6691306, -0.7431448, 0.0],
    [0.3090170, 0.9510565, 0.0],
    [-0.1045285, -0.9945219, 0.0],
    [-0.5000000, 0.8660254, 0.0],
    [-0.8090170, -0.5877852, 0.0],
    [-0.9781476, 0.2079117, 0.0],
    [-0.9781476, 0.2079117, 0.0],
];

/// A fighter force's slot offset for the member that makes its count `n`
/// (`0x5ebee0`, slot `+0x30` of the `FUN_005c72b0` mover): none for the lead
/// (`n` = 1), then the table's entry
/// `(n - 2) mod 30`.
#[must_use]
pub fn original_fighter_slot_offset(n: usize, forward_z: f32) -> TacticalWorldVector {
    if n <= 1 {
        return TacticalWorldVector::ZERO;
    }
    let [x, y, z] = ORIGINAL_FIGHTER_SLOT_OFFSETS[(n - 2) % 30];
    let sign = if forward_z < 0.0 { -1.0 } else { 1.0 };
    TacticalWorldVector {
        x: x * sign,
        y: y * sign,
        z: z * sign,
    }
}

/// A fighter force (class 4, one per group: `FUN_005adfa0`) and its mover
/// (`FUN_005c72b0`, vtables `0x66cb50` / `0x66cb30`, on `FUN_005ea030`): an
/// anchor stepped toward its point at the slowest member's speed, never
/// turning, with each squadron's place the anchor less its slot.
#[derive(Debug, Clone, PartialEq)]
pub struct TacticalFighterMover {
    pub alliance: bool,
    pub group: u8,
    /// `+0x3c`: set to the first member's position when it is added.
    pub anchor: TacticalWorldPosition,
    /// `+0x1c`.
    pub forward: TacticalWorldVector,
    /// `+0x78`.
    pub velocity: TacticalWorldVector,
    /// `+0x124`: the slowest positive member speed, once a turn.
    pub speed: f32,
    /// The members (fighter indices) and their slot offsets, in order of
    /// joining.
    pub slots: Vec<(usize, TacticalWorldVector)>,
}

/// A squadron's forward and velocity (class `0x66c028`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticalFighterFrame {
    pub forward: TacticalWorldVector,
    pub velocity: TacticalWorldVector,
}

/// `DAT_0066c6c4`: a squadron moves at least a quarter of its speed, however
/// far it faces from its heading.
pub const ORIGINAL_FIGHTER_ALIGNMENT_FLOOR: f32 = 0.25;
/// `DAT_0066d864`: within 2.0 of its place (squared 4.0) a squadron takes
/// its mover's velocity as its heading (`0x5ebe30`).
pub const ORIGINAL_FIGHTER_PLACE_RADIUS_SQUARED: f32 = 4.0;

/// `0x5b97a0`, the turn (slot `+4`) of the fighter class `0x66c028`
/// (`FUN_005b9c60`): with `q = rate × ms / (angle × 1000)`,
/// at least 1 snaps onto the heading; otherwise the level part turns by
/// `acos(f.x·h.x + f.z·h.z) × q`, its sign that of `h.z·f.x − h.x·f.z`, the
/// height moves `q` of the way, and the forward is renormalized. It has no
/// dead band.
pub fn turn_original_fighter(
    forward: TacticalWorldVector,
    heading: TacticalWorldVector,
    rate: f32,
    delta_milliseconds: f32,
) -> TacticalWorldVector {
    let angle = source_direction_angle(forward, heading);
    if angle == 0.0 || rate == 0.0 || delta_milliseconds == 0.0 {
        return forward;
    }
    let q = rate * delta_milliseconds / (angle * 1000.0);
    if q >= 1.0 {
        return heading.normalized();
    }
    let f = forward.normalized();
    let h = heading.normalized();
    let level = (f.x * h.x + f.z * h.z).clamp(-1.0, 1.0).acos() * q;
    let level = if h.z * f.x - h.x * f.z < 0.0 { -level } else { level };
    let (sin, cos) = level.sin_cos();
    TacticalWorldVector {
        x: f.x * cos - sin * f.z,
        y: f.y + (h.y - f.y) * q,
        z: f.x * sin + cos * f.z,
    }
    .normalized()
}

/// `0x005ec0d0`, the fighter mover's step: beyond `radius` of the point the
/// anchor moves by the unit delta × speed × ms × 0.001 (`DAT_0066d894`), in
/// three dimensions, unclamped; within it the velocity is cleared. Returns
/// whether it moved (1).
pub fn step_fighter_anchor(
    mover: &mut TacticalFighterMover,
    point: TacticalWorldPosition,
    radius: f32,
    delta_milliseconds: f32,
) -> bool {
    let delta = source_delta(point, mover.anchor);
    let length = delta.x.hypot(delta.y).hypot(delta.z);
    if delta == TacticalWorldVector::ZERO || length <= radius {
        mover.velocity = TacticalWorldVector::ZERO;
        return false;
    }
    let unit = delta.normalized();
    mover.velocity = TacticalWorldVector {
        x: unit.x * mover.speed,
        y: unit.y * mover.speed,
        z: unit.z * mover.speed,
    };
    let seconds = delta_milliseconds * 0.001;
    mover.anchor.x += mover.velocity.x * seconds;
    mover.anchor.y += mover.velocity.y * seconds;
    mover.anchor.z += mover.velocity.z * seconds;
    true
}

/// `FUN_005cd460` on any vector: the signed level turn toward `desired` at
/// `rate` radians per second, or a snap onto it once the step reaches the
/// angle. Returns the signed angle applied.
pub fn turn_original_vector(
    current: &mut TacticalWorldVector,
    desired: TacticalWorldVector,
    rate: f32,
    delta_milliseconds: f32,
) -> f32 {
    if desired == TacticalWorldVector::ZERO {
        return 0.0;
    }
    let from = current.normalized();
    let to = desired.normalized();
    let angle = source_direction_angle(from, to);
    let sign = if -from.z * to.x + to.z * from.x + to.y * from.y < 0.0 {
        -1.0
    } else {
        1.0
    };
    let step = rate * delta_milliseconds * 0.001;
    if step >= angle {
        *current = to;
        return sign * angle;
    }
    let (sin, cos) = (sign * step).sin_cos();
    *current = TacticalWorldVector {
        x: from.x * cos - sin * from.z,
        y: from.y,
        z: from.x * sin + cos * from.z,
    }
    .normalized();
    sign * step
}

/// Surround's band (`0x005edf00`): 0 to 5.0 × 0.5 (`DAT_0066daa8`,
/// `DAT_0066daac`).
pub const ORIGINAL_SURROUND_BAND_MAX: f32 = 2.5;
/// Surround's step toward the target (slot `+0xc`, `DAT_0066cc94`).
pub const ORIGINAL_SURROUND_APPROACH_STEP: f32 = 5.0;
/// `FUN_005cc300` compares `r - DAT_0066d008` (-2.0) with the band.
pub const ORIGINAL_ENGAGEMENT_MARGIN: f32 = 2.0;
/// Stand Off's smallest step (`DAT_0066d018`) and the mover's radius.
pub const ORIGINAL_STAND_OFF_MIN_STEP: f32 = 2.0;
/// A Stand Off force too close backs off by `min × DAT_0066d00c`.
pub const ORIGINAL_STAND_OFF_BACK_OFF: f32 = -1.1;
/// Stand Off's band floor (`DAT_0066daa8`).
pub const ORIGINAL_STAND_OFF_BAND_FLOOR: f32 = 5.0;
/// Stand Off's max when its band is empty: 5.0 + 5.0 (`DAT_0066daa8`,
/// `0x005ee1bd`).
pub const ORIGINAL_STAND_OFF_EMPTY_MAX: f32 = 10.0;

/// `DAT_006bd88c`: 5.0 (`DAT_0066d840`) times 2.5 (`DAT_0066d85c`), set by
/// the initializer `0x005e9db0`.
pub const ORIGINAL_SURROUND_SPACING: f32 = 12.5;

/// `0x005eb8d0`'s 22 directions by `index % 22` (table `0x5ebc30`; cases 15
/// and 19 share `0x005ebb04`), before `0x005b50b0` normalizes them.
const ORIGINAL_SURROUND_DIRECTIONS: [[f32; 3]; 22] = [
    [1.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [1.0, 1.0, 1.0],
    [-1.0, -1.0, -1.0],
    [-1.0, 1.0, 1.0],
    [1.0, -1.0, -1.0],
    [1.0, -1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
    [0.0, -1.0, 1.0],
    [-1.0, 1.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, -1.0, -1.0],
    [0.0, 1.0, 1.0],
    [-1.0, -1.0, 0.0],
];

/// The Surround formation's slot for member `index` (1 for the lead unit,
/// `0x005ea250`), from `0x005eb8d0` (run in an emulator): its direction,
/// normalized, times `(floor(index / 22) * 0.25 + 1) * DAT_006bd88c`
/// (`DAT_0066d8a8` = 0.25, `DAT_0066d878` = -1.0). World axes: it reads no
/// orientation.
#[must_use]
pub fn original_surround_slot(index: usize) -> TacticalWorldVector {
    let [x, y, z] = ORIGINAL_SURROUND_DIRECTIONS[index % 22];
    let direction = TacticalWorldVector { x, y, z }.normalized();
    let rings = (index / 22) as f32;
    let length = rings.mul_add(0.25, 1.0) * ORIGINAL_SURROUND_SPACING;
    TacticalWorldVector {
        x: direction.x * length,
        y: direction.y * length,
        z: direction.z * length,
    }
}

/// A side's vector (`+0x48`, `FUN_005be160`): the Alliance (side 0, class
/// `0x66c7a0`) looks along +z, the Empire (side 1, `0x66c778`) along -z.
pub fn original_side_vector(is_alliance: bool) -> TacticalWorldVector {
    TacticalWorldVector {
        x: 0.0,
        y: 0.0,
        z: if is_alliance { 1.0 } else { -1.0 },
    }
}

/// Exact alternating slot sequence from `FUN_005ec400`: leader center, first
/// follower right one spacing, second follower left one spacing, then right
/// and left at each increasing multiple.
pub fn original_formation_lateral_offset(member_ordinal: usize, spacing: f32) -> f32 {
    if member_ordinal == 0 {
        return 0.0;
    }
    let magnitude = member_ordinal.div_ceil(2) as f32 * spacing;
    if member_ordinal & 1 == 1 {
        magnitude
    } else {
        -magnitude
    }
}

/// `FUN_005b9c60` installs pi / 12 for the slowest hulls. Faster hulls use
/// pi * 0.5 * clamp(raw + 1, 9) / 9 radians per second.
pub fn original_capital_turn_rate(maneuverability: u32) -> f32 {
    let effective = maneuverability.saturating_add(1) as f32;
    if effective > 1.0 {
        std::f32::consts::PI * 0.5 * effective.min(ORIGINAL_CAPITAL_MANEUVERABILITY_MAX)
            / ORIGINAL_CAPITAL_MANEUVERABILITY_MAX
    } else {
        ORIGINAL_CAPITAL_TURN_RATE_FALLBACK
    }
}

pub fn source_direction_angle(current: TacticalWorldVector, desired: TacticalWorldVector) -> f32 {
    let current = current.normalized();
    let desired = desired.normalized();
    current.dot(desired).clamp(-1.0, 1.0).acos()
}

/// Apply `FUN_005cd460`'s signed XZ turn and snap branch.
pub fn turn_original_tactical_ship(ship: &mut TacticalShip, delta_milliseconds: f32) {
    if ship.source_desired_forward == TacticalWorldVector::ZERO {
        return;
    }
    let angle = source_direction_angle(ship.source_forward, ship.source_desired_forward);
    if angle <= f32::EPSILON {
        return;
    }
    let turn_step = original_capital_turn_rate(ship.maneuverability) * delta_milliseconds * 0.001;
    if turn_step >= angle {
        ship.source_forward = ship.source_desired_forward.normalized();
        return;
    }

    let current = ship.source_forward.normalized();
    let desired = ship.source_desired_forward.normalized();
    let signed_angle =
        if -current.z * desired.x + desired.z * current.x + desired.y * current.y < 0.0 {
            -turn_step
        } else {
            turn_step
        };
    let (sin, cos) = signed_angle.sin_cos();
    ship.source_forward = TacticalWorldVector {
        x: current.x * cos - sin * current.z,
        y: current.y,
        z: current.x * sin + cos * current.z,
    }
    .normalized();
}

/// `FUN_005b17f0` engine power less 25% per engine hit; `FUN_005b16b0` ratio as a percentage.

pub fn original_tactical_mobility(
    ships: &[TacticalShip],
    field_effects: &[TacticalFieldEffect],
    target: usize,
) -> Option<TacticalSubsystemMobility> {
    let ship = ships.get(target)?;
    let engine_mode_bonus = original_engine_mode_bonus(ship.maneuver_state_value);
    if !ship.subsystem_capacity.engines || ship.sublight_engine_power <= 0.0 {
        return Some(TacticalSubsystemMobility {
            base_engine_power: ship.sublight_engine_power.max(0.0),
            engine_mode_bonus,
            active_tractor_power: 0.0,
            effective_engine_power: 0.0,
            engine_percent: 0,
        });
    }
    let active_tractor_power = field_effects
        .iter()
        .filter(|effect| effect.target == target)
        .flat_map(|effect| effect.tractor_sources.iter().copied())
        .filter_map(|source| ships.get(source))
        .map(original_effective_tractor_power)
        .sum::<f32>();
    let undamaged_power = (ship.sublight_engine_power + engine_mode_bonus).max(0.0);
    let effective_engine_power = (undamaged_power
        - undamaged_power * 0.25 * f32::from(ship.subsystem_damage.engines)
        - active_tractor_power)
        .max(0.0);
    let engine_percent =
        (effective_engine_power * 100.0 / ship.sublight_engine_power).clamp(0.0, 100.0) as u8;
    Some(TacticalSubsystemMobility {
        base_engine_power: ship.sublight_engine_power,
        engine_mode_bonus,
        active_tractor_power,
        effective_engine_power,
        engine_percent,
    })
}

pub fn refresh_original_subsystem_conditions(
    ships: &mut [TacticalShip],
    field_effects: &[TacticalFieldEffect],
) {
    let engine_percentages: Vec<u8> = (0..ships.len())
        .map(|target| {
            original_tactical_mobility(ships, field_effects, target)
                .map_or(0, |mobility| mobility.engine_percent)
        })
        .collect();
    for (ship, engine_percent) in ships.iter_mut().zip(engine_percentages) {
        ship.refresh_subsystem_condition();
        ship.subsystem_condition.engines = engine_percent;
    }
}

pub fn original_subsystem_repair_kind(
    damage: TacticalSubsystemDamage,
    mut ordinal: u16,
) -> Option<TacticalSubsystemKind> {
    if ordinal == 0 || ordinal > damage.total_hits() {
        return None;
    }
    // FUN_005b1490 selects uniformly among individual outstanding hits in
    // engine, shield, hyperdrive, tractor, weapon order.
    for (kind, hits) in [
        (TacticalSubsystemKind::Engines, damage.engines),
        (TacticalSubsystemKind::Shields, damage.shields),
        (TacticalSubsystemKind::Hyperdrive, damage.hyperdrive),
        (TacticalSubsystemKind::Tractor, damage.tractor),
        (TacticalSubsystemKind::Weapons, damage.weapons),
    ] {
        if ordinal <= u16::from(hits) {
            return Some(kind);
        }
        ordinal -= u16::from(hits);
    }
    None
}

/// `FUN_005b1490` draws 1–100 against the damage-control chance.

pub fn attempt_original_subsystem_repair(
    ship: &mut TacticalShip,
    ship_index: usize,
    chance_roll: u8,
    selection_roll: u16,
) -> Option<TacticalSubsystemRepair> {
    if ship.subsystem_damage.total_hits() == 0
        || ship.damage_control == 0
        || !(1..=100).contains(&chance_roll)
        || chance_roll > ship.damage_control
    {
        return None;
    }
    let kind = original_subsystem_repair_kind(ship.subsystem_damage, selection_roll)?;
    let (hits_before, hits_after) = ship.remove_subsystem_hit(kind)?;
    Some(TacticalSubsystemRepair {
        ship: ship_index,
        kind,
        hits_before,
        hits_after,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalTacticalDamageRolls {
    pub shield: u8,
    pub hull: u8,
}

/// Tactical manager random stream recovered from `FUN_0061a310` and
/// `FUN_005a8a70`. Every tactical consumer advances this one stream in event
/// order instead of deriving independent hashes from the frame number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalTacticalRng {
    pub state: u32,
}

impl OriginalTacticalRng {
    #[must_use]
    pub const fn new(state: u32) -> Self {
        Self { state }
    }

    #[must_use]
    pub const fn state(self) -> u32 {
        self.state
    }

    /// `FUN_0061a310` MSVC rand: state * 0x343fd + 0x269ec3, bits 16–30.

    pub fn next_u15(&mut self) -> u16 {
        self.state = self.state.wrapping_mul(0x343f_d).wrapping_add(0x269e_c3);
        ((self.state >> 16) & 0x7fff) as u16
    }

    pub fn inclusive(&mut self, minimum: u32, maximum: u32) -> u32 {
        debug_assert!(minimum <= maximum);
        let span = maximum.saturating_sub(minimum).saturating_add(1);
        minimum + u32::from(self.next_u15()) % span
    }
}

/// `FUN_005b54d0` draws 0–100 for the shield and the hull subsystem.

pub fn original_tactical_damage_rolls(
    ships: &[TacticalShip],
    target: usize,
    damage: i32,
    rng: &mut OriginalTacticalRng,
) -> OriginalTacticalDamageRolls {
    let Some(ship) = ships.get(target) else {
        return OriginalTacticalDamageRolls { shield: 0, hull: 0 };
    };
    if !ship.alive || ship.hull_current <= 0 || damage <= 0 {
        return OriginalTacticalDamageRolls { shield: 0, hull: 0 };
    }
    let shield = if ship.shield > 0 {
        u8::try_from(rng.inclusive(0, 100)).unwrap_or(100)
    } else {
        0
    };
    let hull = if ship.shield.saturating_sub(damage) < 0 {
        u8::try_from(rng.inclusive(0, 100)).unwrap_or(100)
    } else {
        0
    };
    OriginalTacticalDamageRolls { shield, hull }
}

/// `FUN_005b54d0` score bands (`ghidra/notes/tactical-subsystem-damage-path.md`).

pub fn original_hull_subsystem_kind(score: f32) -> Option<TacticalSubsystemKind> {
    if score <= 50.0 {
        None
    } else if score <= 65.0 {
        Some(TacticalSubsystemKind::Shields)
    } else if score <= 80.0 {
        Some(TacticalSubsystemKind::Weapons)
    } else if score <= 90.0 {
        Some(TacticalSubsystemKind::Tractor)
    } else if score <= 95.0 {
        Some(TacticalSubsystemKind::Engines)
    } else if score <= 100.0 {
        Some(TacticalSubsystemKind::Hyperdrive)
    } else {
        None
    }
}

pub fn apply_original_capital_damage(
    ships: &mut [TacticalShip],
    field_effects: &mut Vec<TacticalFieldEffect>,
    target: usize,
    damage: i32,
    rolls: OriginalTacticalDamageRolls,
) -> OriginalTacticalImpactStage {
    original_capital_damage(ships, field_effects, target, damage, rolls, None)
}

/// A tactical hit, the capital ship's slot `+0xe0` (`FUN_005b5f50`): the
/// shield and hull take the damage at once, but each subsystem hit is posted
/// through slot `+0x28` as event `0x1e`..`0x22` (`0x005b6151`..
/// `0x005b6228`), so it is returned here to be queued.
pub fn apply_original_tactical_capital_hit(
    ships: &mut [TacticalShip],
    field_effects: &mut Vec<TacticalFieldEffect>,
    target: usize,
    damage: i32,
    rolls: OriginalTacticalDamageRolls,
) -> (OriginalTacticalImpactStage, Vec<TacticalSubsystemKind>) {
    let mut hits = Vec::new();
    let stage = original_capital_damage(ships, field_effects, target, damage, rolls, Some(&mut hits));
    (stage, hits)
}

fn original_capital_damage(
    ships: &mut [TacticalShip],
    field_effects: &mut Vec<TacticalFieldEffect>,
    target: usize,
    damage: i32,
    rolls: OriginalTacticalDamageRolls,
    mut deferred: Option<&mut Vec<TacticalSubsystemKind>>,
) -> OriginalTacticalImpactStage {
    let mut hit = |ship: &mut TacticalShip, kind: TacticalSubsystemKind| match deferred.as_mut() {
        Some(hits) => {
            hits.push(kind);
            false
        }
        None => ship.add_subsystem_hit(kind),
    };
    let (hull_before, hull_after, tractor_disabled) = {
        let Some(ship) = ships.get_mut(target) else {
            return OriginalTacticalImpactStage::Hit;
        };
        if !ship.alive || ship.hull_current <= 0 || damage <= 0 {
            return OriginalTacticalImpactStage::Hit;
        }

        let hull_before = ship.hull_current;
        let shield_before = ship.shield;
        let shield_after = shield_before.saturating_sub(damage);
        ship.shield = shield_after;

        let mut tractor_disabled = false;
        if shield_before > 0 {
            let shield_loss_percent =
                (100.0 / shield_before as f32) * (shield_before - shield_after) as f32;
            if f32::from(rolls.shield) <= shield_loss_percent.max(1.0) {
                tractor_disabled |= hit(ship, TacticalSubsystemKind::Shields);
            }
        }

        if ship.shield < 0 {
            // FUN_005b54d0 carries negative shield overflow into hull and enforces
            // at least one hull point on the shield-crossing branch.
            let hull_damage = (-ship.shield).max(1);
            ship.shield = 0;
            ship.hull_current = ship.hull_current.saturating_sub(hull_damage).max(0);
            let score = f32::from(rolls.hull)
                + (ship.hull_current - hull_before) as f32 * 100.0 / hull_before as f32;
            if let Some(kind) = original_hull_subsystem_kind(score) {
                tractor_disabled |= hit(ship, kind);
            }
        }

        if ship.hull_current == 0 {
            ship.alive = false;
        }
        (hull_before, ship.hull_current, tractor_disabled)
    };
    if tractor_disabled {
        remove_tractor_source(field_effects, target);
    }
    refresh_original_subsystem_conditions(ships, field_effects);

    if hull_after == 0 {
        OriginalTacticalImpactStage::Destroyed
    } else if hull_after < hull_before {
        OriginalTacticalImpactStage::Damage
    } else {
        OriginalTacticalImpactStage::Hit
    }
}

/// Battle outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatWinner {
    Attacker,
    Defender,
    Draw,
}

/// Result of the source Death Star trench-run branch.
///
/// `TACTICALRESULT_UPDATE` states 6 and 7 route to the successful and failed
/// films respectively. The presentation layer consumes this value once and
/// returns to the live tactical session afterward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticalTrenchRunOutcome {
    Success,
    Failure,
}

/// `FUN_005b0e50` four batteries, consumed by `FUN_005b3f10`.

pub fn original_capital_weapon_candidates(
    ships: &[TacticalShip],
    source: usize,
    targets: &[usize],
    direct_target: Option<usize>,
) -> [OriginalTacticalArcCandidates; 4] {
    let mut candidates = [OriginalTacticalArcCandidates::default(); 4];
    let Some(source_ship) = ships.get(source) else {
        return candidates;
    };
    let scale_for_hull = direct_target.is_none();

    for &target in targets {
        if direct_target.is_some_and(|direct| direct != target) {
            continue;
        }
        let Some(target_ship) = ships
            .get(target)
            .filter(|ship| ship.alive && !ship.jumping())
        else {
            continue;
        };
        let arc_index = original_tactical_weapon_arc(source_ship, target_ship).index();
        let arc = source_ship.weapon_arcs[arc_index];
        if !arc.ready {
            continue;
        }
        let delta = source_delta(target_ship.source_position, source_ship.source_position);
        let distance = delta.x.hypot(delta.y).hypot(delta.z);

        for kind in [
            WeaponKind::LaserCannon,
            WeaponKind::IonCannon,
            WeaponKind::Turbolaser,
        ] {
            let count = original_weapon_count(arc, kind);
            let range = original_weapon_range(source_ship, kind);
            if count == 0 || range <= 0.0 || distance > range {
                continue;
            }
            let strength =
                original_capital_weapon_strength(source_ship, count, kind, scale_for_hull);
            let candidate = match kind {
                WeaponKind::LaserCannon => &mut candidates[arc_index].laser,
                WeaponKind::IonCannon => &mut candidates[arc_index].ion,
                WeaponKind::Turbolaser => &mut candidates[arc_index].turbolaser,
                WeaponKind::FighterAttack => continue,
            };
            // The executable keeps the first target on an equal score.
            if candidate.strength < strength {
                *candidate = OriginalTacticalWeaponCandidate {
                    strength,
                    target: Some(target),
                };
            }
        }
    }

    candidates
}

pub fn original_tactical_integer_damage(strength: f32) -> i32 {
    if !strength.is_finite() || strength <= 0.0 {
        0
    } else {
        // The source event transports a float. The current public hull and
        // shield state remains integral, so round only at that storage edge.
        strength.round().max(1.0) as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalFighterTarget {
    Capital(usize),
    Fighter(usize),
    DeathStar,
}

pub fn original_fighter_weapon_strength(
    fighter: &TacticalFighter,
    count: u32,
    kind: WeaponKind,
    target_is_fighter: bool,
) -> f32 {
    let target_modifier = if target_is_fighter {
        match kind {
            WeaponKind::IonCannon => ORIGINAL_ION_VS_FIGHTER,
            WeaponKind::LaserCannon => ORIGINAL_LASER_VS_FIGHTER,
            WeaponKind::Turbolaser => ORIGINAL_TURBOLASER_VS_FIGHTER,
            WeaponKind::FighterAttack => 0.0,
        }
    } else {
        match kind {
            WeaponKind::LaserCannon => ORIGINAL_LASER_VS_CAPITAL,
            WeaponKind::IonCannon | WeaponKind::Turbolaser => 1.0,
            WeaponKind::FighterAttack => 0.0,
        }
    };
    count as f32 * fighter.weapon_hull_factor() * ORIGINAL_CAPITAL_WEAPON_STRENGTH * target_modifier
}

pub fn original_fighter_weapon_range(fighter: &TacticalFighter, kind: WeaponKind) -> f32 {
    match kind {
        WeaponKind::LaserCannon => fighter.weapon_ranges.laser_cannon,
        WeaponKind::IonCannon => fighter.weapon_ranges.ion_cannon,
        WeaponKind::Turbolaser => fighter.weapon_ranges.turbolaser,
        WeaponKind::FighterAttack => fighter.torpedo_range,
    }
}

pub fn apply_original_fighter_damage(
    fighters: &mut [TacticalFighter],
    target: usize,
    event_strength: f32,
    source_maneuverability: f32,
) -> OriginalTacticalImpactStage {
    let Some(fighter) = fighters.get_mut(target) else {
        return OriginalTacticalImpactStage::Hit;
    };
    if !fighter.alive || fighter.hull_current <= 0.0 || event_strength <= 0.0 {
        return OriginalTacticalImpactStage::Hit;
    }

    // FUN_005b5100, FUN_005b5b60, and FUN_005b5f50 divide incoming damage
    // by target/source maneuverability for a fighter recipient, clamped at
    // one tenth before division.
    let maneuver_ratio =
        (fighter.effective_maneuverability() / source_maneuverability.max(1.0)).max(0.1);
    let damage = (event_strength / maneuver_ratio).max(1.0);
    let hull_before = fighter.hull_current;
    fighter.shield -= damage;
    if fighter.shield < 0.0 {
        let overflow = -fighter.shield;
        fighter.shield = 0.0;
        fighter.hull_current = (fighter.hull_current - overflow.max(1.0)).max(0.0);
        fighter.refresh_craft_count();
    }

    if fighter.hull_current <= 0.0 {
        OriginalTacticalImpactStage::Destroyed
    } else if fighter.hull_current < hull_before {
        OriginalTacticalImpactStage::Damage
    } else {
        OriginalTacticalImpactStage::Hit
    }
}

pub fn queue_original_capital_to_fighter_projectile(
    effects: &mut Vec<WeaponEffect>,
    ships: &[TacticalShip],
    fighters: &[TacticalFighter],
    source: usize,
    target: usize,
    weapon: WeaponKind,
    event_strength: f32,
) -> bool {
    let (Some(source_ship), Some(target_fighter)) = (ships.get(source), fighters.get(target))
    else {
        return false;
    };
    let profile = original_projectile_profile(
        weapon,
        event_strength,
        source_ship
            .tactical_resource
            .map(|resource| resource.tactical_ordinal),
        source_ship.identity.is_alliance,
    );
    effects.push(WeaponEffect {
        source,
        target: usize::MAX,
        kind: weapon,
        source_position: source_ship.source_position,
        fallback_source: [source_ship.x, source_ship.y],
        target_position: target_fighter.source_position,
        fallback_target: [target_fighter.x, target_fighter.y],
        longitudinal_scale: profile.longitudinal_scale,
        shape_variant: profile.shape_variant,
        color_selector: profile.color_selector,
        elapsed: 0.0,
        duration: profile.duration,
    });
    true
}

impl BattleSession {
    /// Create a new battle session from two opposing fleets.
    ///
    /// Expands fleet composition into individual ship hulls and positions them
    /// in deployment zones (attacker left, defender right).
    #[must_use]
    pub fn new(
        world: &GameWorld,
        system: SystemKey,
        attacker: FleetKey,
        defender: FleetKey,
        player_is_attacker: bool,
        tick: u64,
        rng_seed: u32,
    ) -> Self {
        Self::new_for_fleets(
            world,
            system,
            &[attacker],
            &[defender],
            player_is_attacker,
            tick,
            rng_seed,
        )
        .session
    }

    /// A battle of several fleets per side. `FUN_0040c780` applies a
    /// battle's result to every fleet at the system (`FUN_004ffe70`), so
    /// every fleet of both sides fights. The first fleet of each side names
    /// it. Each unit's fleet is returned beside the session.
    pub fn new_for_fleets(
        world: &GameWorld,
        system: SystemKey,
        attackers: &[FleetKey],
        defenders: &[FleetKey],
        player_is_attacker: bool,
        tick: u64,
        rng_seed: u32,
    ) -> FleetBattle {
        let attacker = attackers[0];
        let defender = defenders[0];
        let system_name = world
            .systems
            .get(system)
            .map_or_else(|| "Unknown".into(), |s| s.name.clone());
        let system_picture_id = world
            .systems
            .get(system)
            .map_or(1, |system| planet_picture_id(system.dat_id));
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the term is clamped to 1.0..9.0; the draw it meets is an integer"
        )]
        let alliance_commander_term = crate::abstract_battle::trench_commander_term(world, system);
        let trench_run_success_rating = alliance_commander_term as u8;

        let mut ships = Vec::new();
        let mut fighters = Vec::new();

        let mut ship_fleets = Vec::new();
        let mut fighter_fleets = Vec::new();
        // Expand each attacker fleet, then each defender fleet, into hulls.
        for (fleets, is_attacker) in [(attackers, true), (defenders, false)] {
            for &fleet in fleets {
                Self::expand_fleet(world, fleet, is_attacker, &mut ships, &mut fighters);
                ship_fleets.resize(ships.len(), fleet);
                fighter_fleets.resize(fighters.len(), fleet);
            }
        }

        // Retain the working 2D fallback separately from the recovered source
        // world coordinates. P58 will replace each fallback only after its
        // DAT-to-tactical resource identity is proven.
        Self::auto_place_ships(&mut ships);
        Self::auto_place_fighters(&mut fighters, &ships);
        let source_layout = Self::assign_original_world_positions(&mut ships, &mut fighters);
        let navigation_sets = original_navigation_point_sets(source_layout);
        Self::assign_original_task_forces(&mut ships);
        Self::assign_original_fighter_groups(&mut fighters);
        // FUN_0040bcd0: the system's usable Death Star shields (FUN_0051c270,
        // family 0x25), whatever their side.
        let shielded = world.systems.get(system).is_some_and(|sys| {
            sys.defense_facilities.iter().any(|&key| {
                world.defense_facilities.get(key).is_some_and(|facility| {
                    facility.class_dat_id.family() == DEATH_STAR_SHIELD_FAMILY
                })
            })
        });
        let death_star = attackers
            .iter()
            .map(|&fleet| (fleet, true))
            .chain(defenders.iter().map(|&fleet| (fleet, false)))
            .find_map(|(fleet_key, is_attacker)| {
                let fleet = &world.fleets[fleet_key];
                fleet.has_death_star.then_some(TacticalDeathStar {
                    resource: DEATH_STAR_TACTICAL_RESOURCE,
                    is_attacker,
                    is_alliance: fleet.is_alliance,
                    source_position: TacticalWorldPosition {
                        x: 0.0,
                        y: 0.0,
                        z: if fleet.is_alliance {
                            source_layout.outer_negative_z
                        } else {
                            source_layout.outer_positive_z
                        },
                    },
                    hull: ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE,
                    // FUN_005af4e0 stores +0x6c = 100.0: the laser starts
                    // charged (A0 TAC-05-C017, the first tactical frame).
                    laser_charge: ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE,
                    shielded,
                    destroyed: false,
                    action_committed: false,
                })
            });

        // The original Tactical Display opens with the battle paused. The
        // Battle Alert's Take Command choice is the entry action, not a second
        // invented placement phase inside the battle viewport.
        let selected_ship = ships
            .iter()
            .position(|ship| ship.is_attacker == player_is_attacker);
        if let Some(index) = selected_ship {
            ships[index].selected = true;
        }

        let session = BattleSession {
            system,
            system_name,
            system_picture_id,
            attacker_fleet: attacker,
            defender_fleet: defender,
            attacker_is_alliance: world.fleets[attacker].is_alliance,
            player_is_attacker,
            phase: BattlePhase::Combat,
            ships,
            fighters,
            death_star,
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
            alliance_raid_sent: false,
            fighter_orbit_angles: Vec::new(),
            fighter_movers: Vec::new(),
            fighter_frames: Vec::new(),
            trench_run_outcome: None,
            trench_run_remaining_ms: None,
            trench_run_group: None,
            trench_run_success_rating,
            alliance_commander_term,
            trench_run_pending_outcome: None,
            trench_run_participants: Vec::new(),
            trench_run_saved: Vec::new(),
            trench_run_chatter: [ORIGINAL_TRENCH_RUN_SILENT_EVENT; 9],
            trench_run_chatter_cursor: 0,
            trench_run_cinematic_pending: false,
            navigation_sets,
            source_layout,
            selected_ship,
            selected_fighter_group: None,
            placement_confirmed: true,
            start_tick: tick,
            combat_tick: 0,
            tactical_rng: OriginalTacticalRng::new(rng_seed),
            weapon_effects: Vec::new(),
            impact_effects: Vec::new(),
            pending_audio_cues: Vec::new(),
            pending_voice_cues: Vec::new(),
            voice_announcer: TacticalVoiceAnnouncer::default(),
            field_effects: Vec::new(),
            subsystem_repairs: Vec::new(),
            paused: true,
            combat_speed: 1,
            step_accumulator: 0.0,
            winner: None,
            retreat_allowed: [false; 2],
            retreat_done: [false; 2],
            withdrawal_ms: None,
            ai_withdrawal_ms: None,
            diagnostic_log: Vec::new(),
        };
        FleetBattle {
            session,
            ship_fleets,
            fighter_fleets,
        }
    }

    /// Start or stop one source's tractor-field contribution on a live target.
    /// One tractor source can hold one target while a target can retain several
    /// exact sources, matching `FUN_005b23e0` through `FUN_005b25d0`.
    pub fn set_tractor_field(&mut self, source: usize, target: usize, active: bool) -> bool {
        if source >= self.ships.len()
            || target >= self.ships.len()
            || source == target
            || !self.ships[source].alive
            || !self.ships[target].alive
        {
            return false;
        }
        let changed = set_original_tactical_field(
            &mut self.field_effects,
            source,
            target,
            OriginalTacticalFieldKind::Tractor,
            active,
        );
        if changed {
            refresh_original_subsystem_conditions(&mut self.ships, &self.field_effects);
        }
        changed
    }

    /// Start or stop one source's gravity-field contribution on a live target.
    /// One gravity source can hold four exact targets. Gravity preempts tractor
    /// in each target's shared visual slot, matching `FUN_005b24d0` through
    /// `FUN_005b2550`.
    pub fn set_gravity_field(&mut self, source: usize, target: usize, active: bool) -> bool {
        if source >= self.ships.len()
            || target >= self.ships.len()
            || source == target
            || !self.ships[source].alive
            || !self.ships[target].alive
        {
            return false;
        }
        set_original_tactical_field(
            &mut self.field_effects,
            source,
            target,
            OriginalTacticalFieldKind::Gravity,
            active,
        )
    }

    /// Return the source-derived sublight state for one live participant.
    #[must_use]
    pub fn subsystem_mobility(&self, target: usize) -> Option<TacticalSubsystemMobility> {
        original_tactical_mobility(&self.ships, &self.field_effects, target)
    }

    /// Return the exact non-negative current-to-desired direction multiplier
    /// consumed by the original capital-ship movement routine.
    #[must_use]
    pub fn movement_alignment(&self, target: usize) -> Option<f32> {
        self.ships.get(target).map(original_movement_alignment)
    }

    /// Set or clear the active state-record value consumed by the recovered
    /// maneuver-mode producer. Command routing will call this once restored.
    pub fn set_maneuver_state_value(&mut self, target: usize, value: Option<f32>) -> bool {
        let Some(ship) = self.ships.get_mut(target) else {
            return false;
        };
        if value.is_some_and(|value| !value.is_finite()) {
            return false;
        }
        ship.maneuver_state_value = value;
        refresh_original_subsystem_conditions(&mut self.ships, &self.field_effects);
        true
    }
    /// A ship's force key: its group slot of 8 (`+0x164`, `0x0059f610`), or
    /// a force of its own (`FUN_005aded0`).
    pub fn force_key(&self, index: usize) -> TacticalForceKey {
        let ship = &self.ships[index];
        if ship.task_force < 8 {
            TacticalForceKey::Slot {
                attacker: ship.is_attacker,
                slot: ship.task_force,
            }
        } else {
            TacticalForceKey::Ship(index)
        }
    }

    /// The capital forces that move, each lead unit first (`0x005c6070`,
    /// `FUN_005c81d0`): as `capital_forces`, with the withdrawing ships'
    /// forces of their own (`FUN_005aded0`).
    fn moving_forces(&self) -> Vec<Vec<usize>> {
        let in_force = |ship: &TacticalShip| ship.alive && !ship.jumping();
        let mut forces = Vec::new();
        for attacker in [true, false] {
            for slot in 0_u8..8 {
                let mut members = (0..self.ships.len())
                    .filter(|&index| {
                        let ship = &self.ships[index];
                        in_force(ship) && ship.is_attacker == attacker && ship.task_force == slot
                    })
                    .collect::<Vec<_>>();
                if members.is_empty() {
                    continue;
                }
                members.sort_by(|left, right| {
                    original_formation_capability_score(&self.ships[*right])
                        .total_cmp(&original_formation_capability_score(&self.ships[*left]))
                });
                forces.push(members);
            }
        }
        for (index, ship) in self.ships.iter().enumerate() {
            if in_force(ship) && ship.task_force >= 8 {
                forces.push(vec![index]);
            }
        }
        forces
    }

    pub fn force_mover(&self, key: TacticalForceKey) -> Option<&TacticalForceMover> {
        self.force_movers.iter().find(|mover| mover.force == key)
    }

    /// `FUN_005ea740` (the mover's slot `+0x40`, once a turn): the slowest
    /// positive member speed (`0x5b17f0`, 0 with none), the shortest member
    /// turn rate (from 99), and the turns the anchor has stood still.
    fn force_mover_rates(&self, members: &[usize]) -> (f32, f32) {
        let speed = members
            .iter()
            .map(|&index| {
                original_tactical_mobility(&self.ships, &self.field_effects, index)
                    .map_or(0.0, |mobility| mobility.effective_engine_power)
            })
            .filter(|speed| *speed > 0.0)
            .fold(f32::INFINITY, f32::min);
        let turn_rate = members
            .iter()
            .map(|&index| original_capital_turn_rate(self.ships[index].maneuverability))
            .fold(ORIGINAL_FORMATION_TURN_RATE_START, f32::min);
        (if speed.is_finite() { speed } else { 0.0 }, turn_rate)
    }

    pub(crate) fn update_force_movers(&mut self) {
        self.sync_fighter_movers();
        for at in 0..self.fighter_movers.len() {
            let members = self.fighter_movers[at]
                .slots
                .iter()
                .map(|(member, _)| *member)
                .collect::<Vec<_>>();
            self.fighter_movers[at].speed = self.fighter_mover_speed(&members);
        }
        self.sync_force_movers();
        for members in self.moving_forces() {
            let key = self.force_key(members[0]);
            let (speed, turn_rate) = self.force_mover_rates(&members);
            self.update_force_detours(key, &members);
            let Some(mover) = self.force_movers.iter_mut().find(|mover| mover.force == key) else {
                continue;
            };
            mover.speed = speed;
            mover.turn_rate = turn_rate;
            if mover.anchor == mover.last_turn_anchor {
                mover.still_turns = mover.still_turns.saturating_add(1);
            } else {
                mover.still_turns = 0;
            }
            mover.last_turn_anchor = mover.anchor;
        }
    }

    /// `FUN_005ea740`'s member loop, before `+0x134` moves on: a moving
    /// member held in its climb (`+0x414`), in a still formation (velocity
    /// zero, `+0x134` at least 5), more than 12 turns after its last detour
    /// (`+0x410`, `DAT_0066d858`) takes the next one: index
    /// `(id + 1 + last) & 3`. An unblocked member's record goes once those
    /// 12 turns have passed. port: a ship's id is its index + 1, as its
    /// object id elsewhere.
    fn update_force_detours(&mut self, key: TacticalForceKey, members: &[usize]) {
        let Some(mover) = self.force_mover(key).copied() else {
            return;
        };
        if self.member_states.len() < self.ships.len() {
            self.member_states
                .resize(self.ships.len(), TacticalMemberState::default());
        }
        // `+0x87c` as the turn's updates see it.
        let now = self.step_event_stamp() as f32;
        for &index in members {
            let moving = original_tactical_mobility(&self.ships, &self.field_effects, index)
                .is_some_and(|mobility| mobility.effective_engine_power > 0.0);
            if !moving {
                continue;
            }
            let state = self.member_states[index];
            let due = now - state.detour_stamp > ORIGINAL_DETOUR_TURNS;
            if state.blocked
                && mover.velocity == TacticalWorldVector::ZERO
                && mover.still_turns >= ORIGINAL_STILL_TURNS_TO_FACE
                && due
            {
                let id = index + 1;
                let next = ((id + 1 + usize::from(state.detour_index)) & 3) as u8;
                let [x, y, z] = ORIGINAL_DETOUR_OFFSETS[usize::from(next)];
                self.member_states[index].detour_stamp = now;
                self.member_states[index].detour_index = next;
                self.force_detours
                    .retain(|detour| !(detour.force == key && detour.ship == index));
                self.force_detours.push(TacticalForceDetour {
                    force: key,
                    ship: index,
                    offset: TacticalWorldVector { x, y, z },
                });
            } else if !state.blocked && due {
                self.force_detours
                    .retain(|detour| !(detour.force == key && detour.ship == index));
            }
        }
    }

    /// Keep one mover per moving force: a force that is new, or whose tactic
    /// changed (`FUN_005cb1a0` on event `0x3d`), gets a new one at its lead
    /// unit; a force that is gone loses its own.
    fn sync_force_movers(&mut self) {
        let forces = self.moving_forces();
        let keys = forces
            .iter()
            .map(|members| self.force_key(members[0]))
            .collect::<Vec<_>>();
        self.force_movers.retain(|mover| keys.contains(&mover.force));
        self.force_detours.retain(|detour| keys.contains(&detour.force));
        for (members, key) in forces.iter().zip(keys) {
            let leader = members[0];
            let tactic = self.force_tactic(leader);
            if self
                .force_mover(key)
                .is_some_and(|mover| mover.tactic == tactic)
            {
                continue;
            }
            let (speed, turn_rate) = self.force_mover_rates(members);
            let lead = &self.ships[leader];
            // `FUN_005ea030`: forward (0, 0, 1) for a lead id below
            // 1,000,000, else (0, 0, -1). hyp: the Empire's units carry the
            // high ids, so the forward is the lead's side vector, as each
            // unit's at placement (`FUN_005ab650`).
            let forward = original_side_vector(lead.identity.is_alliance);
            let mover = TacticalForceMover {
                force: key,
                tactic,
                anchor: lead.source_position,
                forward,
                heading: forward,
                lateral: TacticalWorldVector {
                    x: forward.z,
                    y: 0.0,
                    z: -forward.x,
                },
                velocity: TacticalWorldVector::ZERO,
                last_delta: TacticalWorldVector::ZERO,
                // port: the speed and turn rate are set at once, not at
                // the next turn's `FUN_005ea740`.
                speed,
                turn_rate,
                still_turns: 0,
                slot_turn: 0.0,
                last_turn_anchor: lead.source_position,
            };
            self.force_detours.retain(|detour| detour.force != key);
            self.force_movers.retain(|existing| existing.force != key);
            self.force_movers.push(mover);
        }
    }

    /// A member's place: the anchor less its slot (`0x005eb040`). Surround's
    /// slots stand on the world axes from index 1 (`0x005eb8d0`); Stand Off's
    /// alternate along its slot axis (`FUN_005ec400`).
    pub(crate) fn force_member_place(
        &self,
        mover: &TacticalForceMover,
        members: &[usize],
        ordinal: usize,
    ) -> TacticalWorldPosition {
        let anchor = mover.anchor;
        if mover.tactic == TacticalTactic::Surround {
            let slot = original_surround_slot(ordinal + 1);
            return TacticalWorldPosition {
                x: anchor.x - slot.x,
                y: anchor.y - slot.y,
                z: anchor.z - slot.z,
            };
        }
        let spacing = members
            .iter()
            .filter_map(|index| self.ships[*index].source_collision_envelope)
            .map(|envelope| (envelope.planar_diameter * ORIGINAL_FORMATION_FOOTPRINT_SCALE).trunc())
            .fold(ORIGINAL_BATTLE_SLOT_SPACING, f32::max);
        let offset = original_formation_lateral_offset(ordinal, spacing);
        if offset == 0.0 {
            return anchor;
        }
        TacticalWorldPosition {
            x: anchor.x + mover.lateral.x * offset,
            y: anchor.y,
            z: anchor.z + mover.lateral.z * offset,
        }
    }

    /// The anchor reached its point: the route moves on for every member
    /// bound for it, and a maneuver's executor ends with its route
    /// (`FUN_005a8f70`); an attack order outlives its route. port: the
    /// members keep their own orders and routes, so a member bound
    /// elsewhere keeps its own.
    fn complete_force_waypoint(
        &mut self,
        members: &[usize],
        point: TacticalWorldPosition,
        completed_player_task_forces: &mut [bool; 9],
    ) {
        for &index in members {
            let player = self.player_is_attacker;
            let ship = &mut self.ships[index];
            if ship.source_waypoint != Some(point) {
                continue;
            }
            if ship.navigation_route.first() == Some(&point) {
                ship.navigation_route.remove(0);
            }
            ship.source_waypoint = ship.navigation_route.first().copied();
            if ship.source_waypoint.is_none()
                && matches!(
                    ship.order,
                    TacticalOrder::LeftHook
                        | TacticalOrder::RightHook
                        | TacticalOrder::Hammer
                        | TacticalOrder::Anvil
                )
            {
                ship.order = TacticalOrder::None;
                if ship.is_attacker == player {
                    completed_player_task_forces[task_force_slot(ship.task_force)] = true;
                }
            }
        }
    }

    /// The anchor's step (the mover's slot `+0xc`: Stand Off `0x005eb1a0`,
    /// Surround `0x005eb540`) toward `point`, then its turn. Returns whether
    /// the anchor is within `radius` of the point (the step's 0).
    pub(crate) fn step_force_anchor(
        mover: &mut TacticalForceMover,
        point: TacticalWorldPosition,
        radius: f32,
        delta_milliseconds: f32,
    ) -> bool {
        mover.slot_turn = 0.0;
        let speed = mover.speed * ORIGINAL_FORMATION_SPEED_SHARE;
        let delta = source_delta(point, mover.anchor);
        let length = delta.x.hypot(delta.y).hypot(delta.z);
        let arrived = if delta == TacticalWorldVector::ZERO || length <= radius {
            mover.velocity = TacticalWorldVector::ZERO;
            true
        } else {
            mover.last_delta = delta;
            let unit = delta.normalized();
            mover.velocity = TacticalWorldVector {
                x: unit.x * speed,
                y: unit.y * speed * ORIGINAL_FORMATION_VERTICAL_SHARE,
                z: unit.z * speed,
            };
            let seconds = delta_milliseconds * 0.001;
            mover.anchor.x += mover.velocity.x * seconds;
            mover.anchor.y += mover.velocity.y * seconds;
            mover.anchor.z += mover.velocity.z * seconds;
            false
        };
        if mover.tactic == TacticalTactic::Surround {
            // `FUN_005eae90`: Surround turns toward its own level movement,
            // else its forward, else (0, 0, 1); a heading behind flips x
            // and z. Its slots stay on the world axes (`0x005ead70`).
            let level = |vector: TacticalWorldVector| {
                TacticalWorldVector {
                    x: vector.x,
                    y: 0.0,
                    z: vector.z,
                }
                .normalized()
            };
            let heading = |mover: &TacticalForceMover| {
                [level(mover.last_delta), level(mover.forward)]
                    .into_iter()
                    .find(|vector| *vector != TacticalWorldVector::ZERO)
                    .unwrap_or(TacticalWorldVector::SOURCE_FORWARD)
            };
            mover.heading = heading(mover);
            if mover.forward.dot(mover.heading) < 0.0 {
                mover.forward.x = -mover.forward.x;
                mover.forward.z = -mover.forward.z;
                mover.heading = heading(mover);
            }
            turn_original_vector(
                &mut mover.forward,
                mover.heading,
                mover.turn_rate,
                delta_milliseconds,
            );
            return arrived;
        }
        // Stand Off: the caller's heading; none level keeps the forward.
        if mover.heading.x == 0.0 && mover.heading.z == 0.0 {
            mover.heading = mover.forward;
            return arrived;
        }
        if mover.forward.dot(mover.heading) < 0.0 {
            mover.forward = TacticalWorldVector {
                x: -mover.forward.x,
                y: -mover.forward.y,
                z: -mover.forward.z,
            };
        }
        let angle = source_direction_angle(mover.forward, mover.heading);
        if angle != 0.0 && angle >= ORIGINAL_STAND_OFF_TURN_DEAD_BAND {
            let applied = turn_original_vector(
                &mut mover.forward,
                mover.heading,
                mover.turn_rate,
                delta_milliseconds,
            );
            // `0x005eae40` → `FUN_005ead90`: the slots turn with it, and
            // the detours with them.
            mover.slot_turn = applied;
            let (sin, cos) = applied.sin_cos();
            let lateral = mover.lateral;
            mover.lateral = TacticalWorldVector {
                x: lateral.x * cos - sin * lateral.z,
                y: 0.0,
                z: lateral.x * sin + cos * lateral.z,
            };
            return arrived;
        }
        // `0x005eb1a0`'s tail: a still formation that did not turn takes
        // the velocity (0, 1, 0).
        if mover.velocity == TacticalWorldVector::ZERO {
            mover.velocity = TacticalWorldVector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            };
        }
        arrived
    }

    /// `FUN_005cdbf0` over the capital ships' tree (`+0x8f8`, sorted by x):
    /// walking out from the ship on each side, it stops at the first whose
    /// x is farther than both reaches, and keeps those whose z is within
    /// them and whose height is within both half-heights plus 0.4 of this
    /// frame's travel `travel`.
    pub fn nearby_capitals(&self, index: usize, travel: f32) -> Vec<usize> {
        let reach = |ship: &TacticalShip| {
            let velocity = ship.source_velocity;
            let speed = velocity.x.hypot(velocity.y).hypot(velocity.z);
            0.5 * speed
                + ORIGINAL_NEIGHBOUR_DIAMETER_SHARE
                    * ship
                        .source_collision_envelope
                        .map_or(0.0, |envelope| envelope.planar_diameter)
        };
        let Some(own_envelope) = self.ships[index].source_collision_envelope else {
            return Vec::new();
        };
        let mut sorted = (0..self.ships.len())
            .filter(|&other| {
                other == index || {
                    let ship = &self.ships[other];
                    ship.alive && !ship.jumping() && ship.source_collision_envelope.is_some()
                }
            })
            .collect::<Vec<_>>();
        sorted.sort_by(|left, right| {
            self.ships[*left]
                .source_position
                .x
                .total_cmp(&self.ships[*right].source_position.x)
                .then(left.cmp(right))
        });
        let at = sorted.iter().position(|&other| other == index).unwrap_or(0);
        let own = &self.ships[index];
        let own_reach = reach(own);
        let mut nearby = Vec::new();
        let mut visit = |other: usize| {
            let ship = &self.ships[other];
            let both = own_reach + reach(ship);
            if (ship.source_position.x - own.source_position.x).abs() > both {
                return false;
            }
            let half = ship.source_collision_envelope.map_or(0.0, |e| e.vertical_radius);
            if (ship.source_position.y - own.source_position.y).abs()
                <= half + own_envelope.vertical_radius + ORIGINAL_NEIGHBOUR_VERTICAL_SLACK * travel
                && (ship.source_position.z - own.source_position.z).abs() <= both
            {
                nearby.push(other);
            }
            true
        };
        for &other in sorted[..at].iter().rev() {
            if !visit(other) {
                break;
            }
        }
        for &other in &sorted[at + 1..] {
            if !visit(other) {
                break;
            }
        }
        nearby
    }

    /// A member's frame (its slot `+0x8`, `FUN_005b2f30`): it steers to its
    /// place at no more than the force's speed, moves up or down apart,
    /// snaps the last stretch, turns past pi / 48 (`0x5b1410`), and moves
    /// along its level forward by how well it faces its heading.
    fn move_force_member(
        &mut self,
        index: usize,
        mover: TacticalForceMover,
        place: TacticalWorldPosition,
        detour: Option<TacticalWorldVector>,
        own_speed: f32,
        delta_milliseconds: f32,
    ) {
        if own_speed <= 0.0 {
            self.ships[index].source_velocity = TacticalWorldVector::ZERO;
            return;
        }
        let speed = if mover.speed <= own_speed {
            mover.speed
        } else {
            own_speed
        };
        let seconds = delta_milliseconds * 0.001;
        let reach = speed * seconds;
        // A detour record replaces the distance with its raw offset
        // (`0x5eb040`).
        let to_place = |ship: &TacticalShip| {
            if let Some(offset) = detour {
                return offset;
            }
            let delta = source_delta(place, ship.source_position);
            if delta.dot(delta) <= ORIGINAL_MEMBER_PLACE_RADIUS_SQUARED {
                TacticalWorldVector::ZERO
            } else {
                delta
            }
        };
        if self.member_states.len() < self.ships.len() {
            self.member_states
                .resize(self.ships.len(), TacticalMemberState::default());
        }
        self.member_states[index].blocked = false;
        // With a distance to cover, the nearby capital ships
        // (`FUN_005cdbf0`) bound the vertical step (`DAT_0066c4c4`,
        // `DAT_0066c4fc`); the overlap tests (`FUN_005b2e60`) use only them.
        let mut delta = to_place(&self.ships[index]);
        let nearby = if delta == TacticalWorldVector::ZERO {
            Vec::new()
        } else {
            self.nearby_capitals(index, reach)
        };
        let overlaps = |session: &Self| {
            nearby
                .iter()
                .any(|&other| original_tactical_objects_overlap(&session.ships[index], &session.ships[other]))
        };
        let before_vertical = self.ships[index].source_position;
        if delta != TacticalWorldVector::ZERO {
            let climb = speed * ORIGINAL_FORMATION_VERTICAL_SHARE * seconds;
            let height = |other: usize| TacticalHeightNeighbour {
                y: self.ships[other].source_position.y,
                half_height: self.ships[other]
                    .source_collision_envelope
                    .map_or(0.0, |envelope| envelope.vertical_radius),
                id: other,
            };
            let neighbours = nearby.iter().map(|&other| height(other)).collect::<Vec<_>>();
            let (y, held) = original_height_band_step(height(index), &neighbours, delta.y, climb);
            self.ships[index].source_position.y = y;
            let mut blocked = held;
            if y != before_vertical.y {
                blocked = overlaps(self);
                if blocked {
                    self.ships[index].source_position = before_vertical;
                }
            }
            self.member_states[index].blocked = blocked;
        }
        let settled = self.ships[index].source_position;
        let ship = &self.ships[index];
        let forward = ship.source_forward;
        let heading = ship.source_desired_forward;
        let alignment = (forward.x * heading.x + forward.z * heading.z).max(0.0);
        delta = to_place(ship);
        let level_distance = delta.x.hypot(delta.z);
        if level_distance != 0.0
            && level_distance <= reach
            && alignment >= ORIGINAL_MEMBER_SNAP_ALIGNMENT
        {
            let ship = &mut self.ships[index];
            ship.source_position.x += delta.x;
            ship.source_position.z += delta.z;
            delta = to_place(ship);
        }
        // The heading (slot `+0x80`, `0x005b4d40`), made level.
        let ship = &self.ships[index];
        let mut wanted = if ship.retreating {
            let exit = self.withdrawal_exit_z(ship.identity.is_alliance);
            TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: exit - ship.source_position.z,
            }
        } else if delta != TacticalWorldVector::ZERO {
            delta
        } else if let Some(target) = ship
            .attack_target
            .filter(|_| {
                mover.velocity == TacticalWorldVector::ZERO
                    && mover.still_turns >= ORIGINAL_STILL_TURNS_TO_FACE
            })
            .and_then(|target| match target {
                // port: only a capital target; a fighter group's slot
                // `+0x20` is the pure-virtual stub.
                TacticalAttackTarget::CapitalShip(_) => self.attack_target_position(target),
                TacticalAttackTarget::FighterGroup(_) => None,
            })
        {
            // Its strongest arc against capital ships (slot `+0x84`, 4 as 0)
            // faces the target: the bearing turned by arc × pi / 2.
            let arc = crate::abstract_battle::strongest_arc(&ship.weapon_arcs, false).unwrap_or(0);
            let bearing = source_delta(target, ship.source_position);
            let turn = arc as f32 * std::f32::consts::FRAC_PI_2;
            let (sin, cos) = turn.sin_cos();
            TacticalWorldVector {
                x: bearing.x * cos - sin * bearing.z,
                y: bearing.y,
                z: bearing.x * sin + cos * bearing.z,
            }
        } else {
            forward
        };
        if wanted.x == 0.0 && wanted.z == 0.0 {
            wanted = forward;
        }
        wanted = TacticalWorldVector {
            x: wanted.x,
            y: 0.0,
            z: wanted.z,
        }
        .normalized();
        let ship = &mut self.ships[index];
        ship.source_desired_forward = wanted;
        if source_direction_angle(ship.source_forward, wanted) > ORIGINAL_MEMBER_TURN_DEAD_BAND {
            turn_original_tactical_ship(ship, delta_milliseconds);
        }
        if alignment != 0.0 && (delta.x != 0.0 || delta.z != 0.0) {
            let forward = ship.source_forward;
            ship.source_velocity = TacticalWorldVector {
                x: forward.x * speed * alignment,
                y: 0.0,
                z: forward.z * speed * alignment,
            };
            ship.source_position.x += ship.source_velocity.x * seconds;
            ship.source_position.z += ship.source_velocity.z * seconds;
        } else {
            ship.source_velocity = TacticalWorldVector::ZERO;
        }
        if self.ships[index].source_position != settled && overlaps(self) {
            self.ships[index].source_position = settled;
        }
    }


    /// Apply `FUN_005cd460`'s current-to-desired turn, `FUN_005b2f30`'s
    /// velocity contract, and `FUN_005cd640`'s millisecond position integrator.
    pub fn advance_original_tactical_movement(&mut self, delta_milliseconds: f32) {
        if !delta_milliseconds.is_finite() || delta_milliseconds <= 0.0 {
            return;
        }
        let engine_power = self
            .ships
            .iter()
            .enumerate()
            .map(|(index, _)| {
                original_tactical_mobility(&self.ships, &self.field_effects, index)
                    .map_or(0.0, |mobility| mobility.effective_engine_power)
            })
            .collect::<Vec<_>>();
        let mut completed_player_task_forces = [false; 9];
        self.sync_force_movers();
        let forces = self.moving_forces();
        let mut places = vec![None; self.ships.len()];
        // The forces' steps first, then the members' moves (`0x5a9ec0`).
        for members in &forces {
            let leader = members[0];
            let key = self.force_key(leader);
            let Some(mover_index) = self.force_movers.iter().position(|mover| mover.force == key)
            else {
                continue;
            };
            let withdrawing = self.ships[leader].retreating
                && self.withdrawal_executor_installed(TacticalUnitRef::Ship(leader));
            if withdrawing {
                // FUN_005cc300, order 3: the force heads for its side's exit
                // line, facing away from its side (`DAT_0066cff8`).
                let exit = self.withdrawal_exit_z(self.ships[leader].identity.is_alliance);
                let anchor = self.force_movers[mover_index].anchor;
                let away = original_side_vector(self.ships[leader].identity.is_alliance);
                let away = TacticalWorldVector {
                    x: -away.x,
                    y: -away.y,
                    z: -away.z,
                };
                let mover = &mut self.force_movers[mover_index];
                mover.forward = away;
                mover.heading = away;
                let ship = &mut self.ships[leader];
                ship.navigation_route.clear();
                ship.source_waypoint = Some(TacticalWorldPosition { z: exit, ..anchor });
            }
            let lead = &self.ships[leader];
            let Some(point) = lead.source_waypoint else {
                // The goal is cleared (`+0x78`..`+0x84`).
                self.force_movers[mover_index].velocity = TacticalWorldVector::ZERO;
                for (ordinal, &index) in members.iter().enumerate() {
                    let mover = &self.force_movers[mover_index];
                    places[index] = Some((mover_index, self.force_member_place(mover, members, ordinal)));
                }
                continue;
            };
            // `FUN_005cc300`'s radii: a route, a maneuver, an escort or the
            // withdrawal arrives within 1.0; closing on a target within 2.0.
            let closing = !withdrawing
                && lead.navigation_route.is_empty()
                && lead.attack_target.is_some()
                && !matches!(
                    lead.order,
                    TacticalOrder::LeftHook
                        | TacticalOrder::RightHook
                        | TacticalOrder::Hammer
                        | TacticalOrder::Anvil
                        | TacticalOrder::Escort
                );
            let radius = if closing {
                ORIGINAL_CLOSING_ARRIVAL_RADIUS
            } else {
                ORIGINAL_ROUTE_ARRIVAL_RADIUS
            };
            if !withdrawing {
                let mover = &mut self.force_movers[mover_index];
                mover.heading = source_delta(point, mover.anchor).normalized();
            }
            let arrived = Self::step_force_anchor(
                &mut self.force_movers[mover_index],
                point,
                radius,
                delta_milliseconds,
            );
            let turn = self.force_movers[mover_index].slot_turn;
            if turn != 0.0 {
                let force = self.force_movers[mover_index].force;
                let (sin, cos) = turn.sin_cos();
                for detour in self.force_detours.iter_mut().filter(|detour| detour.force == force) {
                    let offset = detour.offset;
                    detour.offset = TacticalWorldVector {
                        x: offset.x * cos - sin * offset.z,
                        y: offset.y,
                        z: offset.x * sin + cos * offset.z,
                    };
                }
            }
            if arrived && !withdrawing {
                self.complete_force_waypoint(members, point, &mut completed_player_task_forces);
            }
            for (ordinal, &index) in members.iter().enumerate() {
                let mover = &self.force_movers[mover_index];
                places[index] = Some((mover_index, self.force_member_place(mover, members, ordinal)));
            }
        }
        for (index, own_speed) in engine_power.into_iter().enumerate() {
            let Some((mover_index, place)) = places[index].filter(|_| {
                self.ships[index].alive && self.ships[index].retreat_progress <= 0.0
            }) else {
                self.ships[index].source_velocity = TacticalWorldVector::ZERO;
                continue;
            };
            let mover = self.force_movers[mover_index];
            let detour = self
                .force_detours
                .iter()
                .find(|detour| detour.force == mover.force && detour.ship == index)
                .map(|detour| detour.offset);
            self.move_force_member(index, mover, place, detour, own_speed, delta_milliseconds);
        }
        let faction = self.player_voice_faction();
        for (group, transitioned) in completed_player_task_forces.into_iter().enumerate() {
            if !transitioned {
                continue;
            }
            let group_still_maneuvering = self.ships.iter().any(|ship| {
                ship.alive
                    && ship.is_attacker == self.player_is_attacker
                    && task_force_slot(ship.task_force) == group
                    && matches!(
                        ship.order,
                        TacticalOrder::LeftHook
                            | TacticalOrder::RightHook
                            | TacticalOrder::Hammer
                            | TacticalOrder::Anvil
                    )
            });
            if !group_still_maneuvering {
                self.pending_voice_cues
                    .push(TacticalVoiceCue::capital_maneuver_complete(
                        faction,
                        task_force_of_slot(group),
                    ));
            }
        }
    }

    /// A recovering carrier's docking point: the vt `+4` of its `+0x418`
    /// sub-object (`FUN_005a6360`, `0x5b0b80`), its position lowered by its
    /// vertical radius (`+0x8`), the hull's underside.
    fn carrier_underside(&self, carrier: usize) -> TacticalWorldPosition {
        let ship = &self.ships[carrier];
        let below = ship
            .source_collision_envelope
            .map_or(0.0, |envelope| envelope.vertical_radius);
        TacticalWorldPosition {
            y: ship.source_position.y - below,
            ..ship.source_position
        }
    }

    /// The carrier a recovering squadron flies to (`+0x164`), while alive on
    /// its side.
    fn recovery_carrier(&self, index: usize) -> Option<usize> {
        let fighter = &self.fighters[index];
        fighter.recovery_target.filter(|&carrier| {
            self.ships
                .get(carrier)
                .is_some_and(|ship| ship.alive && ship.is_attacker == fighter.is_attacker)
        })
    }

    /// The members of a recovering force (order 2) fly their frames
    /// (`0x5b8f70`) each step from turn 3. Their distance (vt `+0x7c`, the
    /// force's `0x5c7060`) is the raw one to their carrier's underside, else
    /// the place delta (`0x5cb050`). The force's anchor stands: order 2's
    /// install (`FUN_005ca6d0`) clears its route and target.
    /// port: a squadron flies to the carrier that reserved it; the home
    /// carrier (`FUN_005b84a0`) is not kept.
    pub fn advance_recovering_fighters(&mut self, delta_milliseconds: f32) {
        self.sync_fighter_movers();
        for index in 0..self.fighters.len() {
            let fighter = &self.fighters[index];
            if fighter.retreating || fighter.order != TacticalOrder::Recover {
                continue;
            }
            // A mover holds only the squadrons in the fight.
            let Some(mover) = self.fighter_mover_of(index).cloned() else {
                continue;
            };
            let delta = match fighter.recovery_state {
                TacticalFighterRecoveryState::Returning => {
                    let Some(carrier) = self.recovery_carrier(index) else {
                        continue;
                    };
                    let delta = source_delta(self.carrier_underside(carrier), fighter.source_position);
                    // port: `FUN_005cf980`'s docking pass is taken to come
                    // before the move, so a member already within 2.0 stays.
                    if delta.x.hypot(delta.y).hypot(delta.z) < ORIGINAL_FIGHTER_DOCKING_DISTANCE {
                        continue;
                    }
                    delta
                }
                TacticalFighterRecoveryState::AwaitingCarrier
                | TacticalFighterRecoveryState::Reserved => self.fighter_place_delta(&mover, index),
                TacticalFighterRecoveryState::Docking
                | TacticalFighterRecoveryState::Recovered => continue,
            };
            self.fly_fighter_squadron(index, delta, None, mover.forward, delta_milliseconds);
        }
    }

    /// Advance the recovered `FUN_005cf980` fighter states. In state 2 a
    /// squadron within 2.0 (`DAT_0066d088`) of its carrier's underside enters
    /// state 3, and the following docking callback removes the tactical
    /// object in state 4 without changing its strategic squadron count.
    pub fn advance_original_fighter_recovery(&mut self, delta_milliseconds: f32) {
        if !delta_milliseconds.is_finite() || delta_milliseconds <= 0.0 {
            return;
        }
        let undersides = (0..self.fighters.len())
            .map(|index| self.recovery_carrier(index).map(|carrier| self.carrier_underside(carrier)))
            .collect::<Vec<_>>();
        let player_is_attacker = self.player_is_attacker;
        let player_faction = self.player_voice_faction();
        let mut transitioned_groups = [false; 4];
        for (fighter, underside) in self.fighters.iter_mut().zip(undersides) {
            match fighter.recovery_state {
                TacticalFighterRecoveryState::Returning => {
                    let Some(underside) = underside else {
                        fighter.recovery_state = TacticalFighterRecoveryState::AwaitingCarrier;
                        fighter.recovery_target = None;
                        continue;
                    };
                    let delta = source_delta(underside, fighter.source_position);
                    if delta.x.hypot(delta.y).hypot(delta.z) < ORIGINAL_FIGHTER_DOCKING_DISTANCE {
                        fighter.recovery_state = TacticalFighterRecoveryState::Docking;
                    }
                }
                TacticalFighterRecoveryState::Docking => {
                    fighter.recovery_state = TacticalFighterRecoveryState::Recovered;
                    fighter.order = TacticalOrder::None;
                    fighter.selected = false;
                    fighter.alive = false;
                    let group = usize::from(fighter.fighter_group);
                    if fighter.is_attacker == player_is_attacker
                        && group < transitioned_groups.len()
                        && !transitioned_groups[group]
                    {
                        transitioned_groups[group] = true;
                    }
                }
                TacticalFighterRecoveryState::AwaitingCarrier
                | TacticalFighterRecoveryState::Reserved
                | TacticalFighterRecoveryState::Recovered => {}
            }
        }
        let completed_groups = transitioned_groups
            .into_iter()
            .enumerate()
            .filter_map(|(group, transitioned)| {
                let group_still_active = self.fighters.iter().any(|fighter| {
                    fighter.alive
                        && fighter.is_attacker == player_is_attacker
                        && usize::from(fighter.fighter_group) == group
                });
                (transitioned && !group_still_active).then_some(group as u8)
            })
            .map(|group| TacticalVoiceCue::fighter_recovery_complete(player_faction, group));
        self.pending_voice_cues.extend(completed_groups);
    }

    /// Expand a fleet's composition into individual TacticalShip/TacticalFighter entries.
    pub fn expand_fleet(
        world: &GameWorld,
        fleet_key: FleetKey,
        is_attacker: bool,
        ships: &mut Vec<TacticalShip>,
        fighters: &mut Vec<TacticalFighter>,
    ) {
        let fleet = &world.fleets[fleet_key];
        let first_fleet_ship = ships.len();
        for (ship_idx, ship) in fleet
            .capital_ships
            .iter()
            .enumerate()
            .filter(|(_, ship)| ship.alive)
        {
            let class = &world.capital_ship_classes[ship.class];
            let tactical_resource = capital_ship_tactical_resource(class.dat_id);
            let death_star_resource = death_star_tactical_resource(class.dat_id);

            let turbolaser_total = (class.turbolaser_fore
                + class.turbolaser_aft
                + class.turbolaser_port
                + class.turbolaser_starboard)
                .cast_signed();
            let ion_cannon_total = (class.ion_cannon_fore
                + class.ion_cannon_aft
                + class.ion_cannon_port
                + class.ion_cannon_starboard)
                .cast_signed();
            let laser_cannon_total = (class.laser_cannon_fore
                + class.laser_cannon_aft
                + class.laser_cannon_port
                + class.laser_cannon_starboard)
                .cast_signed();
            let weapon_arcs = [
                TacticalWeaponArc::new(
                    class.laser_cannon_fore,
                    class.ion_cannon_fore,
                    class.turbolaser_fore,
                ),
                TacticalWeaponArc::new(
                    class.laser_cannon_starboard,
                    class.ion_cannon_starboard,
                    class.turbolaser_starboard,
                ),
                TacticalWeaponArc::new(
                    class.laser_cannon_aft,
                    class.ion_cannon_aft,
                    class.turbolaser_aft,
                ),
                TacticalWeaponArc::new(
                    class.laser_cannon_port,
                    class.ion_cannon_port,
                    class.turbolaser_port,
                ),
            ];
            let subsystem_capacity = TacticalSubsystemCapacity {
                shields: class.shield_strength > 0,
                weapons: turbolaser_total + ion_cannon_total + laser_cannon_total > 0,
                tractor: class.tractor_beam_power > 0,
                engines: class.sub_light_engine > 0,
                hyperdrive: u8::from(class.hyperdrive > 0)
                    + u8::from(class.hyperdrive_if_damaged > 0),
            };
            // FUN_0040b8a0 -> FUN_005b05c0: the unit starts from the ship's
            // hit counters (+0x64), the hyperdrive's as primary + backup
            // (ghidra/notes/ship-subsystem-damage.md).
            let subsystem_damage = TacticalSubsystemDamage {
                shields: ship.hits(crate::world::Subsystem::Shield),
                weapons: ship.hits(crate::world::Subsystem::Weapon),
                tractor: ship.hits(crate::world::Subsystem::Tractor),
                engines: ship.hits(crate::world::Subsystem::Speed),
                hyperdrive: ship.hits(crate::world::Subsystem::PrimaryHyperdrive)
                    + ship.hits(crate::world::Subsystem::BackupHyperdrive),
            };
            ships.push(TacticalShip {
                class_key: ship.class,
                identity: TacticalObjectIdentity {
                    class_dat_id: class.dat_id,
                    fleet_roster_index: ship_idx,
                    is_alliance: fleet.is_alliance,
                },
                tactical_resource,
                death_star_resource,
                source_position: TacticalWorldPosition::ORIGIN,
                source_forward: TacticalWorldVector::SOURCE_FORWARD,
                source_desired_forward: TacticalWorldVector::ZERO,
                source_velocity: TacticalWorldVector::ZERO,
                source_waypoint: None,
                navigation_route: Vec::new(),
                source_collision_envelope: None,
                name: class.name.clone(),
                x: 0.0,
                y: 0.0,
                hull_current: ship.hull_current,
                hull_max: class.hull.cast_signed(),
                shield: class.shield_strength.cast_signed(),
                shield_max: class.shield_strength.cast_signed(),
                subsystem_condition: TacticalSubsystemCondition::from_source_state(
                    ship.hull_current,
                    class.hull.cast_signed(),
                    subsystem_capacity,
                    subsystem_damage,
                ),
                subsystem_capacity,
                subsystem_damage,
                sublight_engine_power: class.sub_light_engine as f32
                    * ORIGINAL_SUBLIGHT_ENGINE_SCALE,
                maneuverability: class.maneuverability,
                tractor_beam_power: class.tractor_beam_power as f32,
                fighter_capacity: class.fighter_capacity,
                maneuver_state_value: None,
                damage_control: u8::try_from(class.damage_control.min(100)).unwrap_or(100),
                is_attacker,
                alive: true,
                selected: false,
                order: TacticalOrder::None,
                tactic: TacticalTactic::StandOff,
                // Source initialization of the first task force is known;
                // automatic distribution across later slots is not yet.
                task_force: 0,
                fleet: fleet_key,
                fleet_ship_index: ship_idx,
                // IDs 2001 through 2130 are not a linear class sprite table.
                // Keep the legacy selected-panel slot empty until its exact
                // bitmap role is independently recovered.
                sprite_id: None,
                weapon_arcs,
                weapon_ranges: TacticalWeaponRanges {
                    laser_cannon: class.laser_cannon_range as f32,
                    ion_cannon: class.ion_cannon_range as f32,
                    turbolaser: class.turbolaser_range as f32,
                },
                weapon_recharge_rate: class.weapon_recharge_rate as f32
                    * ORIGINAL_WEAPON_RECHARGE_SCALE,
                shield_recharge_rate: class.shield_recharge_rate as f32,
                shield_recharge_carry: 0.0,
                weapon_recharge_queue: Vec::new(),
                attack_target: None,
                manual_targets: Vec::new(),
                escort_target: None,
                contents: Vec::new(),
                retreating: false,
                retreat_progress: 0.0,
                retreated: false,
            });
        }

        // The strategic model currently retains personnel at fleet granularity.
        // Bind those entries to the fleet's first surviving capital hull, the
        // deterministic command-ship slot, so the original 1302 contents
        // monitor never invents a separate replacement panel.
        if let Some(command_ship) = ships.get_mut(first_fleet_ship) {
            command_ship
                .contents
                .extend(fleet.characters.iter().filter_map(|key| {
                    let character = world.characters.get(*key)?;
                    if character.is_killed {
                        return None;
                    }
                    let resource_id =
                        tactical_character_content_resource(character.dat_id, character.is_major)?;
                    Some(TacticalShipContent {
                        label: character.name.clone(),
                        resource_id,
                    })
                }));
        }

        for (fighter_idx, entry) in fleet.fighters.iter().enumerate() {
            let class = &world.fighter_classes[entry.class];
            for squadron_index in 0..entry.count {
                let squad_count = class.squadron_size;
                let hull = squad_count as f32 * ORIGINAL_FIGHTER_HULL_PER_CRAFT;
                fighters.push(TacticalFighter {
                    class_key: entry.class,
                    identity: TacticalObjectIdentity {
                        class_dat_id: class.dat_id,
                        fleet_roster_index: fighter_idx,
                        is_alliance: fleet.is_alliance,
                    },
                    tactical_resource: fighter_tactical_resource(class.dat_id),
                    source_position: TacticalWorldPosition::ORIGIN,
                    sublight_engine_power: class.sub_light_engine as f32
                        * ORIGINAL_FIGHTER_SUBLIGHT_ENGINE_SCALE,
                    fleet: fleet_key,
                    fleet_fighter_index: fighter_idx,
                    fleet_squadron_index: squadron_index,
                    name: class.name.clone(),
                    x: 0.0,
                    y: 0.0,
                    squad_count,
                    hull_current: hull,
                    hull_max: hull,
                    shield: class.shield_strength as f32,
                    shield_max: class.shield_strength as f32,
                    weapon_arc: TacticalWeaponArc::new(
                        class.laser_cannon_fore,
                        class.ion_cannon_fore,
                        class.turbolaser_fore,
                    ),
                    weapon_ranges: TacticalWeaponRanges {
                        laser_cannon: class.laser_cannon_range as f32,
                        ion_cannon: class.ion_cannon_range as f32,
                        turbolaser: class.turbolaser_range as f32,
                    },
                    torpedo_strength: class.torpedoes,
                    torpedo_range: class.torpedoes_range as f32,
                    maneuverability: class.maneuverability,
                    has_hyperdrive: class.hyperdrive > 0 || class.hyperdrive_if_damaged > 0,
                    is_attacker,
                    alive: squad_count > 0,
                    // FUN_005c63f0 initializes color state to 4 (white/unassigned).
                    fighter_group: 4,
                    selected: false,
                    order: TacticalOrder::None,
                    tactic: TacticalTactic::StandOff,
                    attack_target: None,
                    escort_target: None,
                    recovery_state: TacticalFighterRecoveryState::AwaitingCarrier,
                    recovery_target: None,
                    retreating: false,
                    retreat_progress: 0.0,
                });
            }
        }
    }

    /// Bind every active battle participant to the source X-slot and Z-lane
    /// initialization performed by `FUN_005ab650` through `FUN_005a9030`.
    pub fn assign_original_world_positions(
        ships: &mut [TacticalShip],
        fighters: &mut [TacticalFighter],
    ) -> OriginalTacticalLayout {
        let active_count = |is_alliance| {
            ships
                .iter()
                .filter(|ship| ship.alive && ship.identity.is_alliance == is_alliance)
                .count()
                + fighters
                    .iter()
                    .filter(|fighter| {
                        fighter.alive
                            && fighter.squad_count > 0
                            && fighter.identity.is_alliance == is_alliance
                    })
                    .count()
        };
        // Original tactical side zero is the Alliance collection at +0x08;
        // side one is the Imperial collection at +0x0c.
        let layout =
            OriginalTacticalLayout::from_active_counts(active_count(true), active_count(false));

        fn place_ships(ships: &mut [TacticalShip], is_alliance: bool, z: f32) {
            let mut slots = OriginalTacticalSlots::default();
            for ship in ships
                .iter_mut()
                .filter(|ship| ship.alive && ship.identity.is_alliance == is_alliance)
            {
                ship.source_position = TacticalWorldPosition {
                    x: slots.next().unwrap_or(0.0),
                    y: 0.0,
                    z,
                };
                // `0x005ab921`: each unit's forward `+0x10` becomes its
                // side's vector (`FUN_005b5ad0`, `FUN_005a7490`), set in
                // `FUN_005be160` at `+0x48`: the Alliance's (0, 0, 1), the
                // Empire's (0, 0, -1). Each side faces the other's lane.
                ship.source_forward = original_side_vector(is_alliance);
            }
        }

        fn place_fighters(fighters: &mut [TacticalFighter], is_alliance: bool, z: f32) {
            let mut slots = OriginalTacticalSlots::default();
            for fighter in fighters.iter_mut().filter(|fighter| {
                fighter.alive
                    && fighter.squad_count > 0
                    && fighter.identity.is_alliance == is_alliance
            }) {
                fighter.source_position = TacticalWorldPosition {
                    x: slots.next().unwrap_or(0.0),
                    y: 0.0,
                    z,
                };
            }
        }

        place_ships(ships, true, layout.outer_negative_z);
        place_ships(ships, false, layout.outer_positive_z);
        place_fighters(fighters, true, layout.inner_negative_z);
        place_fighters(fighters, false, layout.inner_positive_z);
        layout
    }

    /// Reproduce `FUN_005ae460`'s class-first red, blue, green, and gold
    /// fighter assignment independently for the Alliance and Empire. Empty
    /// colors split the first half of the largest group only when it contains
    /// more than seven squadrons.
    pub fn assign_original_fighter_groups(fighters: &mut [TacticalFighter]) {
        for is_alliance in [true, false] {
            let mut groups: [Vec<usize>; 4] = std::array::from_fn(|_| Vec::new());
            for (index, fighter) in fighters.iter().enumerate().filter(|(_, fighter)| {
                fighter.alive
                    && fighter.squad_count > 0
                    && fighter.identity.is_alliance == is_alliance
            }) {
                let Some(group) = fighter.tactical_resource.and_then(|resource| {
                    let base = if resource.tactical_ordinal < 33 {
                        29
                    } else {
                        33
                    };
                    resource
                        .tactical_ordinal
                        .checked_sub(base)
                        .filter(|group| *group < 4)
                }) else {
                    continue;
                };
                groups[usize::from(group)].push(index);
            }

            for empty in 0..groups.len() {
                if !groups[empty].is_empty() {
                    continue;
                }
                let donor = groups
                    .iter()
                    .enumerate()
                    .max_by_key(|(index, members)| (members.len(), std::cmp::Reverse(*index)))
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                if groups[donor].len() <= 7 {
                    continue;
                }
                let split = groups[donor].len() / 2;
                let moved = groups[donor].drain(..split).collect::<Vec<_>>();
                groups[empty] = moved;
            }

            for (group, members) in groups.iter().enumerate() {
                for &index in members {
                    fighters[index].fighter_group = u8::try_from(group).unwrap_or(3);
                }
            }
        }
    }

    /// Reproduce `FUN_005c81d0` through `FUN_005c83c0`'s three source lists,
    /// exact capability ordering, square-root leader count, stable follower
    /// ownership, and eight leader slots.
    pub fn assign_original_task_forces(ships: &mut [TacticalShip]) {
        fn category(ship: &TacticalShip) -> usize {
            let has_lasers = ship.weapon_arcs.iter().any(|arc| arc.laser_cannons > 0);
            let has_weapons = ship
                .weapon_arcs
                .iter()
                .any(|arc| arc.laser_cannons > 0 || arc.ion_cannons > 0 || arc.turbolasers > 0);
            if !has_weapons {
                2
            } else if has_lasers {
                1
            } else {
                0
            }
        }

        fn leader_count(members: usize, available_slots: usize) -> usize {
            if members == 0 || available_slots == 0 {
                return 0;
            }
            let root = if members < 2 {
                1
            } else {
                (members as f32).sqrt().floor() as usize
            };
            root.max(1).min(available_slots)
        }

        fn leader_y(slot: u8) -> f32 {
            match slot {
                0 => 0.0,
                1 => 8.0,
                2 => -8.0,
                3 => 16.0,
                4 => -16.0,
                5 => 24.0,
                6 => -24.0,
                _ => 32.0,
            }
        }

        for is_alliance in [true, false] {
            let mut categories: [Vec<usize>; 3] = std::array::from_fn(|_| Vec::new());
            for (index, ship) in ships
                .iter()
                .enumerate()
                .filter(|(_, ship)| ship.alive && ship.identity.is_alliance == is_alliance)
            {
                categories[category(ship)].push(index);
            }
            for members in &mut categories {
                members.sort_by(|left, right| {
                    original_formation_capability_score(&ships[*right])
                        .total_cmp(&original_formation_capability_score(&ships[*left]))
                });
            }

            let mut next_slot = 0_usize;
            for (category_index, members) in categories.iter().enumerate() {
                let category_end = match category_index {
                    0 => 3,
                    1 => (next_slot + 3).min(7),
                    _ => 7,
                };
                let available = category_end.saturating_sub(next_slot) + 1;
                let leaders = leader_count(members.len(), available);
                if leaders == 0 {
                    continue;
                }
                let stride = members.len() / leaders;
                let mut leader_ordinal = 0_usize;
                let mut active_slot = next_slot;
                for (member_ordinal, &ship_index) in members.iter().enumerate() {
                    if member_ordinal % stride == 0 && leader_ordinal < leaders {
                        active_slot = next_slot + leader_ordinal;
                        let slot = u8::try_from(active_slot).unwrap_or(7).min(7);
                        ships[ship_index].source_position.y = leader_y(slot);
                        leader_ordinal += 1;
                    }
                    ships[ship_index].task_force = u8::try_from(active_slot).unwrap_or(7).min(7);
                }
                next_slot = (next_slot + leaders).min(8);
                if next_slot >= 8 {
                    break;
                }
            }
        }
        Self::refresh_original_task_force_geometry(ships, &[[TacticalTactic::StandOff; 8]; 2]);
    }

    /// Apply `FUN_005ea250`, `FUN_005ec400`, and `FUN_005cbbc0`'s retained
    /// follower placement to the current task-force ownership. Before decoded
    /// mesh footprints arrive, the recovered five-unit source setup spacing is
    /// used. A later asset report expands the group to the largest 1.2-scaled
    /// close-mesh footprint without changing its source-selected leader. The
    /// lateral axis follows the leader's current retained-mode forward vector,
    /// so the formation remains attached while the leader turns and moves.
    pub fn refresh_original_task_force_geometry(
        ships: &mut [TacticalShip],
        formations: &[[TacticalTactic; 8]; 2],
    ) {
        for is_alliance in [true, false] {
            for task_force in 0_u8..8 {
                let mut members = ships
                    .iter()
                    .enumerate()
                    .filter(|(_, ship)| {
                        ship.alive
                            && ship.identity.is_alliance == is_alliance
                            && ship.task_force == task_force
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if members.is_empty() {
                    continue;
                }
                members.sort_by(|left, right| {
                    original_formation_capability_score(&ships[*right])
                        .total_cmp(&original_formation_capability_score(&ships[*left]))
                });
                let leader = members[0];
                let anchor = ships[leader].source_position;
                if formations[usize::from(ships[leader].is_attacker)][usize::from(task_force)]
                    == TacticalTactic::Surround
                {
                    // `0x005eb860`'s slots: each member stands at the
                    // anchor less its slot (`0x005eb040`), the lead unit at
                    // index 1 (`0x005ea250`). port: anchored on the lead
                    // unit, a member is its slot 1 less its own.
                    let lead_slot = original_surround_slot(1);
                    for (member_ordinal, index) in members.into_iter().enumerate().skip(1) {
                        let slot = original_surround_slot(member_ordinal + 1);
                        ships[index].source_position = TacticalWorldPosition {
                            x: anchor.x + lead_slot.x - slot.x,
                            y: anchor.y + lead_slot.y - slot.y,
                            z: anchor.z + lead_slot.z - slot.z,
                        };
                    }
                    continue;
                }
                let forward = ships[leader].source_forward.normalized();
                let lateral = if forward == TacticalWorldVector::ZERO {
                    TacticalWorldVector {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    }
                } else {
                    // `FUN_005ec400` derives a horizontal perpendicular from
                    // the leader direction. This is the equivalent vector in
                    // the reconstruction's retained left-handed coordinates.
                    TacticalWorldVector {
                        x: forward.z,
                        y: 0.0,
                        z: -forward.x,
                    }
                    .normalized()
                };
                let spacing = members
                    .iter()
                    .filter_map(|index| ships[*index].source_collision_envelope)
                    .map(|envelope| {
                        (envelope.planar_diameter * ORIGINAL_FORMATION_FOOTPRINT_SCALE).trunc()
                    })
                    .fold(ORIGINAL_BATTLE_SLOT_SPACING, f32::max);
                for (member_ordinal, index) in members.into_iter().enumerate() {
                    if member_ordinal == 0 {
                        // Retain the source slot's signed zero and every other
                        // exact anchor component without a `+ 0.0` rewrite.
                        ships[index].source_position = anchor;
                        continue;
                    }
                    let offset = original_formation_lateral_offset(member_ordinal, spacing);
                    ships[index].source_position = TacticalWorldPosition {
                        x: anchor.x + lateral.x * offset,
                        y: anchor.y,
                        z: anchor.z + lateral.z * offset,
                    };
                }
            }
        }
    }

    /// Auto-place ships in their respective deployment zones.
    ///
    /// Attacker ships go on the left side, defender ships on the right.
    /// Ships are stacked vertically, centered.
    #[expect(
        clippy::cast_precision_loss,
        reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
    )]
    /// port: 2D fallback placement; the original layout is `FUN_005ec400`.
    pub fn auto_place_ships(ships: &mut [TacticalShip]) {
        let atk_ships: Vec<usize> = ships
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_attacker)
            .map(|(i, _)| i)
            .collect();
        let def_ships: Vec<usize> = ships
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.is_attacker)
            .map(|(i, _)| i)
            .collect();

        // Attacker: left deployment zone.
        let atk_x = ARENA_WIDTH * DEPLOY_ZONE_FRACTION * 0.5;
        let atk_start_y = (ARENA_HEIGHT - (atk_ships.len() as f32 * SHIP_SPACING)) / 2.0;
        for (row, &idx) in atk_ships.iter().enumerate() {
            ships[idx].x = atk_x;
            ships[idx].y = atk_start_y + row as f32 * SHIP_SPACING;
        }

        // Defender: right deployment zone.
        let def_x = ARENA_WIDTH - ARENA_WIDTH * DEPLOY_ZONE_FRACTION * 0.5;
        let def_start_y = (ARENA_HEIGHT - (def_ships.len() as f32 * SHIP_SPACING)) / 2.0;
        for (row, &idx) in def_ships.iter().enumerate() {
            ships[idx].x = def_x;
            ships[idx].y = def_start_y + row as f32 * SHIP_SPACING;
        }
    }

    /// Auto-place fighter squadrons near their side's capital ships.
    #[expect(
        clippy::cast_precision_loss,
        reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
    )]
    /// port: 2D fallback fighter placement.
    pub fn auto_place_fighters(fighters: &mut [TacticalFighter], ships: &[TacticalShip]) {
        // Find average Y position for each side's capital ships.
        let atk_center = Self::side_center(ships, true);
        let def_center = Self::side_center(ships, false);

        let atk_fighters: Vec<usize> = fighters
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_attacker)
            .map(|(i, _)| i)
            .collect();
        let def_fighters: Vec<usize> = fighters
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.is_attacker)
            .map(|(i, _)| i)
            .collect();

        // Attacker fighters: slightly ahead (right) of capital ships.
        let atk_x = ARENA_WIDTH * DEPLOY_ZONE_FRACTION * 0.5 + 80.0;
        let atk_start_y = atk_center - (atk_fighters.len() as f32 * 24.0) / 2.0;
        for (row, &idx) in atk_fighters.iter().enumerate() {
            fighters[idx].x = atk_x;
            fighters[idx].y = atk_start_y + row as f32 * 24.0;
        }

        // Defender fighters: slightly ahead (left) of capital ships.
        let def_x = ARENA_WIDTH - ARENA_WIDTH * DEPLOY_ZONE_FRACTION * 0.5 - 80.0;
        let def_start_y = def_center - (def_fighters.len() as f32 * 24.0) / 2.0;
        for (row, &idx) in def_fighters.iter().enumerate() {
            fighters[idx].x = def_x;
            fighters[idx].y = def_start_y + row as f32 * 24.0;
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
    )]
    /// port: 2D fallback centre of a side.
    pub fn side_center(ships: &[TacticalShip], is_attacker: bool) -> f32 {
        let side: Vec<f32> = ships
            .iter()
            .filter(|s| s.is_attacker == is_attacker)
            .map(|s| s.y)
            .collect();
        if side.is_empty() {
            ARENA_HEIGHT / 2.0
        } else {
            side.iter().sum::<f32>() / side.len() as f32
        }
    }

    /// Returns true if the player controls this side of the battle.
    pub fn player_ships(&self) -> impl Iterator<Item = (usize, &TacticalShip)> {
        let player_is_atk = self.player_is_attacker;
        self.ships
            .iter()
            .enumerate()
            .filter(move |(_, s)| s.is_attacker == player_is_atk)
    }

    /// Returns true if this side is controlled by the AI.
    pub fn ai_ships(&self) -> impl Iterator<Item = (usize, &TacticalShip)> {
        let player_is_atk = self.player_is_attacker;
        self.ships
            .iter()
            .enumerate()
            .filter(move |(_, s)| s.is_attacker != player_is_atk)
    }

    // -------------------------------------------------------------------
    // Combat step — one tick of real-time combat
    // -------------------------------------------------------------------

    /// `FUN_005d3cc0` advances the field effect through eight frames.

    pub fn advance_presentational_effects(&mut self, elapsed: f32) {
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.weapon_effects.retain_mut(|effect| {
            effect.elapsed += elapsed;
            effect.elapsed < effect.duration
        });
        self.impact_effects.retain_mut(|effect| {
            effect.frame_elapsed += elapsed;
            while effect.frame_elapsed >= ORIGINAL_TACTICAL_EFFECT_FRAME_SECONDS {
                effect.frame_elapsed -= ORIGINAL_TACTICAL_EFFECT_FRAME_SECONDS;
                effect.frame = effect.frame.saturating_add(1);
            }
            effect.frame < effect.sequence.frame_count()
        });
        for effect in &mut self.field_effects {
            effect.frame_elapsed += elapsed;
            while effect.frame_elapsed >= ORIGINAL_TACTICAL_EFFECT_FRAME_SECONDS {
                effect.frame_elapsed -= ORIGINAL_TACTICAL_EFFECT_FRAME_SECONDS;
                effect.frame = (effect.frame + 1) % 8;
            }
        }
        if let Some(beam) = self.death_star_beam.as_mut().filter(|beam| beam.fired) {
            beam.elapsed += elapsed;
        }
    }

    /// `+0x878`: the event turns begun so far.
    #[must_use]
    pub fn event_turn(&self) -> u32 {
        self.combat_tick / ORIGINAL_TACTICAL_TURN_STEPS
    }

    /// `+0x87c` between turns, as an order from the interface sees it.
    #[must_use]
    pub fn command_event_stamp(&self) -> u32 {
        self.event_turn() + ORIGINAL_TACTICAL_EVENT_LEAD_TURNS
    }

    /// `+0x87c` within a step. A turn's updates run before `+0x87c`
    /// advances (`0x005bedb9`, `0x005bedf6`), so on a turn's first step it
    /// is one less.
    pub(crate) fn step_event_stamp(&self) -> u32 {
        let stamp = self.command_event_stamp();
        if self.combat_tick % ORIGINAL_TACTICAL_TURN_STEPS == 0 {
            stamp - 1
        } else {
            stamp
        }
    }

    pub fn post_turn_event(&mut self, stamp: u32, event: TacticalTurnEvent) {
        self.turn_events.push(TacticalQueuedEvent { stamp, event });
    }

    pub(crate) fn turn_event_pending(&self, event: &TacticalTurnEvent) -> bool {
        self.turn_events.iter().any(|queued| queued.event == *event)
    }

    /// `FUN_005a96b0` at a turn's start (`0x005bec3c`): once `+0x878` has
    /// reached `+0x8ac`, the events stamped `+0x878 - 1` run in order,
    /// older ones are dropped and newer ones kept.
    fn run_turn_events(&mut self) {
        if self.combat_tick % ORIGINAL_TACTICAL_TURN_STEPS != 0
            || self.event_turn() < ORIGINAL_TACTICAL_EVENT_LEAD_TURNS
        {
            return;
        }
        let due = self.event_turn() - 1;
        let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.turn_events)
            .into_iter()
            .filter(|queued| queued.stamp >= due)
            .partition(|queued| queued.stamp == due);
        self.turn_events = later;
        for queued in now {
            self.run_turn_event(queued.event);
        }
    }

    fn run_turn_event(&mut self, event: TacticalTurnEvent) {
        match event {
            TacticalTurnEvent::DeathStarBeam { target } => self.start_death_star_beam(target),
            TacticalTurnEvent::DeathStarKill { target } => self.resolve_death_star_shot(target),
            TacticalTurnEvent::JumpStart(unit) => self.start_jump(unit),
            TacticalTurnEvent::JumpEnd(unit) => self.end_jump(unit),
            TacticalTurnEvent::TrenchRunState(5) => self.commit_original_trench_run(),
            // `FUN_005cfec0` state 8, posted by the withdrawal
            // (`0x005ad5e7`).
            TacticalTurnEvent::TrenchRunState(8) => self.abort_original_trench_run(),
            // State 2 only sets the approach (`FUN_005cfec0`'s default).
            TacticalTurnEvent::TrenchRunState(_) => {}
            TacticalTurnEvent::WeaponFire(shot) => self.fire_weapon_shot(shot),
            TacticalTurnEvent::WeaponHit(shot) => self.hit_weapon_shot(shot),
            TacticalTurnEvent::SubsystemHit { ship, kind } => self.hit_subsystem(ship, kind),
            TacticalTurnEvent::OrderExecutor {
                order,
                capitals,
                fighters,
            } => self.run_order_executor(order, &capitals, &fighters),
            TacticalTurnEvent::TaskForceTactic {
                attacker,
                task_force,
                tactic,
            } => {
                if let Some(formation) = self.task_force_formations[usize::from(attacker)]
                    .get_mut(usize::from(task_force))
                {
                    *formation = tactic;
                }
            }
            TacticalTurnEvent::TaskForceTarget { units, target } => {
                for unit in units {
                    match unit {
                        TacticalUnitRef::Ship(index) => {
                            if let Some(ship) = self.ships.get_mut(index).filter(|ship| ship.alive) {
                                ship.attack_target = Some(target);
                            }
                        }
                        TacticalUnitRef::Fighter(index) => {
                            if let Some(fighter) =
                                self.fighters.get_mut(index).filter(|fighter| fighter.alive)
                            {
                                fighter.attack_target = Some(target);
                            }
                        }
                    }
                }
            }
            TacticalTurnEvent::WithdrawalExecutor(units) => {
                for unit in units {
                    let in_battle = match unit {
                        TacticalUnitRef::Ship(index) => {
                            self.ships.get(index).is_some_and(|ship| ship.alive)
                        }
                        TacticalUnitRef::Fighter(index) => self
                            .fighters
                            .get(index)
                            .is_some_and(|fighter| fighter.alive),
                    };
                    if in_battle && !self.withdrawal_executors.contains(&unit) {
                        self.withdrawal_executors.push(unit);
                    }
                }
            }
        }
    }

    fn withdrawal_executor_installed(&self, unit: TacticalUnitRef) -> bool {
        self.withdrawal_executors.contains(&unit)
    }

    /// `SetOrder` (`0x005ca530`): the order itself (`+0x68`) is already set;
    /// its executor follows on the command stamp (`+0x87c`, event `0xa`).
    pub fn post_order_executor(
        &mut self,
        order: TacticalOrder,
        capitals: Vec<usize>,
        fighters: Vec<usize>,
    ) {
        let stamp = self.command_event_stamp();
        self.post_order_executor_stamped(order, capitals, fighters, stamp);
    }

    fn post_order_executor_stamped(
        &mut self,
        order: TacticalOrder,
        capitals: Vec<usize>,
        fighters: Vec<usize>,
        stamp: u32,
    ) {
        self.post_turn_event(
            stamp,
            TacticalTurnEvent::OrderExecutor {
                order,
                capitals,
                fighters,
            },
        );
    }

    /// Run the queued order executors (event `0xa`, the withdrawal's too)
    /// now instead of on their turn. Test fixtures use it to present an
    /// order's result without running the battle for the three turns
    /// between.
    pub fn run_posted_order_executors(&mut self) {
        for queued in std::mem::take(&mut self.turn_events) {
            match queued.event {
                TacticalTurnEvent::OrderExecutor {
                    order,
                    capitals,
                    fighters,
                } => self.run_order_executor(order, &capitals, &fighters),
                event @ TacticalTurnEvent::WithdrawalExecutor(_) => self.run_turn_event(event),
                event => self.turn_events.push(TacticalQueuedEvent {
                    stamp: queued.stamp,
                    event,
                }),
            }
        }
    }

    /// `FUN_005ca6d0` for each unit given `order`. `port:` a unit gone from
    /// the battle by its turn has no task force for the event to reach.
    pub fn run_order_executor(
        &mut self,
        order: TacticalOrder,
        capitals: &[usize],
        fighters: &[usize],
    ) {
        let capitals = capitals
            .iter()
            .copied()
            .filter(|&index| self.ships.get(index).is_some_and(|ship| ship.alive))
            .collect::<Vec<_>>();
        let fighters = fighters
            .iter()
            .copied()
            .filter(|&index| {
                self.fighters
                    .get(index)
                    .is_some_and(|fighter| fighter.alive && fighter.squad_count > 0)
            })
            .collect::<Vec<_>>();
        begin_original_capital_order(self, &capitals, order);
        begin_original_attack_order(self, &capitals, &fighters, order);
        if order == TacticalOrder::Recover {
            self.begin_original_fighter_recovery(&fighters);
        }
    }

    /// `0x005ba520` starts the beam (`0x005c12d0`), aimed at the target
    /// where it is now.
    fn start_death_star_beam(&mut self, target: usize) {
        let position = self.ships.get(target).map(|ship| ship.source_position);
        let Some(beam) = self.death_star_beam.as_mut().filter(|beam| beam.target == target) else {
            return;
        };
        beam.fired = true;
        if let Some(position) = position {
            beam.target_position = position;
        }
        self.diagnostic_log.push(format!(
            "[tactical_death_star] superlaser_fired target_object_id={}",
            target + 1
        ));
    }

    /// `0x005b2b60` with the Death Star as its source.
    fn resolve_death_star_shot(&mut self, target: usize) {
        self.death_star_beam = None;
        // The Death Star must still be there, and the target still in the
        // fight (`FUN_005ad870`).
        let death_star_present = self.death_star.is_some_and(|death_star| !death_star.destroyed);
        if let Some(ship) = self
            .ships
            .get_mut(target)
            .filter(|ship| death_star_present && ship.alive && !ship.jumping())
        {
            ship.hull_current = 0;
            ship.alive = false;
            ship.selected = false;
            ship.attack_target = None;
            queue_original_tactical_impact(
                &mut self.impact_effects,
                target,
                WeaponKind::Turbolaser,
                OriginalTacticalImpactStage::Destroyed,
            );
        }
        if let Some(death_star) = self.death_star.as_mut() {
            death_star.action_committed = false;
        }
        self.diagnostic_log.push(format!(
            "[tactical_death_star] superlaser_resolved target_object_id={}",
            target + 1
        ));
    }

    /// `FUN_005b1e40`: a unit still in the fight (`FUN_005ad870`) with a
    /// working hyperdrive (`FUN_005b1b70`) enters state 7, and
    /// `FUN_005a6b40` starts its flight with the jump sound (`0x1d` /
    /// `0x1f`). One whose hyperdrive no longer works is destroyed
    /// (`FUN_005b2720`, slot `+0x1c`, `FUN_005b2d20`).
    fn start_jump(&mut self, unit: TacticalUnitRef) {
        match unit {
            TacticalUnitRef::Ship(index) => {
                let works = original_ship_hyperdrive_works(&self.ships, &self.field_effects, index);
                let Some(ship) = self
                    .ships
                    .get_mut(index)
                    .filter(|ship| ship.alive && !ship.jumping())
                else {
                    return;
                };
                if !works {
                    ship.hull_current = 0;
                    ship.shield = 0;
                    ship.alive = false;
                    ship.retreating = false;
                    return;
                }
                // The step's flight then starts from 0 (`FUN_005a6b40`).
                ship.retreat_progress = f32::MIN_POSITIVE;
                ship.source_waypoint = None;
                self.pending_audio_cues.push(TacticalAudioCue::from_variant(
                    TacticalAudioEvent::CapitalJump,
                    0,
                ));
            }
            TacticalUnitRef::Fighter(index) => {
                let Some(fighter) = self
                    .fighters
                    .get_mut(index)
                    .filter(|fighter| fighter.alive && !fighter.jumping())
                else {
                    return;
                };
                if !original_fighter_hyperdrive_works(fighter) {
                    fighter.hull_current = 0.0;
                    fighter.refresh_craft_count();
                    fighter.alive = false;
                    fighter.retreating = false;
                    return;
                }
                // The step's flight then starts from 0 (`FUN_005a6b40`).
                fighter.retreat_progress = f32::MIN_POSITIVE;
                self.pending_audio_cues.push(TacticalAudioCue::from_variant(
                    TacticalAudioEvent::FighterJump,
                    0,
                ));
            }
        }
    }

    /// Event `0x2e` (`FUN_005a90c0`): state 9, the unit has left.
    fn end_jump(&mut self, unit: TacticalUnitRef) {
        match unit {
            TacticalUnitRef::Ship(index) => {
                if let Some(ship) = self.ships.get_mut(index).filter(|ship| ship.alive) {
                    ship.alive = false;
                    ship.retreated = true;
                }
            }
            TacticalUnitRef::Fighter(index) => {
                if let Some(fighter) = self.fighters.get_mut(index).filter(|fighter| fighter.alive)
                {
                    fighter.alive = false;
                }
            }
        }
    }

    pub fn commit_death_star_shot(&mut self, target: usize) -> bool {
        let stamp = self.command_event_stamp();
        if !self.commit_death_star_shot_by(self.player_is_attacker, target, stamp) {
            return false;
        }
        // The interface's fire control (`Class_0066be80` slot `+0xc`)
        // queues the firing voice after `FUN_005ba720`; the commander's
        // shot has none.
        self.pending_voice_cues
            .push(TacticalVoiceCue::death_star_firing());
        true
    }

    /// `FUN_005ba720`: the Death Star's owner fires its full superlaser at
    /// an enemy capital ship; event `0x44` at `stamp`, the kill a turn
    /// later.
    fn commit_death_star_shot_by(&mut self, owner_is_attacker: bool, target: usize, stamp: u32) -> bool {
        if self.death_star_beam.is_some()
            || self
                .ships
                .get(target)
                .is_none_or(|ship| !ship.alive || ship.is_attacker == owner_is_attacker)
        {
            return false;
        }
        let Some(death_star) = self.death_star.as_mut().filter(|death_star| {
            death_star.is_attacker == owner_is_attacker
                && death_star.operational()
                && death_star.laser_charge >= ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
        }) else {
            return false;
        };
        let target_position = self.ships[target].source_position;
        death_star.action_committed = true;
        death_star.laser_charge = 0.0;
        self.death_star_beam = Some(TacticalDeathStarBeam {
            target,
            source_position: death_star.source_position,
            target_position,
            fired: false,
            elapsed: 0.0,
        });
        // Event 0x44 at `+0x87c`, the kill at `+0x87c + 1.0`.
        self.post_turn_event(stamp, TacticalTurnEvent::DeathStarBeam { target });
        self.post_turn_event(stamp + 1, TacticalTurnEvent::DeathStarKill { target });
        self.diagnostic_log.push(format!(
            "[tactical_death_star] superlaser_committed target_object_id={} charge=0",
            target + 1
        ));
        true
    }

    /// Advance combat by one tick. Applies weapon fire, shield regen, hull
    /// damage, and fighter engagement. Produces weapon effects for rendering.
    ///
    /// Returns `true` if the battle has ended this tick.
    pub fn step(&mut self) -> bool {
        if self.phase != BattlePhase::Combat || self.paused {
            return false;
        }

        self.combat_tick += 1;
        self.subsystem_repairs.clear();
        // A turn's first step runs its events, then its updates
        // (`0x005bec3c`, `0x005bed12`).
        let turn_step = self.combat_tick.is_multiple_of(ORIGINAL_TACTICAL_TURN_STEPS);
        self.run_turn_events();

        self.advance_withdrawal(turn_step);
        refresh_original_escort_targets(self);
        // The frame's slot `+0x98` (`0x005a9ec0`) moves the units only once
        // `+0x878` has reached `+0x8ac` (`0x005bef1d`): nothing moves for
        // the first three turns.
        if self.event_turn() >= ORIGINAL_TACTICAL_EVENT_LEAD_TURNS {
            advance_original_escort_movement(self, ORIGINAL_TACTICAL_STEP_MILLISECONDS);
            // The forces' order steps (slot `+0x14`) come before the units'
            // moves (slot `+0x8`) in the frame (`0x005a9ec0`).
            self.advance_engagement_approach();
            self.advance_fighter_approach();
            self.advance_recovering_fighters(ORIGINAL_TACTICAL_STEP_MILLISECONDS);
            self.advance_original_tactical_movement(ORIGINAL_TACTICAL_STEP_MILLISECONDS);
            self.advance_trench_run_approach(ORIGINAL_TACTICAL_STEP_MILLISECONDS);
            self.keep_units_in_the_field();
        }
        // port: the recovery's states (FUN_005cf980, a task-force order
        // step) and its flight are not yet split between turn and frame.
        self.advance_original_fighter_recovery(ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        refresh_original_attack_targets(self);
        if turn_step {
            self.run_turn_updates();
        }
        self.advance_original_trench_run(ORIGINAL_TACTICAL_STEP_MILLISECONDS);

        // Check for battle end.
        let atk_remaining = self.ships.iter().any(|s| s.is_attacker && s.alive)
            || self
                .fighters
                .iter()
                .any(|f| f.is_attacker && f.alive && f.squad_count > 0)
            || self
                .death_star
                .is_some_and(|death_star| death_star.is_attacker && !death_star.destroyed);
        let def_remaining = self.ships.iter().any(|s| !s.is_attacker && s.alive)
            || self
                .fighters
                .iter()
                .any(|f| !f.is_attacker && f.alive && f.squad_count > 0)
            || self
                .death_star
                .is_some_and(|death_star| !death_star.is_attacker && !death_star.destroyed);

        if !atk_remaining || !def_remaining {
            let winner = match (atk_remaining, def_remaining) {
                (true, false) => CombatWinner::Attacker,
                (false, true) => CombatWinner::Defender,
                _ => CombatWinner::Draw,
            };
            self.queue_battle_outcome_voice(winner);
            self.winner = Some(winner);
            self.phase = BattlePhase::Results;
            return true;
        }

        false
    }

    /// `0x005a9ec0` (`0x005aa089`..`0x005aa17a`): after its move, each unit
    /// in the fight (state 2) is held within `+-2/3` of the battle extent
    /// (`_DAT_0066c290`, `_DAT_0066c294`) on each axis.
    fn keep_units_in_the_field(&mut self) {
        let limit = self.source_layout.battle_extent * ORIGINAL_FIELD_LIMIT_EXTENT_SCALE;
        let keep = |position: &mut TacticalWorldPosition| {
            position.x = position.x.clamp(-limit, limit);
            position.y = position.y.clamp(-limit, limit);
            position.z = position.z.clamp(-limit, limit);
        };
        for ship in self.ships.iter_mut().filter(|ship| ship.alive && !ship.jumping()) {
            keep(&mut ship.source_position);
        }
        for fighter in self
            .fighters
            .iter_mut()
            .filter(|fighter| fighter.alive && !fighter.jumping())
        {
            keep(&mut fighter.source_position);
        }
    }

    /// A turn's updates, each given the turn's 1000 ms:
    /// - the commanders (slot `+0x80`, `FUN_005c8780`): the Death Star's
    ///   laser (`FUN_005ba7f0` at `0x005c88ec`) and the AI side's review;
    /// - the units (slot `+0x88`, `0x005a9de0`): the recharge
    ///   (`FUN_005b6320`), the subsystem repair (slot `+0x9c`,
    ///   `FUN_005b0330`) and the weapons (slot `+0x88`, `FUN_005b3a40`).
    ///
    /// The withdrawal's commander and task-force parts run in
    /// `advance_withdrawal`. port: each part runs for all units in turn,
    /// not unit by unit.
    fn run_turn_updates(&mut self) {
        let death_star_was_ready = self.death_star.is_some_and(|death_star| {
            death_star.is_attacker == self.player_is_attacker
                && death_star.laser_charge >= ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
        });
        if let Some(death_star) = self.death_star.as_mut() {
            death_star.advance_laser_charge(ORIGINAL_TACTICAL_TURN_SECONDS);
        }
        if !death_star_was_ready
            && self.death_star.is_some_and(|death_star| {
                death_star.is_attacker == self.player_is_attacker
                    && death_star.laser_charge >= ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
            })
        {
            self.pending_voice_cues
                .push(TacticalVoiceCue::death_star_ready());
        }
        self.ai_side_leaves();
        // `+0x84` (`0x005a9d40`): each force's slot `+0xc` recomputes its
        // band.
        self.update_engagement_bands();
        // `FUN_005ea740` (the movers' slot `+0x40`) follows the bands.
        self.update_force_movers();

        for ship in &mut self.ships {
            ship.advance_original_recharge(ORIGINAL_TACTICAL_TURN_SECONDS);
        }
        for fighter in &mut self.fighters {
            fighter.advance_original_recharge(ORIGINAL_TACTICAL_TURN_SECONDS);
        }

        // FUN_005b0330 invokes FUN_005b1490 every 50 turns. Both draws
        // advance the one recovered tactical manager stream in source call
        // order.
        if self
            .event_turn()
            .is_multiple_of(ORIGINAL_SUBSYSTEM_REPAIR_INTERVAL_TURNS)
        {
            for (ship_index, ship) in self.ships.iter_mut().enumerate() {
                let hits = ship.subsystem_damage.total_hits();
                if hits == 0 {
                    continue;
                }
                let chance_roll = u8::try_from(self.tactical_rng.inclusive(1, 100)).unwrap_or(100);
                let selection_roll =
                    u16::try_from(self.tactical_rng.inclusive(1, u32::from(hits))).unwrap_or(hits);
                if let Some(repair) =
                    attempt_original_subsystem_repair(ship, ship_index, chance_roll, selection_roll)
                {
                    self.subsystem_repairs.push(repair);
                }
            }
            refresh_original_subsystem_conditions(&mut self.ships, &self.field_effects);
        }

        // Collect the ship indices per side still in the fight: a withdrawing
        // ship fights and can be hit until its jump starts (`FUN_005cdf50`
        // and `FUN_005b3a40` take states 2 and 4, not 6, `FUN_005ad870`).
        let atk_alive: Vec<usize> = self
            .ships
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_attacker && s.alive && !s.jumping())
            .map(|(i, _)| i)
            .collect();
        let def_alive: Vec<usize> = self
            .ships
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.is_attacker && s.alive && !s.jumping())
            .map(|(i, _)| i)
            .collect();
        let mut shots = Vec::new();
        Self::fire_side(&mut self.ships, &atk_alive, &def_alive, &mut shots);
        Self::fire_side(&mut self.ships, &def_alive, &atk_alive, &mut shots);
        self.fire_capitals_at_fighters(&mut shots);
        // Fighter groups use the same slot (`0x66c560 + 0x88`,
        // `FUN_005b3a40`); their own readiness and recharge gate each shot.
        self.fighter_step(&mut shots);
        let stamp = self.step_event_stamp();
        for shot in shots {
            self.post_turn_event(stamp + 1, TacticalTurnEvent::WeaponFire(shot));
            self.post_turn_event(stamp + 2, TacticalTurnEvent::WeaponHit(shot));
        }
    }

    /// A subsystem hit event (`0x005b28d0` and its siblings): the ship's
    /// slot `+0xa0`. The handler only finds the ship; it does not ask
    /// whether it is still in the fight.
    pub fn hit_subsystem(&mut self, index: usize, kind: TacticalSubsystemKind) {
        let Some(ship) = self.ships.get_mut(index) else {
            return;
        };
        if ship.add_subsystem_hit(kind) {
            remove_tractor_source(&mut self.field_effects, index);
        }
        refresh_original_subsystem_conditions(&mut self.ships, &self.field_effects);
    }

    fn unit_in_the_fight(&self, unit: OriginalFighterTarget) -> bool {
        match unit {
            OriginalFighterTarget::Capital(index) => self
                .ships
                .get(index)
                .is_some_and(|ship| ship.alive && !ship.jumping()),
            OriginalFighterTarget::Fighter(index) => self.fighters.get(index).is_some_and(|fighter| {
                fighter.alive && fighter.squad_count > 0 && !fighter.jumping()
            }),
            OriginalFighterTarget::DeathStar => self
                .death_star
                .is_some_and(|death_star| !death_star.destroyed && death_star.hull > 0.0),
        }
    }

    /// A weapon fire event (`FUN_005b1ea0` and its siblings): with both
    /// units still in the fight (`FUN_005ad870`), the projectile and its
    /// fire sound. hyp: the sound's variant is drawn here.
    pub fn fire_weapon_shot(&mut self, shot: TacticalShot) {
        let source = match shot.source {
            TacticalUnitRef::Ship(index) => OriginalFighterTarget::Capital(index),
            TacticalUnitRef::Fighter(index) => OriginalFighterTarget::Fighter(index),
        };
        if !self.unit_in_the_fight(source) || !self.unit_in_the_fight(shot.target) {
            return;
        }
        let mut effects = Vec::new();
        match (shot.source, shot.target) {
            (TacticalUnitRef::Ship(from), OriginalFighterTarget::Capital(to)) => {
                queue_original_tactical_projectile(
                    &mut effects,
                    &self.ships,
                    from,
                    to,
                    shot.kind,
                    shot.strength,
                );
            }
            (TacticalUnitRef::Ship(from), OriginalFighterTarget::Fighter(to)) => {
                queue_original_capital_to_fighter_projectile(
                    &mut effects,
                    &self.ships,
                    &self.fighters,
                    from,
                    to,
                    shot.kind,
                    shot.strength,
                );
            }
            (TacticalUnitRef::Fighter(from), OriginalFighterTarget::Capital(to)) => {
                queue_original_fighter_projectile(
                    &mut effects,
                    &self.fighters,
                    from,
                    Some((to, &self.ships[to])),
                    None,
                    shot.kind,
                    shot.strength,
                );
            }
            (TacticalUnitRef::Fighter(from), OriginalFighterTarget::Fighter(to)) => {
                queue_original_fighter_projectile(
                    &mut effects,
                    &self.fighters,
                    from,
                    None,
                    Some((to, &self.fighters[to])),
                    shot.kind,
                    shot.strength,
                );
            }
            (TacticalUnitRef::Fighter(from), OriginalFighterTarget::DeathStar) => {
                if let Some(death_star) = self.death_star {
                    queue_original_fighter_to_death_star_projectile(
                        &mut effects,
                        &self.fighters,
                        from,
                        death_star,
                        shot.kind,
                        shot.strength,
                    );
                }
            }
            (TacticalUnitRef::Ship(_), OriginalFighterTarget::DeathStar) => {}
        }
        self.weapon_effects.extend(effects);
        let fire_event = TacticalAudioEvent::fire_for(shot.kind);
        let variant = u8::try_from(
            self.tactical_rng
                .inclusive(0, u32::from(fire_event.variant_count() - 1)),
        )
        .unwrap_or(0);
        self.pending_audio_cues
            .push(TacticalAudioCue::from_variant(fire_event, variant));
    }

    /// A weapon hit event (`0x005b2a20` and its siblings): with the target
    /// still in the fight (`FUN_005ad870`), its damage (slot `+0xe0`), the
    /// impact and the impact sound. hyp: the sound's variant is drawn here.
    pub fn hit_weapon_shot(&mut self, shot: TacticalShot) {
        if !self.unit_in_the_fight(shot.target) {
            return;
        }
        let player_is_attacker = self.player_is_attacker;
        let player_faction = self.player_voice_faction();
        let (source_is_attacker, source_group, source_maneuverability) = match shot.source {
            TacticalUnitRef::Ship(index) => {
                let ship = &self.ships[index];
                (
                    ship.is_attacker,
                    TacticalVoiceGroup::Capital(ship.task_force),
                    (ship.maneuverability as f32 + 1.0)
                        .clamp(1.0, ORIGINAL_CAPITAL_MANEUVERABILITY_MAX),
                )
            }
            TacticalUnitRef::Fighter(index) => {
                let fighter = &self.fighters[index];
                (
                    fighter.is_attacker,
                    TacticalVoiceGroup::Fighter(fighter.fighter_group),
                    fighter.effective_maneuverability(),
                )
            }
        };
        match shot.target {
            OriginalFighterTarget::Capital(index) => {
                let target_is_attacker = self.ships[index].is_attacker;
                let target_group = self.ships[index].task_force;
                let damage = original_tactical_integer_damage(shot.strength);
                let rolls =
                    original_tactical_damage_rolls(&self.ships, index, damage, &mut self.tactical_rng);
                let (stage, subsystem_hits) = apply_original_tactical_capital_hit(
                    &mut self.ships,
                    &mut self.field_effects,
                    index,
                    damage,
                    rolls,
                );
                let stamp = self.step_event_stamp();
                for kind in subsystem_hits {
                    self.post_turn_event(stamp, TacticalTurnEvent::SubsystemHit { ship: index, kind });
                }
                queue_original_destruction_voice(
                    &mut self.pending_voice_cues,
                    player_faction,
                    player_is_attacker,
                    source_is_attacker,
                    source_group,
                    target_is_attacker,
                    TacticalVoiceGroup::Capital(target_group),
                    stage,
                );
                queue_original_tactical_impact(&mut self.impact_effects, index, shot.kind, stage);
            }
            OriginalFighterTarget::Fighter(index) => {
                let target_is_attacker = self.fighters[index].is_attacker;
                let target_group = self.fighters[index].fighter_group;
                let stage = apply_original_fighter_damage(
                    &mut self.fighters,
                    index,
                    shot.strength,
                    source_maneuverability,
                );
                queue_original_destruction_voice(
                    &mut self.pending_voice_cues,
                    player_faction,
                    player_is_attacker,
                    source_is_attacker,
                    source_group,
                    target_is_attacker,
                    TacticalVoiceGroup::Fighter(target_group),
                    stage,
                );
            }
            OriginalFighterTarget::DeathStar => {
                if let Some(death_star) = self.death_star.as_mut() {
                    death_star.hull = (death_star.hull - shot.strength).max(0.0);
                    if death_star.hull == 0.0 {
                        death_star.destroyed = true;
                        death_star.action_committed = false;
                    }
                }
            }
        }
        let impact_event = TacticalAudioEvent::impact_for(shot.kind);
        let variant = if shot.kind == WeaponKind::FighterAttack {
            0
        } else {
            u8::try_from(
                self.tactical_rng
                    .inclusive(0, u32::from(impact_event.variant_count() - 1)),
            )
            .unwrap_or(0)
        };
        self.pending_audio_cues
            .push(TacticalAudioCue::from_variant(impact_event, variant));
    }

    /// One side's ships fire through the four source battery records recovered
    /// from `FUN_005b3a40`, `FUN_005b3f10`, and `FUN_005b6530`.
    pub fn fire_side(
        ships: &mut [TacticalShip],
        firing: &[usize],
        targets: &[usize],
        shots: &mut Vec<TacticalShot>,
    ) {
        if targets.is_empty() {
            return;
        }

        for &fire_idx in firing {
            if !ships[fire_idx].alive || ships[fire_idx].jumping() {
                continue;
            }

            // A typed capital target follows the direct-target callback. The
            // autonomous path evaluates every in-range target independently
            // for each family and keeps the first strongest target.
            let direct_target = match ships[fire_idx].attack_target {
                Some(TacticalAttackTarget::CapitalShip(target))
                    if target < ships.len()
                        && ships[target].alive
                        && !ships[target].jumping()
                        && targets.contains(&target) =>
                {
                    Some(target)
                }
                Some(TacticalAttackTarget::CapitalShip(_))
                    if ships[fire_idx].order == TacticalOrder::AttackCapitalShips =>
                {
                    // The attack executor never redirects an invalid typed
                    // target into an unrelated class or random capital. Its
                    // target-list lifecycle reacquires before this phase.
                    continue;
                }
                Some(TacticalAttackTarget::FighterGroup(_)) => {
                    // FUN_005d0bb0 accepts only object class 1, and
                    // FUN_005a8fc0 links that typed fighter target. Once the
                    // shared list lifecycle clears it, do not enter the
                    // autonomous capital-target path.
                    continue;
                }
                None if matches!(
                    ships[fire_idx].order,
                    TacticalOrder::AttackCapitalShips | TacticalOrder::AttackFighters
                ) =>
                {
                    // No eligible target of the requested class remains.
                    continue;
                }
                _ => None,
            };
            let mut candidates =
                original_capital_weapon_candidates(ships, fire_idx, targets, direct_target);

            loop {
                let mut strongest = 0.0;
                let mut selected_arc = None;
                for (arc_index, arc_candidates) in candidates.iter().enumerate() {
                    let total = arc_candidates.total();
                    if strongest < total {
                        strongest = total;
                        selected_arc = Some(arc_index);
                    }
                }
                let Some(arc_index) = selected_arc else {
                    break;
                };

                let arc_candidates = candidates[arc_index];
                let arc = ships[fire_idx].weapon_arcs[arc_index];
                let mut energy_spent = 0.0;
                // Preserve the executable's event order: ion, laser, then
                // turbolaser. Every populated family in the selected arc fires.
                for (kind, candidate) in [
                    (WeaponKind::IonCannon, arc_candidates.ion),
                    (WeaponKind::LaserCannon, arc_candidates.laser),
                    (WeaponKind::Turbolaser, arc_candidates.turbolaser),
                ] {
                    let Some(target_idx) = candidate.target else {
                        continue;
                    };
                    shots.push(TacticalShot {
                        source: TacticalUnitRef::Ship(fire_idx),
                        target: OriginalFighterTarget::Capital(target_idx),
                        kind,
                        strength: candidate.strength,
                    });
                    energy_spent += original_weapon_count(arc, kind) as f32;
                }

                if energy_spent > 0.0 {
                    let source = &mut ships[fire_idx];
                    let arc = &mut source.weapon_arcs[arc_index];
                    arc.energy = (arc.energy - energy_spent).max(0.0);
                    arc.ready = false;
                    let arc_index = u8::try_from(arc_index).unwrap_or(3);
                    if !source.weapon_recharge_queue.contains(&arc_index) {
                        source.weapon_recharge_queue.push(arc_index);
                    }
                }
                candidates[arc_index] = OriginalTacticalArcCandidates::default();
            }
        }
    }

    /// `FUN_005b3f10` capital batteries against fighters; index 3 bounds the four batteries of `FUN_005b0e50`.

    pub fn fire_capitals_at_fighters(&mut self, shots: &mut Vec<TacticalShot>) {
        for source in 0..self.ships.len() {
            let Some(TacticalAttackTarget::FighterGroup(target)) = self.ships[source].attack_target
            else {
                continue;
            };
            if self.ships[source].order != TacticalOrder::AttackFighters
                || !self.ships[source].alive
                || self.ships[source].jumping()
                || !self.fighters.get(target).is_some_and(|fighter| {
                    fighter.alive
                        && fighter.squad_count > 0
                        && !fighter.jumping()
                        && fighter.is_attacker != self.ships[source].is_attacker
                })
            {
                continue;
            }

            let target_position = self.fighters[target].source_position;
            let delta = source_delta(target_position, self.ships[source].source_position);
            let distance = delta.x.hypot(delta.y).hypot(delta.z);
            let arc_index =
                original_tactical_weapon_arc_to_position(&self.ships[source], target_position)
                    .index();
            let arc = self.ships[source].weapon_arcs[arc_index];
            if !arc.ready {
                continue;
            }
            let mut fired = false;
            for kind in [WeaponKind::LaserCannon, WeaponKind::Turbolaser] {
                let count = original_weapon_count(arc, kind);
                if count == 0 || distance > original_weapon_range(&self.ships[source], kind) {
                    continue;
                }
                let strength =
                    original_capital_weapon_strength(&self.ships[source], count, kind, false)
                        * if kind == WeaponKind::Turbolaser {
                            ORIGINAL_TURBOLASER_VS_FIGHTER
                        } else {
                            ORIGINAL_LASER_VS_FIGHTER / ORIGINAL_LASER_VS_CAPITAL
                        };
                shots.push(TacticalShot {
                    source: TacticalUnitRef::Ship(source),
                    target: OriginalFighterTarget::Fighter(target),
                    kind,
                    strength,
                });
                fired = true;
            }
            if fired {
                let ship = &mut self.ships[source];
                let arc = &mut ship.weapon_arcs[arc_index];
                arc.energy = 0.0;
                arc.ready = false;
                let arc_index = u8::try_from(arc_index).unwrap_or(3);
                if !ship.weapon_recharge_queue.contains(&arc_index) {
                    ship.weapon_recharge_queue.push(arc_index);
                }
            }
        }
    }

    pub fn fighter_target(&self, source: usize) -> Option<OriginalFighterTarget> {
        let fighter = self.fighters.get(source)?;
        if fighter.jumping() {
            return None;
        }
        let requested = match (fighter.order, fighter.attack_target) {
            (
                TacticalOrder::AttackCapitalShips,
                Some(TacticalAttackTarget::CapitalShip(target)),
            ) => Some(OriginalFighterTarget::Capital(target)),
            (TacticalOrder::AttackFighters, Some(TacticalAttackTarget::FighterGroup(target))) => {
                Some(OriginalFighterTarget::Fighter(target))
            }
            // State 5 owns the timed trench-run producer. These fighters do
            // not chip the Death Star through the ordinary battery path.
            (TacticalOrder::AttackDeathStar, _) => return None,
            (TacticalOrder::AttackCapitalShips | TacticalOrder::AttackFighters, _) => return None,
            _ => None,
        };
        if let Some(target) = requested {
            return self
                .fighter_target_in_range(source, target)
                .then_some(target);
        }

        // Autonomous executors retain the first target on equal strength.
        let mut selected = None;
        let mut selected_strength = 0.0;
        for (target, ship) in self.ships.iter().enumerate() {
            if !ship.alive || ship.jumping() || ship.is_attacker == fighter.is_attacker {
                continue;
            }
            let candidate = OriginalFighterTarget::Capital(target);
            let strength = self.fighter_target_strength(source, candidate);
            if selected_strength < strength {
                selected = Some(candidate);
                selected_strength = strength;
            }
        }
        for (target, opponent) in self.fighters.iter().enumerate() {
            if target == source
                || !opponent.alive
                || opponent.squad_count == 0
                || opponent.recovery_state == TacticalFighterRecoveryState::Recovered
                || opponent.is_attacker == fighter.is_attacker
            {
                continue;
            }
            let candidate = OriginalFighterTarget::Fighter(target);
            let strength = self.fighter_target_strength(source, candidate);
            if selected_strength < strength {
                selected = Some(candidate);
                selected_strength = strength;
            }
        }
        selected
    }

    pub fn fighter_target_in_range(&self, source: usize, target: OriginalFighterTarget) -> bool {
        self.fighter_target_strength(source, target) > 0.0
    }

    /// Fighter receivers clamp the maneuverability ratio at 0.1 (`FUN_005b5100`).

    pub fn fighter_target_strength(&self, source: usize, target: OriginalFighterTarget) -> f32 {
        let Some(fighter) = self.fighters.get(source).filter(|fighter| {
            fighter.alive
                && fighter.squad_count > 0
                && !fighter.jumping()
                && fighter.weapon_arc.ready
        }) else {
            return 0.0;
        };
        let (position, target_is_fighter, target_maneuverability, target_unshielded) = match target
        {
            OriginalFighterTarget::Capital(index) => {
                let Some(ship) = self.ships.get(index).filter(|ship| {
                    ship.alive && !ship.jumping() && ship.is_attacker != fighter.is_attacker
                }) else {
                    return 0.0;
                };
                (ship.source_position, false, 1.0, ship.shield <= 0)
            }
            OriginalFighterTarget::Fighter(index) => {
                let Some(opponent) = self.fighters.get(index).filter(|opponent| {
                    opponent.alive
                        && opponent.squad_count > 0
                        && !opponent.jumping()
                        && opponent.recovery_state != TacticalFighterRecoveryState::Recovered
                        && opponent.is_attacker != fighter.is_attacker
                }) else {
                    return 0.0;
                };
                (
                    opponent.source_position,
                    true,
                    opponent.effective_maneuverability(),
                    false,
                )
            }
            OriginalFighterTarget::DeathStar => {
                let Some(death_star) = self.death_star.filter(|death_star| {
                    death_star.operational() && death_star.is_attacker != fighter.is_attacker
                }) else {
                    return 0.0;
                };
                (death_star.source_position, false, 1.0, true)
            }
        };
        let delta = source_delta(position, fighter.source_position);
        let distance = delta.x.hypot(delta.y).hypot(delta.z);
        let maneuver_modifier = if target_is_fighter {
            (fighter.effective_maneuverability() / target_maneuverability).max(0.1)
        } else {
            1.0
        };
        let mut strength = 0.0;
        for kind in [
            WeaponKind::IonCannon,
            WeaponKind::LaserCannon,
            WeaponKind::Turbolaser,
        ] {
            let count = original_weapon_count(fighter.weapon_arc, kind);
            if count > 0 && distance <= original_fighter_weapon_range(fighter, kind) {
                strength +=
                    original_fighter_weapon_strength(fighter, count, kind, target_is_fighter)
                        * maneuver_modifier;
            }
        }
        if !target_is_fighter
            && target_unshielded
            && fighter.torpedo_strength > 0
            && distance <= fighter.torpedo_range
        {
            strength += fighter.torpedo_strength as f32 * ORIGINAL_FIGHTER_TORPEDO_STRENGTH;
        }
        strength
    }

    /// Fire each live fighter group through the source fore battery record.
    pub fn fighter_step(&mut self, shots: &mut Vec<TacticalShot>) {
        for source in 0..self.fighters.len() {
            let Some(target) = self.fighter_target(source) else {
                continue;
            };
            let source_snapshot = self.fighters[source].clone();
            let target_position = match target {
                OriginalFighterTarget::Capital(index) => self.ships[index].source_position,
                OriginalFighterTarget::Fighter(index) => self.fighters[index].source_position,
                OriginalFighterTarget::DeathStar => {
                    let Some(death_star) = self.death_star else {
                        continue;
                    };
                    death_star.source_position
                }
            };
            let delta = source_delta(target_position, source_snapshot.source_position);
            let distance = delta.x.hypot(delta.y).hypot(delta.z);
            let target_is_fighter = matches!(target, OriginalFighterTarget::Fighter(_));
            let target_was_unshielded = match target {
                OriginalFighterTarget::Capital(index) => self.ships[index].shield <= 0,
                OriginalFighterTarget::Fighter(_) => false,
                OriginalFighterTarget::DeathStar => true,
            };
            let mut fired = false;

            // Preserve the executable family-event order used by FUN_005b3f10.
            for kind in [
                WeaponKind::IonCannon,
                WeaponKind::LaserCannon,
                WeaponKind::Turbolaser,
            ] {
                let count = original_weapon_count(source_snapshot.weapon_arc, kind);
                if count == 0 || distance > original_fighter_weapon_range(&source_snapshot, kind) {
                    continue;
                }
                let strength = original_fighter_weapon_strength(
                    &source_snapshot,
                    count,
                    kind,
                    target_is_fighter,
                );
                if strength <= 0.0 {
                    continue;
                }
                shots.push(TacticalShot {
                    source: TacticalUnitRef::Fighter(source),
                    target,
                    kind,
                    strength,
                });
                fired = true;
            }

            if matches!(
                target,
                OriginalFighterTarget::Capital(_) | OriginalFighterTarget::DeathStar
            ) && target_was_unshielded
                && source_snapshot.torpedo_strength > 0
                && distance <= source_snapshot.torpedo_range
            {
                let strength =
                    source_snapshot.torpedo_strength as f32 * ORIGINAL_FIGHTER_TORPEDO_STRENGTH;
                shots.push(TacticalShot {
                    source: TacticalUnitRef::Fighter(source),
                    target,
                    kind: WeaponKind::FighterAttack,
                    strength,
                });
                fired = true;
            }
            if fired {
                self.fighters[source].consume_weapon_arc();
            }
        }
    }

    pub fn has_active_trench_run(&self) -> bool {
        let Some(death_star) = self
            .death_star
            .filter(|death_star| death_star.hull > 0.0 && !death_star.destroyed)
        else {
            return false;
        };
        self.fighters.iter().any(|fighter| {
            fighter.alive
                && fighter.squad_count > 0
                && fighter.is_attacker != death_star.is_attacker
                && fighter.order == TacticalOrder::AttackDeathStar
        })
    }

    /// Advance and resolve `TACTICALRESULT_UPDATE` state 5. `FUN_005cfec0`
    /// supplies the 120 second timer; `FUN_005d04e0` consumes the shared RNG
    /// for maneuver losses and the surviving torpedo attempt.
    pub fn advance_original_trench_run(&mut self, milliseconds: f32) {
        let Some(remaining) = self.trench_run_remaining_ms else {
            return;
        };
        if self.trench_run_outcome.is_some() {
            self.trench_run_remaining_ms = None;
            return;
        }
        if self.trench_run_pending_outcome.is_none() && !self.has_active_trench_run() {
            self.record_trench_run_outcome(TacticalTrenchRunOutcome::Failure);
            return;
        }
        let remaining = (remaining - milliseconds.max(0.0)).max(0.0);
        self.trench_run_remaining_ms = Some(remaining);
        self.queue_original_trench_run_chatter(remaining);
        if remaining == 0.0 {
            self.resolve_original_trench_run();
        }
    }

    /// `FUN_005d0ae0` (`DAT_006bc978`): a run is under way from its
    /// creation (`0x005d0863`) until it ends or is aborted.
    #[must_use]
    pub fn trench_run_under_way(&self) -> bool {
        self.trench_run_remaining_ms.is_some()
            || (self.trench_run_group.is_some() && self.trench_run_outcome.is_none())
    }

    /// The approach (result state 2, which `FUN_005c6610` waits on at
    /// `0x005c6887`): after its event 0x3c with state 2 has run and before
    /// the run (state 5) begins.
    fn trench_run_approaching(&self) -> bool {
        self.trench_run_group.is_some()
            && self.trench_run_remaining_ms.is_none()
            && self.trench_run_outcome.is_none()
            && !self.turn_event_pending(&TacticalTurnEvent::TrenchRunState(2))
            && !self.turn_event_pending(&TacticalTurnEvent::TrenchRunState(5))
    }

    /// The Attack Death Star order (6) for a fighter group: the result
    /// object (`0x005d0820`) is created, which marks the run under way and
    /// posts its state 2. Its first update (`0x005d0430` -> `0x005d0340`)
    /// plays the group's start line for the Alliance's player, or 0x115
    /// for the Empire's. Returns false when neither player hears it (port:
    /// the run is then not started).
    pub fn begin_original_trench_run(&mut self, fighter_group: u8, fighter_is_attacker: bool) -> bool {
        let Some(death_star) = self.death_star else {
            return false;
        };
        if self.player_voice_faction() == TacticalVoiceFaction::Alliance
            && fighter_is_attacker == self.player_is_attacker
        {
            self.pending_voice_cues
                .push(TacticalVoiceCue::trench_run_started(fighter_group));
        } else if self.player_voice_faction() == TacticalVoiceFaction::Empire
            && death_star.is_attacker == self.player_is_attacker
        {
            self.pending_voice_cues
                .push(TacticalVoiceCue::death_star_under_attack());
        } else {
            return false;
        }
        self.trench_run_group = Some(fighter_group);
        let stamp = self.command_event_stamp();
        self.post_turn_event(stamp, TacticalTurnEvent::TrenchRunState(2));
        true
    }

    /// The approach, in `FUN_005c6610`'s order 6 step (the task force's
    /// slot `+0x14`, run each frame by `0x005a9ff8`): the force flies toward
    /// the side vector turned a quarter about y, `(-side.z, 0, side.x)`,
    /// times half the extent (`0x005c6848`..`0x005c68f5`), with the move's
    /// radius 0.2 (`0x005c68f0`). On arrival it posts state 5
    /// (`0x005c6924`).
    ///
    /// port: the force's position is its first member's, and it moves at
    /// the slowest member's engine power. The step's time is flown in
    /// frames of `PORT_MOVER_FRAME_MILLISECONDS`: in one 250 ms step a
    /// fighter group would pass the point by more than the radius and swing
    /// back and forth without arriving.
    fn advance_trench_run_approach(&mut self, milliseconds: f32) {
        let mut left = milliseconds;
        while left > 0.0 {
            let frame = left.min(PORT_MOVER_FRAME_MILLISECONDS);
            left -= frame;
            if !self.advance_trench_run_approach_frame(frame) {
                return;
            }
        }
    }

    /// One frame of `FUN_005c6610`'s approach. Returns false once it has
    /// arrived or there is no approach.
    fn advance_trench_run_approach_frame(&mut self, milliseconds: f32) -> bool {
        if !self.trench_run_approaching() {
            return false;
        }
        let Some(group) = self.trench_run_group else {
            return false;
        };
        let members = self
            .fighters
            .iter()
            .enumerate()
            .filter(|(_, fighter)| {
                fighter.alive
                    && fighter.squad_count > 0
                    && fighter.fighter_group == group
                    && fighter.order == TacticalOrder::AttackDeathStar
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let Some(&first) = members.first() else {
            return false;
        };
        let side_z = if self.fighters[first].identity.is_alliance { 1.0 } else { -1.0 };
        let half = self.source_layout.battle_extent * ORIGINAL_WITHDRAWAL_EXIT_EXTENT_SCALE;
        let point = TacticalWorldPosition {
            x: -side_z * half,
            y: 0.0,
            z: 0.0,
        };
        let delta = source_delta(point, self.fighters[first].source_position);
        let distance = delta.x.hypot(delta.y).hypot(delta.z);
        if distance <= ORIGINAL_TRENCH_RUN_APPROACH_RADIUS {
            let stamp = self.step_event_stamp();
            self.post_turn_event(stamp, TacticalTurnEvent::TrenchRunState(5));
            return false;
        }
        let speed = members
            .iter()
            .map(|&index| self.fighters[index].sublight_engine_power.max(0.0))
            .fold(f32::INFINITY, f32::min);
        let step = speed * milliseconds * 0.001 / distance;
        for index in members {
            let fighter = &mut self.fighters[index];
            fighter.source_position.x += delta.x * step;
            fighter.source_position.y += delta.y * step;
            fighter.source_position.z += delta.z * step;
        }
        true
    }

    /// `FUN_005cfec0` state 5: the run's 120 000 ms timer
    /// (`DAT_0066d080`), the group's follow-up line for the Alliance's
    /// player or 0x11d for the Empire's, and the outcome's draws
    /// (`FUN_005d04e0`).
    pub fn commit_original_trench_run(&mut self) {
        let Some(group) = self.trench_run_group.filter(|_| self.trench_run_outcome.is_none()) else {
            return;
        };
        self.trench_run_remaining_ms = Some(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);
        self.diagnostic_log.push(format!(
            "[tactical_death_star] trench_run_state=5 group={group} remaining_ms={ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS} source=FUN_005cfec0"
        ));
        if self.player_voice_faction() == TacticalVoiceFaction::Alliance {
            self.pending_voice_cues
                .push(TacticalVoiceCue::trench_run_committed(group));
        } else {
            self.pending_voice_cues
                .push(TacticalVoiceCue::death_star_attack_continuing());
        }
        self.prepare_original_trench_run();
    }

    /// Run `FUN_005d04e0` when state 5 begins. The source consumes maneuver
    /// and success draws immediately, then retains the selected result while
    /// `FUN_005d03f0` advances the presentation chatter.
    pub fn prepare_original_trench_run(&mut self) {
        let Some(death_star) = self
            .death_star
            .filter(|death_star| death_star.hull > 0.0 && !death_star.destroyed)
        else {
            return;
        };
        let participants = self
            .fighters
            .iter()
            .enumerate()
            .filter(|(_, fighter)| {
                fighter.alive
                    && fighter.squad_count > 0
                    && fighter.is_attacker != death_star.is_attacker
                    && fighter.order == TacticalOrder::AttackDeathStar
            })
            .map(|(index, _)| index)
            .take(1001)
            .collect::<Vec<_>>();

        // `FUN_005d08d0`: the participants' hull and shields are kept for
        // an abort.
        self.trench_run_saved = participants
            .iter()
            .map(|&index| {
                let fighter = &self.fighters[index];
                (index, fighter.hull_current, fighter.shield)
            })
            .collect();
        let mut success = false;
        let mut maneuver_damage = false;
        for &index in &participants {
            let maneuverability = self.fighters[index]
                .effective_maneuverability()
                .round()
                .clamp(0.0, 9.0) as u32;
            if maneuverability < self.tactical_rng.inclusive(1, 10) {
                maneuver_damage = true;
                let damage = if maneuverability <= 1 {
                    100_000.0
                } else {
                    self.tactical_rng.inclusive(1, maneuverability) as f32
                        * ORIGINAL_TRENCH_RUN_DAMAGE_SCALE
                };
                self.fighters[index].hull_current -= damage;
                self.fighters[index].refresh_craft_count();
            }
            if !self.fighters[index].alive {
                continue;
            }
            let success_rating = u32::from(self.trench_run_success_rating.clamp(1, 9));
            if self.tactical_rng.inclusive(1, 100) <= success_rating {
                success = true;
                break;
            }
        }

        self.trench_run_participants = participants;
        self.trench_run_pending_outcome = Some(if success {
            TacticalTrenchRunOutcome::Success
        } else {
            TacticalTrenchRunOutcome::Failure
        });
        self.trench_run_chatter = match (success, maneuver_damage) {
            (true, false) => ORIGINAL_TRENCH_RUN_SUCCESS_CHATTER,
            (true, true) => ORIGINAL_TRENCH_RUN_DAMAGED_SUCCESS_CHATTER,
            (false, _) => ORIGINAL_TRENCH_RUN_FAILURE_CHATTER,
        };
        self.trench_run_chatter_cursor = 0;
        self.queue_original_trench_run_chatter(ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS);
    }

    /// Emit every `FUN_005d03f0` threshold crossed by this update. Production
    /// advances at 10 Hz, while deterministic fixtures may cross several
    /// thresholds in one call; preserving order makes both paths equivalent.
    pub fn queue_original_trench_run_chatter(&mut self, remaining_ms: f32) {
        while self.trench_run_chatter_cursor < self.trench_run_chatter.len() {
            let slot = self.trench_run_chatter_cursor;
            let threshold = ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS
                - slot as f32 * ORIGINAL_TRENCH_RUN_DURATION_MILLISECONDS * 0.1;
            if remaining_ms > threshold {
                break;
            }
            self.trench_run_chatter_cursor += 1;
            let source_event = self.trench_run_chatter[slot];
            if source_event != ORIGINAL_TRENCH_RUN_SILENT_EVENT {
                self.pending_voice_cues
                    .push(TacticalVoiceCue::trench_run_chatter(source_event));
            }
        }
    }

    /// Apply state 6 or 7 after the retained two-minute presentation window.
    ///
    /// `FUN_005d03f0` casualties: none for 0–1, one for 2–3, half thereafter.
    pub fn resolve_original_trench_run(&mut self) {
        let Some(outcome) = self.trench_run_pending_outcome else {
            return;
        };
        let participants = self.trench_run_participants.clone();

        if outcome == TacticalTrenchRunOutcome::Success {
            if let Some(death_star) = self.death_star.as_mut() {
                death_star.hull = 0.0;
                death_star.destroyed = true;
                death_star.action_committed = false;
            }
            // State 6 removes the first bounded half of the participating list
            // before returning the survivors. Counts 0 and 1 lose none, 2 or
            // 3 lose one, and larger groups lose floor(n / 2).
            let casualties = match participants.len() {
                0 | 1 => 0,
                2 | 3 => 1,
                count => count / 2,
            };
            for &index in participants.iter().take(casualties) {
                self.fighters[index].hull_current = 0.0;
                self.fighters[index].refresh_craft_count();
            }
            self.record_trench_run_outcome(outcome);
        } else {
            for &index in &participants {
                self.fighters[index].hull_current = 0.0;
                self.fighters[index].refresh_craft_count();
            }
            self.record_trench_run_outcome(outcome);
        }
    }

    /// `FUN_005cfec0` state 8: a run under way (state 5) ends without a
    /// result; each participant gets back the hull and shields it had when
    /// the run began and returns to the fight (state 2). The run's
    /// executor goes with the group's new order (`FUN_005d0950` clears
    /// `DAT_006bc978`).
    pub fn abort_original_trench_run(&mut self) {
        if !self.trench_run_under_way() || self.trench_run_outcome.is_some() {
            return;
        }
        self.turn_events
            .retain(|queued| !matches!(queued.event, TacticalTurnEvent::TrenchRunState(_)));
        for (index, hull, shield) in std::mem::take(&mut self.trench_run_saved) {
            if let Some(fighter) = self.fighters.get_mut(index) {
                fighter.hull_current = hull;
                fighter.refresh_craft_count();
                fighter.alive = fighter.squad_count > 0;
                fighter.shield = shield;
            }
        }
        self.trench_run_remaining_ms = None;
        self.trench_run_pending_outcome = None;
        self.trench_run_participants.clear();
        self.trench_run_group = None;
        self.trench_run_chatter_cursor = self.trench_run_chatter.len();
    }

    pub fn record_trench_run_outcome(&mut self, outcome: TacticalTrenchRunOutcome) {
        if self.trench_run_outcome.is_some() {
            return;
        }
        if self.player_is_attacker == self.attacker_is_alliance {
            let group = self.trench_run_group.or_else(|| {
                self.fighters
                    .iter()
                    .find(|fighter| {
                        fighter.identity.is_alliance
                            && fighter.order == TacticalOrder::AttackDeathStar
                    })
                    .map(|fighter| fighter.fighter_group)
            });
            self.pending_voice_cues
                .push(TacticalVoiceCue::trench_run_outcome(
                    group.unwrap_or(0),
                    outcome,
                ));
        } else {
            self.pending_voice_cues.push(match outcome {
                TacticalTrenchRunOutcome::Failure => {
                    TacticalVoiceCue::death_star_attack_broken_off()
                }
                TacticalTrenchRunOutcome::Success => TacticalVoiceCue::death_star_destroyed(),
            });
        }
        self.trench_run_outcome = Some(outcome);
        self.trench_run_remaining_ms = None;
        self.trench_run_pending_outcome = None;
        self.trench_run_participants.clear();
        self.trench_run_cinematic_pending = true;
        self.paused = true;
    }

    pub fn take_pending_trench_run_cinematic(&mut self) -> Option<TacticalTrenchRunOutcome> {
        if !self.trench_run_cinematic_pending {
            return None;
        }
        self.trench_run_cinematic_pending = false;
        self.trench_run_outcome
    }

    pub fn take_pending_audio_cues(&mut self) -> Vec<TacticalAudioCue> {
        std::mem::take(&mut self.pending_audio_cues)
    }

    pub fn take_pending_voice_cues(&mut self) -> Vec<TacticalVoiceCue> {
        std::mem::take(&mut self.pending_voice_cues)
    }

    /// Queue the original faction-specific readiness call when command passes
    /// from Battle Alert into the active tactical manager.
    pub fn queue_battle_ready_voice(&mut self) {
        let faction = self.player_voice_faction();
        self.pending_voice_cues
            .push(TacticalVoiceCue::battle_ready(faction));
        // `FUN_005ada10` state 0 plays that line on its first update (the
        // countdown starts at 0) and moves on: to state 1 when the player
        // is the Alliance facing an operational Death Star, else to 1000.
        let facing_death_star = faction == TacticalVoiceFaction::Alliance
            && self.death_star.is_some_and(TacticalDeathStar::operational);
        self.voice_announcer = TacticalVoiceAnnouncer {
            state: if facing_death_star {
                TacticalVoiceAnnouncerState::Opening(1)
            } else {
                TacticalVoiceAnnouncerState::Waiting
            },
            countdown_ms: ORIGINAL_VOICE_ANNOUNCER_GAP_MS,
            attack_death_star_open: false,
        };
    }

    /// `FUN_005ad930` without the mission control's selection: an
    /// attackable Death Star (`FUN_005ba450`), no trench run
    /// (`FUN_005d0ae0`; port: state 5's timer to its result), fewer Imperial
    /// than Alliance fighter groups on the field (`+0x988`, `+0x168`), and
    /// no player group already on order 6 (`+0x64`/`+0x68`).
    #[must_use]
    pub fn attack_death_star_open(&self) -> bool {
        let (alliance, empire) = self
            .fighters
            .iter()
            .filter(|fighter| fighter.alive)
            .fold((0_usize, 0_usize), |(alliance, empire), fighter| {
                if fighter.identity.is_alliance {
                    (alliance + 1, empire)
                } else {
                    (alliance, empire + 1)
                }
            });
        self.death_star.is_some_and(TacticalDeathStar::attackable)
            && !self.trench_run_under_way()
            && empire < alliance
            && !self.fighters.iter().any(|fighter| {
                fighter.alive
                    && fighter.is_attacker == self.player_is_attacker
                    && fighter.order == TacticalOrder::AttackDeathStar
            })
    }

    /// Whether a side still has a live fighter group. hyp: the side lists
    /// `+0x64` that `FUN_005ada10` asks to be non-empty hold its groups.
    fn side_has_fighter_groups(&self, player_side: bool) -> bool {
        self.fighters.iter().any(|fighter| {
            fighter.alive && (fighter.is_attacker == self.player_is_attacker) == player_side
        })
    }

    /// `FUN_005ada10(elapsed)`, every manager update, paused or not (A0
    /// TAC-01-C012's recording has 0x11c 6.0 s after the opening line, while
    /// the battle is still paused).
    pub fn advance_voice_announcer(&mut self, elapsed_ms: f32) {
        use TacticalVoiceAnnouncerState as State;
        let alliance_player = self.player_voice_faction() == TacticalVoiceFaction::Alliance;
        match self.voice_announcer.state {
            State::Idle => {}
            State::Waiting => {
                self.voice_announcer.countdown_ms -= elapsed_ms;
                if self.voice_announcer.countdown_ms <= 0.0 {
                    self.voice_announcer.state = State::Watching;
                    self.watch_attack_death_star_window(alliance_player);
                }
            }
            State::Watching => self.watch_attack_death_star_window(alliance_player),
            State::Opening(step) => {
                self.voice_announcer.countdown_ms -= elapsed_ms;
                if self.voice_announcer.countdown_ms > 0.0 {
                    return;
                }
                self.voice_announcer.countdown_ms = ORIGINAL_VOICE_ANNOUNCER_GAP_MS;
                let event = ALLIANCE_OPENING_EVENTS[usize::from(step.min(3))];
                self.pending_voice_cues.push(TacticalVoiceCue::from_source_event(
                    TacticalVoiceEvent::DeathStarWarning,
                    event,
                ));
                let next = match (step, self.death_star) {
                    (1, Some(death_star)) if death_star.operational() && death_star.shielded => {
                        State::Opening(2)
                    }
                    (1, _)
                        if self.side_has_fighter_groups(true)
                            && !self.attack_death_star_open()
                            && !self.trench_run_under_way()
                            && self.alliance_fighter_groups() <= self.empire_fighter_groups() =>
                    {
                        State::Opening(3)
                    }
                    _ => State::Waiting,
                };
                self.voice_announcer.state = next;
            }
        }
    }

    fn alliance_fighter_groups(&self) -> usize {
        self.fighters
            .iter()
            .filter(|fighter| fighter.alive && fighter.identity.is_alliance)
            .count()
    }

    fn empire_fighter_groups(&self) -> usize {
        self.fighters
            .iter()
            .filter(|fighter| fighter.alive && !fighter.identity.is_alliance)
            .count()
    }

    /// `FUN_005ada10` state 4: a change in `FUN_005ad930` speaks. Opening for
    /// the Alliance: 0x92 (attack open); for the Empire with its groups:
    /// 0x11c (fighter screen too thin). Closing for the Alliance, with groups
    /// on both sides, no trench run and none of its groups on order 6: 0x94
    /// (fighter screen).
    fn watch_attack_death_star_window(&mut self, alliance_player: bool) {
        let open = self.attack_death_star_open();
        let was_open = self.voice_announcer.attack_death_star_open;
        if was_open && !open {
            if alliance_player
                && self.side_has_fighter_groups(true)
                && self.side_has_fighter_groups(false)
                && !self.trench_run_under_way()
                && !self.fighters.iter().any(|fighter| {
                    fighter.alive
                        && fighter.is_attacker == self.player_is_attacker
                        && fighter.order == TacticalOrder::AttackDeathStar
                })
            {
                self.pending_voice_cues.push(TacticalVoiceCue::from_source_event(
                    TacticalVoiceEvent::DeathStarWarning,
                    0x94,
                ));
            }
        } else if !was_open && open {
            if alliance_player {
                self.pending_voice_cues.push(TacticalVoiceCue::from_source_event(
                    TacticalVoiceEvent::DeathStarWarning,
                    0x92,
                ));
            } else if self.side_has_fighter_groups(true) {
                self.pending_voice_cues.push(TacticalVoiceCue::from_source_event(
                    TacticalVoiceEvent::DeathStarFighterScreen,
                    0x11c,
                ));
            }
        }
        self.voice_announcer.attack_death_star_open = open;
    }

    /// Queue the original player-faction withdrawal acknowledgement.
    pub fn queue_withdrawal_started_voice(&mut self) {
        self.pending_voice_cues
            .push(TacticalVoiceCue::withdrawal_started(
                self.player_voice_faction(),
            ));
    }

    /// Queue the warning of `FUN_005a0240` (voice `0x95` / `0x10e`) when the
    /// withdrawing player's side has a live capital ship that cannot jump
    /// (`FUN_005b1b70`: a hyperdrive and engine power). The source leaves
    /// such ships behind, to be destroyed at the side's end.
    pub fn queue_withdrawal_hyperdrive_warning(&mut self) -> bool {
        let has_stranded_ship = self.ships.iter().enumerate().any(|(index, ship)| {
            ship.alive
                && ship.is_attacker == self.player_is_attacker
                && !original_ship_hyperdrive_works(&self.ships, &self.field_effects, index)
        });
        if has_stranded_ship {
            self.pending_voice_cues
                .push(TacticalVoiceCue::withdrawal_hyperdrive_warning(
                    self.player_voice_faction(),
                ));
        }
        has_stranded_ship
    }

    pub const fn player_voice_faction(&self) -> TacticalVoiceFaction {
        if self.player_is_attacker == self.attacker_is_alliance {
            TacticalVoiceFaction::Alliance
        } else {
            TacticalVoiceFaction::Empire
        }
    }

    /// `FUN_005ad530` for the player's side.
    pub fn begin_player_withdrawal(&mut self) {
        self.begin_withdrawal(self.player_is_attacker);
    }

    /// `FUN_005ad530` for a side's capital ships: each living one, with a
    /// hyperdrive or not (its second loop over `+0x58`), drops its attack
    /// (`FUN_005a8d30`), takes order 3 and heads for its exit
    /// (`FUN_005cc300`). The side's time becomes at least 30 s, or 150 s
    /// for the Empire while its Death Star is operational.
    pub fn begin_withdrawal(&mut self, side_is_attacker: bool) {
        let stamp = self.command_event_stamp();
        self.begin_withdrawal_stamped(side_is_attacker, stamp);
    }

    /// `begin_withdrawal` with its events' stamp: the command stamp from
    /// the interface, the step's from the commander (`FUN_005c8780`).
    fn begin_withdrawal_stamped(&mut self, side_is_attacker: bool, stamp: u32) {
        let mut units = Vec::new();
        for (index, ship) in self.ships.iter_mut().enumerate() {
            if ship.alive && !ship.retreating && ship.is_attacker == side_is_attacker {
                ship.retreating = true;
                ship.selected = false;
                ship.order = TacticalOrder::None;
                ship.attack_target = None;
                ship.manual_targets.clear();
                // `FUN_005aded0`: a force of its own, outside the slots.
                ship.task_force = TACTICAL_UNGROUPED_TASK_FORCE;
                units.push(TacticalUnitRef::Ship(index));
            }
        }
        // `SetOrder(3)` (`0x005ad6b8`): the executor follows on the
        // command stamp (event `0xa`).
        if !units.is_empty() {
            self.post_turn_event(stamp, TacticalTurnEvent::WithdrawalExecutor(units));
        }
        self.start_side_withdrawal_time(side_is_attacker);
    }

    /// `FUN_005ad530`'s task-force loop for a side's fighter groups (class
    /// 4, `FUN_005a6700`): a group whose hyperdrive count (`FUN_005b1bc0`) is
    /// 0 takes order 2, the recovery to a carrier (`FUN_005ca6d0` installs
    /// the `FUN_005cf980` executor); the others take order 3 and jump when
    /// the side's time ends; meanwhile it heads for its side's exit
    /// (`FUN_005c6610`). Either order ends the group's attack.
    pub fn withdraw_fighters(&mut self, side_is_attacker: bool) {
        let stamp = self.command_event_stamp();
        self.withdraw_fighters_stamped(side_is_attacker, stamp);
    }

    fn withdraw_fighters_stamped(&mut self, side_is_attacker: bool, stamp: u32) {
        // A group of the side on the trench run (order 6) has it aborted
        // first (event `0x3c` with state 8, `FUN_005caf20` -> `FUN_005cfec0`).
        let on_the_run = self.trench_run_participants.iter().any(|&index| {
            self.fighters
                .get(index)
                .is_some_and(|fighter| fighter.is_attacker == side_is_attacker)
        }) || self.fighters.iter().any(|fighter| {
            fighter.alive
                && fighter.is_attacker == side_is_attacker
                && fighter.order == TacticalOrder::AttackDeathStar
        });
        if on_the_run {
            self.post_turn_event(stamp, TacticalTurnEvent::TrenchRunState(8));
        }
        // port: a group on the run whose craft its draws have taken (the
        // hull `FUN_005d08d0` kept comes back with state 8) is still a
        // task force the withdrawal orders.
        let on_run = if on_the_run {
            self.trench_run_participants.clone()
        } else {
            Vec::new()
        };
        let mut groups = Vec::new();
        let mut withdrawing = Vec::new();
        for (index, fighter) in self.fighters.iter_mut().enumerate() {
            if fighter.is_attacker == side_is_attacker
                && ((fighter.alive && fighter.squad_count > 0) || on_run.contains(&index))
            {
                fighter.attack_target = None;
                fighter.escort_target = None;
                fighter.selected = false;
                if fighter.has_hyperdrive {
                    fighter.order = TacticalOrder::None;
                    fighter.retreating = true;
                    withdrawing.push(TacticalUnitRef::Fighter(index));
                } else {
                    fighter.order = TacticalOrder::Recover;
                    groups.push(index);
                }
            }
        }
        // `SetOrder(2)` and `SetOrder(3)` (`0x005ad604`, `0x005ad60f`): the
        // executors follow on the command stamp (event `0xa`).
        if !groups.is_empty() {
            self.post_turn_event(
                stamp,
                TacticalTurnEvent::OrderExecutor {
                    order: TacticalOrder::Recover,
                    capitals: Vec::new(),
                    fighters: groups,
                },
            );
        }
        if !withdrawing.is_empty() {
            self.post_turn_event(stamp, TacticalTurnEvent::WithdrawalExecutor(withdrawing));
        }
    }

    /// `FUN_005cf940` resets each selected group's recovery and
    /// `FUN_005cf980` reserves a compatible carrier with room and begins the
    /// inbound leg. A player's group with no room queues its voice once.
    pub fn begin_original_fighter_recovery(&mut self, selected: &[usize]) {
        let player_is_attacker = self.player_is_attacker;
        let player_faction = self.player_voice_faction();
        let mut warned_groups = [false; 4];
        let mut capacity_cues = Vec::new();
        let mut carrier_slots = self
            .ships
            .iter()
            .enumerate()
            .filter(|(_, ship)| ship.alive && ship.fighter_capacity > 0)
            .map(|(index, ship)| (index, ship.is_attacker, ship.fighter_capacity))
            .collect::<Vec<_>>();
        for (index, fighter) in self.fighters.iter().enumerate() {
            if selected.contains(&index)
                || !matches!(
                    fighter.recovery_state,
                    TacticalFighterRecoveryState::Reserved
                        | TacticalFighterRecoveryState::Returning
                        | TacticalFighterRecoveryState::Docking
                )
            {
                continue;
            }
            if let Some((_, _, remaining)) = fighter.recovery_target.and_then(|target| {
                carrier_slots
                    .iter_mut()
                    .find(|(carrier, _, _)| *carrier == target)
            }) {
                *remaining = remaining.saturating_sub(1);
            }
        }
        for &fighter_index in selected {
            let fighter = &mut self.fighters[fighter_index];
            // `FUN_005cf940` always resets the state before `FUN_005cf980`
            // reserves a compatible carrier and begins the inbound leg.
            fighter.recovery_state = TacticalFighterRecoveryState::AwaitingCarrier;
            fighter.recovery_target = None;
            if let Some((carrier, _, remaining)) = carrier_slots
                .iter_mut()
                .find(|(_, side, remaining)| *side == fighter.is_attacker && *remaining > 0)
            {
                fighter.recovery_state = TacticalFighterRecoveryState::Reserved;
                fighter.recovery_target = Some(*carrier);
                *remaining -= 1;
                fighter.recovery_state = TacticalFighterRecoveryState::Returning;
            } else {
                let group = usize::from(fighter.fighter_group);
                if fighter.is_attacker == player_is_attacker
                    && group < warned_groups.len()
                    && !warned_groups[group]
                {
                    warned_groups[group] = true;
                    capacity_cues.push(TacticalVoiceCue::fighter_recovery_capacity(
                        player_faction,
                        fighter.fighter_group,
                    ));
                }
            }
        }
        self.pending_voice_cues.extend(capacity_cues);
    }

    /// The player's side's withdrawal time.
    pub fn start_withdrawal_time(&mut self) {
        self.start_side_withdrawal_time(self.player_is_attacker);
    }

    /// The side's withdrawal time, `FUN_005ad530`: the larger of what is
    /// left and the floor, 150 s for the Empire (side 1) while its Death Star
    /// is operational.
    fn start_side_withdrawal_time(&mut self, side_is_attacker: bool) {
        let empire = self.attacker_is_alliance != side_is_attacker;
        let floor = if empire && self.death_star.is_some_and(TacticalDeathStar::operational) {
            ORIGINAL_DEATH_STAR_WITHDRAWAL_MILLISECONDS
        } else {
            ORIGINAL_WITHDRAWAL_MILLISECONDS
        };
        let slot = self.withdrawal_slot(side_is_attacker);
        *slot = Some(slot.map_or(floor, |left| left.max(floor)));
    }

    /// The time of the player's side or of the AI's.
    fn withdrawal_slot(&mut self, side_is_attacker: bool) -> &mut Option<f32> {
        if side_is_attacker == self.player_is_attacker {
            &mut self.withdrawal_ms
        } else {
            &mut self.ai_withdrawal_ms
        }
    }

    /// The exit line of a side's withdrawal: its tactical commander's
    /// vector (`FUN_005be160`) times `-1.0` and half the battle extent
    /// (`FUN_005cc300`).
    pub fn withdrawal_exit_z(&self, is_alliance: bool) -> f32 {
        let side_z = if is_alliance { 1.0 } else { -1.0 };
        -side_z * self.source_layout.battle_extent * ORIGINAL_WITHDRAWAL_EXIT_EXTENT_SCALE
    }

    /// The capital forces in the fight, each lead unit first (`0x005c6070`):
    /// the group slots (`+0x164`) in their formation order
    /// (`original_formation_capability_score`, `FUN_005c81d0`) and every
    /// ship of a force of its own (`FUN_005aded0`) alone.
    pub(crate) fn capital_forces(&self) -> Vec<Vec<usize>> {
        let in_force = |ship: &TacticalShip| ship.alive && !ship.jumping() && !ship.retreating;
        let mut forces = Vec::new();
        for attacker in [true, false] {
            for slot in 0_u8..8 {
                let mut members = (0..self.ships.len())
                    .filter(|&index| {
                        let ship = &self.ships[index];
                        in_force(ship) && ship.is_attacker == attacker && ship.task_force == slot
                    })
                    .collect::<Vec<_>>();
                if members.is_empty() {
                    continue;
                }
                members.sort_by(|left, right| {
                    original_formation_capability_score(&self.ships[*right])
                        .total_cmp(&original_formation_capability_score(&self.ships[*left]))
                });
                forces.push(members);
            }
        }
        for (index, ship) in self.ships.iter().enumerate() {
            if in_force(ship) && ship.task_force == TACTICAL_UNGROUPED_TASK_FORCE {
                forces.push(vec![index]);
            }
        }
        forces
    }

    /// A force's tactic: its slot's formation (`+0x3c`, set on event
    /// `0x3d`). port: a force of its own uses its ship's tactic at once.
    fn force_tactic(&self, leader: usize) -> TacticalTactic {
        let ship = &self.ships[leader];
        if ship.task_force < 8 {
            self.task_force_formations[usize::from(ship.is_attacker)][usize::from(ship.task_force)]
        } else {
            ship.tactic
        }
    }

    pub(crate) fn attack_target_position(
        &self,
        target: TacticalAttackTarget,
    ) -> Option<TacticalWorldPosition> {
        match target {
            TacticalAttackTarget::CapitalShip(index) => self
                .ships
                .get(index)
                .filter(|ship| ship.alive && !ship.jumping())
                .map(|ship| ship.source_position),
            TacticalAttackTarget::FighterGroup(index) => self
                .fighters
                .get(index)
                .filter(|fighter| fighter.alive && fighter.squad_count > 0 && !fighter.jumping())
                .map(|fighter| fighter.source_position),
        }
    }

    /// `FUN_005b49e0`: a unit's weapons against a target type, each family
    /// of its strongest arc (slot `+0x84`, `FUN_005b4c60`) as a band
    /// [0, range) worth count × slot `+0x60` × strength × modifier; the
    /// families' ranges are slots `+0x54` (laser), `+0x50` (ion) and `+0x58`
    /// (turbolaser).
    pub(crate) fn weapon_bands(
        arcs: &[TacticalWeaponArc],
        ranges: TacticalWeaponRanges,
        recharge: f32,
        vs_fighter: bool,
        sign: f32,
        bands: &mut Vec<(f32, f32)>,
    ) {
        let Some(arc) =
            crate::abstract_battle::strongest_arc(arcs, vs_fighter).map(|arc| arcs[arc])
        else {
            return;
        };
        let strengths = crate::abstract_battle::family_strengths(vs_fighter);
        for (count, strength, range) in [
            (arc.ion_cannons, strengths[1], ranges.ion_cannon),
            (arc.turbolasers, strengths[2], ranges.turbolaser),
            (arc.laser_cannons, strengths[0], ranges.laser_cannon),
        ] {
            if count > 0 {
                bands.push((range, sign * count as f32 * recharge * strength));
            }
        }
    }

    /// `FUN_005edf80`, Stand Off's band against the force's first target:
    /// - the force's weapons add their worth up to their range, and the
    ///   target's weapons against the lead unit take theirs away
    ///   (`FUN_005b49e0`, summed band by band as `FUN_005b7e50` splits
    ///   them);
    /// - the best band is the worthiest whose far edge the force reaches
    ///   (the later of equals);
    /// - min is at least 5.0, and an empty max becomes 10;
    /// - an unarmed force keeps max(target range × 1.2, 10) to 1e6.
    pub(crate) fn stand_off_band(
        &self,
        members: &[TacticalUnitRef],
        target: TacticalAttackTarget,
    ) -> (f32, f32, f32) {
        let max_range = |ranges: TacticalWeaponRanges| {
            ranges
                .ion_cannon
                .max(ranges.laser_cannon)
                .max(ranges.turbolaser)
        };
        let unit_weapons = |unit: TacticalUnitRef| match unit {
            TacticalUnitRef::Ship(index) => {
                let ship = &self.ships[index];
                (
                    ship.weapon_arcs.to_vec(),
                    ship.weapon_ranges,
                    ship.effective_weapon_recharge(),
                )
            }
            TacticalUnitRef::Fighter(index) => {
                let fighter = &self.fighters[index];
                (
                    vec![fighter.weapon_arc],
                    fighter.weapon_ranges,
                    fighter.weapon_hull_factor(),
                )
            }
        };
        let target_unit = match target {
            TacticalAttackTarget::CapitalShip(index) => TacticalUnitRef::Ship(index),
            TacticalAttackTarget::FighterGroup(index) => TacticalUnitRef::Fighter(index),
        };
        let (target_arcs, target_ranges, target_recharge) = unit_weapons(target_unit);
        let vs_fighter = matches!(target, TacticalAttackTarget::FighterGroup(_));
        let leader_is_fighter = matches!(members[0], TacticalUnitRef::Fighter(_));
        let target_reach = max_range(target_ranges);
        let mut weapons = Vec::new();
        let mut own_reach = 0.0_f32;
        for &member in members {
            let (arcs, ranges, recharge) = unit_weapons(member);
            Self::weapon_bands(&arcs, ranges, recharge, vs_fighter, 1.0, &mut weapons);
            own_reach = own_reach.max(max_range(ranges));
        }
        Self::weapon_bands(
            &target_arcs,
            target_ranges,
            target_recharge,
            leader_is_fighter,
            -1.0,
            &mut weapons,
        );
        // The bands all start at 0, so they split at each range: band
        // [lo, hi) is worth every weapon reaching hi.
        let mut edges = weapons
            .iter()
            .map(|(range, _)| *range)
            .filter(|range| *range > 0.0)
            .collect::<Vec<_>>();
        edges.sort_by(f32::total_cmp);
        edges.dedup();
        let mut reach = 0.0_f32;
        let mut best: Option<(f32, f32)> = None;
        let mut best_worth = f32::MIN;
        let mut lo = 0.0;
        for &hi in &edges {
            let worth = weapons
                .iter()
                .filter(|(range, _)| *range >= hi)
                .map(|(_, worth)| *worth)
                .sum::<f32>();
            if reach <= hi && target_reach < hi {
                reach = hi - target_reach;
            }
            if best_worth <= worth && hi <= own_reach {
                best_worth = worth;
                best = Some((lo, hi));
            }
            lo = hi;
        }
        // `+0xc`: 5.0 from the constructor (`DAT_0066cc94`), then the reach
        // beyond the target's, or a third (`DAT_0066dab8`) into the band.
        let mut step = ORIGINAL_SURROUND_APPROACH_STEP;
        if reach > 0.0 {
            step = reach;
        } else if let Some((lo, hi)) = best {
            step = (hi - lo).mul_add(1.0 / 3.0, lo);
        }
        let (mut min, mut max) = best.unwrap_or((0.0, 0.0));
        if own_reach == 0.0 {
            let keep = (target_reach * 1.2).max(10.0);
            step = keep;
            min = keep;
            max = 1_000_000.0;
        }
        if max == 0.0 {
            max = ORIGINAL_STAND_OFF_EMPTY_MAX;
        }
        min = min.max(ORIGINAL_STAND_OFF_BAND_FLOOR);
        (min, max, step)
    }

    /// Once a turn (`0x005a9d40` -> force slot `+0xc`, `0x005c9bd0`): each
    /// force's tactic object recomputes its band against its first target.
    pub(crate) fn update_engagement_bands(&mut self) {
        let mut forces = self
            .capital_forces()
            .into_iter()
            .map(|members| {
                members
                    .into_iter()
                    .map(TacticalUnitRef::Ship)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        forces.extend(
            self.fighter_forces()
                .into_iter()
                .map(|index| vec![TacticalUnitRef::Fighter(index)]),
        );
        let mut bands = Vec::new();
        for members in forces {
            let leader = members[0];
            let (target, tactic) = match leader {
                TacticalUnitRef::Ship(index) => {
                    (self.ships[index].attack_target, self.force_tactic(index))
                }
                TacticalUnitRef::Fighter(index) => {
                    (self.fighter_force_target(index), self.fighters[index].tactic)
                }
            };
            let Some(target) = target else {
                continue;
            };
            if self.attack_target_position(target).is_none() {
                continue;
            }
            let (min, max, step) = if tactic == TacticalTactic::Surround {
                (
                    0.0,
                    ORIGINAL_SURROUND_BAND_MAX,
                    ORIGINAL_SURROUND_APPROACH_STEP,
                )
            } else {
                self.stand_off_band(&members, target)
            };
            bands.push(TacticalEngagementBand {
                leader,
                min,
                max,
                step,
            });
        }
        self.engagement_bands = bands;
    }

    /// A fighter force's target (`+0x8c`, set by slot `+0x1c`,
    /// `FUN_005cb600`): its attack target, or the ship it escorts. Order 1's
    /// install (`FUN_005ca6d0`) keeps `+0x8c` and clears it only when the
    /// order is left; its executor (class `0x66ceb8`, `0x5ae1e0`) does
    /// nothing, so `FUN_005c6610` closes on and circles the escorted ship as
    /// on any target.
    pub(crate) fn fighter_force_target(&self, index: usize) -> Option<TacticalAttackTarget> {
        let fighter = &self.fighters[index];
        if fighter.order == TacticalOrder::Escort {
            fighter.escort_target.map(TacticalAttackTarget::CapitalShip)
        } else {
            fighter.attack_target
        }
    }

    /// The fighter groups that close on targets: in the fight, not
    /// withdrawing, recovering, holding or on the trench run.
    /// port: each group entry is a force, as in the withdrawal.
    pub(crate) fn fighter_forces(&self) -> Vec<usize> {
        (0..self.fighters.len())
            .filter(|&index| {
                let fighter = &self.fighters[index];
                fighter.alive
                    && fighter.squad_count > 0
                    && !fighter.jumping()
                    && !fighter.retreating
                    && fighter.recovery_state == TacticalFighterRecoveryState::AwaitingCarrier
                    && !self.trench_run_participants.contains(&index)
                    && !matches!(
                        fighter.order,
                        TacticalOrder::Recover
                            | TacticalOrder::AttackDeathStar
                            | TacticalOrder::HoldPosition
                    )
            })
            .collect()
    }

    /// The squadrons in the fight of a side's group, in index order.
    fn fighter_group_members(&self, alliance: bool, group: u8) -> Vec<usize> {
        (0..self.fighters.len())
            .filter(|&index| {
                let fighter = &self.fighters[index];
                fighter.alive
                    && fighter.squad_count > 0
                    && !fighter.jumping()
                    && fighter.identity.is_alliance == alliance
                    && fighter.fighter_group == group
            })
            .collect()
    }

    /// `FUN_005ea740` for a fighter mover: the slowest positive member
    /// speed (slot `+0x90`, `0x5ba190`).
    fn fighter_mover_speed(&self, members: &[usize]) -> f32 {
        let speed = members
            .iter()
            .map(|&index| self.original_fighter_speed(index))
            .filter(|speed| *speed > 0.0)
            .fold(f32::INFINITY, f32::min);
        if speed.is_finite() {
            speed
        } else {
            0.0
        }
    }

    /// Keep one mover per side and group with squadrons in the fight
    /// (`FUN_005adfa0`, `FUN_005c62b0`): a new one starts on its first
    /// member, facing its side (hyp, as the capital movers); a member that
    /// joins takes the slot of the count it makes (`0x5ebee0`), one that
    /// leaves frees the count (`0x5ea630`). A tactic does not rebuild it
    /// (`FUN_005c9c50` rebuilds only class 3).
    pub(crate) fn sync_fighter_movers(&mut self) {
        let mut wanted = Vec::new();
        for alliance in [true, false] {
            for group in 0..4_u8 {
                let members = self.fighter_group_members(alliance, group);
                if !members.is_empty() {
                    wanted.push((alliance, group, members));
                }
            }
        }
        self.fighter_movers
            .retain(|mover| wanted.iter().any(|(a, g, _)| *a == mover.alliance && *g == mover.group));
        for (alliance, group, members) in wanted {
            let speed = self.fighter_mover_speed(&members);
            let position = self.fighters[members[0]].source_position;
            let at = match self
                .fighter_movers
                .iter()
                .position(|mover| mover.alliance == alliance && mover.group == group)
            {
                Some(at) => at,
                None => {
                    self.fighter_movers.push(TacticalFighterMover {
                        alliance,
                        group,
                        anchor: position,
                        forward: original_side_vector(alliance),
                        velocity: TacticalWorldVector::ZERO,
                        // port: set at once, not at the next turn.
                        speed,
                        slots: Vec::new(),
                    });
                    self.fighter_movers.len() - 1
                }
            };
            let mover = &mut self.fighter_movers[at];
            mover.slots.retain(|(index, _)| members.contains(index));
            for index in members {
                if mover.slots.iter().any(|(member, _)| *member == index) {
                    continue;
                }
                let offset = original_fighter_slot_offset(mover.slots.len() + 1, mover.forward.z);
                mover.slots.push((index, offset));
            }
        }
    }

    /// A squadron's mover, if it has one.
    pub fn fighter_mover_of(&self, index: usize) -> Option<&TacticalFighterMover> {
        self.fighter_movers
            .iter()
            .find(|mover| mover.slots.iter().any(|(member, _)| *member == index))
    }

    /// A squadron's speed (slot `+0x90`, `0x5ba190`): its side's commander
    /// term (`FUN_005ad7e0`) plus its sublight rating (`+0x3f8`), less its
    /// hits `+0x36c` × that × 0.25 (`DAT_0066c6c4`) and its tractors' pulls
    /// (`+0x360`, `FUN_005b1790`), at least 0. port: the port's squadrons
    /// keep no hit counter or tractor list, and `+0x3b4` (0 speed while
    /// set) is not mapped.
    pub fn original_fighter_speed(&self, index: usize) -> f32 {
        let fighter = &self.fighters[index];
        let term = if fighter.identity.is_alliance {
            self.alliance_commander_term
        } else {
            1.0
        };
        (term + fighter.sublight_engine_power).max(0.0)
    }

    /// A squadron's frame (slot `+0x8`, `0x5b8f70`): its heading
    /// (`0x5b9a20`) is its distance `d` in three dimensions, else the
    /// bearing to its force's target, else its mover's forward; it turns
    /// (`0x5b97a0`, the capital turn rate formula of `FUN_005b9c60`), and
    /// moves along its forward at its speed times its alignment, at least
    /// 0.25 (`DAT_0066c6c4`). port: its speed is its sublight rating, and
    /// the height bands are not yet applied.
    fn fly_fighter_squadron(
        &mut self,
        index: usize,
        delta: TacticalWorldVector,
        bearing_to: Option<TacticalWorldPosition>,
        mover_forward: TacticalWorldVector,
        delta_milliseconds: f32,
    ) {
        if self.fighter_frames.len() < self.fighters.len() {
            let from = self.fighter_frames.len();
            for fighter in &self.fighters[from..] {
                self.fighter_frames.push(TacticalFighterFrame {
                    forward: original_side_vector(fighter.identity.is_alliance),
                    velocity: TacticalWorldVector::ZERO,
                });
            }
        }
        let speed = self.original_fighter_speed(index);
        let fighter = &self.fighters[index];
        if speed == 0.0 {
            self.fighter_frames[index].velocity = TacticalWorldVector::ZERO;
            return;
        }
        let position = fighter.source_position;
        let heading = if delta != TacticalWorldVector::ZERO {
            delta
        } else if let Some(target) = bearing_to {
            source_delta(target, position)
        } else {
            mover_forward
        }
        .normalized();
        let rate = original_capital_turn_rate(fighter.maneuverability);
        let frame = &mut self.fighter_frames[index];
        if heading != TacticalWorldVector::ZERO {
            frame.forward =
                turn_original_fighter(frame.forward, heading, rate, delta_milliseconds);
        }
        let alignment = frame
            .forward
            .dot(heading)
            .max(ORIGINAL_FIGHTER_ALIGNMENT_FLOOR);
        let seconds = delta_milliseconds * 0.001;
        // The height bands (`0x5b9102`..`0x5b969d`), before the move: the
        // search travel is speed × alignment, the step speed × 0.4
        // (`DAT_0066c6b4`); it sets no blocked flag.
        let nearby = self.nearby_capitals_of_squadron(index, speed * alignment * seconds);
        if !nearby.is_empty() {
            let height = |other: usize| TacticalHeightNeighbour {
                y: self.ships[other].source_position.y,
                half_height: self.ships[other]
                    .source_collision_envelope
                    .map_or(0.0, |envelope| envelope.vertical_radius),
                id: other,
            };
            let neighbours = nearby.iter().map(|&other| height(other)).collect::<Vec<_>>();
            // port: a squadron's id sorts after every ship's.
            let own = TacticalHeightNeighbour {
                y: self.fighters[index].source_position.y,
                half_height: 0.0,
                id: self.ships.len() + index,
            };
            self.fighters[index].source_position.y = original_fighter_band_step(
                own,
                &neighbours,
                speed * ORIGINAL_FORMATION_VERTICAL_SHARE * seconds,
            );
        }
        let frame = &mut self.fighter_frames[index];
        frame.velocity = TacticalWorldVector {
            x: frame.forward.x * speed * alignment,
            y: frame.forward.y * speed * alignment,
            z: frame.forward.z * speed * alignment,
        };
        let velocity = frame.velocity;
        let fighter = &mut self.fighters[index];
        fighter.source_position.x += velocity.x * seconds;
        fighter.source_position.y += velocity.y * seconds;
        fighter.source_position.z += velocity.z * seconds;
    }

    /// `FUN_005cdbf0` for a squadron (`0x5b9102`): it searches the capital
    /// tree only (`+0x8f8`, never the fighters' `+0x900`), entered for the
    /// search at its x (`FUN_005ce950`, `FUN_005ee370`), its own reach half
    /// its last velocity. port: a squadron has no envelope, so its
    /// half-height and diameter are 0, and it sorts after ships of equal x.
    pub fn nearby_capitals_of_squadron(&self, index: usize, travel: f32) -> Vec<usize> {
        let reach = |ship: &TacticalShip| {
            let velocity = ship.source_velocity;
            let speed = velocity.x.hypot(velocity.y).hypot(velocity.z);
            0.5 * speed
                + ORIGINAL_NEIGHBOUR_DIAMETER_SHARE
                    * ship
                        .source_collision_envelope
                        .map_or(0.0, |envelope| envelope.planar_diameter)
        };
        let own = self.fighters[index].source_position;
        let own_reach = self.fighter_frames.get(index).map_or(0.0, |frame| {
            let velocity = frame.velocity;
            0.5 * velocity.x.hypot(velocity.y).hypot(velocity.z)
        });
        let mut sorted = (0..self.ships.len())
            .filter(|&other| {
                let ship = &self.ships[other];
                ship.alive && !ship.jumping() && ship.source_collision_envelope.is_some()
            })
            .collect::<Vec<_>>();
        sorted.sort_by(|left, right| {
            self.ships[*left]
                .source_position
                .x
                .total_cmp(&self.ships[*right].source_position.x)
                .then(left.cmp(right))
        });
        let at = sorted
            .iter()
            .position(|&other| self.ships[other].source_position.x > own.x)
            .unwrap_or(sorted.len());
        let mut nearby = Vec::new();
        let mut visit = |other: usize| {
            let ship = &self.ships[other];
            let both = own_reach + reach(ship);
            if (ship.source_position.x - own.x).abs() > both {
                return false;
            }
            let half = ship.source_collision_envelope.map_or(0.0, |e| e.vertical_radius);
            if (ship.source_position.y - own.y).abs()
                <= half + ORIGINAL_NEIGHBOUR_VERTICAL_SLACK * travel
                && (ship.source_position.z - own.z).abs() <= both
            {
                nearby.push(other);
            }
            true
        };
        for &other in sorted[..at].iter().rev() {
            if !visit(other) {
                break;
            }
        }
        for &other in &sorted[at..] {
            if !visit(other) {
                break;
            }
        }
        nearby
    }

    /// `0x5ebe30`, the mover's slot `+0x24`: a squadron's distance to its
    /// place, or, within 2.0 of it, its mover's velocity.
    pub(crate) fn fighter_place_delta(&self, mover: &TacticalFighterMover, index: usize) -> TacticalWorldVector {
        let Some((_, offset)) = mover.slots.iter().find(|(member, _)| *member == index) else {
            return TacticalWorldVector::ZERO;
        };
        let place = TacticalWorldPosition {
            x: mover.anchor.x - offset.x,
            y: mover.anchor.y - offset.y,
            z: mover.anchor.z - offset.z,
        };
        let delta = source_delta(place, self.fighters[index].source_position);
        if delta.dot(delta) <= ORIGINAL_FIGHTER_PLACE_RADIUS_SQUARED {
            mover.velocity
        } else {
            delta
        }
    }

    /// `FUN_005c6610` for a fighter force with a live target, each step from
    /// turn 3: it heads for a point one `+0xc` step toward the target (the
    /// move `0x005ec0d0`, radius 2.0, a full step at its speed beyond it).
    /// Within the radius it circles the target: the angle `+0xc0` grows by
    /// the step's milliseconds × pi × 0.0002 (`DAT_0066cafc`, `DAT_0066cb20`)
    /// and wraps at 2 pi, the Empire's (side 1) negated each time; the
    /// radius is the target's size × 2.4 (`DAT_0066cb28`), at least 1.5
    /// (`DAT_0066cb08`), with half of it above the target (`DAT_0066cb14`).
    pub(crate) fn advance_fighter_approach(&mut self) {
        if self.fighter_orbit_angles.len() < self.fighters.len() {
            self.fighter_orbit_angles.resize(self.fighters.len(), 0.0);
        }
        self.sync_fighter_movers();
        let free = self.fighter_forces();
        for at in 0..self.fighter_movers.len() {
            // The force's order step runs for its first free member.
            let Some(lead) = self.fighter_movers[at]
                .slots
                .iter()
                .map(|(member, _)| *member)
                .find(|member| free.contains(member))
            else {
                continue;
            };
            let Some(target) = self.fighter_force_target(lead) else {
                continue;
            };
            let Some(target_position) = self.attack_target_position(target) else {
                continue;
            };
            let step = self
                .engagement_bands
                .iter()
                .find(|band| band.leader == TacticalUnitRef::Fighter(lead))
                .map_or(ORIGINAL_SURROUND_APPROACH_STEP, |band| band.step);
            let anchor = self.fighter_movers[at].anchor;
            let side_z = if self.fighters[lead].identity.is_alliance {
                1.0
            } else {
                -1.0
            };
            let mut delta = source_delta(target_position, anchor);
            if delta == TacticalWorldVector::ZERO {
                delta = TacticalWorldVector {
                    x: 0.0,
                    y: 0.0,
                    z: side_z,
                };
            }
            let unit = delta.normalized();
            let goal = TacticalWorldPosition {
                x: anchor.x + unit.x * step,
                y: anchor.y + unit.y * step,
                z: anchor.z + unit.z * step,
            };
            let moved = step_fighter_anchor(
                &mut self.fighter_movers[at],
                goal,
                ORIGINAL_STAND_OFF_MIN_STEP,
                ORIGINAL_TACTICAL_STEP_MILLISECONDS,
            );
            if !moved {
                let size = match target {
                    TacticalAttackTarget::CapitalShip(ship) => self.ships[ship]
                        .source_collision_envelope
                        .map_or(0.0, |envelope| envelope.planar_diameter),
                    TacticalAttackTarget::FighterGroup(_) => 0.0,
                };
                let full_turn = 2.0 * std::f32::consts::PI;
                let mut angle = self.fighter_orbit_angles[lead]
                    + ORIGINAL_TACTICAL_STEP_MILLISECONDS * std::f32::consts::PI * 0.000_2;
                if full_turn <= angle {
                    angle -= full_turn;
                }
                if !self.fighters[lead].identity.is_alliance {
                    angle = -angle;
                }
                self.fighter_orbit_angles[lead] = angle;
                let radius = (size * 2.4).max(1.5);
                let (sin, cos) = angle.sin_cos();
                // On arrival the anchor goes to the circling point at once.
                self.fighter_movers[at].anchor = TacticalWorldPosition {
                    x: target_position.x + (cos * radius - sin * radius),
                    y: target_position.y + radius * 0.5,
                    z: target_position.z + cos * radius + sin * radius,
                };
            }
            // Then each free member flies its frame toward its place.
            let mover = self.fighter_movers[at].clone();
            for (member, _) in &mover.slots {
                if free.contains(member) {
                    let delta = self.fighter_place_delta(&mover, *member);
                    self.fly_fighter_squadron(
                        *member,
                        delta,
                        Some(target_position),
                        mover.forward,
                        ORIGINAL_TACTICAL_STEP_MILLISECONDS,
                    );
                }
            }
        }
    }

    /// `FUN_005cc300`'s step for a force with a target, every frame from
    /// turn 3: unless it is on a route or holding, its lead unit heads for
    /// a point one step toward the target while out of its band, and stops
    /// inside it. Stand Off backs off when too close, unless the target
    /// attacks one of the force.
    pub(crate) fn advance_engagement_approach(&mut self) {
        for members in self.capital_forces() {
            let leader = members[0];
            let ship = &self.ships[leader];
            if matches!(
                ship.order,
                TacticalOrder::HoldPosition
                    | TacticalOrder::Escort
                    | TacticalOrder::LeftHook
                    | TacticalOrder::RightHook
                    | TacticalOrder::Hammer
                    | TacticalOrder::Anvil
            ) || !ship.navigation_route.is_empty()
            {
                continue;
            }
            let Some(target) = ship.attack_target else {
                continue;
            };
            let Some(target_position) = self.attack_target_position(target) else {
                continue;
            };
            let band = self
                .engagement_bands
                .iter()
                .find(|band| band.leader == TacticalUnitRef::Ship(leader))
                .copied()
                .unwrap_or(TacticalEngagementBand {
                    leader: TacticalUnitRef::Ship(leader),
                    min: 0.0,
                    max: 0.0,
                    step: ORIGINAL_SURROUND_APPROACH_STEP,
                });
            // `FUN_005cc300` measures from the mover's anchor.
            let anchor = self
                .force_mover(self.force_key(leader))
                .map_or(ship.source_position, |mover| mover.anchor);
            let side_z = if ship.identity.is_alliance { 1.0 } else { -1.0 };
            let mut delta = source_delta(target_position, anchor);
            if delta == TacticalWorldVector::ZERO {
                delta = TacticalWorldVector {
                    x: 0.0,
                    y: 0.0,
                    z: -5.0 * side_z,
                };
            }
            let distance = delta.x.hypot(delta.y).hypot(delta.z);
            let tactic = self.force_tactic(leader);
            if band.min <= distance + ORIGINAL_ENGAGEMENT_MARGIN
                && distance + ORIGINAL_ENGAGEMENT_MARGIN <= band.max
            {
                // The mover's goal is cleared (`+0x78`..`+0x84`).
                self.ships[leader].source_waypoint = None;
                self.ships[leader].source_desired_forward = TacticalWorldVector::ZERO;
                continue;
            }
            if tactic == TacticalTactic::StandOff && distance < band.min {
                // The target force's first target among this force's
                // members (`0x005ca4b0` on `+0x1c`): it holds.
                let attacks_us = match target {
                    TacticalAttackTarget::CapitalShip(index) => self.ships[index]
                        .attack_target
                        .is_some_and(|its| {
                            matches!(its, TacticalAttackTarget::CapitalShip(own) if members.contains(&own))
                        }),
                    TacticalAttackTarget::FighterGroup(index) => self.fighters[index]
                        .attack_target
                        .is_some_and(|its| {
                            matches!(its, TacticalAttackTarget::CapitalShip(own) if members.contains(&own))
                        }),
                };
                if attacks_us {
                    self.ships[leader].source_waypoint = None;
                    self.ships[leader].source_desired_forward = TacticalWorldVector::ZERO;
                    continue;
                }
                delta = TacticalWorldVector {
                    x: 0.0,
                    y: 0.0,
                    z: side_z * band.min * ORIGINAL_STAND_OFF_BACK_OFF,
                };
            }
            // `0x005ccdda`: a computer side's local Stand Off force (`+0x38`
            // 2, `+0x34` 0, its side's `+0x8` 1) of more than 7 members takes
            // Surround (SetTactic `0x005c9be0(1)`, event `0x3d`). port: the
            // computer's side is the one the player does not command.
            if tactic == TacticalTactic::StandOff
                && self.ships[leader].tactic == TacticalTactic::StandOff
                && self.ships[leader].is_attacker != self.player_is_attacker
                && members.len() > 7
            {
                for &member in &members {
                    self.ships[member].tactic = TacticalTactic::Surround;
                }
                let task_force = self.ships[leader].task_force;
                if task_force < 8 {
                    let stamp = self.step_event_stamp();
                    self.post_turn_event(
                        stamp,
                        TacticalTurnEvent::TaskForceTactic {
                            attacker: self.ships[leader].is_attacker,
                            task_force,
                            tactic: TacticalTactic::Surround,
                        },
                    );
                }
            }
            let step = if tactic == TacticalTactic::Surround {
                ORIGINAL_SURROUND_APPROACH_STEP
            } else {
                let share = match members.len() {
                    0..=2 => 0.5,
                    3..=5 => 0.75,
                    _ => 0.9,
                };
                (share * (band.max - band.min) + band.min).max(ORIGINAL_STAND_OFF_MIN_STEP)
            };
            let unit = delta.normalized();
            let goal = TacticalWorldPosition {
                x: anchor.x + unit.x * step,
                y: anchor.y + unit.y * step,
                z: anchor.z + unit.z * step,
            };
            let ship = &mut self.ships[leader];
            ship.source_waypoint = Some(goal);
            ship.source_desired_forward = unit;
        }
    }

    /// Both sides' withdrawals, the player's first.
    /// On a turn's first step `turn` is set: the commanders run then. The
    /// task forces' order steps (slot `+0x14`, `FUN_005c6610` /
    /// `FUN_005cc300`) and the jump flights run in the frame
    /// (`0x005a9ff8`, `0x005aa2a3`), from turn 3.
    fn advance_withdrawal(&mut self, turn: bool) {
        self.advance_side_withdrawal(self.player_is_attacker, turn);
        self.advance_side_withdrawal(!self.player_is_attacker, turn);
    }

    /// `FUN_005c8780`: the withdrawing side's time runs down by the step.
    /// When it ends, each withdrawing ship and fighter group with a working
    /// hyperdrive (`FUN_005b1b70`) starts its jump once it is within 8.0 of
    /// its exit line (`FUN_005cc300`, `FUN_005c6610`: state 6, event
    /// `0x2d`, `FUN_005b1e40`: state 7) and leaves when the jump's 4000 ms
    /// end (`FUN_005a6b40`, event `0x2e`, `FUN_005a90c0`: state 9). The side
    /// waits while any unit with a working hyperdrive is still in the fight
    /// (state 3). Then (state 4) every unit of the side without one gets
    /// event `0x2f` and is destroyed (`FUN_005b2720` -> slot `+0x1c`,
    /// `FUN_005b2d20`: hull 0, state `0xb`). port: the jump's flight is not
    /// drawn.
    fn advance_side_withdrawal(&mut self, side: bool, turn: bool) {
        let Some(left) = *self.withdrawal_slot(side) else {
            return;
        };
        // The commander counts the time down by its turn's 1000 ms.
        let now = if turn {
            (left - ORIGINAL_TACTICAL_TURN_MILLISECONDS).max(0.0)
        } else {
            left
        };
        *self.withdrawal_slot(side) = Some(now);
        let exits = [self.withdrawal_exit_z(true), self.withdrawal_exit_z(false)];
        let frames = self.event_turn() >= ORIGINAL_TACTICAL_EVENT_LEAD_TURNS;
        self.advance_withdrawing_fighters(side, now, exits, frames);
        if now > 0.0 {
            return;
        }
        let works = (0..self.ships.len())
            .map(|index| original_ship_hyperdrive_works(&self.ships, &self.field_effects, index))
            .collect::<Vec<_>>();
        let requested = (0..self.ships.len())
            .map(|index| {
                self.turn_event_pending(&TacticalTurnEvent::JumpStart(TacticalUnitRef::Ship(index)))
            })
            .collect::<Vec<_>>();
        let installed = self.withdrawal_executors.clone();
        let stamp = self.step_event_stamp();
        let mut posts = Vec::new();
        for (index, ship) in self.ships.iter_mut().enumerate() {
            if !ship.alive
                || !ship.retreating
                || ship.is_attacker != side
                || !installed.contains(&TacticalUnitRef::Ship(index))
            {
                continue;
            }
            let ordinal = ship
                .tactical_resource
                .map_or(0, |resource| resource.tactical_ordinal);
            let factor = ORIGINAL_JUMP_ACCELERATIONS[original_jump_factor_index(ordinal, index)];
            let side_z = if ship.identity.is_alliance { 1.0 } else { -1.0 };
            if ship.retreat_progress > 0.0 {
                // State 8 waits for its event `0x2e`.
                if ship.retreat_progress >= ORIGINAL_JUMP_MILLISECONDS {
                    continue;
                }
                let from = ship.retreat_progress;
                ship.retreat_progress += ORIGINAL_TACTICAL_STEP_MILLISECONDS;
                ship.source_position.z -=
                    side_z * original_jump_travel(factor, from, ship.retreat_progress);
                if ship.retreat_progress >= ORIGINAL_JUMP_MILLISECONDS {
                    posts.push(TacticalTurnEvent::JumpEnd(TacticalUnitRef::Ship(index)));
                }
                continue;
            }
            // The task forces' steps (`FUN_005cc300`) run in the frame.
            if !frames || !works[index] {
                // One that cannot jump waits at its exit for the side's end
                // (`FUN_005cc300` keeps steering it).
                continue;
            }
            let exit = if ship.identity.is_alliance {
                exits[0]
            } else {
                exits[1]
            };
            // `FUN_005cc300` tries the jump of its force's first member
            // (`0x005c6070`) once the force's move has arrived
            // (`0x005cc3ef`). `FUN_005ad530` has given each ship a force of
            // its own (`FUN_005aded0` -> `0x005cbf50`), so that member is the
            // ship. A ship already in state 6 waits for its event `0x2d`.
            if requested[index]
                || (exit - ship.source_position.z).abs() > ORIGINAL_WITHDRAWAL_ARRIVAL_RADIUS
            {
                continue;
            }
            if -((exit - ship.source_position.z) * side_z) <= ORIGINAL_JUMP_EXIT_DISTANCE {
                posts.push(TacticalTurnEvent::JumpStart(TacticalUnitRef::Ship(index)));
            }
        }
        for event in posts {
            self.post_turn_event(stamp, event);
        }
        // The commander's state 3 and 4 run once a turn.
        if !turn {
            return;
        }
        // A unit in its jump is waited on (port: one whose engines died
        // mid-jump still finishes it).
        let waiting = self.ships.iter().enumerate().any(|(index, ship)| {
            ship.alive
                && ship.retreating
                && ship.is_attacker == side
                && (works[index] || ship.jumping())
        }) || self.fighters.iter().any(|fighter| {
            fighter.alive
                && fighter.retreating
                && fighter.is_attacker == side
                && (original_fighter_hyperdrive_works(fighter) || fighter.jumping())
        });
        if waiting {
            return;
        }
        // State 4: event `0x2f` for each unit of the side without a working
        // hyperdrive still in the fight.
        for (index, ship) in self.ships.iter_mut().enumerate() {
            if ship.alive && ship.is_attacker == side && !works[index] && !ship.jumping() {
                ship.hull_current = 0;
                ship.shield = 0;
                ship.alive = false;
                ship.retreating = false;
            }
        }
        for fighter in &mut self.fighters {
            if fighter.alive
                && fighter.squad_count > 0
                && fighter.is_attacker == side
                && !original_fighter_hyperdrive_works(fighter)
                && !fighter.jumping()
            {
                fighter.hull_current = 0.0;
                fighter.refresh_craft_count();
                fighter.alive = false;
                fighter.retreating = false;
            }
        }
        *self.withdrawal_slot(side) = None;
    }

    /// `FUN_005c6610`, a fighter force's step on order 3: it heads for its
    /// side's exit line (the side vector times `-1.0`, `_DAT_0066cb10`, and
    /// half, `_DAT_0066cb14`, the extent). Once the side's time is not above
    /// 0 (`_DAT_0066cb18`), each member with a working hyperdrive
    /// (`FUN_005b1b70`) within 8.0 (`_DAT_0066cb0c`) of that line starts its
    /// jump (state 6, event `0x2d`), and leaves when the jump's 4000 ms end
    /// (`FUN_005a6b40`, event `0x2e`) with its craft. One that cannot jump
    /// is destroyed at the side's end (`FUN_005b2720`).
    ///
    /// The move (`0x005ec0d0`) aims the group's anchor at its own x and y
    /// and the line's z. Beyond the arrival radius it flies straight there
    /// at the slowest member's speed (slot `+0x90`, `0x005ea740`) for the
    /// step's time, unclamped, and returns 1, which skips the jumps; within
    /// it the force stops and returns 0. port: a squadron's speed is its
    /// sublight rating, and each member stands at its place until its own
    /// frame (`0x5b8f70`) is ported.
    fn advance_withdrawing_fighters(
        &mut self,
        side: bool,
        time_left: f32,
        exits: [f32; 2],
        frames: bool,
    ) {
        let requested = (0..self.fighters.len())
            .map(|index| {
                self.turn_event_pending(&TacticalTurnEvent::JumpStart(TacticalUnitRef::Fighter(
                    index,
                )))
            })
            .collect::<Vec<_>>();
        let installed = self.withdrawal_executors.clone();
        let stamp = self.step_event_stamp();
        let mut posts = Vec::new();
        self.sync_fighter_movers();
        let mut movers = std::mem::take(&mut self.fighter_movers);
        let mut stepped = vec![false; movers.len()];
        let mut arrived = vec![false; movers.len()];
        let mut flights = Vec::new();
        for (index, fighter) in self.fighters.iter_mut().enumerate() {
            if !fighter.alive
                || !fighter.retreating
                || fighter.is_attacker != side
                || !installed.contains(&TacticalUnitRef::Fighter(index))
            {
                continue;
            }
            let ordinal = fighter
                .tactical_resource
                .map_or(0, |resource| resource.tactical_ordinal);
            let factor = ORIGINAL_JUMP_ACCELERATIONS[original_jump_factor_index(ordinal, index)];
            let (exit, side_z) = if fighter.identity.is_alliance {
                (exits[0], 1.0)
            } else {
                (exits[1], -1.0)
            };
            if fighter.retreat_progress > 0.0 {
                // State 8 waits for its event `0x2e`.
                if fighter.retreat_progress >= ORIGINAL_JUMP_MILLISECONDS {
                    continue;
                }
                let from = fighter.retreat_progress;
                fighter.retreat_progress += ORIGINAL_TACTICAL_STEP_MILLISECONDS;
                fighter.source_position.z -=
                    side_z * original_jump_travel(factor, from, fighter.retreat_progress);
                if fighter.retreat_progress >= ORIGINAL_JUMP_MILLISECONDS {
                    posts.push(TacticalTurnEvent::JumpEnd(TacticalUnitRef::Fighter(index)));
                }
                continue;
            }
            // The task force's step (`FUN_005c6610`) runs in the frame, on
            // the group's anchor; its members stand at their places.
            if !frames {
                continue;
            }
            let Some(at) = movers
                .iter()
                .position(|mover| mover.slots.iter().any(|(member, _)| *member == index))
            else {
                continue;
            };
            if !stepped[at] {
                stepped[at] = true;
                let mover = &mut movers[at];
                let point = TacticalWorldPosition {
                    z: exit,
                    ..mover.anchor
                };
                arrived[at] = !step_fighter_anchor(
                    mover,
                    point,
                    ORIGINAL_WITHDRAWAL_ARRIVAL_RADIUS,
                    ORIGINAL_TACTICAL_STEP_MILLISECONDS,
                );
            }
            // Force slot `+0x18` (`0x5cb050`) on order 3: the member's
            // distance is along z to the exit; at it, it stays put.
            flights.push((index, exit - fighter.source_position.z, movers[at].forward));
            // The jumps are tried only once the move has arrived (0).
            if !arrived[at] {
                continue;
            }
            // A member already in state 6 waits for its event `0x2d`.
            if time_left > 0.0 || requested[index] || !original_fighter_hyperdrive_works(fighter) {
                continue;
            }
            if -((exit - fighter.source_position.z) * side_z) <= ORIGINAL_JUMP_EXIT_DISTANCE {
                posts.push(TacticalTurnEvent::JumpStart(TacticalUnitRef::Fighter(index)));
            }
        }
        self.fighter_movers = movers;
        // The members' frames follow the forces' steps (`0x5a9ec0`).
        for (index, along, forward) in flights {
            if along == 0.0 {
                continue;
            }
            let delta = TacticalWorldVector {
                x: 0.0,
                y: 0.0,
                z: along,
            };
            self.fly_fighter_squadron(index, delta, None, forward, ORIGINAL_TACTICAL_STEP_MILLISECONDS);
        }
        for event in posts {
            self.post_turn_event(stamp, event);
        }
    }

    /// The AI side's tactical commander, `FUN_005c8780`: its count `+0x40`
    /// starts at 0 (`FUN_005c7cb0`) and falls by 1.0 a turn
    /// (`_DAT_0066cbd4`); at 0 or under it is set to 6.0 (`DAT_0066cba4`) and
    /// the commander reviews the battle, so on the first turn and every
    /// sixth after. With its retreat order it withdraws through
    /// `FUN_005ad530` like the player's side when
    /// `abstract_battle::tactical_side_withdraws` says so, unless a trench
    /// run is under way (`FUN_005d0ae0`; port: from state 5's timer to its
    /// result).
    fn ai_side_leaves(&mut self) {
        // Observing, the player's commander reviews too: from the turn it
        // was switched on (its count starts at 0), every sixth. It never
        // withdraws its own side (`0x005c8a8f`: the local side in mode 1).
        if self.player_observing {
            if self.observer_review_countdown == 0 {
                let player_alliance = self.attacker_is_alliance == self.player_is_attacker;
                self.link_commander_targets(player_alliance);
                self.commander_superlaser(player_alliance);
                self.commander_trench_run(player_alliance, false);
                self.observer_review_countdown = ORIGINAL_COMMANDER_REVIEW_TURNS;
            }
            self.observer_review_countdown -= 1;
        }
        if (self.event_turn() - 1) % ORIGINAL_COMMANDER_REVIEW_TURNS != 0 {
            return;
        }
        let alliance = self.attacker_is_alliance != self.player_is_attacker;
        self.link_commander_targets(alliance);
        let side = usize::from(!alliance);
        if self.retreat_allowed[side]
            && !self.retreat_done[side]
            && !self.trench_run_under_way()
            && crate::abstract_battle::tactical_side_withdraws(self, alliance)
        {
            let ai = !self.player_is_attacker;
            let stamp = self.step_event_stamp();
            self.begin_withdrawal_stamped(ai, stamp);
            self.withdraw_fighters_stamped(ai, stamp);
            // `+0x84` = 2: the side cannot order it again (`FUN_005c7e80`).
            self.retreat_done[side] = true;
        }
        self.commander_superlaser(alliance);
        self.commander_trench_run(alliance, true);
    }

    /// The units in the fight of a side (`+0x988`), fighter groups only.
    fn fighters_in_fight(&self, alliance: bool) -> Vec<usize> {
        (0..self.fighters.len())
            .filter(|&index| {
                let fighter = &self.fighters[index];
                fighter.alive
                    && fighter.squad_count > 0
                    && !fighter.jumping()
                    && fighter.identity.is_alliance == alliance
            })
            .collect()
    }

    /// `FUN_005ad930`: a Death Star run is worth sending when the Death Star
    /// is attackable (`FUN_005ba450`), no run is under way (`FUN_005d0ae0`),
    /// the Alliance has more fighter groups in the fight than the Empire,
    /// and no Alliance force is already on order 6 (`+0x64`, `+0x68`).
    fn death_star_run_wanted(&self) -> bool {
        let attackable = self
            .death_star
            .is_some_and(|death_star| !death_star.is_alliance && death_star.attackable());
        attackable
            && !self.trench_run_under_way()
            && self.fighters_in_fight(false).len() < self.fighters_in_fight(true).len()
            && !self.fighters.iter().any(|fighter| {
                fighter.alive
                    && fighter.identity.is_alliance
                    && fighter.order == TacticalOrder::AttackDeathStar
            })
            && !self.ships.iter().any(|ship| {
                ship.alive
                    && ship.identity.is_alliance
                    && ship.order == TacticalOrder::AttackDeathStar
            })
    }

    /// `0x5c8c47`..`0x5c8cb0`, the review's Alliance branch (a Death Star
    /// in the battle, `+0x18` not yet set, no run under way): with a run
    /// worth sending, the group of
    /// its first fighter in the fight takes order 6 (`SetOrder`, the
    /// executor at the command stamp) and `+0x18` is set; with no fighter
    /// left, a side that may leave withdraws (`FUN_005ad530`). port: only
    /// the computer's commander withdraws here.
    fn commander_trench_run(&mut self, alliance: bool, computer: bool) {
        // The whole branch needs the Death Star in the battle
        // (`FUN_005a9330`) and operational (`FUN_005ba420`).
        let death_star_present = self
            .death_star
            .is_some_and(|death_star| !death_star.is_alliance && death_star.operational());
        if !alliance
            || !death_star_present
            || self.alliance_raid_sent
            || self.trench_run_under_way()
        {
            return;
        }
        if self.death_star_run_wanted() {
            let Some(&first) = self.fighters_in_fight(true).first() else {
                return;
            };
            if self.fighters[first].order == TacticalOrder::AttackDeathStar {
                return;
            }
            let group = self.fighters[first].fighter_group;
            let members = self
                .fighters_in_fight(true)
                .into_iter()
                .filter(|&index| self.fighters[index].fighter_group == group)
                .collect::<Vec<_>>();
            for &index in &members {
                self.fighters[index].order = TacticalOrder::AttackDeathStar;
                self.fighters[index].escort_target = None;
            }
            self.alliance_raid_sent = true;
            let stamp = self.step_event_stamp();
            self.post_order_executor_stamped(
                TacticalOrder::AttackDeathStar,
                Vec::new(),
                members,
                stamp,
            );
            return;
        }
        let side = usize::from(!alliance);
        if computer
            && self.fighters_in_fight(true).is_empty()
            && self.retreat_allowed[side]
            && !self.retreat_done[side]
        {
            let ai = !self.player_is_attacker;
            let stamp = self.step_event_stamp();
            self.begin_withdrawal_stamped(ai, stamp);
            self.withdraw_fighters_stamped(ai, stamp);
            self.retreat_done[side] = true;
        }
    }

    /// `0x5c8bbc`..`0x5c8c33`, after the withdrawal test of a review: the
    /// Empire's commander, its Death Star operational and charged to
    /// exactly 100 (`DAT_0066cbd8`), fires at the first capital ship of its
    /// target list still in the fight (`FUN_005ba720`).
    fn commander_superlaser(&mut self, alliance: bool) {
        if alliance {
            return;
        }
        // The Empire attacks when the Alliance does not.
        let owner = !self.attacker_is_alliance;
        let ready = self.death_star.is_some_and(|death_star| {
            !death_star.is_alliance
                && death_star.operational()
                && death_star.laser_charge == ORIGINAL_DEATH_STAR_LASER_FULL_CHARGE
        });
        if !ready {
            return;
        }
        if let Some(target) = crate::abstract_battle::commander_superlaser_target(self, false) {
            let stamp = self.step_event_stamp();
            self.commit_death_star_shot_by(owner, target, stamp);
        }
    }

    /// Command `0x7b` (`FUN_0059fd40`): Observe Battle hands the player's
    /// side to its commander, Take Command takes it back. Switching it on
    /// starts the commander's count afresh (`FUN_005c7e10`).
    pub fn set_player_observing(&mut self, observing: bool) {
        if observing && !self.player_observing {
            self.observer_review_countdown = 0;
            // `+0x18` records a trench run already under way.
            if self.attacker_is_alliance == self.player_is_attacker {
                self.alliance_raid_sent = self.trench_run_under_way();
            }
        }
        self.player_observing = observing;
    }

    /// `FUN_005c9200` comes before the withdrawal test: the commander
    /// links its task forces to targets (`FUN_005a8fc0`). Each new link
    /// is event `0x36` at the command stamp; a kept target posts none.
    fn link_commander_targets(&mut self, alliance: bool) {
        let stamp = self.step_event_stamp();
        let mut links: Vec<(TacticalAttackTarget, Vec<TacticalUnitRef>)> = Vec::new();
        for (unit, target) in crate::abstract_battle::commander_targets(self, alliance) {
            let (unit, current) = match unit {
                crate::abstract_battle::AbstractUnit::Capital(index) => {
                    (TacticalUnitRef::Ship(index), self.ships[index].attack_target)
                }
                crate::abstract_battle::AbstractUnit::Fighter(index) => {
                    (TacticalUnitRef::Fighter(index), self.fighters[index].attack_target)
                }
            };
            if current == Some(target) {
                continue;
            }
            match links.iter_mut().find(|(linked, _)| *linked == target) {
                Some((_, units)) => units.push(unit),
                None => links.push((target, vec![unit])),
            }
        }
        for (target, units) in links {
            self.post_turn_event(stamp, TacticalTurnEvent::TaskForceTarget { units, target });
        }
    }

    pub fn queue_battle_outcome_voice(&mut self, winner: CombatWinner) {
        let player_won = matches!(
            (winner, self.player_is_attacker),
            (CombatWinner::Attacker, true) | (CombatWinner::Defender, false)
        );
        if winner == CombatWinner::Draw {
            return;
        }
        let faction = self.player_voice_faction();
        let player_withdrew = self
            .ships
            .iter()
            .any(|ship| ship.is_attacker == self.player_is_attacker && ship.retreated);
        let opponent_withdrew = self
            .ships
            .iter()
            .any(|ship| ship.is_attacker != self.player_is_attacker && ship.retreated);
        let cue = if player_won {
            TacticalVoiceCue::battle_won(faction, opponent_withdrew)
        } else if player_withdrew {
            TacticalVoiceCue::withdrawal_complete(faction)
        } else {
            TacticalVoiceCue::battle_lost(faction)
        };
        self.pending_voice_cues.push(cue);
    }
}

// ---------------------------------------------------------------------------
// TacticalState — top-level UI state
// ---------------------------------------------------------------------------

pub fn source_delta(to: TacticalWorldPosition, from: TacticalWorldPosition) -> TacticalWorldVector {
    TacticalWorldVector {
        x: to.x - from.x,
        y: to.y - from.y,
        z: to.z - from.z,
    }
}

pub fn escort_target_is_eligible(
    session: &BattleSession,
    source_side: bool,
    target: usize,
) -> bool {
    session
        .ships
        .get(target)
        .is_some_and(|ship| ship.alive && !ship.retreating && ship.is_attacker == source_side)
}

/// Clear order-1 target slots when the protected hull is destroyed or leaves
/// combat. This mirrors the executor's object-lifetime boundary and prevents a
/// stale white escort box.
pub fn refresh_original_escort_targets(session: &mut BattleSession) {
    for index in 0..session.ships.len() {
        if session.ships[index].order != TacticalOrder::Escort {
            session.ships[index].escort_target = None;
            continue;
        }
        let valid = session.ships[index].escort_target.is_some_and(|target| {
            target != index
                && escort_target_is_eligible(session, session.ships[index].is_attacker, target)
        });
        if !valid {
            let ship = &mut session.ships[index];
            ship.order = TacticalOrder::None;
            ship.escort_target = None;
            ship.source_waypoint = None;
            ship.source_desired_forward = TacticalWorldVector::ZERO;
        }
    }
    for index in 0..session.fighters.len() {
        if session.fighters[index].order != TacticalOrder::Escort {
            session.fighters[index].escort_target = None;
            continue;
        }
        let valid = session.fighters[index].escort_target.is_some_and(|target| {
            escort_target_is_eligible(session, session.fighters[index].is_attacker, target)
        });
        if !valid {
            session.fighters[index].order = TacticalOrder::None;
            session.fighters[index].escort_target = None;
        }
    }
}

/// Keep order-1 capital ships around their protected ship while autonomous
/// weapons continue to select nearby opportunity targets, through the normal
/// retained movement integrator. Fighter groups escort through their force's
/// step (`BattleSession::fighter_force_target`).
///
/// port: the capital escort offsets are the port's own; movement integrates per `FUN_005cd640`.
pub fn advance_original_escort_movement(session: &mut BattleSession, delta_milliseconds: f32) {
    if !delta_milliseconds.is_finite() || delta_milliseconds <= 0.0 {
        return;
    }
    let target_positions = session
        .ships
        .iter()
        .map(|ship| ship.source_position)
        .collect::<Vec<_>>();
    for (index, ship) in session.ships.iter_mut().enumerate() {
        let Some(target) = ship
            .escort_target
            .filter(|_| ship.order == TacticalOrder::Escort)
        else {
            continue;
        };
        let Some(mut waypoint) = target_positions.get(target).copied() else {
            continue;
        };
        let side = if index % 2 == 0 { 1.0 } else { -1.0 };
        waypoint.x += side * 8.0;
        waypoint.z -= 6.0 + (index % 3) as f32 * 3.0;
        ship.navigation_route.clear();
        ship.navigation_route.push(waypoint);
        ship.source_waypoint = Some(waypoint);
        ship.source_desired_forward = source_delta(waypoint, ship.source_position).normalized();
    }
}

/// `DAT_0066d07c`, applied after each source maneuver rotation.
pub const ORIGINAL_MANEUVER_WAYPOINT_SCALE: f32 = 0.75;
/// `DAT_0066d094` and `DAT_0066d098` multiply pi by +/- one eighth.
pub const ORIGINAL_MANEUVER_ANGLE: f32 = std::f32::consts::PI / 8.0;

pub fn original_maneuver_waypoint(
    order: TacticalOrder,
    current: TacticalWorldPosition,
    target: TacticalWorldPosition,
) -> Option<TacticalWorldPosition> {
    let delta = source_delta(target, current);
    let (sin, cos) = match order {
        TacticalOrder::LeftHook | TacticalOrder::Anvil => ORIGINAL_MANEUVER_ANGLE.sin_cos(),
        TacticalOrder::RightHook | TacticalOrder::Hammer => (-ORIGINAL_MANEUVER_ANGLE).sin_cos(),
        _ => return None,
    };
    let rotated = match order {
        TacticalOrder::LeftHook | TacticalOrder::RightHook => TacticalWorldVector {
            x: cos.mul_add(delta.x, -sin * delta.z),
            y: delta.y,
            z: cos.mul_add(delta.z, sin * delta.x),
        },
        TacticalOrder::Hammer | TacticalOrder::Anvil => TacticalWorldVector {
            x: delta.x,
            y: cos.mul_add(delta.y, sin * delta.z),
            z: cos.mul_add(delta.z, -sin * delta.y),
        },
        _ => return None,
    };
    Some(TacticalWorldPosition {
        x: current.x + rotated.x * ORIGINAL_MANEUVER_WAYPOINT_SCALE,
        y: current.y + rotated.y * ORIGINAL_MANEUVER_WAYPOINT_SCALE,
        z: current.z + rotated.z * ORIGINAL_MANEUVER_WAYPOINT_SCALE,
    })
}

pub fn original_maneuver_target(
    ships: &[TacticalShip],
    source: usize,
    order: TacticalOrder,
) -> Option<TacticalWorldPosition> {
    let source_ship = ships.get(source)?;
    let opponents = ships
        .iter()
        .filter(|ship| ship.alive && ship.is_attacker != source_ship.is_attacker)
        .collect::<Vec<_>>();
    let count = opponents.len() as f32;
    if count == 0.0 {
        return None;
    }
    let anchor = opponents
        .iter()
        .fold(TacticalWorldPosition::ORIGIN, |sum, ship| {
            TacticalWorldPosition {
                x: sum.x + ship.source_position.x / count,
                y: sum.y + ship.source_position.y / count,
                z: sum.z + ship.source_position.z / count,
            }
        });
    let delta = source_delta(anchor, source_ship.source_position);
    let selector = match order {
        TacticalOrder::LeftHook => TacticalWorldVector {
            x: -delta.z,
            y: delta.y,
            z: delta.x,
        },
        TacticalOrder::RightHook => TacticalWorldVector {
            x: delta.z,
            y: delta.y,
            z: -delta.x,
        },
        TacticalOrder::Hammer => TacticalWorldVector {
            x: delta.x,
            y: -delta.y,
            z: delta.z,
        },
        TacticalOrder::Anvil => delta,
        _ => return None,
    }
    .normalized();
    let mut selected = None;
    let mut selected_score = f32::NEG_INFINITY;
    for opponent in opponents {
        let position = opponent.source_position;
        let score = selector.dot(TacticalWorldVector {
            x: position.x,
            y: position.y,
            z: position.z,
        });
        if score > selected_score {
            selected = Some(position);
            selected_score = score;
        }
    }
    selected
}

pub fn begin_original_capital_order(
    session: &mut BattleSession,
    selected: &[usize],
    order: TacticalOrder,
) {
    for &index in selected {
        if order == TacticalOrder::HoldPosition {
            let ship = &mut session.ships[index];
            ship.source_waypoint = None;
            ship.navigation_route.clear();
            ship.source_desired_forward = TacticalWorldVector::ZERO;
            ship.source_velocity = TacticalWorldVector::ZERO;
            continue;
        }
        if !matches!(
            order,
            TacticalOrder::LeftHook
                | TacticalOrder::RightHook
                | TacticalOrder::Hammer
                | TacticalOrder::Anvil
        ) {
            continue;
        }
        let current = session.ships[index].source_position;
        let waypoint = original_maneuver_target(&session.ships, index, order)
            .and_then(|target| original_maneuver_waypoint(order, current, target));
        let ship = &mut session.ships[index];
        ship.navigation_route.clear();
        if let Some(waypoint) = waypoint {
            ship.source_waypoint = Some(waypoint);
            ship.source_desired_forward = source_delta(waypoint, current).normalized();
        } else {
            // `FUN_005ca6d0` returns the object to no-orders when no executor
            // target can be created.
            ship.order = TacticalOrder::None;
            ship.source_waypoint = None;
            ship.source_desired_forward = TacticalWorldVector::ZERO;
            ship.source_velocity = TacticalWorldVector::ZERO;
        }
    }
}

/// Run the target-acquisition half of `FUN_005d0b10` and `FUN_005d0bb0`.
/// Both source executors traverse one stable tactical-object list, filter it
/// by the vtable type code (capital 0, fighter 1), accept the first eligible
/// hostile object, mark the executor active, and dispatch event `0x36`.
pub fn begin_original_attack_order(
    session: &mut BattleSession,
    selected_capitals: &[usize],
    selected_fighters: &[usize],
    order: TacticalOrder,
) {
    if order == TacticalOrder::AttackDeathStar {
        let Some(death_star) = session
            .death_star
            .filter(|death_star| death_star.hull > 0.0 && !death_star.destroyed)
        else {
            return;
        };
        let Some((fighter_group, fighter_is_attacker)) =
            selected_fighters.iter().find_map(|&index| {
                session
                    .fighters
                    .get(index)
                    .filter(|fighter| {
                        fighter.alive
                            && fighter.squad_count > 0
                            && fighter.is_attacker != death_star.is_attacker
                    })
                    .map(|fighter| (fighter.fighter_group, fighter.is_attacker))
            })
        else {
            return;
        };
        if session.trench_run_under_way() || session.trench_run_outcome.is_some() {
            return;
        }
        // FUN_005caae9 creates the result object; the run then goes
        // through its approach (FUN_005c6610) and state 5 (FUN_005cfec0).
        let _ = death_star;
        let status = if session.begin_original_trench_run(fighter_group, fighter_is_attacker) {
            "approach"
        } else {
            "rejected"
        };
        session.diagnostic_log.push(format!(
            "[tactical_death_star] trench_run_launch status={status} fighter_members={} group={fighter_group} remaining_ms={} rng_state={}",
            selected_fighters.len(),
            session.trench_run_remaining_ms.unwrap_or(0.0),
            session.tactical_rng.state(),
        ));
        return;
    }

    if !matches!(
        order,
        TacticalOrder::AttackCapitalShips | TacticalOrder::AttackFighters
    ) {
        return;
    }

    for &index in selected_capitals {
        let source_is_attacker = session.ships[index].is_attacker;
        let current = session.ships[index].attack_target;
        if current.is_some_and(|target| {
            tactical_attack_target_is_eligible(session, source_is_attacker, target, order)
        }) {
            continue;
        }
        let target = first_original_attack_target(session, source_is_attacker, order);
        session.ships[index].attack_target = target;
    }
    for &index in selected_fighters {
        let source_is_attacker = session.fighters[index].is_attacker;
        let current = session.fighters[index].attack_target;
        if current.is_some_and(|target| {
            tactical_attack_target_is_eligible(session, source_is_attacker, target, order)
        }) {
            continue;
        }
        let target = first_original_attack_target(session, source_is_attacker, order);
        session.fighters[index].attack_target = target;
    }
}

pub fn tactical_attack_target_is_eligible(
    session: &BattleSession,
    source_is_attacker: bool,
    target: TacticalAttackTarget,
    order: TacticalOrder,
) -> bool {
    match (order, target) {
        (TacticalOrder::AttackCapitalShips, TacticalAttackTarget::CapitalShip(index)) => {
            session.ships.get(index).is_some_and(|ship| {
                ship.alive && !ship.jumping() && ship.is_attacker != source_is_attacker
            })
        }
        (TacticalOrder::AttackFighters, TacticalAttackTarget::FighterGroup(index)) => {
            session.fighters.get(index).is_some_and(|fighter| {
                fighter.alive
                    && fighter.squad_count > 0
                    && !fighter.jumping()
                    && fighter.recovery_state != TacticalFighterRecoveryState::Recovered
                    && fighter.is_attacker != source_is_attacker
            })
        }
        _ => false,
    }
}

pub fn manual_attack_target_is_eligible(
    session: &BattleSession,
    source_is_attacker: bool,
    target: TacticalAttackTarget,
) -> bool {
    match target {
        TacticalAttackTarget::CapitalShip(index) => session.ships.get(index).is_some_and(|ship| {
            ship.alive && !ship.jumping() && ship.is_attacker != source_is_attacker
        }),
        TacticalAttackTarget::FighterGroup(index) => {
            session.fighters.get(index).is_some_and(|fighter| {
                fighter.alive
                    && fighter.squad_count > 0
                    && !fighter.jumping()
                    && fighter.recovery_state != TacticalFighterRecoveryState::Recovered
                    && fighter.is_attacker != source_is_attacker
            })
        }
    }
}

pub fn first_original_attack_target(
    session: &BattleSession,
    source_is_attacker: bool,
    order: TacticalOrder,
) -> Option<TacticalAttackTarget> {
    match order {
        TacticalOrder::AttackCapitalShips => session
            .ships
            .iter()
            .enumerate()
            .find(|(_, ship)| {
                ship.alive && !ship.jumping() && ship.is_attacker != source_is_attacker
            })
            .map(|(index, _)| TacticalAttackTarget::CapitalShip(index)),
        TacticalOrder::AttackFighters => session
            .fighters
            .iter()
            .enumerate()
            .find(|(_, fighter)| {
                fighter.alive
                    && fighter.squad_count > 0
                    && !fighter.jumping()
                    && fighter.recovery_state != TacticalFighterRecoveryState::Recovered
                    && fighter.is_attacker != source_is_attacker
            })
            .map(|(index, _)| TacticalAttackTarget::FighterGroup(index)),
        _ => None,
    }
}

/// Maintain the typed target list created by the original attack executors.
///
/// `FUN_005a8fc0` links the first eligible typed target into the owner's group.
/// The manager then exposes separate add, remove, replace, and change-target
/// messages plus stable next/previous target selection. Open Rebellion models
/// that lifecycle directly: an invalid entry is replaced by the first eligible
/// hostile object of the same requested class, and an exhausted class clears
/// the target instead of falling through to another class.
pub fn refresh_original_attack_targets(session: &mut BattleSession) {
    let mut completed_player_task_forces = [false; 9];
    let mut completed_player_fighter_groups = [false; 4];
    for index in 0..session.ships.len() {
        let source_is_attacker = session.ships[index].is_attacker;
        let had_attack_target = session.ships[index].attack_target.is_some();
        let retained_manual_targets = session.ships[index]
            .manual_targets
            .iter()
            .copied()
            .filter(|target| manual_attack_target_is_eligible(session, source_is_attacker, *target))
            .collect::<Vec<_>>();
        if !retained_manual_targets.is_empty() {
            session.ships[index].attack_target = retained_manual_targets.first().copied();
            session.ships[index].manual_targets = retained_manual_targets;
            continue;
        }
        if !session.ships[index].manual_targets.is_empty() {
            session.ships[index].manual_targets.clear();
            session.ships[index].attack_target = None;
        }
        let order = session.ships[index].order;
        if !matches!(
            order,
            TacticalOrder::AttackCapitalShips | TacticalOrder::AttackFighters
        ) {
            continue;
        }
        if !session.ships[index].alive || session.ships[index].jumping() {
            session.ships[index].attack_target = None;
            continue;
        }
        let current = session.ships[index].attack_target;
        if current.is_some_and(|target| {
            tactical_attack_target_is_eligible(session, source_is_attacker, target, order)
        }) {
            continue;
        }
        session.ships[index].attack_target =
            first_original_attack_target(session, source_is_attacker, order);
        if had_attack_target && session.ships[index].attack_target.is_none() {
            let ship = &session.ships[index];
            if ship.is_attacker == session.player_is_attacker {
                completed_player_task_forces[task_force_slot(ship.task_force)] = true;
            }
        }
    }

    for index in 0..session.fighters.len() {
        let had_attack_target = session.fighters[index].attack_target.is_some();
        let order = session.fighters[index].order;
        if !matches!(
            order,
            TacticalOrder::AttackCapitalShips | TacticalOrder::AttackFighters
        ) {
            continue;
        }
        if !session.fighters[index].alive
            || session.fighters[index].squad_count == 0
            || session.fighters[index].recovery_state == TacticalFighterRecoveryState::Recovered
        {
            session.fighters[index].attack_target = None;
            continue;
        }
        let source_is_attacker = session.fighters[index].is_attacker;
        let current = session.fighters[index].attack_target;
        if current.is_some_and(|target| {
            tactical_attack_target_is_eligible(session, source_is_attacker, target, order)
        }) {
            continue;
        }
        session.fighters[index].attack_target =
            first_original_attack_target(session, source_is_attacker, order);
        if had_attack_target && session.fighters[index].attack_target.is_none() {
            let fighter = &session.fighters[index];
            if fighter.is_attacker == session.player_is_attacker {
                completed_player_fighter_groups[usize::from(fighter.fighter_group.min(3))] = true;
            }
        }
    }

    let faction = session.player_voice_faction();
    for (group, transitioned) in completed_player_task_forces.into_iter().enumerate() {
        if transitioned
            && !session.ships.iter().any(|ship| {
                ship.alive
                    && ship.is_attacker == session.player_is_attacker
                    && task_force_slot(ship.task_force) == group
                    && ship.attack_target.is_some()
            })
        {
            session
                .pending_voice_cues
                .push(TacticalVoiceCue::capital_orders_complete(
                    faction,
                    task_force_of_slot(group),
                ));
        }
    }
    for (group, transitioned) in completed_player_fighter_groups.into_iter().enumerate() {
        if transitioned
            && !session.fighters.iter().any(|fighter| {
                fighter.alive
                    && fighter.is_attacker == session.player_is_attacker
                    && usize::from(fighter.fighter_group) == group
                    && fighter.attack_target.is_some()
            })
        {
            session
                .pending_voice_cues
                .push(TacticalVoiceCue::fighter_orders_complete(
                    faction,
                    group as u8,
                ));
        }
    }
}

pub fn player_is_alliance(session: &BattleSession) -> bool {
    session.player_is_attacker == session.attacker_is_alliance
}
