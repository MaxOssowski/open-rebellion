//! Externalized tuning parameters for AI and production systems.
//!
//! All hardcoded constants are centralized here as `GameConfig`. The struct
//! is `Serialize`/`Deserialize` so the autoresearch loop can mutate parameters
//! via JSON, and the playtest binary can accept `--config <path>`.
//!
//! Each field is tagged as **parity** (matches original REBEXE.EXE behavior)
//! or **augmentation** (our improvement beyond the original).
//!
//! `Default` produces the values that match the current hardcoded constants.

use serde::{Deserialize, Serialize};

/// Root configuration for all tunable game parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GameConfig {
    pub ai: AiConfig,
    pub production: ProductionConfig,
    pub scoring: ScoringConfig,
}

/// AI behavior tuning parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// Maximum popularity at which AI still sends diplomacy missions. **Augmentation.**
    /// Unread since 2026-10-18: the AI's Diplomacy planner takes systems
    /// below 70 support or neutral (`FUN_00481720`, record `+0x2c` bit
    /// `0x40`). Kept so the save layout and saved configs still load.
    pub diplomacy_target_popularity_cap: f32,

    /// Maximum covert operations dispatched per evaluation cycle. **Augmentation.**
    /// Unread since 2026-10-19: the heuristic Sabotage and Assassination it
    /// capped are gone; the sector planners order those missions
    /// (`FUN_004bda60`). Kept so the save layout and saved configs still load.
    pub max_covert_ops_per_eval: usize,

    /// Maximum construction yards per system. **Parity.**
    pub max_construction_yards: usize,

    /// Minimum troop garrison count before a controlled system is considered
    /// a troop deployment receiver. Systems with fewer than this many friendly
    /// troops will receive reinforcements from donor systems. **Augmentation.**
    pub troop_garrison_min: usize,

    /// Maximum troop count at a system above which it becomes a donor.
    /// Systems with more than this many friendly troops will send excess to
    /// undersupplied systems. **Augmentation.**
    pub troop_garrison_donor_threshold: usize,

    /// Maximum reconnaissance missions the AI will queue per evaluation cycle.
    /// Prevents over-investing in intelligence gathering. **Augmentation.**
    pub max_recon_per_eval: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            diplomacy_target_popularity_cap: 0.8,
            max_covert_ops_per_eval: 3,
            max_construction_yards: 5,
            troop_garrison_min: 1,
            troop_garrison_donor_threshold: 3,
            max_recon_per_eval: 2,
        }
    }
}

/// Production doctrine tuning parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductionConfig {
    /// Build capital ships when fewer than this many in the fleet. **Augmentation.**
    pub capship_threshold: usize,

    /// Fighter-to-capship ratio target. **Augmentation.**
    pub fighter_ratio: usize,
}

impl Default for ProductionConfig {
    fn default() -> Self {
        Self {
            capship_threshold: 5,
            fighter_ratio: 3,
        }
    }
}

/// Scoring targets for `eval_game_quality.py` (not used in Rust, but stored
/// here for autoresearch config symmetry).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoringConfig {
    /// Ideal victory tick (bell curve center). **Augmentation.**
    pub victory_center: u32,
    /// Ideal combat event percentage (bell curve center). **Augmentation.**
    pub combat_pct_center: f64,
    /// Ideal manufacturing completions per 5000 ticks. **Augmentation.**
    pub mfg_target: u32,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            victory_center: 3000,
            combat_pct_center: 10.0,
            mfg_target: 50,
        }
    }
}
