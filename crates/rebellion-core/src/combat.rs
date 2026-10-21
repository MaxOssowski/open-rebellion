//! The result of a space battle, as the strategic layer applies it.
//!
//! The original resolves a battle in its tactical module, displayed or not
//! (`ghidra/notes/battle-flow.md`). The port does the same in
//! [`crate::abstract_battle`], which reports its losses in these types.
//! `FUN_0040c780` reads the module's result and applies the losses to each
//! fleet, capital ship and squadron at the system.

use serde::{Deserialize, Serialize};

use crate::ids::{FleetKey, SystemKey};

/// The outcome of a space battle, named after the first fleet of each side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceCombatResult {
    pub attacker_fleet: FleetKey,
    pub defender_fleet: FleetKey,
    pub system: SystemKey,
    pub winner: CombatSide,
    pub ship_damage: Vec<ShipDamageEvent>,
    pub fighter_losses: Vec<FighterLossEvent>,
    pub tick: u64,
}

/// Which side won a combat engagement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatSide {
    Attacker,
    Defender,
    Draw,
}

/// A hull's new value and hit counters after a battle (`FUN_0040cfc0`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipDamageEvent {
    pub fleet: FleetKey,
    /// The hull's place among the fleet's live hulls when the battle began.
    pub ship_index: usize,
    pub hull_before: i32,
    pub hull_after: i32,
    /// The ship's `+0x64` hit counters after the battle
    /// ([`crate::world::ShipInstance::subsystem_hits`]).
    #[serde(default)]
    pub hits_after: u32,
}

/// A fighter entry's surviving squadrons after a battle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FighterLossEvent {
    pub fleet: FleetKey,
    pub fighter_index: usize,
    pub squads_before: u32,
    pub squads_after: u32,
}
